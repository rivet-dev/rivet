use chirp_workflow::prelude::*;

use crate::types::GameClient;

#[derive(Debug, Default)]
pub struct Input {
	pub game_id: Uuid,
	pub host_networking_enabled: Option<bool>,
	pub root_user_enabled: Option<bool>,
	pub client: Option<GameClient>,
}

#[operation]
pub async fn ds_game_config_update(ctx: &OperationCtx, input: &Input) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.game_config
		SET
			host_networking_enabled = COALESCE($2, host_networking_enabled),
			root_user_enabled = COALESCE($3, root_user_enabled),
			client = COALESCE($4, client)
		WHERE game_id = $1
		",
		&input.game_id,
		input.host_networking_enabled,
		input.root_user_enabled,
		input.client.map(|x| x as i32),
	)
	.await?;

	Ok(())
}
