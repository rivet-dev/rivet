use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum_test::TestServer;
use futures_util::StreamExt;
use serde_json::{Value, json};
use sqlite_storage::{
	admin::{self, OpProgress, OpStatus},
	compactor::{CompactorConfig, worker},
	keys,
	types::{
		CheckpointMeta, DeltaMeta, RetentionConfig, decode_checkpoint_meta, encode_checkpoint_meta,
		encode_delta_meta,
	},
};
use universaldb::utils::IsolationLevel::Serializable;
use uuid::Uuid;

static WORKER_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct TestHarness {
	server: TestServer,
	deps: rivet_test_deps::TestDeps,
	worker: Option<tokio::task::JoinHandle<Result<()>>>,
}

impl Drop for TestHarness {
	fn drop(&mut self) {
		if let Some(worker) = &self.worker {
			worker.abort();
		}
	}
}

impl TestHarness {
	async fn new(http_transport: bool) -> Result<Self> {
		let deps = rivet_test_deps::TestDeps::new().await?;
		let app = rivet_api_public::router(deps.config.clone(), deps.pools.clone()).await?;
		let server = if http_transport {
			TestServer::builder().http_transport().build(app)?
		} else {
			TestServer::new(app)?
		};

		Ok(Self {
			server,
			deps,
			worker: None,
		})
	}

	async fn start_compactor(&mut self) -> Result<()> {
		let udb = self.udb()?;
		let ups = self.deps.pools.ups()?;
		let holder = self.deps.pools.node_id();
		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		worker::test_hooks::set_run_ready_signal(ready_tx);
		let handle = tokio::spawn(worker::test_hooks::run_for_test(
			udb,
			ups,
			CompactorConfig::default(),
			holder,
		));
		ready_rx.await.context("sqlite compactor worker did not start")?;
		self.worker = Some(handle);
		Ok(())
	}

	fn udb(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.deps.pools.udb()?.deref().clone()))
	}
}

#[tokio::test]
async fn post_restore_returns_op_id_immediately() -> Result<()> {
	let harness = TestHarness::new(false).await?;
	let actor_id = actor_id("restore-immediate");
	let started = tokio::time::Instant::now();

	let response = harness
		.server
		.post(&format!("/actors/{actor_id}/sqlite/restore"))
		.json(&json!({
			"target": { "kind": "txid", "txid": 1 },
			"mode": "dry_run",
		}))
		.await;

	response.assert_status(StatusCode::ACCEPTED);
	assert!(started.elapsed() < Duration::from_millis(100));
	let body = response.json::<Value>();
	assert_eq!(body["status"], "pending");
	let op_id = parse_op_id(&body)?;
	let record = admin::read(harness.udb()?, op_id)
		.await?
		.context("admin op record should exist")?;
	assert_eq!(record.status, OpStatus::Pending);
	assert_eq!(record.actor_id, actor_id);

	Ok(())
}

#[tokio::test]
async fn post_fork_returns_op_id_immediately() -> Result<()> {
	let harness = TestHarness::new(false).await?;
	let dst_actor_id = actor_id("fork-dst");
	let actor_id = actor_id("fork-immediate");
	let started = tokio::time::Instant::now();

	let response = harness
		.server
		.post(&format!("/actors/{actor_id}/sqlite/fork"))
		.json(&json!({
			"target": { "kind": "latest_checkpoint" },
			"mode": "dry_run",
			"dst": { "kind": "existing", "dst_actor_id": dst_actor_id },
		}))
		.await;

	response.assert_status(StatusCode::ACCEPTED);
	assert!(started.elapsed() < Duration::from_millis(100));
	let body = response.json::<Value>();
	assert_eq!(body["status"], "pending");
	let op_id = parse_op_id(&body)?;
	let record = admin::read(harness.udb()?, op_id)
		.await?
		.context("admin op record should exist")?;
	assert_eq!(record.status, OpStatus::Pending);
	assert_eq!(record.actor_id, actor_id);

	Ok(())
}

