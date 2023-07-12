use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "game-user-session-create")]
async fn worker(
	ctx: &OperationContext<game_user::msg::session_create::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-game-user").await?;

	let game_user_id = internal_unwrap!(ctx.game_user_id).as_uuid();
	let refresh_jti = internal_unwrap!(ctx.refresh_jti).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO sessions (session_id, game_user_id, refresh_jti, start_ts)
		VALUES ($1, $2, $3, $4)
		"
	))
	.bind(Uuid::new_v4())
	.bind(game_user_id)
	.bind(refresh_jti)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	Ok(())
}
