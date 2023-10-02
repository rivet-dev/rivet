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

	let game_configs = sqlx::query_as::<_, Game>(indoc!(
		"
		SELECT game_id
		FROM db_cloud.game_configs
		WHERE game_id = ANY($1)
		"
	))
	.bind(game_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?
	.into_iter()
	.map(Into::<backend::cloud::Game>::into)
	.collect::<Vec<_>>();

	Ok(cloud::game_config_get::Response { game_configs })
}
