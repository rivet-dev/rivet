use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-search-update-user-update")]
async fn worker(ctx: &OperationContext<user::msg::update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap_owned!(ctx.user_id);

	msg!([ctx] user::msg::search_update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	Ok(())
}
