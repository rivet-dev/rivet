use super::*;

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{ActorLifecycle, SqliteRuntimeError};
use rusqlite::types::{Value as RusqliteValue, ValueRef};
use tokio::sync::OwnedMutexGuard;

struct TestEndpointDb {
	connection: Arc<parking_lot::Mutex<rusqlite::Connection>>,
	gate: Arc<tokio::sync::Mutex<()>>,
}

impl TestEndpointDb {
	fn new() -> Self {
		Self {
			connection: Arc::new(parking_lot::Mutex::new(
				rusqlite::Connection::open_in_memory().unwrap(),
			)),
			gate: Arc::new(tokio::sync::Mutex::new(())),
		}
	}
}

struct PausingBeginDb {
	inner: Arc<TestEndpointDb>,
	began: Notify,
	release: Notify,
	expired: Arc<Notify>,
}

struct NotifyingTransaction {
	inner: Arc<dyn EndpointTransaction>,
	expired: Arc<Notify>,
}

#[async_trait]
impl EndpointTransaction for NotifyingTransaction {
	async fn exec(&self, sql: String) -> Result<()> {
		self.inner.exec(sql).await
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.inner.query(sql, params).await
	}

	async fn commit(&self) -> Result<()> {
		self.inner.commit().await
	}

	async fn rollback(&self) -> Result<()> {
		self.inner.rollback().await
	}

	async fn expire(&self) -> Result<()> {
		let result = self.inner.expire().await;
		self.expired.notify_one();
		result
	}
}

#[async_trait]
impl EndpointDatabase for PausingBeginDb {
	async fn exec(&self, sql: String) -> Result<()> {
		self.inner.exec(sql).await
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.inner.query(sql, params).await
	}

	async fn begin(
		&self,
		key: String,
		timeout: Option<Duration>,
	) -> Result<Arc<dyn EndpointTransaction>> {
		let transaction = self.inner.begin(key, timeout).await?;
		self.began.notify_one();
		self.release.notified().await;
		Ok(Arc::new(NotifyingTransaction {
			inner: transaction,
			expired: Arc::clone(&self.expired),
		}))
	}
}

struct FailFirstBeginDb {
	inner: TestEndpointDb,
	fail: AtomicBool,
}

#[async_trait]
impl EndpointDatabase for FailFirstBeginDb {
	async fn exec(&self, sql: String) -> Result<()> {
		self.inner.exec(sql).await
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.inner.query(sql, params).await
	}

	async fn begin(
		&self,
		key: String,
		timeout: Option<Duration>,
	) -> Result<Arc<dyn EndpointTransaction>> {
		if self.fail.swap(false, Ordering::AcqRel) {
			return Err(TransactionQueueFullError.into());
		}
		self.inner.begin(key, timeout).await
	}
}

struct TestEndpointTransaction {
	connection: Arc<parking_lot::Mutex<rusqlite::Connection>>,
	guard: Mutex<Option<OwnedMutexGuard<()>>>,
	expired: AtomicBool,
	timeout_ms: u64,
}

impl TestEndpointTransaction {
	fn ensure_active(&self) -> Result<()> {
		if self.expired.load(Ordering::Acquire) {
			return Err(TransactionExpiredError {
				timeout_ms: self.timeout_ms,
			}
			.into());
		}
		Ok(())
	}

	async fn finish(&self, sql: &str, expired: bool) -> Result<()> {
		if expired {
			self.expired.store(true, Ordering::Release);
		} else {
			self.ensure_active()?;
		}
		let mut guard = self.guard.lock().await;
		if guard.is_some() {
			self.connection
				.lock()
				.execute_batch(sql)
				.map_err(test_sql_error)?;
			guard.take();
		}
		Ok(())
	}
}

#[async_trait]
impl EndpointTransaction for TestEndpointTransaction {
	async fn exec(&self, sql: String) -> Result<()> {
		self.ensure_active()?;
		self.connection
			.lock()
			.execute_batch(&sql)
			.map_err(test_sql_error)
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.ensure_active()?;
		query_connection(&mut self.connection.lock(), &sql, params)
	}

	async fn commit(&self) -> Result<()> {
		self.finish("COMMIT", false).await
	}

	async fn rollback(&self) -> Result<()> {
		self.finish("ROLLBACK", false).await
	}

	async fn expire(&self) -> Result<()> {
		self.finish("ROLLBACK", true).await
	}
}

