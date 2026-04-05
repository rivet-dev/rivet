//! Integration tests for the native VFS with a mock WebSocket KV server.
//!
//! These tests exercise the full VFS pipeline through SQLite operations,
//! verifying chunk mapping, boundary handling, metadata persistence, and
//! channel reconnection. They use a mock WebSocket server with an in-memory
//! KV store that implements the KV channel protocol.
//!
//! End-to-end tests (Layer 2) are in the driver test suite:
//! `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`

use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use libsqlite3_sys::*;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::sync::{broadcast, mpsc, Mutex, Semaphore};
use tokio_tungstenite::{
	accept_hdr_async,
	tungstenite::{
		handshake::server::{Request, Response},
		Message,
	},
};

use crate::channel::{ChannelError, KvChannel, KvChannelConfig};
use crate::kv;
use crate::protocol::*;
use crate::vfs;
use crate::vfs::decode_file_meta;

// MARK: VFS Name Counter

static VFS_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_vfs_name(actor_id: &str) -> String {
	let id = VFS_COUNTER.fetch_add(1, Ordering::Relaxed);
	format!("test-vfs-{actor_id}-{id}")
}

// MARK: Mock KV Server

/// Operation recorded by the mock server for test verification.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum MockOp {
	Open { actor_id: String },
	Close { actor_id: String },
	Get { actor_id: String, keys: Vec<Vec<u8>> },
	Put { actor_id: String, keys: Vec<Vec<u8>> },
	Delete { actor_id: String, keys: Vec<Vec<u8>> },
	DeleteRange { actor_id: String, start: Vec<u8>, end: Vec<u8> },
}

struct MockState {
	/// Per-actor KV stores. BTreeMap for ordered range operations.
	stores: Mutex<HashMap<String, BTreeMap<Vec<u8>, Vec<u8>>>>,
	/// Single-writer locks: actor_id -> connection_id.
	locks: Mutex<HashMap<String, u64>>,
	/// Recorded operations for test assertions.
	ops: Mutex<Vec<MockOp>>,
	/// Connection ID counter.
	next_conn_id: AtomicU64,
	/// Broadcast to force-close all connections (for reconnection testing).
	kill_tx: broadcast::Sender<()>,
	/// Semaphore gate for ActorOpenResponse. When set with 0 permits,
	/// open responses block until permits are added.
	open_gate: Mutex<Option<Arc<Semaphore>>>,
}

struct MockKvServer {
	port: u16,
	state: Arc<MockState>,
}

impl MockKvServer {
	async fn start() -> Self {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let port = listener.local_addr().unwrap().port();
		let (kill_tx, _) = broadcast::channel::<()>(16);
		let state = Arc::new(MockState {
			stores: Mutex::new(HashMap::new()),
			locks: Mutex::new(HashMap::new()),
			ops: Mutex::new(Vec::new()),
			next_conn_id: AtomicU64::new(1),
			kill_tx,
			open_gate: Mutex::new(None),
		});

		let state_clone = state.clone();
		tokio::spawn(async move {
			mock_accept_loop(listener, state_clone).await;
		});

		MockKvServer { port, state }
	}

	fn url(&self) -> String {
		format!("ws://127.0.0.1:{}", self.port)
	}

	async fn get_store(&self, actor_id: &str) -> BTreeMap<Vec<u8>, Vec<u8>> {
		self.state
			.stores
			.lock()
			.await
			.get(actor_id)
			.cloned()
			.unwrap_or_default()
	}

	async fn ops(&self) -> Vec<MockOp> {
		self.state.ops.lock().await.clone()
	}

	async fn reset_ops(&self) {
		self.state.ops.lock().await.clear();
	}

	async fn close_all_connections(&self) {
		let _ = self.state.kill_tx.send(());
	}
}