#[tokio::test]
async fn get_operation_polls_record() -> Result<()> {
	let harness = TestHarness::new(false).await?;
	let actor_id = actor_id("poll");
	let op_id = post_restore(&harness.server, &actor_id).await?;

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/operations/{op_id}"))
		.await;
	response.assert_status_ok();
	assert_eq!(response.json::<Value>()["status"], "Pending");

	admin::update_status(harness.udb()?, op_id, OpStatus::InProgress, None).await?;
	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/operations/{op_id}"))
		.await;
	response.assert_status_ok();
	assert_eq!(response.json::<Value>()["status"], "InProgress");

	Ok(())
}

#[tokio::test]
async fn sse_stream_emits_progress() -> Result<()> {
	let harness = TestHarness::new(true).await?;
	let actor_id = actor_id("sse-progress");
	let op_id = post_restore(&harness.server, &actor_id).await?;
	let url = harness
		.server
		.server_url(&format!("/actors/{actor_id}/sqlite/operations/{op_id}/sse"))?;
	let udb = harness.udb()?;

	let update = tokio::spawn(async move {
		tokio::time::sleep(Duration::from_millis(50)).await;
		admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, None).await?;
		admin::update_progress(
			Arc::clone(&udb),
			op_id,
			OpProgress {
				step: "copying".to_string(),
				bytes_done: 1,
				bytes_total: 2,
				started_at_ms: 1,
				eta_ms: Some(10),
				current_tx_index: 1,
				total_tx_count: 2,
			},
		)
		.await?;
		admin::update_status(udb, op_id, OpStatus::Completed, None).await
	});

	let events = collect_sse_events(url.as_str(), 3).await?;
	update.await??;
	assert!(events.iter().any(|event| event.contains("\"Pending\"")));
	assert!(events.iter().any(|event| event.contains("\"copying\"")));

	Ok(())
}

#[tokio::test]
async fn sse_stream_closes_on_terminal() -> Result<()> {
	let harness = TestHarness::new(true).await?;
	let actor_id = actor_id("sse-terminal");
	let op_id = post_restore(&harness.server, &actor_id).await?;
	let udb = harness.udb()?;
	admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, None).await?;
	admin::update_status(udb, op_id, OpStatus::Completed, None).await?;
	let url = harness
		.server
		.server_url(&format!("/actors/{actor_id}/sqlite/operations/{op_id}/sse"))?;

	let events = collect_sse_events(url.as_str(), 2).await?;
	assert_eq!(events.len(), 1);
	assert!(events[0].contains("\"Completed\""));

	Ok(())
}

#[tokio::test]
async fn get_retention_basic() -> Result<()> {
	let _lock = WORKER_TEST_LOCK.lock().await;
	let mut harness = TestHarness::new(false).await?;
	harness.start_compactor().await?;
	let actor_id = actor_id("retention-get");

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/retention"))
		.await;

	response.assert_status_ok();
	let body = response.json::<Value>();
	assert_eq!(body["retention_config"]["retention_ms"], 0);
	assert_eq!(body["checkpoints"].as_array().unwrap().len(), 0);

	Ok(())
}

#[tokio::test]
async fn put_retention_updates() -> Result<()> {
	let _lock = WORKER_TEST_LOCK.lock().await;
	let mut harness = TestHarness::new(false).await?;
	harness.start_compactor().await?;
	let actor_id = actor_id("retention-put");
	let config = RetentionConfig {
		retention_ms: 10_000,
		checkpoint_interval_ms: 1_000,
		max_checkpoints: 3,
	};

	let response = harness
		.server
		.put(&format!("/actors/{actor_id}/sqlite/retention"))
		.json(&config)
		.await;
	response.assert_status_ok();
	assert_eq!(response.json::<Value>()["retention_config"]["retention_ms"], 10_000);

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/retention"))
		.await;
	response.assert_status_ok();
	assert_eq!(response.json::<Value>()["retention_config"]["max_checkpoints"], 3);

	Ok(())
}

