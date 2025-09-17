mod common;

use std::time::Duration;

use reqwest::Client;
use serde_json::json;

// MARK: Basic

#[test]
#[ignore]
fn get_actor_id_for_existing_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "test-actor";
		let key = "test-key-123";
		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some(key.to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		tokio::time::sleep(Duration::from_millis(500)).await;

		let response =
			common::get_actor_by_id(&namespace, name, key, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor_id"], actor_id);
	});
}

#[test]
fn get_null_actor_id_for_non_existent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::get_actor_by_id(
			&namespace,
			"non-existent-actor",
			"non-existent-key",
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(
			body["actor_id"],
			json!(null),
			"Should return null for non-existent actor"
		);
	});
}

// MARK: Error cases

#[test]
fn test_with_parameter_variations() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = Client::new();

		// Test missing required parameters
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id",
				ctx.leader_dc().guard_port()
			))
			.query(&[("name", "test"), ("key", "test")])
			.send()
			.await
			.expect("Failed to send request");

		assert!(!response.status().is_success(), "Should fail without namespace");

		// Test empty parameter values
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id",
				ctx.leader_dc().guard_port()
			))
			.query(&[("namespace", ""), ("name", "test"), ("key", "test")])
			.send()
			.await
			.expect("Failed to send request");

		assert!(!response.status().is_success(), "Should fail with empty namespace");
	});
}

#[test]
fn get_by_id_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::get_actor_by_id(
			"non-existent-namespace",
			"test-actor",
			"test-key",
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(
			!response.status().is_success(),
			"Should fail with non-existent namespace but instead got {}", response.text().await.unwrap()
		);
		common::assert_error_response(response, "not_found").await;
	});
}

#[test]
fn get_by_id_missing_parameters() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id?namespace={}&key=test-key",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.send()
			.await
			.expect("Failed to send request");
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for missing name parameter"
		);

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id?namespace={}&name=test-actor",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.send()
			.await
			.expect("Failed to send request");
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for missing key parameter"
		);

		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id?name=test-actor&key=test-key",
				ctx.leader_dc().guard_port()
			))
			.send()
			.await
			.expect("Failed to send request");
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for missing namespace parameter"
		);
	});
}
