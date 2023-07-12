use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let request_id = Uuid::new_v4();
	let res =
		msg!([ctx] external::msg::request_call(request_id) -> external::msg::request_call_complete {
			request_id: Some(request_id.into()),
			config: Some(backend::net::ExternalRequestConfig {
				url: "https://example.com".to_string(),
				method: backend::net::HttpMethod::Get as i32,
				headers: HashMap::new(),
				timeout: 0,
			}),
			body: None,
			read_response_body: true,
		})
		.await
		.unwrap();

	assert_eq!(200, res.status_code);
	// TODO: Verify body
	tracing::info!(body=?std::str::from_utf8(res.body.as_ref().unwrap()));
}

#[worker_test]
async fn delete(ctx: TestCtx) {
	let request_id = Uuid::new_v4();
	let res =
		msg!([ctx] external::msg::request_call(request_id) -> external::msg::request_call_complete {
			request_id: Some(request_id.into()),
			config: Some(backend::net::ExternalRequestConfig {
				url: "https://example.com".to_string(),
				method: backend::net::HttpMethod::Delete as i32,
				headers: HashMap::new(),
				timeout: 0,
			}),
			body: None,
			read_response_body: false,
		})
		.await
		.unwrap();

	assert_eq!(405, res.status_code);
}

#[worker_test]
async fn timeout(ctx: TestCtx) {
	let request_id = Uuid::new_v4();
	let res =
		msg!([ctx] external::msg::request_call(request_id) -> external::msg::request_call_complete {
			request_id: Some(request_id.into()),
			config: Some(backend::net::ExternalRequestConfig {
				url: "https://httpstat.us/200?sleep=10000".to_string(),
				method: backend::net::HttpMethod::Get as i32,
				headers: HashMap::new(),
				timeout: 1000,
			}),
			body: None,
			read_response_body: false,
		})
		.await
		.unwrap();

	assert_eq!(408, res.status_code);
}
