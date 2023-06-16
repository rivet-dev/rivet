use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "kv-config-namespace-create")]
async fn handle(
	ctx: OperationContext<kv_config::namespace_create::Request>,
) -> GlobalResult<kv_config::namespace_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO game_namespaces (namespace_id)
		VALUES ($1)
		"
	))
	.bind(namespace_id)
	.execute(&ctx.crdb("db-kv-config").await?)
	.await?;

	Ok(kv_config::namespace_create::Response {})
}
