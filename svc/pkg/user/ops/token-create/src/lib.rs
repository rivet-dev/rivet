use proto::backend::pkg::*;
use rivet_operation::prelude::*;

// Also see api-auth/src/route/tokens.rs
pub const TOKEN_TTL: i64 = util::duration::minutes(15);
pub const REFRESH_TOKEN_TTL: i64 = util::duration::days(90);

#[operation(name = "user-token-create")]
async fn handle(
	ctx: OperationContext<user::token_create::Request>,
) -> GlobalResult<user::token_create::Response> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: TOKEN_TTL,
		}),
		refresh_token_config: Some(token::create::request::TokenConfig {
			ttl: REFRESH_TOKEN_TTL,
		}),
		issuer: Self::NAME.to_owned(),
		client: ctx.client.clone(),
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew {
				entitlements: vec![proto::claims::Entitlement {
					kind: Some(proto::claims::entitlement::Kind::User(proto::claims::entitlement::User {
						user_id: Some(user_id.into()),
					})),
				}],
			},
		)),
		label: Some("usr".into()),
		..Default::default()
	})
	.await?;

	let token = internal_unwrap!(token_res.token);
	let refresh_token = internal_unwrap!(token_res.refresh_token);
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

	sqlx::query("INSERT INTO db_user.user_tokens (user_id, token_session_id) VALUES ($1, $2)")
		.bind(user_id)
		.bind(token_session_id)
		.execute(&ctx.crdb().await?)
		.await?;

	Ok(user::token_create::Response {
		token: token.token.clone(),
		refresh_token: refresh_token.token.clone(),
	})
}
