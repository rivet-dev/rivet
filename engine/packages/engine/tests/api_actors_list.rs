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
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
			actor_ids.push(res.actor.actor_id.to_string());
		}

		// List actors by name
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(response.actors.len(), 3, "Should return all 3 actors");

		// Verify all created actors are in the response
		let returned_ids: HashSet<String> = response
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
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
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
			actor_ids.push(res.actor.actor_id.to_string());
		}

		// First page - limit 2
		let response1 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(2),
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(
			response1.actors.len(),
			2,
			"Should return 2 actors with limit=2"
		);

		// Get all actors to verify ordering
		let all_response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list all actors");

		// Verify we have all 5 actors when querying without limit
		assert_eq!(
			all_response.actors.len(),
			5,
			"Should return all 5 actors when no limit specified"
		);

		// Use actors from position 2-4 as actors2 for remaining test logic
		let actors2 = if all_response.actors.len() > 2 {
			&all_response.actors[2..std::cmp::min(4, all_response.actors.len())]
		} else {
			&[]
		};

		// Verify no duplicates between pages
		let ids1: HashSet<String> = response1
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		let ids2: HashSet<String> = actors2.iter().map(|a| a.actor_id.to_string()).collect();
		assert!(
			ids1.is_disjoint(&ids2),
			"Pages should not have duplicate actors"
		);

		// Verify consistent ordering using the full actor list
		let all_timestamps: Vec<i64> = all_response.actors.iter().map(|a| a.create_ts).collect();

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
		let paginated_timestamps: Vec<i64> = response1.actors.iter().map(|a| a.create_ts).collect();

		assert_eq!(
			paginated_timestamps,
			all_timestamps[0..2].to_vec(),
			"Paginated result should return the 2 newest actors"
		);

		// Test that limit=2 actually limits results to 2
		assert_eq!(
			response1.actors.len(),
			2,
			"Limit=2 should return exactly 2 actors"
		);
		assert_eq!(
			all_response.actors.len(),
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
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some("non-existent-actor".to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(response.actors.len(), 0, "Should return empty array");
	});
}

// MARK: List by Name + Key

#[test]
fn list_actors_by_namespace_name_and_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "keyed-actor";
		let key1 = "key1".to_string();
		let key2 = "key2".to_string();

		// Create actors with different keys
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some(key1.clone()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor1");
		let actor_id1 = res1.actor.actor_id.to_string();

		let _res2 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some(key2.clone()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor2");

		// List with key1 - should find actor1
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: Some("key1".to_string()),
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(response.actors.len(), 1, "Should return 1 actor");
		assert_eq!(response.actors[0].actor_id.to_string(), actor_id1);
	});
}

#[test]
fn list_with_include_destroyed_false() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "destroyed-test";

		// Create and destroy an actor
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some("destroyed-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let destroyed_actor_id = res1.actor.actor_id;

		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: destroyed_actor_id,
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Create an active actor
		let res2 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some("active-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let active_actor_id = res2.actor.actor_id.to_string();

		// List without include_destroyed (default false)
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: Some(false),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(response.actors.len(), 1, "Should only return active actor");
		assert_eq!(response.actors[0].actor_id.to_string(), active_actor_id);
	});
}

