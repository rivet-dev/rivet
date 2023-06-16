use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let email = util::faker::email();

	let create_res = op!([ctx] email_verification_create {
		email: email,
	})
	.await
	.unwrap();

	op!([ctx] debug_email_res {
		verification_id: create_res.verification_id,
	})
	.await
	.unwrap();
}
