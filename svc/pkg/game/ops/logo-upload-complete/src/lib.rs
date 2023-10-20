use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-logo-upload-complete")]
async fn handle(
	ctx: OperationContext<game::logo_upload_complete::Request>,
) -> GlobalResult<game::logo_upload_complete::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();
	let upload_id = unwrap_ref!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-game-logo".into()),
	})
	.await?;

	// Set avatar id
	sqlx::query(indoc!(
		"
		UPDATE db_game.games
		SET logo_upload_id = $2
		WHERE game_id = $1
		"
	))
	.bind(game_id)
	.bind(upload_id)
	.execute(&ctx.crdb().await?)
	.await?;

	msg!([ctx] game::msg::update(game_id) {
		game_id: ctx.game_id,
	})
	.await?;

	Ok(game::logo_upload_complete::Response {})
}
