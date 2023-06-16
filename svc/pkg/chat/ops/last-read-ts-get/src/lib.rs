use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Thread {
	thread_id: Uuid,
	last_read_ts: i64,
}

impl From<Thread> for chat::last_read_ts_get::response::Thread {
	fn from(value: Thread) -> chat::last_read_ts_get::response::Thread {
		chat::last_read_ts_get::response::Thread {
			thread_id: Some(value.thread_id.into()),
			last_read_ts: value.last_read_ts,
		}
	}
}

#[operation(name = "chat-last-read-ts-get")]
async fn handle(
	ctx: OperationContext<chat::last_read_ts_get::Request>,
) -> GlobalResult<chat::last_read_ts_get::Response> {
	let crdb = ctx.crdb("db-chat").await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let thread_ids = ctx
		.thread_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let threads = sqlx::query_as::<_, Thread>(indoc!(
		"
		SELECT thread_id, last_read_ts
		FROM thread_user_settings
		WHERE user_id = $1 AND thread_id = ANY($2)
		"
	))
	.bind(user_id)
	.bind(&thread_ids)
	.fetch_all(&crdb)
	.await?;

	Ok(chat::last_read_ts_get::Response {
		threads: threads
			.into_iter()
			.map(|thread| Ok(Into::into(thread)))
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