#[test]
fn list_with_include_destroyed_true() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "destroyed-included";

		// Create and destroy an actor
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some("destroyed-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let destroyed_actor_id = res1.actor.actor_id.to_string();

		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: res1.actor.actor_id,
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Create an active actor
		let res2 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some("active-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let active_actor_id = res2.actor.actor_id.to_string();

		// List with include_destroyed=true
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: Some(true),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(
			response.actors.len(),
			2,
			"Should return both active and destroyed actors"
		);

		// Verify both actors are in results
		let returned_ids: HashSet<String> = response
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
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
			common::bulk_create_actors(ctx.leader_dc().guard_port(), &namespace, "id-list-test", 5)
				.await;

		// Select specific actors to list
		let selected_ids = vec![
			actor_ids[0].clone(),
			actor_ids[2].clone(),
			actor_ids[4].clone(),
		];

		// List by actor IDs (comma-separated string)
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: selected_ids.clone(),
				actor_ids: None,
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(
			response.actors.len(),
			3,
			"Should return exactly the requested actors"
		);

		// Verify correct actors returned
		let returned_ids: HashSet<String> = response
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		for id in &selected_ids {
			assert!(
				returned_ids.contains(&id.to_string()),
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
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "multi-dc-actor".to_string(),
				key: Some("dc1-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC1");
		let actor_id_dc1 = res1.actor.actor_id;

		let res2 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: "multi-dc-actor".to_string(),
				key: Some("dc2-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC2");
		let actor_id_dc2 = res2.actor.actor_id;

		// List by actor IDs - should fetch from both DCs
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: vec![actor_id_dc1, actor_id_dc2],
				actor_ids: None,
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(
			response.actors.len(),
			2,
			"Should return actors from both DCs"
		);
	});
}

// MARK: Error cases

#[test]
fn list_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Try to list with non-existent namespace
		let res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: "non-existent-namespace".to_string(),
				name: Some("test-actor".to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await;

		// Should fail with namespace not found
		assert!(res.is_err(), "Should fail with non-existent namespace");
	});
}

#[test]
fn list_with_key_but_no_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list with key but no name (validation error)
		let res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: Some("key1".to_string()),
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await;

		// Should fail with validation error
		assert!(res.is_err(), "Should return error for key without name");
	});
}

#[test]
fn list_with_more_than_32_actor_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list with more than 32 actor IDs
		let actor_ids: Vec<rivet_util::Id> = (0..33)
			.map(|_| rivet_util::Id::new_v1(ctx.leader_dc().config.dc_label()))
			.collect();

		let res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: actor_ids,
				actor_ids: None,
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await;

		// Should fail with validation error
		assert!(res.is_err(), "Should return error for too many actor IDs");
	});
}

#[test]
fn list_without_name_when_not_using_actor_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try to list without name or actor_ids
		let res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await;

		// Should fail with validation error
		assert!(
			res.is_err(),
			"Should return error when neither name nor actor_ids provided"
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
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
			actor_ids.push(res.actor.actor_id.to_string());
		}

		// List actors
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		// Verify order - newest first (descending by create_ts)
		for i in 0..response.actors.len() {
			assert_eq!(
				response.actors[i].actor_id.to_string(),
				actor_ids[actor_ids.len() - 1 - i],
				"Actors should be sorted by create_ts descending"
			);
		}
	});
}

// MARK: Cross-datacenter

#[test]
fn list_aggregates_results_from_all_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "fanout-test-actor";

		// Create actors in both DCs
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some("dc1-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC1");
		let actor_id_dc1 = res1.actor.actor_id.to_string();

		let res2 = common::api::public::actors_create(
			ctx.get_dc(2).guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: name.to_string(),
				key: Some("dc2-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC2");
		let actor_id_dc2 = res2.actor.actor_id.to_string();

		// List by name - should fanout to all DCs
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		assert_eq!(
			response.actors.len(),
			2,
			"Should return actors from both DCs"
		);

		// Verify both actors are present
		let returned_ids: HashSet<String> = response
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		assert!(returned_ids.contains(&actor_id_dc1));
		assert!(returned_ids.contains(&actor_id_dc2));
	});
}

// MARK: Edge cases

#[test]
fn list_with_exactly_32_actor_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create exactly 32 actor IDs (boundary condition)
		let actor_ids: Vec<rivet_util::Id> = (0..32)
			.map(|_| rivet_util::Id::new_v1(ctx.leader_dc().config.dc_label()))
			.collect();

		// Should succeed with exactly 32 IDs
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: actor_ids,
				actor_ids: None,
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("should succeed with exactly 32 actor IDs");

		// Since these are fake IDs, we expect 0 results, but no error
		assert_eq!(
			response.actors.len(),
			0,
			"Fake IDs should return empty results"
		);
	});
}

