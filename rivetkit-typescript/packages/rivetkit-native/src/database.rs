use std::sync::Arc;

use async_trait::async_trait;
use napi_derive::napi;
use rivet_envoy_client::handle::EnvoyHandle;
use rivetkit_sqlite_native::sqlite_kv::{KvGetResult, SqliteKv, SqliteKvError};
use rivetkit_sqlite_native::vfs::{KvVfs, NativeDatabase};
use tokio::runtime::Handle;

use crate::envoy_handle::JsEnvoyHandle;

/// SqliteKv adapter that routes operations through the envoy handle's KV methods.
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

/// Native SQLite database handle exposed to JavaScript.
#[napi]
pub struct JsNativeDatabase {
	db: NativeDatabase,
}

impl JsNativeDatabase {
	pub fn as_ptr(&self) -> *mut libsqlite3_sys::sqlite3 {
		self.db.as_ptr()
	}
}

/// Open a native SQLite database backed by the envoy's KV channel.
#[napi]
pub async fn open_database_from_envoy(
	js_handle: &JsEnvoyHandle,
	actor_id: String,
) -> napi::Result<JsNativeDatabase> {
	let envoy_kv = Arc::new(EnvoyKv::new(js_handle.handle.clone(), actor_id.clone()));

	let rt_handle = Handle::current();
	let vfs_name = format!("envoy-kv-{}", actor_id);

	let vfs = KvVfs::register(&vfs_name, envoy_kv, actor_id.clone(), rt_handle)
		.map_err(|e| napi::Error::from_reason(format!("failed to register VFS: {}", e)))?;

	let db = rivetkit_sqlite_native::vfs::open_database(vfs, &actor_id)
		.map_err(|e| napi::Error::from_reason(format!("failed to open database: {}", e)))?;

	Ok(JsNativeDatabase { db })
}
