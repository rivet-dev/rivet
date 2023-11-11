use chirp_worker::prelude::*;
use proto::backend;

use std::collections::HashMap;

fn generate_topic() -> HashMap<String, String> {
	HashMap::from([("test".to_string(), Uuid::new_v4().to_string())])
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::hcaptcha() {
		return;
	}

	op!([ctx] captcha_verify {
		topic: generate_topic(),
		remote_address: util::faker::ip_addr_v4().to_string(),
		origin_host: None,
		captcha_config: Some(backend::captcha::CaptchaConfig {
			requests_before_reverify: 15,
			verification_ttl: util::duration::hours(1),
			hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
				level: backend::captcha::captcha_config::hcaptcha::Level::Easy as i32,
			}),
			..Default::default()
		}),
		client_response: Some(backend::captcha::CaptchaClientResponse {
			kind: Some(backend::captcha::captcha_client_response::Kind::Hcaptcha(
				backend::captcha::captcha_client_response::Hcaptcha {
					client_response: "10000000-aaaa-bbbb-cccc-000000000001".to_owned(),
				}
			))
		})
	})
	.await
	.unwrap();
}

#[worker_test]
async fn captcha_counts(ctx: TestCtx) {
	if !util::feature::hcaptcha() {
		return;
	}

	let topic = generate_topic();
	let remote_address = util::faker::ip_addr_v4().to_string();
	let requests_before_reverify = 15;
	let captcha_config = backend::captcha::CaptchaConfig {
		requests_before_reverify,
		verification_ttl: util::duration::hours(1),
		hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
			level: backend::captcha::captcha_config::hcaptcha::Level::Easy as i32,
		}),
		..Default::default()
	};

	tracing::info!("initial requests, all should fail");
	for _ in 0..5 {
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;
		assert!(needs_verification, "should require initial verification");
	}

	tracing::info!("pass initial captcha");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, true).await;

	tracing::info!("make requests until requires verification");
	for i in 0..(requests_before_reverify + 5) {
		tracing::info!(?i, ?requests_before_reverify, "making request");
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;

		if i < requests_before_reverify {
			assert!(
				!needs_verification,
				"should not require verification for iter {}/{}",
				i, requests_before_reverify
			);
		} else {
			assert!(
				needs_verification,
				"needs verification for iter {}/{}",
				i, requests_before_reverify
			);
		}
	}

	tracing::info!("pass captcha");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, true).await;

	tracing::info!("make some requests without hitting limit");
	let req_count = 5;
	for i in 0..req_count {
		tracing::info!(?i, ?requests_before_reverify, "making request");
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;
		assert!(
			!needs_verification,
			"should not require verification for iter {}/{}",
			i, requests_before_reverify
		);
	}

	tracing::info!("fail captcha");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, false).await;

	tracing::info!("make requests until requires verification");
	for i in 0..(requests_before_reverify + 5) {
		tracing::info!(?i, ?requests_before_reverify, "making request");
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;

		if i < requests_before_reverify - req_count {
			assert!(
				!needs_verification,
				"should not require verification for iter {}/{}",
				i,
				requests_before_reverify - req_count
			);
		} else {
			assert!(
				needs_verification,
				"needs verification for iter {}/{}",
				i,
				requests_before_reverify - req_count
			)
		}
	}

	tracing::info!("pass captcha after the fail");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, true).await;

	tracing::info!("make some requests without hitting limit");
	let req_count = 5;
	for i in 0..req_count {
		tracing::info!(?i, ?requests_before_reverify, "making request");
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;
		assert!(
			!needs_verification,
			"should not require verification for iter {}/{}",
			i, req_count
		);
	}

	tracing::info!("pass captcha again even though we don't need to");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, true).await;

	tracing::info!("check that the needless pass reset captcha count");
	for i in 0..(requests_before_reverify + 5) {
		tracing::info!(?i, ?requests_before_reverify, "making request");
		let needs_verification =
			captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;

		if i < requests_before_reverify {
			assert!(
				!needs_verification,
				"should not require verification for iter {}/{}",
				i, requests_before_reverify
			);
		} else {
			assert!(
				needs_verification,
				"needs verification for iter {}/{}",
				i, requests_before_reverify
			)
		}
	}
}

