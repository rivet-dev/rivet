//! Regression coverage for the staging bug where `pegboard_outbound::serverless_outbound`
//! leaks `open_dbs` entries on the process-wide `SqliteEngine` when its surrounding
//! future is dropped or its long-lived SSE call errors out before the trailing
//! `sqlite_engine.close()` runs.
//!
//! Symptom on staging:
//!
//! ```text
//! ERROR pegboard_outbound: outbound handler failed
//! err: "sqlite db already open for actor"
//! ```
//!
//! emitted for every retry on the same actor until the engine process restarts.
//!
//! This file holds two layers of coverage:
//!
//! * `serverless_outbound_releases_sqlite_when_outbound_future_is_dropped` — focused
//!   unit-style cancellation test that opens an actor on a real `SqliteEngine`,
//!   parks the owner future, then aborts it. Asserts the owner future's Drop path
//!   closes only its old generation so takeover opens are not clobbered.
//! * `serverless_outbound_releases_sqlite_after_stream_ended_early_integration`
//!   and `serverless_outbound_releases_sqlite_when_outbound_request_lifespan_elapses_integration`
//!   — full engine integration tests with a mock serverless `/start` and `/metadata`
//!   endpoint. They drive `pegboard-outbound` end-to-end through the bug-prone SSE
//!   failure paths and assert the leak signature `"sqlite db already open for actor"`
//!   never appears in engine logs during the test window.

use super::super::common;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
	Json, Router,
	body::Body,
	http::{Response, StatusCode, header},
	routing::{any, get},
};
use futures_util::{future, stream::StreamExt};
use serde_json::json;
use sqlite_storage::{engine::SqliteEngine, open::OpenConfig};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tempfile::Builder;
use tokio::sync::Notify;
use universaldb::Subspace;

// ---------------------------------------------------------------------------
// Unit-level cancellation test
// ---------------------------------------------------------------------------

struct ForceCloseOnDrop {
	engine: Arc<SqliteEngine>,
	actor_id: String,
	generation: u64,
	cleanup_notify: Arc<Notify>,
}

impl Drop for ForceCloseOnDrop {
	fn drop(&mut self) {
		let engine = self.engine.clone();
		let actor_id = std::mem::take(&mut self.actor_id);
		let generation = self.generation;
		let cleanup_notify = self.cleanup_notify.clone();
		tokio::spawn(async move {
			let _ = engine.close(&actor_id, generation).await;
			cleanup_notify.notify_one();
		});
	}
}

async fn setup_sqlite_engine() -> Result<Arc<SqliteEngine>> {
	let path = Builder::new()
		.prefix("serverless-outbound-sqlite-lifecycle-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	let db = universaldb::Database::new(Arc::new(driver));
	let subspace = Subspace::new(&("serverless-outbound-sqlite-lifecycle",));
	let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);

	Ok(Arc::new(engine))
}

#[tokio::test]
async fn serverless_outbound_releases_sqlite_when_outbound_future_is_dropped() -> Result<()> {
	let engine = setup_sqlite_engine().await?;
	let actor_id = "serverless-outbound-cancelled-actor".to_string();
	let cleanup_notify = Arc::new(Notify::new());
	let (opened_tx, opened_rx) = tokio::sync::oneshot::channel();

	let task_engine = engine.clone();
	let task_actor_id = actor_id.clone();
	let task_cleanup_notify = cleanup_notify.clone();
	let task = tokio::spawn(async move {
		let open = task_engine.open(&task_actor_id, OpenConfig::new(1)).await?;
		let _guard = ForceCloseOnDrop {
			engine: task_engine,
			actor_id: task_actor_id,
			generation: open.generation,
			cleanup_notify: task_cleanup_notify,
		};
		let _ = opened_tx.send(open.generation);
		future::pending::<()>().await;
		Ok::<_, anyhow::Error>(())
	});

	let first_generation = opened_rx.await.context("open task dropped before open")?;
	let takeover = engine.open(&actor_id, OpenConfig::new(2)).await?;
	assert!(
		takeover.generation > first_generation,
		"takeover generation {} must fence old generation {}",
		takeover.generation,
		first_generation
	);

	task.abort();
	let join_err = task
		.await
		.expect_err("aborted task should not finish cleanly");
	assert!(join_err.is_cancelled(), "unexpected join error: {join_err}");
	cleanup_notify.notified().await;

	engine.close(&actor_id, takeover.generation).await?;

	Ok(())
}

