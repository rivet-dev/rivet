use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Version {
	lobby_group_id: Uuid,
	version_id: Uuid,
}

impl From<Version> for mm_config::lobby_group_resolve_version::response::Version {
	fn from(value: Version) -> Self {
		mm_config::lobby_group_resolve_version::response::Version {
			lobby_group_id: Some(value.lobby_group_id.into()),
			version_id: Some(value.version_id.into()),
		}
	}
}

#[operation(name = "mm-config-lobby-group-resolve-version")]
async fn handle(
	ctx: OperationContext<mm_config::lobby_group_resolve_version::Request>,
) -> GlobalResult<mm_config::lobby_group_resolve_version::Response> {
	let lobby_group_ids = ctx
		.lobby_group_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let versions = ctx
		.cache()
		.fetch_all_proto(
			"lobby_groups",
			lobby_group_ids,
			|mut cache, lobby_group_ids| {
				let ctx = ctx.base();

				async move {
					let versions = sql_fetch_all!(
						[ctx, Version]
						"
						SELECT version_id, lobby_group_id
						FROM db_mm_config.lobby_groups
						WHERE lobby_group_id = ANY($1)
						",
						lobby_group_ids,
					)
					.await?
					.into_iter();

					for version in versions {
						let lobby_group_id = version.lobby_group_id;
						cache.resolve(
							&lobby_group_id,
							Into::<mm_config::lobby_group_resolve_version::response::Version>::into(
								version,
							),
						);
					}

					Ok(cache)
				}
			},
		)
		.await?;

	Ok(mm_config::lobby_group_resolve_version::Response { versions })
}
