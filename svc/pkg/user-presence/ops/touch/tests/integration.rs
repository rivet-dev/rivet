use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let mut redis = ctx.redis_user_presence().await.unwrap();

	let res = op!([ctx] faker_user {
	})
	.await
	.unwrap();
	let user_id = *res.user_id.unwrap();

	{
		// Test arrive
		let mut arrive_sub = subscribe!([ctx] user_presence::msg::arrive(user_id))
			.await
			.unwrap();
		op!([ctx] user_presence_touch {
			user_id: res.user_id,
		})
		.await
		.unwrap();
		arrive_sub.next().await.unwrap();

		// Test it doesn't send arrive message again
		op!([ctx] user_presence_touch {
			user_id: res.user_id,
		})
		.await
		.unwrap();
		tokio::time::timeout(Duration::from_secs(1), arrive_sub.next())
			.await
			.expect_err("should not have received second arrive message");
	}

	// Force user to leave
	msg!([ctx] user_presence::msg::leave(user_id) -> user_presence::msg::status_set {
		user_id: res.user_id,
	})
	.await
	.unwrap();

	let score: Option<i64> = redis
		.zscore(
			util_user_presence::key::user_presence_touch(),
			user_id.to_string(),
		)
		.await
		.unwrap();
	assert!(score.is_none(), "user presence not removed");

	// Make sure user status updates when arrive again
	//
	// We have to make another `arrive_sub` since we test the `next` with a
	// timeout before.
	{
		let mut arrive_sub = subscribe!([ctx] user_presence::msg::arrive(user_id))
			.await
			.unwrap();
		op!([ctx] user_presence_touch {
			user_id: res.user_id,
		})
		.await
		.unwrap();
		arrive_sub.next().await.unwrap();
	}
}
