use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();

	// make user_a follow user_b
	let mut user_follow_sub = subscribe!([ctx] user_follow::msg::create(user_a, user_b))
		.await
		.unwrap();
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_a.into()),
		following_user_id: Some(user_b.into()),
		active: true,
	})
	.await
	.unwrap();
	user_follow_sub.next().await.unwrap();

	// assert user_a is following user_b
	let (num_follows,): (i64,) = sqlx::query_as(
		"SELECT count(*) FROM db_user_follow.user_follows WHERE follower_user_id = $1 AND following_user_id = $2",
	)
	.bind(user_a)
	.bind(user_b)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert_eq!(num_follows, 1);

	// make user_a unfollow user_b
	let mut user_follow_sub = subscribe!([ctx] user_follow::msg::delete(user_a, user_b))
		.await
		.unwrap();
	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_a.into()),
		following_user_id: Some(user_b.into()),
		active: false,
	})
	.await
	.unwrap();
	user_follow_sub.next().await.unwrap();

	// assert user_a is not following user_b
	let (num_follows,): (i64,) = sqlx::query_as(
		"SELECT count(*) FROM db_user_follow.user_follows WHERE follower_user_id = $1 AND following_user_id = $2",
	)
	.bind(user_a)
	.bind(user_b)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert_eq!(num_follows, 0);
}

#[worker_test]
async fn mutual(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();

	// Follow (not mutuals)
	{
		let mut user_mutual_follow_sub = subscribe!([ctx] user::msg::mutual_follow_create(user_a))
			.await
			.unwrap();

		op!([ctx] user_follow_toggle {
			follower_user_id: Some(user_a.into()),
			following_user_id: Some(user_b.into()),
			active: true,
		})
		.await
		.unwrap();

		let mutual_created = util::macros::select_with_timeout!([3 SEC] {
			_ = user_mutual_follow_sub.next() => {
				true
			}
		});
		assert!(
			!mutual_created,
			"should not broadcast mutual create message"
		);
	}

	// Follow (mutuals)
	{
		let mut user_mutual_follow_sub = subscribe!([ctx] user::msg::mutual_follow_create(user_a))
			.await
			.unwrap();

		op!([ctx] user_follow_toggle {
			follower_user_id: Some(user_b.into()),
			following_user_id: Some(user_a.into()),
			active: true,
		})
		.await
		.unwrap();

		let mutual_created = util::macros::select_with_timeout!([3 SEC] {
			_ = user_mutual_follow_sub.next() => {
				true
			}
		});
		assert!(mutual_created, "mutuality not achieved");
	}

	// Unfollow (mutuals)
	{
		let mut user_mutual_follow_sub = subscribe!([ctx] user::msg::mutual_follow_delete(user_a))
			.await
			.unwrap();

		op!([ctx] user_follow_toggle {
			follower_user_id: Some(user_b.into()),
			following_user_id: Some(user_a.into()),
			active: false,
		})
		.await
		.unwrap();

		let mutual_deleted = util::macros::select_with_timeout!([3 SEC] {
			_ = user_mutual_follow_sub.next() => {
				true
			}
		});
		assert!(mutual_deleted, "mutuality not removed");
	}

	// Unfollow (not mutuals)
	{
		let mut user_mutual_follow_sub = subscribe!([ctx] user::msg::mutual_follow_delete(user_a))
			.await
			.unwrap();

		op!([ctx] user_follow_toggle {
			follower_user_id: Some(user_a.into()),
			following_user_id: Some(user_b.into()),
			active: false,
		})
		.await
		.unwrap();

		let mutual_deleted = util::macros::select_with_timeout!([3 SEC] {
			_ = user_mutual_follow_sub.next() => {
				true
			}
		});
		assert!(
			!mutual_deleted,
			"should not broadcast mutual delete message"
		);
	}
}
