//! Native SQLite addon for RivetKit.
//!
//! Routes SQLite page-level KV operations over a WebSocket KV channel protocol.
//! This is the native Rust counterpart to the WASM implementation in `@rivetkit/sqlite-vfs`.
//!
//! The native VFS and WASM VFS must match 1:1 in behavior:
//! - KV key layout and encoding (see `kv.rs` and `sqlite-vfs/src/kv.ts`)
//! - Chunk size (4 KiB)
//! - PRAGMA settings (page_size=4096, busy_timeout=5000)
//! - VFS callback-to-KV-operation mapping
//! - Delete/truncate strategy (both use deleteRange)
//! - Journal mode

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::num::NonZeroUsize;
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex};

use libsqlite3_sys::{
	sqlite3, sqlite3_bind_blob, sqlite3_bind_double, sqlite3_bind_int64,
	sqlite3_bind_null, sqlite3_bind_text, sqlite3_changes, sqlite3_clear_bindings,
	sqlite3_column_blob, sqlite3_column_bytes, sqlite3_column_count, sqlite3_column_double,
	sqlite3_column_int64, sqlite3_column_name, sqlite3_column_text, sqlite3_column_type,
	sqlite3_errmsg, sqlite3_finalize, sqlite3_prepare_v2, sqlite3_reset, sqlite3_step,
	sqlite3_stmt, SQLITE_BLOB, SQLITE_DONE, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL,
	SQLITE_OK, SQLITE_ROW,
};
use lru::LruCache;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value as JsonValue;
use tokio::runtime::{Handle, Runtime};

/// Typed bind parameter passed from JavaScript.
///
/// Replaces `Vec<JsonValue>` for statement parameters, avoiding 20x
/// serialization overhead for blob data. Instead of JSON arrays of numbers,
/// blobs are passed as `Buffer` (a single memcpy from JS heap to Rust).
///
/// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FIXES.md M7.
#[napi(object)]
pub struct BindParam {
	/// One of: "null", "int", "float", "text", "blob"
	pub kind: String,
	pub int_value: Option<i64>,
	pub float_value: Option<f64>,
	pub text_value: Option<String>,
	pub blob_value: Option<Buffer>,
}

/// KV key layout. Mirrors `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`.
pub mod kv;

/// BARE serialization/deserialization for KV channel protocol messages.
/// Types generated from `engine/sdks/schemas/kv-channel-protocol/v1.bare`.
pub use rivet_kv_channel_protocol as protocol;

/// WebSocket KV channel client with reconnection and request correlation.
pub mod channel;

/// Custom SQLite VFS that maps VFS callbacks to KV operations via the channel.
pub mod vfs;

#[cfg(test)]
mod integration_tests;

use channel::{KvChannel, KvChannelConfig};

// MARK: Statement Cache

/// Default number of prepared statements to cache per database.
const STMT_CACHE_CAPACITY: usize = 128;

/// Wrapper around a raw `sqlite3_stmt` pointer that finalizes on drop.
/// Used as the value type in the LRU cache so evicted entries are
/// automatically cleaned up.
struct CachedStmt(*mut sqlite3_stmt);

unsafe impl Send for CachedStmt {}

impl Drop for CachedStmt {
	fn drop(&mut self) {
		if !self.0.is_null() {
			unsafe {
				sqlite3_finalize(self.0);
			}
		}
	}
}

// MARK: Runtime

/// Initialize a tracing subscriber for log output (stderr).
/// Uses RUST_LOG env var for filtering (defaults to warn). try_init()
/// is a no-op if a subscriber is already set by the host process.
fn init_tracing() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
		)
		.try_init();
}

// MARK: JS Types

/// Configuration for connecting to the KV channel endpoint.
#[napi(object)]
pub struct ConnectConfig {
	pub url: String,
	pub token: Option<String>,
	pub namespace: String,
}

/// Result of an execute() call.
#[napi(object)]
pub struct ExecuteResult {
	pub changes: i64,
}