// ---------------------------------------------------------------------------
// Integration-level tests
// ---------------------------------------------------------------------------

/// `/metadata` handler that advertises a v2 envoy protocol version. The engine reads this to
/// populate `runner_config.protocol_version`, which is what makes the actor `create` op
/// dispatch the v2 (`actor2`) workflow that drives `pegboard-outbound` for serverless. Without
/// this endpoint the runner stays "v1" and the integration tests would never exercise the
/// pegboard-outbound bug surface.
async fn metadata_handler() -> Json<serde_json::Value> {
	Json(serde_json::json!({
		"runtime": "rivetkit",
		"version": "1",
		"envoyProtocolVersion": rivet_envoy_protocol::PROTOCOL_VERSION,
	}))
}

/// Mock serverless `/start` handler that returns an empty SSE response and closes the
/// connection immediately. This is the `ServerlessStreamEndedEarly` path on the engine
/// side. Returns the bound socket address, the server JoinHandle, and a counter that
/// increments on every `/start` request.
async fn start_mock_serverless_stream_ended_early()
-> (SocketAddr, tokio::task::JoinHandle<()>, Arc<AtomicU32>) {
	let connection_count = Arc::new(AtomicU32::new(0));
	let connection_count_clone = connection_count.clone();

	let router = Router::new()
		.route("/metadata", get(metadata_handler))
		.route(
			"/start",
			any(move || {
				let connection_count = connection_count_clone.clone();
				async move {
					connection_count.fetch_add(1, Ordering::SeqCst);
					Response::builder()
						.status(StatusCode::OK)
						.header(header::CONTENT_TYPE, "text/event-stream")
						.header(header::CACHE_CONTROL, "no-cache")
						.body(Body::empty())
						.unwrap()
				}
			}),
		);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	(addr, handle, connection_count)
}

/// Mock serverless `/start` handler that opens an SSE stream, emits a single `ping`
/// event so the client sees a healthy connection, and then parks indefinitely so the
/// engine's `serverless_outbound_req` is held in `source.next()` until `request_lifespan`
/// triggers the drain path.
async fn start_mock_serverless_hang() -> (SocketAddr, tokio::task::JoinHandle<()>, Arc<AtomicU32>) {
	let connection_count = Arc::new(AtomicU32::new(0));
	let connection_count_clone = connection_count.clone();

	let router = Router::new()
		.route("/metadata", get(metadata_handler))
		.route(
			"/start",
			any(move || {
				let connection_count = connection_count_clone.clone();
				async move {
					connection_count.fetch_add(1, Ordering::SeqCst);
					let initial = futures_util::stream::once(async move {
						Ok::<_, std::convert::Infallible>(axum::body::Bytes::from_static(
							b"event: ping\ndata: \n\n",
						))
					});
					let body_stream = initial.chain(futures_util::stream::pending());
					Response::builder()
						.status(StatusCode::OK)
						.header(header::CONTENT_TYPE, "text/event-stream")
						.header(header::CACHE_CONTROL, "no-cache")
						.body(Body::from_stream(body_stream))
						.unwrap()
				}
			}),
		);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	(addr, handle, connection_count)
}

