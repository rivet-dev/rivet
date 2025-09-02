mod common;

use serde_json::json;

// MARK: Basic

#[test]
fn get_existing_actor_id_with_matching_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "existing-actor";
		let key = "key1".to_string();

		let existing_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some(key.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		let response = common::get_or_create_actor_by_id(
			&namespace,
			name,
			Some(key),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor_id"], existing_actor_id);
		common::assert_created_response(&body, false).await;
	});
}

#[test]
fn create_new_actor_id_when_none_exists() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "new-actor";
		let key = "new-key".to_string();

		let response = common::get_or_create_actor_by_id(
			&namespace,
			name,
			Some(key),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actor_id = body["actor_id"].as_str().expect("Missing actor_id");
		assert!(!actor_id.is_empty());
		common::assert_created_response(&body, true).await;

		common::assert_actor_exists(actor_id, &namespace, ctx.leader_dc().guard_port()).await;
	});
}

#[test]
fn create_actor_id_in_specific_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "dc-specific-actor";
		let key = "dc-key".to_string();

		let response = common::get_or_create_actor_by_id(
			&namespace,
			name,
			Some(key),
			Some("dc-2"),
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actor_id = body["actor_id"].as_str().expect("Missing actor_id");
		common::assert_created_response(&body, true).await;

		let actor =
			common::assert_actor_exists(actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		let actor_id_str = actor["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

// MARK: Error Cases

#[test]
fn get_or_create_by_id_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::get_or_create_actor_by_id(
			"non-existent-namespace",
			"test-actor",
			Some("key".to_string()),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(
			!response.status().is_success(),
			"Should fail with non-existent namespace"
		);
		common::assert_error_response(response, "namespace_not_found").await;
	});
}

#[test]
fn get_or_create_by_id_invalid_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::get_or_create_actor_by_id(
			&namespace,
			"test-actor",
			Some("key".to_string()),
			Some("invalid-dc"),
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(
			!response.status().is_success(),
			"Should fail with invalid datacenter"
		);
	});
}
