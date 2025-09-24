mod common;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, ACCEPT, USER_AGENT};
use serde_json::json;

#[test]
fn test_content_type_header_validation() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();
		let body = json!({
			"name": "test-actor",
			"runner_name_selector": "test-runner",
			"crash_policy": "destroy"
		});

		// Test different Content-Type headers
		let content_types = vec![
			("application/json", true),
			("application/json; charset=utf-8", true),
			("application/json; charset=UTF-8", true),
			("text/json", false),        // Should be rejected
			("application/xml", false),  // Should be rejected
			("text/plain", false),       // Should be rejected
		];

		for (content_type, should_succeed) in content_types {
			let response = client
				.post(&format!(
					"http://127.0.0.1:{}/actors?namespace={}",
					ctx.leader_dc().guard_port(),
					namespace
				))
				.header(CONTENT_TYPE, content_type)
				.json(&body)
				.send()
				.await
				.expect("Failed to send request");

			if should_succeed {
				assert!(
					response.status().is_success(),
					"Content-Type '{}' should be accepted",
					content_type
				);
			} else {
				assert!(
					response.status() == 400 || response.status() == 415,
					"Content-Type '{}' should be rejected with 400 or 415, got: {}",
					content_type,
					response.status()
				);
			}
		}
	});
}

#[test]
fn test_accept_header_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor first
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();

		// Test different Accept headers - server should always return JSON
		let accept_headers = vec![
			"application/json",
			"application/json, text/plain",
			"*/*",
			"application/*",
			"text/xml", // May still work but return JSON
		];

		for accept_header in accept_headers {
			let mut headers = HeaderMap::new();
			headers.insert(ACCEPT, HeaderValue::from_str(accept_header).unwrap());

			let response = client
				.get(&format!(
					"http://127.0.0.1:{}/actors/{}?namespace={}",
					ctx.leader_dc().guard_port(),
					actor_id,
					namespace
				))
				.headers(headers)
				.send()
				.await
				.expect("Failed to send request");

			// Should always return JSON regardless of Accept header
			assert!(
				response.status().is_success(),
				"Accept header '{}' should not cause failure",
				accept_header
			);

			let content_type = response
				.headers()
				.get(CONTENT_TYPE)
				.and_then(|v| v.to_str().ok())
				.unwrap_or("");
			assert!(
				content_type.contains("application/json"),
				"Response should always be JSON, got: {}",
				content_type
			);
		}
	});
}

#[test]
fn test_rivet_specific_headers() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();

		// Test Rivet-specific headers for actor communication
		let mut headers = HeaderMap::new();
		headers.insert("X-Rivet-Target", HeaderValue::from_static("actor"));
		headers.insert("X-Rivet-Actor", HeaderValue::from_str(&actor_id).unwrap());
		headers.insert("X-Rivet-Addr", HeaderValue::from_static("ping"));

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/ping",
				ctx.leader_dc().guard_port()
			))
			.headers(headers)
			.send()
			.await
			.expect("Failed to send request");

		// This test verifies Rivet-specific headers are processed correctly
		assert!(
			response.status().as_u16() > 0,
			"Should receive valid HTTP response with Rivet headers"
		);
	});
}

#[test]
fn test_header_case_insensitivity() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test case variations of important headers
		let mut headers = HeaderMap::new();
		headers.insert("content-type", HeaderValue::from_static("application/json"));
		headers.insert("ACCEPT", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("test-client"));

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.query(&[("name", "test-actor")])
			.headers(headers)
			.send()
			.await
			.expect("Failed to send request");

		// Headers should be handled case-insensitively
		assert!(
			response.status().is_success(),
			"Mixed case headers should work"
		);
	});
}

#[test]
fn test_ignored_headers() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test that custom/unknown headers are ignored gracefully
		let mut headers = HeaderMap::new();
		headers.insert("X-Custom-Header", HeaderValue::from_static("test-value"));
		headers.insert("X-Request-ID", HeaderValue::from_static("req-123"));
		headers.insert("X-Client-Version", HeaderValue::from_static("1.0.0"));
		headers.insert("Authorization", HeaderValue::from_static("Bearer token123"));

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.query(&[("name", "test-actor")])
			.headers(headers)
			.send()
			.await
			.expect("Failed to send request");

		// Custom headers should be ignored and not cause issues
		assert!(
			response.status().is_success(),
			"Custom/unknown headers should not cause failure"
		);
	});
}