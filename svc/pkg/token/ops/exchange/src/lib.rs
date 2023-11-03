use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "token-exchange")]
async fn handle(
	ctx: OperationContext<token::exchange::Request>,
) -> GlobalResult<token::exchange::Response> {
	let crdb = ctx.crdb().await?;

	let jti = unwrap_ref!(ctx.jti).as_uuid();

	let update_query = sql_query!(
		[ctx]
		"
		UPDATE db_token.tokens
		SET revoke_ts = $2
		WHERE jti = $1 AND revoke_ts IS NULL AND exp > $2
		",
		jti,
		ctx.ts(),
	)
	.await?;

	ensure_with!(update_query.rows_affected() > 0, TOKEN_EXCHANGE_FAILED);

	Ok(token::exchange::Response {})
}
