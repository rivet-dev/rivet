use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

#[cfg(test)]
use anyhow::bail;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use depot_client::query::SqliteStatementError;
use depot_client::worker::{
	SQLITE_WORKER_QUEUE_CAPACITY, SqliteWorkerClosingError, SqliteWorkerOverloadedError,
};
use rivet_actor_runtime_socket_protocol as wire;
use rivet_error::RivetError;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vbare::OwnedVersionedData;

use crate::actor::sqlite::{
	BindParam, ColumnValue, ExecuteResult, SqliteBackend, SqliteDb, SqliteTransaction,
	TRANSACTION_COORDINATOR_QUEUE_CAPACITY, TransactionClosedError, TransactionExpiredError,
	TransactionInvalidArgumentError, TransactionQueueFullError, TransactionUnknownError,
};

const DEFAULT_MAX_FRAME_BYTES: u32 = 32 * 1024 * 1024;
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const LISTENER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(50);
const FRAME_QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorRuntimeSocketEndpointInfo {
	pub path: String,
}

#[derive(RivetError, Debug, Serialize)]
#[error(
	"actor_runtime_socket",
	"not_enabled",
	"Experimental Actor Runtime Socket is not enabled for this actor."
)]
pub struct ActorRuntimeSocketNotEnabledError;

#[derive(RivetError, Debug, Serialize)]
#[error(
	"actor_runtime_socket",
	"closed",
	"Actor Runtime Socket is closed for this actor generation."
)]
pub struct ActorRuntimeSocketClosedError;

#[derive(RivetError, Debug, Serialize)]
#[error(
	"actor_runtime_socket",
	"database_unavailable",
	"Actor Runtime Socket requires an enabled LocalNative SQLite database."
)]
pub struct ActorRuntimeSocketDatabaseUnavailableError;

#[derive(Clone)]
pub struct ActorRuntimeSocketEndpoint {
	inner: Arc<Inner>,
}

struct Inner {
	enabled: bool,
	db: SqliteDb,
	max_frame_bytes: u32,
	serving: Mutex<EndpointState>,
}

enum EndpointState {
	Unprovisioned,
	Serving(Serving),
	Closed,
}

struct Serving {
	info: ActorRuntimeSocketEndpointInfo,
	cancel: CancellationToken,
	task: JoinHandle<()>,
}

#[async_trait]
trait EndpointTransaction: Send + Sync {
	async fn exec(&self, sql: String) -> Result<()>;
	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult>;
	async fn commit(&self) -> Result<()>;
	async fn rollback(&self) -> Result<()>;
	async fn expire(&self) -> Result<()>;
}

struct CoreEndpointTransaction(SqliteTransaction);

#[async_trait]
impl EndpointTransaction for CoreEndpointTransaction {
	async fn exec(&self, sql: String) -> Result<()> {
		self.0.exec(sql).await.map(|_| ())
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.0.execute(sql, Some(params)).await
	}

	async fn commit(&self) -> Result<()> {
		self.0.commit().await.map(|_| ())
	}

	async fn rollback(&self) -> Result<()> {
		self.0.rollback().await
	}

	async fn expire(&self) -> Result<()> {
		self.0.expire().await
	}
}

#[async_trait]
trait EndpointDatabase: Send + Sync {
	async fn exec(&self, sql: String) -> Result<()>;
	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult>;
	async fn begin(
		&self,
		key: String,
		timeout: Option<Duration>,
	) -> Result<Arc<dyn EndpointTransaction>>;
}

#[async_trait]
impl EndpointDatabase for SqliteDb {
	async fn exec(&self, sql: String) -> Result<()> {
		self.exec(sql).await.map(|_| ())
	}

	async fn query(&self, sql: String, params: Vec<BindParam>) -> Result<ExecuteResult> {
		self.execute(sql, Some(params)).await
	}

	async fn begin(
		&self,
		key: String,
		timeout: Option<Duration>,
	) -> Result<Arc<dyn EndpointTransaction>> {
		Ok(Arc::new(CoreEndpointTransaction(
			self.begin_transaction_with_key(key, timeout).await?,
		)))
	}
}

impl ActorRuntimeSocketEndpoint {
	pub(crate) fn new(enabled: bool, db: SqliteDb) -> Self {
		Self {
			inner: Arc::new(Inner {
				enabled,
				db,
				max_frame_bytes: configured_max_frame_bytes(),
				serving: Mutex::new(EndpointState::Unprovisioned),
			}),
		}
	}

