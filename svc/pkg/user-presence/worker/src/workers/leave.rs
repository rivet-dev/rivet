use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

const DEFAULT_USER_SET_STATUS: i32 = backend::user::Status::Online as i32;

#[worker(name = "user-presence-leave")]
async fn worker(ctx: OperationContext<user_presence::msg::leave::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-user-presence").await?;

	let mut redis = ctx.redis_user_presence().await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	let user_set_status =
		sqlx::query_as::<_, (Option<i64>,)>("SELECT user_set_status FROM user_presences")
			.fetch_optional(&crdb)
			.await?
			.and_then(|x| x.0)
			.map(|x| x as i32)
			.unwrap_or(DEFAULT_USER_SET_STATUS);

	// Remove the user from Redis.
	//
	// Remove the user presence from the register. User may have already been
	// removed by user-presence-gc.
	let pipe = redis::pipe()
		.atomic()
		.unlink(util_user_presence::key::user_presence(user_id))
		.unlink(util_user_presence::key::game_activity(user_id))
		.zrem(
			util_user_presence::key::user_presence_touch(),
			user_id.to_string(),
		)
		.query_async(&mut redis)
		.await?;

	// Clear game activity
	msg!([ctx] user_presence::msg::game_activity_set(user_id) {
		user_id: ctx.user_id,
		game_activity: None,
	})
	.await?;

	// No changes need to be made if user set themselves as invisible
	if !matches!(
		internal_unwrap!(backend::user::Status::from_i32(user_set_status)),
		backend::user::Status::Offline
	) {
		msg!([ctx] user_presence::msg::status_set(user_id) {
			user_id: ctx.user_id,
			status: backend::user::Status::Offline as i32,
			user_set_status: false,
			silent: false,
		})
		.await?;
	}

	Ok(())
}
