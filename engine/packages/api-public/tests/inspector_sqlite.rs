use std::{ops::Deref, sync::Arc};

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum_test::TestServer;
use rivet_config::{config::Auth, secret::Secret};
use rivet_util::Id;
use serde_json::{Value, json};
use sqlite_storage::{
	admin::{AdminOpRecord, AuditFields, OpKind, OpStatus, encode_admin_op_record},
	keys,
	types::{
		CheckpointEntry, CheckpointMeta, Checkpoints, encode_checkpoint_meta, encode_checkpoints,
	},
};
use uuid::Uuid;

static INSPECTOR_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct TestHarness {
	server: TestServer,
	deps: rivet_test_deps::TestDeps,
}

impl TestHarness {
	async fn new(auth_token: Option<&str>) -> Result<Self> {
		let deps = rivet_test_deps::TestDeps::new().await?;
		let config = if let Some(token) = auth_token {
			let mut root = deps.config.deref().clone();
			root.auth = Some(Auth {
				admin_token: Secret::new(token.to_string()),
			});
			rivet_config::Config::from_root(root)
		} else {
			deps.config.clone()
		};
		let app = rivet_api_public::router(config, deps.pools.clone()).await?;
		let server = TestServer::builder().http_transport().build(app)?;
		Ok(Self { server, deps })
	}

	fn udb(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.deps.pools.udb()?.deref().clone()))
	}
}

#[tokio::test]
async fn inspector_lists_checkpoints() -> Result<()> {
	let _lock = INSPECTOR_TEST_LOCK.lock().await;
	let harness = TestHarness::new(None).await?;
	let actor_id = actor_id("checkpoints");
	seed_checkpoints(harness.udb()?, &actor_id, vec![(1, 100, 10, 0), (2, 200, 20, 2), (3, 300, 30, 0)]).await?;

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/checkpoints"))
		.await;

	response.assert_status_ok();
	let body = response.json::<Value>();
	let checkpoints = body["checkpoints"].as_array().context("checkpoints should be an array")?;
	assert_eq!(checkpoints.len(), 3);
	assert_eq!(checkpoints[1]["ckp_txid"], 2);
	assert_eq!(checkpoints[1]["taken_at_ms"], 200);
	assert_eq!(checkpoints[1]["byte_count"], 20);
	assert_eq!(checkpoints[1]["refcount"], 2);
	assert_eq!(checkpoints[1]["pinned_reason"], "test pin");

	Ok(())
}

#[tokio::test]
async fn inspector_namespace_overview_aggregates() -> Result<()> {
	let _lock = INSPECTOR_TEST_LOCK.lock().await;
	let harness = TestHarness::new(None).await?;
	let namespace_id = Uuid::new_v4();
	let udb = harness.udb()?;
	for idx in 0..5 {
		seed_namespace_metrics(Arc::clone(&udb), namespace_id, &format!("actor-{idx}"), 100 + idx, 10 + idx, idx + 1, idx % 2).await?;
		seed_admin_record(
			Arc::clone(&udb),
			&actor_id(&format!("overview-{idx}")),
			namespace_id,
			OpKind::Restore,
			now_ms()? - 1_000 + i64::from(idx),
		).await?;
	}

	let response = harness
		.server
		.get(&format!("/namespaces/{namespace_id}/sqlite/overview"))
		.await;

	response.assert_status_ok();
	let body = response.json::<Value>();
	assert_eq!(body["storage_used_live_bytes"], 510);
	assert_eq!(body["storage_used_pitr_bytes"], 60);
	assert_eq!(body["checkpoint_count"], 15);
	assert_eq!(body["pinned_checkpoint_warnings"], 2);
	assert_eq!(body["recent_op_counts"]["restore"], 5);

	Ok(())
}

#[tokio::test]
async fn inspector_admin_op_history_paginates() -> Result<()> {
	let _lock = INSPECTOR_TEST_LOCK.lock().await;
	let harness = TestHarness::new(None).await?;
	let actor_id = actor_id("history");
	let namespace_id = Uuid::new_v4();
	let base = now_ms()? - 10_000;
	for idx in 0..100 {
		seed_admin_record(
			harness.udb()?,
			&actor_id,
			namespace_id,
			if idx % 2 == 0 { OpKind::Restore } else { OpKind::Fork },
			base + idx,
		).await?;
	}

	let first = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/admin-ops?since={}&limit=40", base - 1))
		.await;
	first.assert_status_ok();
	let first_body = first.json::<Value>();
	assert_eq!(first_body["operations"].as_array().unwrap().len(), 40);
	let next_since = first_body["next_since"].as_i64().context("next_since should exist")?;

	let second = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/admin-ops?since={next_since}&limit=100"))
		.await;
	second.assert_status_ok();
	let second_body = second.json::<Value>();
	assert_eq!(second_body["operations"].as_array().unwrap().len(), 60);
	assert!(second_body["next_since"].is_null());

	Ok(())
}

