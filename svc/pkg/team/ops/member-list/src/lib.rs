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
) -> Result<team::member_list::Response, GlobalError> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let query_string = formatdoc!(
		"
		SELECT team_id, user_id, join_ts
		FROM team_members
		WHERE
			team_id = ANY($1)
			{join}
		ORDER BY join_ts ASC
		{limit}
		",
		join = ctx.anchor.map(|_| "AND join_ts < $2").unwrap_or_default(),
		limit = ctx
			.limit
			.map(|_| format!("LIMIT ${}", if ctx.anchor.is_some() { 3 } else { 2 }))
			.unwrap_or_default()
	);
	let query = sqlx::query_as(&query_string).bind(&team_ids);

	// Bind offset anchor
	let query = if let Some(offset) = ctx.anchor {
		query.bind(offset)
	} else {
		query
	};

	// Bind limit
	let query = if let Some(limit) = ctx.limit {
		query.bind(limit as i64)
	} else {
		query
	};

	tracing::info!(anchor=?ctx.anchor, limit=?ctx.limit, query_string);

	// Fetch all members
	let members: Vec<TeamMember> = query.fetch_all(&ctx.crdb("db-team").await?).await?;

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
