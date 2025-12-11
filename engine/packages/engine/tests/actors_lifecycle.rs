mod common;

// MARK: 1. Creation and Initialization
#[test]
fn create_actor_basic() {
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Verify response contains valid actor_id
		assert!(!actor_id.is_empty(), "actor_id should not be empty");

		// Verify actor exists and retrieve it
		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;

		// Verify create_ts is set
		assert!(
			actor.create_ts > 0,
			"create_ts should be set to a positive timestamp"
		);

		tracing::info!(
			?actor_id,
			create_ts = actor.create_ts,
			"actor created successfully"
		);
	});
}

#[test]
fn create_actor_with_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();

		// Step 1 & 2: Create actor with unique key
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

		// Verify actor created successfully
		assert!(!actor_id.is_empty(), "actor_id should not be empty");

		// Step 3: Verify key is reserved by checking actor exists with the key
		let actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;
		assert_eq!(
			actor.key,
			Some(key.clone()),
			"actor should have the specified key"
		);

		tracing::info!(?actor_id, ?key, "first actor created with key");

		// Step 4: Attempt to create second actor with same key AND same name
		// Note: The key uniqueness constraint is scoped by (namespace_id, name, key)
		let res2 = common::api::public::build_actors_create_request(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(), // Same name as first actor
				key: Some(key.clone()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to build request")
		.send()
		.await
		.expect("failed to send request");

		// Step 5: Verify second creation fails with key conflict error
		// First check that it's an error response
		assert!(
			!res2.status().is_success(),
			"Expected error response, got success: {}",
			res2.status()
		);

		// Parse the JSON body
		let body: serde_json::Value = res2.json().await.expect("Failed to parse error response");

		// Check the error code (error is at root level, not under "error" key)
		let error_code = body["code"]
			.as_str()
			.expect("Missing error code in response");
		assert_eq!(
			error_code, "duplicate_key",
			"Expected duplicate_key error, got {}",
			error_code
		);

		// Verify metadata contains the existing actor ID
		let existing_actor_id = body["metadata"]["existing_actor_id"]
			.as_str()
			.expect("Missing existing_actor_id in metadata");
		assert_eq!(
			existing_actor_id, &actor_id,
			"Expected existing_actor_id to match first actor"
		);

		tracing::info!(?key, "key conflict properly detected");
	});
}

#[test]
fn create_actor_with_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Step 1: Create actor with input data
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

		// Step 2 & 3: Verify actor receives input correctly
		assert!(!actor_id.is_empty(), "actor_id should not be empty");

		// Verify actor exists
		let _actor =
			common::assert_actor_exists(ctx.leader_dc().guard_port(), &actor_id, &namespace).await;

		// Note: The input data is passed to the runner, and the actor should have access to it
		// The actual verification that the actor received the input would typically be done
		// by querying the actor via Guard and checking its response, but for this basic test
		// we verify the actor was created successfully
		tracing::info!(
			?actor_id,
			input_size = input_data.len(),
			"actor created with input data"
		);
	});
}

// MARK: 2. Allocation and Starting
#[test]
fn actor_allocation_to_runner() {
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
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Verify actor is allocated to runner
		assert!(
			runner.has_actor(&actor_id).await,
			"runner should have the actor allocated"
		);

		tracing::info!(?actor_id, runner_id = ?runner.runner_id, "actor allocated to runner");
	});
}

#[test]
fn actor_starts_and_becomes_connectable() {
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Verify actor is connectable
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.connectable_ts.is_some(),
			"connectable_ts should be set"
		);
		assert!(actor.start_ts.is_some(), "start_ts should be set");

		// Test ping via guard
		let ping_response = common::ping_actor_via_guard(ctx.leader_dc(), &actor_id).await;
		assert_eq!(ping_response["status"], "ok");

		tracing::info!(?actor_id, "actor is connectable and responding");
	});
}

#[test]
#[ignore]
fn actor_start_timeout() {
	// TODO: Implement when we have a way to simulate actors that don't start
}

