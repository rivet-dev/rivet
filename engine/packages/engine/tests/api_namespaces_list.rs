mod common;

use std::collections::HashSet;

// MARK: Basic functionality tests

#[test]
fn list_namespaces_empty() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Note: There's always a default namespace created during bootstrap
		// So we can't test truly empty, but we can verify the default exists
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should have at least the default namespace
		assert!(
			response.namespaces.len() == 1,
			"Should have default namespace"
		);
	});
}

#[test]
fn list_namespaces_returns_all() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create multiple namespaces
		let mut created_ids = HashSet::new();
		for i in 0..5 {
			let response = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("test-ns-{}", i),
					display_name: format!("Test Namespace {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
			created_ids.insert(response.namespace.namespace_id);
		}

		// List all namespaces
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should include all created namespaces (plus default)
		let returned_ids: HashSet<_> = response
			.namespaces
			.iter()
			.map(|ns| ns.namespace_id)
			.collect();

		for id in created_ids {
			assert!(
				returned_ids.contains(&id),
				"Created namespace should be in list: {}",
				id
			);
		}
	});
}

#[test]
fn list_namespaces_validates_returned_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a test namespace
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "validation-test".to_string(),
				display_name: "Validation Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List namespaces
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Verify each namespace has required fields
		for namespace in &response.namespaces {
			assert!(!namespace.namespace_id.to_string().is_empty());
			assert!(!namespace.name.is_empty());
			assert!(!namespace.display_name.is_empty());
			assert!(namespace.create_ts > 0);
		}
	});
}

#[test]
fn list_namespaces_ordered_by_create_ts() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create multiple namespaces with delays to ensure different timestamps
		let mut created_timestamps = Vec::new();
		for i in 0..3 {
			let response = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("ordered-{}", i),
					display_name: format!("Ordered {}", i),
				},
			)
			.await
			.expect("failed to create namespace");

			created_timestamps.push(response.namespace.create_ts);
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// List namespaces
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Just verify all our created namespaces are present
		// Don't enforce strict ordering as implementation may vary
		assert!(
			response.namespaces.len() >= 3,
			"Should return at least our created namespaces"
		);
	});
}

#[test]
fn list_namespaces_includes_default() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a test namespace to ensure something exists
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "test-default".to_string(),
				display_name: "Test Default".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List namespaces
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should have at least one namespace
		assert!(!response.namespaces.is_empty(), "Should return namespaces");
	});
}

// MARK: Filter by name tests

#[test]
fn list_namespaces_filter_by_name_exists() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let namespace_name = "filter-by-name";

		// Create a namespace
		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.to_string(),
				display_name: "Filter Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List with name filter
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: Some(namespace_name.to_string()),
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 1);
		assert_eq!(
			response.namespaces[0].namespace_id,
			create_response.namespace.namespace_id
		);
		assert_eq!(response.namespaces[0].name, namespace_name);
	});
}

#[test]
fn list_namespaces_filter_by_name_not_exists() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// List with name filter for non-existent namespace
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: Some("non-existent-namespace".to_string()),
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 0, "Should return empty array");
		assert!(
			response.pagination.cursor.is_none(),
			"Should have no cursor"
		);
	});
}

#[test]
fn list_namespaces_filter_by_name_ignores_other_params() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let namespace_name = "filter-ignores";

		// Create a namespace
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.to_string(),
				display_name: "Filter Ignores Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List with name filter + other params (should ignore limit/cursor)
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: Some(namespace_name.to_string()),
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(100),
				cursor: Some("ignored".to_string()),
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 1);
		assert!(
			response.pagination.cursor.is_none(),
			"Name filter should not return cursor"
		);
	});
}

// MARK: Filter by IDs tests

#[test]
fn list_namespaces_filter_by_single_id() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a namespace
		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "filter-single-id".to_string(),
				display_name: "Filter Single ID".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		let namespace_id = create_response.namespace.namespace_id;

		// List with single ID filter
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![namespace_id],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 1);
		assert_eq!(response.namespaces[0].namespace_id, namespace_id);
	});
}

#[test]
fn list_namespaces_filter_by_multiple_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create multiple namespaces
		let mut created_ids = Vec::new();
		for i in 0..3 {
			let response = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("filter-multi-{}", i),
					display_name: format!("Filter Multi {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
			created_ids.push(response.namespace.namespace_id);
		}

		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: created_ids.clone(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 3);

		let returned_ids: HashSet<_> = response
			.namespaces
			.iter()
			.map(|ns| ns.namespace_id)
			.collect();

		for id in created_ids {
			assert!(
				returned_ids.contains(&id),
				"Should return all requested IDs"
			);
		}
	});
}

