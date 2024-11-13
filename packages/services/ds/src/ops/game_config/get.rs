use std::convert::{TryFrom, TryInto};

use chirp_workflow::prelude::*;

use crate::types::{GameConfig, ServerRuntime};

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
	runtime: i64,
}

impl TryFrom<GameConfigRow> for GameConfig {
	type Error = GlobalError;

	fn try_from(value: GameConfigRow) -> GlobalResult<GameConfig> {
		Ok(GameConfig {
			game_id: value.game_id,
			host_networking_enabled: value.host_networking_enabled,
			root_user_enabled: value.root_user_enabled,
			runtime: unwrap!(ServerRuntime::from_repr(value.runtime.try_into()?)),
		})
	}
}

#[operation]
pub async fn ds_game_config_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let rows = sql_fetch_all!(
		[ctx, GameConfigRow]
		"
		SELECT game_id, host_networking_enabled, root_user_enabled, runtime
		FROM db_ds.game_config
		WHERE game_id = ANY($1)
		",
		&input.game_ids,
	)
	.await?;

	let game_configs = input
		.game_ids
		.iter()
		.map(|game_id| {
			if let Some(row) = rows.iter().find(|x| x.game_id == *game_id) {
				row.clone().try_into()
			} else {
				Ok(GameConfig::default(*game_id))
			}
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { game_configs })
}