async fn create_serverless_runner_config(
	guard_port: u16,
	namespace: &str,
	runner_name: &str,
	serverless_url: &str,
	request_lifespan: u32,
	max_runners: u32,
	slots_per_runner: u32,
) {
	let client = reqwest::Client::new();
	let response = client
		.put(format!(
			"http://127.0.0.1:{}/runner-configs/{}?namespace={}",
			guard_port, runner_name, namespace
		))
		.json(&json!({
			"datacenters": {
				"dc-1": {
					"serverless": {
						"url": serverless_url,
						"max_runners": max_runners,
						"slots_per_runner": slots_per_runner,
						"request_lifespan": request_lifespan,
					}
				}
			}
		}))
		.send()
		.await
		.unwrap();

	if !response.status().is_success() {
		let text = response.text().await.unwrap();
		panic!("failed to create runner config: {}", text);
	}
}

async fn create_actor(
	guard_port: u16,
	namespace: &str,
	runner_name: &str,
	key: &str,
) -> std::result::Result<String, serde_json::Value> {
	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"http://127.0.0.1:{}/actors?namespace={}",
			guard_port, namespace
		))
		.json(&json!({
			"name": "test",
			"key": key,
			"crash_policy": "sleep",
			"runner_name_selector": runner_name,
		}))
		.send()
		.await
		.unwrap();

	let success = response.status().is_success();
	let body: serde_json::Value = response.json().await.unwrap();

	if success {
		Ok(body["actor"]["actor_id"].as_str().unwrap().to_string())
	} else {
		Err(body)
	}
}

/// Sends a best-effort GET to the guard targeting `actor_id`. The intent is to wake the
/// actor from sleep so its workflow re-runs allocation, dispatching another `/start` to
/// the serverless mock. The response is ignored because the engine cannot route to a
/// dead actor; the side effect is the wake signal.
async fn poke_actor_via_guard(guard_port: u16, actor_id: &str) {
	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(2))
		.build()
		.unwrap();
	let _ = client
		.get(format!("http://127.0.0.1:{}/ping", guard_port))
		.header("X-Rivet-Target", "actor")
		.header("X-Rivet-Actor", actor_id)
		.send()
		.await;
}

/// Snapshots the captured log buffer offset at construction time so the integration
/// test can ignore any leak lines that came from earlier tests in the same process.
/// The pegboard-outbound `tracing::error!` event records the actor id into a span
/// that is not declared as a structured field, so the formatted log line does not
/// include the id; matching on the message alone is the only option without changing
/// the production logging shape.
struct LeakWatcher {
	baseline_len: usize,
}

impl LeakWatcher {
	fn new() -> Self {
		Self {
			baseline_len: common::captured_logs_snapshot().len(),
		}
	}

	/// Returns leak lines emitted after construction.
	fn new_lines(&self) -> Vec<String> {
		let logs = common::captured_logs_snapshot();
		let suffix = if logs.len() > self.baseline_len {
			&logs[self.baseline_len..]
		} else {
			""
		};
		suffix
			.lines()
			.filter(|line| line.contains("sqlite db already open for actor"))
			.map(|line| line.to_string())
			.collect()
	}
}

