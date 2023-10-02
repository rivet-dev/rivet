use proto::backend::pkg::{user_follow::count::request::Kind as RequestKind, *};
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct FollowCount {
	user_id: Uuid,
	count: i64,
}

#[operation(name = "user-follow-count")]
async fn handle(
	ctx: OperationContext<user_follow::count::Request>,
) -> GlobalResult<user_follow::count::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let req_kind = internal_unwrap_owned!(RequestKind::from_i32(ctx.kind));

	let follows = match req_kind {
		RequestKind::Mutual => {
			sqlx::query_as::<_, FollowCount>(indoc!(
				"
				SELECT follower_user_id as user_id, COUNT(*)
				FROM (
				SELECT
					uf.follower_user_id,
					EXISTS(
						SELECT 1
						FROM db_user_follow.user_follows AS uf2
						WHERE
							uf2.follower_user_id = uf.following_user_id AND
							uf2.following_user_id = uf.follower_user_id
					) AS is_mutual
				FROM UNNEST($1::UUID[]) AS q
				INNER JOIN db_user_follow.user_follows AS uf
				ON uf.follower_user_id = q
				) as f
				WHERE is_mutual
				GROUP BY follower_user_id
				"
			))
			.bind(&user_ids)
			.fetch_all(&ctx.crdb().await?)
			.await?
		}
		_ => {
			sqlx::query_as::<_, FollowCount>(&formatdoc!(
				"
				SELECT {join_column} as user_id, COUNT(*)
				FROM db_user_follow.user_follows
				WHERE {join_column} = ANY($1)
				GROUP BY {join_column}
				",
				// Columns are inverted
				join_column = match req_kind {
					RequestKind::Follower => "following_user_id",
					RequestKind::Following => "follower_user_id",
					RequestKind::Mutual => unreachable!(),
				},
			))
			.bind(&user_ids)
			.fetch_all(&ctx.crdb().await?)
			.await?
		}
	};

	let follows = user_ids
		.iter()
		.cloned()
		.map(|user_id| {
			let count = follows
				.iter()
				.find(|f| f.user_id == user_id)
				.map(|f| f.count)
				.unwrap_or_default();

			user_follow::count::response::Follows {
				user_id: Some(user_id.into()),
				count,
			}
		})
		.collect::<Vec<_>>();

	Ok(user_follow::count::Response { follows })
}
