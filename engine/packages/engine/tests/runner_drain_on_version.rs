mod common;

use anyhow::{Result, bail};
use gas::prelude::Id;
use std::collections::HashMap;
use std::time::Duration;

/// Helper to wait for a specific runner version to be drained (drain_ts set).
/// Polls the database until the condition is met or timeout occurs.
async fn wait_for_runner_drained(
	ctx: &common::TestCtx,
	namespace_id: Id,
	runner_name: &str,
	expected_version: u32,
	timeout_secs: u64,
) -> Result<()> {
	let start = std::time::Instant::now();
	loop {
		let runners_res = ctx
			.leader_dc()
			.workflow_ctx
			.op(pegboard::ops::runner::list_for_ns::Input {
				namespace_id,
				name: Some(runner_name.to_string()),
				include_stopped: true,
				created_before: None,
				limit: 100,
			})
			.await?;

		let is_drained = runners_res
			.runners
			.iter()
			.any(|r| r.version == expected_version && r.drain_ts.is_some());

		if is_drained {
			return Ok(());
		}

		if start.elapsed() > std::time::Duration::from_secs(timeout_secs) {
			let versions: Vec<_> = runners_res
				.runners
				.iter()
				.map(|r| (r.version, r.drain_ts.is_some()))
				.collect();
			bail!(
				"timeout waiting for runner v{} to be drained. Current runners (version, is_drained): {:?}",
				expected_version,
				versions
			);
		}

		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}

/// Helper to wait for multiple runner versions to be drained.
async fn wait_for_runners_drained(
	ctx: &common::TestCtx,
	namespace_id: Id,
	runner_name: &str,
	expected_versions: &[u32],
	timeout_secs: u64,
) -> Result<()> {
	let start = std::time::Instant::now();
	loop {
		let runners_res = ctx
			.leader_dc()
			.workflow_ctx
			.op(pegboard::ops::runner::list_for_ns::Input {
				namespace_id,
				name: Some(runner_name.to_string()),
				include_stopped: true,
				created_before: None,
				limit: 100,
			})
			.await?;

		let all_drained = expected_versions.iter().all(|&version| {
			runners_res
				.runners
				.iter()
				.any(|r| r.version == version && r.drain_ts.is_some())
		});

		if all_drained {
			return Ok(());
		}

		if start.elapsed() > std::time::Duration::from_secs(timeout_secs) {
			let versions: Vec<_> = runners_res
				.runners
				.iter()
				.map(|r| (r.version, r.drain_ts.is_some()))
				.collect();
			bail!(
				"timeout waiting for runners {:?} to be drained. Current runners (version, is_drained): {:?}",
				expected_versions,
				versions
			);
		}

		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}

// MARK: Normal runner drain tests

#[test]
fn drain_on_version_upgrade_normal_runner() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_name = "drain-test-normal";

		// Create runner config with drain_on_version_upgrade enabled
		let mut datacenters = HashMap::new();
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

		// Start runner v1
		let runner_v1 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v1")
			.with_version(1)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v1");

		runner_v1.start().await.expect("failed to start runner v1");
		let runner_v1_id = runner_v1.wait_ready().await;

		tracing::info!(%runner_v1_id, "runner v1 ready");

		// Wait for runner to be registered
		tokio::time::sleep(Duration::from_millis(500)).await;

		// Start runner v2 - should trigger drain of v1
		let runner_v2 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v2")
			.with_version(2)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v2");

		runner_v2.start().await.expect("failed to start runner v2");
		let runner_v2_id = runner_v2.wait_ready().await;

		tracing::info!(%runner_v2_id, "runner v2 ready");

		// Wait for v1 to be drained
		wait_for_runner_drained(&ctx, namespace_id, runner_name, 1, 10)
			.await
			.expect("v1 runner should be drained");

		// Cleanup
		runner_v2.shutdown().await;
	});
}

