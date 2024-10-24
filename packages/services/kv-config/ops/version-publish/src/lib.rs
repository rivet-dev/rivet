use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "kv-config-version-publish")]
async fn handle(
	ctx: OperationContext<kv_config::version_publish::Request>,
) -> GlobalResult<kv_config::version_publish::Response> {
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();
	let _config = unwrap_ref!(ctx.config);
	let _config_ctx = unwrap_ref!(ctx.config_ctx);

	sql_execute!(
		[ctx]
		"INSERT INTO db_kv_config.game_versions (version_id) VALUES ($1)",
		version_id,
	)
	.await?;

	Ok(kv_config::version_publish::Response {})
}
