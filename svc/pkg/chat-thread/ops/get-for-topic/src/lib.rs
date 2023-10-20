use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct ThreadRow {
	thread_id: Uuid,
	create_ts: i64,

	team_team_id: Option<Uuid>,
	direct_user_a_id: Option<Uuid>,
	direct_user_b_id: Option<Uuid>,
}

#[operation(name = "chat-thread-get-for-topic")]
async fn handle(
	ctx: OperationContext<chat_thread::get_for_topic::Request>,
) -> GlobalResult<chat_thread::get_for_topic::Response> {
	let crdb = ctx.crdb().await?;

	// Split up rows in to associated IDs
	let mut team_ids = Vec::new();
	let mut direct_user_a_ids = Vec::new();
	let mut direct_user_b_ids = Vec::new();

	for topic in &ctx.topics {
		let kind = unwrap_ref!(topic.kind);

		match kind {
			backend::chat::topic::Kind::Team(team) => {
				team_ids.push(unwrap_ref!(team.team_id).as_uuid())
			}
			backend::chat::topic::Kind::Direct(direct) => {
				let (user_a_id, user_b_id) = util::sort::id_pair(
					unwrap_ref!(direct.user_a_id).as_uuid(),
					unwrap_ref!(direct.user_b_id).as_uuid(),
				);

				direct_user_a_ids.push(user_a_id);
				direct_user_b_ids.push(user_b_id);
			}
		}
	}

	// Query threads
	let threads = sqlx::query_as::<_, ThreadRow>(indoc!(
		"
		SELECT thread_id, create_ts, team_team_id, NULL AS direct_user_a_id, NULL AS direct_user_b_id
		FROM db_chat.threads
		WHERE team_team_id = ANY($1)

		UNION

		SELECT thread_id, create_ts, NULL, direct_user_a_id, direct_user_b_id
		FROM unnest($2, $3) AS direct (user_a_id, user_b_id)
		INNER JOIN db_chat.threads ON
			direct_user_a_id = direct.user_a_id AND
			direct_user_b_id = direct.user_b_id
		"
	))
	.bind(team_ids)
	.bind(direct_user_a_ids)
	.bind(direct_user_b_ids)
	.fetch_all(&crdb)
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
					bail!("missing thread kind data")
				}),
			}),
		})
	})
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(chat_thread::get_for_topic::Response { threads })
}
