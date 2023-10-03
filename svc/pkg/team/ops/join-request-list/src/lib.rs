use proto::backend::pkg::*;
use rivet_operation::prelude::*;

// TEMP:
const MAX_JOIN_REQUESTS: i64 = 256;

#[derive(sqlx::FromRow)]
struct JoinRequest {
	team_id: Uuid,
	user_id: Uuid,
	ts: i64,
}

#[operation(name = "team-join-request-list")]
async fn handle(
	ctx: OperationContext<team::join_request_list::Request>,
) -> GlobalResult<team::join_request_list::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch all members
	let join_requests: Vec<JoinRequest> = sqlx::query_as(indoc!(
		"
		SELECT team_id, user_id, ts
		FROM db_team.join_requests
		WHERE team_id = ANY($1)
		LIMIT $2
		"
	))
	.bind(&team_ids)
	.bind(MAX_JOIN_REQUESTS)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	// Group in to teams
	let teams = team_ids
		.iter()
		.map(|team_id| team::join_request_list::response::Team {
			team_id: Some((*team_id).into()),
			join_requests: join_requests
				.iter()
				.filter(|join_request| join_request.team_id == *team_id)
				.map(
					|join_request| team::join_request_list::response::JoinRequest {
						user_id: Some(join_request.user_id.into()),
						ts: join_request.ts,
					},
				)
				.collect::<Vec<_>>(),
		})
		.collect::<Vec<_>>();

	Ok(team::join_request_list::Response { teams })
}
