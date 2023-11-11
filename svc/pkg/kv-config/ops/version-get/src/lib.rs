use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
}

#[operation(name = "kv-config-version-get")]
async fn handle(
	ctx: OperationContext<kv_config::version_get::Request>,
) -> GlobalResult<kv_config::version_get::Response> {
	let version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let versions = sql_fetch_all!(
		[ctx, GameVersion]
		"
			SELECT version_id
			FROM db_kv_config.game_versions
			WHERE version_id = ANY($1)
		",
		version_ids,
	)
	.await?
	.into_iter()
	.map(|version| kv_config::version_get::response::Version {
		version_id: Some(version.version_id.into()),
		config: Some(backend::kv::VersionConfig {}),
		config_meta: Some(backend::kv::VersionConfigMeta {}),
	})
	.collect::<Vec<_>>();

	Ok(kv_config::version_get::Response { versions })
}
