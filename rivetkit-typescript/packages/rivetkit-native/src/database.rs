use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use libsqlite3_sys::{
	SQLITE_BLOB, SQLITE_DONE, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW,
	SQLITE_TEXT, SQLITE_TRANSIENT, sqlite3, sqlite3_bind_blob, sqlite3_bind_double,
	sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text, sqlite3_changes, sqlite3_column_blob,
	sqlite3_column_bytes, sqlite3_column_count, sqlite3_column_double, sqlite3_column_int64,
	sqlite3_column_name, sqlite3_column_text, sqlite3_column_type, sqlite3_errmsg,
	sqlite3_finalize, sqlite3_prepare_v2, sqlite3_step,
};
use napi::bindgen_prelude::Buffer;
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

/// Native SQLite database handle exposed to JavaScript.
#[napi]
pub struct JsNativeDatabase {
	db: Arc<Mutex<Option<NativeDatabase>>>,
}

impl JsNativeDatabase {
	pub fn as_ptr(&self) -> *mut libsqlite3_sys::sqlite3 {
		self.db
			.lock()
			.ok()
			.and_then(|guard| guard.as_ref().map(NativeDatabase::as_ptr))
			.unwrap_or(ptr::null_mut())
	}

	fn take_last_kv_error_inner(&self) -> Option<String> {
		self.db
			.lock()
			.ok()
			.and_then(|guard| guard.as_ref().and_then(NativeDatabase::take_last_kv_error))
	}
}

#[napi(object)]
pub struct JsBindParam {
	pub kind: String,
	pub int_value: Option<i64>,
	pub float_value: Option<f64>,
	pub text_value: Option<String>,
	pub blob_value: Option<Buffer>,
}

#[napi(object)]
pub struct ExecuteResult {
	pub changes: i64,
}

#[napi(object)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<serde_json::Value>>,
}

#[napi]
impl JsNativeDatabase {
	#[napi]
	pub fn take_last_kv_error(&self) -> Option<String> {
		self.take_last_kv_error_inner()
	}

	#[napi]
	pub async fn run(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<ExecuteResult> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let guard = db
				.lock()
				.map_err(|_| napi::Error::from_reason("database mutex poisoned"))?;
			let native_db = guard
				.as_ref()
				.ok_or_else(|| napi::Error::from_reason("database is closed"))?;
			execute_statement(native_db.as_ptr(), &sql, params.as_deref())
		})
		.await
		.map_err(|err| napi::Error::from_reason(err.to_string()))?
	}

	#[napi]
	pub async fn query(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<QueryResult> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let guard = db
				.lock()
				.map_err(|_| napi::Error::from_reason("database mutex poisoned"))?;
			let native_db = guard
				.as_ref()
				.ok_or_else(|| napi::Error::from_reason("database is closed"))?;
			query_statement(native_db.as_ptr(), &sql, params.as_deref())
		})
		.await
		.map_err(|err| napi::Error::from_reason(err.to_string()))?
	}

	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<QueryResult> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let guard = db
				.lock()
				.map_err(|_| napi::Error::from_reason("database mutex poisoned"))?;
			let native_db = guard
				.as_ref()
				.ok_or_else(|| napi::Error::from_reason("database is closed"))?;
			exec_statements(native_db.as_ptr(), &sql)
		})
		.await
		.map_err(|err| napi::Error::from_reason(err.to_string()))?
	}

	#[napi]
	pub async fn close(&self) -> napi::Result<()> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let mut guard = db
				.lock()
				.map_err(|_| napi::Error::from_reason("database mutex poisoned"))?;
			guard.take();
			Ok(())
		})
		.await
		.map_err(|err| napi::Error::from_reason(err.to_string()))?
	}
}

fn sqlite_error(db: *mut sqlite3, context: &str) -> napi::Error {
	let message = unsafe {
		if db.is_null() {
			"unknown sqlite error".to_string()
		} else {
			CStr::from_ptr(sqlite3_errmsg(db))
				.to_string_lossy()
				.into_owned()
		}
	};
	napi::Error::from_reason(format!("{context}: {message}"))
}

fn bind_params(
	db: *mut sqlite3,
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	params: &[JsBindParam],
) -> napi::Result<()> {
	for (index, param) in params.iter().enumerate() {
		let bind_index = (index + 1) as i32;
		let rc = match param.kind.as_str() {
			"null" => unsafe { sqlite3_bind_null(stmt, bind_index) },
			"int" => unsafe {
				sqlite3_bind_int64(stmt, bind_index, param.int_value.unwrap_or_default())
			},
			"float" => unsafe {
				sqlite3_bind_double(stmt, bind_index, param.float_value.unwrap_or_default())
			},
			"text" => {
				let text = CString::new(param.text_value.clone().unwrap_or_default())
					.map_err(|err| napi::Error::from_reason(err.to_string()))?;
				unsafe {
					sqlite3_bind_text(stmt, bind_index, text.as_ptr(), -1, SQLITE_TRANSIENT())
				}
			}
			"blob" => {
				let blob = param
					.blob_value
					.as_ref()
					.map(|value| value.as_ref().to_vec())
					.unwrap_or_default();
				unsafe {
					sqlite3_bind_blob(
						stmt,
						bind_index,
						blob.as_ptr() as *const _,
						blob.len() as i32,
						SQLITE_TRANSIENT(),
					)
				}
			}
			other => {
				return Err(napi::Error::from_reason(format!(
					"unsupported bind param kind: {other}"
				)));
			}
		};

		if rc != SQLITE_OK {
			return Err(sqlite_error(db, "failed to bind sqlite parameter"));
		}
	}

	Ok(())
}

