use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Clone, sqlx::FromRow)]
struct Session {
	user_id: Uuid,
	namespace_id: Uuid,
	start_ts: i64,
}

#[operation(name = "game-user-recent-session-list")]
async fn handle(
	ctx: OperationContext<game_user::recent_session_list::Request>,
) -> GlobalResult<game_user::recent_session_list::Response> {
	let _crdb = ctx.crdb().await?;

	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch all recent sessions for users
	let session_rows = sql_fetch_all!(
		[ctx, Session]
		"
		SELECT gu.user_id, gu.namespace_id, max(s.start_ts) AS start_ts
		FROM (
			SELECT game_user_id, user_id, namespace_id
			FROM db_game_user.game_users
			WHERE user_id = ANY($1)
		) gu
		INNER JOIN LATERAL (
			SELECT start_ts
			FROM db_game_user.sessions AS s
			WHERE s.game_user_id = gu.game_user_id
			ORDER BY start_ts DESC
			LIMIT 1
		) s ON true
		GROUP BY gu.user_id, gu.namespace_id
		",
		&user_ids,
	)
	.await?;

	// Aggregate by user
	let users = user_ids
		.iter()
		.map(|user_id| {
			let mut sessions = session_rows
				.iter()
				.filter(|x| x.user_id == *user_id)
				.map(
					|session| game_user::recent_session_list::response::Session {
						namespace_id: Some(session.namespace_id.into()),
						start_ts: session.start_ts,
					},
				)
				.collect::<Vec<_>>();
			sessions.sort_by_key(|x| -x.start_ts);

			game_user::recent_session_list::response::User {
				user_id: Some((*user_id).into()),
				sessions,
			}
		})
		.collect();

	Ok(game_user::recent_session_list::Response { users })
}
