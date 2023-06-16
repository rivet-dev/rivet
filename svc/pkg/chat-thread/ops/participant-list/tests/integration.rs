use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a_res = op!([ctx] faker_user {}).await.unwrap();
	let user_b_res = op!([ctx] faker_user {}).await.unwrap();

	// Create test direct chat with thread
	let res = op!([ctx] chat_thread_get_or_create_for_topic {
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Direct(
				backend::chat::topic::Direct {
					user_a_id: user_a_res.user_id,
					user_b_id: user_b_res.user_id,
				},
			)),
		}),
	})
	.await
	.unwrap();
	let _thread_id = res.thread_id.unwrap().as_uuid();

	// Fetch the thread ID created by direct chat creation
	let thread = op!([ctx] chat_thread_get_for_topic {
		topics: vec![backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Direct(
				backend::chat::topic::Direct {
					user_a_id: user_a_res.user_id,
					user_b_id: user_b_res.user_id,
				},
			)),
		}],
	})
	.await
	.unwrap()
	.threads
	.first()
	.unwrap()
	.clone();
	let thread_id = thread.thread_id.unwrap().as_uuid();

	// List participants
	let res = op!([ctx] chat_thread_participant_list {
		thread_ids: vec![thread_id.into()],
	})
	.await
	.unwrap();

	// Validate participants
	let thread = res.threads.first().unwrap();
	let user_a_id = user_a_res.user_id.as_ref().unwrap().as_uuid();
	let user_b_id = user_b_res.user_id.as_ref().unwrap().as_uuid();

	assert_eq!(2, thread.participants.len(), "wrong participant count");
	assert!(thread
		.participants
		.iter()
		.any(|p| user_a_id == p.user_id.as_ref().unwrap().as_uuid()));
	assert!(thread
		.participants
		.iter()
		.any(|p| user_b_id == p.user_id.as_ref().unwrap().as_uuid()));
}

// TODO: Test party