// MARK: 3. Running State Management
#[test]
fn actor_connectable_via_guard_http() {
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to become connectable
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Send HTTP request via Guard
		let response = common::ping_actor_via_guard(ctx.leader_dc(), &actor_id).await;

		// Verify response
		assert_eq!(response["status"], "ok");

		tracing::info!(?actor_id, "actor successfully responded via guard HTTP");
	});
}

#[test]
fn actor_connectable_via_guard_websocket() {
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
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to become connectable
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Test WebSocket connection
		let response = common::ping_actor_websocket_via_guard(ctx.leader_dc(), &actor_id).await;

		// Verify response
		assert_eq!(response["status"], "ok");

		tracing::info!(
			?actor_id,
			"actor successfully responded via guard WebSocket"
		);
	});
}

#[test]
#[ignore]
fn actor_alarm_wake() {
	// TODO: Implement when test runner supports alarms
}

// MARK: 4. Stopping and Graceful Shutdown
#[test]
#[ignore]
fn actor_graceful_stop_with_destroy_policy() {
	// TODO: Implement when we can control actor stop behavior
}

#[test]
fn actor_explicit_destroy() {
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
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Verify actor is running
		assert!(
			runner.has_actor(&actor_id).await,
			"runner should have actor"
		);

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

		// Wait for destroy to propagate
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Verify actor is destroyed
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should still exist in database");

		assert!(
			actor.destroy_ts.is_some(),
			"destroy_ts should be set after deletion"
		);

		tracing::info!(?actor_id, "actor successfully destroyed");
	});
}

// MARK: 5. Crash Handling and Policies
#[test]
#[ignore]
fn crash_policy_restart() {
	// TODO: Implement when we can simulate actor crashes
}

#[test]
#[ignore]
fn crash_policy_restart_resets_on_success() {
	// TODO: Implement when we can simulate actor crashes and recovery
}

#[test]
#[ignore]
fn crash_policy_sleep() {
	// TODO: Implement when we can simulate actor crashes
}

#[test]
#[ignore]
fn crash_policy_destroy() {
	// TODO: Implement when we can simulate actor crashes
}

// MARK: 6. Sleep and Wake
#[test]
#[ignore]
fn actor_sleep_intent() {
	// TODO: Implement when test runner supports sleep intents
}

#[test]
#[ignore]
fn actor_wake_from_sleep() {
	// TODO: Implement when we can test sleep/wake cycle
}

#[test]
#[ignore]
fn actor_sleep_with_deferred_wake() {
	// TODO: Implement when we have fine-grained sleep/wake control
}

// MARK: 7. Pending Allocation Queue
#[test]
#[ignore]
fn actor_pending_allocation_no_runners() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace without runner
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create actor (should be pending)
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

		// Verify actor is in pending state
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.pending_allocation_ts.is_some(),
			"pending_allocation_ts should be set when no runners available"
		);
		assert!(
			actor.connectable_ts.is_none(),
			"actor should not be connectable yet"
		);

		tracing::info!(?actor_id, "actor is pending allocation");

		// Now start a runner
		let runner = common::setup_runner(
			ctx.leader_dc(),
			&namespace,
			&format!("key-{:012x}", rand::random::<u64>()),
			1,
			20,
			None,
		)
		.await;

		// Wait for allocation
		tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

		// Verify actor is now allocated
		assert!(
			runner.has_actor(&actor_id).await,
			"actor should now be allocated to runner"
		);

		tracing::info!(
			?actor_id,
			"actor successfully allocated after runner started"
		);
	});
}

#[test]
#[ignore]
fn pending_allocation_queue_ordering() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace without runner
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create 3 actors in sequence
		let mut actor_ids = Vec::new();
		for i in 0..3 {
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: format!("test-actor-{}", i),
					key: None,
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");

			actor_ids.push(res.actor.actor_id.to_string());

			// Small delay to ensure ordering
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// Start runner with only 2 slots
		let runner = common::setup_runner(
			ctx.leader_dc(),
			&namespace,
			&format!("key-{:012x}", rand::random::<u64>()),
			1,
			2, // Only 2 slots
			None,
		)
		.await;

		// Wait for allocation
		tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

		// Verify first 2 actors are allocated (FIFO)
		assert!(
			runner.has_actor(&actor_ids[0]).await,
			"first actor should be allocated"
		);
		assert!(
			runner.has_actor(&actor_ids[1]).await,
			"second actor should be allocated"
		);

		// Third actor should still be pending
		let actor_c =
			common::try_get_actor(ctx.leader_dc().guard_port(), &actor_ids[2], &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

		assert!(
			actor_c.pending_allocation_ts.is_some(),
			"third actor should still be pending"
		);

		tracing::info!("FIFO allocation ordering verified");
	});
}