#[test]
fn list_by_key_with_include_destroyed_true() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "key-destroyed-test";
		let key = "test-key";

		// Create and destroy an actor with a key
		let res1 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some(key.to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let destroyed_actor_id = res1.actor.actor_id.to_string();

		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: res1.actor.actor_id,
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
			},
		)
		.await
		.expect("failed to delete actor");

		// Create a new actor with the same key
		let res2 = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: name.to_string(),
				key: Some(key.to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let active_actor_id = res2.actor.actor_id.to_string();

		// List by key with include_destroyed=true
		// This should use the fanout path, not the optimized key path
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: Some(key.to_string()),
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: Some(true),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		// Should return both actors (destroyed and active)
		assert_eq!(
			response.actors.len(),
			2,
			"Should return both destroyed and active actors with same key"
		);

		let returned_ids: HashSet<String> = response
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		assert!(returned_ids.contains(&destroyed_actor_id));
		assert!(returned_ids.contains(&active_actor_id));
	});
}

#[test]
fn list_default_limit_100() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "limit-test";

		// Create 105 actors to test the default limit of 100
		let actor_ids =
			common::bulk_create_actors(ctx.leader_dc().guard_port(), &namespace, name, 105).await;

		assert_eq!(actor_ids.len(), 105, "Should have created 105 actors");

		// List without specifying limit - should use default limit of 100
		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None, // No limit specified - should default to 100
				cursor: None,
			},
		)
		.await
		.expect("failed to list actors");

		// Should return exactly 100 actors due to default limit
		assert_eq!(
			response.actors.len(),
			100,
			"Should return exactly 100 actors when default limit is applied"
		);

		// Verify cursor exists since there are more results
		assert!(
			response.pagination.cursor.is_some(),
			"Cursor should exist when there are more results beyond the limit"
		);
	});
}

#[test]
fn list_with_invalid_actor_id_format_in_comma_list() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create a valid actor
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: Some("test-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		let valid_actor_id = res.actor.actor_id.to_string();

		// Mix valid and invalid IDs in the comma-separated list
		let mixed_ids = vec![
			valid_actor_id.clone(),
			"invalid-uuid".to_string(),
			"not-a-uuid".to_string(),
			valid_actor_id.clone(),
		];

		let response = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: vec![],
				actor_ids: Some(mixed_ids.join(",")),
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("should filter out invalid IDs gracefully");

		// Should return only the valid actor (twice) (parsed IDs are filtered)
		assert_eq!(
			response.actors.len(),
			2,
			"Should filter out invalid IDs and return only valid ones"
		);
		assert_eq!(response.actors[0].actor_id.to_string(), valid_actor_id);
	});
}

// MARK: Cursor pagination

#[test]
fn list_with_cursor_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "cursor-test-actor";

		// Create 5 actors with same name
		let mut actor_ids = Vec::new();
		for i in 0..5 {
			let res = common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("cursor-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
			actor_ids.push(res.actor.actor_id.to_string());
		}

		// Fetch first page with limit=2
		let page1 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(2),
				cursor: None,
			},
		)
		.await
		.expect("failed to list page 1");

		assert_eq!(page1.actors.len(), 2, "Page 1 should have 2 actors");
		assert!(
			page1.pagination.cursor.is_some(),
			"Page 1 should return a cursor"
		);

		// Fetch second page using cursor
		let page2 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(2),
				cursor: page1.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list page 2");

		assert_eq!(page2.actors.len(), 2, "Page 2 should have 2 actors");
		assert!(
			page2.pagination.cursor.is_some(),
			"Page 2 should return a cursor"
		);

		// Fetch third page using cursor
		let page3 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(2),
				cursor: page2.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list page 3");

		assert_eq!(page3.actors.len(), 1, "Page 3 should have 1 actor");

		// Verify no duplicates across pages
		let ids1: HashSet<String> = page1
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		let ids2: HashSet<String> = page2
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		let ids3: HashSet<String> = page3
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();

		assert!(
			ids1.is_disjoint(&ids2),
			"Page 1 and 2 should have no duplicates"
		);
		assert!(
			ids1.is_disjoint(&ids3),
			"Page 1 and 3 should have no duplicates"
		);
		assert!(
			ids2.is_disjoint(&ids3),
			"Page 2 and 3 should have no duplicates"
		);

		// Verify all actors are returned across all pages
		let mut all_returned_ids = ids1;
		all_returned_ids.extend(ids2);
		all_returned_ids.extend(ids3);

		assert_eq!(
			all_returned_ids.len(),
			5,
			"All 5 actors should be returned across pages"
		);
		for actor_id in &actor_ids {
			assert!(
				all_returned_ids.contains(&actor_id.to_string()),
				"Actor {} should be in results",
				actor_id
			);
		}
	});
}

