use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let all_user_ids = [user_a, user_b, user_c];

	let follows = [
		(user_a, user_b),
		(user_a, user_c),
		(user_b, user_c),
		(user_c, user_a),
		(user_c, user_b),
	];
	for &(follower, following) in follows.iter() {
		op!([ctx] user_follow_toggle {
			follower_user_id: Some(follower.into()),
			following_user_id: Some(following.into()),
			active: true,
		})
		.await
		.unwrap();
	}

	let tests = all_user_ids
		.iter()
		.flat_map(|&this_user| {
			all_user_ids
				.iter()
				.map(move |&other_user| (this_user, other_user))
		})
		.collect::<Vec<_>>();
	let test_users = tests
		.iter()
		.map(
			|&(this_user, other_user)| user_follow::relationship_get::request::User {
				this_user_id: Some(this_user.into()),
				other_user_id: Some(other_user.into()),
			},
		)
		.collect();
	let res = op!([ctx] user_follow_relationship_get {
		users: test_users,
	})
	.await
	.unwrap();

	tracing::info!(?res, ?tests);

	res.users.iter().for_each(|relationship| {
		let this_user = relationship.this_user_id.unwrap().as_uuid();
		let other_user = relationship.other_user_id.unwrap().as_uuid();

		assert_eq!(
			follows
				.iter()
				.any(|x| x.0 == this_user && x.1 == other_user),
			relationship.is_follower,
			"bad follower"
		);
		assert_eq!(
			follows
				.iter()
				.any(|x| x.1 == this_user && x.0 == other_user),
			relationship.is_following,
			"bad following"
		);
		assert_eq!(
			relationship.is_follower && relationship.is_following,
			relationship.is_mutual,
			"bad mutual"
		);
	});
}
