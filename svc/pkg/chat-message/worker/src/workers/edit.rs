use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/edit.lua"));
}

// TODO: Implement editing for `Text` message bodies with an `edited` property (See RIV-460)
#[worker(name = "chat-message-edit")]
async fn worker(ctx: &OperationContext<chat_message::msg::edit::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let chat_message_id = unwrap_ref!(ctx.chat_message_id).as_uuid();
	let body = unwrap_ref!(ctx.body);

	// Encode body
	let mut body_buf = Vec::with_capacity(body.encoded_len());
	body.encode(&mut body_buf)?;

	// Update message
	let (thread_id, send_ts) = sql_fetch_one!(
		[ctx, (Uuid, i64)]
		"
		UPDATE db_chat.messages
		SET body = $2
		WHERE message_id = $1
		RETURNING thread_id, send_ts
		",
		chat_message_id,
		&body_buf,
	)
	.await?;

	// Build chat message
	let message = backend::chat::Message {
		chat_message_id: Some(chat_message_id.into()),
		thread_id: Some(thread_id.into()),
		send_ts,
		body: Some(body.clone()),
	};
	// Encode message
	let mut message_buf = Vec::with_capacity(message.encoded_len());
	message.encode(&mut message_buf)?;

	// Run redis script (updates the thread cache)
	let mut script = REDIS_SCRIPT.prepare_invoke();
	script
		.key(util_chat::key::thread_tail_message(thread_id))
		.arg(chat_message_id.to_string())
		.arg(message_buf.clone())
		.invoke_async::<_, ()>(&mut ctx.redis_cache().await?)
		.await?;

	// Fetch thread participants
	let thread_participants_res = op!([ctx] chat_thread_participant_list {
		thread_ids: vec![thread_id.into()],
	})
	.await?;
	let thread_participants = unwrap!(thread_participants_res.threads.first());

	// Dispatch events
	msg!([ctx] chat_thread::msg::update(thread_id) {
		kind: Some(chat_thread::msg::update::message::Kind::ChatMessage(message.clone())),
	})
	.await?;
	msg!([ctx] chat_message::msg::edit_complete(chat_message_id) {
		thread_id: Some(thread_id.into()),
		chat_message_id: Some(chat_message_id.into()),
		chat_message: Some(message.clone()),
		participant_user_ids: thread_participants
			.participants.iter()
			.map(|p| Ok(unwrap!(p.user_id)))
			.collect::<GlobalResult<Vec<_>>>()?,
	})
	.await?;

	Ok(())
}
