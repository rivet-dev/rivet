use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Game {
	game_id: Uuid,
	// No important data here, this is a placeholder for things to come
}

impl From<Game> for backend::cloud::Game {
	fn from(value: Game) -> Self {
		backend::cloud::Game {
			game_id: Some(value.game_id.into()),
		}
	}
}

#[operation(name = "cloud-game-config-get")]
async fn handle(
	ctx: OperationContext<cloud::game_config_get::Request>,
) -> GlobalResult<cloud::game_config_get::Response> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let game_configs = sql_fetch_all!(
		[ctx, Game]
		"
		SELECT game_id
		FROM db_cloud.game_configs
		WHERE game_id = ANY($1)
		",
		game_ids,
	)
	.await?
	.into_iter()
	.map(Into::<backend::cloud::Game>::into)
	.collect::<Vec<_>>();

	Ok(cloud::game_config_get::Response { game_configs })
}