#[test]
fn drain_on_version_upgrade_disabled_normal_runner() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_name = "no-drain-test-normal";

		// Create runner config with drain_on_version_upgrade disabled
		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
				drain_on_version_upgrade: false,
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

		// Start runner v1
		let runner_v1 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v1")
			.with_version(1)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v1");

		runner_v1.start().await.expect("failed to start runner v1");
		runner_v1.wait_ready().await;

		// Wait for runner to be registered
		tokio::time::sleep(Duration::from_millis(500)).await;

		// Start runner v2
		let runner_v2 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v2")
			.with_version(2)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v2");

		runner_v2.start().await.expect("failed to start runner v2");
		runner_v2.wait_ready().await;

		tokio::time::sleep(Duration::from_secs(1)).await;

		// Both runners should still be active
		let runners_res = ctx
			.leader_dc()
			.workflow_ctx
			.op(pegboard::ops::runner::list_for_ns::Input {
				namespace_id,
				name: Some(runner_name.to_string()),
				include_stopped: false,
				created_before: None,
				limit: 100,
			})
			.await
			.expect("failed to list runners");

		let active_runners: Vec<_> = runners_res
			.runners
			.iter()
			.filter(|r| r.stop_ts.is_none())
			.collect();

		assert_eq!(
			active_runners.len(),
			2,
			"Both runners should be active when drain is disabled"
		);

		// Cleanup
		runner_v1.shutdown().await;
		runner_v2.shutdown().await;
	});
}

// MARK: Serverless runner drain tests

#[test]
fn drain_on_version_upgrade_serverless_runner() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_name = "drain-test-serverless";

		// Create serverless runner config with drain_on_version_upgrade enabled
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
		.expect("failed to upsert serverless runner config");

		// Start runner v1
		let runner_v1 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v1")
			.with_version(1)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v1");

		runner_v1.start().await.expect("failed to start runner v1");
		runner_v1.wait_ready().await;

		tokio::time::sleep(Duration::from_millis(500)).await;

		// Start runner v2 - should trigger drain of v1
		let runner_v2 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v2")
			.with_version(2)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v2");

		runner_v2.start().await.expect("failed to start runner v2");
		runner_v2.wait_ready().await;

		// Wait for v1 to be drained
		wait_for_runner_drained(&ctx, namespace_id, runner_name, 1, 10)
			.await
			.expect("v1 runner should be drained");

		// Cleanup
		runner_v2.shutdown().await;
	});
}

#[test]
fn drain_on_version_upgrade_multiple_older_versions() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_name = "drain-test-multiple";

		// Create runner config with drain_on_version_upgrade enabled
		let mut datacenters = HashMap::new();
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

		// Start runners v1 and v2
		let runner_v1 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v1")
			.with_version(1)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v1");

		runner_v1.start().await.expect("failed to start runner v1");
		runner_v1.wait_ready().await;

		tokio::time::sleep(Duration::from_millis(300)).await;

		let runner_v2 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v2")
			.with_version(2)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v2");

		runner_v2.start().await.expect("failed to start runner v2");
		runner_v2.wait_ready().await;

		tokio::time::sleep(Duration::from_millis(500)).await;

		// Start runner v3 - should drain both v1 and v2
		let runner_v3 = common::test_runner::TestRunnerBuilder::new(&namespace)
			.with_runner_name(runner_name)
			.with_runner_key("key-v3")
			.with_version(3)
			.with_total_slots(10)
			.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
			.build(ctx.leader_dc())
			.await
			.expect("failed to build runner v3");

		runner_v3.start().await.expect("failed to start runner v3");
		runner_v3.wait_ready().await;

		// Wait for v1 and v2 to be drained
		wait_for_runners_drained(&ctx, namespace_id, runner_name, &[1, 2], 10)
			.await
			.expect("v1 and v2 runners should be drained");

		// Cleanup
		runner_v3.shutdown().await;
	});
}
