use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-search-update-user-follow-create")]
async fn worker(ctx: OperationContext<user_follow::msg::create::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap_owned!(ctx.following_user_id).as_uuid();

	msg!([ctx] user::msg::search_update(user_id) {
		user_id: ctx.following_user_id,
	})
	.await?;

	Ok(())
}