async fn mock_accept_loop(listener: TcpListener, state: Arc<MockState>) {
	loop {
		match listener.accept().await {
			Ok((stream, _)) => {
				let conn_id = state.next_conn_id.fetch_add(1, Ordering::Relaxed);
				let state = state.clone();
				let mut kill_rx = state.kill_tx.subscribe();
				tokio::spawn(async move {
					let ws = match accept_hdr_async(stream, |req: &Request, mut response: Response| {
						if let Some(protocols) = req.headers().get("Sec-WebSocket-Protocol") {
							if protocols
								.to_str()
								.ok()
								.into_iter()
								.flat_map(|value| value.split(','))
								.any(|value| value.trim() == "rivet")
							{
								response
									.headers_mut()
									.insert("Sec-WebSocket-Protocol", "rivet".parse().unwrap());
							}
						}
						Ok(response)
					})
					.await
					{
						Ok(ws) => ws,
						Err(_) => return,
					};
					let (write, mut read) = ws.split();
					let open_actors: Arc<Mutex<Vec<String>>> =
						Arc::new(Mutex::new(Vec::new()));

					// Write responses via mpsc channel so spawned tasks can send.
					let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<Vec<u8>>();
					let write_handle = tokio::spawn(async move {
						let mut write = write;
						while let Some(bytes) = resp_rx.recv().await {
							if write
								.send(Message::Binary(bytes.into()))
								.await
								.is_err()
							{
								break;
							}
						}
					});

					loop {
						tokio::select! {
							msg = read.next() => {
								match msg {
									Some(Ok(Message::Binary(data))) => {
										if let Ok(ToRivet::ToRivetRequest(req)) = decode_to_server(&data) {
											let state = state.clone();
											let resp_tx = resp_tx.clone();
											let open_actors = open_actors.clone();
											let actor_id = req.actor_id.clone();
											let request_id = req.request_id;
											let data = req.data;
											tokio::spawn(async move {
												let resp_data = mock_handle_request(
													&state, conn_id, &actor_id, data, &open_actors,
												).await;
												let resp = ToClient::ToClientResponse(ToClientResponse {
													request_id,
													data: resp_data,
												});
												if let Ok(bytes) = encode_to_client(&resp) {
													let _ = resp_tx.send(bytes);
												}
											});
										}
									}
									Some(Ok(_)) => {}
									_ => break,
								}
							}
							_ = kill_rx.recv() => break,
						}
					}

					// Release all locks held by this connection.
					let oa = open_actors.lock().await;
					let mut locks = state.locks.lock().await;
					for actor_id in oa.iter() {
						if locks.get(actor_id) == Some(&conn_id) {
							locks.remove(actor_id);
						}
					}

					// Clean up writer task.
					drop(resp_tx);
					write_handle.abort();
				});
			}
			Err(_) => break,
		}
	}
}

