use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-config-namespace-create")]
async fn handle(
	ctx: OperationContext<mm_config::namespace_create::Request>,
) -> GlobalResult<mm_config::namespace_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO game_namespaces (namespace_id)
		VALUES ($1)
	"
	))
	.bind(namespace_id)
	.execute(&ctx.crdb("db-mm-config").await?)
	.await?;

	Ok(mm_config::namespace_create::Response {})
}
