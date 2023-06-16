use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cdn-namespace-create")]
async fn handle(
	ctx: OperationContext<cdn::namespace_create::Request>,
) -> GlobalResult<cdn::namespace_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO game_namespaces (namespace_id)
		VALUES ($1)
		"
	))
	.bind(namespace_id)
	.execute(&ctx.crdb("db-cdn").await?)
	.await?;

	Ok(cdn::namespace_create::Response {})
}
