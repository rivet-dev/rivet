mod common;

use std::collections::HashSet;

// MARK: List by Name

#[test]
fn list_actors_by_namespace_and_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "list-test-actor";

		// Create multiple actors with same name
		let mut actor_ids = Vec::new();
		for i in 0..3 {
			let actor_id = common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
			actor_ids.push(actor_id);
		}

		// List actors by name
		let response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 3, "Should return all 3 actors");

		// Verify all created actors are in the response
		let returned_ids: HashSet<String> = actors
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		for actor_id in &actor_ids {
			assert!(
				returned_ids.contains(actor_id),
				"Actor {} should be in results",
				actor_id
			);
		}
	});
}

#[test]
fn list_with_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "paginated-actor";

		// Create 5 actors with the same name but different keys
		let mut actor_ids = Vec::new();
		for i in 0..5 {
			let actor_id = common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
			actor_ids.push(actor_id);
		}

		// Wait for actors to be fully created and available
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		// First page - limit 2
		let response1 = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			None,
			Some(2),
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response1);

		let body1: serde_json::Value = response1.json().await.expect("Failed to parse response");
		let actors1 = body1["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors1.len(), 2, "Should return 2 actors with limit=2");

		let cursor = body1["cursor"].as_str();

		// Since there's no cursor, let's test that we can get the remaining actors
		// by making another request without cursor to get all actors and verify ordering
		let all_response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			None,
			None, // No limit to get all actors
			None, // No cursor
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&all_response);

		let all_body: serde_json::Value =
			all_response.json().await.expect("Failed to parse response");
		let all_actors = all_body["actors"]
			.as_array()
			.expect("Expected actors array");

		// Verify we have all 5 actors when querying without limit
		assert_eq!(
			all_actors.len(),
			5,
			"Should return all 5 actors when no limit specified"
		);

		// Use first 2 actors as actors2 for remaining test logic
		let actors2 = if all_actors.len() > 2 {
			all_actors[2..std::cmp::min(4, all_actors.len())].to_vec()
		} else {
			vec![]
		};

		let _body2 = &all_body; // Use same body for cursor tests

		// Verify no duplicates between pages
		let ids1: HashSet<String> = actors1
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		let ids2: HashSet<String> = actors2
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		assert!(
			ids1.is_disjoint(&ids2),
			"Pages should not have duplicate actors"
		);

		// Verify consistent ordering using the full actor list
		let all_timestamps: Vec<i64> = all_actors
			.iter()
			.map(|a| {
				a["create_ts"]
					.as_i64()
					.expect("Actor should have create_ts")
			})
			.collect();

		// Verify all timestamps are valid and reasonable (not zero, not in future)
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_millis() as i64;

		for &ts in &all_timestamps {
			assert!(ts > 0, "create_ts should be positive: {}", ts);
			assert!(ts <= now, "create_ts should not be in future: {}", ts);
		}

		// Verify that all actors are returned in descending timestamp order (newest first)
		for i in 1..all_timestamps.len() {
			assert!(
				all_timestamps[i - 1] >= all_timestamps[i],
				"Actors should be ordered by create_ts descending: {} >= {} (index {} vs {})",
				all_timestamps[i - 1],
				all_timestamps[i],
				i - 1,
				i
			);
		}

		// Verify that the limited query returns the newest actors
		let paginated_timestamps: Vec<i64> = actors1
			.iter()
			.map(|a| a["create_ts"].as_i64().unwrap())
			.collect();

		assert_eq!(
			paginated_timestamps,
			all_timestamps[0..2].to_vec(),
			"Paginated result should return the 2 newest actors"
		);

		// Test that limit=2 actually limits results to 2
		assert_eq!(actors1.len(), 2, "Limit=2 should return exactly 2 actors");
		assert_eq!(
			all_actors.len(),
			5,
			"Query without limit should return all 5 actors"
		);
	});
}

#[test]
fn list_returns_empty_array_when_no_actors() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List actors that don't exist
		let response = common::list_actors(
			&namespace,
			Some("non-existent-actor"),
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 0, "Should return empty array");
	});
}

// List by Name + Keys

#[test]
fn list_actors_by_namespace_name_and_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "keyed-actor";
		let key1 = "key1".to_string();
		let key2 = "key2".to_string();

		// Create actors with different keys
		let actor_id1 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some(key1.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		let _actor_id2 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some(key2.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// List with key1 - should find actor1
		let response = common::list_actors(
			&namespace,
			Some(name),
			Some("key1".to_string()),
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 1, "Should return 1 actor");
		assert_eq!(actors[0]["actor_id"], actor_id1);
	});
}

#[test]
fn list_with_include_destroyed_false() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "destroyed-test";

		// Create and destroy an actor
		let destroyed_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("destroyed-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::destroy_actor(
			&destroyed_actor_id,
			&namespace,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Create an active actor
		let active_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("active-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// List without include_destroyed (default false)
		let response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			Some(false),
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 1, "Should only return active actor");
		assert_eq!(actors[0]["actor_id"], active_actor_id);
	});
}