/// Result of a query() call.
#[napi(object)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<JsonValue>>,
}

/// A shared WebSocket connection to the KV channel server.
/// One per process, shared across all actors.
///
/// The tokio runtime is owned here so it is dropped when the channel is dropped,
/// ensuring clean process exit after disconnect. The runtime MUST NOT be dropped
/// before all actors have closed their databases.
#[napi(js_name = "KvChannel")]
pub struct JsKvChannel {
	rt: Runtime,
	channel: Arc<KvChannel>,
}

/// An open SQLite database backed by KV storage via the channel.
///
/// The `db` field is wrapped in `Arc<Mutex<Option<...>>>` so that
/// `close_database` can atomically take the handle while concurrent
/// `execute`/`query`/`exec` closures hold an Arc clone. Any operation
/// that finds `None` returns a "database is closed" error. This prevents
/// use-after-free if `close_database` runs between pointer extraction
/// and `spawn_blocking` task execution.
///
/// Field order matters for drop safety: `stmt_cache` is declared before `db`
/// so cached statements are finalized before the database connection is closed.
#[napi(js_name = "NativeDatabase")]
pub struct JsNativeDatabase {
	stmt_cache: Arc<Mutex<LruCache<String, CachedStmt>>>,
	db: Arc<std::sync::Mutex<Option<vfs::NativeDatabase>>>,
	rt_handle: Handle,
	channel: Arc<KvChannel>,
	actor_id: String,
}

// MARK: Exported Functions

/// Open the shared KV channel WebSocket connection.
///
/// In production, token is the engine's admin_token (RIVET__AUTH__ADMIN_TOKEN).
/// In local dev, token is config.token (RIVET_TOKEN), optional in dev mode.
#[napi]
pub fn connect(config: ConnectConfig) -> JsKvChannel {
	init_tracing();

	let rt = Runtime::new().expect("failed to create tokio runtime");
	// Enter the runtime context so KvChannel::connect can call tokio::spawn.
	let _guard = rt.enter();
	let channel = KvChannel::connect(KvChannelConfig {
		url: config.url,
		token: config.token,
		namespace: config.namespace,
	});
	JsKvChannel {
		rt,
		channel: Arc::new(channel),
	}
}

/// Open a database for an actor. Sends ActorOpenRequest optimistically.
///
/// VFS registration and sqlite3_open_v2 run inside `spawn_blocking` because
/// they trigger synchronous VFS callbacks that call `Handle::block_on()` for
/// KV I/O. This is safe from a blocking thread but would deadlock or freeze
/// the Node.js main thread if called via `rt.block_on()`.
#[napi(js_name = "openDatabase")]
pub async fn open_database(
	channel: &JsKvChannel,
	actor_id: String,
) -> Result<JsNativeDatabase> {
	// Send ActorOpenRequest and wait for the response to ensure the
	// server-side actor lock is acquired before VFS operations begin.
	let ch = channel.channel.clone();
	let aid = actor_id.clone();
	ch.open_actor(&aid)
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?;

	// Register VFS and open database inside spawn_blocking since VFS
	// callbacks use Handle::block_on() which is safe from blocking threads
	// but not from the Node.js main thread.
	let rt_handle = channel.rt.handle().clone();
	let ch2 = channel.channel.clone();
	let aid2 = actor_id.clone();
	let rt_handle2 = rt_handle.clone();
	let native_db = channel
		.rt
		.spawn_blocking(move || {
			let vfs_name = format!("kv-{aid2}");
			let kv_vfs =
				vfs::KvVfs::register(&vfs_name, ch2, aid2.clone(), rt_handle2)?;
			vfs::open_database(kv_vfs, &aid2)
		})
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?
		.map_err(Error::from_reason)?;

	Ok(JsNativeDatabase {
		stmt_cache: Arc::new(Mutex::new(LruCache::new(
			NonZeroUsize::new(STMT_CACHE_CAPACITY).unwrap(),
		))),
		db: Arc::new(std::sync::Mutex::new(Some(native_db))),
		rt_handle,
		channel: channel.channel.clone(),
		actor_id,
	})
}

