use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::{self, backend::pkg::*};

pub const TOKEN_TTL: i64 = util::duration::minutes(15);

#[derive(Debug)]
pub struct Input {
	pub username: String,
}

#[derive(Debug)]
pub struct Output {
	pub url: String,
}

#[operation]
pub async fn login_create(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let token_res = op!([ctx] token_create {
		issuer: "admin".to_string(),
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
							name: input.username.clone(),
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
	let access_token_link_url = util::route::access_token_link(ctx.config(), &access_token_token);

	Ok(Output {
		url: access_token_link_url,
	})
}
