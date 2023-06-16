use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
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

	msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_message::msg::create_complete {
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

	let new_body = backend::chat::MessageBody {
		kind: Some(backend::chat::message_body::Kind::Deleted(
			backend::chat::message_body::Deleted {
				sender_user_id: Some(user_a_id.into()),
			},
		)),
	};
	msg!([ctx] chat_message::msg::edit(chat_message_id) -> chat_message::msg::edit_complete {
		chat_message_id: Some(chat_message_id.into()),
		body: Some(new_body.clone()),
	})
	.await
	.unwrap();

	let messages_res = op!([ctx] chat_message_list {
		thread_id: Some(thread_id.into()),
		ts: util::timestamp::now(),
		count: 1,
		query_direction: chat_message::list::request::QueryDirection::Before as i32,
	})
	.await
	.unwrap();
	tracing::info!(?messages_res);
	let message = messages_res.messages.first().unwrap();
	let body = message.body.as_ref().unwrap();

	assert_eq!(&new_body, body, "message not edited");
}

// TODO: Check that tail message updated
