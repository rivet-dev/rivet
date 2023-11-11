use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] cf_turnstile_verify {
		client_response: "XXXX.DUMMY.TOKEN.XXXX".into(),
		remote_address: "96.65.213.66".into(),
		// https://developers.cloudflare.com/turnstile/reference/testing/
		secret_key: "1x0000000000000000000000000000000AA".into(),
	})
	.await
	.unwrap();
	assert!(res.success, "captcha failed");
}
