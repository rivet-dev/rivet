use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn valid(ctx: TestCtx) {
	let sender_user_id = Uuid::new_v4();

	op!([ctx] chat_message_body_validate {
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::Text(
				backend::chat::message_body::Text {
					sender_user_id: Some(sender_user_id.into()),
					body: "Hello, world!".to_owned(),
				},
			)),
		}),
	})
	.await
	.unwrap();
}

#[worker_test]
async fn invalid(ctx: TestCtx) {
	let sender_user_id = Uuid::new_v4();

	op!([ctx] chat_message_body_validate {
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::Text(
				backend::chat::message_body::Text {
					sender_user_id: Some(sender_user_id.into()),
					body: "".to_owned(),
				},
			)),
		}),
	})
	.await
	.unwrap_err();
}
