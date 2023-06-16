use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let mut chat_message_ids = Vec::new();

	for _i in 0..3 {
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
			chat_message_id: Some((chat_message_id).into()),
			thread_id: Some((thread_id).into()),
			send_ts: util::timestamp::now(),
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(backend::chat::message_body::Text {
					sender_user_id: Some(user_a_id.into()),
					body: "abc123".to_owned(),
				}))
			}),
		})
		.await.unwrap();

		chat_message_ids.push(chat_message_id);
	}

	// Nonexistent
	chat_message_ids.push(Uuid::new_v4());

	let res = op!([ctx] chat_message_get {
		chat_message_ids: chat_message_ids
			.into_iter()
			.map(Into::into)
			.collect::<Vec<_>>(),
	})
	.await
	.unwrap();

	assert_eq!(3, res.messages.len(), "wrong message count");
}
