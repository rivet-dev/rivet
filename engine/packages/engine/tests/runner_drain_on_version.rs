mod common;

use anyhow::{Result, bail};
use axum::{Json, Router, extract::State, routing::get};
use gas::prelude::Id;
use std::collections::HashMap;
use std::sync::{
	Arc,
	atomic::{AtomicU32, Ordering},
};
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
					metadata_poll_interval: None,
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

// MARK: Metadata polling version drain tests

/// Mock metadata server state for testing metadata polling.
struct MockMetadataState {
	runner_version: AtomicU32,
}

/// Metadata response matching the RivetKit format.
#[derive(serde::Serialize)]
struct MetadataResponse {
	runtime: String,
	version: String,
	runner: MetadataRunner,
	#[serde(rename = "actorNames")]
	actor_names: HashMap<String, serde_json::Value>,
}

#[derive(serde::Serialize)]
struct MetadataRunner {
	kind: MetadataRunnerKind,
	version: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum MetadataRunnerKind {
	Serverless {},
}

async fn metadata_handler(State(state): State<Arc<MockMetadataState>>) -> Json<MetadataResponse> {
	Json(MetadataResponse {
		runtime: "rivetkit".to_string(),
		version: "1.0.0".to_string(),
		runner: MetadataRunner {
			kind: MetadataRunnerKind::Serverless {},
			version: state.runner_version.load(Ordering::Relaxed),
		},
		actor_names: HashMap::new(),
	})
}

/// Test that the metadata poller drains older runners when it detects a version upgrade
/// in the metadata response.
#[test]
fn drain_on_version_upgrade_via_metadata_polling() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;

			let runner_name = "drain-test-metadata-poll";

			// Create mock metadata server
			let mock_state = Arc::new(MockMetadataState {
				runner_version: AtomicU32::new(1),
			});

			let app = Router::new()
				.route("/metadata", get(metadata_handler))
				.with_state(mock_state.clone());

			let mock_port = portpicker::pick_unused_port().expect("failed to pick port");
			let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", mock_port))
				.await
				.expect("failed to bind");

			let server_handle = tokio::spawn(async move {
				axum::serve(listener, app).await.expect("server error");
			});

			// Give the server time to start
			tokio::time::sleep(Duration::from_millis(100)).await;

			let mock_url = format!("http://localhost:{}", mock_port);

			// Create serverless runner config with drain_on_version_upgrade enabled
			let mut datacenters = HashMap::new();
			datacenters.insert(
				"dc-1".to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind:
						rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
							url: mock_url.clone(),
							headers: None,
							request_lifespan: 30,
							slots_per_runner: 10,
							min_runners: Some(0),
							max_runners: 0,
							runners_margin: Some(0),
							metadata_poll_interval: Some(1000),
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
			let runner_v1_id = runner_v1.wait_ready().await;

			tracing::info!(%runner_v1_id, "runner v1 ready");

			// Wait for runner to be registered
			tokio::time::sleep(Duration::from_millis(500)).await;

			// Update the mock server to return version 2
			tracing::info!("updating mock metadata server to return version 2");
			mock_state.runner_version.store(2, Ordering::Relaxed);

			// Wait for the metadata poller to detect the version upgrade and drain v1.
			// The poller runs every 10 seconds, so we wait up to 20 seconds.
			wait_for_runner_drained(&ctx, namespace_id, runner_name, 1, 20)
				.await
				.expect("v1 runner should be drained via metadata polling");

			tracing::info!("v1 runner was drained via metadata polling");

			// Cleanup
			server_handle.abort();
		},
	);
}