async fn mock_handle_request(
	state: &MockState,
	conn_id: u64,
	actor_id: &str,
	data: RequestData,
	open_actors: &Mutex<Vec<String>>,
) -> ResponseData {
	match data {
		RequestData::ActorOpenRequest => {
			// Wait for gate if set (for testing reconnect waiting).
			{
				let gate = state.open_gate.lock().await;
				if let Some(sem) = gate.as_ref() {
					let sem = sem.clone();
					drop(gate);
					let _permit = sem.acquire().await.unwrap();
				}
			}
			let mut locks = state.locks.lock().await;
			if let Some(&holder) = locks.get(actor_id) {
				if holder != conn_id {
					return ResponseData::ErrorResponse(ErrorResponse {
						code: "actor_locked".into(),
						message: "actor is locked by another connection".into(),
					});
				}
			}
			locks.insert(actor_id.to_string(), conn_id);
			open_actors.lock().await.push(actor_id.to_string());
			state.stores.lock().await.entry(actor_id.to_string()).or_default();
			state.ops.lock().await.push(MockOp::Open { actor_id: actor_id.to_string() });
			ResponseData::ActorOpenResponse
		}
		RequestData::ActorCloseRequest => {
			let mut locks = state.locks.lock().await;
			if locks.get(actor_id) == Some(&conn_id) {
				locks.remove(actor_id);
			}
			open_actors.lock().await.retain(|a| a != actor_id);
			state.ops.lock().await.push(MockOp::Close { actor_id: actor_id.to_string() });
			ResponseData::ActorCloseResponse
		}
		RequestData::KvGetRequest(req) => {
			{
				let locks = state.locks.lock().await;
				if locks.get(actor_id) != Some(&conn_id) {
					return ResponseData::ErrorResponse(ErrorResponse {
						code: "actor_not_open".into(),
						message: "actor is not open".into(),
					});
				}
			}
			state.ops.lock().await.push(MockOp::Get {
				actor_id: actor_id.to_string(),
				keys: req.keys.clone(),
			});
			let stores = state.stores.lock().await;
			let store = stores.get(actor_id);
			let mut found_keys = Vec::new();
			let mut found_values = Vec::new();
			for key in &req.keys {
				if let Some(s) = store {
					if let Some(v) = s.get(key) {
						found_keys.push(key.clone());
						found_values.push(v.clone());
					}
				}
			}
			ResponseData::KvGetResponse(KvGetResponse {
				keys: found_keys,
				values: found_values,
			})
		}
		RequestData::KvPutRequest(req) => {
			{
				let locks = state.locks.lock().await;
				if locks.get(actor_id) != Some(&conn_id) {
					return ResponseData::ErrorResponse(ErrorResponse {
						code: "actor_not_open".into(),
						message: "actor is not open".into(),
					});
				}
			}
			state.ops.lock().await.push(MockOp::Put {
				actor_id: actor_id.to_string(),
				keys: req.keys.clone(),
			});
			let mut stores = state.stores.lock().await;
			let store = stores.entry(actor_id.to_string()).or_default();
			for (k, v) in req.keys.into_iter().zip(req.values) {
				store.insert(k, v);
			}
			ResponseData::KvPutResponse
		}
		RequestData::KvDeleteRequest(req) => {
			{
				let locks = state.locks.lock().await;
				if locks.get(actor_id) != Some(&conn_id) {
					return ResponseData::ErrorResponse(ErrorResponse {
						code: "actor_not_open".into(),
						message: "actor is not open".into(),
					});
				}
			}
			state.ops.lock().await.push(MockOp::Delete {
				actor_id: actor_id.to_string(),
				keys: req.keys.clone(),
			});
			let mut stores = state.stores.lock().await;
			if let Some(store) = stores.get_mut(actor_id) {
				for k in &req.keys {
					store.remove(k);
				}
			}
			ResponseData::KvDeleteResponse
		}
		RequestData::KvDeleteRangeRequest(req) => {
			{
				let locks = state.locks.lock().await;
				if locks.get(actor_id) != Some(&conn_id) {
					return ResponseData::ErrorResponse(ErrorResponse {
						code: "actor_not_open".into(),
						message: "actor is not open".into(),
					});
				}
			}
			state.ops.lock().await.push(MockOp::DeleteRange {
				actor_id: actor_id.to_string(),
				start: req.start.clone(),
				end: req.end.clone(),
			});
			let mut stores = state.stores.lock().await;
			if let Some(store) = stores.get_mut(actor_id) {
				let to_remove: Vec<Vec<u8>> = store
					.range(req.start.clone()..req.end.clone())
					.map(|(k, _)| k.clone())
					.collect();
				for k in to_remove {
					store.remove(&k);
				}
			}
			ResponseData::KvDeleteResponse
		}
	}
}

// MARK: Test Helpers

fn create_runtime() -> Runtime {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
}

/// Set up a mock server, connect channel, and open an actor.
async fn setup_server_and_channel(actor_id: &str) -> (MockKvServer, Arc<KvChannel>) {
	let server = MockKvServer::start().await;
	let channel = KvChannel::connect(KvChannelConfig {
		url: server.url(),
		token: None,
		namespace: "test".into(),
	});
	tokio::time::sleep(Duration::from_millis(300)).await;
	let channel = Arc::new(channel);
	channel.open_actor(actor_id).await.unwrap();
	(server, channel)
}

/// Open a SQLite database via the KV VFS.
fn open_test_db(rt: &Runtime, channel: Arc<KvChannel>, actor_id: &str) -> vfs::NativeDatabase {
	let vfs_name = unique_vfs_name(actor_id);
	let kv_vfs = vfs::KvVfs::register(
		&vfs_name,
		channel,
		actor_id.to_string(),
		rt.handle().clone(),
	)
	.unwrap();
	vfs::open_database(kv_vfs, actor_id).unwrap()
}

/// Execute a SQL statement, panicking on failure.
unsafe fn exec_sql(db: *mut sqlite3, sql: &str) {
	let c_sql = CString::new(sql).unwrap();
	let rc = sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut());
	if rc != SQLITE_OK {
		let msg = CStr::from_ptr(sqlite3_errmsg(db)).to_string_lossy();
		panic!("SQL '{}' failed (rc={}): {}", sql, rc, msg);
	}
}

