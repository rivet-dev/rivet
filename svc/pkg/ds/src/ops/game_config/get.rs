use std::convert::{TryFrom, TryInto};

use chirp_workflow::prelude::*;

use crate::types::{GameClient, GameConfig};

#[derive(Debug, Default)]
pub struct Input {
	pub game_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub game_configs: Vec<GameConfig>,
}

#[derive(sqlx::FromRow)]
struct GameConfigRow {
	game_id: Uuid,
	host_networking_enabled: bool,
	root_user_enabled: bool,
	client: i64,
}

impl TryFrom<GameConfigRow> for GameConfig {
	type Error = GlobalError;

	fn try_from(value: GameConfigRow) -> GlobalResult<GameConfig> {
		Ok(GameConfig {
			game_id: value.game_id,
			host_networking_enabled: value.host_networking_enabled,
			root_user_enabled: value.root_user_enabled,
			client: unwrap!(GameClient::from_repr(value.client.try_into()?)),
		})
	}
}

#[operation]
pub async fn ds_game_config_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let game_configs = sql_fetch_all!(
		[ctx, GameConfigRow]
		"
		SELECT game_id, host_networking_enabled, root_user_enabled, client
		FROM db_ds.game_config
		WHERE game_id = ANY($1)
		",
		&input.game_ids,
	)
	.await?
	.into_iter()
	.map(TryInto::try_into)
	.collect::<GlobalResult<_>>()?;

	Ok(Output { game_configs })
}
