use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameRow {
	game_id: Uuid,
	host_networking_enabled: bool,
	root_user_enabled: bool,
}

#[operation(name = "mm-config-game-get")]
pub async fn handle(
	ctx: OperationContext<mm_config::game_get::Request>,
) -> GlobalResult<mm_config::game_get::Response> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let rows = sql_fetch_all!(
		[ctx, GameRow]
		"
		SELECT game_id, host_networking_enabled, root_user_enabled
		FROM db_mm_config.games
		WHERE game_id = ANY($1)
		",
		&game_ids,
	)
	.await?;

	let games = game_ids
		.iter()
		.map(|game_id| {
			let row = rows.iter().find(|row| row.game_id == *game_id);
			mm_config::game_get::response::Game {
				game_id: Some((*game_id).into()),
				config: Some(backend::matchmaker::GameConfig {
					host_networking_enabled: row.map_or(false, |row| row.host_networking_enabled),
					root_user_enabled: row.map_or(false, |row| row.root_user_enabled),
				}),
			}
		})
		.collect();

	Ok(mm_config::game_get::Response { games })
}