/// Query a SQL statement and return rows as Vec<Vec<String>>.
unsafe fn query_rows(db: *mut sqlite3, sql: &str) -> Vec<Vec<String>> {
	let c_sql = CString::new(sql).unwrap();
	let mut stmt: *mut sqlite3_stmt = ptr::null_mut();
	let rc = sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
	if rc != SQLITE_OK {
		let msg = CStr::from_ptr(sqlite3_errmsg(db)).to_string_lossy();
		panic!("Prepare '{}' failed: {}", sql, msg);
	}
	let col_count = sqlite3_column_count(stmt);
	let mut rows = Vec::new();
	loop {
		let rc = sqlite3_step(stmt);
		if rc == SQLITE_DONE {
			break;
		}
		assert_eq!(rc, SQLITE_ROW, "step failed with rc={rc}");
		let mut row = Vec::new();
		for i in 0..col_count {
			let ptr = sqlite3_column_text(stmt, i);
			if ptr.is_null() {
				row.push("NULL".to_string());
			} else {
				row.push(
					CStr::from_ptr(ptr as *const _)
						.to_string_lossy()
						.into_owned(),
				);
			}
		}
		rows.push(row);
	}
	sqlite3_finalize(stmt);
	rows
}

fn key_targets_file_tag(key: &[u8], file_tag: u8) -> bool {
	key.len() >= 4
		&& key[0] == kv::SQLITE_PREFIX
		&& (key[2] == kv::META_PREFIX || key[2] == kv::CHUNK_PREFIX)
		&& key[3] == file_tag
}

fn op_targets_file_tag(op: &MockOp, file_tag: u8) -> bool {
	match op {
		MockOp::Get { keys, .. } | MockOp::Put { keys, .. } | MockOp::Delete { keys, .. } => {
			keys.iter().any(|key| key_targets_file_tag(key, file_tag))
		}
		MockOp::DeleteRange { start, end, .. } => {
			key_targets_file_tag(start, file_tag) || key_targets_file_tag(end, file_tag)
		}
		MockOp::Open { .. } | MockOp::Close { .. } => false,
	}
}

// MARK: Tests