/// Verifies the natural `ServerlessStreamEndedEarly` failure path runs the trailing
/// SQLite close so subsequent allocations on the same actor are not blocked by a stale
/// `open_dbs` entry.
///
/// This used to reproduce the staging leak intermittently because allocation N+1
/// could call `sqlite_engine.open` while allocation N's `sqlite_engine.close` was
/// still in flight. SQLite open takeover removes that race by making the second
/// open advance the generation instead of failing.
#[test]
fn serverless_outbound_releases_sqlite_after_stream_ended_early_integration() {
	common::run(
		common::TestOpts::new(1)
			.with_timeout(60)
			.with_pegboard_outbound(),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (mock_addr, _mock_handle, connection_count) =
				start_mock_serverless_stream_ended_early().await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-leakprobe-1-{}", rand::random::<u16>());
			create_serverless_runner_config(
				guard_port,
				&namespace,
				&runner_name,
				&serverless_url,
				/* request_lifespan */ 5,
				/* max_runners */ 2,
				/* slots_per_runner */ 1,
			)
			.await;

			// Give the metadata poller time to fetch `envoyProtocolVersion` so the actor
			// `create` op dispatches the v2 (actor2) workflow that drives pegboard-outbound.
			// Without this the actor would fall back to the legacy serverless conn workflow
			// which does not exercise the bug surface.
			wait_for_protocol_version(&ctx, &namespace, &runner_name, Duration::from_secs(20))
				.await;

			// Snapshot the log buffer before any test-owned actor runs so leak lines from
			// earlier tests in the same process are excluded from the assertion.
			let leak_watcher = LeakWatcher::new();

			let actor_a = create_actor(guard_port, &namespace, &runner_name, "leakprobe-a")
				.await
				.expect("first actor creation should succeed");
			tracing::info!(actor_id = %actor_a, "created actor (first attempt)");

			// Wait until the engine's outbound handler dispatches at least one /start.
			// `connection_count` goes 0 -> 1 once pegboard-outbound has consumed the
			// ToOutbound message and started the SSE request.
			wait_for_connection_count(
				&connection_count,
				1,
				Duration::from_secs(20),
				"first serverless start was never dispatched",
			)
			.await;
			tracing::info!("first serverless start dispatched");

			// Wake the same actor and wait for a second /start dispatch. If the first
			// attempt leaked the open_dbs entry, the second outbound handler errors with
			// the leak signature.
			let attempts_after_first = connection_count.load(Ordering::SeqCst);
			poke_actor_via_guard(guard_port, &actor_a).await;
			wait_for_connection_count_with_periodic_poke(
				guard_port,
				&actor_a,
				&connection_count,
				attempts_after_first + 1,
				Duration::from_secs(20),
				"second serverless start never dispatched after wake",
			)
			.await;

			// Give close-path retries one more sweep before asserting.
			tokio::time::sleep(Duration::from_millis(500)).await;

			let leak_lines = leak_watcher.new_lines();
			assert!(
				leak_lines.is_empty(),
				"open_dbs leak observed for actor {actor_a} during ServerlessStreamEndedEarly retry path:\n{}",
				leak_lines.join("\n")
			);

			tracing::info!(
				connections = connection_count.load(Ordering::SeqCst),
				"serverless start dispatched repeatedly without leak"
			);
		},
	);
}

/// Reproduces the staging bug: parks the SSE future at `source.next()` for the entire
/// `request_lifespan`, then forces the engine through a second allocation cycle on the
/// same actor and asserts the second `sqlite_engine.open()` is not blocked by a leaked
/// `open_dbs` entry from the first attempt.
///
/// On staging this test failed with the verbatim `"sqlite db already open for actor"`
/// log line. With SQLite open takeover in place, the second allocation succeeds even
/// if the previous owner has not finished closing yet.
#[test]
fn serverless_outbound_releases_sqlite_when_outbound_request_lifespan_elapses_integration() {
	common::run(
		common::TestOpts::new(1)
			.with_timeout(60)
			.with_pegboard_outbound(),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (mock_addr, _mock_handle, connection_count) = start_mock_serverless_hang().await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-leakprobe-2-{}", rand::random::<u16>());
			create_serverless_runner_config(
				guard_port,
				&namespace,
				&runner_name,
				&serverless_url,
				/* request_lifespan */ 5,
				/* max_runners */ 2,
				/* slots_per_runner */ 1,
			)
			.await;

			wait_for_protocol_version(&ctx, &namespace, &runner_name, Duration::from_secs(20))
				.await;

			let leak_watcher = LeakWatcher::new();

			let actor_a = create_actor(guard_port, &namespace, &runner_name, "leakprobe-b")
				.await
				.expect("first actor creation should succeed");
			tracing::info!(actor_id = %actor_a, "created actor (first attempt)");

			wait_for_connection_count(
				&connection_count,
				1,
				Duration::from_secs(20),
				"first serverless start was never dispatched",
			)
			.await;
			tracing::info!("first serverless start dispatched (parked)");

			// Give `request_lifespan + drain_grace_period` enough time to elapse so the
			// engine's outbound handler completes the natural drain and runs the close
			// path. The leak would manifest as a stale open_dbs entry surviving past
			// this window.
			tokio::time::sleep(Duration::from_secs(15)).await;

			let attempts_after_first = connection_count.load(Ordering::SeqCst);
			poke_actor_via_guard(guard_port, &actor_a).await;
			wait_for_connection_count_with_periodic_poke(
				guard_port,
				&actor_a,
				&connection_count,
				attempts_after_first + 1,
				Duration::from_secs(20),
				"second serverless start never dispatched after request_lifespan elapsed",
			)
			.await;

			let leak_lines = leak_watcher.new_lines();
			assert!(
				leak_lines.is_empty(),
				"open_dbs leak observed for actor {actor_a} across attempts:\n{}",
				leak_lines.join("\n")
			);
		},
	);
}

