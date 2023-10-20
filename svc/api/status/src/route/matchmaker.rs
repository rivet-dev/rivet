use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use rivet_status_server::models;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

// MARK: GET /matchmaker
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusQuery {
	region: String,
}

pub async fn status(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: StatusQuery,
) -> GlobalResult<models::MatchmakerResponse> {
	let domain_cdn = unwrap!(util::env::domain_cdn());

	// Build client
	let client = rivet_matchmaker::Config::builder()
		.set_uri("http://traefik.traefik.svc.cluster.local:80/matchmaker")
		.build_client();

	// Create bypass token
	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(15),
		}),
		refresh_token_config: None,
		issuer: "api-status".to_owned(),
		client: Some(ctx.client_info()),
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
	let token = unwrap_ref!(token_res.token).token.clone();

	tracing::info!("finding lobby");
	let origin = format!("https://test-game.{domain_cdn}/");
	client
		.find_lobby()
		.origin(origin)
		.bypass_token(token)
		.game_modes("default")
		.regions(&query.region)
		.send()
		.await?;

	// TODO: Include connecting to socket, see stress test

	Ok(models::MatchmakerResponse {})
}
