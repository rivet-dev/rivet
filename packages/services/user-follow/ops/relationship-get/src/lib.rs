use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Relationship {
	this_user_id: Uuid,
	other_user_id: Uuid,
	is_follower: bool,
	is_following: bool,
}

#[operation(name = "user-follow-relationship-get")]
async fn handle(
	ctx: OperationContext<user_follow::relationship_get::Request>,
) -> GlobalResult<user_follow::relationship_get::Response> {
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
	let relationships = sql_fetch_all!(
		[ctx, Relationship]
		"
		SELECT
			(q->>0)::UUID AS this_user_id,
			(q->>1)::UUID AS other_user_id,
			exists(
				SELECT 1
				FROM db_user_follow.user_follows AS uf
				WHERE
					uf.follower_user_id = (q->>0)::UUID AND
					uf.following_user_id = (q->>1)::UUID
			) AS is_follower,
			exists(
				SELECT 1
				FROM db_user_follow.user_follows AS uf
				WHERE
					uf.follower_user_id = (q->>1)::UUID AND
					uf.following_user_id = (q->>0)::UUID
			) AS is_following
		FROM jsonb_array_elements($1::JSONB) AS q
		",
		serde_json::to_string(&query_users)?,
	)
	.await?;

	let users = relationships
		.iter()
		.map(|x| user_follow::relationship_get::response::User {
			this_user_id: Some(x.this_user_id.into()),
			other_user_id: Some(x.other_user_id.into()),
			is_mutual: x.is_follower && x.is_following,
			is_follower: x.is_follower,
			is_following: x.is_following,
		})
		.collect();

	Ok(user_follow::relationship_get::Response { users })
}
