use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let topic = vec![("test".to_string(), Uuid::new_v4().to_string())]
		.into_iter()
		.collect::<HashMap<String, String>>();

	op!([ctx] captcha_request {
		topic: topic,
		remote_address: util::faker::ip_addr_v4().to_string(),
		captcha_config: Some(backend::captcha::CaptchaConfig {
			requests_before_reverify: 15,
			verification_ttl: util::duration::hours(1),
			hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
				site_key: Some("10000000-ffff-ffff-ffff-000000000001".to_string()),
				secret_key: Some("0x0000000000000000000000000000000000000000".to_string()),
			}),
			..Default::default()
		})
	})
	.await
	.unwrap();
}