#[test]
fn test_basic_sql_through_vfs() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-basic"));
	let db = open_test_db(&rt, channel.clone(), "actor-basic");

	unsafe {
		exec_sql(db.as_ptr(), "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)");
		exec_sql(db.as_ptr(), "INSERT INTO test VALUES (1, 'hello')");
		exec_sql(db.as_ptr(), "INSERT INTO test VALUES (2, 'world')");

		let rows = query_rows(db.as_ptr(), "SELECT id, value FROM test ORDER BY id");
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0], vec!["1", "hello"]);
		assert_eq!(rows[1], vec!["2", "world"]);
	}

	// Verify KV store has main file metadata and at least chunk 0.
	let store = rt.block_on(server.get_store("actor-basic"));
	let meta_key = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	assert!(store.contains_key(&meta_key), "metadata key missing");
	let chunk0_key = kv::get_chunk_key(kv::FILE_TAG_MAIN, 0).to_vec();
	assert!(store.contains_key(&chunk0_key), "chunk 0 missing");

	// Verify metadata decodes to a valid file size.
	let meta = store.get(&meta_key).unwrap();
	let file_size = decode_file_meta(meta).expect("metadata decode failed");
	assert!(file_size > 0, "file size should be positive");

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_open_prewrites_empty_main_page() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-empty-page"));
	let db = open_test_db(&rt, channel.clone(), "actor-empty-page");

	let store = rt.block_on(server.get_store("actor-empty-page"));
	let meta_key = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	let chunk0_key = kv::get_chunk_key(kv::FILE_TAG_MAIN, 0).to_vec();

	let meta = store.get(&meta_key).expect("main metadata key missing");
	assert_eq!(decode_file_meta(meta).unwrap(), kv::CHUNK_SIZE as i64);

	let chunk0 = store.get(&chunk0_key).expect("main chunk 0 missing");
	assert_eq!(chunk0.len(), kv::CHUNK_SIZE);
	assert_eq!(&chunk0[..16], b"SQLite format 3\0");

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_warm_update_uses_batch_atomic_put_without_journal() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-batch-atomic"));
	let db = open_test_db(&rt, channel.clone(), "actor-batch-atomic");

	unsafe {
		exec_sql(db.as_ptr(), "CREATE TABLE counter (value INTEGER)");
		exec_sql(db.as_ptr(), "INSERT INTO counter VALUES (0)");
	}

	rt.block_on(server.reset_ops());

	unsafe {
		exec_sql(db.as_ptr(), "UPDATE counter SET value = value + 1");
	}

	let ops = rt.block_on(server.ops());
	let put_ops: Vec<_> = ops
		.iter()
		.filter(|op| matches!(op, MockOp::Put { .. }))
		.collect();
	let get_ops: Vec<_> = ops
		.iter()
		.filter(|op| matches!(op, MockOp::Get { .. }))
		.collect();

	assert_eq!(put_ops.len(), 1, "warm update should flush with a single put");
	assert_eq!(get_ops.len(), 0, "warm update should not need KV reads");
	assert!(
		!ops.iter().any(|op| op_targets_file_tag(op, kv::FILE_TAG_JOURNAL)),
		"warm update should not touch journal keys when BATCH_ATOMIC is active"
	);

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_multi_chunk_data() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-multi-chunk"));
	let db = open_test_db(&rt, channel.clone(), "actor-multi-chunk");

	unsafe {
		exec_sql(db.as_ptr(), "CREATE TABLE big (id INTEGER PRIMARY KEY, data TEXT)");
		// Insert enough data to span multiple 4 KiB chunks.
		for i in 0..20 {
			let data = "X".repeat(1000);
			let sql = format!("INSERT INTO big VALUES ({i}, '{data}')");
			exec_sql(db.as_ptr(), &sql);
		}
	}

	let store = rt.block_on(server.get_store("actor-multi-chunk"));

	// Count chunk keys for the main file.
	let chunk_keys: Vec<_> = store
		.keys()
		.filter(|k| {
			k.len() == 8
				&& k[0] == kv::SQLITE_PREFIX
				&& k[2] == kv::CHUNK_PREFIX
				&& k[3] == kv::FILE_TAG_MAIN
		})
		.collect();
	assert!(
		chunk_keys.len() >= 2,
		"expected at least 2 chunks, got {}",
		chunk_keys.len()
	);

	// Verify chunk indices are sequential starting from 0.
	let mut indices: Vec<u32> = chunk_keys
		.iter()
		.map(|k| u32::from_be_bytes([k[4], k[5], k[6], k[7]]))
		.collect();
	indices.sort();
	for (i, &idx) in indices.iter().enumerate() {
		assert_eq!(idx, i as u32, "chunk indices should be sequential");
	}

	// Verify metadata shows file size spanning 2+ chunks.
	let meta_key = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	let file_size = decode_file_meta(store.get(&meta_key).unwrap()).unwrap();
	assert!(
		file_size >= (kv::CHUNK_SIZE * 2) as i64,
		"file should span at least 2 chunks, size={file_size}"
	);

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_chunk_boundary_data_integrity() {
	let rt = create_runtime();
	let (_server, channel) = rt.block_on(setup_server_and_channel("actor-boundary"));
	let db = open_test_db(&rt, channel.clone(), "actor-boundary");

	unsafe {
		exec_sql(
			db.as_ptr(),
			"CREATE TABLE chunks (id INTEGER PRIMARY KEY, payload TEXT)",
		);
		// Insert enough data to span chunk boundaries.
		for i in 0..50 {
			let payload = format!("{:0>500}", i);
			let sql = format!("INSERT INTO chunks VALUES ({i}, '{payload}')");
			exec_sql(db.as_ptr(), &sql);
		}

		// Verify all data reads back correctly despite chunk boundaries.
		let rows = query_rows(db.as_ptr(), "SELECT id, payload FROM chunks ORDER BY id");
		assert_eq!(rows.len(), 50);
		for (i, row) in rows.iter().enumerate() {
			assert_eq!(row[0], i.to_string());
			assert_eq!(row[1], format!("{:0>500}", i));
		}
	}

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_large_truncate_journal_fallback_produces_delete_batches() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-truncate"));
	let db = open_test_db(&rt, channel.clone(), "actor-truncate");

	unsafe {
		exec_sql(db.as_ptr(), "PRAGMA journal_mode = truncate");
		exec_sql(db.as_ptr(), "CREATE TABLE trunc (x TEXT)");
	}

	rt.block_on(server.reset_ops());

	unsafe {
		exec_sql(db.as_ptr(), "BEGIN");
		for i in 0..200 {
			let data = "Z".repeat(3500);
			exec_sql(
				db.as_ptr(),
				&format!("INSERT INTO trunc VALUES ('{data}{i:03}')"),
			);
		}
		exec_sql(db.as_ptr(), "COMMIT");
	}

	let ops = rt.block_on(server.ops());
	let delete_ops: Vec<_> = ops
		.iter()
		.filter(|op| matches!(op, MockOp::Delete { .. }) && op_targets_file_tag(op, kv::FILE_TAG_JOURNAL))
		.collect();
	assert!(
		!delete_ops.is_empty(),
		"expected journal Delete operations from truncate fallback"
	);

	for op in &delete_ops {
		if let MockOp::Delete { keys, .. } = op {
			for key in keys {
				assert_eq!(key[0], kv::SQLITE_PREFIX, "key should have SQLITE_PREFIX");
				assert_eq!(key[2], kv::CHUNK_PREFIX, "key should have CHUNK_PREFIX");
			}
		}
	}

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_small_default_transaction_avoids_journal_keys() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-del-journal"));
	let db = open_test_db(&rt, channel.clone(), "actor-del-journal");

	unsafe {
		exec_sql(db.as_ptr(), "CREATE TABLE djtest (x TEXT)");
		exec_sql(db.as_ptr(), "INSERT INTO djtest VALUES ('seed')");
	}

	rt.block_on(server.reset_ops());

	unsafe {
		exec_sql(db.as_ptr(), "BEGIN");
		for i in 0..20 {
			exec_sql(db.as_ptr(), &format!("INSERT INTO djtest VALUES ('row_{i}')"));
		}
		exec_sql(db.as_ptr(), "COMMIT");
	}

	let ops = rt.block_on(server.ops());
	let put_ops: Vec<_> = ops
		.iter()
		.filter(|op| matches!(op, MockOp::Put { .. }))
		.collect();
	assert!(
		!ops.iter().any(|op| op_targets_file_tag(op, kv::FILE_TAG_JOURNAL)),
		"small transactions should avoid journal keys when BATCH_ATOMIC is active"
	);
	assert_eq!(
		put_ops.len(),
		1,
		"small transactions should flush once at COMMIT_ATOMIC_WRITE"
	);

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_metadata_tracks_file_size() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-metadata"));
	let db = open_test_db(&rt, channel.clone(), "actor-metadata");

	unsafe {
		exec_sql(db.as_ptr(), "CREATE TABLE meta_test (id INTEGER)");
	}

	let store = rt.block_on(server.get_store("actor-metadata"));
	let meta_key = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	let meta = store.get(&meta_key).unwrap();
	let file_size = decode_file_meta(meta).unwrap();

	// After CREATE TABLE, the file should be at least 1 page (4096 bytes).
	assert!(
		file_size >= 4096,
		"file should be at least 1 page, got {file_size}"
	);
	assert_eq!(
		file_size % 4096,
		0,
		"file size should be page-aligned, got {file_size}"
	);

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_close_flushes_and_reopen_preserves_data() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-reopen"));

	// Write data and close the database.
	{
		let db = open_test_db(&rt, channel.clone(), "actor-reopen");
		unsafe {
			exec_sql(db.as_ptr(), "CREATE TABLE persist (id INTEGER, val TEXT)");
			exec_sql(db.as_ptr(), "INSERT INTO persist VALUES (1, 'saved')");
			exec_sql(db.as_ptr(), "INSERT INTO persist VALUES (2, 'data')");
		}
		drop(db); // xClose flushes metadata
	}

	// Verify metadata was flushed to the store.
	let store = rt.block_on(server.get_store("actor-reopen"));
	let meta_key = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	assert!(store.contains_key(&meta_key), "metadata should be flushed on close");

	// Close and reopen actor (release and reacquire lock).
	rt.block_on(async {
		channel.close_actor("actor-reopen").await.unwrap();
		channel.open_actor("actor-reopen").await.unwrap();
	});

	// Reopen database and verify data persists.
	{
		let db = open_test_db(&rt, channel.clone(), "actor-reopen");
		unsafe {
			let rows = query_rows(db.as_ptr(), "SELECT id, val FROM persist ORDER BY id");
			assert_eq!(rows.len(), 2);
			assert_eq!(rows[0], vec!["1", "saved"]);
			assert_eq!(rows[1], vec!["2", "data"]);
		}
		drop(db);
	}

	rt.block_on(channel.disconnect());
}

