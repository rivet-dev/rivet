use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
	config: Vec<u8>,
}

#[operation(name = "module-game-version-get")]
async fn handle(
	ctx: OperationContext<module::game_version_get::Request>,
) -> GlobalResult<module::game_version_get::Response> {
	let crdb = ctx.crdb().await?;

	let version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let versions = sqlx::query_as::<_, GameVersion>(indoc!(
		"
		SELECT version_id, config
		FROM db_module.game_versions
		WHERE version_id = ANY($1)
		"
	))
	.bind(&version_ids)
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(|x| -> GlobalResult<_> {
		let config = backend::module::GameVersionConfig::decode(x.config.as_slice())?;
		Ok(module::game_version_get::response::Version {
			version_id: Some(x.version_id.into()),
			config: Some(config),
			config_meta: Some(backend::module::GameVersionConfigMeta {}),
		})
	})
	.collect::<GlobalResult<_>>()?;

	Ok(module::game_version_get::Response { versions })
}