/// Execute a statement (INSERT, UPDATE, DELETE, CREATE, etc.).
///
/// SQLite operations run on tokio's blocking thread pool via `spawn_blocking`.
/// VFS callbacks call `Handle::block_on()` from blocking threads (not tokio
/// worker threads), which is safe. The Node.js main thread is never blocked.
///
/// Three threading approaches were considered:
///
/// 1. **spawn_blocking** (chosen): napi `async fn` dispatches to tokio's
///    blocking thread pool (default cap 512). Simplest, idiomatic, tokio
///    manages the pool. Minor downside: thread may change between queries
///    (slightly worse cache locality).
///
/// 2. **Dedicated thread per actor**: One `std::thread` per actor, receives
///    SQL via mpsc, sends results via oneshot. Best cache locality, but
///    requires manual lifecycle management and one idle thread per open actor.
///
/// 3. **Channel + block-in-place**: Sync napi function, VFS callbacks send
///    requests via `std::sync::mpsc` and block on `recv()`. Does NOT solve
///    the core problem because the Node.js main thread is still blocked.
///
/// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FINDINGS.md Finding 1.
#[napi]
pub async fn execute(
	db: &JsNativeDatabase,
	sql: String,
	params: Option<Vec<BindParam>>,
) -> Result<ExecuteResult> {
	let db_arc = db.db.clone();
	let cache = db.stmt_cache.clone();

	db.rt_handle
		.spawn_blocking(move || {
			let guard = db_arc.lock().unwrap();
			let native_db = guard
				.as_ref()
				.ok_or_else(|| Error::from_reason("database is closed"))?;
			let db_ptr = native_db.as_ptr();

			// Phase 1: Check cache for existing statement, then drop the lock.
			// The mutex must not be held during sqlite3_step because VFS
			// callbacks call block_on(WebSocket I/O).
			// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FIXES.md L3.
			let cached_stmt = {
				let mut cache_guard = cache.lock().unwrap();
				pop_cached_stmt(&mut cache_guard, &sql)
			};

			let stmt = if let Some(s) = cached_stmt {
				s
			} else {
				prepare_stmt(db_ptr, &sql)?
			};

			if let Some(ref p) = params {
				if let Err(e) = bind_params(db_ptr, stmt, p) {
					unsafe { sqlite3_finalize(stmt) };
					return Err(e);
				}
			}

			// Execute with no cache mutex held. VFS I/O happens here.
			let rc = unsafe { sqlite3_step(stmt) };
			if rc != SQLITE_DONE && rc != SQLITE_ROW {
				let msg = unsafe { sqlite_errmsg(db_ptr) };
				unsafe { sqlite3_finalize(stmt) };
				return Err(Error::from_reason(msg));
			}

			let changes = unsafe { sqlite3_changes(db_ptr) } as i64;

			// Phase 2: Return statement to cache.
			{
				let mut cache_guard = cache.lock().unwrap();
				cache_guard.put(sql, CachedStmt(stmt));
			}

			Ok(ExecuteResult { changes })
		})
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?
}

