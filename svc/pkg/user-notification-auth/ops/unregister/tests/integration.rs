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
			FROM db_user_notification_auth.users
			WHERE user_id = $1
		)
		"
	))
	.bind(user_id.as_uuid())
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();

	assert!(exists);

	op!([ctx] user_notification_auth_unregister {
		user_id: Some(user_id),
		service: user_notification_auth::unregister::request::Service::Firebase as i32,
	})
	.await
	.unwrap();

	let (exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (
			SELECT firebase_access_key
			FROM db_user_notification_auth.users
			WHERE user_id = $1
		)
		"
	))
	.bind(user_id.as_uuid())
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();

	assert!(!exists);
}
