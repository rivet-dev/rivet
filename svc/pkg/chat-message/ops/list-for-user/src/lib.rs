use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct ChatMessage {
	message_id: Uuid,
	thread_id: Uuid,
	send_ts: i64,
	body: Vec<u8>,
}

#[operation(name = "chat-message-list-for-user")]
async fn handle(
	ctx: OperationContext<chat_message::list_for_user::Request>,
) -> GlobalResult<chat_message::list_for_user::Response> {
	let crdb = ctx.crdb().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();
	let direction = unwrap!(
		chat_message::list_for_user::request::QueryDirection::from_i32(ctx.query_direction)
	);

	let messages = match direction {
		chat_message::list_for_user::request::QueryDirection::Before => {
			let mut msgs = sqlx::query_as::<_, ChatMessage>(indoc!(
				"
				SELECT message_id, thread_id, send_ts, body
				FROM db_chat.messages
				WHERE sender_user_id = $1 AND send_ts < $2
				ORDER BY send_ts DESC, message_id DESC
				LIMIT $3
				"
			))
			.bind(user_id)
			.bind(ctx.ts)
			.bind(ctx.count as i64)
			.fetch_all(&crdb)
			.await?;
			msgs.reverse();
			msgs
		}
		chat_message::list_for_user::request::QueryDirection::BeforeAndAfter => {
			sqlx::query_as::<_, ChatMessage>(indoc!(
				"
				SELECT * FROM (
					SELECT message_id, thread_id, send_ts, body
					FROM db_chat.messages
					WHERE sender_user_id = $1 AND send_ts <= $2
					ORDER BY send_ts DESC, message_id DESC
					LIMIT $3
				)

				UNION

				SELECT * FROM (
					SELECT message_id, thread_id, send_ts, body
					FROM db_chat.messages
					WHERE sender_user_id = $1 AND send_ts > $2
					ORDER BY send_ts ASC, message_id ASC
					LIMIT $3
				)

				ORDER BY send_ts ASC, message_id ASC
				"
			))
			.bind(user_id)
			.bind(ctx.ts)
			.bind(ctx.count as i64)
			.fetch_all(&crdb)
			.await?
		}
		chat_message::list_for_user::request::QueryDirection::After => {
			sqlx::query_as::<_, ChatMessage>(indoc!(
				"
				SELECT message_id, thread_id, send_ts, body
				FROM db_chat.messages
				WHERE sender_user_id = $1 AND send_ts > $2
				ORDER BY send_ts ASC, message_id ASC
				LIMIT $3
				"
			))
			.bind(user_id)
			.bind(ctx.ts)
			.bind(ctx.count as i64)
			.fetch_all(&crdb)
			.await?
		}
	};

	let messages = messages
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

	Ok(chat_message::list_for_user::Response { messages })
}
