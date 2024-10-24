use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

/// Generates a short random string used to make the search results unique.
fn gen_part() -> String {
	let mut rng = rand::thread_rng();
	std::iter::repeat_with(|| {
		let idx = rng.gen_range(0..util::faker::IDENT_CHARSET_ALPHANUM.len());
		util::faker::IDENT_CHARSET_ALPHANUM[idx] as char
	})
	.take(8)
	.collect::<String>()
}

async fn create_user(
	ctx: &TestCtx,
	display_name: Option<String>,
	account_number_option: Option<u32>,
) -> (Uuid, String, String) {
	// to ensure name-uniqueness
	let part1 = display_name.unwrap_or_else(gen_part);

	let account_number = account_number_option.unwrap_or(1234);

	let part2 = gen_part();

	let user_id = Uuid::new_v4();
	msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
		user_id: Some(user_id.into()),
		namespace_id: None,
	})
	.await
	.unwrap();

	msg!([ctx] user::msg::profile_set(user_id) -> user::msg::update {
		user_id: Some(user_id.into()),
		display_name: Some(format!("{} {}", part1, part2)),
		account_number: Some(account_number),
		bio: None,
	})
	.await
	.unwrap();

	// Must be registered to be searchable
	let email = util::faker::email();
	op!([ctx] user_identity_create {
		user_id: Some(user_id.into()),
		identity: Some(backend::user_identity::Identity {
			kind: Some(backend::user_identity::identity::Kind::Email(
				backend::user_identity::identity::Email {
					email: email.clone(),
				}
			)),
		}),
	})
	.await
	.unwrap();

	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	(user_id, part1, part2)
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let rand_str = gen_part();

	let (_, _, part2) = create_user(&ctx, Some(rand_str.clone()), None).await;
	create_user(&ctx, Some(rand_str.clone()), None).await;

	let res = op!([ctx] user_search {
		query: rand_str.clone(),
		limit: 1,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());

	let res = op!([ctx] user_search {
		query: rand_str.clone(),
		limit: 5,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(2, res.user_ids.len());

	let res = op!([ctx] user_search {
		query: part2,
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
}

#[worker_test]
async fn account_numbers_test(ctx: TestCtx) {
	let rand_str = gen_part();

	let (user_a, _, _) = create_user(&ctx, Some(rand_str.clone()), Some(1234)).await;
	let (user_b, _, _) = create_user(&ctx, Some(rand_str.clone()), Some(0051)).await;

	let res = op!([ctx] user_search {
		query: format!("{}#{}", rand_str, 123),
		limit: 2,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
	assert_eq!(user_a, res.user_ids[0].as_uuid());

	let res = op!([ctx] user_search {
		query: format!("{}#005", rand_str),
		limit: 2,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
	assert_eq!(user_b, res.user_ids[0].as_uuid());
}

#[worker_test]
async fn pagination(ctx: TestCtx) {
	let rand_str = gen_part();

	let (user_a, _, _) = create_user(&ctx, Some(rand_str.clone()), Some(1234)).await;
	let (user_b, _, _) = create_user(&ctx, Some(format!("a{}", rand_str)), Some(0051)).await;

	let res = op!([ctx] user_search {
		query: rand_str.clone(),
		limit: 1,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
	assert_eq!(user_b, res.user_ids[0].as_uuid());

	let res = op!([ctx] user_search {
		query: rand_str.clone(),
		limit: 1,
		anchor: res.anchor,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
	assert_eq!(user_a, res.user_ids[0].as_uuid());
}

#[worker_test]
async fn split(ctx: TestCtx) {
	let (user_a, part1, part2) = create_user(&ctx, None, None).await;

	let res = op!([ctx] user_search {
		query: format!("{} {}", part1, part2),
		limit: 1,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.user_ids.len());
	assert_eq!(user_a, res.user_ids[0].as_uuid());
}
