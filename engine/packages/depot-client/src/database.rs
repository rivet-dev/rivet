use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;

use crate::{
	query::{BindParam, ExecResult, ExecuteResult, QueryResult},
	vfs::{
		NativeVfsHandle, SqliteTransportHandle, SqliteVfs, SqliteVfsMetrics,
		SqliteVfsMetricsSnapshot, VfsConfig, VfsPreloadHintSnapshot,
		fetch_initial_pages_for_registration,
	},
	worker::{SqliteWorkerFatalError, SqliteWorkerHandle},
};

#[derive(Clone)]
pub struct NativeDatabaseHandle {
	vfs: NativeVfsHandle,
	worker: SqliteWorkerHandle,
}

pub fn vfs_name_for_actor_database(actor_id: &str, generation: u64) -> String {
	format!("envoy-sqlite-{actor_id}-g{generation}")
}

struct GenerationFencedTransport {
	inner: SqliteTransportHandle,
	generation: u64,
}

#[async_trait]
impl crate::vfs::SqliteTransport for GenerationFencedTransport {
	async fn get_pages(
		&self,
		mut request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.get_pages(request).await
	}

	async fn commit(
		&self,
		mut request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.commit(request).await
	}
}

pub async fn open_database_from_transport(
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = vfs_name_for_actor_database(&actor_id, generation);
	let config = VfsConfig::default();
	let transport: SqliteTransportHandle = Arc::new(GenerationFencedTransport {
		inner: transport,
		generation,
	});
	let initial_pages =
		fetch_initial_pages_for_registration(transport.clone(), &actor_id, generation, &config)
			.await
			.map_err(|e| anyhow!("failed to preload sqlite pages: {e}"))?;
	let vfs = Arc::new(
		SqliteVfs::register_with_transport_and_initial_pages(
			&vfs_name,
			transport,
			actor_id.clone(),
			rt_handle,
			config,
			initial_pages,
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
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.exec(sql).await)
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
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.execute(sql, params).await)
	}

	pub async fn close(&self) -> Result<()> {
		match self.worker.close().await {
			Ok(()) => Ok(()),
			Err(error) => Err(self.fatal_error().unwrap_or(error)),
		}
	}

	pub async fn wait_for_worker_failure(&self) -> bool {
		self.worker.wait_for_failure().await
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self.vfs.take_last_error()
	}

	pub fn clone_fatal_error(&self) -> Option<String> {
		self.vfs.clone_fatal_error()
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
		self.map_worker_result(self.worker.wait_ready().await)
	}

	fn check_fatal_error(&self) -> Result<()> {
		if let Some(error) = self.fatal_error() {
			return Err(error);
		}

		Ok(())
	}

	fn map_worker_result<T>(&self, result: Result<T>) -> Result<T> {
		match result {
			Ok(value) => {
				self.check_fatal_error()?;
				Ok(value)
			}
			Err(error) => Err(self.fatal_error().unwrap_or(error)),
		}
	}

	fn fatal_error(&self) -> Option<anyhow::Error> {
		self.clone_fatal_error()
			.map(|message| SqliteWorkerFatalError::new(message).into())
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
