use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "kv-config-version-publish")]
async fn handle(
	ctx: OperationContext<kv_config::version_publish::Request>,
) -> GlobalResult<kv_config::version_publish::Response> {
	let version_id = internal_unwrap!(ctx.version_id).as_uuid();
	let _config = internal_unwrap!(ctx.config);
	let _config_ctx = internal_unwrap!(ctx.config_ctx);

	sqlx::query("INSERT INTO db_kv_config.game_versions (version_id) VALUES ($1)")
		.bind(version_id)
		.execute(&ctx.crdb().await?)
		.await?;

	Ok(kv_config::version_publish::Response {})
}
