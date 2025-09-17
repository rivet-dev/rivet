mod common;

use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

#[test]
fn test_response_structure_consistency() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();

		// Test GET actor response structure
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/{}?namespace={}",
				ctx.leader_dc().guard_port(),
				actor_id,
				namespace
			))
			.send()
			.await
			.expect("Failed to send request");

		assert!(response.status().is_success());
		let body: Value = response.json().await.expect("Failed to parse JSON");

		// Verify response structure
		assert!(body.get("actor").is_some(), "Response should have 'actor' field");
		let actor = &body["actor"];
		assert!(actor.get("actor_id").is_some(), "Actor should have 'actor_id'");
		assert!(actor.get("name").is_some(), "Actor should have 'name'");
		assert!(actor.get("create_ts").is_some(), "Actor should have 'create_ts'");

		// Test LIST actors response structure
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors?namespace={}&name=test-actor",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.send()
			.await
			.expect("Failed to send request");

		assert!(response.status().is_success());
		let body: Value = response.json().await.expect("Failed to parse JSON");

		// Verify list response structure
		assert!(body.get("actors").is_some(), "Response should have 'actors' array");
		assert!(body["actors"].is_array(), "actors should be an array");
	});
}

#[test]
fn test_error_response_structure() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let client = reqwest::Client::new();

		// Test error response structure for non-existent namespace
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors?namespace=non-existent&name=test",
				ctx.leader_dc().guard_port()
			))
			.send()
			.await
			.expect("Failed to send request");

		assert!(!response.status().is_success());
		let body: Value = response.json().await.expect("Failed to parse JSON");

		// Verify error response structure
		assert!(body.get("code").is_some(), "Error should have 'code'");
		assert!(
			body.get("message").is_some() || body.get("description").is_some(),
			"Error should have 'message' or 'description'"
		);
	});
}

#[test]
fn test_response_content_type() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test different endpoints return proper JSON content type
		let endpoints = vec![
			("GET", format!("/actors?namespace={}&name=test", namespace)),
			("GET", format!("/actors/names?namespace={}", namespace)),
		];

		for (method, path) in endpoints {
			let response = match method {
				"GET" => client
					.get(&format!("http://127.0.0.1:{}{}", ctx.leader_dc().guard_port(), path))
					.send()
					.await
					.expect("Failed to send GET request"),
				_ => panic!("Unsupported method: {}", method),
			};

			assert!(response.status().is_success(), "Request should succeed");

			let content_type = response
				.headers()
				.get(CONTENT_TYPE)
				.and_then(|v| v.to_str().ok())
				.unwrap_or("");

			assert!(
				content_type.contains("application/json"),
				"Response should be JSON for {} {}, got: {}",
				method,
				path,
				content_type
			);
		}
	});
}

#[test]
fn test_response_encoding_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test response with Accept-Encoding header
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors?namespace={}&name=test",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.header("Accept-Encoding", "gzip, deflate")
			.send()
			.await
			.expect("Failed to send request");

		assert!(response.status().is_success());

		// Verify we can parse the response regardless of encoding
		let body: Value = response
			.json()
			.await
			.expect("Failed to parse response (encoding issue?)");

		assert!(body.is_object(), "Response should be parseable JSON");
	});
}

#[test]
fn test_large_response_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create many actors to generate a large response
		let actor_ids = common::bulk_create_actors(
			&namespace,
			"large-response",
			32,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// List all actors to get large response
		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(actor_ids),
			None,
			Some(32),
			None,
			ctx.leader_dc().guard_port()
		).await;

		assert!(response.status().is_success());

		let body: Value = response
			.json()
			.await
			.expect("Failed to parse large response");

		// Verify large response structure
		assert!(body["actors"].is_array());
		let actors = body["actors"].as_array().unwrap();
		assert!(
			actors.len() == 32,
			"Should return all created actors: {} != 32",
			actors.len()
		);
	});
}

#[test]
fn test_field_type_consistency() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/{}?namespace={}",
				ctx.leader_dc().guard_port(),
				actor_id,
				namespace
			))
			.send()
			.await
			.expect("Failed to send request");

		assert!(response.status().is_success());
		let body: Value = response.json().await.expect("Failed to parse JSON");

		let actor = &body["actor"];

		// Verify field types are consistent
		assert!(actor["actor_id"].is_string(), "actor_id should be string");
		assert!(actor["name"].is_string(), "name should be string");

		// Timestamps should be numbers (i64 milliseconds)
		if let Some(create_ts) = actor.get("create_ts") {
			assert!(create_ts.is_number(), "create_ts should be number");
		}

		if let Some(destroy_ts) = actor.get("destroy_ts") {
			assert!(destroy_ts.is_number() || destroy_ts.is_null(), "destroy_ts should be number or null");
		}
	});
}
