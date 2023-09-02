use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;

#[worker(name = "user-presence-game-activity-set")]
async fn worker(
	ctx: &OperationContext<user_presence::msg::game_activity_set::Message>,
) -> GlobalResult<()> {
	let mut redis = ctx.redis_user_presence().await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	if let Some(game_activity) = &ctx.game_activity {
		use util_user_presence::key;

		let game_id = internal_unwrap!(game_activity.game_id).as_uuid();

		// TODO: Validate user is online atomically
		redis::cmd("HSET")
			.arg(key::game_activity(user_id))
			.arg(key::game_activity::USER_ID)
			.arg(user_id.to_string())
			.arg(key::game_activity::GAME_ID)
			.arg(game_id.to_string())
			.arg(key::game_activity::UPDATE_TS)
			.arg(ctx.ts())
			.arg(key::game_activity::MESSAGE)
			.arg(&game_activity.message)
			.arg(key::game_activity::PUBLIC_METADATA_JSON)
			.arg(&game_activity.public_metadata)
			.arg(key::game_activity::FRIEND_METADATA_JSON)
			.arg(&game_activity.friend_metadata)
			.query_async::<_, ()>(&mut redis)
			.await?;
	} else {
		redis
			.unlink(util_user_presence::key::game_activity(user_id))
			.await?;
	}

	// TODO: Don't publish if user status is set to offline
	msg!([ctx] user_presence::msg::update(user_id) {
		user_id: Some(user_id.into()),
		update_ts: ctx.ts(),
		kind: ctx.game_activity
			.clone()
			.map(user_presence::msg::update::message::Kind::GameActivity),
	})
	.await?;

	Ok(())
}
