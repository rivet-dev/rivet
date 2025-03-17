use std::convert::{TryFrom, TryInto};

use chirp_workflow::prelude::*;

use crate::types::GameConfig;

#[derive(Debug, Default)]
pub struct Input {
	pub game_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub game_configs: Vec<GameConfig>,
}

#[derive(sqlx::FromRow, Clone)]
struct GameConfigRow {
	game_id: Uuid,
	host_networking_enabled: bool,
	root_user_enabled: bool,
}

impl TryFrom<GameConfigRow> for GameConfig {
	type Error = GlobalError;

	fn try_from(value: GameConfigRow) -> GlobalResult<GameConfig> {
		Ok(GameConfig {
			game_id: value.game_id,
			host_networking_enabled: value.host_networking_enabled,
			root_user_enabled: value.root_user_enabled,
		})
	}
}

#[operation]
pub async fn pegboard_game_config_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let game_configs = ctx
		.cache()
		.fetch_all_json("pegboard.game_config", input.game_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, game_ids| {
				let ctx = ctx.clone();
				async move {
					let rows = sql_fetch_all!(
						[ctx, GameConfigRow]
						"
						SELECT game_id, host_networking_enabled, root_user_enabled
						FROM db_pegboard2.game_config
						WHERE game_id = ANY($1)
						",
						&game_ids,
					)
					.await?;

					for game_id in game_ids {
						let game_config =
							if let Some(row) = rows.iter().find(|x| x.game_id == game_id) {
								row.clone().try_into()?
							} else {
								GameConfig::default(game_id)
							};

						cache.resolve(&game_id, game_config);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(Output { game_configs })
}
