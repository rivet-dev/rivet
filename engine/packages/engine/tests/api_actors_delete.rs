mod common;

// MARK: Basic
#[test]
fn delete_existing_actor_with_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Verify actor exists
		common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;

		// Delete the actor with namespace
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Verify actor is destroyed
		common::assert_actor_is_destroyed(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await;
	});
}

#[test]
fn delete_existing_actor_without_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Verify actor exists
		common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;

		// Delete the actor without namespace parameter
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery { namespace: None },
		)
		.await
		.expect("failed to delete actor");

		// Verify actor is destroyed
		common::assert_actor_is_destroyed(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await;
	});
}

#[test]
fn delete_actor_current_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor in current datacenter
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Delete the actor
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Verify actor is destroyed
		common::assert_actor_is_destroyed(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await;
	});
}

#[test]
fn delete_actor_remote_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor in DC2
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

		// Delete the actor from DC1 (will route to DC2)
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Verify actor is destroyed in DC2
		common::assert_actor_is_destroyed(ctx.get_dc(2).guard_port(), &actor_id, &namespace).await;
	});
}

// MARK: Error cases

#[test]
fn delete_non_existent_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (_namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Generate a fake actor ID with valid format but non-existent
		let fake_actor_id = rivet_util::Id::new_v1(ctx.leader_dc().config.dc_label());

		let res = common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: fake_actor_id,
			},
			common::api_types::actors::delete::DeleteQuery { namespace: None },
		)
		.await;

		assert!(res.is_err(), "should fail to delete non-existent actor");
	});
}

#[test]
fn delete_actor_wrong_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace1, _, _runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, _runner2) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor in namespace1
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace1.clone(),
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
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Try to delete with namespace2
		let res = common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace2.clone()),
			},
		)
		.await;

		assert!(
			res.is_err(),
			"should fail to delete actor with wrong namespace"
		);

		// Verify actor still exists in namespace1
		common::assert_actor_is_alive(ctx.leader_dc().guard_port(), &actor_id, &namespace1).await;
	});
}

#[test]
fn delete_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Try to delete with non-existent namespace
		let res = common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some("non-existent-namespace".to_string()),
			},
		)
		.await;

		assert!(res.is_err(), "should fail with non-existent namespace");
	});
}

// Note: Invalid actor ID format test removed because it would be caught at parsing level
// before the API call, and the API already validates UUID format in the path parameter

// MARK: Cross-datacenter tests

#[test]
fn delete_remote_actor_verify_propagation() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor in DC2
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

		// Verify actor exists in both datacenters
		common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		common::assert_actor_exists(ctx.get_dc(2).guard_port(), &actor_id, &namespace).await;

		// Delete the actor from DC1
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Verify actor is destroyed in both datacenters
		common::assert_actor_is_destroyed(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await;
		common::assert_actor_is_destroyed(ctx.get_dc(2).guard_port(), &actor_id, &namespace).await;
	});
}

// MARK: Edge cases

#[test]
fn delete_already_destroyed_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Delete the actor once
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Delete the actor again - should handle gracefully (WorkflowNotFound)
		// The implementation logs a warning but doesn't error when workflow is not found
		let res = common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await;

		// Should succeed even though actor was already destroyed
		assert!(
			res.is_ok(),
			"deleting already destroyed actor should succeed gracefully"
		);
	});
}

#[test]
fn delete_actor_twice_rapidly() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create an actor
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let actor_id = res.actor.actor_id.to_string();

		// Send two delete requests in rapid succession
		let actor_id_clone = actor_id.clone();
		let namespace_clone = namespace.clone();
		let port = ctx.leader_dc().guard_port();

		let delete1 = tokio::spawn(async move {
			common::api::public::actors_delete(
				port,
				common::api_types::actors::delete::DeletePath {
					actor_id: actor_id.parse().expect("failed to parse actor_id"),
				},
				common::api_types::actors::delete::DeleteQuery {
					namespace: Some(namespace.clone()),
				},
			)
			.await
		});

		let delete2 = tokio::spawn(async move {
			common::api::public::actors_delete(
				port,
				common::api_types::actors::delete::DeletePath {
					actor_id: actor_id_clone.parse().expect("failed to parse actor_id"),
				},
				common::api_types::actors::delete::DeleteQuery {
					namespace: Some(namespace_clone.clone()),
				},
			)
			.await
		});

		// Both should complete without panicking
		let (res1, res2) = tokio::join!(delete1, delete2);

		// At least one should succeed
		let res1 = res1.expect("task should not panic");
		let res2 = res2.expect("task should not panic");

		// Both requests should succeed or fail gracefully (no panics)
		assert!(
			res1.is_ok() || res2.is_ok(),
			"at least one delete should succeed in race condition"
		);
	});
}