#[worker_test]
async fn captcha_timing(ctx: TestCtx) {
	if !util::feature::hcaptcha() {
		return;
	}

	let topic = generate_topic();
	let remote_address = util::faker::ip_addr_v4().to_string();
	let verification_ttl = util::duration::hours(1);
	let captcha_config = backend::captcha::CaptchaConfig {
		requests_before_reverify: 15,
		verification_ttl,
		hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
			level: backend::captcha::captcha_config::hcaptcha::Level::Easy as i32,
		}),
		..Default::default()
	};

	tracing::info!("initial verification");
	captcha_verify(&ctx, &topic, &remote_address, &captcha_config, true).await;
	let verified_ts = util::timestamp::now();

	tracing::info!("checking verification worked");
	let needs_verification = captcha_request(&ctx, &topic, &remote_address, &captcha_config).await;
	assert!(
		!needs_verification,
		"initial request should not need verification"
	);

	// TODO: Implement override_ts for operation context
	// tracing::info!("failing verification after time");
	// let res = ctx
	// 	.chirp()
	// 	.rpc_debug::<captcha::request::Endpoint>(
	// 		None,
	// 		captcha::request::Request {
	// 			topic: topic.clone(),
	// 			remote_address: remote_address.to_string(),
	// 			captcha_config: Some(captcha_config.clone()),
	// 			..Default::default()
	// 		},
	// 		Some(chirp_client::RequestDebug {
	// 			override_ts: verified_ts + verification_ttl,
	// 		}),
	// 		false,
	// 	)
	// 	.await
	// 	.unwrap();
	// assert!(
	// 	res.needs_verification,
	// 	"initial request should not need verification"
	// );
}

#[worker_test]
async fn turnstile(ctx: TestCtx) {
	let topic = generate_topic();
	let remote_address = util::faker::ip_addr_v4().to_string();
	let verification_ttl = util::duration::hours(1);
	let captcha_config = backend::captcha::CaptchaConfig {
		requests_before_reverify: 15,
		verification_ttl,
		turnstile: Some(backend::captcha::captcha_config::Turnstile {
			domains: vec![backend::captcha::captcha_config::turnstile::Domain {
				domain: "rivet.gg".to_string(),
				// Always passes
				secret_key: "1x0000000000000000000000000000000AA".to_string(),
			}],
		}),
		..Default::default()
	};

	op!([ctx] captcha_verify {
		topic: topic.clone(),
		remote_address: remote_address.to_string(),
		origin_host: Some("test.rivet.gg".to_string()),
		captcha_config: Some(captcha_config.clone()),
		client_response: Some(backend::captcha::CaptchaClientResponse {
			kind: Some(backend::captcha::captcha_client_response::Kind::Turnstile(
				backend::captcha::captcha_client_response::Turnstile {
					client_response: "unimportant".to_owned(),
				},
			))
		})
	})
	.await
	.unwrap();

	op!([ctx] captcha_verify {
		topic: topic.clone(),
		remote_address: remote_address.to_string(),
		origin_host: Some("test.com".to_string()),
		captcha_config: Some(captcha_config.clone()),
		client_response: Some(backend::captcha::CaptchaClientResponse {
			kind: Some(backend::captcha::captcha_client_response::Kind::Turnstile(
				backend::captcha::captcha_client_response::Turnstile {
					client_response: "unimportant".to_owned(),
				},
			))
		})
	})
	.await
	.unwrap_err();
}

async fn captcha_verify(
	ctx: &TestCtx,
	topic: &HashMap<String, String>,
	remote_address: &str,
	captcha_config: &backend::captcha::CaptchaConfig,
	pass: bool,
) {
	if !util::feature::hcaptcha() {
		return;
	}

	let res = op!([ctx] captcha_verify {
		topic: topic.clone(),
		remote_address: remote_address.to_string(),
		origin_host: None,
		captcha_config: Some((*captcha_config).clone()),
		client_response: Some(backend::captcha::CaptchaClientResponse {
			kind: Some(backend::captcha::captcha_client_response::Kind::Hcaptcha(
				backend::captcha::captcha_client_response::Hcaptcha {
					client_response: if pass {
						"10000000-aaaa-bbbb-cccc-000000000001".to_owned()
					} else {
						"bad response".to_owned()
					},
				}
			))
		})
	})
	.await;
	match res {
		Ok(_) => {
			assert!(pass, "verify should not have passed");
		}
		Err(err) if err.is(formatted_error::code::CAPTCHA_CAPTCHA_FAILED) => {
			assert!(!pass, "verify should have passed");
		}
		Err(err) => Err(err).unwrap(),
	}
}

async fn captcha_request(
	ctx: &TestCtx,
	topic: &HashMap<String, String>,
	remote_address: &str,
	captcha_config: &backend::captcha::CaptchaConfig,
) -> bool {
	let res = op!([ctx] captcha_request {
		topic: topic.clone(),
		remote_address: remote_address.to_string(),
		captcha_config: Some((*captcha_config).clone()),
	})
	.await
	.unwrap();
	res.needs_verification
}