/// Run a query (SELECT, PRAGMA, etc.).
///
/// See `execute` for threading model documentation.
#[napi]
pub async fn query(
	db: &JsNativeDatabase,
	sql: String,
	params: Option<Vec<BindParam>>,
) -> Result<QueryResult> {
	let db_arc = db.db.clone();
	let cache = db.stmt_cache.clone();

	db.rt_handle
		.spawn_blocking(move || {
			let guard = db_arc.lock().unwrap();
			let native_db = guard
				.as_ref()
				.ok_or_else(|| Error::from_reason("database is closed"))?;
			let db_ptr = native_db.as_ptr();

			// Phase 1: Check cache for existing statement, then drop the lock.
			// The mutex must not be held during sqlite3_step because VFS
			// callbacks call block_on(WebSocket I/O).
			// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FIXES.md L3.
			let cached_stmt = {
				let mut cache_guard = cache.lock().unwrap();
				pop_cached_stmt(&mut cache_guard, &sql)
			};

			let stmt = if let Some(s) = cached_stmt {
				s
			} else {
				prepare_stmt(db_ptr, &sql)?
			};

			if let Some(ref p) = params {
				if let Err(e) = bind_params(db_ptr, stmt, p) {
					unsafe { sqlite3_finalize(stmt) };
					return Err(e);
				}
			}

			// Read column names.
			let col_count = unsafe { sqlite3_column_count(stmt) };
			let columns: Vec<String> = (0..col_count)
				.map(|i| unsafe {
					let name = sqlite3_column_name(stmt, i);
					if name.is_null() {
						String::new()
					} else {
						CStr::from_ptr(name).to_string_lossy().into_owned()
					}
				})
				.collect();

			// Read rows. No cache mutex held; VFS I/O happens during step.
			let mut rows: Vec<Vec<JsonValue>> = Vec::new();
			loop {
				let rc = unsafe { sqlite3_step(stmt) };
				if rc == SQLITE_DONE {
					break;
				}
				if rc != SQLITE_ROW {
					let msg = unsafe { sqlite_errmsg(db_ptr) };
					unsafe { sqlite3_finalize(stmt) };
					return Err(Error::from_reason(msg));
				}

				let row: Vec<JsonValue> = (0..col_count)
					.map(|i| unsafe { extract_column_value(stmt, i) })
					.collect();
				rows.push(row);
			}

			// Phase 2: Return statement to cache.
			{
				let mut cache_guard = cache.lock().unwrap();
				cache_guard.put(sql, CachedStmt(stmt));
			}

			Ok(QueryResult { columns, rows })
		})
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?
}

/// Execute multi-statement SQL without parameters.
/// Uses sqlite3_prepare_v2 in a loop with tail pointer tracking to handle
/// multiple statements (e.g., migrations). Returns columns and rows from
/// the last statement that produced results.
///
/// See `execute` for threading model documentation.
#[napi]
pub async fn exec(db: &JsNativeDatabase, sql: String) -> Result<QueryResult> {
	let db_arc = db.db.clone();

	db.rt_handle
		.spawn_blocking(move || {
			let guard = db_arc.lock().unwrap();
			let native_db = guard
				.as_ref()
				.ok_or_else(|| Error::from_reason("database is closed"))?;
			let db_ptr = native_db.as_ptr();

			let c_sql =
				CString::new(sql.as_str()).map_err(|e| Error::from_reason(e.to_string()))?;
			let sql_bytes = c_sql.to_bytes();
			let sql_ptr = c_sql.as_ptr();
			let sql_end = unsafe { sql_ptr.add(sql_bytes.len()) };

			let mut tail: *const c_char = sql_ptr;
			let mut all_rows: Vec<Vec<JsonValue>> = Vec::new();
			let mut last_columns: Vec<String> = Vec::new();

			while tail < sql_end && !tail.is_null() {
				let mut stmt: *mut sqlite3_stmt = ptr::null_mut();
				let mut next_tail: *const c_char = ptr::null();
				let remaining = (sql_end as usize - tail as usize) as c_int;

				let rc = unsafe {
					sqlite3_prepare_v2(db_ptr, tail, remaining, &mut stmt, &mut next_tail)
				};
				if rc != SQLITE_OK {
					return Err(Error::from_reason(unsafe { sqlite_errmsg(db_ptr) }));
				}

				// No more statements.
				if stmt.is_null() {
					break;
				}

				let col_count = unsafe { sqlite3_column_count(stmt) };
				if col_count > 0 {
					last_columns = (0..col_count)
						.map(|i| unsafe {
							let name = sqlite3_column_name(stmt, i);
							if name.is_null() {
								String::new()
							} else {
								CStr::from_ptr(name).to_string_lossy().into_owned()
							}
						})
						.collect();
				}

				loop {
					let rc = unsafe { sqlite3_step(stmt) };
					if rc == SQLITE_DONE {
						break;
					}
					if rc != SQLITE_ROW {
						let msg = unsafe { sqlite_errmsg(db_ptr) };
						unsafe { sqlite3_finalize(stmt) };
						return Err(Error::from_reason(msg));
					}
					let row: Vec<JsonValue> = (0..col_count)
						.map(|i| unsafe { extract_column_value(stmt, i) })
						.collect();
					all_rows.push(row);
				}

				unsafe { sqlite3_finalize(stmt) };
				tail = next_tail;
			}

			Ok(QueryResult {
				columns: last_columns,
				rows: all_rows,
			})
		})
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?
}

