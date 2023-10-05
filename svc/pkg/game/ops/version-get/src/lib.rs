use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameVersion {
	version_id: Uuid,
	game_id: Uuid,
	create_ts: i64,
	display_name: String,
}

impl From<GameVersion> for backend::game::Version {
	fn from(value: GameVersion) -> Self {
		backend::game::Version {
			version_id: Some(value.version_id.into()),
			game_id: Some(value.game_id.into()),
			create_ts: value.create_ts,
			display_name: value.display_name,
		}
	}
}

#[operation(name = "game-version-get")]
async fn handle(
	ctx: OperationContext<game::version_get::Request>,
) -> GlobalResult<game::version_get::Response> {
	let version_ids = ctx
		.version_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let versions = ctx
		.cache()
		.immutable()
		.fetch_all_proto("version_ids", version_ids, |mut cache, version_ids| {
			let ctx = ctx.base();
			async move {
				sqlx::query_as::<_, GameVersion>(indoc!(
					"
					SELECT version_id, game_id, create_ts, display_name
					FROM db_game.game_versions
					WHERE version_id = ANY($1)
					ORDER BY create_ts DESC
					"
				))
				.bind(version_ids)
				.fetch_all(&ctx.crdb().await?)
				.await?
				.into_iter()
				.for_each(|row| {
					let version_id = row.version_id;
					cache.resolve_with_topic(
						&version_id,
						Into::<backend::game::Version>::into(row),
						("game_versions", &version_id),
					)
				});
				Ok(cache)
			}
		})
		.await?;

	Ok(game::version_get::Response { versions })
}
