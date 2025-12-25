mod common;

// MARK: Basic functionality tests

#[test]
fn list_runner_configs_empty() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Response may be empty if no runner configs exist, which is fine
		// Just verify the request succeeds and returns valid data
		assert!(response.pagination.cursor.is_none());
	});
}

#[test]
fn list_runner_configs_single_runner() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "test-runner";

		// Create a runner config
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
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert runner config");

		// List and verify
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		let runner = response
			.runner_configs
			.get(runner_name)
			.expect("runner config should exist");

		assert!(runner.datacenters.contains_key("dc-1"));
	});
}

#[test]
fn list_runner_configs_multiple_runners() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create multiple runner configs
		for i in 1..=3 {
			let runner_name = format!("runner-{}", i);
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
					runner_name: runner_name.clone(),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert runner config");
		}

		// List and verify
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Should have at least the 3 we created
		assert!(response.runner_configs.len() >= 3);
		assert!(response.runner_configs.contains_key("runner-1"));
		assert!(response.runner_configs.contains_key("runner-2"));
		assert!(response.runner_configs.contains_key("runner-3"));
	});
}

#[test]
fn list_runner_configs_multiple_dcs() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "multi-dc-runner";
		let mut datacenters = std::collections::HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
				drain_on_version_upgrade: true,
			},
		);
		datacenters.insert(
			"dc-2".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
				drain_on_version_upgrade: true,
			},
		);

		common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert runner config");

		// List and verify both DCs
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		let runner = response
			.runner_configs
			.get(runner_name)
			.expect("runner config should exist");

		assert!(runner.datacenters.contains_key("dc-1"));
		assert!(runner.datacenters.contains_key("dc-2"));
	});
}

// MARK: Filtering tests

#[test]
fn list_runner_configs_filter_by_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create multiple runner configs
		for runner_name in &["filter-test-1", "filter-test-2", "other-runner"] {
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
					runner_name: runner_name.to_string(),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert runner config");
		}

		// Filter by specific names
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec!["filter-test-1".to_string(), "filter-test-2".to_string()],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		assert!(response.runner_configs.contains_key("filter-test-1"));
		assert!(response.runner_configs.contains_key("filter-test-2"));
		assert!(!response.runner_configs.contains_key("other-runner"));
	});
}

#[test]
fn list_runner_configs_filter_by_variant_normal() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create normal runner config
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
				runner_name: "normal-runner".to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert runner config");

		// Filter by Normal variant
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: Some(
					rivet_types::keys::namespace::runner_config::RunnerConfigVariant::Normal,
				),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Should contain our normal runner
		assert!(response.runner_configs.contains_key("normal-runner"));
	});
}

#[test]
fn list_runner_configs_filter_by_variant_serverless() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create serverless runner config
		let mut datacenters = std::collections::HashMap::new();
		let mut headers = std::collections::HashMap::new();
		headers.insert("Authorization".to_string(), "Bearer test".to_string());
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://localhost:8080".to_string(),
					headers: Some(headers),
					request_lifespan: 300,
					slots_per_runner: 10,
					min_runners: Some(1),
					max_runners: 5,
					runners_margin: Some(2),
				},
				metadata: None,
				drain_on_version_upgrade: true,
			},
		);

		common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: "serverless-runner".to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert runner config");

		// Filter by Serverless variant
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: Some(
					rivet_types::keys::namespace::runner_config::RunnerConfigVariant::Serverless,
				),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Should contain our serverless runner
		assert!(response.runner_configs.contains_key("serverless-runner"));
	});
}

// MARK: Edge cases

#[test]
fn list_runner_configs_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let result = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: "non-existent-namespace".to_string(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await;

		assert!(result.is_err(), "Should fail with non-existent namespace");
	});
}

#[test]
fn list_runner_configs_empty_runner_names() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Empty string should be treated as None
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec!["".to_string()],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Empty string means no filter - just verify request succeeds
		assert!(response.pagination.cursor.is_none());
	});
}

#[test]
fn list_runner_configs_non_existent_runner() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec!["non-existent-runner".to_string()],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Should return empty
		assert!(!response.runner_configs.contains_key("non-existent-runner"));
	});
}

// MARK: Validation tests

#[test]
fn list_runner_configs_validates_returned_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "validation-runner";
		let mut datacenters = std::collections::HashMap::new();
		let mut headers = std::collections::HashMap::new();
		headers.insert("X-Custom-Header".to_string(), "value".to_string());
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://localhost:9000".to_string(),
					headers: Some(headers),
					request_lifespan: 600,
					slots_per_runner: 20,
					min_runners: Some(2),
					max_runners: 10,
					runners_margin: Some(3),
				},
				metadata: Some(serde_json::json!({"key": "value"})),
				drain_on_version_upgrade: true,
			},
		);

		common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert runner config");

		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![runner_name.to_string()],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		let runner = response
			.runner_configs
			.get(runner_name)
			.expect("runner should exist");

		let dc_config = runner.datacenters.get("dc-1").expect("dc-1 should exist");

		// Validate serverless config fields
		if let rivet_types::runner_configs::RunnerConfigKind::Serverless {
			url,
			headers,
			request_lifespan,
			slots_per_runner,
			min_runners,
			max_runners,
			runners_margin,
			..
		} = &dc_config.config.kind
		{
			assert_eq!(url, "http://localhost:9000");
			assert_eq!(request_lifespan, &600);
			assert_eq!(slots_per_runner, &20);
			assert_eq!(min_runners, &2);
			assert_eq!(max_runners, &10);
			assert_eq!(runners_margin, &3);
			assert_eq!(headers.get("X-Custom-Header").unwrap(), "value");
		} else {
			panic!("Expected serverless config");
		}

		assert!(dc_config.config.metadata.is_some());
	});
}

// MARK: Multiple variants

#[test]
fn list_runner_configs_mixed_variants() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create normal runner
		let mut normal_datacenters = std::collections::HashMap::new();
		normal_datacenters.insert(
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
				runner_name: "normal-test".to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: normal_datacenters,
			},
		)
		.await
		.expect("failed to upsert normal runner config");

		// Create serverless runner
		let mut serverless_datacenters = std::collections::HashMap::new();
		let mut headers = std::collections::HashMap::new();
		headers.insert("Auth".to_string(), "token".to_string());
		serverless_datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://localhost:7000".to_string(),
					headers: Some(headers),
					request_lifespan: 300,
					slots_per_runner: 10,
					min_runners: Some(1),
					max_runners: 5,
					runners_margin: Some(2),
				},
				metadata: None,
				drain_on_version_upgrade: true,
			},
		);

		common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: "serverless-test".to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: serverless_datacenters,
			},
		)
		.await
		.expect("failed to upsert serverless runner config");

		// List all (no filter)
		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Should contain both
		assert!(response.runner_configs.contains_key("normal-test"));
		assert!(response.runner_configs.contains_key("serverless-test"));
	});
}

#[test]
fn list_runner_configs_pagination_cursor_always_none() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let response = common::api::public::runner_configs_list(
			ctx.leader_dc().guard_port(),
			rivet_api_types::runner_configs::list::ListQuery {
				namespace: namespace.clone(),
				runner_names: None,
				runner_name: vec![],
				variant: None,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to list runner configs");

		// Cursor should always be None (no pagination)
		assert!(response.pagination.cursor.is_none());
	});
}
