use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn team(ctx: TestCtx) {
	let user_a = op!([ctx] faker_user {}).await.unwrap();
	let user_b = op!([ctx] faker_user {}).await.unwrap();

	let request_id = Uuid::new_v4();
	let res =
		msg!([ctx] chat_thread::msg::create(request_id) -> chat_thread::msg::create_complete {
			request_id: Some(request_id.into()),
			topic: Some(backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
					user_a_id: user_a.user_id,
					user_b_id: user_b.user_id,
				}))
			}),
			..Default::default()
		})
		.await
		.unwrap();
	let thread_id = res.thread_id.unwrap().as_uuid();

	let topic = backend::chat::Topic {
		kind: Some(backend::chat::topic::Kind::Direct(
			backend::chat::topic::Direct {
				user_a_id: user_a.user_id,
				user_b_id: user_b.user_id,
			},
		)),
	};

	// Create first message
	let chat_message_id = Uuid::new_v4();
	let create_res = op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(chat_message_id.into()),
		topic: Some(topic.clone()),
		send_ts: 0,
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::Text(
				backend::chat::message_body::Text {
					sender_user_id: Some(Uuid::new_v4().into()),
					body: "Hello, world!".to_owned(),
				},
			)),
		}),
	})
	.await
	.unwrap();
	let thread_id = create_res.thread_id.unwrap().as_uuid();

	// Validate that subsequent messages have the same thread ID
	for _ in 0..4 {
		let chat_message_id = Uuid::new_v4();
		let create_res = op!([ctx] chat_message_create_with_topic {
			chat_message_id: Some(chat_message_id.into()),
			topic: Some(topic.clone()),
			send_ts: 0,
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(
					backend::chat::message_body::Text {
						sender_user_id: Some(Uuid::new_v4().into()),
						body: "Hello, world!".to_owned(),
					},
				)),
			}),
		})
		.await
		.unwrap();
		assert_eq!(
			thread_id,
			create_res.thread_id.unwrap().as_uuid(),
			"thread id does not match"
		);
	}
}
