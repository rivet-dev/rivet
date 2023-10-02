use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "chat-thread-unread-count")]
async fn handle(
	ctx: OperationContext<chat_thread::unread_count::Request>,
) -> GlobalResult<chat_thread::unread_count::Response> {
	let crdb = ctx.crdb().await?;

	let thread_ids = ctx
		.thread_ids
		.iter()
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();

	// Fetch missing last read timestamps that aren't already provided by the
	// request
	let missing_read_ts = ctx
		.thread_ids
		.clone()
		.into_iter()
		.filter(|id| {
			!ctx.read_ts_threads
				.iter()
				.any(|rtt| rtt.thread_id.as_ref() == Some(id))
		})
		.collect::<Vec<_>>();
	let missing_read_ts_res = if !missing_read_ts.is_empty() {
		op!([ctx] chat_last_read_ts_get {
			user_id: Some(*internal_unwrap!(ctx.user_id)),
			thread_ids: missing_read_ts,
		})
		.await?
		.threads
	} else {
		Vec::new()
	};

	// Merge read timestamps
	let read_ts_threads = ctx
		.read_ts_threads
		.clone()
		.into_iter()
		.chain(missing_read_ts_res.into_iter().map(|v| {
			chat_thread::unread_count::request::ReadTsThread {
				thread_id: v.thread_id,
				last_read_ts: v.last_read_ts,
			}
		}))
		.collect::<Vec<_>>();

	// Fetch unread count
	let query_thread_ids = read_ts_threads
		.iter()
		.filter_map(|x| x.thread_id)
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();
	let query_read_ts = read_ts_threads
		.iter()
		.map(|x| x.last_read_ts)
		.collect::<Vec<_>>();
	let threads = sqlx::query_as::<_, (Uuid, i64)>(indoc!(
		"
		SELECT thread_id, (
			SELECT COUNT(*)
			FROM db_chat.messages
			WHERE thread_id = query.thread_id AND send_ts > query.last_read_ts
			LIMIT 100
		)
		FROM unnest($1::UUID[], $2::INT[]) query (thread_id, last_read_ts)
		"
	))
	.bind(query_thread_ids)
	.bind(query_read_ts)
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(
		|(thread_id, count)| chat_thread::unread_count::response::ThreadTail {
			thread_id: Some(thread_id.into()),
			unread_count: count as u64,
		},
	)
	.collect::<Vec<_>>();

	// Does not include threads with no messages in result
	Ok(chat_thread::unread_count::Response { threads })
}