	pub async fn provision(&self) -> Result<ActorRuntimeSocketEndpointInfo> {
		if !self.inner.enabled {
			return Err(ActorRuntimeSocketNotEnabledError.build());
		}
		let mut serving = self.inner.serving.lock().await;
		match &*serving {
			EndpointState::Serving(serving) => return Ok(serving.info.clone()),
			EndpointState::Closed => return Err(ActorRuntimeSocketClosedError.build()),
			EndpointState::Unprovisioned => {}
		}
		if !self.inner.db.is_enabled() || self.inner.db.backend() != SqliteBackend::LocalNative {
			return Err(ActorRuntimeSocketDatabaseUnavailableError.build());
		}

		self.inner.db.open().await?;
		let (listener, socket_path) = bind_actor_socket()?;
		let info = ActorRuntimeSocketEndpointInfo {
			path: socket_path.to_string_lossy().into_owned(),
		};
		let cancel = CancellationToken::new();
		let task = tokio::spawn(serve_listener(
			listener,
			socket_path,
			Arc::new(self.inner.db.clone()),
			cancel.clone(),
			self.inner.max_frame_bytes,
		));
		*serving = EndpointState::Serving(Serving {
			info: info.clone(),
			cancel,
			task,
		});
		Ok(info)
	}

	pub(crate) async fn shutdown(&self) {
		let serving = {
			let mut state = self.inner.serving.lock().await;
			match std::mem::replace(&mut *state, EndpointState::Closed) {
				EndpointState::Serving(serving) => Some(serving),
				EndpointState::Unprovisioned | EndpointState::Closed => None,
			}
		};
		if let Some(mut serving) = serving {
			serving.cancel.cancel();
			// Unlink before waiting for the listener task so destroy cannot expose the
			// previous generation's socket path while shutdown is still joining it.
			if let Err(error) = std::fs::remove_file(&serving.info.path) {
				if error.kind() != io::ErrorKind::NotFound {
					tracing::warn!(path = %serving.info.path, %error, "failed to remove Actor Runtime Socket");
				}
			}
			match tokio::time::timeout(LISTENER_SHUTDOWN_TIMEOUT, &mut serving.task).await {
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					tracing::warn!(?error, "Actor Runtime Socket task failed during shutdown");
				}
				Err(_) => {
					serving.task.abort();
					let _ = serving.task.await;
				}
			}
		}
	}
}

fn bind_actor_socket() -> Result<(UnixListener, PathBuf)> {
	let socket_path = process_socket_dir()?.join(format!("{}.sock", short_random()));
	let listener = UnixListener::bind(&socket_path)
		.with_context(|| format!("bind Actor Runtime Socket at {}", socket_path.display()))?;
	std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
	Ok((listener, socket_path))
}

fn configured_max_frame_bytes() -> u32 {
	let Ok(value) = std::env::var("RIVET_ACTOR_RUNTIME_SOCKET_MAX_FRAME_BYTES") else {
		return DEFAULT_MAX_FRAME_BYTES;
	};
	match value.parse::<u32>() {
		Ok(value) if value >= 1024 => value,
		_ => {
			tracing::warn!(
				%value,
				default = DEFAULT_MAX_FRAME_BYTES,
				"invalid Actor Runtime Socket frame limit; using default"
			);
			DEFAULT_MAX_FRAME_BYTES
		}
	}
}

