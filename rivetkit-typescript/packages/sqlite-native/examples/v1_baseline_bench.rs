use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use libsqlite3_sys::*;
use rivetkit_sqlite_native::sqlite_kv::{KvGetResult, SqliteKv, SqliteKvError};
use rivetkit_sqlite_native::vfs::{open_database, KvVfs};

const PAGE_SIZE_BYTES: usize = 4096;

#[derive(Clone, Copy, Default)]
struct OpTotals {
	get: u64,
	put: u64,
	delete: u64,
	delete_range: u64,
}

impl OpTotals {
	fn round_trips(self) -> u64 {
		self.get + self.put + self.delete + self.delete_range
	}
}

#[derive(Default)]
struct MemoryKv {
	stores: Mutex<HashMap<String, HashMap<Vec<u8>, Vec<u8>>>>,
	op_totals: Mutex<HashMap<String, OpTotals>>,
}

impl MemoryKv {
	fn record_get(&self, actor_id: &str) {
		let mut totals = self.op_totals.lock().unwrap();
		totals.entry(actor_id.to_string()).or_default().get += 1;
	}

	fn record_put(&self, actor_id: &str) {
		let mut totals = self.op_totals.lock().unwrap();
		totals.entry(actor_id.to_string()).or_default().put += 1;
	}

	fn record_delete(&self, actor_id: &str) {
		let mut totals = self.op_totals.lock().unwrap();
		totals.entry(actor_id.to_string()).or_default().delete += 1;
	}

	fn record_delete_range(&self, actor_id: &str) {
		let mut totals = self.op_totals.lock().unwrap();
		totals.entry(actor_id.to_string()).or_default().delete_range += 1;
	}

	fn totals_for(&self, actor_id: &str) -> OpTotals {
		self.op_totals
			.lock()
			.unwrap()
			.get(actor_id)
			.copied()
			.unwrap_or_default()
	}
}

#[async_trait]
impl SqliteKv for MemoryKv {
	async fn batch_get(
		&self,
		actor_id: &str,
		keys: Vec<Vec<u8>>,
	) -> Result<KvGetResult, SqliteKvError> {
		self.record_get(actor_id);

		let store_guard = self.stores.lock().unwrap();
		let actor_store = store_guard.get(actor_id);
		let mut found_keys = Vec::new();
		let mut found_values = Vec::new();

		for key in keys {
			if let Some(value) = actor_store.and_then(|store| store.get(&key)) {
				found_keys.push(key);
				found_values.push(value.clone());
			}
		}

		Ok(KvGetResult {
			keys: found_keys,
			values: found_values,
		})
	}

	async fn batch_put(
		&self,
		actor_id: &str,
		keys: Vec<Vec<u8>>,
		values: Vec<Vec<u8>>,
	) -> Result<(), SqliteKvError> {
		if keys.len() != values.len() {
			return Err(SqliteKvError::new("keys and values length mismatch"));
		}

		self.record_put(actor_id);

		let mut stores = self.stores.lock().unwrap();
		let actor_store = stores.entry(actor_id.to_string()).or_default();
		for (key, value) in keys.into_iter().zip(values.into_iter()) {
			actor_store.insert(key, value);
		}

		Ok(())
	}

	async fn batch_delete(&self, actor_id: &str, keys: Vec<Vec<u8>>) -> Result<(), SqliteKvError> {
		self.record_delete(actor_id);

		let mut stores = self.stores.lock().unwrap();
		let actor_store = stores.entry(actor_id.to_string()).or_default();
		for key in keys {
			actor_store.remove(&key);
		}

		Ok(())
	}

	async fn delete_range(
		&self,
		actor_id: &str,
		start: Vec<u8>,
		end: Vec<u8>,
	) -> Result<(), SqliteKvError> {
		self.record_delete_range(actor_id);

		let mut stores = self.stores.lock().unwrap();
		let actor_store = stores.entry(actor_id.to_string()).or_default();
		actor_store.retain(|key, _| {
			!(key.as_slice() >= start.as_slice() && key.as_slice() < end.as_slice())
		});

		Ok(())
	}
}