#[test]
fn list_cursor_filters_by_timestamp() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "timestamp-filter-test";

		// Create 3 actors
		for i in 0..3 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("ts-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// Get all actors to find a middle timestamp
		let all_actors = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list all actors");

		assert_eq!(all_actors.actors.len(), 3, "Should have 3 actors");

		// Use the first actor's timestamp as cursor (should filter out that actor and newer)
		let cursor = all_actors.actors[0].create_ts.to_string();

		// List with cursor
		let filtered = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: Some(cursor.clone()),
			},
		)
		.await
		.expect("failed to list with cursor");

		// Should return only actors older than the cursor timestamp
		assert!(
			filtered.actors.len() < 3,
			"Cursor should filter out some actors"
		);

		// Verify all returned actors have timestamps less than cursor
		let cursor_ts: i64 = cursor.parse().expect("cursor should be valid i64");
		for actor in &filtered.actors {
			assert!(
				actor.create_ts < cursor_ts,
				"Actor timestamp {} should be less than cursor {}",
				actor.create_ts,
				cursor_ts
			);
		}
	});
}

#[test]
fn list_cursor_with_exact_timestamp_boundary() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "boundary-test";

		// Create 3 actors
		for i in 0..3 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("boundary-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// Get first page with limit=1
		let page1 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(1),
				cursor: None,
			},
		)
		.await
		.expect("failed to list page 1");

		assert_eq!(page1.actors.len(), 1, "Page 1 should have 1 actor");
		let first_actor_id = page1.actors[0].actor_id.to_string();

		// Get second page using cursor
		let page2 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: page1.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list page 2");

		// Verify first actor is NOT in page 2 (exact boundary excluded)
		let page2_ids: HashSet<String> = page2
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		assert!(
			!page2_ids.contains(&first_actor_id),
			"Actor with exact cursor timestamp should be excluded"
		);
	});
}

#[test]
fn list_cursor_empty_results_when_no_more_actors() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "empty-cursor-test";

		// Create 2 actors
		for i in 0..2 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("empty-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// List all actors
		let all_actors = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(10),
				cursor: None,
			},
		)
		.await
		.expect("failed to list all actors");

		assert_eq!(all_actors.actors.len(), 2, "Should have 2 actors");

		// Use cursor to fetch next page (should be empty)
		if let Some(cursor) = all_actors.pagination.cursor {
			let next_page = common::api::public::actors_list(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::list::ListQuery {
					namespace: namespace.clone(),
					name: Some(name.to_string()),
					key: None,
					actor_ids: None,
					actor_id: vec![],
					include_destroyed: None,
					limit: Some(10),
					cursor: Some(cursor),
				},
			)
			.await
			.expect("failed to list next page");

			assert_eq!(
				next_page.actors.len(),
				0,
				"Should return empty results when no more actors"
			);
			assert!(
				next_page.pagination.cursor.is_none(),
				"Should not return cursor when no more results"
			);
		}
	});
}

#[test]
fn list_invalid_cursor_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "invalid-cursor-test";

		// Try to list with invalid cursor (non-numeric string)
		let res = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: None,
				cursor: Some("not-a-number".to_string()),
			},
		)
		.await;

		// Should fail with parse error
		assert!(
			res.is_err(),
			"Should return error for invalid cursor format"
		);
	});
}

