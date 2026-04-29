use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use axum::http::StatusCode;
use axum_test::TestServer;
use namespace::types::SqliteNamespaceConfig;
use rivet_util::Id;
use serde_json::{Value, json};
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
async fn rate_limit_per_namespace() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let mut config = enabled_config();
	config.admin_op_rate_per_min = 10;
	config.concurrent_admin_ops = 100;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;

	for idx in 0..10 {
		let response = post_restore(&harness.server, &format!("rate-{idx}"), namespace_id).await;
		response.assert_status(StatusCode::ACCEPTED);
	}

	let response = post_restore(&harness.server, "rate-limited", namespace_id).await;
	assert_error_code(response.json::<Value>(), "sqlite_admin", "admin_op_rate_limited");
	Ok(())
}

#[tokio::test]
async fn concurrent_admin_ops_gate() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let mut config = enabled_config();
	config.concurrent_admin_ops = 4;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;

	for idx in 0..4 {
		let response = post_restore(&harness.server, &format!("concurrent-{idx}"), namespace_id).await;
		response.assert_status(StatusCode::ACCEPTED);
	}

	let response = post_restore(&harness.server, "concurrent-rejected", namespace_id).await;
	assert_error_code(response.json::<Value>(), "sqlite_admin", "admin_op_rate_limited");
	Ok(())
}

#[tokio::test]
async fn concurrent_forks_per_src_gate() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let mut config = enabled_config();
	config.concurrent_admin_ops = 100;
	config.concurrent_forks_per_src = 2;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;

	for idx in 0..2 {
		let response = post_fork(&harness.server, "fork-src", &format!("fork-dst-{idx}"), namespace_id).await;
		response.assert_status(StatusCode::ACCEPTED);
	}

	let response = post_fork(&harness.server, "fork-src", "fork-dst-rejected", namespace_id).await;
	assert_error_code(response.json::<Value>(), "sqlite_admin", "admin_op_rate_limited");
	Ok(())
}

#[tokio::test]
async fn rate_limit_does_not_starve_describe_retention() -> Result<()> {
	let harness = TestHarness::new().await?;
	let namespace_id = Uuid::new_v4();
	let actor_id = Id::new_v1(0);
	let mut config = enabled_config();
	config.admin_op_rate_per_min = 1;
	config.concurrent_admin_ops = 100;
	seed_namespace_config(harness.udb()?, namespace_id, config).await?;
	seed_actor_namespace(harness.udb()?, actor_id, namespace_id).await?;

	let response = post_restore(&harness.server, "starve-restore", namespace_id).await;
	response.assert_status(StatusCode::ACCEPTED);

	let response = harness
		.server
		.get(&format!("/actors/{actor_id}/sqlite/retention"))
		.await;
	let body = response.json::<Value>();
	assert_ne!(body["code"], "admin_op_rate_limited");

	Ok(())
}

async fn post_restore(
	server: &TestServer,
	actor_id: &str,
	namespace_id: Uuid,
) -> axum_test::TestResponse {
	server
		.post(&format!("/actors/{actor_id}/sqlite/restore"))
		.json(&json!({
			"namespace_id": namespace_id,
			"target": { "kind": "txid", "txid": 1 },
			"mode": "dry_run",
		}))
		.await
}

async fn post_fork(
	server: &TestServer,
	src_actor_id: &str,
	dst_actor_id: &str,
	namespace_id: Uuid,
) -> axum_test::TestResponse {
	server
		.post(&format!("/actors/{src_actor_id}/sqlite/fork"))
		.json(&json!({
			"namespace_id": namespace_id,
			"target": { "kind": "latest_checkpoint" },
			"mode": "dry_run",
			"dst": { "kind": "existing", "dst_actor_id": dst_actor_id },
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

async fn seed_actor_namespace(
	udb: Arc<universaldb::Database>,
	actor_id: Id,
	namespace_id: Uuid,
) -> Result<()> {
	udb.run(move |tx| async move {
		let tx = tx.with_subspace(pegboard::keys::subspace());
		tx.write(
			&pegboard::keys::actor::NamespaceIdKey::new(actor_id),
			Id::v1(namespace_id, 0),
		)?;
		tx.write(
			&pegboard::keys::actor::NameKey::new(actor_id),
			"starve-actor".to_string(),
		)?;
		Ok(())
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
