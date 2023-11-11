use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "chat-message-create-with-topic")]
async fn handle(
	ctx: OperationContext<chat_message::create_with_topic::Request>,
) -> GlobalResult<chat_message::create_with_topic::Response> {
	let topic = unwrap_ref!(ctx.topic);
	let chat_message_id = unwrap_ref!(ctx.chat_message_id).as_uuid();

	let get_thread_res = op!([ctx] chat_thread_get_or_create_for_topic {
		send_ts: Some(ctx.send_ts),
		topic: Some(topic.clone()),
	})
	.await?;
	let thread_id = unwrap_ref!(get_thread_res.thread_id).as_uuid();

	// Don't create message if request is attempting to create a `ChatCreate` message for a thread that
	// hasn't been made yet, the above block of code handles that automatically
	let is_chat_create = matches!(
		ctx.body.as_ref().and_then(|body| body.kind.as_ref()),
		Some(backend::chat::message_body::Kind::ChatCreate(
			backend::chat::message_body::ChatCreate { .. },
		))
	);
	if get_thread_res.new_thread && !is_chat_create {
		// Create the message
		msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) {
			chat_message_id: ctx.chat_message_id,
			thread_id: Some(thread_id.into()),
			send_ts: ctx.send_ts.saturating_sub(1),
			body: ctx.body.clone(),
		})
		.await?;
	}

	Ok(chat_message::create_with_topic::Response {
		thread_id: Some(thread_id.into()),
	})
}
