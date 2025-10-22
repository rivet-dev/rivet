mod common;

use serde_json::json;

// MARK: Basic

#[test]
fn get_existing_actor_with_matching_key() {
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

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key),
			false,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], existing_actor_id);
		common::assert_created_response(&body, false).await;
	});
}

#[test]
fn get_existing_actor_from_remote_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "remote-actor";
		let key = "remote-key".to_string();

		let existing_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some(key.clone()),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		common::wait_for_actor_propagation(&existing_actor_id, 1).await;

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key),
			false,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], existing_actor_id);
		common::assert_created_response(&body, false).await;
		let actor_id_str = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

#[test]
fn create_new_actor_when_none_exists() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "new-actor";
		let key = "new-key".to_string();

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key.clone()),
			false,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actor_id = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id");
		assert!(!actor_id.is_empty());
		assert_eq!(body["actor"]["name"], name);
		assert_eq!(body["actor"]["key"], json!(key));
		common::assert_created_response(&body, true).await;
	});
}

#[test]
fn create_actor_with_input_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "actor-with-input";
		let key = "input-key".to_string();
		let input = common::generate_test_input_data();

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key),
			false,
			None,
			Some(input),
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		common::assert_created_response(&body, true).await;
	});
}

#[test]
fn create_durable_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "durable-actor";
		let key = "durable-key".to_string();

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key),
			true,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["crash_policy"], "restart");
		common::assert_created_response(&body, true).await;
	});
}

#[test]
fn create_actor_in_specific_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "dc-specific-actor";
		let key = "dc-key".to_string();

		let response = common::get_or_create_actor(
			&namespace,
			name,
			Some(key),
			false,
			Some("dc-2"),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		common::assert_created_response(&body, true).await;
		let actor_id_str = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

// MARK: Error Cases

#[test]
fn get_or_create_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::get_or_create_actor(
			"non-existent-namespace",
			"test-actor",
			Some("key".to_string()),
			false,
			None,
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
fn get_or_create_invalid_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::get_or_create_actor(
			&namespace,
			"test-actor",
			Some("key".to_string()),
			false,
			Some("invalid-dc"),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(
			!response.status().is_success(),
			"Should fail with invalid datacenter"
		);
	});
}

#[test]
fn get_or_create_wrong_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace1, _, runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, runner2) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "cross-namespace-actor";
		let key = "key".to_string();

		let existing_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace1.clone(),
				name: name.to_string(),
				key: Some(key.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		let response = common::get_or_create_actor(
			&namespace2,
			name,
			Some(key),
			false,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_ne!(body["actor"]["actor_id"], existing_actor_id);
		common::assert_created_response(&body, true).await;
	});
}
