use chirp_worker::prelude::*;
use proto::backend;

use std::collections::HashMap;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] captcha_request {
		topic: vec![
			("test".to_string(), Uuid::new_v4().to_string())
		].into_iter().collect::<HashMap<String, String>>(),
		remote_address: util::faker::ip_addr_v4().to_string(),
		captcha_config: Some(backend::captcha::CaptchaConfig {
			requests_before_reverify: 15,
			verification_ttl: util::duration::hours(1),
			hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
				level: backend::captcha::captcha_config::hcaptcha::Level::Easy as i32,
			}),
			..Default::default()
		})
	})
	.await
	.unwrap();
}
