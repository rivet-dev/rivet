use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct Follow {
	follower_user_id: Uuid,
	following_user_id: Uuid,
	create_ts: i64,
}

#[operation(name = "user-follow-request-list")]
async fn handle(
	ctx: OperationContext<user_follow::request_list::Request>,
) -> GlobalResult<user_follow::request_list::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	let limit = ctx.limit;

	internal_assert!(limit != 0, "limit too low");
	internal_assert!(limit <= 32, "limit too high");

	let follows = sqlx::query_as::<_, Follow>(indoc!(
		"
		SELECT follower_user_id, following_user_id, create_ts, ignored, is_mutual
		FROM (
			SELECT
				uf.follower_user_id, uf.following_user_id, uf.create_ts, uf.ignored,
				EXISTS(
					SELECT 1
					FROM db_user_follow.user_follows AS uf2
					WHERE
						uf2.follower_user_id = uf.following_user_id AND
						uf2.following_user_id = uf.follower_user_id
				) AS is_mutual
			FROM unnest($1::UUID[]) AS q
			INNER JOIN db_user_follow.user_follows AS uf
			ON uf.following_user_id = q
		)
		WHERE
			create_ts > $2 AND
			NOT is_mutual AND
			NOT ignored
		ORDER BY create_ts DESC
		LIMIT $3
		",
	))
	.bind(&user_ids)
	.bind(ctx.anchor.unwrap_or_default())
	.bind(limit as i64)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let follows = user_ids
		.iter()
		.cloned()
		.map(|user_id| {
			let follows = follows
				.iter()
				.filter(|f| f.following_user_id == user_id)
				.map(|follow| user_follow::request_list::response::Follow {
					user_id: Some(follow.follower_user_id.into()),
					create_ts: follow.create_ts,
				})
				.collect::<Vec<_>>();

			let anchor = follows
				.last()
				.and_then(|follow| (follows.len() >= limit as usize).then_some(follow.create_ts));

			user_follow::request_list::response::Follows {
				user_id: Some(user_id.into()),
				follows,
				anchor,
			}
		})
		.collect::<Vec<_>>();

	Ok(user_follow::request_list::Response { follows })
}