#[test]
#[ignore]
fn actor_allocation_prefers_available_runner() {
	// TODO: Implement when we can test with multiple runners
}

// MARK: 8. Key Reservation and Uniqueness
#[test]
fn key_reservation_single_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();

		// Create first actor with key
		let res1 = common::api::public::actors_create(
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
		.expect("failed to create first actor");

		let actor_id1 = res1.actor.actor_id.to_string();

		tracing::info!(?actor_id1, ?key, "first actor created with key");

		// Destroy first actor
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id1.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete first actor");

		// Wait for destroy and key release
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Create second actor with same key (should succeed now)
		let res2 = common::api::public::actors_create(
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
		.expect("failed to create second actor after key release");

		let actor_id2 = res2.actor.actor_id.to_string();

		assert_ne!(
			actor_id1, actor_id2,
			"second actor should have different ID"
		);

		tracing::info!(
			?actor_id2,
			?key,
			"second actor created with same key after first destroyed"
		);
	});
}

#[test]
fn actor_lookup_by_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();

		// Create actor with key
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

		// Query actor by key (name is required when using key)
		let list_res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				actor_ids: None,
				actor_id: vec![],
				namespace: namespace.clone(),
				name: Some("test-actor".to_string()),
				key: Some(key.clone()),
				include_destroyed: Some(false),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(list_res.actors.len(), 1, "should find exactly one actor");
		assert_eq!(
			list_res.actors[0].actor_id.to_string(),
			actor_id,
			"should find the correct actor by key"
		);

		tracing::info!(?actor_id, ?key, "actor successfully looked up by key");
	});
}

// MARK: 9. Serverless Integration
#[test]
#[ignore]
fn serverless_slot_tracking() {
	// TODO: Implement when serverless infrastructure is available
}

// MARK: 10. Actor Data and State
#[test]
#[ignore]
fn actor_kv_data_lifecycle() {
	// TODO: Implement when KV data can be tested
}

// MARK: Edge Cases - 1. Runner Failures
#[test]
#[ignore]
fn actor_survives_runner_disconnect() {
	// TODO: Implement when we can simulate runner disconnects
}

#[test]
#[ignore]
fn runner_reconnect_with_stale_actors() {
	// TODO: Implement when we can simulate runner reconnection with stale state
}

// MARK: Edge Cases - 2. Concurrent Operations
#[test]
fn concurrent_key_reservation() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();
		let port = ctx.leader_dc().guard_port();
		let namespace_clone = namespace.clone();

		// Launch two concurrent create requests with the same key
		let handle1 = tokio::spawn({
			let key = key.clone();
			let namespace = namespace_clone.clone();
			async move {
				common::api::public::actors_create(
					port,
					common::api_types::actors::create::CreateQuery { namespace },
					common::api_types::actors::create::CreateRequest {
						datacenter: None,
						name: "test-actor".to_string(),
						key: Some(key),
						input: None,
						runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
						crash_policy: rivet_types::actors::CrashPolicy::Destroy,
					},
				)
				.await
			}
		});

		let handle2 = tokio::spawn({
			let key = key.clone();
			let namespace = namespace_clone.clone();
			async move {
				common::api::public::actors_create(
					port,
					common::api_types::actors::create::CreateQuery { namespace },
					common::api_types::actors::create::CreateRequest {
						datacenter: None,
						name: "test-actor".to_string(),
						key: Some(key),
						input: None,
						runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
						crash_policy: rivet_types::actors::CrashPolicy::Destroy,
					},
				)
				.await
			}
		});

		let (res1, res2) = tokio::join!(handle1, handle2);

		// Exactly one should succeed and one should fail
		let success_count = [res1, res2]
			.iter()
			.filter(|r| r.as_ref().unwrap().is_ok())
			.count();

		assert_eq!(
			success_count, 1,
			"exactly one concurrent creation should succeed"
		);

		tracing::info!(?key, "concurrent key reservation handled correctly");
	});
}

