use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use unzip_n::unzip_n;

#[operation(name = "module-game-version-publish")]
async fn handle(
	ctx: OperationContext<module::game_version_publish::Request>,
) -> GlobalResult<module::game_version_publish::Response> {
	let crdb = ctx.crdb("db-module").await?;

	let version_id = internal_unwrap!(ctx.version_id).as_uuid();
	let config = internal_unwrap!(ctx.config);

	let mut config_buf = Vec::with_capacity(config.encoded_len());
	config.encode(&mut config_buf)?;

	sqlx::query("INSERT INTO game_versions (version_id, config) VALUES ($1, $2)")
		.bind(version_id)
		.bind(config_buf)
		.execute(&crdb)
		.await?;

	Ok(module::game_version_publish::Response {})
}
