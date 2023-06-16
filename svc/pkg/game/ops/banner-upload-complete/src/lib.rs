use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-banner-upload-complete")]
async fn handle(
	ctx: OperationContext<game::banner_upload_complete::Request>,
) -> GlobalResult<game::banner_upload_complete::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let upload_id = internal_unwrap!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-game-banner".into()),
	})
	.await?;

	// Set avatar id
	sqlx::query(indoc!(
		"
		UPDATE games set banner_upload_id = $2
		WHERE game_id = $1
		"
	))
	.bind(game_id)
	.bind(upload_id)
	.execute(&ctx.crdb("db-game").await?)
	.await?;

	msg!([ctx] game::msg::update(game_id) {
		game_id: ctx.game_id,
	})
	.await?;

	Ok(game::banner_upload_complete::Response {})
}
