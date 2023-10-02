use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "identity-config-namespace-create")]
async fn handle(
	ctx: OperationContext<identity_config::namespace_create::Request>,
) -> GlobalResult<identity_config::namespace_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO db_identity_config.game_namespaces (namespace_id)
		VALUES ($1)
		"
	))
	.bind(namespace_id)
	.execute(&ctx.crdb().await?)
	.await?;

	Ok(identity_config::namespace_create::Response {})
}
