use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct CustomDisplayName {
	display_name: String,
	version_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct CustomAvatar {
	upload_id: Uuid,
	version_id: Uuid,
}

#[operation(name = "identity-config-version-get")]
async fn handle(
	ctx: OperationContext<identity_config::version_get::Request>,
) -> GlobalResult<identity_config::version_get::Response> {
	let version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let crdb = ctx.crdb().await?;
	let (versions, custom_display_names, custom_avatars) = tokio::try_join!(
		sqlx::query_as::<_, GameVersion>(indoc!(
			"
				SELECT version_id
				FROM db_identity_config.game_versions
				WHERE version_id = ANY($1)
			"
		))
		.bind(&version_ids)
		.fetch_all(&crdb),
		sqlx::query_as::<_, CustomDisplayName>(indoc!(
			"
			SELECT display_name, version_id
			FROM db_identity_config.custom_display_names
			WHERE version_id = ANY($1)
			"
		))
		.bind(&version_ids)
		.fetch_all(&crdb),
		sqlx::query_as::<_, CustomAvatar>(indoc!(
			"
			SELECT upload_id, version_id
			FROM db_identity_config.custom_avatars
			WHERE version_id = ANY($1)
			"
		))
		.bind(&version_ids)
		.fetch_all(&crdb),
	)?;

	let versions = versions
		.into_iter()
		.map(|v| {
			let custom_display_names = custom_display_names
				.iter()
				.filter(|c| c.version_id == v.version_id)
				.map(|custom_display_name| backend::identity::CustomDisplayName {
					display_name: custom_display_name.display_name.clone(),
				})
				.collect::<Vec<_>>();
			let custom_avatars = custom_avatars
				.iter()
				.filter(|c| c.version_id == v.version_id)
				.map(|custom_avatar| backend::identity::CustomAvatar {
					upload_id: Some(custom_avatar.upload_id.into()),
				})
				.collect::<Vec<_>>();

			identity_config::version_get::response::Version {
				version_id: Some(v.version_id.into()),
				config: Some(backend::identity::VersionConfig {
					custom_display_names,
					custom_avatars,
				}),
				config_meta: Some(backend::identity::VersionConfigMeta {}),
			}
		})
		.collect::<Vec<_>>();

	Ok(identity_config::version_get::Response { versions })
}