#[test]
fn test_file_tags_encoding() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-tags"));
	let db = open_test_db(&rt, channel.clone(), "actor-tags");

	unsafe {
		// A write transaction creates a journal file with a different file tag.
		exec_sql(db.as_ptr(), "BEGIN");
		exec_sql(db.as_ptr(), "CREATE TABLE tag_test (x INTEGER)");
		exec_sql(db.as_ptr(), "INSERT INTO tag_test VALUES (1)");
		exec_sql(db.as_ptr(), "COMMIT");
	}

	let store = rt.block_on(server.get_store("actor-tags"));

	// Main file metadata and chunks should exist.
	let main_meta = kv::get_meta_key(kv::FILE_TAG_MAIN).to_vec();
	assert!(store.contains_key(&main_meta), "main metadata should exist");
	let main_chunk0 = kv::get_chunk_key(kv::FILE_TAG_MAIN, 0).to_vec();
	assert!(store.contains_key(&main_chunk0), "main chunk 0 should exist");

	// All chunk keys should have valid file tags.
	let chunk_keys: Vec<_> = store
		.keys()
		.filter(|k| k.len() == 8 && k[0] == kv::SQLITE_PREFIX && k[2] == kv::CHUNK_PREFIX)
		.collect();
	for key in &chunk_keys {
		assert!(
			key[3] == kv::FILE_TAG_MAIN
				|| key[3] == kv::FILE_TAG_JOURNAL
				|| key[3] == kv::FILE_TAG_WAL
				|| key[3] == kv::FILE_TAG_SHM,
			"unexpected file tag: {}",
			key[3]
		);
	}

	drop(db);
	rt.block_on(channel.disconnect());
}