#[async_trait]
impl EndpointDatabase for TestEndpointDb {
	async fn exec(&self, sql: String) -> Result<()> {
		let _gate = Arc::clone(&self.gate).lock_owned().await;
		self.connection
			.lock()
			.execute_batch(&sql)
			.map_err(test_sql_error)
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		let _gate = Arc::clone(&self.gate).lock_owned().await;
		query_connection(&mut self.connection.lock(), &sql, params)
	}

	async fn begin(
		&self,
		_key: String,
		timeout: Option<Duration>,
	) -> Result<Arc<dyn EndpointTransaction>> {
		let guard = Arc::clone(&self.gate).lock_owned().await;
		self.connection
			.lock()
			.execute_batch("BEGIN")
			.map_err(test_sql_error)?;
		let timeout = timeout.unwrap_or(crate::actor::sqlite::DEFAULT_TRANSACTION_TIMEOUT);
		let transaction = Arc::new(TestEndpointTransaction {
			connection: Arc::clone(&self.connection),
			guard: Mutex::new(Some(guard)),
			expired: AtomicBool::new(false),
			timeout_ms: timeout.as_millis() as u64,
		});
		let weak = Arc::downgrade(&transaction);
		tokio::spawn(async move {
			tokio::time::sleep(timeout).await;
			if let Some(transaction) = weak.upgrade() {
				let _ = transaction.expire().await;
			}
		});
		Ok(transaction)
	}
}

fn query_connection(
	connection: &mut rusqlite::Connection,
	sql: &str,
	params: Vec<BindParam>,
) -> Result<ExecuteResult> {
	if sql.trim().trim_end_matches(';').contains(';') {
		bail!("SQLite execute only supports one statement");
	}
	let mut statement = connection.prepare(sql).map_err(test_sql_error)?;
	let columns = statement
		.column_names()
		.into_iter()
		.map(str::to_owned)
		.collect::<Vec<_>>();
	let params = params.into_iter().map(|param| match param {
		BindParam::Null => RusqliteValue::Null,
		BindParam::Integer(value) => RusqliteValue::Integer(value),
		BindParam::Float(value) => RusqliteValue::Real(value),
		BindParam::Text(value) => RusqliteValue::Text(value),
		BindParam::Blob(value) => RusqliteValue::Blob(value),
	});
	let mut query = statement
		.query(rusqlite::params_from_iter(params))
		.map_err(test_sql_error)?;
	let mut rows = Vec::new();
	while let Some(row) = query.next().map_err(test_sql_error)? {
		rows.push(
			(0..columns.len())
				.map(|index| match row.get_ref(index).unwrap() {
					ValueRef::Null => ColumnValue::Null,
					ValueRef::Integer(value) => ColumnValue::Integer(value),
					ValueRef::Real(value) => ColumnValue::Float(value),
					ValueRef::Text(value) => {
						ColumnValue::Text(String::from_utf8_lossy(value).into_owned())
					}
					ValueRef::Blob(value) => ColumnValue::Blob(value.to_vec()),
				})
				.collect(),
		);
	}
	drop(query);
	drop(statement);
	let changes = connection.changes() as i64;
	Ok(ExecuteResult {
		columns,
		rows,
		changes,
		last_insert_row_id: (changes > 0).then(|| connection.last_insert_rowid()),
		readonly: None,
		commit_seq: None,
	})
}

fn test_sql_error(error: rusqlite::Error) -> anyhow::Error {
	error.into()
}

async fn send_hello(stream: &mut UnixStream, version: u16) {
	let payload = wire::versioned::ClientHello::wrap_latest(())
		.serialize_with_embedded_version(version)
		.unwrap();
	write_payload(stream, &payload).await.unwrap();
}

async fn read_hello(stream: &mut UnixStream) -> wire::ServerHello {
	let FrameRead::Payload(payload) = read_frame(stream, DEFAULT_MAX_FRAME_BYTES).await.unwrap()
	else {
		panic!("expected hello payload")
	};
	wire::versioned::ServerHello::deserialize_with_embedded_version(&payload).unwrap()
}

async fn send_request(
	stream: &mut UnixStream,
	request_id: u32,
	lease_key: Option<&str>,
	payload: wire::RequestPayload,
) {
	let payload =
		wire::versioned::ClientFrame::wrap_latest(wire::ClientFrame::Request(wire::Request {
			request_id,
			lease_key: lease_key.map(str::to_owned),
			payload,
		}))
		.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
		.unwrap();
	write_payload(stream, &payload).await.unwrap();
}

