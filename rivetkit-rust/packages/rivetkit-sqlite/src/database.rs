use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;

use crate::sqlite_kv::{KvGetResult, SqliteKv, SqliteKvError};
use crate::v2::vfs::{NativeDatabaseV2, SqliteVfsMetricsSnapshot, SqliteVfsV2, VfsV2Config};
use crate::vfs::{KvVfs, NativeDatabase};

pub struct EnvoyKv {
	handle: EnvoyHandle,
	actor_id: String,
}

impl EnvoyKv {
	pub fn new(handle: EnvoyHandle, actor_id: String) -> Self {
		Self { handle, actor_id }
	}
}

#[async_trait]
impl SqliteKv for EnvoyKv {
	fn on_error(&self, actor_id: &str, error: &SqliteKvError) {
		tracing::error!(%actor_id, %error, "native sqlite kv operation failed");
	}

	async fn on_open(&self, _actor_id: &str) -> Result<(), SqliteKvError> {
		Ok(())
	}

	async fn on_close(&self, _actor_id: &str) -> Result<(), SqliteKvError> {
		Ok(())
	}

	async fn batch_get(
		&self,
		_actor_id: &str,
		keys: Vec<Vec<u8>>,
	) -> Result<KvGetResult, SqliteKvError> {
		let result = self
			.handle
			.kv_get(self.actor_id.clone(), keys.clone())
			.await
			.map_err(|e| SqliteKvError::new(e.to_string()))?;

		let mut out_keys = Vec::new();
		let mut out_values = Vec::new();
		for (i, val) in result.into_iter().enumerate() {
			if let Some(v) = val {
				out_keys.push(keys[i].clone());
				out_values.push(v);
			}
		}

		Ok(KvGetResult {
			keys: out_keys,
			values: out_values,
		})
	}

	async fn batch_put(
		&self,
		_actor_id: &str,
		keys: Vec<Vec<u8>>,
		values: Vec<Vec<u8>>,
	) -> Result<(), SqliteKvError> {
		let entries: Vec<(Vec<u8>, Vec<u8>)> = keys.into_iter().zip(values).collect();
		self.handle
			.kv_put(self.actor_id.clone(), entries)
			.await
			.map_err(|e| SqliteKvError::new(e.to_string()))
	}

	async fn batch_delete(&self, _actor_id: &str, keys: Vec<Vec<u8>>) -> Result<(), SqliteKvError> {
		self.handle
			.kv_delete(self.actor_id.clone(), keys)
			.await
			.map_err(|e| SqliteKvError::new(e.to_string()))
	}

	async fn delete_range(
		&self,
		_actor_id: &str,
		start: Vec<u8>,
		end: Vec<u8>,
	) -> Result<(), SqliteKvError> {
		self.handle
			.kv_delete_range(self.actor_id.clone(), start, end)
			.await
			.map_err(|e| SqliteKvError::new(e.to_string()))
	}
}

pub enum NativeDatabaseHandle {
	V1(NativeDatabase),
	V2(NativeDatabaseV2),
}

impl NativeDatabaseHandle {
	pub fn as_ptr(&self) -> *mut libsqlite3_sys::sqlite3 {
		match self {
			Self::V1(db) => db.as_ptr(),
			Self::V2(db) => db.as_ptr(),
		}
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		match self {
			Self::V1(db) => db.take_last_kv_error(),
			Self::V2(db) => db.take_last_kv_error(),
		}
	}

	pub fn sqlite_vfs_metrics(&self) -> Option<SqliteVfsMetricsSnapshot> {
		match self {
			Self::V1(_) => None,
			Self::V2(db) => Some(db.sqlite_vfs_metrics()),
		}
	}
}

pub fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	schema_version: u32,
	startup_data: Option<protocol::SqliteStartupData>,
	preloaded_entries: Vec<(Vec<u8>, Vec<u8>)>,
	rt_handle: Handle,
) -> Result<NativeDatabaseHandle> {
	match schema_version {
		1 => {
			let vfs_name = format!("envoy-kv-{actor_id}");
			let envoy_kv = Arc::new(EnvoyKv::new(handle, actor_id.clone()));
			let vfs = KvVfs::register(
				&vfs_name,
				envoy_kv,
				actor_id.clone(),
				rt_handle,
				preloaded_entries,
			)
			.map_err(|e| anyhow!("failed to register VFS: {e}"))?;

			crate::vfs::open_database(vfs, &actor_id)
				.map(NativeDatabaseHandle::V1)
				.map_err(|e| anyhow!("failed to open database: {e}"))
		}
		2 => {
			let startup = startup_data.ok_or_else(|| {
				anyhow!("missing sqlite startup data for actor {actor_id} using schema version 2")
			})?;
			let vfs_name = format!("envoy-sqlite-v2-{actor_id}");
			let vfs = SqliteVfsV2::register(
				&vfs_name,
				handle,
				actor_id.clone(),
				rt_handle,
				startup,
				VfsV2Config::default(),
			)
			.map_err(|e| anyhow!("failed to register V2 VFS: {e}"))?;

			crate::v2::vfs::open_database(vfs, &actor_id)
				.map(NativeDatabaseHandle::V2)
				.map_err(|e| anyhow!("failed to open V2 database: {e}"))
		}
		version => Err(anyhow!(
			"unsupported sqlite schema version {version} for actor {actor_id}"
		)),
	}
}
