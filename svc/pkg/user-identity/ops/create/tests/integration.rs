use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_res = op!([ctx] faker_user {
		..Default::default()
	})
	.await
	.unwrap();

	let email = util::faker::email();
	op!([ctx] user_identity_create {
		user_id: user_res.user_id,
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

	let (sql_exists,) = sqlx::query_as::<_, (bool,)>(
		"SELECT EXISTS (SELECT 1 FROM db_user_identity.emails WHERE email = $1)",
	)
	.bind(&email)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(sql_exists, "identity not created");
}

// TODO: Access token test
