use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-config-namespace-create")]
async fn handle(
	ctx: OperationContext<mm_config::namespace_create::Request>,
) -> GlobalResult<mm_config::namespace_create::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_mm_config.game_namespaces (namespace_id)
		VALUES ($1)
	",
		namespace_id,
	)
	.await?;

	Ok(mm_config::namespace_create::Response {})
}