async fn read_response(stream: &mut UnixStream) -> wire::Response {
	let FrameRead::Payload(payload) = read_frame(stream, DEFAULT_MAX_FRAME_BYTES).await.unwrap()
	else {
		panic!("expected response payload")
	};
	let wire::ServerFrame::Response(response) =
		wire::versioned::ServerFrame::deserialize_with_embedded_version(&payload).unwrap()
	else {
		panic!("expected response frame")
	};
	response
}

async fn connect_test_db(
	db: Arc<TestEndpointDb>,
) -> (UnixStream, CancellationToken, JoinHandle<Result<()>>) {
	let (server, mut client) = UnixStream::pair().unwrap();
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_connection(
		server,
		db,
		cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	send_hello(&mut client, wire::PROTOCOL_VERSION).await;
	assert!(matches!(
		read_hello(&mut client).await,
		wire::ServerHello::HelloOk(_)
	));
	(client, cancel, task)
}

async fn begin(stream: &mut UnixStream, request_id: u32, key: &str, timeout_ms: Option<u64>) {
	send_request(
		stream,
		request_id,
		None,
		wire::RequestPayload::SqliteBegin(wire::SqliteBegin {
			lease_key: key.to_owned(),
			timeout_ms,
		}),
	)
	.await;
}

#[tokio::test]
async fn handshake_accepts_supported_version_and_rejects_both_skew_directions() {
	let (server, mut client) = UnixStream::pair().unwrap();
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_connection(
		server,
		Arc::new(TestEndpointDb::new()),
		cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	send_hello(&mut client, wire::PROTOCOL_VERSION).await;
	assert!(matches!(
		read_hello(&mut client).await,
		wire::ServerHello::HelloOk(_)
	));
	let mut wrong_version =
		wire::versioned::ClientFrame::wrap_latest(wire::ClientFrame::Request(wire::Request {
			request_id: 1,
			lease_key: None,
			payload: wire::RequestPayload::SqliteExec(wire::SqliteExec {
				script: "SELECT 1".to_owned(),
			}),
		}))
		.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
		.unwrap();
	wrong_version[..2].copy_from_slice(&(wire::PROTOCOL_VERSION + 1).to_le_bytes());
	write_payload(&mut client, &wrong_version).await.unwrap();
	let FrameRead::Payload(payload) = read_frame(&mut client, DEFAULT_MAX_FRAME_BYTES)
		.await
		.unwrap()
	else {
		panic!("expected go away")
	};
	assert!(matches!(
		wire::versioned::ServerFrame::deserialize_with_embedded_version(&payload).unwrap(),
		wire::ServerFrame::GoAway(wire::GoAway {
			reason: wire::GoAwayReason::MalformedFrame
		})
	));
	task.await.unwrap().unwrap();
	cancel.cancel();

	for version in [wire::PROTOCOL_VERSION - 1, wire::PROTOCOL_VERSION + 1] {
		let (server, mut client) = UnixStream::pair().unwrap();
		let task = tokio::spawn(serve_connection(
			server,
			Arc::new(TestEndpointDb::new()),
			CancellationToken::new(),
			DEFAULT_MAX_FRAME_BYTES,
		));
		write_payload(&mut client, &version.to_le_bytes())
			.await
			.unwrap();
		assert!(matches!(
			read_hello(&mut client).await,
			wire::ServerHello::HelloRejectUnsupportedVersion
		));
		task.await.unwrap().unwrap();
	}
}

#[tokio::test]
async fn duplicate_request_id_goes_away() {
	let (mut client, _cancel, task) = connect_test_db(Arc::new(TestEndpointDb::new())).await;
	begin(&mut client, 1, "owner", None).await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteBeginOk
	));
	for _ in 0..2 {
		send_request(
			&mut client,
			2,
			None,
			wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
				sql: "SELECT 1".to_owned(),
				params: Vec::new(),
			}),
		)
		.await;
	}
	let FrameRead::Payload(payload) = read_frame(&mut client, DEFAULT_MAX_FRAME_BYTES)
		.await
		.unwrap()
	else {
		panic!("expected go away")
	};
	assert!(matches!(
		wire::versioned::ServerFrame::deserialize_with_embedded_version(&payload).unwrap(),
		wire::ServerFrame::GoAway(wire::GoAway {
			reason: wire::GoAwayReason::MalformedFrame
		})
	));
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn completed_request_id_is_immediately_reusable() {
	let (mut client, cancel, task) = connect_test_db(Arc::new(TestEndpointDb::new())).await;
	for expected in [1, 2] {
		send_request(
			&mut client,
			7,
			None,
			wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
				sql: format!("SELECT {expected}"),
				params: Vec::new(),
			}),
		)
		.await;
		let response = read_response(&mut client).await;
		assert_eq!(response.request_id, 7);
		let wire::ResponsePayload::SqliteQueryOk(result) = response.payload else {
			panic!("expected query response");
		};
		assert_eq!(result.rows, [vec![wire::SqlValue::SqlInteger(expected)]]);
	}
	cancel.cancel();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn terminal_go_away_suppresses_later_responses() {
	let (server, mut client) = UnixStream::pair().unwrap();
	let (_, stream) = server.into_split();
	let writer = Arc::new(Mutex::new(ConnectionWriter {
		stream,
		terminal: false,
	}));
	let request_ids: RequestIds = Arc::new(Mutex::new(BTreeSet::from([9])));
	let cancel = CancellationToken::new();

	write_connection_go_away(
		&writer,
		wire::GoAwayReason::MalformedFrame,
		wire::PROTOCOL_VERSION,
	)
	.await
	.unwrap();
	write_request_response(
		&writer,
		&request_ids,
		&cancel,
		9,
		wire::ResponsePayload::SqliteExecOk,
		wire::PROTOCOL_VERSION,
		DEFAULT_MAX_FRAME_BYTES,
	)
	.await;

	let FrameRead::Payload(payload) = read_frame(&mut client, DEFAULT_MAX_FRAME_BYTES)
		.await
		.unwrap()
	else {
		panic!("expected go away");
	};
	assert!(matches!(
		wire::versioned::ServerFrame::deserialize_with_embedded_version(&payload).unwrap(),
		wire::ServerFrame::GoAway(wire::GoAway {
			reason: wire::GoAwayReason::MalformedFrame
		})
	));
	assert!(request_ids.lock().await.is_empty());
	assert!(
		tokio::time::timeout(
			Duration::from_millis(25),
			read_frame(&mut client, DEFAULT_MAX_FRAME_BYTES)
		)
		.await
		.is_err(),
		"a normal response must not follow GoAway"
	);
}