#[test]
fn list_cursor_across_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "multi-dc-cursor-test";

		// Create actors in both DC1 and DC2
		for i in 0..3 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(format!("dc1-cursor-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor in DC1");
		}

		for i in 0..3 {
			common::api::public::actors_create(
				ctx.get_dc(2).guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: Some("dc-2".to_string()),
					name: name.to_string(),
					key: Some(format!("dc2-cursor-key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor in DC2");
		}

		// Fetch first page with limit=3
		let page1 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: vec![],
				include_destroyed: None,
				limit: Some(3),
				cursor: None,
			},
		)
		.await
		.expect("failed to list page 1");

		assert!(
			page1.actors.len() <= 3,
			"Page 1 should have at most 3 actors"
		);

		// Fetch second page using cursor
		if let Some(cursor) = page1.pagination.cursor {
			let page2 = common::api::public::actors_list(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::list::ListQuery {
					namespace: namespace.clone(),
					name: Some(name.to_string()),
					key: None,
					actor_ids: None,
					actor_id: vec![],
					include_destroyed: None,
					limit: Some(3),
					cursor: Some(cursor),
				},
			)
			.await
			.expect("failed to list page 2");

			// Verify no duplicates between pages
			let ids1: HashSet<String> = page1
				.actors
				.iter()
				.map(|a| a.actor_id.to_string())
				.collect();
			let ids2: HashSet<String> = page2
				.actors
				.iter()
				.map(|a| a.actor_id.to_string())
				.collect();

			assert!(
				ids1.is_disjoint(&ids2),
				"Pages should have no duplicate actors across DCs"
			);
		}
	});
}

#[test]
fn list_actor_ids_with_cursor_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let name = "actor-ids-cursor-test";

		// Create 5 actors
		let actor_ids =
			common::bulk_create_actors(ctx.leader_dc().guard_port(), &namespace, name, 5).await;

		// List by actor_ids with limit=2
		let page1 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: actor_ids.clone(),
				actor_ids: None,
				include_destroyed: None,
				limit: Some(2),
				cursor: None,
			},
		)
		.await
		.expect("failed to list page 1");

		assert_eq!(
			page1.actors.len(),
			2,
			"Page 1 should return exactly 2 actors"
		);
		assert!(
			page1.pagination.cursor.is_some(),
			"Page 1 should return a cursor"
		);

		// Fetch second page using cursor
		let page2 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: actor_ids.clone(),
				actor_ids: None,
				include_destroyed: None,
				limit: Some(2),
				cursor: page1.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list page 2");

		assert_eq!(
			page2.actors.len(),
			2,
			"Page 2 should return exactly 2 actors"
		);
		assert!(
			page2.pagination.cursor.is_some(),
			"Page 2 should return a cursor"
		);

		// Fetch third page using cursor
		let page3 = common::api::public::actors_list(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_id: actor_ids.clone(),
				actor_ids: None,
				include_destroyed: None,
				limit: Some(2),
				cursor: page2.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list page 3");

		assert_eq!(
			page3.actors.len(),
			1,
			"Page 3 should return 1 remaining actor"
		);

		// Verify no duplicates across pages
		let ids1: HashSet<String> = page1
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		let ids2: HashSet<String> = page2
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();
		let ids3: HashSet<String> = page3
			.actors
			.iter()
			.map(|a| a.actor_id.to_string())
			.collect();

		assert!(
			ids1.is_disjoint(&ids2),
			"Page 1 and 2 should have no duplicates"
		);
		assert!(
			ids1.is_disjoint(&ids3),
			"Page 1 and 3 should have no duplicates"
		);
		assert!(
			ids2.is_disjoint(&ids3),
			"Page 2 and 3 should have no duplicates"
		);

		// Verify all actors are returned across all pages
		let mut all_returned_ids = ids1;
		all_returned_ids.extend(ids2);
		all_returned_ids.extend(ids3);

		assert_eq!(
			all_returned_ids.len(),
			5,
			"All 5 actors should be returned across pages"
		);
		for actor_id in &actor_ids {
			assert!(
				all_returned_ids.contains(&actor_id.to_string()),
				"Actor {} should be in results",
				actor_id
			);
		}
	});
}
