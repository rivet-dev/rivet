use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
	// No important data here, this is a placeholder for things to come
}

#[operation(name = "cloud-version-get")]
async fn handle(
	ctx: OperationContext<cloud::version_get::Request>,
) -> GlobalResult<cloud::version_get::Response> {
	let req_version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let cloud_versions = sqlx::query_as::<_, GameVersion>(indoc!(
		"
		SELECT version_id
		FROM game_versions
		WHERE version_id = ANY($1)
	"
	))
	.bind(req_version_ids)
	.fetch_all(&ctx.crdb("db-cloud").await?)
	.await?;

	// Get all version IDs that exist. If a row doesn't exist in `game_configs`, then this version
	// is ignored.
	let all_version_ids_proto = cloud_versions
		.iter()
		.map(|c| c.version_id)
		.map(common::Uuid::from)
		.collect::<Vec<_>>();

	// Fetch all dependent configs
	let (cdn_configs_res, mm_configs_res, kv_configs_res, identity_configs_res, module_configs_res) =
		tokio::try_join!(
			op!([ctx] cdn_version_get {
				version_ids: all_version_ids_proto.clone(),
			}),
			op!([ctx] mm_config_version_get {
				version_ids: all_version_ids_proto.clone(),
			}),
			op!([ctx] kv_config_version_get {
				version_ids: all_version_ids_proto.clone(),
			}),
			op!([ctx] identity_config_version_get {
				version_ids: all_version_ids_proto.clone(),
			}),
			op!([ctx] module_game_version_get {
				version_ids: all_version_ids_proto.clone(),
			}),
		)?;

	let versions = cloud_versions
		.iter()
		.map(|cloud_version| {
			let version_id_proto = common::Uuid::from(cloud_version.version_id);
			backend::cloud::GameVersion {
				version_id: Some(version_id_proto),
				config: Some(backend::cloud::VersionConfig {
					cdn: cdn_configs_res
						.versions
						.iter()
						.find(|cdn_version| {
							cdn_version.version_id.as_ref() == Some(&version_id_proto)
						})
						.map(|v| v.config.clone().unwrap()),
					matchmaker: mm_configs_res
						.versions
						.iter()
						.find(|mm_version| {
							mm_version.version_id.as_ref() == Some(&version_id_proto)
						})
						.map(|v| v.config.clone().unwrap()),
					kv: kv_configs_res
						.versions
						.iter()
						.find(|kv_version| {
							kv_version.version_id.as_ref() == Some(&version_id_proto)
						})
						.map(|v| v.config.clone().unwrap()),
					identity: identity_configs_res
						.versions
						.iter()
						.find(|identity_version| {
							identity_version.version_id.as_ref() == Some(&version_id_proto)
						})
						.map(|v| v.config.clone().unwrap()),
					module: module_configs_res
						.versions
						.iter()
						.find(|module_version| {
							module_version.version_id.as_ref() == Some(&version_id_proto)
						})
						.map(|v| v.config.clone().unwrap()),
				}),
			}
		})
		.collect::<Vec<_>>();

	Ok(cloud::version_get::Response { versions })
}
