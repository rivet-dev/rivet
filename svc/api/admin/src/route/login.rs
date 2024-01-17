use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

pub const TOKEN_TTL: i64 = util::duration::minutes(15);

// MARK: POST /login
pub async fn login(
	ctx: Ctx<Auth>,
	body: models::AdminLoginRequest,
) -> GlobalResult<models::AdminLoginResponse> {
	let token_res = op!([ctx] token_create {
		issuer: "api-admin".to_string(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: TOKEN_TTL,
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::AccessToken(proto::claims::entitlement::AccessToken {
							name: body.name.clone(),
						})
					)
				}
			],
		})),
		label: Some("access".to_string()),
		..Default::default()
	})
	.await?;

	let access_token_token = unwrap!(token_res.token).token;
	let access_token_link_url = util::route::access_token_link(&access_token_token);

	Ok(models::AdminLoginResponse {
		url: access_token_link_url,
	})
}
