use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "custom-user-avatar-upload-complete")]
async fn handle(
	ctx: OperationContext<custom_user_avatar::upload_complete::Request>,
) -> GlobalResult<custom_user_avatar::upload_complete::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();
	let upload_id = unwrap_ref!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-user-avatar".into()),
	})
	.await?;

	sql_query!(
		[ctx]
		"
		INSERT INTO db_game_custom_avatar.custom_avatars (game_id, upload_id, create_ts)
		VALUES ($1, $2, $3)
		",
		game_id,
		upload_id,
		ctx.ts(),
	)
	.await?;

	msg!([ctx] game::msg::update(game_id) {
		game_id: ctx.game_id,
	})
	.await?;

	Ok(custom_user_avatar::upload_complete::Response {})
}
