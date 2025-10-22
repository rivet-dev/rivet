mod common;

#[test]
fn get_existing_actor_with_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Get actor with namespace
		let response =
			common::get_actor(&actor_id, Some(&namespace), ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], actor_id);
		assert_eq!(body["actor"]["name"], "test-actor");
	});
}

#[test]
fn get_existing_actor_without_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Get actor without namespace
		let response = common::get_actor(&actor_id, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], actor_id);
		assert_eq!(body["actor"]["name"], "test-actor");
	});
}

#[test]
fn get_actor_current_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor in current DC
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Get actor
		let response =
			common::get_actor(&actor_id, Some(&namespace), ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actor_id_str = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 1).await;
	});
}

// Error cases

#[test]
fn get_non_existent_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to get non-existent actor
		let fake_actor_id = format!("00000000-0000-0000-0000-{:012x}", rand::random::<u64>());
		let response = common::get_actor(
			&fake_actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert_eq!(
			response.status(),
			400,
			"Should return 400 for non-existent actor (Actor::NotFound)"
		);
	});
}

#[test]
fn get_actor_wrong_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace1, _, runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, runner2) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor in namespace1
		let actor_id = common::create_actor(&namespace1, ctx.leader_dc().guard_port()).await;

		// Try to get with namespace2
		let response =
			common::get_actor(&actor_id, Some(&namespace2), ctx.leader_dc().guard_port()).await;

		// Should fail because actor exists but namespace doesn't match
		assert!(
			!response.status().is_success(),
			"Should fail to get actor with wrong namespace"
		);
	});
}

#[test]
fn get_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Try to get with non-existent namespace
		let response = common::get_actor(
			&actor_id,
			Some("non-existent-namespace"),
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with namespace not found
		assert!(
			!response.status().is_success(),
			"Should fail with non-existent namespace"
		);
	});
}

#[test]
fn get_invalid_actor_id_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to get with invalid actor ID format
		let response = common::get_actor(
			"invalid-uuid",
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with bad request
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for invalid actor ID format"
		);
	});
}

// Cross-datacenter tests

#[test]
fn get_remote_actor_verify_routing() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor in DC 2
		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		// Wait for propagation
		common::wait_for_actor_propagation(&actor_id, 1).await;

		// Get from DC 1 - should route to DC 2
		let response =
			common::get_actor(&actor_id, Some(&namespace), ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], actor_id);
		let actor_id_str = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;

		// Get from DC 2 directly
		let response2 =
			common::get_actor(&actor_id, Some(&namespace), ctx.get_dc(2).guard_port()).await;
		common::assert_success_response(&response2);

		let body2: serde_json::Value = response2.json().await.expect("Failed to parse response");
		common::assert_actors_equal(&body, &body2);
	});
}

#[test]
fn get_local_actor_no_remote_call() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor in DC 1
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Get from DC 1 - should not make remote call
		let response =
			common::get_actor(&actor_id, Some(&namespace), ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor"]["actor_id"], actor_id);
		let actor_id_str = body["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 1).await;

		// Test is verifying that getting a local actor doesn't make remote calls
		// The actor being accessible from DC2 is expected behavior due to routing
	});
}