#[tokio::test]
async fn pending_lease_waiter_cannot_miss_begin_completion() {
	let db = TestEndpointDb::new();
	let transaction = db.begin("coordinator-key".to_owned(), None).await.unwrap();
	let notify = Arc::new(Notify::new());
	let hook = Arc::new(PendingWaitTestHook::default());
	let leases = Arc::new(Mutex::new(ConnectionLeaseState {
		entries: BTreeMap::from([(
			"wire-key".to_owned(),
			LeaseEntry::Pending(Arc::clone(&notify)),
		)]),
		closed: false,
		pending_wait_test_hook: Some(Arc::clone(&hook)),
	}));

	let rechecked = hook.rechecked.notified();
	let waiter_leases = Arc::clone(&leases);
	let waiter = tokio::spawn(async move { active_transaction(&waiter_leases, "wire-key").await });
	rechecked.await;

	// Complete BEGIN in the exact window after the waiter rechecked Pending but
	// before it awaits Notify. This used to lose notify_waiters and hang forever.
	assert!(!complete_begin(&leases, "wire-key".to_owned(), Arc::clone(&transaction)).await);
	hook.resume.notify_one();
	let activated = tokio::time::timeout(Duration::from_secs(1), waiter)
		.await
		.expect("armed waiter must observe BEGIN completion")
		.unwrap()
		.unwrap();
	assert!(Arc::ptr_eq(&activated, &transaction));
	activated.rollback().await.unwrap();
}

