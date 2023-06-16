use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn correct(ctx: TestCtx) {
	let create_res = op!([ctx] email_verification_create {
		email: "test@rivet.gg".into(),
	})
	.await
	.unwrap();
	let verification_id = create_res.verification_id.as_ref().unwrap().as_uuid();

	let res = op!([ctx] debug_email_res {
		verification_id: Some(verification_id.into()),
	})
	.await
	.unwrap();
	let code = &res.code;

	let res = op!([ctx] email_verification_complete {
		verification_id: Some(verification_id.into()),
		code: code.clone(),
	})
	.await
	.unwrap();
	assert_eq!(
		email_verification::complete::response::Status::Correct as i32,
		res.status,
		"should be correct"
	);

	let (sql_complete_ts,) = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT complete_ts FROM verifications WHERE verification_id = $1",
	)
	.bind(verification_id)
	.fetch_one(&ctx.crdb("db-email-verification").await.unwrap())
	.await
	.unwrap();
	assert!(sql_complete_ts.is_some(), "not flagged as complete");

	let res = op!([ctx] email_verification_complete {
		verification_id: Some(verification_id.into()),
		code: code.clone(),
	})
	.await
	.unwrap();
	assert_eq!(
		email_verification::complete::response::Status::AlreadyComplete as i32,
		res.status,
		"not flagged as already complete"
	);
}

#[worker_test]
async fn incorrect(ctx: TestCtx) {
	let create_res = op!([ctx] email_verification_create {
		email: "test@rivet.gg".into(),
	})
	.await
	.unwrap();
	let verification_id = create_res.verification_id.as_ref().unwrap().as_uuid();

	for _ in 0usize..4 {
		// See MAX_ATTEMPTS
		let res = op!([ctx] email_verification_complete {
			verification_id: Some(verification_id.into()),
			code: "THIS IS WRONG".into(),
		})
		.await
		.unwrap();
		assert_eq!(
			email_verification::complete::response::Status::Incorrect as i32,
			res.status,
			"expected incorrect"
		);
	}

	let res = op!([ctx] email_verification_complete {
		verification_id: Some(verification_id.into()),
		code: "THIS IS WRONG".into(),
	})
	.await
	.unwrap();
	assert_eq!(
		email_verification::complete::response::Status::TooManyAttempts as i32,
		res.status,
		"expected too many attempts"
	);
}