#[test]
fn test_error_actor_not_open() {
	let rt = create_runtime();
	let (_server, channel) = rt.block_on(async {
		let server = MockKvServer::start().await;
		let channel = KvChannel::connect(KvChannelConfig {
			url: server.url(),
			token: None,
			namespace: "test".into(),
		});
		tokio::time::sleep(Duration::from_millis(300)).await;
		(server, Arc::new(channel))
	});

	// Send a KV request without opening the actor.
	let result = rt.block_on(
		channel.send_request(
			"unopened-actor",
			RequestData::KvGetRequest(KvGetRequest {
				keys: vec![vec![1]],
			}),
		),
	);

	assert!(
		matches!(
			result,
			Err(ChannelError::ServerError(ref e)) if e.code == "actor_not_open"
		),
		"expected actor_not_open error, got: {result:?}"
	);

	rt.block_on(channel.disconnect());
}

#[test]
fn test_error_actor_locked() {
	let rt = create_runtime();
	let (_server, ch1, ch2) = rt.block_on(async {
		let server = MockKvServer::start().await;
		let config = KvChannelConfig {
			url: server.url(),
			token: None,
			namespace: "test".into(),
		};
		let ch1 = KvChannel::connect(config.clone());
		let ch2 = KvChannel::connect(config);
		tokio::time::sleep(Duration::from_millis(300)).await;

		let ch1 = Arc::new(ch1);
		let ch2 = Arc::new(ch2);

		// First channel opens the actor.
		ch1.open_actor("shared-actor").await.unwrap();
		(server, ch1, ch2)
	});

	// Second channel tries to open the same actor.
	let result = rt.block_on(ch2.open_actor("shared-actor"));
	assert!(
		matches!(
			result,
			Err(ChannelError::ServerError(ref e)) if e.code == "actor_locked"
		),
		"expected actor_locked error, got: {result:?}"
	);

	rt.block_on(ch1.disconnect());
	rt.block_on(ch2.disconnect());
}

#[test]
fn test_optimistic_open_pipelining() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-pipeline"));

	// Fire off multiple KV requests concurrently (pipelined on the WebSocket).
	let results: Vec<Result<ResponseData, ChannelError>> = rt.block_on(async {
		let mut handles = Vec::new();
		for i in 0..5u8 {
			let ch = channel.clone();
			handles.push(tokio::spawn(async move {
				ch.send_request(
					"actor-pipeline",
					RequestData::KvPutRequest(KvPutRequest {
						keys: vec![vec![i]],
						values: vec![vec![i, i]],
					}),
				)
				.await
			}));
		}
		let mut results = Vec::new();
		for h in handles {
			results.push(h.await.unwrap());
		}
		results
	});

	// All pipelined requests should succeed.
	for (i, result) in results.iter().enumerate() {
		assert!(
			matches!(result, Ok(ResponseData::KvPutResponse)),
			"pipelined request {i} failed: {result:?}"
		);
	}

	// Verify all 5 keys were stored.
	let store = rt.block_on(server.get_store("actor-pipeline"));
	for i in 0..5u8 {
		assert_eq!(store.get(&vec![i]), Some(&vec![i, i]));
	}

	rt.block_on(channel.disconnect());
}

