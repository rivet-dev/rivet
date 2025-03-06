use chirp_workflow::prelude::*;

#[derive(Debug, Default)]
pub struct Input {
	pub game_id: Uuid,
	pub host_networking_enabled: Option<bool>,
	pub root_user_enabled: Option<bool>,
}

#[operation]
pub async fn ds_game_config_upsert(ctx: &OperationCtx, input: &Input) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_pegboard.game_config (game_id, host_networking_enabled, root_user_enabled)
		SELECT $1, COALESCE($2, false), COALESCE($3, false)
		ON CONFLICT (game_id) DO UPDATE
		SET
			host_networking_enabled = COALESCE($2, EXCLUDED.host_networking_enabled),
			root_user_enabled = COALESCE($3, EXCLUDED.root_user_enabled)
		",
		&input.game_id,
		input.host_networking_enabled,
		input.root_user_enabled,
	)
	.await?;

	Ok(())
}
