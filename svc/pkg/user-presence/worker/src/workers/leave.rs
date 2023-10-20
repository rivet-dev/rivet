use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

const DEFAULT_USER_SET_STATUS: i32 = backend::user::Status::Online as i32;

#[worker(name = "user-presence-leave")]
async fn worker(ctx: &OperationContext<user_presence::msg::leave::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let mut redis = ctx.redis_user_presence().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	let user_set_status = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT user_set_status FROM db_user_presence.user_presences WHERE user_id = $1",
	)
	.bind(user_id)
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

	// No changes need to be made if user previously set themselves as invisible. Although we delete the user
	// presence data from redis prior to this point (effectively making the user appear offline), this
	// block of code sends out a status set event which other subscribers can pick up on for event-based code.
	if !matches!(
		unwrap_ref!(backend::user::Status::from_i32(user_set_status)),
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