/// Polls the runner-configs API until the metadata poller has populated
/// `protocol_version` for the given runner, indicating that subsequent actor `create`
/// ops will dispatch the v2 (actor2) workflow that drives pegboard-outbound.
async fn wait_for_protocol_version(
	ctx: &common::TestCtx,
	namespace: &str,
	runner_name: &str,
	timeout: Duration,
) {
	let start = std::time::Instant::now();
	while start.elapsed() < timeout {
		let configs = ctx
			.leader_dc()
			.workflow_ctx
			.op(pegboard::ops::runner_config::get::Input {
				runners: vec![(
					name_to_namespace_id(ctx, namespace).await,
					runner_name.to_string(),
				)],
				bypass_cache: true,
			})
			.await
			.expect("failed to read runner config");
		if configs
			.first()
			.and_then(|cfg| cfg.protocol_version)
			.is_some()
		{
			return;
		}
		// Polling required: the metadata poller runs on its own schedule and there is no
		// direct subscription channel into the test for "protocol version available".
		tokio::time::sleep(Duration::from_millis(200)).await;
	}
	panic!("metadata poller did not populate protocol_version within {timeout:?}");
}

async fn name_to_namespace_id(ctx: &common::TestCtx, namespace: &str) -> rivet_util::Id {
	ctx.leader_dc()
		.workflow_ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: namespace.to_string(),
		})
		.await
		.expect("failed to resolve namespace name")
		.expect("namespace not found")
		.namespace_id
}

async fn wait_for_connection_count(
	connection_count: &Arc<AtomicU32>,
	target: u32,
	timeout: Duration,
	message: &str,
) {
	let start = std::time::Instant::now();
	while connection_count.load(Ordering::SeqCst) < target {
		if start.elapsed() > timeout {
			panic!(
				"{message} (count={}, target={target})",
				connection_count.load(Ordering::SeqCst),
			);
		}
		// Polling required: connection_count is updated inside the mock axum handler,
		// running on a separate task with no direct signal back to the test.
		tokio::time::sleep(Duration::from_millis(200)).await;
	}
}

async fn wait_for_connection_count_with_periodic_poke(
	guard_port: u16,
	actor_id: &str,
	connection_count: &Arc<AtomicU32>,
	target: u32,
	timeout: Duration,
	message: &str,
) {
	let start = std::time::Instant::now();
	let mut last_poke = std::time::Instant::now();
	while connection_count.load(Ordering::SeqCst) < target {
		if start.elapsed() > timeout {
			panic!(
				"{message} (count={}, target={target})",
				connection_count.load(Ordering::SeqCst),
			);
		}
		// Re-poke periodically so a sleeping actor that swallowed the first wake still
		// re-runs allocation. Polling required for the same reason as above.
		if last_poke.elapsed() >= Duration::from_secs(2) {
			poke_actor_via_guard(guard_port, actor_id).await;
			last_poke = std::time::Instant::now();
		}
		tokio::time::sleep(Duration::from_millis(200)).await;
	}
}