/// Close the database connection and release the actor lock.
/// Sends ActorCloseRequest to the server.
///
/// Locks the db mutex and takes the Option, so concurrent/subsequent
/// execute/query/exec operations see None and return "database is closed".
#[napi(js_name = "closeDatabase")]
pub async fn close_database(db: &JsNativeDatabase) -> Result<()> {
	// Finalize all cached statements before closing the database.
	db.stmt_cache.lock().unwrap().clear();

	// Lock the mutex and take the database handle. Any concurrent
	// spawn_blocking closures that haven't acquired the lock yet will
	// find None and return an error instead of using a freed pointer.
	{
		let mut guard = db.db.lock().unwrap();
		let _ = guard.take();
	}

	// Send ActorCloseRequest to release the server-side lock.
	let ch = db.channel.clone();
	let aid = db.actor_id.clone();
	ch.close_actor(&aid)
		.await
		.map_err(|e| Error::from_reason(e.to_string()))?;

	Ok(())
}

/// Close the KV channel WebSocket connection.
#[napi]
pub async fn disconnect(channel: &JsKvChannel) -> Result<()> {
	channel.channel.disconnect().await;
	Ok(())
}

// MARK: Internal Helpers

/// Pop a prepared statement from the cache if available.
///
/// Uses `pop` to remove the statement from the cache during use, so the
/// mutex can be released before `sqlite3_step` triggers VFS I/O. The
/// caller must return the statement to the cache via `put` after execution.
fn pop_cached_stmt(
	cache: &mut LruCache<String, CachedStmt>,
	sql: &str,
) -> Option<*mut sqlite3_stmt> {
	cache.pop(sql).map(|cs| {
		let stmt = cs.0;
		std::mem::forget(cs); // Prevent Drop from calling sqlite3_finalize.
		unsafe {
			sqlite3_reset(stmt);
			sqlite3_clear_bindings(stmt);
		}
		stmt
	})
}

