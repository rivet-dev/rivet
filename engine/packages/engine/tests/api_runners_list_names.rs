mod common;

use std::collections::HashSet;

// MARK: Basic

#[test]
fn list_all_runner_names_in_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List runner names
		let response = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names");

		// Should return at least the test runner name
		assert!(
			!response.names.is_empty(),
			"Should have at least one runner name"
		);
		assert!(
			response
				.names
				.contains(&common::TEST_RUNNER_NAME.to_string()),
			"Should contain test runner name"
		);
	});
}

// MARK: Pagination tests

#[test]
fn list_runner_names_with_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create additional runner configs to have more names to paginate
		for i in 0..5 {
			let mut datacenters = std::collections::HashMap::new();
			datacenters.insert(
				"dc-1".to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
					metadata: None,
					drain_on_version_upgrade: true,
				},
			);

			common::api::public::runner_configs_upsert(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::runner_configs::UpsertPath {
					runner_name: format!("pagination-runner-{:02}", i),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert runner config");
		}

		// First page - limit 3
		let response1 = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: Some(3),
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names page 1");

		assert_eq!(
			response1.names.len(),
			3,
			"Should return 3 names with limit=3"
		);

		let cursor = response1
			.pagination
			.cursor
			.as_ref()
			.expect("Should have cursor for pagination");

		// Second page - use cursor
		let response2 = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: Some(3),
				cursor: Some(cursor.clone()),
			},
		)
		.await
		.expect("failed to list runner names page 2");

		// Verify no duplicates between pages
		let set1: HashSet<String> = response1.names.iter().cloned().collect();
		let set2: HashSet<String> = response2.names.iter().cloned().collect();
		assert!(
			set1.is_disjoint(&set2),
			"Pages should not have duplicate names. Page 1: {:?}, Page 2: {:?}",
			set1,
			set2
		);
	});
}

#[test]
fn list_runner_names_pagination_no_duplicates_comprehensive() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create runner configs with sequential names
		for i in 0..9 {
			let mut datacenters = std::collections::HashMap::new();
			datacenters.insert(
				"dc-1".to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
					metadata: None,
					drain_on_version_upgrade: true,
				},
			);

			common::api::public::runner_configs_upsert(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::runner_configs::UpsertPath {
					runner_name: format!("runner-{:02}", i),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert runner config");
		}

		// Paginate through all results with small page size
		let mut all_names: HashSet<String> = HashSet::new();
		let mut cursor: Option<String> = None;
		let mut page_count = 0;

		loop {
			let response = common::api::public::runners_list_names(
				ctx.leader_dc().guard_port(),
				rivet_api_types::runners::list_names::ListNamesQuery {
					namespace: namespace.clone(),
					limit: Some(3),
					cursor: cursor.clone(),
				},
			)
			.await
			.expect("failed to list runner names");

			page_count += 1;

			// Check for duplicates
			for name in &response.names {
				assert!(
					!all_names.contains(name),
					"Duplicate name found: {} on page {}. All names so far: {:?}",
					name,
					page_count,
					all_names
				);
				all_names.insert(name.clone());
			}

			// Move to next page or break
			if response.pagination.cursor.is_none() || response.names.is_empty() {
				break;
			}
			cursor = response.pagination.cursor;

			// Safety limit to prevent infinite loops
			if page_count > 20 {
				panic!("Too many pages, possible infinite loop");
			}
		}

		// Should have found all runner names (9 created + 1 test runner)
		assert!(
			all_names.len() >= 9,
			"Should find at least 9 runner names, found: {}",
			all_names.len()
		);
	});
}

#[test]
fn list_runner_names_returns_empty_for_empty_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace without runners
		let namespace_name = format!("empty-runners-ns-{}", common::generate_unique_key());
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.clone(),
				display_name: "Empty Runners NS".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List names in empty namespace
		let response = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace_name.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names");

		assert_eq!(
			response.names.len(),
			0,
			"Should return empty list for empty namespace"
		);
	});
}

// MARK: Error cases

#[test]
fn list_runner_names_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Try to list names with non-existent namespace
		let res = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
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

// MARK: Edge cases

#[test]
fn list_runner_names_default_limit_100() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List without specifying limit - should use default limit of 100
		let response = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names");

		// Should not exceed default limit
		assert!(
			response.names.len() <= 100,
			"Should not exceed default limit of 100"
		);
	});
}

#[test]
fn list_runner_names_empty_response_no_cursor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace without runners
		let namespace_name = format!("no-cursor-ns-{}", common::generate_unique_key());
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.clone(),
				display_name: "No Cursor NS".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List names in empty namespace
		let response = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace_name.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names");

		// Empty response should have no cursor
		assert_eq!(response.names.len(), 0, "Should return empty list");
		assert!(
			response.pagination.cursor.is_none(),
			"Empty response should not have a cursor"
		);
	});
}

#[test]
fn list_runner_names_alphabetical_sorting() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create runners with names that need sorting
		let unsorted_names = vec!["zebra-runner", "alpha-runner", "beta-runner"];
		for name in &unsorted_names {
			let mut datacenters = std::collections::HashMap::new();
			datacenters.insert(
				"dc-1".to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
					metadata: None,
					drain_on_version_upgrade: true,
				},
			);

			common::api::public::runner_configs_upsert(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::runner_configs::UpsertPath {
					runner_name: name.to_string(),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert runner config");
		}

		// List names
		let response = common::api::public::runners_list_names(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list_names::ListNamesQuery {
				namespace: namespace.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner names");

		// Filter to just our test names and verify they're sorted
		let test_names: Vec<_> = response
			.names
			.iter()
			.filter(|n| unsorted_names.contains(&n.as_str()))
			.cloned()
			.collect();

		let mut sorted_test_names = test_names.clone();
		sorted_test_names.sort();

		assert_eq!(
			test_names, sorted_test_names,
			"Names should be in alphabetical order"
		);
	});
}
