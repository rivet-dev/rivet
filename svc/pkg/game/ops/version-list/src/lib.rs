use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct VersionRow {
	version_id: Uuid,
	game_id: Uuid,
}

#[operation(name = "game-version-list")]
async fn handle(
	ctx: OperationContext<game::version_list::Request>,
) -> GlobalResult<game::version_list::Response> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let version_rows = sqlx::query_as::<_, VersionRow>(indoc!(
		"
		SELECT version_id, game_id
		FROM game_versions
		WHERE game_id = ANY($1)
		ORDER BY create_ts DESC
		"
	))
	.bind(&game_ids)
	.fetch_all(&ctx.crdb("db-game").await?)
	.await?;

	let games = game_ids
		.iter()
		.map(|game_id| game::version_list::response::Game {
			game_id: Some((*game_id).into()),
			version_ids: version_rows
				.iter()
				.filter(|row| row.game_id == *game_id)
				.map(|row| common::Uuid::from(row.version_id))
				.collect::<Vec<_>>(),
		})
		.collect::<Vec<_>>();

	Ok(game::version_list::Response { games })
}
