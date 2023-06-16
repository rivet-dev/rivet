use std::{collections::HashMap, time::SystemTime};

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let (user_a_id, thread_ids, tail_ts_map) = setup(&ctx).await;

	// Fetch tail
	let res = op!([ctx] chat_thread_recent_for_user {
		user_id: user_a_id,
		after_ts: None,
	})
	.await
	.unwrap();

	assert_eq!(thread_ids.len(), res.threads.len(), "threads not returned");

	// Check the tail dates
	for thread in &res.threads {
		let thread_id = thread
			.thread
			.as_ref()
			.unwrap()
			.thread_id
			.as_ref()
			.unwrap()
			.as_uuid();
		let tail_ts = tail_ts_map.get(&thread_id).expect("unexpected thread id");
		let msg = thread.tail_message.as_ref().unwrap();

		assert_eq!(
			*tail_ts, msg.send_ts,
			"tail send ts does not match, {}",
			thread_id
		);
	}
}

#[worker_test]
async fn repopulate(ctx: TestCtx) {
	let (user_a_id, thread_ids, tail_ts_map) = setup(&ctx).await;

	// Repopulate tail
	op!([ctx] chat_thread_recent_for_user {
		user_id: user_a_id,
		after_ts: None,
	})
	.await
	.unwrap();

	// Fetch tail
	let res = op!([ctx] chat_thread_recent_for_user {
		user_id: user_a_id,
		after_ts: None,
	})
	.await
	.unwrap();

	assert_eq!(thread_ids.len(), res.threads.len(), "threads not returned");

	// Check the tail dates
	for thread in &res.threads {
		let thread_id = thread
			.thread
			.as_ref()
			.unwrap()
			.thread_id
			.as_ref()
			.unwrap()
			.as_uuid();
		let tail_ts = tail_ts_map.get(&thread_id).expect("unexpected thread id");
		let msg = thread.tail_message.as_ref().unwrap();

		assert_eq!(
			*tail_ts, msg.send_ts,
			"tail send ts does not match, {}",
			thread_id
		);
	}
}

async fn setup(ctx: &TestCtx) -> (Option<common::Uuid>, Vec<Uuid>, HashMap<Uuid, i64>) {
	// Create test threads
	let mut thread_ids = Vec::new();

	let user_a = op!([ctx] faker_user {}).await.unwrap();

	for _ in 0usize..8 {
		let user_b = op!([ctx] faker_user {}).await.unwrap();

		// Create direct chat
		let res = op!([ctx] chat_thread_get_or_create_for_topic {
			topic: Some(backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
					user_a_id: user_a.user_id,
					user_b_id: user_b.user_id,
				}))
			}),
		})
		.await
		.unwrap();

		// Get matching thread
		let thread_res = op!([ctx] chat_thread_get_for_topic {
			topics: vec![backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(
					backend::chat::topic::Direct {
						user_a_id: user_a.user_id,
						user_b_id: user_b.user_id,
					},
				)),
			}],
		})
		.await
		.unwrap();
		let thread_id = thread_res
			.threads
			.first()
			.unwrap()
			.clone()
			.thread_id
			.unwrap()
			.as_uuid();
		thread_ids.push(thread_id);
	}

	// Create messages with randomized dates. The ts must be in the future, since
	// `chat_direct_create` creates messages with the current timestamp.
	let mut rng = rand::thread_rng();
	let mut tail_ts_map = HashMap::<Uuid, i64>::new();
	let min_ts = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_millis() as i64
		+ util::duration::days(1);
	for i in 0..10 {
		let thread_id = thread_ids[i % thread_ids.len()];
		let send_ts = min_ts + rng.gen_range(10..1000);

		// Send messages in batch (wait for a thread update sub)
		let chat_message_id = Uuid::new_v4();
		msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_thread::msg::update(thread_id) {
			chat_message_id: Some(chat_message_id.into()),
			thread_id: Some(thread_id.into()),
			send_ts,
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(
					backend::chat::message_body::Text {
						sender_user_id: Some(Uuid::new_v4().into()),
						body: util::faker::ident(),
					},
				)),
			}),
		})
		.await
		.unwrap();

		// Update tail date if this is higher
		let tail_date = tail_ts_map.entry(thread_id).or_default();
		if send_ts > *tail_date {
			*tail_date = send_ts
		}
	}

	(user_a.user_id, thread_ids, tail_ts_map)
}
