use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-admin-set")]
async fn worker(ctx: &OperationContext<user::msg::admin_set::Message>) -> GlobalResult<()> {
	let user_id = unwrap!(ctx.user_id);

	// TODO: Don't run if already admin

	sql_execute!(
		[ctx]
		"
		UPDATE db_user.users
		SET
			is_admin = true
		WHERE user_id = $1
		",
		*user_id,
	)
	.await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: Some(user_id),
	})
	.await?;

	Ok(())
}
