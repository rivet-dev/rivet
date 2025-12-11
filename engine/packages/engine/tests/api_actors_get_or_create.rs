mod common;

// MARK: Basic get-or-create tests

#[test]
fn get_or_create_creates_new_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "test-actor";
		let actor_key = "unique-key-1";

		// First call should create the actor
		let response = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: actor_key.to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(response.created, "Actor should be newly created");
		assert_eq!(response.actor.name, actor_name);
		assert_eq!(response.actor.key.as_ref().unwrap(), actor_key);
	});
}

#[test]
fn get_or_create_returns_existing_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "test-actor";
		let actor_key = "unique-key-2";

		// First call - create
		let response1 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: actor_key.to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(response1.created, "First call should create actor");
		let first_actor_id = response1.actor.actor_id;

		// Second call with same key - should return existing
		let response2 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: actor_key.to_string(),
				input: Some("different-input".to_string()), // Different input should be ignored
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(
			!response2.created,
			"Second call should return existing actor"
		);
		assert_eq!(
			response2.actor.actor_id, first_actor_id,
			"Should return the same actor ID"
		);
	});
}

#[test]
fn get_or_create_same_name_different_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "shared-name";

		// Create first actor with key1
		let response1 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: "key1".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor 1");

		// Create second actor with same name but different key
		let response2 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: "key2".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor 2");

		assert!(response1.created, "First actor should be created");
		assert!(response2.created, "Second actor should be created");
		assert_ne!(
			response1.actor.actor_id, response2.actor.actor_id,
			"Different keys should create different actors"
		);
		assert_eq!(response1.actor.name, actor_name);
		assert_eq!(response2.actor.name, actor_name);
	});
}

#[test]
fn get_or_create_idempotent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "idempotent-actor";
		let actor_key = "idempotent-key";

		// Make multiple calls with the same key
		let mut actor_id = None;
		for i in 0..5 {
			let response = common::api::public::actors_get_or_create(
				ctx.leader_dc().guard_port(),
				common::api::public::GetOrCreateQuery {
					namespace: namespace.clone(),
				},
				common::api::public::GetOrCreateRequest {
					datacenter: None,
					name: actor_name.to_string(),
					key: actor_key.to_string(),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to get or create actor");

			if i == 0 {
				assert!(response.created, "First call should create");
				actor_id = Some(response.actor.actor_id);
			} else {
				assert!(!response.created, "Subsequent calls should return existing");
				assert_eq!(
					response.actor.actor_id,
					actor_id.unwrap(),
					"All calls should return the same actor"
				);
			}
		}
	});
}

// MARK: Race condition tests

#[test]
fn get_or_create_race_condition_handling() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "race-actor";
		let actor_key = "race-key";
		let port = ctx.leader_dc().guard_port();
		let namespace_clone1 = namespace.clone();
		let namespace_clone2 = namespace.clone();

		// Launch two concurrent get_or_create requests with the same key
		let handle1 = tokio::spawn(async move {
			common::api::public::actors_get_or_create(
				port,
				common::api::public::GetOrCreateQuery {
					namespace: namespace_clone1,
				},
				common::api::public::GetOrCreateRequest {
					datacenter: None,
					name: actor_name.to_string(),
					key: actor_key.to_string(),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
		});

		let handle2 = tokio::spawn(async move {
			common::api::public::actors_get_or_create(
				port,
				common::api::public::GetOrCreateQuery {
					namespace: namespace_clone2,
				},
				common::api::public::GetOrCreateRequest {
					datacenter: None,
					name: actor_name.to_string(),
					key: actor_key.to_string(),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
		});

		let (result1, result2) = tokio::join!(handle1, handle2);
		let response1 = result1.expect("task 1 panicked").expect("request 1 failed");
		let response2 = result2.expect("task 2 panicked").expect("request 2 failed");

		// Both should succeed
		assert_eq!(
			response1.actor.actor_id, response2.actor.actor_id,
			"Both requests should return the same actor"
		);

		// Exactly one should have created=true
		let created_count = [response1.created, response2.created]
			.iter()
			.filter(|&&c| c)
			.count();
		assert_eq!(
			created_count, 1,
			"Exactly one request should report creation"
		);
	});
}

#[test]
fn get_or_create_returns_winner_on_race() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "race-winner-actor";
		let actor_key = "race-winner-key";
		let port = ctx.leader_dc().guard_port();

		// Launch multiple concurrent requests
		let mut handles = vec![];
		for _ in 0..10 {
			let namespace_clone = namespace.clone();
			let handle = tokio::spawn(async move {
				common::api::public::actors_get_or_create(
					port,
					common::api::public::GetOrCreateQuery {
						namespace: namespace_clone,
					},
					common::api::public::GetOrCreateRequest {
						datacenter: None,
						name: actor_name.to_string(),
						key: actor_key.to_string(),
						input: None,
						runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
						crash_policy: rivet_types::actors::CrashPolicy::Destroy,
					},
				)
				.await
			});
			handles.push(handle);
		}

		// Wait for all to complete
		let mut results = vec![];
		for handle in handles {
			let task_result = handle.await.expect("task panicked");
			// Handle destroyed_during_creation error which can occur in race conditions
			match task_result {
				Ok(response) => results.push(response),
				Err(e) => {
					// destroyed_during_creation is an expected race condition error
					if !e.to_string().contains("destroyed_during_creation") {
						panic!("unexpected error: {}", e);
					}
					// Skip this result and retry with get_or_create again
					let retry_result = common::api::public::actors_get_or_create(
						port,
						common::api::public::GetOrCreateQuery {
							namespace: namespace.clone(),
						},
						common::api::public::GetOrCreateRequest {
							datacenter: None,
							name: actor_name.to_string(),
							key: actor_key.to_string(),
							input: None,
							runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
							crash_policy: rivet_types::actors::CrashPolicy::Destroy,
						},
					)
					.await
					.expect("retry request failed");
					results.push(retry_result);
				}
			}
		}

		// All should return the same actor ID
		let first_actor_id = results[0].actor.actor_id;
		for result in &results {
			assert_eq!(
				result.actor.actor_id, first_actor_id,
				"All requests should return the same actor"
			);
		}

		// At least one request should report creation
		let created_count = results.iter().filter(|r| r.created).count();
		assert!(
			created_count >= 1,
			"At least one request should report creation"
		);
	});
}