#[tokio::test]
async fn inspector_authz_required() -> Result<()> {
	let _lock = INSPECTOR_TEST_LOCK.lock().await;
	let harness = TestHarness::new(Some("inspector-token")).await?;
	let actor_id = actor_id("authz");

	let missing = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/checkpoints"))
		.await;
	missing.assert_status(StatusCode::UNAUTHORIZED);

	let invalid = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/checkpoints"))
		.authorization_bearer("wrong")
		.await;
	invalid.assert_status(StatusCode::UNAUTHORIZED);

	let valid = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/checkpoints"))
		.authorization_bearer("inspector-token")
		.await;
	valid.assert_status_ok();

	Ok(())
}

#[tokio::test]
async fn inspector_ws_mirrors_http() -> Result<()> {
	let _lock = INSPECTOR_TEST_LOCK.lock().await;
	let harness = TestHarness::new(None).await?;
	let actor_id = actor_id("ws");
	seed_checkpoints(harness.udb()?, &actor_id, vec![(7, 700, 70, 1)]).await?;

	let http = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/checkpoints"))
		.await;
	http.assert_status_ok();

	let mut websocket = harness
		.server
		.get_websocket("/sqlite/inspector/ws")
		.await
		.into_websocket()
		.await;
	websocket
		.send_json(&json!({
			"route": "checkpoints",
			"actor_id": actor_id,
		}))
		.await;
	let ws_body = websocket.receive_json::<Value>().await;
	assert_eq!(ws_body["data"], http.json::<Value>());

	Ok(())
}

async fn seed_checkpoints(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	rows: Vec<(u64, i64, u64, u32)>,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let rows = rows.clone();
		async move {
			let entries = rows
				.iter()
				.map(|(ckp_txid, taken_at_ms, byte_count, refcount)| CheckpointEntry {
					ckp_txid: *ckp_txid,
					taken_at_ms: *taken_at_ms,
					byte_count: *byte_count,
					refcount: *refcount,
				})
				.collect::<Vec<_>>();
			tx.informal().set(
				&keys::meta_checkpoints_key(&actor_id),
				&encode_checkpoints(Checkpoints { entries })?,
			);
			for (ckp_txid, taken_at_ms, byte_count, refcount) in rows {
				tx.informal().set(
					&keys::checkpoint_meta_key(&actor_id, ckp_txid),
					&encode_checkpoint_meta(CheckpointMeta {
						taken_at_ms,
						head_txid: ckp_txid,
						db_size_pages: 1,
						byte_count,
						refcount,
						pinned_reason: Some("test pin".to_string()),
					})?,
				);
			}
			Ok(())
		}
	})
	.await
}

async fn seed_namespace_metrics(
	udb: Arc<universaldb::Database>,
	namespace_id: Uuid,
	actor_name: &str,
	live: i64,
	pitr: i64,
	checkpoints: i64,
	pinned: i64,
) -> Result<()> {
	let actor_name = actor_name.to_string();
	udb.run(move |tx| {
		let actor_name = actor_name.clone();
		async move {
			let namespace_id = Id::v1(namespace_id, 0);
			let namespace_tx = tx.with_subspace(namespace::keys::subspace());
			namespace_tx.write(
				&namespace::keys::metric::MetricKey::new(
					namespace_id,
					namespace::keys::metric::Metric::SqliteStorageLiveUsed(actor_name.clone()),
				),
				live,
			)?;
			namespace_tx.write(
				&namespace::keys::metric::MetricKey::new(
					namespace_id,
					namespace::keys::metric::Metric::SqliteStoragePitrUsed(actor_name.clone()),
				),
				pitr,
			)?;
			namespace_tx.write(
				&namespace::keys::metric::MetricKey::new(
					namespace_id,
					namespace::keys::metric::Metric::SqliteCheckpointCount(actor_name.clone()),
				),
				checkpoints,
			)?;
			namespace_tx.write(
				&namespace::keys::metric::MetricKey::new(
					namespace_id,
					namespace::keys::metric::Metric::SqliteCheckpointPinned(actor_name),
				),
				pinned,
			)?;
			Ok(())
		}
	})
	.await
}

async fn seed_admin_record(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	namespace_id: Uuid,
	op_kind: OpKind,
	created_at_ms: i64,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let op_id = Uuid::new_v4();
			let record = AdminOpRecord {
				operation_id: op_id,
				op_kind,
				actor_id: actor_id.clone(),
				created_at_ms,
				last_progress_at_ms: created_at_ms,
				status: OpStatus::Completed,
				holder_id: None,
				progress: None,
				result: None,
				audit: AuditFields {
					caller_id: "test".to_string(),
					request_origin_ts_ms: created_at_ms,
					namespace_id,
				},
			};
			tx.informal().set(
				&keys::meta_admin_op_key(&actor_id, op_id),
				&encode_admin_op_record(record)?,
			);
			Ok(())
		}
	})
	.await
}

fn actor_id(suffix: &str) -> String {
	format!("api-sqlite-inspector-{suffix}-{}", Uuid::new_v4())
}

fn now_ms() -> Result<i64> {
	Ok(std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)?
		.as_millis()
		.try_into()?)
}
