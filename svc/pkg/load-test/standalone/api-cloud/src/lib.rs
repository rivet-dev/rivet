use std::collections::HashSet;

use futures_util::StreamExt;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::{apis::configuration::Configuration, models};
use rivet_operation::prelude::*;
use serde_json::json;
use tokio::time::{interval, Duration, Instant};

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("load-test-api-cloud").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-api-cloud");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-api-cloud".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	// Create temp team
	let (team_id, primary_user_id) = {
		// Create team
		let create_res = op!([ctx] faker_team {
			is_dev: true,
			..Default::default()
		})
		.await?;
		let team_id = internal_unwrap!(create_res.team_id).as_uuid();
		let primary_user_id = create_res.member_user_ids[0].as_uuid();

		// Register user
		op!([ctx] user_identity_create {
			user_id: Some(primary_user_id.into()),
			identity: Some(backend::user_identity::Identity {
				kind: Some(backend::user_identity::identity::Kind::Email(backend::user_identity::identity::Email {
					email: util::faker::email()
				}))
			})
		})
		.await
		.unwrap();

		(team_id, primary_user_id)
	};

	// Create games
	for _ in 0..4 {
		op!([ctx] faker_game {
			dev_team_id: Some(team_id.into()),
		})
		.await?;
	}

	// Encode user token
	let auth_token = {
		let token_res = op!([ctx] token_create {
			issuer: "test".into(),
			token_config: Some(token::create::request::TokenConfig {
				ttl: util::duration::hours(1),
			}),
			refresh_token_config: None,
			client: Some(backend::net::ClientInfo {
				user_agent: Some("Test".into()),
				remote_address: Some("0.0.0.0".into()),
			}),
			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(
							proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
								user_id: Some(primary_user_id.into()),
							})
						)
					},
				],
			})),
			label: Some("usr".into()),
			..Default::default()
		})
		.await
		.unwrap();
		let token = token_res.token.unwrap();

		token.token
	};
	let bypass_token = {
		let token_res = op!([ctx] token_create {
			token_config: Some(token::create::request::TokenConfig {
				ttl: util::duration::hours(1)
			}),
			refresh_token_config: None,
			issuer: "api-status".to_owned(),
			client: None,
			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(
							proto::claims::entitlement::Kind::Bypass(proto::claims::entitlement::Bypass { })
						)
					}
				],
			})),
			label: Some("byp".to_owned()),
			..Default::default()
		})
		.await?;
		internal_unwrap!(token_res.token).token.clone()
	};

	let client = reqwest::Client::builder()
		.default_headers({
			let mut headers = reqwest::header::HeaderMap::new();
			headers.insert(
				"x-bypass-token",
				reqwest::header::HeaderValue::from_str(&bypass_token)?,
			);
			headers
		})
		.build()?;
	let config = Configuration {
		client,
		base_path: "http://traefik.traefik.svc.cluster.local:80".into(),
		bearer_access_token: Some(auth_token),
		..Default::default()
	};

	let start = Instant::now();
	let mut next_log = Instant::now();
	let mut elapsed = 0.0;
	loop {
		// Delay
		let Some(delay) = calc_delay(elapsed) else {
			break;
		};
		elapsed += delay;
		tokio::time::sleep_until(start + Duration::from_secs_f64(elapsed)).await;

		// Log
		if Instant::now() > next_log {
			tracing::info!(?elapsed, rps = ?(1.0 / delay), "sending request");
			next_log += Duration::from_secs(1);
		}

		let config = config.clone();
		tokio::spawn(async move {
			// Taint
			match rivet_api::apis::cloud_games_games_api::cloud_games_games_get_games(&config, None)
				.await
			{
				Ok(_) => {}
				Err(err) => tracing::error!(?err, "error"),
			}
		});
	}

	Ok(())
}

fn calc_delay(elapsed: f64) -> Option<f64> {
	let start_rps = 1.0;
	let finish_rps = 200.0;
	let duration = 120.0;

	if elapsed > duration {
		return None;
	}

	let lerp = elapsed / duration;

	let rps = (finish_rps - start_rps) * lerp + start_rps;
	let delay = 1.0 / rps;

	Some(delay)
}
