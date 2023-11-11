use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct CustomAvatar {
	upload_id: Uuid,
}

#[operation(name = "custom-user-avatar-list-for-game")]
async fn handle(
	ctx: OperationContext<custom_user_avatar::list_for_game::Request>,
) -> GlobalResult<custom_user_avatar::list_for_game::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let custom_avatars = sql_fetch_all!(
		[ctx, CustomAvatar]
		"
		SELECT upload_id
		FROM db_game_custom_avatar.custom_avatars
		WHERE game_id = $1
		",
		game_id,
	)
	.await?;

	Ok(custom_user_avatar::list_for_game::Response {
		custom_avatars: custom_avatars
			.into_iter()
			.map(
				|custom_avatar| custom_user_avatar::list_for_game::response::CustomAvatar {
					upload_id: Some(custom_avatar.upload_id.into()),
				},
			)
			.collect::<Vec<_>>(),
	})
}