fn process_socket_dir() -> Result<PathBuf> {
	static DIR: OnceLock<parking_lot::Mutex<Option<PathBuf>>> = OnceLock::new();
	let parent = std::env::var_os("XDG_RUNTIME_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(std::env::temp_dir);
	initialize_process_socket_dir(DIR.get_or_init(Default::default), &parent)
}

fn initialize_process_socket_dir(
	cache: &parking_lot::Mutex<Option<PathBuf>>,
	parent: &Path,
) -> Result<PathBuf> {
	let mut cached = cache.lock();
	if let Some(dir) = &*cached {
		return Ok(dir.clone());
	}

	let dir = parent.join(format!("rivet-actor-runtime.{}", short_random()));
	std::fs::create_dir(&dir)
		.with_context(|| format!("create Actor Runtime Socket directory at {}", dir.display()))?;
	std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;

	// Do not cache initialization errors. A transiently unavailable runtime
	// directory must not disable provisioning for the rest of the process.
	*cached = Some(dir.clone());
	Ok(dir)
}

fn short_random() -> String {
	Uuid::new_v4().simple().to_string()[..12].to_owned()
}

async fn serve_listener(
	listener: UnixListener,
	path: PathBuf,
	db: Arc<dyn EndpointDatabase>,
	cancel: CancellationToken,
	max_frame_bytes: u32,
) {
	let mut connections = JoinSet::new();
	loop {
		tokio::select! {
			biased;
			_ = cancel.cancelled() => break,
			accepted = listener.accept() => match accepted {
				Ok((stream, _)) => {
					let db = Arc::clone(&db);
					let connection_cancel = cancel.child_token();
					connections.spawn(async move {
						if let Err(error) = serve_connection(stream, db, connection_cancel, max_frame_bytes).await {
							tracing::debug!(?error, "Actor Runtime Socket connection closed");
						}
					});
				}
				Err(error) => {
					tracing::warn!(%error, "Actor Runtime Socket accept failed; retrying");
					tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
				}
			},
			completed = connections.join_next(), if !connections.is_empty() => {
				if let Some(Err(error)) = completed {
					tracing::debug!(?error, "Actor Runtime Socket connection task failed");
				}
			}
		}
	}
	drop(listener);
	cancel.cancel();
	let drain = async { while connections.join_next().await.is_some() {} };
	if tokio::time::timeout(CONNECTION_DRAIN_TIMEOUT, drain)
		.await
		.is_err()
	{
		connections.abort_all();
		while connections.join_next().await.is_some() {}
	}
	if let Err(error) = std::fs::remove_file(&path) {
		if error.kind() != io::ErrorKind::NotFound {
			tracing::warn!(path = %path.display(), %error, "failed to remove Actor Runtime Socket");
		}
	}
}

enum LeaseEntry {
	Pending(Arc<Notify>),
	Active(Arc<dyn EndpointTransaction>),
	Terminal(&'static str),
}

#[cfg(test)]
#[derive(Default)]
struct PendingWaitTestHook {
	rechecked: Notify,
	resume: Notify,
}

#[derive(Default)]
struct ConnectionLeaseState {
	entries: BTreeMap<String, LeaseEntry>,
	closed: bool,
	#[cfg(test)]
	pending_wait_test_hook: Option<Arc<PendingWaitTestHook>>,
}

type ConnectionLeases = Arc<Mutex<ConnectionLeaseState>>;

struct ConnectionWriter {
	stream: tokio::net::unix::OwnedWriteHalf,
	terminal: bool,
}

type SharedConnectionWriter = Arc<Mutex<ConnectionWriter>>;
type RequestIds = Arc<Mutex<BTreeSet<u32>>>;

async fn serve_connection(
	mut stream: UnixStream,
	db: Arc<dyn EndpointDatabase>,
	cancel: CancellationToken,
	max_frame_bytes: u32,
) -> Result<()> {
	let handshake = tokio::select! {
		biased;
		_ = cancel.cancelled() => return Ok(()),
		handshake = tokio::time::timeout(DEFAULT_HANDSHAKE_TIMEOUT, read_frame(&mut stream, max_frame_bytes)) => handshake,
	};
	let payload = match handshake {
		Ok(Ok(FrameRead::Payload(payload))) => payload,
		Ok(Ok(FrameRead::Oversized)) => {
			write_go_away(
				&mut stream,
				wire::GoAwayReason::FrameTooLarge,
				wire::PROTOCOL_VERSION,
			)
			.await?;
			return Ok(());
		}
		_ => return Ok(()),
	};
	let Some(version) = embedded_version(&payload) else {
		return Ok(());
	};
	if version != wire::PROTOCOL_VERSION {
		write_server_hello(
			&mut stream,
			wire::ServerHello::HelloRejectUnsupportedVersion,
			wire::PROTOCOL_VERSION,
		)
		.await?;
		return Ok(());
	}
	if wire::versioned::ClientHello::deserialize_with_embedded_version(&payload).is_err() {
		return Ok(());
	}
	write_server_hello(
		&mut stream,
		wire::ServerHello::HelloOk(wire::HelloOk { max_frame_bytes }),
		version,
	)
	.await?;

	let connection_prefix = Arc::new(short_random());
	let (reader, writer) = stream.into_split();
	let (frame_tx, mut frame_rx) = mpsc::channel(FRAME_QUEUE_CAPACITY);
	let mut reader_task = tokio::spawn(read_frames(reader, frame_tx, max_frame_bytes));
	let writer = Arc::new(Mutex::new(ConnectionWriter {
		stream: writer,
		terminal: false,
	}));
	let leases: ConnectionLeases = Arc::new(Mutex::new(ConnectionLeaseState::default()));
	let request_ids: RequestIds = Arc::new(Mutex::new(BTreeSet::new()));
	let mut requests = JoinSet::new();
	let mut protocol_abort = false;

	loop {
		let frame = tokio::select! {
			biased;
			_ = cancel.cancelled() => break,
			joined = requests.join_next(), if !requests.is_empty() => {
				if let Some(Err(error)) = joined {
					tracing::warn!(?error, "Actor Runtime Socket request task failed");
					cancel.cancel();
					break;
				}
				continue;
			}
			frame = frame_rx.recv() => match frame {
				Some(frame) => frame,
				None => break,
			},
		};
		let payload = match frame {
			Ok(FrameRead::Payload(payload)) => payload,
			Ok(FrameRead::Oversized) => {
				let _ =
					write_connection_go_away(&writer, wire::GoAwayReason::FrameTooLarge, version)
						.await;
				protocol_abort = true;
				break;
			}
			Err(_) => break,
		};
		if embedded_version(&payload) != Some(version) {
			let _ = write_connection_go_away(&writer, wire::GoAwayReason::MalformedFrame, version)
				.await;
			protocol_abort = true;
			break;
		}
		let request =
			match wire::versioned::ClientFrame::deserialize_with_embedded_version(&payload) {
				Ok(wire::ClientFrame::Request(request)) => request,
				Err(_) => {
					let _ = write_connection_go_away(
						&writer,
						wire::GoAwayReason::MalformedFrame,
						version,
					)
					.await;
					protocol_abort = true;
					break;
				}
			};
		let request_id = request.request_id;
		{
			// Request admission and response completion take these locks in the
			// same order. Once a response is visible, its ID has been removed;
			// an immediate reuse therefore cannot race the old completion.
			let mut connection_writer = writer.lock().await;
			if !request_ids.lock().await.insert(request_id) {
				connection_writer.terminal = true;
				let _ = write_go_away(
					&mut connection_writer.stream,
					wire::GoAwayReason::MalformedFrame,
					version,
				)
				.await;
				protocol_abort = true;
				break;
			}
		}
		let preflight = preflight_request(&leases, &request).await;
		let db = Arc::clone(&db);
		let connection_prefix = Arc::clone(&connection_prefix);
		let leases = Arc::clone(&leases);
		let writer = Arc::clone(&writer);
		let request_ids = Arc::clone(&request_ids);
		let request_cancel = cancel.clone();
		requests.spawn(async move {
			// Keep coordinator lifecycle operations cancellation-safe. The response
			// can stop immediately, but BEGIN/COMMIT/ROLLBACK must finish even when
			// the connection closes after SQLite has started executing them.
			let operation_cancel = request_cancel.clone();
			let operation = tokio::spawn(async move {
				match preflight {
					Some(response) => response,
					None => {
						execute_request(
							db.as_ref(),
							&connection_prefix,
							&leases,
							&operation_cancel,
							request,
						)
						.await
					}
				}
			});
			let response = tokio::select! {
				biased;
				_ = request_cancel.cancelled() => wire::ResponsePayload::EndpointClosed,
				response = operation => response.unwrap_or(wire::ResponsePayload::EndpointClosed),
			};
			write_request_response(
				&writer,
				&request_ids,
				&request_cancel,
				request_id,
				response,
				version,
				max_frame_bytes,
			)
			.await;
		});
	}

	reader_task.abort();
	let _ = (&mut reader_task).await;
	if protocol_abort {
		requests.abort_all();
		while requests.join_next().await.is_some() {}
	}
	cancel.cancel();
	let active = close_connection_leases(&leases).await;
	for transaction in active {
		let _ = transaction.expire().await;
	}
	if !protocol_abort {
		let drain = async { while requests.join_next().await.is_some() {} };
		if tokio::time::timeout(CONNECTION_DRAIN_TIMEOUT, drain)
			.await
			.is_err()
		{
			requests.abort_all();
			while requests.join_next().await.is_some() {}
		}
	}
	let entries = std::mem::take(&mut leases.lock().await.entries);
	for entry in entries.into_values() {
		if let LeaseEntry::Active(transaction) = entry {
			let _ = transaction.expire().await;
		}
	}
	Ok(())
}

async fn preflight_request(
	leases: &ConnectionLeases,
	request: &wire::Request,
) -> Option<wire::ResponsePayload> {
	let wire::RequestPayload::SqliteBegin(begin) = &request.payload else {
		return None;
	};
	if request.lease_key.is_some() || begin.lease_key.is_empty() {
		return Some(invalid_lease(
			"begin requires a non-empty payload leaseKey and no Request.leaseKey",
		));
	}
	let mut state = leases.lock().await;
	if state.closed {
		return Some(wire::ResponsePayload::EndpointClosed);
	}
	if state.entries.contains_key(&begin.lease_key) {
		return Some(invalid_lease(
			"leaseKey was already used on this connection",
		));
	}
	state.entries.insert(
		begin.lease_key.clone(),
		LeaseEntry::Pending(Arc::new(Notify::new())),
	);
	None
}

async fn write_request_response(
	writer: &SharedConnectionWriter,
	request_ids: &RequestIds,
	cancel: &CancellationToken,
	request_id: u32,
	response: wire::ResponsePayload,
	version: u16,
	max_frame_bytes: u32,
) {
	let mut connection_writer = writer.lock().await;
	if connection_writer.terminal {
		request_ids.lock().await.remove(&request_id);
		return;
	}
	let response = if cancel.is_cancelled() {
		wire::ResponsePayload::EndpointClosed
	} else {
		response
	};
	// Once a frame owns the writer, finish it without a cancellation branch.
	// Connection shutdown has its own bounded drain; aborting every request at
	// that boundary closes the stream instead of appending after a partial frame.
	let write_failed = write_response(
		&mut connection_writer.stream,
		request_id,
		response,
		version,
		max_frame_bytes,
	)
	.await
	.is_err();
	// Keep the writer locked until the ID is released. Admission uses the same
	// writer -> request_ids lock order, which establishes the visibility edge
	// needed for clients to reuse an ID immediately after reading its response.
	request_ids.lock().await.remove(&request_id);
	if write_failed {
		cancel.cancel();
	}
}

async fn write_connection_go_away(
	writer: &SharedConnectionWriter,
	reason: wire::GoAwayReason,
	version: u16,
) -> Result<()> {
	let mut connection_writer = writer.lock().await;
	if connection_writer.terminal {
		return Ok(());
	}

	// Mark the connection terminal before the write. Any response already holding
	// the writer may finish before GoAway; no later response may follow it.
	connection_writer.terminal = true;
	write_go_away(&mut connection_writer.stream, reason, version).await
}

async fn read_frames(
	mut reader: tokio::net::unix::OwnedReadHalf,
	frames: mpsc::Sender<Result<FrameRead>>,
	max_frame_bytes: u32,
) {
	loop {
		let frame = read_frame(&mut reader, max_frame_bytes).await;
		let terminal = frame.is_err() || matches!(frame, Ok(FrameRead::Oversized));
		if frames.send(frame).await.is_err() || terminal {
			break;
		}
	}
}

async fn execute_request(
	db: &dyn EndpointDatabase,
	connection_prefix: &str,
	leases: &ConnectionLeases,
	cancel: &CancellationToken,
	request: wire::Request,
) -> wire::ResponsePayload {
	let result = match request.payload {
		wire::RequestPayload::SqliteExec(exec) => {
			match transaction_for_request(leases, request.lease_key.as_deref()).await {
				Ok(Some(transaction)) => transaction.exec(exec.script).await,
				Ok(None) => db.exec(exec.script).await,
				Err(error) => return error,
			}
			.map(|_| wire::ResponsePayload::SqliteExecOk)
		}
		wire::RequestPayload::SqliteQuery(query) => {
			let params = query.params.into_iter().map(bind_param).collect();
			let result = match transaction_for_request(leases, request.lease_key.as_deref()).await {
				Ok(Some(transaction)) => transaction.query(query.sql, params).await,
				Ok(None) => db.query(query.sql, params).await,
				Err(error) => return error,
			};
			result.map(query_response)
		}
		wire::RequestPayload::SqliteBegin(begin) => {
			let core_key = format!("{connection_prefix}:{}", begin.lease_key);
			match db
				.begin(core_key, begin.timeout_ms.map(Duration::from_millis))
				.await
			{
				Ok(transaction) => {
					let connection_closed =
						complete_begin(leases, begin.lease_key, Arc::clone(&transaction)).await;
					if connection_closed || cancel.is_cancelled() {
						let _ = transaction.expire().await;
						return wire::ResponsePayload::EndpointClosed;
					}
					Ok(wire::ResponsePayload::SqliteBeginOk)
				}
				Err(error) => {
					remove_pending_begin(leases, &begin.lease_key).await;
					Err(error)
				}
			}
		}
		wire::RequestPayload::SqliteCommit(commit) => {
			if request.lease_key.is_some() {
				return invalid_lease("commit carries leaseKey only in its payload");
			}
			let transaction = match active_transaction(leases, &commit.lease_key).await {
				Ok(transaction) => transaction,
				Err(error) => return error,
			};
			match transaction.commit().await {
				Ok(()) => {
					leases
						.lock()
						.await
						.entries
						.insert(commit.lease_key, LeaseEntry::Terminal("committed"));
					Ok(wire::ResponsePayload::SqliteCommitOk)
				}
				Err(error) => Err(error),
			}
		}
		wire::RequestPayload::SqliteRollback(rollback) => {
			if request.lease_key.is_some() {
				return invalid_lease("rollback carries leaseKey only in its payload");
			}
			let transaction = match active_transaction(leases, &rollback.lease_key).await {
				Ok(transaction) => transaction,
				Err(error) => return error,
			};
			match transaction.rollback().await {
				Ok(()) => {
					leases
						.lock()
						.await
						.entries
						.insert(rollback.lease_key, LeaseEntry::Terminal("rolled back"));
					Ok(wire::ResponsePayload::SqliteRollbackOk)
				}
				Err(error) => Err(error),
			}
		}
	};
	match result {
		Ok(response) => response,
		Err(error) => map_error(error),
	}
}

async fn complete_begin(
	leases: &ConnectionLeases,
	key: String,
	transaction: Arc<dyn EndpointTransaction>,
) -> bool {
	let mut state = leases.lock().await;
	let notify = match state.entries.get(&key) {
		Some(LeaseEntry::Pending(notify)) => Some(Arc::clone(notify)),
		_ => None,
	};
	let closed = state.closed;
	state.entries.insert(
		key,
		if closed {
			LeaseEntry::Terminal("expired after connection closed")
		} else {
			LeaseEntry::Active(transaction)
		},
	);
	drop(state);
	if let Some(notify) = notify {
		notify.notify_waiters();
	}
	closed
}

async fn close_connection_leases(leases: &ConnectionLeases) -> Vec<Arc<dyn EndpointTransaction>> {
	let mut state = leases.lock().await;
	state.closed = true;
	let mut pending = Vec::new();
	let mut active = Vec::new();
	for entry in state.entries.values_mut() {
		match entry {
			LeaseEntry::Pending(notify) => {
				pending.push(Arc::clone(notify));
				*entry = LeaseEntry::Terminal("expired after connection closed");
			}
			LeaseEntry::Active(transaction) => active.push(Arc::clone(transaction)),
			LeaseEntry::Terminal(_) => {}
		}
	}
	drop(state);
	for notify in pending {
		notify.notify_waiters();
	}
	active
}

async fn remove_pending_begin(leases: &ConnectionLeases, key: &str) {
	let mut state = leases.lock().await;
	let notify = match state.entries.remove(key) {
		Some(LeaseEntry::Pending(notify)) => Some(notify),
		Some(entry) => {
			state.entries.insert(key.to_owned(), entry);
			None
		}
		None => None,
	};
	drop(state);
	if let Some(notify) = notify {
		notify.notify_waiters();
	}
}

async fn transaction_for_request(
	leases: &ConnectionLeases,
	key: Option<&str>,
) -> std::result::Result<Option<Arc<dyn EndpointTransaction>>, wire::ResponsePayload> {
	match key {
		Some(key) => active_transaction(leases, key).await.map(Some),
		None => Ok(None),
	}
}

async fn active_transaction(
	leases: &ConnectionLeases,
	key: &str,
) -> std::result::Result<Arc<dyn EndpointTransaction>, wire::ResponsePayload> {
	loop {
		let notify = {
			let state = leases.lock().await;
			if state.closed {
				return Err(wire::ResponsePayload::EndpointClosed);
			}
			match state.entries.get(key) {
				Some(LeaseEntry::Pending(notify)) => Arc::clone(notify),
				Some(LeaseEntry::Active(transaction)) => {
					return Ok(Arc::clone(transaction));
				}
				Some(LeaseEntry::Terminal(state)) => {
					return Err(invalid_lease(&format!("leaseKey is already {state}")));
				}
				None => return Err(invalid_lease("leaseKey is unknown on this connection")),
			}
		};
		let notified = notify.notified();
		tokio::pin!(notified);
		// `Notify::notify_waiters` does not retain a permit for a future that has
		// merely been constructed. Register this waiter before the state recheck so
		// BEGIN completion cannot land between the recheck and the first poll of
		// `notified`, which would otherwise park this request forever.
		notified.as_mut().enable();
		let still_pending = {
			let state = leases.lock().await;
			matches!(
				state.entries.get(key),
				Some(LeaseEntry::Pending(current)) if Arc::ptr_eq(current, &notify)
			)
		};
		#[cfg(test)]
		if still_pending {
			let hook = leases.lock().await.pending_wait_test_hook.clone();
			if let Some(hook) = hook {
				hook.rechecked.notify_one();
				hook.resume.notified().await;
			}
		}
		if still_pending {
			notified.await;
		}
	}
}

fn query_response(result: ExecuteResult) -> wire::ResponsePayload {
	wire::ResponsePayload::SqliteQueryOk(wire::SqliteQueryOk {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(sql_value).collect())
			.collect(),
		changes: result.changes,
		last_insert_row_id: result.last_insert_row_id,
	})
}

fn invalid_lease(message: &str) -> wire::ResponsePayload {
	wire::ResponsePayload::InvalidLeaseKey(wire::InvalidLeaseKey {
		message: message.to_owned(),
	})
}

fn bind_param(value: wire::SqlValue) -> BindParam {
	match value {
		wire::SqlValue::SqlNull => BindParam::Null,
		wire::SqlValue::SqlInteger(value) => BindParam::Integer(value),
		wire::SqlValue::SqlReal(value) => BindParam::Float(value),
		wire::SqlValue::SqlText(value) => BindParam::Text(value),
		wire::SqlValue::SqlBlob(value) => BindParam::Blob(value),
	}
}

fn sql_value(value: ColumnValue) -> wire::SqlValue {
	match value {
		ColumnValue::Null => wire::SqlValue::SqlNull,
		ColumnValue::Integer(value) => wire::SqlValue::SqlInteger(value),
		ColumnValue::Float(value) => wire::SqlValue::SqlReal(value),
		ColumnValue::Text(value) => wire::SqlValue::SqlText(value),
		ColumnValue::Blob(value) => wire::SqlValue::SqlBlob(value),
	}
}

fn map_error(error: anyhow::Error) -> wire::ResponsePayload {
	if let Some(error) = error.downcast_ref::<SqliteStatementError>() {
		return wire::ResponsePayload::SqlError(wire::SqlError {
			code: error.code,
			statement_index: error.statement_index,
			message: error.message.clone(),
		});
	}
	if error.downcast_ref::<TransactionQueueFullError>().is_some() {
		return wire::ResponsePayload::QueueFull(wire::QueueFull {
			limit: "transactionCoordinatorQueue".to_owned(),
			capacity: TRANSACTION_COORDINATOR_QUEUE_CAPACITY as u32,
		});
	}
	if let Some(error) = error.downcast_ref::<TransactionInvalidArgumentError>() {
		return wire::ResponsePayload::SqlError(wire::SqlError {
			code: -1,
			statement_index: 0,
			message: error.to_string(),
		});
	}
	if let Some(error) = error.downcast_ref::<TransactionExpiredError>() {
		return wire::ResponsePayload::LeaseExpired(wire::LeaseExpired {
			timeout_ms: error.timeout_ms,
			message: format!(
				"SQLite transaction expired after {} ms and was rolled back. This timeout is a deadlock-safety backstop; increase SqliteBegin.timeoutMs if the transaction legitimately needs longer, and check for a nested transaction or outer database call waiting behind this lease.",
				error.timeout_ms
			),
		});
	}
	if let Some(error) = error.downcast_ref::<TransactionUnknownError>() {
		return invalid_lease(&error.to_string());
	}
	if let Some(error) = error.downcast_ref::<TransactionClosedError>() {
		return if error.coordinator {
			wire::ResponsePayload::EndpointClosed
		} else {
			invalid_lease(&error.to_string())
		};
	}
	if error
		.downcast_ref::<SqliteWorkerOverloadedError>()
		.is_some()
	{
		return wire::ResponsePayload::QueueFull(wire::QueueFull {
			limit: "sqliteWorkerQueue".to_owned(),
			capacity: SQLITE_WORKER_QUEUE_CAPACITY as u32,
		});
	}
	if error.downcast_ref::<SqliteWorkerClosingError>().is_some() {
		return wire::ResponsePayload::EndpointClosed;
	}

	let structured = rivet_error::RivetError::extract(&error);
	match (structured.group(), structured.code()) {
		("actor", "overloaded") => wire::ResponsePayload::QueueFull(wire::QueueFull {
			limit: "sqliteWorkerQueue".to_owned(),
			capacity: SQLITE_WORKER_QUEUE_CAPACITY as u32,
		}),
		("sqlite", "closed" | "transaction_closed") => wire::ResponsePayload::EndpointClosed,
		_ => wire::ResponsePayload::SqlError(wire::SqlError {
			code: -1,
			statement_index: 0,
			message: structured.message().to_owned(),
		}),
	}
}

enum FrameRead {
	Payload(Vec<u8>),
	Oversized,
}

async fn read_frame<R: AsyncRead + Unpin>(
	reader: &mut R,
	max_frame_bytes: u32,
) -> Result<FrameRead> {
	let length = reader.read_u32().await?;
	if length > max_frame_bytes {
		return Ok(FrameRead::Oversized);
	}
	let mut payload = vec![0; length as usize];
	reader.read_exact(&mut payload).await?;
	Ok(FrameRead::Payload(payload))
}

fn embedded_version(payload: &[u8]) -> Option<u16> {
	payload
		.get(..2)
		.map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
}

async fn write_server_hello<W: AsyncWrite + Unpin>(
	writer: &mut W,
	hello: wire::ServerHello,
	version: u16,
) -> Result<()> {
	let payload = wire::versioned::ServerHello::wrap_latest(hello)
		.serialize_with_embedded_version(version)?;
	write_payload(writer, &payload).await
}

async fn write_go_away<W: AsyncWrite + Unpin>(
	writer: &mut W,
	reason: wire::GoAwayReason,
	version: u16,
) -> Result<()> {
	let payload =
		wire::versioned::ServerFrame::wrap_latest(wire::ServerFrame::GoAway(wire::GoAway {
			reason,
		}))
		.serialize_with_embedded_version(version)?;
	write_payload(writer, &payload).await
}

async fn write_response<W: AsyncWrite + Unpin>(
	writer: &mut W,
	request_id: u32,
	response: wire::ResponsePayload,
	version: u16,
	max_frame_bytes: u32,
) -> Result<()> {
	let frame = |payload| {
		wire::versioned::ServerFrame::wrap_latest(wire::ServerFrame::Response(wire::Response {
			request_id,
			payload,
		}))
		.serialize_with_embedded_version(version)
	};
	let mut payload = frame(response)?;
	if payload.len() > max_frame_bytes as usize {
		payload = frame(wire::ResponsePayload::ResponseTooLarge)?;
	}
	write_payload(writer, &payload).await
}

async fn write_payload<W: AsyncWrite + Unpin>(writer: &mut W, payload: &[u8]) -> Result<()> {
	let length = u32::try_from(payload.len())
		.map_err(|_| anyhow!("Actor Runtime Socket frame is too large"))?;
	writer.write_u32(length).await?;
	writer.write_all(payload).await?;
	writer.flush().await?;
	Ok(())
}

#[cfg(test)]
#[path = "../../tests/actor_runtime_socket.rs"]
mod tests;
