use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-follow-request-ignore")]
async fn worker(
	ctx: &OperationContext<user_follow::msg::request_ignore::Message>,
) -> Result<(), GlobalError> {
	let follower_user_id = internal_unwrap!(ctx.follower_user_id).as_uuid();
	let following_user_id = internal_unwrap!(ctx.following_user_id).as_uuid();

	internal_assert!(follower_user_id != following_user_id, "cannot follow self");

	sqlx::query(indoc!(
		"
		UPDATE user_follows
		SET ignored = TRUE
		WHERE
			follower_user_id = $1 AND
			following_user_id = $2
		"
	))
	.bind(follower_user_id)
	.bind(following_user_id)
	.execute(&ctx.crdb("db-user-follow").await?)
	.await?;

	msg!([ctx] user_follow::msg::request_ignore_complete(follower_user_id, following_user_id) {
		follower_user_id: Some(follower_user_id.into()),
		following_user_id: Some(following_user_id.into()),
	})
	.await?;

	Ok(())
}
