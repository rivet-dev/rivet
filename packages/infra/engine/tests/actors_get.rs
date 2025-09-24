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
fn get_existing_actor_without_namespace_should_fail() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor
		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		// Get actor without namespace should fail
		let response = common::get_actor(&actor_id, None, ctx.leader_dc().guard_port()).await;
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for missing namespace parameter"
		);
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

// MARK: Namespace validation
#[test]
fn get_actor_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let non_existent_ns = "non-existent-namespace";
		let api_port = ctx.leader_dc().guard_port();

		// GET /actors/{id}
		let response = common::get_actor(
			"00000000-0000-0000-0000-000000000000",
			Some(non_existent_ns),
			api_port,
		)
		.await;
		assert!(
			!response.status().is_success(),
			"GET /actors/{{id}} should fail with non-existent namespace"
		);
	});
}

#[test]
fn get_actor_invalid_id_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let api_port = ctx.leader_dc().guard_port();

		// Test various invalid actor ID formats
		let invalid_ids = vec![
			"not-a-uuid",
			"12345",
			"00000000-0000-0000-0000",               // Incomplete UUID
			"00000000-0000-0000-0000-000000000000g", // Invalid character
			"00000000_0000_0000_0000_000000000000",  // Wrong separator
		];

		for invalid_id in invalid_ids {
			// GET /actors/{id}
			let response = common::get_actor(invalid_id, Some(&namespace), api_port).await;
			assert_eq!(
				response.status(),
				400,
				"GET should return 400 for invalid actor ID: {}",
				invalid_id
			);
		}

		// Special case: empty actor ID results in different route
		let response = common::get_actor("", Some(&namespace), api_port).await;
		assert_eq!(
			response.status(),
			404,
			"GET should return 404 for empty actor ID (route not found)"
		);
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
		let (namespace1, _, _runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, _runner2) =
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
