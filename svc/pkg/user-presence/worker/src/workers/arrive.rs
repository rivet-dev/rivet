use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

const DEFAULT_USER_SET_STATUS: i32 = backend::user::Status::Online as i32;

#[worker(name = "user-presence-arrive")]
async fn worker(ctx: &OperationContext<user_presence::msg::arrive::Message>) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	let user_set_status = sql_fetch_optional!(
		[ctx, (Option<i64>,)]
		"SELECT user_set_status FROM db_user_presence.user_presences WHERE user_id = $1",
		user_id,
	)
	.await?
	.and_then(|x| x.0)
	.map(|x| x as i32)
	.unwrap_or(DEFAULT_USER_SET_STATUS);

	// No changes need to be made if user set themselves as invisible. Otherwise set to user selected status.
	if !matches!(
		unwrap_ref!(backend::user::Status::from_i32(user_set_status)),
		backend::user::Status::Offline
	) {
		msg!([ctx] user_presence::msg::status_set(user_id) {
			user_id: ctx.user_id,
			status: user_set_status,
			user_set_status: false,
			silent: ctx.silent,
		})
		.await?;
	}

	Ok(())
}
