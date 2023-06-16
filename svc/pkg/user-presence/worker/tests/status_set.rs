use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	let status = backend::user::Status::Away;

	let mut user_presence_sub = subscribe!([ctx] user_presence::msg::update(user_id))
		.await
		.unwrap();
	msg!([ctx] user_presence::msg::status_set(user_id) {
		user_id: Some(user_id.into()),
		status: status.into(),
		user_set_status: false,
		silent: false,
	})
	.await
	.unwrap();
	user_presence_sub.next().await.unwrap();

	let mut redis = ctx.redis_user_presence().await.unwrap();
	let redis_status: i32 = redis
		.hget(
			util_user_presence::key::user_presence(user_id),
			util_user_presence::key::user_presence::STATUS,
		)
		.await
		.unwrap();
	assert_eq!(status as i32, redis_status);
}