#[test]
fn test_reconnection_reopens_actors() {
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-reconnect"));

	// Verify initial connectivity.
	let result = rt.block_on(channel.send_request(
		"actor-reconnect",
		RequestData::KvPutRequest(KvPutRequest {
			keys: vec![vec![0x01]],
			values: vec![vec![0xAA]],
		}),
	));
	assert!(result.is_ok(), "initial put failed: {result:?}");

	// Force-close all connections to simulate network failure.
	rt.block_on(async {
		server.close_all_connections().await;
		// Give the connection handlers time to release locks.
		tokio::time::sleep(Duration::from_millis(200)).await;
	});

	// Wait for reconnect (initial backoff ~1s + connection time).
	rt.block_on(async {
		tokio::time::sleep(Duration::from_secs(3)).await;
	});

	// After reconnect, the channel should have re-opened the actor.
	// Verify by reading back the data we stored before the disconnect.
	let result = rt.block_on(channel.send_request(
		"actor-reconnect",
		RequestData::KvGetRequest(KvGetRequest {
			keys: vec![vec![0x01]],
		}),
	));
	match &result {
		Ok(ResponseData::KvGetResponse(resp)) => {
			assert_eq!(resp.keys, vec![vec![0x01u8]]);
			assert_eq!(resp.values, vec![vec![0xAAu8]]);
		}
		other => panic!("KV get after reconnect failed: {other:?}"),
	}

	// Verify the actor was opened at least twice (initial + reconnect).
	let ops = rt.block_on(server.ops());
	let open_count = ops
		.iter()
		.filter(|op| {
			matches!(op, MockOp::Open { actor_id } if actor_id == "actor-reconnect")
		})
		.count();
	assert!(
		open_count >= 2,
		"actor should have been opened at least twice (initial + reconnect), got {open_count}"
	);
}

#[test]
fn test_reconnect_kv_waits_for_open_response() {
	// Verify that on reconnect, KV requests block until ActorOpenResponse is
	// received. Uses a semaphore gate on the mock server to hold the open
	// response, then checks that a KV request hasn't completed (client is
	// waiting), and finally releases the gate to confirm the request succeeds.
	//
	// The mock server processes messages concurrently (spawned tasks), so
	// without client-side waiting, a KV request sent during the gate hold
	// would hit actor_not_open (lock not yet acquired). With client-side
	// waiting, the KV request is held on the client until the open completes.
	let rt = create_runtime();
	let (server, channel) = rt.block_on(setup_server_and_channel("actor-rwait"));

	// Write initial data.
	rt.block_on(
		channel.send_request(
			"actor-rwait",
			RequestData::KvPutRequest(KvPutRequest {
				keys: vec![vec![0x01]],
				values: vec![vec![0xEE]],
			}),
		),
	)
	.unwrap();

	// Set up gate (0 permits = blocks open responses).
	let gate = Arc::new(Semaphore::new(0));
	rt.block_on(async {
		*server.state.open_gate.lock().await = Some(gate.clone());
	});

	// Force disconnect.
	rt.block_on(async {
		server.close_all_connections().await;
		tokio::time::sleep(Duration::from_millis(200)).await;
	});

	// Wait for WebSocket to reconnect (backoff ~1s + connection time).
	// The reconnect ActorOpenRequest is sent and received by the mock server,
	// but the response is held by the gate.
	rt.block_on(async {
		tokio::time::sleep(Duration::from_secs(2)).await;
	});

	// Spawn a task that sends a KV request. With reconnect waiting, this
	// should block until the ActorOpenResponse arrives.
	let ch = channel.clone();
	let kv_handle = rt.spawn(async move {
		ch.send_request(
			"actor-rwait",
			RequestData::KvGetRequest(KvGetRequest {
				keys: vec![vec![0x01]],
			}),
		)
		.await
	});

	// Give the KV task time to reach the wait point.
	rt.block_on(async {
		tokio::time::sleep(Duration::from_millis(500)).await;
	});

	// Verify the KV task is still pending (blocked by reconnect readiness).
	// Without client-side waiting, the concurrent mock server would have
	// already returned actor_not_open and the task would be finished.
	assert!(
		!kv_handle.is_finished(),
		"KV request should be waiting for ActorOpenResponse"
	);

	// Release the gate so the mock server sends ActorOpenResponse.
	gate.add_permits(1);

	// KV request should now complete successfully.
	let result = rt.block_on(kv_handle).unwrap();
	match &result {
		Ok(ResponseData::KvGetResponse(resp)) => {
			assert_eq!(resp.keys, vec![vec![0x01u8]]);
			assert_eq!(resp.values, vec![vec![0xEEu8]]);
		}
		other => panic!("KV get after gated reconnect failed: {other:?}"),
	}

	// Clean up gate.
	rt.block_on(async {
		*server.state.open_gate.lock().await = None;
	});
	rt.block_on(channel.disconnect());
}
