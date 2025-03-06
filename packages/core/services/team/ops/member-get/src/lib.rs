use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct TeamMember {
	team_id: Uuid,
	user_id: Uuid,
	join_ts: i64,
}

#[operation(name = "team-member-get")]
async fn handle(
	ctx: OperationContext<team::member_get::Request>,
) -> GlobalResult<team::member_get::Response> {
	let members = ctx
		.members
		.iter()
		.map(|members| {
			Ok((
				unwrap_ref!(members.team_id).as_uuid(),
				unwrap_ref!(members.user_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<(Uuid, Uuid)>>>()?;

	let members: Vec<TeamMember> = sql_fetch_all!(
		[ctx, TeamMember]
		"
		SELECT 
			tm.team_id, tm.user_id, tm.join_ts
		FROM (
			SELECT (member->>0)::UUID AS team_id, (member->>1)::UUID AS user_id
			FROM jsonb_array_elements($1) AS member
		) AS q
		INNER JOIN db_team.team_members AS tm
		ON 
			tm.team_id = q.team_id AND
			tm.user_id = q.user_id
		",
		serde_json::to_value(members)?,
	)
	.await?;

	let members = members
		.iter()
		.map(|member| team::member_get::response::TeamMember {
			team_id: Some(member.team_id.into()),
			user_id: Some(member.user_id.into()),
			join_ts: member.join_ts,
		})
		.collect::<Vec<_>>();

	Ok(team::member_get::Response { members })
}
