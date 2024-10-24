use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn admin_set(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user { }).await.unwrap();
	let user_id = user_res.user_id.unwrap();

	// Turn user into admin
	msg!([ctx] user::msg::admin_set(user_id) -> user::msg::update {
		user_id: Some(user_id),
	})
	.await
	.unwrap();

	let (exists,) = sql_fetch_one!(
		[ctx, (bool,)]
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_user.users
			WHERE
				user_id = $1 AND
				is_admin = true
		)
		",
		user_id.as_uuid(),
	)
	.await
	.unwrap();

	assert!(exists, "user not made into an admin");
}