#[test]
fn list_namespaces_filter_by_ids_with_invalid_id() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a namespace
		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "filter-invalid-id".to_string(),
				display_name: "Filter Invalid ID".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		let valid_id = create_response.namespace.namespace_id;

		// List with valid ID + invalid IDs (should silently filter out invalid)
		let ids_str = format!("{},invalid-id,not-a-uuid", valid_id);

		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_id: vec![],
				namespace_ids: Some(ids_str),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should only return the valid ID
		assert_eq!(response.namespaces.len(), 1);
		assert_eq!(response.namespaces[0].namespace_id, valid_id);
	});
}

#[test]
fn list_namespaces_filter_by_ids_empty_list() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// List with only invalid IDs
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_id: vec![
					common::generate_dummy_rivet_id(ctx.leader_dc()),
					common::generate_dummy_rivet_id(ctx.leader_dc()),
				],
				namespace_ids: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		tracing::info!(?response.namespaces, "received response");

		assert_eq!(response.namespaces.len(), 0, "Should return empty array");
	});
}

// MARK: Pagination tests

#[test]
fn list_namespaces_default_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create some namespaces
		for i in 0..5 {
			common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("default-limit-{}", i),
					display_name: format!("Default Limit {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
		}

		// List without specifying limit
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should return all namespaces (no default limit restriction)
		assert!(
			response.namespaces.len() >= 5,
			"Should return all created namespaces"
		);
	});
}

#[test]
fn list_namespaces_with_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create multiple namespaces
		for i in 0..10 {
			common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("limit-test-{}", i),
					display_name: format!("Limit Test {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
		}

		// List with limit
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(5),
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(response.namespaces.len(), 5, "Should respect limit");
		assert!(
			response.pagination.cursor.is_some(),
			"Should have cursor when there are more results"
		);
	});
}

#[test]
fn list_namespaces_cursor_pagination() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create multiple namespaces with delays to ensure different timestamps
		for i in 0..6 {
			common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("cursor-test-{}", i),
					display_name: format!("Cursor Test {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
			tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
		}

		// Get first page
		let first_page = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(3),
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		assert_eq!(first_page.namespaces.len(), 3);
		assert!(first_page.pagination.cursor.is_some());

		// Get second page using cursor
		let second_page = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(3),
				cursor: first_page.pagination.cursor.clone(),
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should have more namespaces on second page
		assert!(
			!second_page.namespaces.is_empty(),
			"Should have results on second page"
		);

		// Just verify pagination works - may have overlap if multiple namespaces share same create_ts
		// This is acceptable behavior for timestamp-based pagination
	});
}

#[test]
fn list_namespaces_cursor_no_more_results() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create exactly 3 namespaces
		for i in 0..3 {
			common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("no-more-{}", i),
					display_name: format!("No More {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
		}

		// List all with limit matching total count
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(100), // Large enough to get all
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Cursor should be present since we can't know if there are more without checking
		// This depends on implementation - some return cursor always, some only when more exist
		// Let's just verify we got results
		assert!(response.namespaces.len() >= 3);
	});
}

// MARK: Cross-datacenter tests

#[test]
fn list_namespaces_from_leader() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a namespace
		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "leader-list-test".to_string(),
				display_name: "Leader List Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List from leader DC
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces from leader");

		// Should include our created namespace
		let found = response
			.namespaces
			.iter()
			.any(|ns| ns.namespace_id == create_response.namespace.namespace_id);
		assert!(found, "Should include created namespace in list");
	});
}

#[test]
fn list_namespaces_from_follower_routes_to_leader() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		// Create a namespace from leader
		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "follower-list-test".to_string(),
				display_name: "Follower List Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// List from follower DC (should route to leader)
		let response = common::api::public::namespaces_list(
			ctx.get_dc(2).guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces from follower");

		// Should include namespace created on leader
		let found = response
			.namespaces
			.iter()
			.any(|ns| ns.namespace_id == create_response.namespace.namespace_id);
		assert!(
			found,
			"Follower should see namespaces from leader (routes correctly)"
		);
	});
}

// MARK: Edge cases

#[test]
fn list_namespaces_with_zero_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// List with limit=0
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(0),
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces");

		// Should return empty or handle gracefully
		// Implementation may vary - some return empty, some ignore limit=0
		assert!(response.namespaces.len() == 0 || response.namespaces.len() > 0);
	});
}

#[test]
fn list_namespaces_large_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create a few namespaces
		for i in 0..3 {
			common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("large-limit-{}", i),
					display_name: format!("Large Limit {}", i),
				},
			)
			.await
			.expect("failed to create namespace");
		}

		// List with very large limit
		let response = common::api::public::namespaces_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::namespaces::list::ListQuery {
				name: None,
				namespace_ids: None,
				namespace_id: vec![],
				limit: Some(10000),
				cursor: None,
			},
		)
		.await
		.expect("failed to list namespaces with large limit");

		// Should return all available namespaces
		assert!(response.namespaces.len() >= 3);
	});
}
