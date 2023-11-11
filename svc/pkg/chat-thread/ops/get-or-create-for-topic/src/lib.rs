use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "chat-thread-get-or-create-for-topic")]
async fn handle(
	ctx: OperationContext<chat_thread::get_or_create_for_topic::Request>,
) -> GlobalResult<chat_thread::get_or_create_for_topic::Response> {
	let topic = unwrap_ref!(ctx.topic);

	// Find the thread ID
	let thread_id = op!([ctx] chat_thread_get_for_topic {
		topics: vec![topic.clone()],
	})
	.await?
	.threads
	.first()
	.and_then(|x| x.thread_id)
	.map(|x| x.as_uuid());

	// Create new thread if needed
	let (thread_id, new_thread) = if let Some(thread_id) = thread_id {
		tracing::info!(?thread_id, "thread already exists");

		(thread_id, false)
	} else {
		// Create new thread
		let request_id = Uuid::new_v4();
		let create_res =
			msg!([ctx] chat_thread::msg::create(request_id) -> chat_thread::msg::create_complete {
				request_id: Some(request_id.into()),
				topic: Some(topic.clone()),
				override_create_ts: ctx.send_ts,
			})
			.await?;
		let thread_id = unwrap_ref!(create_res.thread_id).as_uuid();

		(thread_id, true)
	};

	Ok(chat_thread::get_or_create_for_topic::Response {
		thread_id: Some(thread_id.into()),
		new_thread,
	})
}
