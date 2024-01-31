use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "game-user-session-create")]
async fn worker(
	ctx: &OperationContext<game_user::msg::session_create::Message>,
) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let game_user_id = unwrap_ref!(ctx.game_user_id).as_uuid();
	let refresh_jti = unwrap_ref!(ctx.refresh_jti).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_game_user.sessions (session_id, game_user_id, refresh_jti, start_ts)
		VALUES ($1, $2, $3, $4)
		",
		Uuid::new_v4(),
		game_user_id,
		refresh_jti,
		ctx.ts(),
	)
	.await?;

	Ok(())
}
