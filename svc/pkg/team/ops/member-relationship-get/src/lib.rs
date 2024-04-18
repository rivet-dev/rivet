use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-member-relationship-get")]
async fn handle(
	ctx: OperationContext<team::member_relationship_get::Request>,
) -> GlobalResult<team::member_relationship_get::Response> {
	// Map user pairs
	let query_users = ctx
		.users
		.iter()
		.map(|x| -> GlobalResult<(Uuid, Uuid)> {
			Ok(util::sort::id_pair(
				unwrap_ref!(x.this_user_id).as_uuid(),
				unwrap_ref!(x.other_user_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<(Uuid, Uuid)>>>()?;

	// Query relationships
	let relationships = sql_fetch_all!(
		[ctx, (Uuid, Uuid, Vec<Uuid>,)]
		"
		SELECT
			(q->>0)::UUID AS this_user_id,
			(q->>1)::UUID AS other_user_id,
			ARRAY(
				SELECT this_tm.team_id
				FROM db_team.team_members AS this_tm
				INNER JOIN db_team.team_members AS other_tm ON this_tm.team_id = other_tm.team_id
				WHERE this_tm.user_id = (q->>0)::UUID AND other_tm.user_id = (q->>1)::UUID
			) AS mutual_team_ids
		FROM jsonb_array_elements($1::JSONB) AS q
		",
		serde_json::to_string(&query_users)?,
	)
	.await?;

	let users = relationships
		.into_iter()
		.map(|(this_user_id, other_user_id, team_ids)| {
			team::member_relationship_get::response::User {
				this_user_id: Some(this_user_id.into()),
				other_user_id: Some(other_user_id.into()),
				shared_team_ids: team_ids
					.into_iter()
					.map(Into::<common::Uuid>::into)
					.collect(),
			}
		})
		.collect();

	Ok(team::member_relationship_get::Response { users })
}
