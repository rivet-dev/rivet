use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {}).await.unwrap();
	let user_id = user_res.user_id.as_ref().unwrap().as_uuid();

	// Register user
	let email = util::faker::email();
	let _res = op!([ctx] user_identity_create {
		user_id: Some(user_id.into()),
		identity: Some(backend::user_identity::Identity {
			kind: Some(backend::user_identity::identity::Kind::Email(
				backend::user_identity::identity::Email {
					email: email.clone()
				}
			)),
		}),
	})
	.await
	.unwrap();

	op!([ctx] user_pending_delete_toggle {
		user_id: Some(user_id.into()),
		active: true,
	})
	.await
	.unwrap();

	let (delete_request_ts,): (Option<i64>,) = sqlx::query_as(indoc!(
		"
		SELECT delete_request_ts
			FROM users
			WHERE
				user_id = $1
		",
	))
	.bind(user_id)
	.fetch_one(&ctx.crdb("db-user").await.unwrap())
	.await
	.unwrap();

	assert!(delete_request_ts.is_some());

	op!([ctx] user_pending_delete_toggle {
		user_id: Some(user_id.into()),
		active: false,
	})
	.await
	.unwrap();

	let (delete_request_ts,): (Option<i64>,) = sqlx::query_as(indoc!(
		"
		SELECT delete_request_ts
			FROM users
			WHERE
				user_id = $1
		",
	))
	.bind(user_id)
	.fetch_one(&ctx.crdb("db-user").await.unwrap())
	.await
	.unwrap();

	assert!(delete_request_ts.is_none());
}
