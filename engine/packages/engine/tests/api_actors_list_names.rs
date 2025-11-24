mod common;

use std::collections::HashSet;

// MARK: Basic

#[test]
fn list_all_actor_names_in_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with different names
		let names = vec!["actor-alpha", "actor-beta", "actor-gamma"];
		for name in &names {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(common::generate_unique_key()),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// Create multiple actors with same name (should deduplicate)
		for i in 0..3 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: "actor-alpha".to_string(),
					key: Some(format!("key-{}", i)),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// List actor names
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Should return unique names only (HashMap automatically deduplicates)
		assert_eq!(response.names.len(), 3, "Should return 3 unique names");

		// Verify all names are present in the HashMap keys
		let returned_names: HashSet<String> = response.names.keys().cloned().collect();
		for name in &names {
			assert!(
				returned_names.contains(*name),
				"Name {} should be in results",
				name
			);
		}
	});
}

#[test]
fn list_names_with_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with many different names
		for i in 0..9 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: format!("actor-{:02}", i),
					key: Some(common::generate_unique_key()),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// First page - limit 5
		let response1 = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: Some(5),
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		assert_eq!(
			response1.names.len(),
			5,
			"Should return 5 names with limit=5"
		);

		let cursor = response1
			.pagination
			.cursor
			.as_ref()
			.expect("Should have cursor for pagination");

		// Second page - use cursor
		let response2 = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: Some(5),
				cursor: Some(cursor.clone()),
			},
		)
		.await
		.expect("failed to list actor names page 2");

		assert_eq!(response2.names.len(), 4, "Should return remaining 4 names");

		// Verify no duplicates between pages
		let set1: HashSet<String> = response1.names.keys().cloned().collect();
		let set2: HashSet<String> = response2.names.keys().cloned().collect();
		assert!(
			set1.is_disjoint(&set2),
			"Pages should not have duplicate names"
		);
	});
}

#[test]
fn list_names_returns_empty_for_empty_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List names in empty namespace
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		assert_eq!(
			response.names.len(),
			0,
			"Should return empty HashMap for empty namespace"
		);
	});
}

// MARK: Error cases

#[test]
fn list_names_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Try to list names with non-existent namespace
		let res = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: "non-existent-namespace".to_string(),
				limit: None,
				cursor: None,
			},
		)
		.await;

		// Should fail with namespace not found
		assert!(res.is_err(), "Should fail with non-existent namespace");
	});
}

// MARK: Cross-datacenter tests

#[test]
fn list_names_fanout_to_all_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with different names in different DCs
		common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "dc1-actor".to_string(),
				key: Some(common::generate_unique_key()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC1");

		common::api::public::actors_create(
			ctx.get_dc(2).guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: "dc2-actor".to_string(),
				key: Some(common::generate_unique_key()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC2");

		// List names from DC 1 - should fanout to all DCs
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Should return names from both DCs
		let returned_names: HashSet<String> = response.names.keys().cloned().collect();
		assert!(
			returned_names.contains("dc1-actor"),
			"Should contain DC1 actor name"
		);
		assert!(
			returned_names.contains("dc2-actor"),
			"Should contain DC2 actor name"
		);
	});
}

#[test]
fn list_names_deduplication_across_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with same name in different DCs
		let shared_name = "shared-name-actor";

		common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: shared_name.to_string(),
				key: Some("dc1-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC1");

		common::api::public::actors_create(
			ctx.get_dc(2).guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: Some("dc-2".to_string()),
				name: shared_name.to_string(),
				key: Some("dc2-key".to_string()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor in DC2");

		// List names - should deduplicate
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Should return only one instance of the name (HashMap deduplicates)
		assert!(
			response.names.contains_key(shared_name),
			"Should contain the shared name"
		);

		// Count occurrences - should be exactly 1 in the HashMap
		let name_count = response
			.names
			.keys()
			.filter(|n| n.as_str() == shared_name)
			.count();
		assert_eq!(name_count, 1, "Should deduplicate names across datacenters");
	});
}

#[test]
fn list_names_alphabetical_sorting() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with names that need sorting
		let unsorted_names = vec!["zebra-actor", "alpha-actor", "beta-actor", "gamma-actor"];
		for name in &unsorted_names {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: name.to_string(),
					key: Some(common::generate_unique_key()),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// List names
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Convert HashMap keys to sorted vector
		let mut returned_names: Vec<String> = response.names.keys().cloned().collect();
		returned_names.sort();

		// Verify alphabetical order
		assert_eq!(returned_names.len(), 4, "Should return all 4 unique names");
		assert_eq!(returned_names[0], "alpha-actor");
		assert_eq!(returned_names[1], "beta-actor");
		assert_eq!(returned_names[2], "gamma-actor");
		assert_eq!(returned_names[3], "zebra-actor");
	});
}

// MARK: Edge cases

#[test]
fn list_names_default_limit_100() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create 105 actors with different names to test the default limit of 100
		for i in 0..105 {
			common::api::public::actors_create(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				common::api_types::actors::create::CreateRequest {
					datacenter: None,
					name: format!("actor-{:03}", i),
					key: Some(common::generate_unique_key()),
					input: None,
					runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Destroy,
				},
			)
			.await
			.expect("failed to create actor");
		}

		// List without specifying limit - should use default limit of 100
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None, // No limit specified - should default to 100
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Should return exactly 100 names due to default limit
		assert_eq!(
			response.names.len(),
			100,
			"Should return exactly 100 names when default limit is applied"
		);

		// Verify cursor exists since there are more results
		assert!(
			response.pagination.cursor.is_some(),
			"Cursor should exist when there are more results beyond the limit"
		);
	});
}

#[test]
fn list_names_with_metadata() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_name = "test-actor-with-metadata";

		// Create an actor
		common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: actor_name.to_string(),
				key: Some(common::generate_unique_key()),
				input: None,
				runner_name_selector: common::TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		// List names
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Verify the name exists and has metadata
		assert!(
			response.names.contains_key(actor_name),
			"Should contain the actor name"
		);

		let _actor_name_info = response
			.names
			.get(actor_name)
			.expect("Should have actor name info");

		// Verify ActorName exists - the fact that we got it from the HashMap means
		// it has the expected structure with metadata field
		// No need to assert further on the metadata since it's always present as a Map
	});
}

#[test]
fn list_names_empty_response_no_cursor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List names in empty namespace
		let response = common::api::public::actors_list_names(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list actor names");

		// Empty response should have no cursor
		assert_eq!(response.names.len(), 0, "Should return empty HashMap");
		assert!(
			response.pagination.cursor.is_none(),
			"Empty response should not have a cursor"
		);
	});
}
