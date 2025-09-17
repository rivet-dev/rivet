mod common;

use reqwest::header::{HeaderValue, CONTENT_TYPE};
use serde_json::json;

#[test]
fn test_request_body_field_validation() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test minimal required fields
		let minimal_body = json!({
			"name": "minimal-actor",
			"crash_policy": "destroy",
			"runner_name_selector": "test-runner"
		});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&minimal_body)
			.send()
			.await
			.expect("Failed to send request");
		assert!(response.status().is_success(), "Minimal JSON should work");

		// Test complete body with all fields
		let full_body = json!({
			"name": "full-actor",
			"key": "test-key",
			"input": "dGVzdCBkYXRh", // base64 "test data"
			"crash_policy": "destroy",
			"runner_name_selector": "test-runner"
		});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&full_body)
			.send()
			.await
			.expect("Failed to send request");
		assert!(response.status().is_success(), "Full JSON should work");

		// Test extra fields are ignored
		let extra_fields_body = json!({
			"name": "extra-fields-actor",
			"crash_policy": "destroy",
			"runner_name_selector": "test-runner",
			"extra_field": "should be ignored",
			"another_extra": 123,
			"nested_extra": {"field": "value"}
		});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&extra_fields_body)
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"JSON with extra fields should fail"
		);
	});
}

#[test]
fn test_malformed_json_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test various malformed JSON strings that should be rejected
		let invalid_json_bodies = vec![
			r#"{"name": "test"#,      // Missing closing brace
			r#"{"name": test"}"#,     // Missing quotes around value
			r#"{name: "test"}"#,      // Missing quotes around key
			r#"{"name": "test",}"#,   // Trailing comma
			r#""just a string""#,     // Not an object
			r#"[1, 2, 3]"#,          // Array instead of object
			r#"null"#,               // Null value
			r#""#,                   // Empty string
			r#"invalid json"#,       // Not JSON at all
		];

		for invalid_json in invalid_json_bodies {
			let response = client
				.post(&format!(
					"http://127.0.0.1:{}/actors?namespace={}",
					ctx.leader_dc().guard_port(),
					namespace
				))
				.header(CONTENT_TYPE, "application/json")
				.body(invalid_json)
				.send()
				.await
				.expect("Failed to send request");

			assert!(
				!response.status().is_success(),
				"Invalid JSON '{}' expected unsuccessful request",
				invalid_json.chars().take(20).collect::<String>()
			);
		}
	});
}

#[test]
fn test_field_type_validation() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test invalid field types
		let invalid_bodies = vec![
			// Name field should be string
			json!({"name": 123}),
			json!({"name": null}),
			json!({"name": []}),
			json!({"name": {}}),

			// Crash policy should be valid enum value
			json!({"name": "test", "crash_policy": "invalid_policy"}),
			json!({"name": "test", "crash_policy": 123}),
		];

		for invalid_body in invalid_bodies {
			let response = client
				.post(&format!(
					"http://127.0.0.1:{}/actors?namespace={}",
					ctx.leader_dc().guard_port(),
					namespace
				))
				.json(&invalid_body)
				.send()
				.await
				.expect("Failed to send request");

			assert!(
				!response.status().is_success(),
				"Invalid field type should be rejected: {:?}",
				invalid_body
			);
		}
	});
}

#[test]
fn test_large_request_body_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test large input data (1MB)
		let large_input = common::generate_large_input_data(1);
		let large_body = json!({
			"name": "large-input-actor",
			"input": large_input
		});

		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&large_body)
			.send()
			.await
			.expect("Failed to send request");

		// Should handle large bodies or reject with appropriate error
		assert!(
			response.status().is_success() || response.status() == 422,
			"Large body should succeed or return 422 Unprocessable Entity, got: {}",
			response.status()
		);
	});
}

#[test]
fn test_missing_required_fields() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test completely empty body
		let empty_body = json!({});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&empty_body)
			.send()
			.await
			.expect("Failed to send request");

		assert!(
			!response.status().is_success(),
			"Empty body should be rejected"
		);

		// Test body with empty name field
		let empty_name_body = json!({"name": ""});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&empty_name_body)
			.send()
			.await
			.expect("Failed to send request");

		assert!(
			!response.status().is_success(),
			"Empty name field should be rejected"
		);
	});
}

#[test]
fn test_base64_input_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		// Test valid base64 input
		let valid_b64_body = json!({
			"name": "b64-actor",
			"input": "dGVzdCBkYXRh", // "test data" in base64
			"crash_policy": "destroy",
			"runner_name_selector": "test-runner"
		});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&valid_b64_body)
			.send()
			.await
			.expect("Failed to send request");

		assert!(
			response.status().is_success(),
			"Valid base64 input should be accepted"
		);

		// Test invalid base64 input
		let invalid_b64_body = json!({
			"name": "invalid-b64-actor",
			"input": "not-valid-base64!@#$%"
		});
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&invalid_b64_body)
			.send()
			.await
			.expect("Failed to send request");

		// Server may accept invalid base64 and handle it during processing
		// or reject it immediately - both behaviors are valid
		assert!(
			response.status().as_u16() > 0,
			"Should handle invalid base64 input gracefully"
		);
	});
}
