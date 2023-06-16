use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = user_res.user_id.unwrap();

	op!([ctx] user_notification_auth_register {
		user_id: Some(user_id),
		registration: Some(user_notification_auth::register::request::Registration::Firebase(
			user_notification_auth::register::request::FirebaseRegistration {
				access_key: "--".to_owned(),
			},
		)),
	})
	.await
	.unwrap();

	let (exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (
			SELECT firebase_access_key
			FROM users
			WHERE user_id = $1
		)
		"
	))
	.bind(user_id.as_uuid())
	.fetch_one(&ctx.crdb("db-user-notification-auth").await.unwrap())
	.await
	.unwrap();

	assert!(exists);
}
