#![cfg(debug_assertions)]

use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rivet_pools::NodeId;
use tokio::sync::Notify;
use universaldb::Database;

static PAUSE_RECONCILE: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> = Mutex::new(None);

pub struct PauseGuard {
	slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
}

pub fn pause_reconcile_for_test(database_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
	let reached = Arc::new(Notify::new());
	let release = Arc::new(Notify::new());
	*PAUSE_RECONCILE.lock() = Some((
		database_id.to_string(),
		Arc::clone(&reached),
		Arc::clone(&release),
	));

	(
		PauseGuard {
			slot: &PAUSE_RECONCILE,
		},
		reached,
		release,
	)
}

pub async fn reconcile(udb: &Database, database_id: &str) -> Result<()> {
	reconcile_inner(udb, database_id, None).await
}

pub(crate) async fn reconcile_with_node_id(
	udb: &Database,
	database_id: &str,
	node_id: NodeId,
) -> Result<()> {
	reconcile_inner(udb, database_id, Some(node_id)).await
}

pub(crate) fn reconcile_nonblocking(udb: Arc<Database>, database_id: String, node_id: NodeId) {
	if let Ok(handle) = tokio::runtime::Handle::try_current() {
		handle.spawn(async move {
			if let Err(error) = reconcile_with_node_id(&udb, &database_id, node_id).await {
				tracing::error!(?error, "sqlite takeover reconciliation failed");
			}
		});
		return;
	}

	std::thread::Builder::new()
		.name("sqlite-takeover-reconcile".to_string())
		.spawn(move || {
			let runtime = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.context("build sqlite takeover reconciliation runtime");

			let result = runtime.and_then(|runtime| {
				runtime.block_on(reconcile_with_node_id(&udb, &database_id, node_id))
			});

			if let Err(error) = result {
				tracing::error!(?error, "sqlite takeover reconciliation failed");
			}
		})
		.expect("spawn sqlite takeover reconciliation thread");
}

async fn reconcile_inner(
	_udb: &Database,
	database_id: &str,
	_node_id: Option<NodeId>,
) -> Result<()> {
	maybe_pause_reconcile_for_test(database_id).await;
	// Current depot writes branch-scoped storage, so the old database-scoped takeover scan is v1-only compatibility state.
	Ok(())
}

async fn maybe_pause_reconcile_for_test(database_id: &str) {
	let hook = PAUSE_RECONCILE
		.lock()
		.as_ref()
		.filter(|(hook_database_id, _, _)| hook_database_id == database_id)
		.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

	if let Some((reached, release)) = hook {
		reached.notify_waiters();
		release.notified().await;
	}
}

impl Drop for PauseGuard {
	fn drop(&mut self) {
		*self.slot.lock() = None;
	}
}
