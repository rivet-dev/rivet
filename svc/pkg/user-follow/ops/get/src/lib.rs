use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Follow {
	follower_user_id: Uuid,
	following_user_id: Uuid,
	create_ts: i64,
	is_mutual: bool,
}

#[operation(name = "user-follow-get")]
async fn handle(
	ctx: OperationContext<user_follow::get::Request>,
) -> GlobalResult<user_follow::get::Response> {
	let queries = ctx
		.queries
		.iter()
		.map(|query| {
			Ok((
				unwrap_ref!(query.follower_user_id).as_uuid(),
				unwrap_ref!(query.following_user_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<(Uuid, Uuid)>>>()?;

	let follows = sql_fetch_all!(
		[ctx, Follow]
		"
		SELECT 
			uf.follower_user_id, uf.following_user_id, uf.create_ts,
			exists(
				SELECT 1 
				FROM db_user_follow.user_follows AS uf2
				WHERE
					uf2.follower_user_id = q.following_user_id AND 
					uf2.following_user_id = q.follower_user_id
			) AS is_mutual
		FROM (
			SELECT (query->>0)::UUID AS follower_user_id, (query->>1)::UUID AS following_user_id
			FROM jsonb_array_elements($1) AS query
		) AS q
		INNER JOIN db_user_follow.user_follows AS uf
		ON 
			uf.follower_user_id = q.follower_user_id AND
			uf.following_user_id = q.following_user_id
		",
		serde_json::to_value(queries)?,
	)
	.await?;

	Ok(user_follow::get::Response {
		follows: follows
			.into_iter()
			.map(|follow| user_follow::get::response::Follow {
				follower_user_id: Some(follow.follower_user_id.into()),
				following_user_id: Some(follow.following_user_id.into()),
				create_ts: follow.create_ts,
				is_mutual: follow.is_mutual,
			})
			.collect(),
	})
}