#[derive(Clone, Copy)]
struct WorkloadResult {
	latency_ms: f64,
	round_trips: u64,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_name(prefix: &str) -> String {
	let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
	format!("{prefix}-{id}")
}

fn with_database<T>(
	kv: Arc<MemoryKv>,
	actor_id: &str,
	callback: impl FnOnce(*mut sqlite3) -> T,
) -> T {
	let runtime = tokio::runtime::Builder::new_current_thread()
		.build()
		.unwrap();
	let vfs_name = next_name("sqlite-native-bench-vfs");
	let vfs = KvVfs::register(
		&vfs_name,
		kv,
		actor_id.to_string(),
		runtime.handle().clone(),
		Vec::new(),
	)
	.unwrap();
	let db = open_database(vfs, actor_id).unwrap();
	let output = callback(db.as_ptr());
	drop(db);
	drop(runtime);
	output
}

fn exec_sql(db: *mut sqlite3, sql: &str) {
	let c_sql = CString::new(sql).unwrap();
	let mut err_msg = ptr::null_mut();
	let rc = unsafe { sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), &mut err_msg) };
	if rc != SQLITE_OK {
		let message = if err_msg.is_null() {
			format!("sqlite error {rc}")
		} else {
			let message = unsafe { CStr::from_ptr(err_msg) }
				.to_string_lossy()
				.into_owned();
			unsafe {
				sqlite3_free(err_msg as *mut c_void);
			}
			message
		};
		panic!("sqlite3_exec failed for `{sql}`: {message}");
	}
}

fn prepare_statement(db: *mut sqlite3, sql: &str) -> *mut sqlite3_stmt {
	let c_sql = CString::new(sql).unwrap();
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	assert_eq!(rc, SQLITE_OK, "failed to prepare `{sql}`");
	assert!(
		!stmt.is_null(),
		"sqlite returned null statement for `{sql}`"
	);
	stmt
}

fn finalize_statement(stmt: *mut sqlite3_stmt) {
	let rc = unsafe { sqlite3_finalize(stmt) };
	assert_eq!(rc, SQLITE_OK, "failed to finalize statement");
}

fn insert_blob(db: *mut sqlite3, payload: &[u8]) {
	let stmt = prepare_statement(db, "INSERT INTO payloads (body) VALUES (?1);");
	let bind_rc = unsafe {
		sqlite3_bind_blob(
			stmt,
			1,
			payload.as_ptr() as *const c_void,
			payload.len() as i32,
			SQLITE_TRANSIENT(),
		)
	};
	assert_eq!(bind_rc, SQLITE_OK, "failed to bind blob payload");

	let step_rc = unsafe { sqlite3_step(stmt) };
	assert_eq!(step_rc, SQLITE_DONE, "failed to insert blob payload");
	finalize_statement(stmt);
}

fn insert_page_rows(db: *mut sqlite3, rows: usize) {
	let payload = vec![0x5au8; PAGE_SIZE_BYTES];
	let stmt = prepare_statement(db, "INSERT INTO payloads (body) VALUES (?1);");

	for _ in 0..rows {
		let clear_rc = unsafe { sqlite3_clear_bindings(stmt) };
		assert_eq!(clear_rc, SQLITE_OK, "failed to clear bindings");

		let reset_rc = unsafe { sqlite3_reset(stmt) };
		assert_eq!(reset_rc, SQLITE_OK, "failed to reset statement");

		let bind_rc = unsafe {
			sqlite3_bind_blob(
				stmt,
				1,
				payload.as_ptr() as *const c_void,
				payload.len() as i32,
				SQLITE_TRANSIENT(),
			)
		};
		assert_eq!(bind_rc, SQLITE_OK, "failed to bind page payload");

		let step_rc = unsafe { sqlite3_step(stmt) };
		assert_eq!(step_rc, SQLITE_DONE, "failed to insert page payload");
	}

	finalize_statement(stmt);
}

