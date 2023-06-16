use chirp_worker::prelude::*;
use redis::AsyncCommands;

use ::user_presence_gc::run_from_env;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let pools = rivet_pools::from_env("user-presence-gc-test")
		.await
		.unwrap();
	let ctx = TestCtx::from_env("basic").await.unwrap();
	let mut redis = ctx.redis_user_presence().await.unwrap();

	let user_id = Uuid::new_v4();
	op!([ctx] user_presence_touch {
		user_id: Some(user_id.into()),
	})
	.await
	.unwrap();

	// Check the user isn't removed immediately
	{
		run_from_env(util::timestamp::now(), pools.clone())
			.await
			.unwrap();

		let expire_ts = redis
			.zscore::<_, _, Option<u64>>(
				util_user_presence::key::user_presence_touch(),
				user_id.to_string(),
			)
			.await
			.unwrap();
		assert!(expire_ts.is_some(), "user already removed");
	}

	// Make the GC remove the player
	{
		run_from_env(
			util::timestamp::now() + util_user_presence::USER_PRESENCE_TTL,
			pools.clone(),
		)
		.await
		.unwrap();

		let expire_ts = redis
			.zscore::<_, _, Option<u64>>(
				util_user_presence::key::user_presence_touch(),
				user_id.to_string(),
			)
			.await
			.unwrap();
		assert!(expire_ts.is_none(), "user not removed");
	}
}
