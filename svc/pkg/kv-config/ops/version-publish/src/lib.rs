use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "kv-config-version-publish")]
async fn handle(
	ctx: OperationContext<kv_config::version_publish::Request>,
) -> GlobalResult<kv_config::version_publish::Response> {
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();
	let _config = unwrap_ref!(ctx.config);
	let _config_ctx = unwrap_ref!(ctx.config_ctx);

	sqlx::query("INSERT INTO db_kv_config.game_versions (version_id) VALUES ($1)")
		.bind(version_id)
		.execute(&ctx.crdb().await?)
		.await?;

	Ok(kv_config::version_publish::Response {})
}
