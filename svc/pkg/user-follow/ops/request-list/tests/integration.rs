use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();

	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_b.into()),
		following_user_id: Some(user_a.into()),
		active: true,
	})
	.await
	.unwrap();

	let res = op!([ctx] user_follow_request_list {
		user_ids: vec![user_a.into()],
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();
	let follows = res.follows.first().unwrap();
	assert_eq!(1, follows.follows.len());

	msg!([ctx] user_follow::msg::request_ignore(user_b, user_a) -> user_follow::msg::request_ignore_complete {
		follower_user_id: Some(user_b.into()),
		following_user_id: Some(user_a.into()),
	})
	.await
	.unwrap();

	let res = op!([ctx] user_follow_request_list {
		user_ids: vec![user_a.into()],
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();
	let follows = res.follows.first().unwrap();
	assert_eq!(0, follows.follows.len(), "ignore failed");
}

#[worker_test]
async fn mutual(ctx: TestCtx) {
	let user_a = Uuid::new_v4();
	let user_b = Uuid::new_v4();

	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_b.into()),
		following_user_id: Some(user_a.into()),
		active: true,
	})
	.await
	.unwrap();

	let res = op!([ctx] user_follow_request_list {
		user_ids: vec![user_a.into()],
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();
	let follows = res.follows.first().unwrap();
	assert_eq!(1, follows.follows.len());

	op!([ctx] user_follow_toggle {
		follower_user_id: Some(user_a.into()),
		following_user_id: Some(user_b.into()),
		active: true,
	})
	.await
	.unwrap();

	let res = op!([ctx] user_follow_request_list {
		user_ids: vec![user_a.into()],
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();
	let follows = res.follows.first().unwrap();
	assert_eq!(0, follows.follows.len(), "mutual failed");
}
