use std::sync::Arc;

use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;

use crate::{
	query::{BindParam, ExecResult, ExecuteResult, QueryResult},
	vfs::{
		NativeVfsHandle, SqliteTransport, SqliteVfs, SqliteVfsMetrics, SqliteVfsMetricsSnapshot,
		VfsConfig, VfsPreloadHintSnapshot, fetch_initial_main_page_for_registration,
	},
	worker::SqliteWorkerHandle,
};

#[derive(Clone)]
pub struct NativeDatabaseHandle {
	vfs: NativeVfsHandle,
	worker: SqliteWorkerHandle,
}

pub fn vfs_name_for_actor_database(actor_id: &str, generation: u64) -> String {
	format!("envoy-sqlite-{actor_id}-g{generation}")
}

pub async fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = vfs_name_for_actor_database(&actor_id, generation);
	let transport = SqliteTransport::from_envoy(handle);
	let initial_main_page = fetch_initial_main_page_for_registration(&transport, &actor_id)
		.await
		.map_err(|e| anyhow!("failed to preload sqlite main page: {e}"))?;
	let vfs = Arc::new(
		SqliteVfs::register_with_transport_and_initial_page(
			&vfs_name,
			transport,
			actor_id.clone(),
			rt_handle,
			VfsConfig::default(),
			initial_main_page,
			metrics.clone(),
		)
		.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?,
	);

	let native_db = NativeDatabaseHandle::new_with_metrics(vfs, actor_id, metrics)?;
	native_db.initialize().await?;
	Ok(native_db)
}

pub async fn open_database_from_conveyer(
	db: Arc<depot::conveyer::Db>,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = vfs_name_for_actor_database(&actor_id, generation);
	let transport = SqliteTransport::from_conveyer(db);
	let initial_main_page = fetch_initial_main_page_for_registration(&transport, &actor_id)
		.await
		.map_err(|e| anyhow!("failed to preload sqlite main page: {e}"))?;
	let vfs = Arc::new(
		SqliteVfs::register_with_transport_and_initial_page(
			&vfs_name,
			transport,
			actor_id.clone(),
			rt_handle,
			VfsConfig::default(),
			initial_main_page,
			metrics.clone(),
		)
		.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?,
	);

	let native_db = NativeDatabaseHandle::new_with_metrics(vfs, actor_id, metrics)?;
	native_db.initialize().await?;
	Ok(native_db)
}

impl NativeDatabaseHandle {
	pub fn new(vfs: NativeVfsHandle, file_name: String) -> Result<Self> {
		Self::new_with_metrics(vfs, file_name, None)
	}

	pub fn new_with_metrics(
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Result<Self> {
		Ok(Self {
			worker: SqliteWorkerHandle::start(vfs.clone(), file_name, metrics)?,
			vfs,
		})
	}

	pub async fn exec(&self, sql: String) -> Result<QueryResult> {
		self.worker.exec(sql).await
	}

	pub async fn query(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<QueryResult> {
		self.execute(sql, params).await.map(|result| QueryResult {
			columns: result.columns,
			rows: result.rows,
		})
	}

	pub async fn run(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<ExecResult> {
		self.execute(sql, params).await.map(|result| ExecResult {
			changes: result.changes,
		})
	}

	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.worker.execute(sql, params).await
	}

	pub async fn close(&self) -> Result<()> {
		self.worker.close().await
	}

	pub async fn wait_for_worker_failure(&self) -> bool {
		self.worker.wait_for_failure().await
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self.vfs.take_last_error()
	}

	pub fn snapshot_preload_hints(&self) -> VfsPreloadHintSnapshot {
		self.vfs.snapshot_preload_hints()
	}

	pub fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		self.vfs.sqlite_vfs_metrics()
	}

	#[cfg(test)]
	pub(crate) async fn pause_for_test(&self) -> tokio::sync::oneshot::Sender<()> {
		self.worker.pause_for_test().await
	}

	#[cfg(test)]
	pub(crate) fn is_closing_for_test(&self) -> bool {
		self.worker.is_closing_for_test()
	}

	#[cfg(test)]
	pub(crate) async fn panic_worker_for_test(&self) {
		self.worker.panic_for_test().await
	}

	async fn initialize(&self) -> Result<()> {
		self.worker.wait_ready().await
	}
}

#[cfg(test)]
mod tests {
	use super::vfs_name_for_actor_database;

	#[test]
	fn vfs_name_includes_actor_and_generation() {
		assert_eq!(
			vfs_name_for_actor_database("actor-123", 42),
			"envoy-sqlite-actor-123-g42"
		);
	}
}
