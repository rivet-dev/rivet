use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "kv-config-namespace-create")]
async fn handle(
	ctx: OperationContext<kv_config::namespace_create::Request>,
) -> GlobalResult<kv_config::namespace_create::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	sql_query!(
		[ctx]
		"
		INSERT INTO db_kv_config.game_namespaces (namespace_id)
		VALUES ($1)
		",
		namespace_id,
	)
	.await?;

	Ok(kv_config::namespace_create::Response {})
}
