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

#[operation(name = "chat-thread-list-for-participant")]
async fn handle(
	ctx: OperationContext<chat_thread::list_for_participant::Request>,
) -> GlobalResult<chat_thread::list_for_participant::Response> {
	let crdb = ctx.crdb().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	// Fetch teams the user participates in
	let team_list_res = op!([ctx] user_team_list {
		user_ids: vec![user_id.into()],
	})
	.await?;
	let team_ids = unwrap!(team_list_res.users.first())
		.teams
		.iter()
		.flat_map(|x| x.team_id)
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();

	// Query threads
	let threads = sqlx::query_as::<_, ThreadRow>(indoc!(
		"
		SELECT thread_id, create_ts, team_team_id, NULL AS direct_user_a_id, NULL AS direct_user_b_id
		FROM db_chat.threads
		WHERE team_team_id = ANY($1)

		UNION

		SELECT thread_id, create_ts, NULL, direct_user_a_id, direct_user_b_id
		FROM db_chat.threads
		WHERE
			direct_user_a_id = $2 OR
			direct_user_b_id = $2
		"
	))
	.bind(team_ids)
	.bind(user_id)
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

	Ok(chat_thread::list_for_participant::Response { threads })
}
