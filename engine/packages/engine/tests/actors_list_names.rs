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
			common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: name.to_string(),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
		}

		// Create multiple actors with same name (should deduplicate)
		for i in 0..3 {
			common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: "actor-alpha".to_string(),
					key: Some(format!("key-{}", i)),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
		}

		// List actor names
		let response =
			common::list_actor_names(&namespace, None, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let returned_names = body["names"].as_array().expect("Expected names array");

		// Should return unique names only
		assert_eq!(returned_names.len(), 3, "Should return 3 unique names");

		// Verify all names are present
		let name_set: HashSet<String> = returned_names
			.iter()
			.map(|n| n.as_str().unwrap().to_string())
			.collect();
		for name in &names {
			assert!(
				name_set.contains(*name),
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
		for i in 0..10 {
			common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: format!("actor-{:02}", i),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
		}

		// First page - limit 5
		let response1 =
			common::list_actor_names(&namespace, Some(5), None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response1);

		let body1: serde_json::Value = response1.json().await.expect("Failed to parse response");
		let names1 = body1["names"].as_array().expect("Expected names array");
		assert_eq!(names1.len(), 5, "Should return 5 names with limit=5");

		let cursor = body1["cursor"]
			.as_str()
			.expect("Should have cursor for pagination");

		// Second page - use cursor
		let response2 = common::list_actor_names(
			&namespace,
			Some(5),
			Some(cursor),
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response2);

		let body2: serde_json::Value = response2.json().await.expect("Failed to parse response");
		let names2 = body2["names"].as_array().expect("Expected names array");
		assert_eq!(names2.len(), 5, "Should return remaining 5 names");

		// Verify no duplicates between pages
		let set1: HashSet<String> = names1
			.iter()
			.map(|n| n.as_str().unwrap().to_string())
			.collect();
		let set2: HashSet<String> = names2
			.iter()
			.map(|n| n.as_str().unwrap().to_string())
			.collect();
		assert!(
			set1.is_disjoint(&set2),
			"Pages should not have duplicate names"
		);
	});
}

#[test]
fn list_names_returns_empty_array_for_empty_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// List names in empty namespace
		let response =
			common::list_actor_names(&namespace, None, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let names = body["names"].as_array().expect("Expected names array");
		assert_eq!(
			names.len(),
			0,
			"Should return empty array for empty namespace"
		);
	});
}

// MARK: Error Cases

#[test]
fn list_names_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Try to list names with non-existent namespace
		let response = common::list_actor_names(
			"non-existent-namespace",
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
fn list_names_with_invalid_cursor_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Try with invalid cursor
		let response = common::list_actor_names(
			&namespace,
			None,
			Some("invalid-cursor-format"),
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Should fail with invalid cursor
		assert!(
			!response.status().is_success(),
			"Should fail with invalid cursor"
		);
	});
}

// MARK: Cross-Datacenter Tests

#[test]
fn list_names_fanout_to_all_datacenters() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actors with different names in different DCs
		common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "dc1-actor".to_string(),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "dc2-actor".to_string(),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		// Wait for propagation
		common::wait_for_eventual_consistency().await;

		// List names from DC 1 - should fanout to all DCs
		let response =
			common::list_actor_names(&namespace, None, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let names = body["names"].as_array().expect("Expected names array");

		// Should return names from both DCs
		let name_set: HashSet<String> = names
			.iter()
			.map(|n| n.as_str().unwrap().to_string())
			.collect();
		assert!(
			name_set.contains("dc1-actor"),
			"Should contain DC1 actor name"
		);
		assert!(
			name_set.contains("dc2-actor"),
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

		common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: shared_name.to_string(),
				key: Some("dc1-key".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: shared_name.to_string(),
				key: Some("dc2-key".to_string()),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		// Wait for propagation
		common::wait_for_eventual_consistency().await;

		// List names - should deduplicate
		let response =
			common::list_actor_names(&namespace, None, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let names = body["names"].as_array().expect("Expected names array");

		// Should return only one instance of the name
		let name_count = names
			.iter()
			.filter(|n| n.as_str().unwrap() == shared_name)
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
			common::create_actor_with_options(
				common::CreateActorOptions {
					namespace: namespace.clone(),
					name: name.to_string(),
					..Default::default()
				},
				ctx.leader_dc().guard_port(),
			)
			.await;
		}

		// List names
		let response =
			common::list_actor_names(&namespace, None, None, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		let names = body["names"].as_array().expect("Expected names array");

		// Convert to strings for comparison
		let returned_names: Vec<String> = names
			.iter()
			.map(|n| n.as_str().unwrap().to_string())
			.collect();

		// Verify alphabetical order
		let mut sorted_names = returned_names.clone();
		sorted_names.sort();
		assert_eq!(
			returned_names, sorted_names,
			"Names should be returned in alphabetical order"
		);

		// Verify expected order
		assert_eq!(returned_names[0], "alpha-actor");
		assert_eq!(returned_names[1], "beta-actor");
		assert_eq!(returned_names[2], "gamma-actor");
		assert_eq!(returned_names[3], "zebra-actor");
	});
}
