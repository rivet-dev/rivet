use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct ChatMessage {
	message_id: Uuid,
	thread_id: Uuid,
	send_ts: i64,
	body: Vec<u8>,
}

#[operation(name = "chat-message-get")]
async fn handle(
	ctx: OperationContext<chat_message::get::Request>,
) -> GlobalResult<chat_message::get::Response> {
	let crdb = ctx.crdb().await?;

	let chat_message_ids = ctx
		.chat_message_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let messages = sqlx::query_as::<_, ChatMessage>(indoc!(
		"
		SELECT message_id, thread_id, send_ts, body 
		FROM db_chat.messages
		WHERE message_id = ANY($1)
		"
	))
	.bind(&chat_message_ids)
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(|message| {
		let body = backend::chat::MessageBody::decode(message.body.as_slice())?;

		Ok(backend::chat::Message {
			chat_message_id: Some(message.message_id.into()),
			thread_id: Some(message.thread_id.into()),
			send_ts: message.send_ts,
			body: Some(body),
		})
	})
	.collect::<GlobalResult<Vec<backend::chat::Message>>>()?;

	Ok(chat_message::get::Response { messages })
}