#[tokio::test]
async fn exec_and_query_support_params_empty_results_rowids_and_reject_trailing_content() {
	let (mut client, cancel, task) = connect_test_db(Arc::new(TestEndpointDb::new())).await;
	send_request(
		&mut client,
		1,
		None,
		wire::RequestPayload::SqliteExec(wire::SqliteExec {
			script: "CREATE TABLE items(id INTEGER PRIMARY KEY, i INTEGER, r REAL, t TEXT, b BLOB)"
				.to_owned(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteExecOk
	));

	send_request(
		&mut client,
		2,
		None,
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "INSERT INTO items(i, r, t, b) VALUES (?, ?, ?, ?) RETURNING id, i, r, t, b"
				.to_owned(),
			params: vec![
				wire::SqlValue::SqlInteger(7),
				wire::SqlValue::SqlReal(1.5),
				wire::SqlValue::SqlText("hello".to_owned()),
				wire::SqlValue::SqlBlob(vec![1, 2, 3]),
			],
		}),
	)
	.await;
	let wire::ResponsePayload::SqliteQueryOk(inserted) = read_response(&mut client).await.payload
	else {
		panic!("expected query response")
	};
	assert_eq!(inserted.columns, ["id", "i", "r", "t", "b"]);
	assert_eq!(inserted.rows.len(), 1);
	assert_eq!(inserted.changes, 1);
	assert_eq!(inserted.last_insert_row_id, Some(1));

	for (request_id, sql, expected_columns) in [
		(3, "SELECT i, t FROM items WHERE 0", vec!["i", "t"]),
		(4, "SELECT 1; SELECT 2", Vec::new()),
	] {
		send_request(
			&mut client,
			request_id,
			None,
			wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
				sql: sql.to_owned(),
				params: Vec::new(),
			}),
		)
		.await;
		let response = read_response(&mut client).await;
		if request_id == 3 {
			let wire::ResponsePayload::SqliteQueryOk(empty) = response.payload else {
				panic!("expected empty query response")
			};
			assert_eq!(empty.columns, expected_columns);
			assert!(empty.rows.is_empty());
		} else {
			assert!(matches!(
				response.payload,
				wire::ResponsePayload::SqlError(_)
			));
		}
	}
	cancel.cancel();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn transactions_pipeline_queue_other_callers_commit_and_rollback() {
	let db = Arc::new(TestEndpointDb::new());
	let (mut owner, owner_cancel, owner_task) = connect_test_db(Arc::clone(&db)).await;
	let (mut other, other_cancel, other_task) = connect_test_db(db).await;
	send_request(
		&mut owner,
		1,
		None,
		wire::RequestPayload::SqliteExec(wire::SqliteExec {
			script: "CREATE TABLE items(value INTEGER)".to_owned(),
		}),
	)
	.await;
	let _ = read_response(&mut owner).await;

	begin(&mut owner, 2, "one", None).await;
	send_request(
		&mut owner,
		3,
		Some("one"),
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "INSERT INTO items VALUES (1)".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut owner).await.payload,
		wire::ResponsePayload::SqliteBeginOk
	));
	assert!(matches!(
		read_response(&mut owner).await.payload,
		wire::ResponsePayload::SqliteQueryOk(_)
	));
	for (client, request_id, key) in [(&mut owner, 30, "missing"), (&mut other, 31, "one")] {
		send_request(
			client,
			request_id,
			Some(key),
			wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
				sql: "INSERT INTO items VALUES (99)".to_owned(),
				params: Vec::new(),
			}),
		)
		.await;
		assert!(matches!(
			read_response(client).await.payload,
			wire::ResponsePayload::InvalidLeaseKey(_)
		));
	}

	send_request(
		&mut other,
		4,
		None,
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "SELECT count(*) FROM items".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	assert!(
		tokio::time::timeout(Duration::from_millis(25), read_response(&mut other))
			.await
			.is_err()
	);
	// A second transaction begin may be pipelined before the active transaction's
	// commit. It must wait without blocking the later commit from being read.
	begin(&mut owner, 6, "two", None).await;
	send_request(
		&mut owner,
		5,
		None,
		wire::RequestPayload::SqliteCommit(wire::SqliteCommit {
			lease_key: "one".to_owned(),
		}),
	)
	.await;
	let mut saw_commit = false;
	let mut saw_next_begin = false;
	for _ in 0..2 {
		let response = read_response(&mut owner).await;
		match (response.request_id, response.payload) {
			(5, wire::ResponsePayload::SqliteCommitOk) => saw_commit = true,
			(6, wire::ResponsePayload::SqliteBeginOk) => saw_next_begin = true,
			other => panic!("unexpected pipelined transaction response: {other:?}"),
		}
	}
	assert!(saw_commit && saw_next_begin);
	send_request(
		&mut owner,
		32,
		Some("one"),
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "INSERT INTO items VALUES (99)".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut owner).await.payload,
		wire::ResponsePayload::InvalidLeaseKey(_)
	));
	let wire::ResponsePayload::SqliteQueryOk(count) = read_response(&mut other).await.payload
	else {
		panic!("expected queued query response")
	};
	assert_eq!(count.rows, [vec![wire::SqlValue::SqlInteger(1)]]);

	send_request(
		&mut owner,
		7,
		Some("two"),
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "INSERT INTO items VALUES (2)".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	let _ = read_response(&mut owner).await;
	send_request(
		&mut owner,
		8,
		None,
		wire::RequestPayload::SqliteRollback(wire::SqliteRollback {
			lease_key: "two".to_owned(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut owner).await.payload,
		wire::ResponsePayload::SqliteRollbackOk
	));

	owner_cancel.cancel();
	other_cancel.cancel();
	drop(owner);
	drop(other);
	owner_task.await.unwrap().unwrap();
	other_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn disconnect_during_begin_finishes_rollback_without_orphaning_transaction() {
	let inner = Arc::new(TestEndpointDb::new());
	let db = Arc::new(PausingBeginDb {
		inner: Arc::clone(&inner),
		began: Notify::new(),
		release: Notify::new(),
		expired: Arc::new(Notify::new()),
	});
	let (server, mut client) = UnixStream::pair().unwrap();
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_connection(
		server,
		db.clone(),
		cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	send_hello(&mut client, wire::PROTOCOL_VERSION).await;
	let _ = read_hello(&mut client).await;

	let began = db.began.notified();
	begin(&mut client, 1, "cancelled", None).await;
	began.await;
	assert!(!inner.connection.lock().is_autocommit());
	send_request(
		&mut client,
		2,
		Some("cancelled"),
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "SELECT 1".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;

	let expired = db.expired.notified();
	cancel.cancel();
	db.release.notify_one();
	tokio::time::timeout(Duration::from_secs(1), expired)
		.await
		.expect("detached begin must be rolled back");
	assert!(inner.connection.lock().is_autocommit());
	tokio::time::timeout(
		Duration::from_secs(1),
		inner.exec("CREATE TABLE after_cancel(value INTEGER)".to_owned()),
	)
	.await
	.expect("coordinator gate must be released")
	.unwrap();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn transient_begin_failure_allows_same_key_retry() {
	let db = Arc::new(FailFirstBeginDb {
		inner: TestEndpointDb::new(),
		fail: AtomicBool::new(true),
	});
	let (server, mut client) = UnixStream::pair().unwrap();
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_connection(
		server,
		db,
		cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	send_hello(&mut client, wire::PROTOCOL_VERSION).await;
	let _ = read_hello(&mut client).await;

	begin(&mut client, 1, "retry", None).await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::QueueFull(_)
	));
	begin(&mut client, 2, "retry", None).await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteBeginOk
	));
	send_request(
		&mut client,
		3,
		None,
		wire::RequestPayload::SqliteRollback(wire::SqliteRollback {
			lease_key: "retry".to_owned(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteRollbackOk
	));
	cancel.cancel();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn expired_transaction_is_rolled_back_and_key_remains_poisoned() {
	let (mut client, cancel, task) = connect_test_db(Arc::new(TestEndpointDb::new())).await;
	begin(&mut client, 1, "slow", Some(15)).await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteBeginOk
	));
	tokio::time::sleep(Duration::from_millis(30)).await;
	for request_id in [2, 3] {
		send_request(
			&mut client,
			request_id,
			Some("slow"),
			wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
				sql: "SELECT 1".to_owned(),
				params: Vec::new(),
			}),
		)
		.await;
		let wire::ResponsePayload::LeaseExpired(error) = read_response(&mut client).await.payload
		else {
			panic!("expected lease expiry")
		};
		assert_eq!(error.timeout_ms, 15);
	}
	begin(&mut client, 4, "slow", None).await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::InvalidLeaseKey(_)
	));
	cancel.cancel();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn oversized_frame_goes_away_and_oversized_response_keeps_connection_open() {
	let (server, mut client) = UnixStream::pair().unwrap();
	let task = tokio::spawn(serve_connection(
		server,
		Arc::new(TestEndpointDb::new()),
		CancellationToken::new(),
		1024,
	));
	client.write_u32(1025).await.unwrap();
	let FrameRead::Payload(payload) = read_frame(&mut client, 1024).await.unwrap() else {
		panic!("expected go away")
	};
	assert!(matches!(
		wire::versioned::ServerFrame::deserialize_with_embedded_version(&payload).unwrap(),
		wire::ServerFrame::GoAway(wire::GoAway {
			reason: wire::GoAwayReason::FrameTooLarge
		})
	));
	task.await.unwrap().unwrap();

	let (mut server, mut client) = UnixStream::pair().unwrap();
	let write = tokio::spawn(async move {
		write_response(
			&mut server,
			1,
			wire::ResponsePayload::SqliteQueryOk(wire::SqliteQueryOk {
				columns: vec!["value".to_owned()],
				rows: vec![vec![wire::SqlValue::SqlBlob(vec![0; 4096])]],
				changes: 0,
				last_insert_row_id: None,
			}),
			wire::PROTOCOL_VERSION,
			256,
		)
		.await
		.unwrap();
		write_response(
			&mut server,
			2,
			wire::ResponsePayload::SqliteExecOk,
			wire::PROTOCOL_VERSION,
			256,
		)
		.await
		.unwrap();
	});
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::ResponseTooLarge
	));
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::SqliteExecOk
	));
	write.await.unwrap();

	let db = Arc::new(TestEndpointDb::new());
	let (server, mut client) = UnixStream::pair().unwrap();
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_connection(server, db, cancel.clone(), 256));
	send_hello(&mut client, wire::PROTOCOL_VERSION).await;
	let _ = read_hello(&mut client).await;
	send_request(
		&mut client,
		3,
		None,
		wire::RequestPayload::SqliteExec(wire::SqliteExec {
			script: "CREATE TABLE large_results(value BLOB)".to_owned(),
		}),
	)
	.await;
	let _ = read_response(&mut client).await;
	send_request(
		&mut client,
		4,
		None,
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "INSERT INTO large_results VALUES (zeroblob(4096)) RETURNING value".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	assert!(matches!(
		read_response(&mut client).await.payload,
		wire::ResponsePayload::ResponseTooLarge
	));
	send_request(
		&mut client,
		5,
		None,
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "SELECT count(*) FROM large_results".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	let wire::ResponsePayload::SqliteQueryOk(count) = read_response(&mut client).await.payload
	else {
		panic!("expected count after oversized response")
	};
	assert_eq!(count.rows, [vec![wire::SqlValue::SqlInteger(1)]]);
	cancel.cancel();
	drop(client);
	task.await.unwrap().unwrap();
}

#[tokio::test]
async fn shutdown_answers_inflight_request_and_listener_removes_private_socket() {
	let db = Arc::new(TestEndpointDb::new());
	let (mut owner, owner_cancel, owner_task) = connect_test_db(Arc::clone(&db)).await;
	let (server, mut blocked) = UnixStream::pair().unwrap();
	let blocked_cancel = CancellationToken::new();
	let blocked_task = tokio::spawn(serve_connection(
		server,
		db.clone(),
		blocked_cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	send_hello(&mut blocked, wire::PROTOCOL_VERSION).await;
	let _ = read_hello(&mut blocked).await;
	begin(&mut owner, 1, "owner", None).await;
	let _ = read_response(&mut owner).await;
	send_request(
		&mut owner,
		3,
		Some("owner"),
		wire::RequestPayload::SqliteExec(wire::SqliteExec {
			script: "CREATE TABLE disconnected(value INTEGER); INSERT INTO disconnected VALUES (1)"
				.to_owned(),
		}),
	)
	.await;
	let _ = read_response(&mut owner).await;
	send_request(
		&mut blocked,
		2,
		None,
		wire::RequestPayload::SqliteQuery(wire::SqliteQuery {
			sql: "SELECT 1".to_owned(),
			params: Vec::new(),
		}),
	)
	.await;
	tokio::time::sleep(Duration::from_millis(10)).await;
	blocked_cancel.cancel();
	assert!(matches!(
		read_response(&mut blocked).await.payload,
		wire::ResponsePayload::EndpointClosed
	));
	blocked_task.await.unwrap().unwrap();
	owner_cancel.cancel();
	drop(owner);
	owner_task.await.unwrap().unwrap();
	// Disconnect expiry rolled back the active transaction and released parked work.
	let table_count = db
		.connection
		.lock()
		.query_row(
			"SELECT count(*) FROM sqlite_master WHERE name = 'disconnected'",
			[],
			|row| row.get::<_, i64>(0),
		)
		.unwrap();
	assert_eq!(table_count, 0);

	let (listener, path) = bind_actor_socket().unwrap();
	assert_eq!(
		std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
		0o600
	);
	let cancel = CancellationToken::new();
	let task = tokio::spawn(serve_listener(
		listener,
		path.clone(),
		Arc::new(TestEndpointDb::new()),
		cancel.clone(),
		DEFAULT_MAX_FRAME_BYTES,
	));
	let _stalled = UnixStream::connect(&path).await.unwrap();
	cancel.cancel();
	tokio::time::timeout(Duration::from_secs(3), task)
		.await
		.expect("listener must drain stalled connections")
		.unwrap();
	assert!(!path.exists());
}

#[test]
fn process_directory_is_private() {
	let dir = process_socket_dir().unwrap();
	assert_eq!(
		std::fs::metadata(dir).unwrap().permissions().mode() & 0o777,
		0o700
	);
}

#[test]
fn process_directory_initialization_retries_after_failure() {
	let temp = tempfile::tempdir().unwrap();
	let invalid_parent = temp.path().join("not-a-directory");
	std::fs::write(&invalid_parent, b"file").unwrap();
	let cache = parking_lot::Mutex::new(None);

	initialize_process_socket_dir(&cache, &invalid_parent)
		.expect_err("an invalid runtime directory should fail");
	let initialized = initialize_process_socket_dir(&cache, temp.path())
		.expect("a later attempt with a valid runtime directory should succeed");
	assert!(initialized.is_dir());
	assert_eq!(
		initialize_process_socket_dir(&cache, &invalid_parent).unwrap(),
		initialized,
		"only the successful initialization should be cached"
	);
}

#[tokio::test]
async fn provisioning_is_opt_in_and_generation_terminal() {
	let endpoint = ActorRuntimeSocketEndpoint::new(false, SqliteDb::default());
	let error = endpoint.provision().await.unwrap_err();
	let structured = rivet_error::RivetError::extract(&error);
	assert_eq!(structured.group(), "actor_runtime_socket");
	assert_eq!(structured.code(), "not_enabled");

	let endpoint = ActorRuntimeSocketEndpoint::new(true, SqliteDb::default());
	let error = endpoint.provision().await.unwrap_err();
	let structured = rivet_error::RivetError::extract(&error);
	assert_eq!(structured.group(), "actor_runtime_socket");
	assert_eq!(structured.code(), "database_unavailable");
	endpoint.shutdown().await;
	let error = endpoint.provision().await.unwrap_err();
	let structured = rivet_error::RivetError::extract(&error);
	assert_eq!(structured.group(), "actor_runtime_socket");
	assert_eq!(structured.code(), "closed");
}

#[test]
fn coordinator_errors_keep_endpoint_level_meaning() {
	let wire::ResponsePayload::SqlError(error) = map_error(
		SqliteStatementError {
			code: 19,
			statement_index: 2,
			message: "constraint failed".to_owned(),
		}
		.into(),
	) else {
		panic!("expected SQLite statement error")
	};
	assert_eq!(error.code, 19);
	assert_eq!(error.statement_index, 2);
	assert!(matches!(
		map_error(TransactionQueueFullError.into()),
		wire::ResponsePayload::QueueFull(_)
	));
	assert!(matches!(
		map_error(
			TransactionClosedError {
				coordinator: true,
				key: None,
				state: None,
				message: "sqlite transaction coordinator is closed".to_string(),
			}
			.into()
		),
		wire::ResponsePayload::EndpointClosed
	));
	assert!(matches!(
		map_error(SqliteWorkerOverloadedError.into()),
		wire::ResponsePayload::QueueFull(_)
	));
	assert!(matches!(
		map_error(SqliteWorkerClosingError.into()),
		wire::ResponsePayload::EndpointClosed
	));
	assert!(matches!(
		map_error(
			ActorLifecycle::Overloaded {
				channel: "sqlite_worker".to_owned(),
				capacity: 1,
				operation: "execute".to_owned(),
			}
			.build()
		),
		wire::ResponsePayload::QueueFull(_)
	));
	assert!(matches!(
		map_error(SqliteRuntimeError::Closed.build()),
		wire::ResponsePayload::EndpointClosed
	));
	assert!(matches!(
		map_error(ActorRuntimeSocketClosedError.build()),
		wire::ResponsePayload::SqlError(_)
	));
	assert!(matches!(
		invalid_lease("unknown"),
		wire::ResponsePayload::InvalidLeaseKey(_)
	));
	assert!(matches!(
		map_error(anyhow!("unstructured failure")),
		wire::ResponsePayload::SqlError(_)
	));
}

#[test]
fn multi_statement_error_reports_actual_statement_index() {
	let connection = rusqlite::Connection::open_in_memory().unwrap();
	connection
		.execute_batch("CREATE TABLE unique_items(value INTEGER UNIQUE)")
		.unwrap();
	let error = unsafe {
		depot_client::query::exec_statements(
			connection.handle(),
			"INSERT INTO unique_items VALUES (1); INSERT INTO unique_items VALUES (1);",
		)
	}
	.expect_err("the second statement should violate the unique constraint");

	let wire::ResponsePayload::SqlError(error) = map_error(error) else {
		panic!("expected SQLite statement error");
	};
	assert_eq!(error.code, rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE);
	assert_eq!(error.statement_index, 1);
}
