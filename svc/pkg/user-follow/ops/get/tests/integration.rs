use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();
	let user_c = Uuid::new_v4();

	let rows = vec![
		(user_a, user_b, false),
		(user_b, user_c, true),
		(user_c, user_b, true),
	];
	for (follower, following, _) in &rows {
		op!([ctx] user_follow_toggle {
			follower_user_id: Some((*follower).into()),
			following_user_id: Some((*following).into()),
			active: true,
		})
		.await
		.unwrap();
	}

	let res = op!([ctx] user_follow_get {
		queries: rows
			.iter()
			.map(|row| user_follow::get::request::Query {
				follower_user_id: Some(row.0.into()),
				following_user_id: Some(row.1.into()),
			})
			.collect(),
	})
	.await
	.unwrap();

	assert_eq!(3, res.follows.len());
	for row in &rows {
		let follow = res
			.follows
			.iter()
			.find(|f| {
				f.follower_user_id.as_ref().unwrap().as_uuid() == row.0
					&& f.following_user_id.as_ref().unwrap().as_uuid() == row.1
			})
			.expect("missing follow");
		assert_eq!(row.2, follow.is_mutual, "wrong follow mutual");
	}
}
