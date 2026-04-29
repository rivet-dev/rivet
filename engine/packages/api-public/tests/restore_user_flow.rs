use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum_test::TestServer;
use pegboard::actor_lifecycle;
use serde_json::{Value, json};
use sqlite_storage::admin::{self, OpStatus};
use uuid::Uuid;

struct TestHarness {
	server: TestServer,
	deps: rivet_test_deps::TestDeps,
}

impl TestHarness {
	async fn new() -> Result<Self> {
		let deps = rivet_test_deps::TestDeps::new().await?;
		let app = rivet_api_public::router(deps.config.clone(), deps.pools.clone()).await?;
		let server = TestServer::new(app)?;

		Ok(Self { server, deps })
	}

	fn udb(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.deps.pools.udb()?.deref().clone()))
	}
}

#[tokio::test]
async fn restore_full_user_flow() -> Result<()> {
	let harness = TestHarness::new().await?;
	let actor_id = actor_id("restore-user-flow");
	let op_id = post_restore(&harness.server, &actor_id).await?;
	let udb = harness.udb()?;

	let suspension = actor_lifecycle::read_suspension(&udb, &actor_id)
		.await?
		.context("restore should suspend actor before publish")?;
	assert_eq!(suspension.op_id, op_id);
	assert_eq!(
		actor_lifecycle::RESTORE_WS_CLOSE_CODE,
		1012,
		"restore suspension uses service restart close code"
	);
	assert_eq!(
		actor_lifecycle::RESTORE_WS_CLOSE_REASON,
		"actor.restore_in_progress"
	);

	admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, None).await?;
	admin::update_status(Arc::clone(&udb), op_id, OpStatus::Completed, None).await?;
	wait_until(Duration::from_secs(1), || {
		let udb = Arc::clone(&udb);
		let actor_id = actor_id.clone();
		async move { actor_lifecycle::read_suspension(&udb, &actor_id).await.map(|x| x.is_none()) }
	})
	.await?;

	Ok(())
}

#[tokio::test]
async fn failed_restore_leaves_suspended() -> Result<()> {
	let harness = TestHarness::new().await?;
	let actor_id = actor_id("restore-failed-suspended");
	let op_id = post_restore(&harness.server, &actor_id).await?;
	let udb = harness.udb()?;

	admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, None).await?;
	admin::update_status(Arc::clone(&udb), op_id, OpStatus::Failed, None).await?;
	tokio::time::sleep(Duration::from_millis(150)).await;

	let record = admin::read(Arc::clone(&udb), op_id)
		.await?
		.context("admin op record should exist")?;
	assert_eq!(record.status, OpStatus::Failed);
	assert!(actor_lifecycle::read_suspension(&udb, &actor_id).await?.is_some());

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
	let body = response.json::<Value>();
	let op_id = body["operation_id"]
		.as_str()
		.context("operation_id should be a string")?
		.parse()?;
	Ok(op_id)
}

async fn wait_until<F, Fut>(timeout: Duration, mut predicate: F) -> Result<()>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = Result<bool>>,
{
	let deadline = tokio::time::Instant::now() + timeout;
	loop {
		if predicate().await? {
			return Ok(());
		}
		if tokio::time::Instant::now() >= deadline {
			anyhow::bail!("condition did not become true before timeout");
		}
		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

fn actor_id(prefix: &str) -> String {
	format!("{prefix}-{}", Uuid::new_v4())
}
