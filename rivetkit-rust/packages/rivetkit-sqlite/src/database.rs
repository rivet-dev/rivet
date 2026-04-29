use std::sync::Arc;

use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;

use crate::{
	connection_manager::{NativeConnectionManager, NativeConnectionManagerConfig},
	query::{
		BindParam, ExecResult, QueryResult, exec_statements, execute_statement, query_statement,
	},
	vfs::{
		NativeVfsHandle, SqliteVfs, SqliteVfsMetrics, VfsConfig, VfsPreloadHintSnapshot,
		configure_connection_for_database, verify_batch_atomic_writes,
	},
};

#[derive(Clone)]
pub struct NativeDatabaseHandle {
	file_name: String,
	vfs: NativeVfsHandle,
	manager: NativeConnectionManager,
}

pub fn vfs_name_for_actor_database(actor_id: &str, generation: u64) -> String {
	format!("envoy-sqlite-{actor_id}-g{generation}")
}

pub async fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	startup_data: Option<protocol::SqliteStartupData>,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	let startup =
		startup_data.ok_or_else(|| anyhow!("missing sqlite startup data for actor {actor_id}"))?;
	let vfs_name = vfs_name_for_actor_database(&actor_id, startup.generation);
	let vfs = SqliteVfs::register(
		&vfs_name,
		handle,
		actor_id.clone(),
		rt_handle,
		startup,
		VfsConfig::default(),
		metrics,
	)
	.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?;

	let native_db = NativeDatabaseHandle::new(
		vfs,
		actor_id,
		NativeConnectionManagerConfig::default(),
	);
	native_db.initialize().await?;
	Ok(native_db)
}

impl NativeDatabaseHandle {
	pub fn new(
		vfs: NativeVfsHandle,
		file_name: String,
		config: NativeConnectionManagerConfig,
	) -> Self {
		Self {
			file_name: file_name.clone(),
			manager: NativeConnectionManager::new(vfs.clone(), file_name, config),
			vfs,
		}
	}

	pub async fn exec(&self, sql: String) -> Result<QueryResult> {
		self.with_configured_write_connection(move |db| exec_statements(db, &sql))
			.await
	}

	pub async fn query(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<QueryResult> {
		self.with_configured_write_connection(move |db| {
			query_statement(db, &sql, params.as_deref())
		})
		.await
	}

	pub async fn run(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<ExecResult> {
		self.with_configured_write_connection(move |db| {
			execute_statement(db, &sql, params.as_deref())
		})
		.await
	}

	pub async fn close(&self) -> Result<()> {
		self.manager.close().await
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self.vfs.take_last_error()
	}

	pub fn snapshot_preload_hints(&self) -> VfsPreloadHintSnapshot {
		self.vfs.snapshot_preload_hints()
	}

	async fn initialize(&self) -> Result<()> {
		let vfs = self.vfs.clone();
		let file_name = self.file_name.clone();
		self.manager
			.with_write_connection_state(move |db, newly_opened| {
				if newly_opened {
					configure_connection_for_database(db, &vfs, &file_name)
						.map_err(anyhow::Error::msg)?;
				}
				verify_batch_atomic_writes(db, &vfs, &file_name).map_err(anyhow::Error::msg)
			})
			.await
	}

	async fn with_configured_write_connection<T, F>(&self, f: F) -> Result<T>
	where
		T: Send + 'static,
		F: FnOnce(*mut libsqlite3_sys::sqlite3) -> Result<T> + Send + 'static,
	{
		let vfs = self.vfs.clone();
		let file_name = self.file_name.clone();
		self.manager
			.with_write_connection_state(move |db, newly_opened| {
				if newly_opened {
					configure_connection_for_database(db, &vfs, &file_name)
						.map_err(anyhow::Error::msg)?;
				}
				f(db)
			})
			.await
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
