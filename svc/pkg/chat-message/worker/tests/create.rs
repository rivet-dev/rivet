use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn user_group(ctx: TestCtx) {
	let chat_message_id = Uuid::new_v4();

	let user_a_id = Uuid::new_v4();
	let user_b_id = Uuid::new_v4();

	let res = op!([ctx] chat_thread_get_or_create_for_topic {
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Direct(
				backend::chat::topic::Direct {
					user_a_id: Some(user_a_id.into()),
					user_b_id: Some(user_b_id.into()),
				},
			)),
		}),
	})
	.await
	.unwrap();
	let thread_id = res.thread_id.unwrap().as_uuid();

	// Create message
	msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_thread::msg::update(thread_id) {
		chat_message_id: Some(chat_message_id.into()),
		thread_id: Some(thread_id.into()),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::Text(
				backend::chat::message_body::Text {
					sender_user_id: Some(user_a_id.into()),
					body: "Hello, world!".to_owned(),
				},
			)),
		}),
	})
	.await
	.unwrap();

	// thread_event.await.unwrap();
	// futures_util::future::try_join_all(thread_update_events)
	// .await
	// .unwrap();
}

// TODO: Test sending to teams
// TODO: Test sending to parties
