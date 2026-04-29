use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum_test::TestServer;
use namespace::types::SqliteNamespaceConfig;
use rivet_api_public::actors::sqlite_admin::{AuditStage, test_hooks};
use rivet_util::Id;
use serde_json::{Value, json};
use sqlite_storage::admin::{self, OpStatus, SqliteOpSubject, decode_sqlite_op_request};
use universalpubsub::NextOutput;
use uuid::Uuid;

struct TestHarness {
	server: TestServer,
	deps: rivet_test_deps::TestDeps,
}

impl TestHarness {
	async fn new() -> Result<Self> {
		let deps = rivet_test_deps::TestDeps::new().await?;
		let app = rivet_api_public::router(deps.config.clone(), deps.pools.clone()).await?;
		Ok(Self {
			server: TestServer::new(app)?,
			deps,
		})
	}

	fn udb(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.deps.pools.udb()?.deref().clone()))
	}
}

#[tokio::test]
async fn authz_dry_run_restore_requires_pitr_read() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let mut config = enabled_config();
	config.allow_pitr_read = false;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;

	let response = post_restore(&harness.server, "authz-dry-run", namespace_id, "dry_run").await;

	assert_error_code(response.json::<Value>(), "sqlite_admin", "pitr_disabled_for_namespace");
	Ok(())
}

#[tokio::test]
async fn authz_apply_restore_requires_destructive() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let mut config = enabled_config();
	config.allow_pitr_destructive = false;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;

	let response = post_restore(&harness.server, "authz-apply", namespace_id, "apply").await;

	assert_error_code(
		response.json::<Value>(),
		"sqlite_admin",
		"pitr_destructive_disabled_for_namespace",
	);
	Ok(())
}

#[tokio::test]
async fn authz_fork_requires_both_namespaces() -> Result<()> {
	let harness = TestHarness::new().await?;
	let src_namespace_id = Uuid::new_v4();
	let dst_namespace_id = Uuid::new_v4();
	seed_namespace_config(harness.udb()?, src_namespace_id, enabled_config()).await?;
	let mut dst_config = enabled_config();
	dst_config.allow_fork = false;
	seed_namespace_config(harness.udb()?, dst_namespace_id, dst_config).await?;

	let response = harness
		.server
		.post("/actors/authz-fork-src/sqlite/fork")
		.json(&json!({
			"namespace_id": src_namespace_id,
			"target": { "kind": "latest_checkpoint" },
			"mode": "dry_run",
			"dst": { "kind": "allocate", "dst_namespace_id": dst_namespace_id },
		}))
		.await;

	assert_error_code(response.json::<Value>(), "sqlite_admin", "fork_disabled_for_namespace");
	Ok(())
}

#[tokio::test]
async fn audit_fields_injected_into_envelope() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	seed_namespace_config(harness.udb()?, namespace_id, enabled_config()).await?;
	let mut sub = harness.deps.pools.ups()?.subscribe(SqliteOpSubject).await?;

	let response = post_restore(&harness.server, "audit-envelope", namespace_id, "dry_run").await;

	response.assert_status(StatusCode::ACCEPTED);
	let message = tokio::time::timeout(Duration::from_secs(1), sub.next()).await??;
	let NextOutput::Message(message) = message else {
		panic!("sqlite op subscriber closed");
	};
	let request = decode_sqlite_op_request(&message.payload)?;
	assert_eq!(request.audit.caller_id, "api-public");
	assert_eq!(request.audit.namespace_id, namespace_id);
	assert!(request.audit.request_origin_ts_ms > 0);

	Ok(())
}

#[tokio::test]
async fn audit_log_emitted_on_acked_and_completed() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	seed_namespace_config(harness.udb()?, namespace_id, enabled_config()).await?;
	let _ = test_hooks::take_audit_log();

	let response = post_restore(&harness.server, "audit-log", namespace_id, "dry_run").await;
	response.assert_status(StatusCode::ACCEPTED);
	let body = response.json::<Value>();
	let op_id = parse_op_id(&body)?;
	admin::update_status(harness.udb()?, op_id, OpStatus::InProgress, None).await?;
	admin::update_status(harness.udb()?, op_id, OpStatus::Completed, None).await?;

	let entries = wait_for_audit_events(op_id).await?;
	let entries = entries
		.iter()
		.filter(|entry| entry.operation_id == op_id)
		.collect::<Vec<_>>();
	assert!(entries.iter().any(|entry| entry.stage == AuditStage::Acked));
	assert!(entries.iter().any(|entry| entry.stage == AuditStage::Terminal));
	assert!(entries.iter().all(|entry| entry.namespace_id == namespace_id));

	Ok(())
}

async fn post_restore(
	server: &TestServer,
	actor_id: &str,
	namespace_id: Uuid,
	mode: &str,
) -> axum_test::TestResponse {
	server
		.post(&format!("/actors/{actor_id}/sqlite/restore"))
		.json(&json!({
			"namespace_id": namespace_id,
			"target": { "kind": "txid", "txid": 1 },
			"mode": mode,
		}))
		.await
}

async fn seed_namespace_config(
	udb: Arc<universaldb::Database>,
	namespace_id: Uuid,
	config: SqliteNamespaceConfig,
) -> Result<()> {
	udb.run(move |tx| {
		let config = config.clone();
		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			tx.write(
				&namespace::keys::sqlite_config_key(Id::v1(namespace_id, 0)),
				config,
			)?;
			Ok(())
		}
	})
	.await
}

fn enabled_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		allow_pitr_read: true,
		allow_pitr_destructive: true,
		allow_pitr_admin: true,
		allow_fork: true,
		admin_op_rate_per_min: 100,
		concurrent_admin_ops: 100,
		concurrent_forks_per_src: 100,
		..SqliteNamespaceConfig::default()
	}
}

fn assert_error_code(body: Value, group: &str, code: &str) {
	assert_eq!(body["group"], group);
	assert_eq!(body["code"], code);
}

fn parse_op_id(body: &Value) -> Result<Uuid> {
	body["operation_id"]
		.as_str()
		.context("operation_id should be a string")?
		.parse()
		.context("operation_id should be a uuid")
}

async fn wait_for_audit_events(
	op_id: Uuid,
) -> Result<Vec<rivet_api_public::actors::sqlite_admin::SqliteAdminAuditEvent>> {
	let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
	let mut collected = Vec::new();
	loop {
		collected.extend(test_hooks::take_audit_log());
		let has_terminal = collected
			.iter()
			.any(|entry| entry.operation_id == op_id && entry.stage == AuditStage::Terminal);
		if has_terminal || tokio::time::Instant::now() >= deadline {
			return Ok(collected);
		}
		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}
