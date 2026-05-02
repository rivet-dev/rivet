use std::sync::Arc;

use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;

use crate::{
	connection_manager::{NativeConnectionManager, NativeConnectionManagerConfig},
	optimization_flags::sqlite_optimization_flags,
	query::{
		BindParam, ExecResult, ExecuteResult, ExecuteRoute, QueryResult, classify_statement,
		exec_statements, execute_single_statement, install_reader_authorizer,
	},
	vfs::{
		NativeVfsHandle, SqliteVfs, SqliteVfsMetrics, VfsConfig, VfsPreloadHintSnapshot,
		configure_connection_for_database, verify_batch_atomic_writes,
	},
};

enum ReadQueryRoute {
	Read(ExecuteResult),
	WriteRequired(ExecuteRoute),
}

#[derive(Clone)]
pub struct NativeDatabaseHandle {
	file_name: String,
	vfs: NativeVfsHandle,
	manager: NativeConnectionManager,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
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
	let vfs = Arc::new(SqliteVfs::register(
		&vfs_name,
		handle,
		actor_id.clone(),
		rt_handle,
		VfsConfig::default(),
		metrics.clone(),
	)
	.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?);

	let native_db = NativeDatabaseHandle::new_with_metrics(
		vfs,
		actor_id,
		NativeConnectionManagerConfig::from_optimization_flags(*sqlite_optimization_flags()),
		metrics,
	);
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
	let vfs = Arc::new(
		SqliteVfs::register_with_transport(
			&vfs_name,
			crate::vfs::SqliteTransport::from_conveyer(db),
			actor_id.clone(),
			rt_handle,
			VfsConfig::default(),
			metrics.clone(),
		)
		.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?,
	);

	let native_db = NativeDatabaseHandle::new_with_metrics(
		vfs,
		actor_id,
		NativeConnectionManagerConfig::from_optimization_flags(*sqlite_optimization_flags()),
		metrics,
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
		Self::new_with_metrics(vfs, file_name, config, None)
	}

	pub fn new_with_metrics(
		vfs: NativeVfsHandle,
		file_name: String,
		config: NativeConnectionManagerConfig,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Self {
		Self {
			file_name: file_name.clone(),
			manager: NativeConnectionManager::new_with_metrics(
				vfs.clone(),
				file_name,
				config,
				metrics.clone(),
			),
			vfs,
			metrics,
		}
	}

	pub async fn exec(&self, sql: String) -> Result<QueryResult> {
		self.with_configured_write_connection(move |db| exec_statements(db, &sql))
			.await
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
		if !self.manager.read_pool_enabled() {
			return self.execute_without_read_pool(sql, params).await;
		}
		if self.manager.write_mode_active().await {
			return self.execute_on_writer_with_classification(sql, params).await;
		}

		let read_sql = sql.clone();
		let read_params = params.clone();
		let route = match self.try_read_execute(read_sql, read_params).await? {
			ReadQueryRoute::Read(result) => {
				if let Some(metrics) = &self.metrics {
					metrics.record_read_pool_routed_read_query();
				}
				return Ok(result);
			}
			ReadQueryRoute::WriteRequired(route) => route,
		};
		if matches!(route, ExecuteRoute::WriteFallback) {
			if let Some(metrics) = &self.metrics {
				metrics.record_read_pool_write_fallback_query();
			}
		}

		self.with_configured_write_connection(move |db| {
			execute_single_statement(db, &sql, params.as_deref(), route)
		})
		.await
	}

	pub async fn execute_write(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.with_configured_write_connection(move |db| {
			execute_single_statement(db, &sql, params.as_deref(), ExecuteRoute::Write)
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

	#[cfg(test)]
	pub(crate) fn manager(&self) -> NativeConnectionManager {
		self.manager.clone()
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

	async fn execute_without_read_pool(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.execute_on_writer_with_classification(sql, params).await
	}

	async fn execute_on_writer_with_classification(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		let metrics = self.metrics.clone();
		self.with_configured_write_connection(move |db| {
			let route = classify_statement(db, &sql)
				.map(|classification| write_route_for_classification(&classification))
				.unwrap_or(ExecuteRoute::WriteFallback);
			if matches!(route, ExecuteRoute::WriteFallback) {
				if let Some(metrics) = &metrics {
					metrics.record_read_pool_write_fallback_query();
				}
			}
			execute_single_statement(db, &sql, params.as_deref(), route)
		})
		.await
	}

	async fn try_read_execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ReadQueryRoute> {
		let metrics = self.metrics.clone();
		self.manager
			.with_read_connection_state(move |db, newly_opened| {
				if newly_opened {
					configure_reader_connection(db)?;
				}

				let classification = match classify_statement(db, &sql) {
					Ok(classification) => classification,
					Err(_) => {
						return Ok(ReadQueryRoute::WriteRequired(ExecuteRoute::WriteFallback));
					}
				};
				if !classification.reader_eligible() {
					return Ok(ReadQueryRoute::WriteRequired(write_route_for_classification(
						&classification,
					)));
				}

				install_reader_authorizer(db)?;
				match execute_single_statement(db, &sql, params.as_deref(), ExecuteRoute::Read) {
					Ok(result) => Ok(ReadQueryRoute::Read(result)),
					Err(error) => {
						if reader_rejection_error(&error) {
							if let Some(metrics) = &metrics {
								metrics.record_read_pool_rejected_reader_mutation();
							}
							return Err(error);
						}
						Err(error)
					}
				}
			})
			.await
	}
}

fn reader_rejection_error(error: &anyhow::Error) -> bool {
	let message = error.to_string().to_ascii_lowercase();
	message.contains("not authorized")
		|| message.contains("readonly")
		|| message.contains("read-only")
		|| message.contains("attempt to write")
}

fn write_route_for_classification(
	classification: &crate::query::StatementClassification,
) -> ExecuteRoute {
	if !classification.sqlite_readonly || classification.authorizer.requires_write_route() {
		ExecuteRoute::Write
	} else {
		ExecuteRoute::WriteFallback
	}
}

fn configure_reader_connection(db: *mut libsqlite3_sys::sqlite3) -> Result<()> {
	exec_statements(db, "PRAGMA query_only = ON;")?;
	Ok(())
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
