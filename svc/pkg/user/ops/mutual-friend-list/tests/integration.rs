use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let rows = vec![
		(user_a, user_b),
		(user_a, user_c),
		(user_b, user_a),
		(user_b, user_c),
		(user_c, user_a),
		(user_c, user_b),
	];
	for (follower, following) in &rows {
		op!([ctx] user_follow_toggle {
			follower_user_id: Some((*follower).into()),
			following_user_id: Some((*following).into()),
			active: true,
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] user_mutual_friend_list {
		user_a_id: Some(user_a.into()),
		user_b_id: Some(user_b.into()),
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();

	tracing::info!(?user_a, ?user_b, ?user_c, ?res);

	let mutual_friend = res
		.mutual_friends
		.first()
		.as_ref()
		.unwrap()
		.user_id
		.unwrap()
		.as_uuid();
	assert_eq!(mutual_friend, user_c);
}

#[worker_test]
async fn inverse(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let rows = vec![
		(user_a, user_b),
		(user_a, user_c),
		(user_b, user_a),
		(user_b, user_c),
		(user_c, user_a),
		(user_c, user_b),
	];
	for (follower, following) in &rows {
		op!([ctx] user_follow_toggle {
			follower_user_id: Some((*follower).into()),
			following_user_id: Some((*following).into()),
			active: true,
		})
		.await
		.unwrap();
	}

	let (res1, res2) = tokio::try_join!(
		op!([ctx] user_mutual_friend_list {
			user_a_id: Some(user_a.into()),
			user_b_id: Some(user_b.into()),
			limit: 10,
			anchor: None,
		}),
		op!([ctx] user_mutual_friend_list {
			user_a_id: Some(user_b.into()),
			user_b_id: Some(user_a.into()),
			limit: 10,
			anchor: None,
		}),
	)
	.unwrap();

	assert!(
		res1.mutual_friends
			.iter()
			.zip(res2.mutual_friends.iter())
			.map(|(fa, fb)| fa.user_id == fb.user_id)
			.fold(true, |s, a| s & a),
		"results should be symmetrical"
	);
}
