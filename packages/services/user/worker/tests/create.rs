use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn create(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	tracing::info!(%user_id);
	msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
		user_id: Some(user_id.into()),
		namespace_id: None,
	})
	.await
	.unwrap();

	let (exists,): (bool,) =
		sqlx::query_as("SELECT EXISTS (SELECT 1 FROM db_user.users WHERE user_id = $1)")
			.bind(user_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
	assert!(exists, "user not created");
}

// // MARK: Stress tests:
// #[worker_test]
// async fn stress(ctx: TestCtx) {
// 	let mut idx = 0;
// 	let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
// 	for _ in 0..100_000 {
// 		interval.tick().await;

// 		tracing::warn!(?idx);
// 		idx += 1;

// 		let user_id = Uuid::new_v4();
// 		tracing::info!(%user_id);
// 		msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
// 			user_id: Some(user_id.into()),
// 			namespace_id: None,
// 		})
// 		.await
// 		.unwrap();

// 		let (exists,): (bool,) =
// 			sqlx::query_as("SELECT EXISTS (SELECT 1 FROM db_user.users WHERE user_id = $1)")
// 				.bind(&user_id)
// 				.fetch_one(&ctx.crdb().await.unwrap())
// 				.await
// 				.unwrap();
// 		assert!(exists, "user not created");
// 	}
// }

// #[worker_test]
// async fn stress_slow(ctx: TestCtx) {
// 	let mut idx = 0;
// 	loop {
// 		tokio::time::sleep(std::time::Duration::from_secs(1)).await;

// 		tracing::warn!(?idx);
// 		idx += 1;

// 		let user_id = Uuid::new_v4();
// 		tracing::info!(%user_id);
// 		msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
// 			user_id: Some(user_id.into()),
// 			namespace_id: None,
// 		})
// 		.await
// 		.unwrap();

// 		let (exists,): (bool,) =
// 			sqlx::query_as("SELECT EXISTS (SELECT 1 FROM db_user.users WHERE user_id = $1)")
// 				.bind(&user_id)
// 				.fetch_one(&ctx.crdb().await.unwrap())
// 				.await
// 				.unwrap();
// 		assert!(exists, "user not created");
// 	}
// }

// #[worker_test]
// async fn stress_msg_reply(ctx: TestCtx) {
// 	use std::sync::{
// 		atomic::{AtomicI64, Ordering},
// 		Arc,
// 	};
// 	use tokio::{
// 		task::JoinSet,
// 		time::{Duration, Instant},
// 	};

// 	let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
// 	let in_progress_counter = Arc::new(AtomicI64::new(0));
// 	let mut join_set = JoinSet::new();
// 	for idx in 0..10_000 {
// 		// interval.tick().await;

// 		let in_progress_counter = in_progress_counter.clone();

// 		let in_progress = in_progress_counter.fetch_add(1, Ordering::Relaxed);
// 		tracing::info!(?idx, ?in_progress, "start idx");

// 		let start = Instant::now();
// 		let client = ctx.chirp().clone();
// 		join_set.spawn(async move {
// 			let user_id = Uuid::new_v4();
// 			tracing::info!(%user_id);
// 			msg!([client] user::msg::create(user_id) -> user::msg::create_complete {
// 				user_id: Some(user_id.into()),
// 				namespace_id: None,
// 			})
// 			.await
// 			.unwrap();

// 			let in_progress = in_progress_counter.fetch_add(-1, Ordering::Relaxed);
// 			tracing::info!(?idx, ?in_progress, dt = %(Instant::now() - start).as_secs_f32(), "finish idx");
// 		});
// 	}

// 	while let Some(res) = join_set.join_next().await {
// 		res.unwrap();
// 	}

// 	tracing::info!("complete");
// }
