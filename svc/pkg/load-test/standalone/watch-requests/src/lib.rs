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
	let pools = rivet_pools::from_env("load-test-watch-requests").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-watch-requests");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-watch-requests".into(),
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

			..Default::default()
		})
		.await?;
		let team_id = unwrap_ref!(create_res.team_id).as_uuid();
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
		unwrap_ref!(token_res.token).token.clone()
	};

	let client = reqwest::Client::builder()
		.default_headers({
			let mut headers = reqwest::header::HeaderMap::new();
			headers.insert(
				"x-bypass-token",
				reqwest::header::HeaderValue::from_str(&bypass_token)?,
			);
			headers.insert(
				"host",
				reqwest::header::HeaderValue::from_str(unwrap!(util::env::domain_main_api()))?,
			);
			headers.insert(
				"cf-connecting-ip",
				reqwest::header::HeaderValue::from_str("192.0.2.0")?,
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

	let mut interval = tokio::time::interval(Duration::from_millis(50));
	for i in 0..3_000 {
		interval.tick().await;

		if i % 100 == 0 {
			tracing::info!(?i, "request");
		}

		let config = config.clone();
		tokio::spawn(async move {
			let mut watch_index = Option::<String>::None;
			loop {
				let start = Instant::now();
				match rivet_api::apis::identity_events_api::identity_events_watch(
					&config,
					watch_index.as_ref().map(|x| x.as_str()),
				)
				.await
				{
					Ok(res) => {
						if watch_index.is_none() {
							tracing::info!(elapsed = ?start.elapsed().as_secs_f64(), "received initial response");
						}
						watch_index = Some(res.watch.index);
					}
					Err(err) => tracing::error!(?err, "error"),
				}
			}
		});
	}

	Ok(())
}