#[test]
fn get_or_create_race_condition_across_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		const DC2_RUNNER_NAME: &'static str = "dc-2-runner";

		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let _runner2 = common::setup_runner(
			ctx.get_dc(2),
			&namespace,
			&format!("key-{:012x}", rand::random::<u64>()),
			1,
			20,
			Some(DC2_RUNNER_NAME.to_string()),
		)
		.await;

		let actor_name = "cross-dc-race-actor";
		let actor_key = "cross-dc-race-key";
		let port1 = ctx.leader_dc().guard_port();
		let port2 = ctx.get_dc(2).guard_port();
		let namespace_clone1 = namespace.clone();
		let namespace_clone2 = namespace.clone();

		// Launch concurrent requests from two different datacenters
		let handle1 = tokio::spawn(async move {
			common::api::public::actors_get_or_create(
				port1,
				common::api::public::GetOrCreateQuery {
					namespace: namespace_clone1,
				},
				common::api::public::GetOrCreateRequest {
					datacenter: None,
					name: actor_name.to_string(),
					key: actor_key.to_string(),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
		});

		let handle2 = tokio::spawn(async move {
			common::api::public::actors_get_or_create(
				port2,
				common::api::public::GetOrCreateQuery {
					namespace: namespace_clone2,
				},
				common::api::public::GetOrCreateRequest {
					datacenter: None,
					name: actor_name.to_string(),
					key: actor_key.to_string(),
					input: None,
					runner_name_selector: DC2_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
		});

		let (result1, result2) = tokio::join!(handle1, handle2);
		let response1 = result1
			.expect("DC1 task panicked")
			.expect("DC1 request failed");
		let response2 = result2
			.expect("DC2 task panicked")
			.expect("DC2 request failed");

		// Both should succeed and return the same actor
		assert_eq!(
			response1.actor.actor_id, response2.actor.actor_id,
			"Both datacenters should return the same actor"
		);

		// At least one should report creation
		assert!(
			(response1.created || response2.created) && !(response1.created && response2.created),
			"At least one datacenter should report creation, but not both"
		);
	});
}

// MARK: Datacenter tests

#[test]
fn get_or_create_in_current_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None, // Should default to current DC
				name: "current-dc-actor".to_string(),
				key: "current-dc-key".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(response.created, "Actor should be created");

		// Verify actor is in current DC (DC1)
		let actor_id_str = response.actor.actor_id.to_string();
		common::assert_actor_in_dc(&actor_id_str, 1).await;
	});
}

#[test]
fn get_or_create_in_remote_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Request from DC1 but specify DC2
		let response = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: "remote-dc-actor".to_string(),
				key: "remote-dc-key".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(response.created, "Actor should be created");

		// Wait for actor to propagate across datacenters
		let actor_id_str = response.actor.actor_id.to_string();

		// Verify actor is in DC2
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

// MARK: Error cases

#[test]
fn get_or_create_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let res = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: "non-existent-namespace".to_string(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: "test-key".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(res.is_err(), "Should fail with non-existent namespace");
	});
}

#[test]
fn get_or_create_with_invalid_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let res = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: Some("non-existent-dc".to_string()),
				name: "test-actor".to_string(),
				key: "test-key".to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await;

		assert!(res.is_err(), "Should fail with invalid datacenter");
	});
}

// MARK: Edge cases

#[test]
fn get_or_create_with_destroyed_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "destroyed-actor";
		let actor_key = "destroyed-key";

		// Create actor
		let response1 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: actor_key.to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor");

		assert!(response1.created, "First call should create actor");
		let first_actor_id = response1.actor.actor_id;

		// Destroy the actor
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: first_actor_id,
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Call get_or_create again with same key - should create a new actor
		let response2 = common::api::public::actors_get_or_create(
			ctx.leader_dc().guard_port(),
			common::api::public::GetOrCreateQuery {
				namespace: namespace.clone(),
			},
			common::api::public::GetOrCreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: actor_key.to_string(),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to get or create actor after destroy");

		assert!(
			response2.created,
			"Should create new actor after old one was destroyed"
		);
		assert_ne!(
			response2.actor.actor_id, first_actor_id,
			"Should be a different actor ID"
		);
	});
}
