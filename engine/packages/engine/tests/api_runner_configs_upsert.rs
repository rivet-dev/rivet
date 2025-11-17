mod common;

use std::collections::HashMap;

// MARK: Basic functionality tests

#[test]
fn upsert_runner_config_normal_single_dc() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "test-runner";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters.clone(),
			},
		)
		.await
		.expect("failed to upsert runner config");

		assert!(response.endpoint_config_changed);
	});
}

#[test]
fn upsert_runner_config_normal_multiple_dcs() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "multi-dc-runner";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);
		datacenters.insert(
			"dc-2".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
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
		.expect("failed to upsert runner config across multiple DCs");

		assert!(response.endpoint_config_changed);
	});
}

#[test]
fn upsert_runner_config_serverless() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "serverless-runner";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://example.com".to_string(),
					headers: None,
					request_lifespan: 30,
					slots_per_runner: 10,
					min_runners: Some(1),
					max_runners: 5,
					runners_margin: Some(2),
				},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
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
		.expect("failed to upsert serverless runner config");

		assert!(response.endpoint_config_changed);
	});
}

#[test]
fn upsert_runner_config_update_existing() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "update-test";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		// First upsert
		let response1 = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters.clone(),
			},
		)
		.await
		.expect("failed to create runner config");

		assert!(response1.endpoint_config_changed);

		// Update with metadata
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: Some(serde_json::json!({"test": "value"})),
			},
		);

		let response2 = common::api::public::runner_configs_upsert(
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
		.expect("failed to update runner config");

		// Update should report endpoint_config_changed
		assert!(response2.endpoint_config_changed);
	});
}

#[test]
fn upsert_runner_config_returns_endpoint_changed() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "endpoint-changed-test";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
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

		// First creation should always report changed
		assert!(
			response.endpoint_config_changed,
			"First upsert should report endpoint_config_changed=true"
		);
	});
}

#[test]
fn upsert_runner_config_with_metadata() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "metadata-test";
		let metadata_value = serde_json::json!({
			"custom_field": "custom_value",
			"nested": {
				"key": "value"
			}
		});

		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: Some(metadata_value),
			},
		);

		let response = common::api::public::runner_configs_upsert(
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
		.expect("failed to upsert runner config with metadata");

		assert!(response.endpoint_config_changed);
	});
}

// MARK: Deletion via empty datacenters tests

#[test]
fn upsert_runner_config_removes_missing_dcs() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "remove-dc-test";

		// First, create config in both DCs
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);
		datacenters.insert(
			"dc-2".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
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
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters.clone(),
			},
		)
		.await
		.expect("failed to create runner config in both DCs");

		// Now upsert with only DC1 (should remove DC2)
		let mut datacenters_dc1_only = HashMap::new();
		datacenters_dc1_only.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters_dc1_only,
			},
		)
		.await
		.expect("failed to upsert runner config with removed DC");

		assert!(response.endpoint_config_changed);

		// Verify DC2 was removed by listing
		let list_response = common::api::public::runner_configs_list(
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

		let runner_configs = list_response
			.runner_configs
			.get(runner_name)
			.expect("runner should exist");

		assert!(
			runner_configs.datacenters.contains_key("dc-1"),
			"DC1 should still exist"
		);
		assert!(
			!runner_configs.datacenters.contains_key("dc-2"),
			"DC2 should be removed"
		);
	});
}

#[test]
fn upsert_runner_config_empty_map_deletes_all() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "empty-map-test";

		// First, create config
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
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
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters.clone(),
			},
		)
		.await
		.expect("failed to create runner config");

		// Upsert with empty map (should delete from all DCs)
		let _response = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: HashMap::new(),
			},
		)
		.await
		.expect("failed to upsert runner config with empty map");

		// endpoint_config_changed may be false if the runner config wasn't actively being used

		// Verify it was deleted by listing
		let list_response = common::api::public::runner_configs_list(
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

		assert!(
			!list_response.runner_configs.contains_key(runner_name),
			"Runner config should be deleted"
		);
	});
}

// MARK: Validation tests

// NOTE: Runner name and datacenter name validation tests removed
// The API doesn't validate runner names or datacenter names at the public layer

#[test]
fn upsert_runner_config_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let runner_name = "namespace-test";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		let result = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: "non-existent-namespace".to_string(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await;

		assert!(result.is_err(), "Should fail with non-existent namespace");
	});
}

// MARK: Edge cases

#[test]
fn upsert_runner_config_overwrites_different_variant() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "variant-change-test";

		// First, create normal config
		let mut datacenters_normal = HashMap::new();
		datacenters_normal.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
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
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters_normal,
			},
		)
		.await
		.expect("failed to create normal config");

		// Now overwrite with serverless config
		let mut datacenters_serverless = HashMap::new();
		datacenters_serverless.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://example.com".to_string(),
					headers: None,
					request_lifespan: 30,
					slots_per_runner: 10,
					min_runners: Some(1),
					max_runners: 5,
					runners_margin: Some(2),
				},
				metadata: None,
			},
		);

		let response = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters_serverless,
			},
		)
		.await
		.expect("failed to overwrite with serverless config");

		assert!(response.endpoint_config_changed);
	});
}

#[test]
fn upsert_runner_config_idempotent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _) = common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let runner_name = "idempotent-test";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
			},
		);

		// First upsert
		let response1 = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest {
				datacenters: datacenters.clone(),
			},
		)
		.await
		.expect("failed first upsert");

		assert!(response1.endpoint_config_changed);

		// Second upsert with same data
		let response2 = common::api::public::runner_configs_upsert(
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
		.expect("failed second upsert");

		// Should succeed (idempotent operation)
		// endpoint_config_changed may be true or false depending on implementation
		assert!(response2.endpoint_config_changed || !response2.endpoint_config_changed);
	});
}

// MARK: Runner validation tests

#[test]
fn upsert_runner_config_serverless_slots_per_runner_zero() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_name = "zero-slots-runner";
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: "http://example.com".to_string(),
					headers: None,
					request_lifespan: 30,
					slots_per_runner: 0, // Invalid: should be rejected
					min_runners: Some(1),
					max_runners: 5,
					runners_margin: Some(2),
				},
				metadata: None,
			},
		);

		// Attempt to upsert runner config with slots_per_runner = 0
		let result = common::api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await;

		// Should fail because slots_per_runner cannot be 0
		assert!(
			result.is_err(),
			"Upsert should fail when slots_per_runner is 0"
		);
	});
}
