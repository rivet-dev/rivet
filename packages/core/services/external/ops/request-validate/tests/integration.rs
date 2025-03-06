use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let headers = IntoIterator::into_iter([(
		" invalid  name  ".to_string(),
		"\ninvalid value".to_string(),
	)])
	.collect::<HashMap<_, _>>();

	let res = op!([ctx] external_request_validate {
		config: Some(backend::net::ExternalRequestConfig {
			url: "ssh://example.com".to_string(),
			method: backend::net::HttpMethod::Get as i32,
			headers,
		}),
	})
	.await
	.unwrap();

	tracing::info!(errors=?res.errors);
	assert_eq!(3, res.errors.len());
}

#[worker_test]
async fn dns(ctx: TestCtx) {
	let res = op!([ctx] external_request_validate {
		config: Some(backend::net::ExternalRequestConfig {
			url: "https://httpstat.us/200?sleep=6000".to_string(),
			method: backend::net::HttpMethod::Get as i32,
			headers: HashMap::new(),
		}),
	})
	.await
	.unwrap();

	tracing::info!(errors=?res.errors);
	assert_eq!(1, res.errors.len());
}