fn select_page_rows(db: *mut sqlite3) {
	let stmt = prepare_statement(db, "SELECT body FROM payloads ORDER BY id;");
	let mut rows = 0usize;

	loop {
		let step_rc = unsafe { sqlite3_step(stmt) };
		if step_rc == SQLITE_DONE {
			break;
		}
		assert_eq!(step_rc, SQLITE_ROW, "expected row while reading payloads");
		let bytes = unsafe { sqlite3_column_bytes(stmt, 0) } as usize;
		assert_eq!(bytes, PAGE_SIZE_BYTES, "expected one page per payload row");
		rows += 1;
	}

	assert_eq!(rows, 100, "expected to read 100 payload rows");
	finalize_statement(stmt);
}

fn run_workload(name: &str, callback: impl FnOnce(Arc<MemoryKv>, &str) -> ()) -> WorkloadResult {
	let actor_id = next_name("sqlite-native-bench-actor");
	let kv = Arc::new(MemoryKv::default());
	let started_at = Instant::now();
	callback(kv.clone(), &actor_id);
	let elapsed = started_at.elapsed();
	let totals = kv.totals_for(&actor_id);

	let result = WorkloadResult {
		latency_ms: elapsed.as_secs_f64() * 1000.0,
		round_trips: totals.round_trips(),
	};

	println!(
		"RESULT\t{name}\t{:.3}\t{}",
		result.latency_ms, result.round_trips
	);
	result
}

fn workload_one_mib_insert() -> WorkloadResult {
	run_workload("1 MiB insert", |kv, actor_id| {
		with_database(kv, actor_id, |db| {
			exec_sql(
				db,
				"CREATE TABLE payloads (id INTEGER PRIMARY KEY, body BLOB NOT NULL);",
			);
			let payload = vec![0x11u8; 1024 * 1024];
			insert_blob(db, &payload);
		});
	})
}

fn workload_ten_mib_insert() -> WorkloadResult {
	run_workload("10 MiB insert", |kv, actor_id| {
		with_database(kv, actor_id, |db| {
			exec_sql(
				db,
				"CREATE TABLE payloads (id INTEGER PRIMARY KEY, body BLOB NOT NULL);",
			);
			let payload = vec![0x22u8; 10 * 1024 * 1024];
			insert_blob(db, &payload);
		});
	})
}

fn workload_hot_row_update() -> WorkloadResult {
	run_workload("hot-row update", |kv, actor_id| {
		with_database(kv, actor_id, |db| {
			exec_sql(
				db,
				"CREATE TABLE counters (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
			);
			exec_sql(db, "INSERT INTO counters (id, value) VALUES (1, 0);");
			for _ in 0..100 {
				exec_sql(db, "UPDATE counters SET value = value + 1 WHERE id = 1;");
			}
		});
	})
}

fn workload_cold_read() -> WorkloadResult {
	run_workload("cold read", |kv, actor_id| {
		with_database(kv.clone(), actor_id, |db| {
			exec_sql(
				db,
				"CREATE TABLE payloads (id INTEGER PRIMARY KEY, body BLOB NOT NULL);",
			);
			insert_page_rows(db, 100);
		});

		with_database(kv, actor_id, |db| {
			select_page_rows(db);
		});
	})
}

fn workload_mixed_read_write() -> WorkloadResult {
	run_workload("mixed read/write", |kv, actor_id| {
		with_database(kv, actor_id, |db| {
			exec_sql(
				db,
				"CREATE TABLE items (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
			);
			exec_sql(
				db,
				"INSERT INTO items (id, value) VALUES
					(1, 10), (2, 20), (3, 30), (4, 40), (5, 50);",
			);
			for _ in 0..25 {
				exec_sql(db, "SELECT value FROM items WHERE id = 3;");
				exec_sql(db, "UPDATE items SET value = value + 1 WHERE id = 3;");
				exec_sql(db, "INSERT INTO items (value) VALUES (99);");
				exec_sql(
					db,
					"DELETE FROM items WHERE id = (SELECT MIN(id) FROM items);",
				);
			}
		});
	})
}

fn main() {
	let results = [
		workload_one_mib_insert(),
		workload_ten_mib_insert(),
		workload_hot_row_update(),
		workload_cold_read(),
		workload_mixed_read_write(),
	];

	println!(
		"SUMMARY\tpage_size_bytes={}\tworkloads={}",
		PAGE_SIZE_BYTES,
		results.len()
	);
}
