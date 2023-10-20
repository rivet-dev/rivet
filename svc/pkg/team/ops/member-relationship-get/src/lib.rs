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
			Ok((
				unwrap_ref!(x.this_user_id).as_uuid(),
				unwrap_ref!(x.other_user_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<(Uuid, Uuid)>>>()?;

	// Query relationships
	let relationships = sqlx::query_as::<_, (Vec<Uuid>,)>(&formatdoc!(
		"
		SELECT
			ARRAY(
				SELECT this_tm.team_id
				FROM db_team.team_members AS this_tm
				INNER JOIN db_team.team_members AS other_tm ON this_tm.team_id = other_tm.team_id
				WHERE this_tm.user_id = (q->>0)::UUID AND other_tm.user_id = (q->>1)::UUID
			) AS mutual_team_ids
		FROM jsonb_array_elements($1::JSONB) AS q
		"
	))
	.bind(serde_json::to_string(&query_users)?)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let users = relationships
		.iter()
		.map(
			|(team_ids,)| team::member_relationship_get::response::User {
				shared_team_ids: team_ids
					.iter()
					.cloned()
					.map(Into::<common::Uuid>::into)
					.collect(),
			},
		)
		.collect();

	Ok(team::member_relationship_get::Response { users })
}
