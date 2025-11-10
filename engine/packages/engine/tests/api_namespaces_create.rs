mod common;

use std::collections::HashSet;

// MARK: Basic functionality tests

#[test]
fn create_namespace_success() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "test-namespace".to_string(),
				display_name: "Test Namespace".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		assert_eq!(response.namespace.name, "test-namespace");
		assert_eq!(response.namespace.display_name, "Test Namespace");
	});
}

#[test]
fn create_namespace_validates_returned_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "validate-test".to_string(),
				display_name: "Validation Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		// Verify all required fields are present
		assert!(!response.namespace.namespace_id.to_string().is_empty());
		assert_eq!(response.namespace.name, "validate-test");
		assert_eq!(response.namespace.display_name, "Validation Test");
		assert!(response.namespace.create_ts > 0, "create_ts should be set");
	});
}

#[test]
fn create_namespace_generates_unique_ids() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let mut namespace_ids = HashSet::new();

		// Create 5 namespaces and verify each has a unique ID
		for i in 0..5 {
			let response = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: format!("unique-test-{}", i),
					display_name: format!("Unique Test {}", i),
				},
			)
			.await
			.expect("failed to create namespace");

			let id = response.namespace.namespace_id;
			assert!(
				namespace_ids.insert(id),
				"Duplicate namespace ID found: {}",
				id
			);
		}

		assert_eq!(namespace_ids.len(), 5, "Should have 5 unique IDs");
	});
}

#[test]
fn create_namespace_with_long_display_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Test with a reasonably long display name (100 chars should be acceptable)
		let long_display_name = "A".repeat(100);

		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "long-display".to_string(),
				display_name: long_display_name.clone(),
			},
		)
		.await
		.expect("failed to create namespace with long display name");

		assert_eq!(response.namespace.display_name, long_display_name);
	});
}

#[test]
fn create_namespace_persists_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let namespace_name = "persist-test";

		let create_response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.to_string(),
				display_name: "Persist Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace");

		let namespace_id = create_response.namespace.namespace_id;

		// Retrieve the namespace by name using list endpoint
		let list_response = common::api::public::namespaces_list(
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

		assert_eq!(list_response.namespaces.len(), 1);
		assert_eq!(list_response.namespaces[0].namespace_id, namespace_id);
		assert_eq!(list_response.namespaces[0].name, namespace_name);
	});
}

// MARK: Name validation tests

#[test]
fn create_namespace_with_valid_dns_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let valid_names = vec![
			"lowercase",
			"with-hyphens",
			"with123numbers",
			"a1b2c3",
			"starts-with-letter",
			"ends-with-number1",
		];

		for name in valid_names {
			let response = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: name.to_string(),
					display_name: format!("Valid DNS: {}", name),
				},
			)
			.await
			.unwrap_or_else(|_| panic!("failed to create namespace with valid name: {}", name));

			assert_eq!(response.namespace.name, name);
		}
	});
}

#[test]
fn create_namespace_duplicate_name_fails() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let namespace_name = "duplicate-test";

		// Create first namespace
		common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.to_string(),
				display_name: "First".to_string(),
			},
		)
		.await
		.expect("failed to create first namespace");

		// Attempt to create duplicate
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: namespace_name.to_string(),
				display_name: "Second".to_string(),
			},
		)
		.await;

		assert!(result.is_err(), "should fail to create duplicate namespace");
	});
}

#[test]
fn create_namespace_invalid_uppercase() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "UpperCase".to_string(),
				display_name: "Invalid Uppercase".to_string(),
			},
		)
		.await;

		assert!(
			result.is_err(),
			"should fail to create namespace with uppercase letters"
		);
	});
}

#[test]
fn create_namespace_invalid_special_chars() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let invalid_names = vec![
			"with_underscore",
			"with spaces",
			"with@special",
			"with.dot",
			"with/slash",
		];

		for name in invalid_names {
			let result = common::api::public::namespaces_create(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: name.to_string(),
					display_name: "Invalid Special Chars".to_string(),
				},
			)
			.await;

			assert!(
				result.is_err(),
				"should fail to create namespace with special char: {}",
				name
			);
		}
	});
}

#[test]
fn create_namespace_invalid_starts_with_hyphen() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "-starts-with-hyphen".to_string(),
				display_name: "Invalid Start".to_string(),
			},
		)
		.await;

		assert!(
			result.is_err(),
			"should fail to create namespace starting with hyphen"
		);
	});
}

#[test]
fn create_namespace_invalid_ends_with_hyphen() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "ends-with-hyphen-".to_string(),
				display_name: "Invalid End".to_string(),
			},
		)
		.await;

		assert!(
			result.is_err(),
			"should fail to create namespace ending with hyphen"
		);
	});
}

// MARK: Display name validation tests

#[test]
fn create_namespace_empty_display_name_fails() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "empty-display".to_string(),
				display_name: "".to_string(),
			},
		)
		.await;

		assert!(
			result.is_err(),
			"should fail to create namespace with empty display_name"
		);
	});
}

#[test]
fn create_namespace_with_unicode_display_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let unicode_display = "测试命名空间 🚀 Тест";

		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "unicode-display".to_string(),
				display_name: unicode_display.to_string(),
			},
		)
		.await
		.expect("failed to create namespace with unicode display name");

		assert_eq!(response.namespace.display_name, unicode_display);
	});
}

// MARK: Cross-datacenter tests

#[test]
fn create_namespace_from_leader() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "leader-test".to_string(),
				display_name: "Leader Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace from leader");

		assert_eq!(response.namespace.name, "leader-test");
		// Verify the namespace ID has the leader DC label
		let namespace_id: rivet_util::Id = response.namespace.namespace_id;
		assert_eq!(
			namespace_id.label(),
			ctx.leader_dc().config.dc_label(),
			"Namespace ID should have leader DC label"
		);
	});
}

#[test]
fn create_namespace_from_follower_routes_to_leader() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		// Create namespace from follower DC (DC2)
		let response = common::api::public::namespaces_create(
			ctx.get_dc(2).guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "follower-test".to_string(),
				display_name: "Follower Test".to_string(),
			},
		)
		.await
		.expect("failed to create namespace from follower");

		assert_eq!(response.namespace.name, "follower-test");

		// Verify the namespace ID has the leader DC label (DC1), not follower (DC2)
		let namespace_id: rivet_util::Id = response.namespace.namespace_id;
		assert_eq!(
			namespace_id.label(),
			ctx.leader_dc().config.dc_label(),
			"Namespace ID should have leader DC label even when created from follower"
		);
	});
}

// MARK: Edge cases

#[test]
fn create_namespace_empty_name_fails() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "".to_string(),
				display_name: "Empty Name".to_string(),
			},
		)
		.await;

		assert!(
			result.is_err(),
			"should fail to create namespace with empty name"
		);
	});
}

#[test]
fn create_namespace_min_length_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Single character name (minimum valid DNS subdomain length)
		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: "a".to_string(),
				display_name: "Single Char".to_string(),
			},
		)
		.await
		.expect("failed to create namespace with single character name");

		assert_eq!(response.namespace.name, "a");
	});
}

#[test]
fn create_namespace_max_length_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// DNS subdomain labels have a maximum length of 63 characters
		let max_name = "a".repeat(63);

		let response = common::api::public::namespaces_create(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::namespaces::CreateRequest {
				name: max_name.clone(),
				display_name: "Max Length".to_string(),
			},
		)
		.await
		.expect("failed to create namespace with max length name");

		assert_eq!(response.namespace.name, max_name);
	});
}
