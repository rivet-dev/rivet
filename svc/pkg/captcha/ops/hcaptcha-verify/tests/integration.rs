use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::hcaptcha() {
		return;
	}
	
	let res = op!([ctx] captcha_hcaptcha_verify {
		client_response: "10000000-aaaa-bbbb-cccc-000000000001".into(),
		site_key: "10000000-ffff-ffff-ffff-000000000001".into(),
		remote_address: "96.65.213.66".into(),
	})
	.await
	.unwrap();
	assert!(res.success, "captcha failed");
}
