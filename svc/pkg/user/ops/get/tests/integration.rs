use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use rand::Rng;

#[worker_test]
async fn empty(ctx: TestCtx) {
	tokio::time::sleep(std::time::Duration::from_secs(120)).await;
	let res = op!([ctx] user_get {
		user_ids: Vec::new(),
	})
	.await
	.unwrap();
	assert!(res.users.is_empty());
}

#[worker_test]
async fn fetch(ctx: TestCtx) {
	struct TestUser {
		user_id: Option<Uuid>,
		display_name: String,
		account_number: i64,
		bio: String,
	}

	// Generate test users
	let mut users = std::iter::repeat_with(|| TestUser {
		user_id: None,
		display_name: util::faker::display_name(),
		account_number: rand::thread_rng().gen_range(1..10000),
		bio: util::faker::ident(),
	})
	.take(8)
	.collect::<Vec<_>>();

	// Insert test users
	for user in &mut users {
		let user_res = op!([ctx] faker_user { }).await.unwrap();
		let user_id = user_res.user_id.unwrap().as_uuid();

		msg!([ctx] user::msg::profile_set(user_id) -> user::msg::update {
			user_id: Some(user_id.into()),
			display_name: Some(user.display_name.clone()),
			account_number: Some(user.account_number as u32),
			bio: Some(user.bio.clone()),
		})
		.await
		.unwrap();

		user.user_id = Some(user_id);
	}

	// Fetch the users
	let res = op!([ctx] user_get {
		user_ids: users.iter().map(|u| u.user_id.unwrap().into()).collect(),
	})
	.await
	.unwrap();

	// Validate the users
	assert_eq!(users.len(), res.users.len());
	for user in &users {
		let user_res = res
			.users
			.iter()
			.find(|u| u.user_id.as_ref().unwrap().as_uuid() == user.user_id.unwrap())
			.expect("user not returned");

		assert_eq!(user.display_name, user_res.display_name);
		assert_eq!(user.account_number, user_res.account_number as i64);
		assert_eq!(user.bio, user_res.bio);
	}
}
