use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct TokenRow {
	jti: Uuid,
	exp: Option<i64>,
	iat: i64,

	refresh_jti: Option<Uuid>,
	session_id: Uuid,
	issuer: String,
	user_agent: Option<String>,
	remote_address: Option<String>,
	revoke_ts: Option<i64>,
}

impl From<TokenRow> for token::get::Token {
	fn from(token: TokenRow) -> token::get::Token {
		token::get::Token {
			jti: Some(token.jti.into()),
			exp: token.exp,
			iat: token.iat,

			refresh_jti: token.refresh_jti.map(|x| x.into()),
			session_id: Some(token.session_id.into()),
			issuer: token.issuer,
			user_agent: token.user_agent,
			remote_address: token.remote_address,
			revoke_ts: token.revoke_ts,
		}
	}
}

#[operation(name = "token-get")]
async fn handle(ctx: OperationContext<token::get::Request>) -> GlobalResult<token::get::Response> {
	let _crdb = ctx.crdb().await?;

	let jtis = ctx
		.jtis
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let tokens = sql_fetch_all!(
		[ctx, TokenRow]
		"
		SELECT
			jti,
			exp,
			iat,
			refresh_jti,
			session_id,
			issuer,
			user_agent,
			remote_address,
			revoke_ts
		FROM db_token.tokens
		WHERE jti = ANY($1)
		",
		&jtis,
	)
	.await?
	.into_iter()
	.map(Into::<token::get::Token>::into)
	.collect::<Vec<_>>();

	Ok(token::get::Response { tokens })
}