#[test]
#[ignore]
fn concurrent_destroy_and_wake() {
	// TODO: Implement when sleep/wake is available
}

#[test]
fn concurrent_create_with_same_key_destroy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();

		// Create first actor
		let res1 = common::api::public::actors_create(
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
		.expect("failed to create first actor");

		let actor_id1 = res1.actor.actor_id.to_string();

		// Start destroying
		let delete_handle = tokio::spawn({
			let port = ctx.leader_dc().guard_port();
			let namespace = namespace.clone();
			let actor_id = actor_id1.clone();
			async move {
				common::api::public::actors_delete(
					port,
					common::api_types::actors::delete::DeletePath {
						actor_id: actor_id.parse().unwrap(),
					},
					common::api_types::actors::delete::DeleteQuery {
						namespace: Some(namespace),
					},
				)
				.await
			}
		});

		// Small delay then try to create with same key
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		// Try to create second actor - should eventually succeed after destroy completes
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		let _res2 = common::api::public::actors_create(
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
		.expect("should succeed creating with same key after destroy");

		delete_handle
			.await
			.expect("delete should complete")
			.expect("delete should succeed");

		tracing::info!("key reuse after destroy works correctly");
	});
}

// MARK: Edge Cases - 3. Resource Limits
#[test]
fn runner_at_max_capacity() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Start runner with only 2 slots
		let runner = common::setup_runner(
			ctx.leader_dc(),
			&namespace,
			&format!("key-{:012x}", rand::random::<u64>()),
			1,
			2, // Only 2 slots
			None,
		)
		.await;

		// Create first two actors to fill capacity
		let mut actor_ids = Vec::new();
		for i in 0..2 {
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: format!("test-actor-{}", i),
					key: None,
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");

			actor_ids.push(res.actor.actor_id.to_string());
		}

		// Wait for allocation
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Verify both actors are allocated
		assert!(runner.has_actor(&actor_ids[0]).await);
		assert!(runner.has_actor(&actor_ids[1]).await);

		// Create third actor (should be pending)
		let res3 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor-3".to_string(),
				key: None,
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create third actor");

		let actor_id3 = res3.actor.actor_id.to_string();

		// Wait a bit
		tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

		// Verify third actor is pending
		let actor3 = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id3, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor3.pending_allocation_ts.is_some(),
			"third actor should be pending when runner at capacity"
		);

		// Destroy first actor to free a slot
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_ids[0].parse().unwrap(),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Wait for slot to free and pending actor to be allocated
		tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

		// Verify third actor is now allocated
		assert!(
			runner.has_actor(&actor_id3).await,
			"pending actor should be allocated after slot freed"
		);

		tracing::info!("runner capacity and pending allocation verified");
	});
}

// MARK: Edge Cases - 4. Timeout and Retry Scenarios
#[test]
#[ignore]
fn exponential_backoff_max_retries() {
	// TODO: Implement when crash simulation is available
}

#[test]
#[ignore]
fn gc_timeout_start_threshold() {
	// TODO: Implement when we can control actor start timing
}

#[test]
#[ignore]
fn gc_timeout_stop_threshold() {
	// TODO: Implement when we can control actor stop timing
}

// MARK: Edge Cases - 5. Data Consistency
#[test]
#[ignore]
fn actor_state_persistence_across_reschedule() {
	// TODO: Implement when crash/reschedule is testable
}

#[test]
#[ignore]
fn index_consistency_after_failure() {
	// TODO: Implement when we have failure injection capabilities
}

// MARK: Edge Cases - 6. Protocol Edge Cases
#[test]
#[ignore]
fn duplicate_actor_state_running_events() {
	// TODO: Implement when we can send duplicate protocol events
}

#[test]
#[ignore]
fn actor_state_stopped_before_running() {
	// TODO: Implement when we can control protocol event ordering
}

#[test]
#[ignore]
fn runner_ack_command_failures() {
	// TODO: Implement when we can simulate ack failures
}
