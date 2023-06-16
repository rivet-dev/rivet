use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = res.user_id.unwrap().as_uuid();

	msg!([ctx] user_presence::msg::status_set(user_id) {
		user_id: res.user_id,
		status: backend::user::Status::Online as i32,
		user_set_status: false,
		silent: false,
	})
	.await
	.unwrap();

	msg!([ctx] user_presence::msg::leave(user_id) -> user_presence::msg::update {
		user_id: res.user_id,
	})
	.await
	.unwrap();

	let mut redis = ctx.redis_user_presence().await.unwrap();
	let redis_status: i32 = redis
		.hget(
			util_user_presence::key::user_presence(user_id),
			util_user_presence::key::user_presence::STATUS,
		)
		.await
		.unwrap();
	assert_eq!(
		backend::user::Status::Offline as i32,
		redis_status,
		"status did not set to offline"
	);
}
