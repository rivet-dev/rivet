use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-follow-request-ignore")]
async fn worker(
	ctx: &OperationContext<user_follow::msg::request_ignore::Message>,
) -> GlobalResult<()> {
	let follower_user_id = unwrap_ref!(ctx.follower_user_id).as_uuid();
	let following_user_id = unwrap_ref!(ctx.following_user_id).as_uuid();

	ensure!(follower_user_id != following_user_id, "cannot follow self");

	sql_execute!(
		[ctx]
		"
		UPDATE db_user_follow.user_follows
		SET ignored = TRUE
		WHERE
			follower_user_id = $1 AND
			following_user_id = $2
		",
		follower_user_id,
		following_user_id,
	)
	.await?;

	msg!([ctx] user_follow::msg::request_ignore_complete(follower_user_id, following_user_id) {
		follower_user_id: Some(follower_user_id.into()),
		following_user_id: Some(following_user_id.into()),
	})
	.await?;

	Ok(())
}
