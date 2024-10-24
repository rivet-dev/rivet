use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct TeamMember {
	team_id: Uuid,
	user_id: Uuid,
	join_ts: i64,
}

#[operation(name = "team-member-list")]
async fn handle(
	ctx: OperationContext<team::member_list::Request>,
) -> GlobalResult<team::member_list::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	tracing::info!(anchor=?ctx.anchor, limit=?ctx.limit);

	let members = sql_fetch_all!(
		[ctx, TeamMember]
		"
		SELECT team_id, user_id, join_ts
		FROM db_team.team_members
		WHERE
			team_id = ANY($1)
			AND join_ts < $2
		ORDER BY join_ts ASC
		LIMIT $3
		",
		&team_ids,
		ctx.anchor.unwrap_or(i64::MAX),
		ctx.limit.unwrap_or(128) as i64,
	)
	.await?;

	// Group in to teams
	let teams = team_ids
		.iter()
		.map(|team_id| {
			let members = members
				.iter()
				.filter(|member| member.team_id == *team_id)
				.collect::<Vec<_>>();

			let anchor = match (members.last(), ctx.limit) {
				(Some(last_member), Some(limit)) if members.len() >= limit as usize => {
					Some(last_member.join_ts)
				}
				_ => None,
			};

			team::member_list::response::Team {
				team_id: Some((*team_id).into()),
				members: members
					.into_iter()
					.map(|member| team::member_list::response::TeamMember {
						user_id: Some(member.user_id.into()),
						join_ts: member.join_ts,
					})
					.collect::<Vec<_>>(),
				anchor,
			}
		})
		.collect::<Vec<_>>();

	Ok(team::member_list::Response { teams })
}
