use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "token-revoke")]
async fn handle(
	ctx: OperationContext<token::revoke::Request>,
) -> GlobalResult<token::revoke::Response> {
	let jtis = ctx
		.jtis
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	sql_execute!(
		[ctx]
		"
		UPDATE db_token.tokens
		SET revoke_ts = $2
		WHERE jti = ANY($1)
		",
		&jtis,
		ctx.ts(),
	)
	.await?;

	Ok(token::revoke::Response {})
}
