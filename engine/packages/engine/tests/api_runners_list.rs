mod common;

use std::collections::HashSet;

// MARK: Basic

#[test]
fn list_runners_in_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List runners
		let response = common::api::public::runners_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				runner_ids: None,
				runner_id: vec![],
				include_stopped: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runners");

		// Should return at least the test runner
		assert!(
			!response.runners.is_empty(),
			"Should have at least one runner"
		);
	});
}

// MARK: Pagination tests

#[test]
fn list_runners_with_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// First page - limit 1
		let response1 = common::api::public::runners_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				runner_ids: None,
				runner_id: vec![],
				include_stopped: None,
				limit: Some(1),
				cursor: None,
			},
		)
		.await
		.expect("failed to list runners page 1");

		// With only 1 runner, there shouldn't be more pages
		if response1.runners.len() == 1 && response1.pagination.cursor.is_some() {
			// If there's a cursor, fetch next page
			let response2 = common::api::public::runners_list(
				ctx.leader_dc().guard_port(),
				rivet_api_types::runners::list::ListQuery {
					namespace: namespace.clone(),
					name: None,
					runner_ids: None,
					runner_id: vec![],
					include_stopped: None,
					limit: Some(1),
					cursor: response1.pagination.cursor.clone(),
				},
			)
			.await
			.expect("failed to list runners page 2");

			// Verify no duplicates between pages
			let ids1: HashSet<_> = response1
				.runners
				.iter()
				.map(|r| r.runner_id.to_string())
				.collect();
			let ids2: HashSet<_> = response2
				.runners
				.iter()
				.map(|r| r.runner_id.to_string())
				.collect();
			assert!(
				ids1.is_disjoint(&ids2),
				"Pages should not have duplicate runner IDs"
			);
		}
	});
}

#[test]
fn list_runners_default_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List without specifying limit - should use default limit of 100
		let response = common::api::public::runners_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				runner_ids: None,
				runner_id: vec![],
				include_stopped: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runners");

		// Should not exceed default limit
		assert!(
			response.runners.len() <= 100,
			"Should not exceed default limit of 100"
		);
	});
}

#[test]
fn list_runners_empty_response_no_cursor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace without runners
		let namespace_name = format!("no-runners-ns-{}", common::generate_unique_key());
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.clone(),
				display_name: "No Runners NS".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List runners in empty namespace
		let response = common::api::public::runners_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runners::list::ListQuery {
				namespace: namespace_name.clone(),
				name: None,
				runner_ids: None,
				runner_id: vec![],
				include_stopped: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runners");

		// Empty response should have no cursor
		assert_eq!(response.runners.len(), 0, "Should return empty list");
		assert!(
			response.pagination.cursor.is_none(),
			"Empty response should not have a cursor"
		);
	});
}
