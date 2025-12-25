mod common;

// MARK: Basic
#[test]
fn create_actor_valid_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: runner.name().to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;

		// TODO: Hook into engine instead of sleep
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;

		assert!(
			runner.has_actor(&actor_id).await,
			"runner should have the actor"
		);
	});
}

#[test]
fn create_actor_with_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: Some(key.clone()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		assert!(!actor_id.is_empty(), "actor ID should not be empty");

		// Verify actor exists
		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		assert_eq!(actor.key, Some(key));
	});
}

#[test]
fn create_actor_with_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let input_data = common::generate_test_input_data();
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: Some(input_data.clone()),
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		assert!(!actor_id.is_empty(), "actor ID should not be empty");
	});
}

#[test]
fn create_durable_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Restart,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		assert!(!actor_id.is_empty(), "actor ID should not be empty");

		// Verify actor is durable
		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		assert_eq!(
			actor.crash_policy,
			rivet_types::actors::CrashPolicy::Restart
		);
	});
}

#[test]
fn create_actor_specific_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		assert!(!actor_id.is_empty(), "actor ID should not be empty");

		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		common::assert_actor_in_dc(&actor.actor_id.to_string(), 2).await;
	});
}

// MARK: Error cases
#[test]
fn create_actor_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: "non-existent-namespace".to_string(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(
			res.is_err(),
			"should fail to create actor with non-existent namespace"
		);
	});
}

#[test]
fn create_actor_invalid_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("invalid-dc".to_string()),
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(
			res.is_err(),
			"should fail to create actor with invalid datacenter"
		);
	});
}

// MARK: Cross-datacenter tests
#[test]
fn create_actor_remote_datacenter_verify() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: "test-actor".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		let actor =
			common::assert_actor_exists(ctx.get_dc(2).guard_port(), &actor_id, &namespace).await;
		common::assert_actor_in_dc(&actor.actor_id.to_string(), 2).await;
	});
}

// MARK: Input validation tests
// Note: Input at exactly 4 MiB is tested, but the HTTP layer has a body limit
// that may be lower than the validation limit. The validation is still tested
// by the exceeds test below.

#[test]
fn create_actor_input_large() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create a large input (1 MiB) that should succeed
		let input_size = 1024 * 1024;
		let input_data = "a".repeat(input_size);

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: Some(input_data),
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("should succeed with large input");

		let actor_id = res.actor.actor_id.to_string();
		assert!(!actor_id.is_empty(), "actor ID should not be empty");
	});
}

#[test]
fn create_actor_input_exceeds_max_size() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create input exceeding 4 MiB
		let max_input_size = 4 * 1024 * 1024;
		let input_data = "a".repeat(max_input_size + 1);

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: Some(input_data),
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(
			res.is_err(),
			"should fail to create actor with input exceeding max size"
		);
	});
}

// MARK: Key validation tests
#[test]
fn create_actor_empty_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: Some("".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(res.is_err(), "should fail to create actor with empty key");
	});
}

#[test]
fn create_actor_key_at_max_size() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create key of exactly 1024 bytes
		let key = "a".repeat(1024);

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: Some(key.clone()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("should succeed with key at max size");

		let actor_id = res.actor.actor_id.to_string();
		assert!(!actor_id.is_empty(), "actor ID should not be empty");

		// Verify actor exists with correct key
		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		assert_eq!(actor.key, Some(key));
	});
}

#[test]
fn create_actor_key_exceeds_max_size() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create key exceeding 1024 bytes
		let key = "a".repeat(1025);

		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: Some(key),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(
			res.is_err(),
			"should fail to create actor with key exceeding max size"
		);
	});
}