#[tokio::test]
async fn post_refcount_clear() -> Result<()> {
	let _lock = WORKER_TEST_LOCK.lock().await;
	let mut harness = TestHarness::new(false).await?;
	harness.start_compactor().await?;
	let actor_id = actor_id("refcount-clear");
	seed_checkpoint_refcount(harness.udb()?, &actor_id, 4, 9).await?;

	let response = harness
		.server
		.post(&format!("/actors/{actor_id}/sqlite/refcount/clear"))
		.json(&json!({ "kind": "checkpoint", "txid": 4 }))
		.await;

	response.assert_status_ok();
	assert_eq!(response.json::<Value>()["cleared"], true);
	let refcount = read_checkpoint_refcount(harness.udb()?, &actor_id, 4).await?;
	assert_eq!(refcount, 0);

	Ok(())
}

#[tokio::test]
async fn error_responses_are_rivet_error_shape() -> Result<()> {
	let harness = TestHarness::new(false).await?;
	let actor_id = actor_id("error-shape");
	let op_id = Uuid::new_v4();

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/operations/{op_id}"))
		.await;

	response.assert_status(StatusCode::NOT_FOUND);
	let body = response.json::<Value>();
	assert!(body.get("group").and_then(Value::as_str).is_some());
	assert!(body.get("code").and_then(Value::as_str).is_some());
	assert!(body.get("message").and_then(Value::as_str).is_some());

	Ok(())
}

async fn post_restore(server: &TestServer, actor_id: &str) -> Result<Uuid> {
	let response = server
		.post(&format!("/actors/{actor_id}/sqlite/restore"))
		.json(&json!({
			"target": { "kind": "txid", "txid": 1 },
			"mode": "dry_run",
		}))
		.await;
	response.assert_status(StatusCode::ACCEPTED);
	parse_op_id(&response.json::<Value>())
}

async fn collect_sse_events(url: &str, max_events: usize) -> Result<Vec<String>> {
	let response = reqwest::get(url).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let mut chunks = response.bytes_stream();
	let mut buffer = String::new();
	let mut events = Vec::new();
	let deadline = tokio::time::Instant::now() + Duration::from_secs(2);

	while events.len() < max_events && tokio::time::Instant::now() < deadline {
		let remaining = deadline - tokio::time::Instant::now();
		let Ok(next_chunk) = tokio::time::timeout(remaining, chunks.next()).await else {
			break;
		};
		let Some(chunk) = next_chunk else {
			break;
		};
		let chunk = chunk?;
		buffer.push_str(std::str::from_utf8(&chunk)?);
		while let Some(idx) = buffer.find("\n\n") {
			let raw = buffer[..idx].to_string();
			buffer = buffer[idx + 2..].to_string();
			if let Some(data) = raw.lines().find_map(|line| line.strip_prefix("data:")) {
				events.push(data.trim().to_string());
			}
		}
	}

	Ok(events)
}

async fn seed_checkpoint_refcount(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	txid: u64,
	refcount: u32,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&keys::checkpoint_meta_key(&actor_id, txid),
				&encode_checkpoint_meta(CheckpointMeta {
					taken_at_ms: 1,
					head_txid: txid,
					db_size_pages: 1,
					byte_count: 1,
					refcount,
					pinned_reason: Some("test".to_string()),
				})?,
			);
			tx.informal().set(
				&keys::delta_meta_key(&actor_id, txid),
				&encode_delta_meta(DeltaMeta {
					taken_at_ms: 1,
					byte_count: 1,
					refcount: 0,
				})?,
			);
			Ok(())
		}
	})
	.await
}

async fn read_checkpoint_refcount(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	txid: u64,
) -> Result<u32> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let bytes = tx
				.informal()
				.get(&keys::checkpoint_meta_key(&actor_id, txid), Serializable)
				.await?
				.context("checkpoint meta should exist")?;
			Ok(decode_checkpoint_meta(&bytes)?.refcount)
		}
	})
	.await
}

fn parse_op_id(body: &Value) -> Result<Uuid> {
	body["operation_id"]
		.as_str()
		.context("operation_id should be a string")?
		.parse()
		.context("operation_id should be a uuid")
}

fn actor_id(suffix: &str) -> String {
	format!("api-sqlite-admin-{suffix}-{}", Uuid::new_v4())
}
