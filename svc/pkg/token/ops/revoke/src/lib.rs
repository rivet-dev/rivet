use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "token-revoke")]
async fn handle(
	ctx: OperationContext<token::revoke::Request>,
) -> GlobalResult<token::revoke::Response> {
	let crdb = ctx.crdb().await?;

	let jtis = ctx
		.jtis
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	sqlx::query(indoc!(
		"
		UPDATE db_token.tokens
		SET revoke_ts = $2
		WHERE jti = ANY($1)
		"
	))
	.bind(&jtis)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	Ok(token::revoke::Response {})
}
