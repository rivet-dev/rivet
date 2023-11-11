use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "identity-config-namespace-create")]
async fn handle(
	ctx: OperationContext<identity_config::namespace_create::Request>,
) -> GlobalResult<identity_config::namespace_create::Response> {
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_identity_config.game_namespaces (namespace_id)
		VALUES ($1)
		",
		namespace_id,
	)
	.await?;

	Ok(identity_config::namespace_create::Response {})
}
