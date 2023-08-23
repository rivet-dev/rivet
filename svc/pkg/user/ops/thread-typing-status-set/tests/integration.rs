use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	let thread_id = Uuid::new_v4();
	let topic_key = util_chat::key::typing_statuses(thread_id);
	tracing::info!(?topic_key);

	op!([ctx] user_thread_typing_status_set {
		user_id: Some(user_id.into()),
		thread_id: Some(thread_id.into()),
		status: Some(backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Typing(
				backend::chat::typing_status::Typing {},
			)),
		}),
		no_broadcast: false,
	})
	.await
	.unwrap();

	let res = redis::pipe()
		.atomic()
		.hgetall(topic_key.clone())
		.query_async::<_, Vec<HashMap<String, Vec<u8>>>>(&mut ctx.redis_cache().await.unwrap())
		.await
		.unwrap();
	let res = res.first().unwrap();

	assert!(
		res.contains_key(user_id.to_string().as_str()),
		"user typing status not set"
	);

	op!([ctx] user_thread_typing_status_set {
		user_id: Some(user_id.into()),
		thread_id: Some(thread_id.into()),
		status: Some(backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Idle(
				backend::chat::typing_status::Idle {},
			)),
		}),
		no_broadcast: false,
	})
	.await
	.unwrap();

	let res = redis::pipe()
		.atomic()
		.hgetall(topic_key.clone())
		.query_async::<_, Vec<HashMap<String, Vec<u8>>>>(&mut ctx.redis_cache().await.unwrap())
		.await
		.unwrap();
	let res = res.first().unwrap();

	assert!(
		!res.contains_key(user_id.to_string().as_str()),
		"user typing status not removed"
	);
}

#[worker_test]
async fn expiration(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	let user_id2 = Uuid::new_v4();
	let thread_id = Uuid::new_v4();
	let topic_key = util_chat::key::typing_statuses(thread_id);
	tracing::info!(?topic_key);

	// Insert first user
	op!([ctx] user_thread_typing_status_set {
		user_id: Some(user_id.into()),
		thread_id: Some(thread_id.into()),
		status: Some(backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Typing(
				backend::chat::typing_status::Typing {},
			)),
		}),
		no_broadcast: false,
	})
	.await
	.unwrap();

	tracing::info!("waiting for expiration...");
	tokio::time::sleep(
		tokio::time::Duration::from_secs(util_chat::key::TYPING_STATUS_EXPIRE_DURATION as u64)
			+ tokio::time::Duration::from_secs(1),
	)
	.await;

	// Insert second user
	op!([ctx] user_thread_typing_status_set {
		user_id: Some(user_id2.into()),
		thread_id: Some(thread_id.into()),
		status: Some(backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Typing(
				backend::chat::typing_status::Typing {},
			)),
		}),
		no_broadcast: false,
	})
	.await
	.unwrap();

	let res = redis::pipe()
		.atomic()
		.hgetall(topic_key.clone())
		.query_async::<_, Vec<HashMap<String, Vec<u8>>>>(&mut ctx.redis_cache().await.unwrap())
		.await
		.unwrap();
	let res = res.first().unwrap();

	assert!(
		!res.contains_key(user_id.to_string().as_str()),
		"expired key not removed"
	);
}
