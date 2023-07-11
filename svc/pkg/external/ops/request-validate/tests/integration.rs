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
			timeout: 0,
		}),
	})
	.await
	.unwrap();

	tracing::info!(errors=?res.errors);
	assert_eq!(3, res.errors.len());
}

// TODO: Write DNS test