/// Prepare a new statement via sqlite3_prepare_v2.
fn prepare_stmt(db_ptr: *mut sqlite3, sql: &str) -> Result<*mut sqlite3_stmt> {
	let c_sql = CString::new(sql).map_err(|e| Error::from_reason(e.to_string()))?;
	let mut stmt: *mut sqlite3_stmt = ptr::null_mut();
	let rc =
		unsafe { sqlite3_prepare_v2(db_ptr, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(Error::from_reason(unsafe { sqlite_errmsg(db_ptr) }));
	}
	Ok(stmt)
}

/// Get the last SQLite error message.
unsafe fn sqlite_errmsg(db: *mut sqlite3) -> String {
	let msg = sqlite3_errmsg(db);
	if msg.is_null() {
		"unknown SQLite error".into()
	} else {
		CStr::from_ptr(msg).to_string_lossy().into_owned()
	}
}

/// SQLITE_TRANSIENT tells SQLite to immediately copy bound parameter data.
fn sqlite_transient() -> Option<unsafe extern "C" fn(*mut c_void)> {
	Some(unsafe { std::mem::transmute(-1isize) })
}

/// SQLite column type constant for TEXT.
/// Defined locally because libsqlite3-sys exports vary between SQLITE3_TEXT and SQLITE_TEXT.
const SQLITE_TYPE_TEXT: c_int = 3;

/// Bind typed parameters to a prepared statement.
fn bind_params(
	db: *mut sqlite3,
	stmt: *mut sqlite3_stmt,
	params: &[BindParam],
) -> Result<()> {
	for (i, param) in params.iter().enumerate() {
		let idx = (i + 1) as c_int;
		let rc = match param.kind.as_str() {
			"null" => unsafe { sqlite3_bind_null(stmt, idx) },
			"int" => {
				let v = param.int_value.ok_or_else(|| {
					Error::from_reason(format!("missing int_value at param {idx}"))
				})?;
				unsafe { sqlite3_bind_int64(stmt, idx, v) }
			}
			"float" => {
				let v = param.float_value.ok_or_else(|| {
					Error::from_reason(format!("missing float_value at param {idx}"))
				})?;
				unsafe { sqlite3_bind_double(stmt, idx, v) }
			}
			"text" => {
				let s = param.text_value.as_ref().ok_or_else(|| {
					Error::from_reason(format!("missing text_value at param {idx}"))
				})?;
				let c_str = CString::new(s.as_str())
					.map_err(|e| Error::from_reason(e.to_string()))?;
				unsafe {
					sqlite3_bind_text(
						stmt,
						idx,
						c_str.as_ptr(),
						s.len() as c_int,
						sqlite_transient(),
					)
				}
			}
			"blob" => {
				let buf = param.blob_value.as_ref().ok_or_else(|| {
					Error::from_reason(format!("missing blob_value at param {idx}"))
				})?;
				unsafe {
					sqlite3_bind_blob(
						stmt,
						idx,
						buf.as_ptr() as *const c_void,
						buf.len() as c_int,
						sqlite_transient(),
					)
				}
			}
			other => {
				return Err(Error::from_reason(format!(
					"unsupported bind param kind '{other}' at param {idx}"
				)));
			}
		};
		if rc != SQLITE_OK {
			let msg = unsafe { sqlite_errmsg(db) };
			return Err(Error::from_reason(format!(
				"bind error at param {idx}: {msg}"
			)));
		}
	}
	Ok(())
}

/// Extract a column value from the current row as a JSON value.
unsafe fn extract_column_value(stmt: *mut sqlite3_stmt, col: c_int) -> JsonValue {
	match sqlite3_column_type(stmt, col) {
		SQLITE_NULL => JsonValue::Null,
		SQLITE_INTEGER => {
			let v = sqlite3_column_int64(stmt, col);
			JsonValue::Number(v.into())
		}
		SQLITE_FLOAT => {
			let v = sqlite3_column_double(stmt, col);
			serde_json::Number::from_f64(v)
				.map(JsonValue::Number)
				.unwrap_or(JsonValue::Null)
		}
		SQLITE_TYPE_TEXT => {
			let ptr = sqlite3_column_text(stmt, col);
			if ptr.is_null() {
				JsonValue::Null
			} else {
				let s = CStr::from_ptr(ptr as *const c_char)
					.to_string_lossy()
					.into_owned();
				JsonValue::String(s)
			}
		}
		SQLITE_BLOB => {
			let ptr = sqlite3_column_blob(stmt, col) as *const u8;
			let len = sqlite3_column_bytes(stmt, col) as usize;
			if ptr.is_null() || len == 0 {
				JsonValue::Array(vec![])
			} else {
				let bytes = slice::from_raw_parts(ptr, len);
				JsonValue::Array(
					bytes
						.iter()
						.map(|&b| JsonValue::Number(b.into()))
						.collect(),
				)
			}
		}
		_ => JsonValue::Null,
	}
}

#[cfg(test)]
mod stmt_cache_tests {
	use super::*;

	use libsqlite3_sys::{sqlite3_close, sqlite3_exec, sqlite3_open};

	fn open_memory_db() -> *mut sqlite3 {
		let mut db: *mut sqlite3 = ptr::null_mut();
		let path = CString::new(":memory:").unwrap();
		let rc = unsafe { sqlite3_open(path.as_ptr(), &mut db) };
		assert_eq!(rc, SQLITE_OK);
		db
	}

	#[test]
	fn test_stmt_cache_pop_and_put() {
		let db = open_memory_db();
		let mut cache = LruCache::new(NonZeroUsize::new(STMT_CACHE_CAPACITY).unwrap());

		// Create a table so SELECT has something to prepare against.
		unsafe {
			let sql = CString::new("CREATE TABLE cache_test (id INTEGER, value TEXT)").unwrap();
			sqlite3_exec(db, sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut());
		}

		let select_sql = "SELECT id, value FROM cache_test WHERE id = ?";

		// First lookup - cache miss, prepare manually.
		let popped = pop_cached_stmt(&mut cache, select_sql);
		assert!(popped.is_none(), "first call should not be cached");
		let stmt1 = prepare_stmt(db, select_sql).unwrap();
		cache.put(select_sql.to_string(), CachedStmt(stmt1));

		// Second lookup - cache hit via pop (removes from cache).
		let popped = pop_cached_stmt(&mut cache, select_sql);
		assert!(popped.is_some(), "second call should be cached");
		let stmt2 = popped.unwrap();
		assert_eq!(stmt1, stmt2, "cached statement pointer should match");
		// After pop, cache is empty for this key.
		assert!(pop_cached_stmt(&mut cache, select_sql).is_none());
		// Put it back.
		cache.put(select_sql.to_string(), CachedStmt(stmt2));

		// Third lookup - still cached after put.
		let popped = pop_cached_stmt(&mut cache, select_sql);
		assert!(popped.is_some(), "third call should still be cached");
		let stmt3 = popped.unwrap();
		assert_eq!(stmt1, stmt3);
		// Return to cache for cleanup.
		cache.put(select_sql.to_string(), CachedStmt(stmt3));

		// Different SQL - cache miss.
		let other_sql = "SELECT id FROM cache_test";
		let popped = pop_cached_stmt(&mut cache, other_sql);
		assert!(popped.is_none(), "different SQL should not be cached");

		cache.clear();
		unsafe { sqlite3_close(db) };
	}

	#[test]
	fn test_stmt_cache_eviction() {
		let db = open_memory_db();
		// Tiny cache of size 2 to force eviction.
		let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());

		// Fill cache with 2 statements.
		let sql1 = "SELECT 1";
		let s1 = prepare_stmt(db, sql1).unwrap();
		cache.put(sql1.to_string(), CachedStmt(s1));

		let sql2 = "SELECT 2";
		let s2 = prepare_stmt(db, sql2).unwrap();
		cache.put(sql2.to_string(), CachedStmt(s2));

		assert_eq!(cache.len(), 2);

		// Third statement evicts LRU (sql1). The evicted CachedStmt's
		// Drop impl calls sqlite3_finalize automatically.
		let sql3 = "SELECT 3";
		let s3 = prepare_stmt(db, sql3).unwrap();
		cache.put(sql3.to_string(), CachedStmt(s3));

		assert_eq!(cache.len(), 2);

		// sql1 should be evicted.
		let popped = pop_cached_stmt(&mut cache, sql1);
		assert!(popped.is_none(), "evicted statement should not be cached");
		// sql2 should still be cached.
		let popped = pop_cached_stmt(&mut cache, sql2);
		assert!(popped.is_some(), "sql2 should still be cached");
		// Return sql2 to cache for cleanup.
		cache.put(sql2.to_string(), CachedStmt(popped.unwrap()));

		cache.clear();
		unsafe { sqlite3_close(db) };
	}
}