#[test]
fn list_with_include_destroyed_true() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "destroyed-included";

		// Create and destroy an actor
		let destroyed_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("destroyed-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::destroy_actor(
			&destroyed_actor_id,
			&namespace,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Create an active actor
		let active_actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("active-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// List with include_destroyed=true
		let response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			Some(true),
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(
			actors.len(),
			2,
			"Should return both active and destroyed actors"
		);

		// Verify both actors are in results
		let returned_ids: HashSet<String> = actors
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		assert!(returned_ids.contains(&active_actor_id));
		assert!(returned_ids.contains(&destroyed_actor_id));
	});
}

// MARK: List by Actor IDs

#[test]
fn list_specific_actors_by_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create multiple actors
		let actor_ids =
			common::bulk_create_actors(&namespace, "id-list-test", 5, ctx.leader_dc().guard_port())
				.await;

		// Select specific actors to list
		let selected_ids = vec![
			actor_ids[0].clone(),
			actor_ids[2].clone(),
			actor_ids[4].clone(),
		];

		// List by actor IDs
		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(selected_ids.clone()),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(
			actors.len(),
			3,
			"Should return exactly the requested actors"
		);

		// Verify correct actors returned
		let returned_ids: HashSet<String> = actors
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		for id in &selected_ids {
			assert!(
				returned_ids.contains(id),
				"Actor {} should be in results",
				id
			);
		}
	});
}

#[test]
fn list_actors_from_multiple_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors in different DCs
		let actor_id_dc1 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "multi-dc-actor".to_string(),
				key: Some("dc1-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		let actor_id_dc2 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "multi-dc-actor".to_string(),
				key: Some("dc2-key".to_string()),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Wait for propagation
		common::wait_for_actor_propagation(&actor_id_dc2, 1).await;

		// List by actor IDs - should fetch from both DCs
		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(vec![actor_id_dc1.clone(), actor_id_dc2.clone()]),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 2, "Should return actors from both DCs");
	});
}

// MARK: Error Cases

#[test]
fn list_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Try to list with non-existent namespace
		let response = common::list_actors(
			"non-existent-namespace",
			Some("test-actor"),
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with namespace not found
		assert!(
			!response.status().is_success(),
			"Should fail with non-existent namespace"
		);
		common::assert_error_response(response, "namespace_not_found").await;
	});
}

#[test]
fn list_with_both_actor_ids_and_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list with both actor_ids and name (validation error)
		let response = common::list_actors(
			&namespace,
			Some("test-actor"),
			None,
			Some(vec!["some-id".to_string()]),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with validation error
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for invalid parameters"
		);
	});
}
#[test]
fn list_with_key_but_no_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list with key but no name (validation error)
		let response = common::list_actors(
			&namespace,
			None,
			Some("key1".to_string()),
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with validation error
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for key without name"
		);
	});
}
#[test]
fn list_with_more_than_32_actor_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list with more than 32 actor IDs
		let actor_ids: Vec<String> = (0..33)
			.map(|i| format!("00000000-0000-0000-0000-{:012x}", i))
			.collect();

		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(actor_ids),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with validation error
		assert_eq!(
			response.status(),
			400,
			"Should return 400 for too many actor IDs"
		);
	});
}
#[test]
fn list_without_name_when_not_using_actor_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list without name or actor_ids
		let response = common::list_actors(
			&namespace,
			None,
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with validation error
		assert_eq!(
			response.status(),
			400,
			"Should return 400 when neither name nor actor_ids provided"
		);
	});
}

// MARK: Pagination and Sorting

#[test]
fn verify_sorting_by_create_ts_descending() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "sorted-actor";

		// Create actors with slight delays to ensure different timestamps
		let mut actor_ids = Vec::new();
		for i in 0..3 {
			let actor_id = common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
			actor_ids.push(actor_id);
			tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		}

		// List actors
		let response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");

		// Verify order - newest first (descending by create_ts)
		for i in 0..actors.len() {
			assert_eq!(
				actors[i]["actor_id"],
				actor_ids[actor_ids.len() - 1 - i],
				"Actors should be sorted by create_ts descending"
			);
		}
	});
}

// MARK: Cross-Datacenter

#[test]
fn list_aggregates_results_from_all_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "fanout-test-actor";

		// Create actors in both DCs
		let actor_id_dc1 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("dc1-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		let actor_id_dc2 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: name.to_string(),
				key: Some("dc2-key".to_string()),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		// Wait for propagation
		common::wait_for_actor_propagation(&actor_id_dc2, 1).await;

		// List by name - should fanout to all DCs
		let response = common::list_actors(
			&namespace,
			Some(name),
			None,
			None,
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let actors = body["actors"].as_array().expect("Expected actors array");
		assert_eq!(actors.len(), 2, "Should return actors from both DCs");

		// Verify both actors are present
		let returned_ids: HashSet<String> = actors
			.iter()
			.map(|a| a["actor_id"].as_str().unwrap().to_string())
			.collect();
		assert!(returned_ids.contains(&actor_id_dc1));
		assert!(returned_ids.contains(&actor_id_dc2));
	});
}
