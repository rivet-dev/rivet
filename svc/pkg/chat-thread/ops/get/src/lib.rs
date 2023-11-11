use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("missing thread kind data")]
	MissingThreadKindData,
}

#[derive(sqlx::FromRow)]
struct Thread {
	thread_id: Uuid,
	create_ts: i64,

	team_team_id: Option<Uuid>,
	direct_user_a_id: Option<Uuid>,
	direct_user_b_id: Option<Uuid>,
}

#[operation(name = "chat-thread-get")]
async fn handle(
	ctx: OperationContext<chat_thread::get::Request>,
) -> GlobalResult<chat_thread::get::Response> {
	let crdb = ctx.crdb().await?;

	let thread_ids = ctx
		.thread_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	tracing::info!(?thread_ids, "querying thread ids");

	let threads = sql_fetch_all!(
		[ctx, Thread]
		"
		SELECT 
			thread_id,
			create_ts,
			team_team_id,
			direct_user_a_id,
			direct_user_b_id
		FROM db_chat.threads
		WHERE thread_id = ANY($1)
		",
		&thread_ids,
	)
	.await?
	.into_iter()
	.map(|thread| {
		GlobalResult::Ok(backend::chat::Thread {
			thread_id: Some(thread.thread_id.into()),
			create_ts: thread.create_ts,
			topic: Some(backend::chat::Topic {
				kind: Some(if let Some(team_id) = thread.team_team_id {
					backend::chat::topic::Kind::Team(backend::chat::topic::Team {
						team_id: Some(team_id.into()),
					})
				} else if let (Some(user_a_id), Some(user_b_id)) =
					(thread.direct_user_a_id, thread.direct_user_b_id)
				{
					backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
						user_a_id: Some(user_a_id.into()),
						user_b_id: Some(user_b_id.into()),
					})
				} else {
					return Err(Error::MissingThreadKindData.into());
				}),
			}),
		})
	})
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(chat_thread::get::Response { threads })
}
