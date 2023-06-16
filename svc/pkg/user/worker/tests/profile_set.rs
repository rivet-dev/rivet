use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user { }).await.expect("user create");
	let user_id = user_res.user_id.unwrap();

	let display_name = util::faker::display_name();
	let account_number = 1234;
	let bio = "bio".to_owned();

	msg!([ctx] user::msg::profile_set(user_id) -> user::msg::update {
		user_id: Some(user_id),
		display_name: Some(display_name.clone()),
		account_number: Some(account_number),
		bio: Some(bio.clone()),
	})
	.await
	.expect("set user profile");

	let (sql_display_name, sql_account_number, sql_bio): (String, i64, String) =
		sqlx::query_as("SELECT display_name, account_number, bio FROM users WHERE user_id = $1")
			.bind(*user_id)
			.fetch_one(&ctx.crdb("db-user").await.unwrap())
			.await
			.unwrap();

	assert_eq!(display_name, sql_display_name);
	assert_eq!(account_number as i64, sql_account_number);
	assert_eq!(bio, sql_bio);
}