fn collect_columns(stmt: *mut libsqlite3_sys::sqlite3_stmt) -> Vec<String> {
	let column_count = unsafe { sqlite3_column_count(stmt) };
	(0..column_count)
		.map(|index| unsafe {
			let name_ptr = sqlite3_column_name(stmt, index);
			if name_ptr.is_null() {
				String::new()
			} else {
				CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
			}
		})
		.collect()
}

fn column_value(stmt: *mut libsqlite3_sys::sqlite3_stmt, index: i32) -> serde_json::Value {
	match unsafe { sqlite3_column_type(stmt, index) } {
		SQLITE_NULL => serde_json::Value::Null,
		SQLITE_INTEGER => serde_json::Value::from(unsafe { sqlite3_column_int64(stmt, index) }),
		SQLITE_FLOAT => serde_json::Value::from(unsafe { sqlite3_column_double(stmt, index) }),
		SQLITE_TEXT => {
			let text_ptr = unsafe { sqlite3_column_text(stmt, index) };
			if text_ptr.is_null() {
				serde_json::Value::Null
			} else {
				let text = unsafe { CStr::from_ptr(text_ptr as *const c_char) }
					.to_string_lossy()
					.into_owned();
				serde_json::Value::String(text)
			}
		}
		SQLITE_BLOB => {
			let blob_ptr = unsafe { sqlite3_column_blob(stmt, index) };
			if blob_ptr.is_null() {
				serde_json::Value::Null
			} else {
				let blob_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				let blob = unsafe { std::slice::from_raw_parts(blob_ptr as *const u8, blob_len) };
				serde_json::Value::Array(
					blob.iter()
						.map(|byte| serde_json::Value::from(*byte))
						.collect(),
				)
			}
		}
		_ => serde_json::Value::Null,
	}
}

fn execute_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[JsBindParam]>,
) -> napi::Result<ExecuteResult> {
	let c_sql = CString::new(sql).map_err(|err| napi::Error::from_reason(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to prepare sqlite statement"));
	}
	if stmt.is_null() {
		return Ok(ExecuteResult { changes: 0 });
	}

	let result = (|| {
		if let Some(params) = params {
			bind_params(db, stmt, params)?;
		}

		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(db, "failed to execute sqlite statement"));
			}
		}

		Ok(ExecuteResult {
			changes: unsafe { sqlite3_changes(db) as i64 },
		})
	})();

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

fn query_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[JsBindParam]>,
) -> napi::Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| napi::Error::from_reason(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to prepare sqlite query"));
	}
	if stmt.is_null() {
		return Ok(QueryResult {
			columns: Vec::new(),
			rows: Vec::new(),
		});
	}

	let result = (|| {
		if let Some(params) = params {
			bind_params(db, stmt, params)?;
		}

		let columns = collect_columns(stmt);
		let mut rows = Vec::new();

		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(db, "failed to step sqlite query"));
			}

			let mut row = Vec::with_capacity(columns.len());
			for index in 0..columns.len() {
				row.push(column_value(stmt, index as i32));
			}
			rows.push(row);
		}

		Ok(QueryResult { columns, rows })
	})();

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

fn exec_statements(db: *mut sqlite3, sql: &str) -> napi::Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| napi::Error::from_reason(err.to_string()))?;
	let mut remaining = c_sql.as_ptr();
	let mut final_result = QueryResult {
		columns: Vec::new(),
		rows: Vec::new(),
	};

	while unsafe { *remaining } != 0 {
		let mut stmt = ptr::null_mut();
		let mut tail = ptr::null();
		let rc = unsafe { sqlite3_prepare_v2(db, remaining, -1, &mut stmt, &mut tail) };
		if rc != SQLITE_OK {
			return Err(sqlite_error(db, "failed to prepare sqlite exec statement"));
		}

		if stmt.is_null() {
			if tail == remaining {
				break;
			}
			remaining = tail;
			continue;
		}

		let columns = collect_columns(stmt);
		let mut rows = Vec::new();
		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				unsafe {
					sqlite3_finalize(stmt);
				}
				return Err(sqlite_error(db, "failed to step sqlite exec statement"));
			}

			let mut row = Vec::with_capacity(columns.len());
			for index in 0..columns.len() {
				row.push(column_value(stmt, index as i32));
			}
			rows.push(row);
		}

		unsafe {
			sqlite3_finalize(stmt);
		}

		if !columns.is_empty() || !rows.is_empty() {
			final_result = QueryResult { columns, rows };
		}

		if tail == remaining {
			break;
		}
		remaining = tail;
	}

	Ok(final_result)
}

/// Open a native SQLite database backed by the envoy's KV channel.
#[napi]
pub async fn open_database_from_envoy(
	js_handle: &JsEnvoyHandle,
	actor_id: String,
) -> napi::Result<JsNativeDatabase> {
	let envoy_kv = Arc::new(EnvoyKv::new(js_handle.handle.clone(), actor_id.clone()));
	let rt_handle = Handle::current();
	let db = tokio::task::spawn_blocking(move || {
		let vfs_name = format!("envoy-kv-{}", actor_id);
		let vfs = KvVfs::register(&vfs_name, envoy_kv, actor_id.clone(), rt_handle)
			.map_err(|e| napi::Error::from_reason(format!("failed to register VFS: {}", e)))?;

		rivetkit_sqlite_native::vfs::open_database(vfs, &actor_id)
			.map_err(|e| napi::Error::from_reason(format!("failed to open database: {}", e)))
	})
	.await
	.map_err(|err| napi::Error::from_reason(err.to_string()))??;

	Ok(JsNativeDatabase {
		db: Arc::new(Mutex::new(Some(db))),
	})
}
