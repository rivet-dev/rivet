use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{assert, auth::Auth};

// MARK: POST /games/{}/environments/{}/tokens/service
pub async fn create_service(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	env_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<models::GamesEnvironmentsCreateServiceTokenResponse> {
	ctx.auth()
		.check_game(ctx.op_ctx(), game_id, env_id, false)
		.await?;

	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			// TODO: Allow configuring expiration
			ttl: util::duration::days(15 * 365)
		}),
		refresh_token_config: None,
		issuer: "api-games".to_owned(),
		client: None,
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew { entitlements: vec![proto::claims::Entitlement {
				kind: Some(proto::claims::entitlement::Kind::EnvService(
					proto::claims::entitlement::EnvService {
						env_id: Some(env_id.into()),
					}
				)),
			}]},
		)),
		label: Some("env_svc".to_owned()),
		..Default::default()
	})
	.await?;

	let token = unwrap_ref!(token_res.token);

	Ok(models::GamesEnvironmentsCreateServiceTokenResponse {
		token: token.token.clone(),
	})
}
