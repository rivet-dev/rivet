use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use libsqlite3_sys::*;
use moka::sync::Cache;
use parking_lot::{Mutex, RwLock};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
#[cfg(test)]
use sqlite_storage::{engine::SqliteEngine, error::SqliteStorageError};
use tokio::runtime::Handle;
#[cfg(test)]
use tokio::sync::Notify;

const DEFAULT_CACHE_CAPACITY_PAGES: u64 = 50_000;
const DEFAULT_PREFETCH_DEPTH: usize = 16;
const DEFAULT_MAX_PREFETCH_BYTES: usize = 256 * 1024;
const DEFAULT_MAX_PAGES_PER_STAGE: usize = 4_000;
const DEFAULT_PAGE_SIZE: usize = 4096;
const MAX_PATHNAME: c_int = 64;
const TEMP_AUX_PATH_PREFIX: &str = "__sqlite_v2_temp__";
const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

static NEXT_STAGE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_AUX_ID: AtomicU64 = AtomicU64::new(1);

fn empty_db_page() -> Vec<u8> {
	let mut page = vec![0u8; DEFAULT_PAGE_SIZE];
	page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()].copy_from_slice(&EMPTY_DB_PAGE_HEADER_PREFIX);
	page
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
	if let Some(message) = payload.downcast_ref::<&str>() {
		message.to_string()
	} else if let Some(message) = payload.downcast_ref::<String>() {
		message.clone()
	} else {
		"unknown panic".to_string()
	}
}

macro_rules! vfs_catch_unwind {
	($err_val:expr, $body:expr) => {
		match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
			Ok(result) => result,
			Err(panic) => {
				tracing::error!(
					message = panic_message(&panic),
					"sqlite v2 callback panicked"
				);
				$err_val
			}
		}
	};
}

#[derive(Clone)]
struct SqliteTransport {
	inner: Arc<SqliteTransportInner>,
}

enum SqliteTransportInner {
	Envoy(EnvoyHandle),
	#[cfg(test)]
	Direct {
		engine: Arc<SqliteEngine>,
		hooks: Arc<DirectTransportHooks>,
	},
	#[cfg(test)]
	Test(Arc<MockProtocol>),
}

impl SqliteTransport {
	fn from_envoy(handle: EnvoyHandle) -> Self {
		Self {
			inner: Arc::new(SqliteTransportInner::Envoy(handle)),
		}
	}

	#[cfg(test)]
	fn from_direct(engine: Arc<SqliteEngine>) -> Self {
		Self {
			inner: Arc::new(SqliteTransportInner::Direct {
				engine,
				hooks: Arc::new(DirectTransportHooks::default()),
			}),
		}
	}

	#[cfg(test)]
	fn from_mock(protocol: Arc<MockProtocol>) -> Self {
		Self {
			inner: Arc::new(SqliteTransportInner::Test(protocol)),
		}
	}

	#[cfg(test)]
	fn direct_hooks(&self) -> Option<Arc<DirectTransportHooks>> {
		match &*self.inner {
			SqliteTransportInner::Direct { hooks, .. } => Some(Arc::clone(hooks)),
			_ => None,
		}
	}

	async fn get_pages(
		&self,
		req: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_get_pages(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, .. } => {
				let pgnos = req.pgnos.clone();
				match engine.get_pages(&req.actor_id, req.generation, pgnos).await {
					Ok(pages) => Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
						protocol::SqliteGetPagesOk {
							pages: pages.into_iter().map(protocol_fetched_page).collect(),
							meta: protocol_sqlite_meta(engine.load_meta(&req.actor_id).await?),
						},
					)),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteGetPagesResponse::SqliteFenceMismatch(
								protocol::SqliteFenceMismatch {
									actual_meta: protocol_sqlite_meta(
										engine.load_meta(&req.actor_id).await?,
									),
									reason: reason.clone(),
								},
							))
						} else if matches!(
							sqlite_storage_error(&err),
							Some(SqliteStorageError::MetaMissing { operation })
								if *operation == "get_pages" && req.generation == 1
						) {
							match engine
								.takeover(
									&req.actor_id,
									sqlite_storage::takeover::TakeoverConfig::new(1),
								)
								.await
							{
								Ok(_) => {}
								Err(takeover_err)
									if matches!(
										sqlite_storage_error(&takeover_err),
										Some(SqliteStorageError::ConcurrentTakeover)
									) => {}
								Err(takeover_err) => {
									return Ok(
										protocol::SqliteGetPagesResponse::SqliteErrorResponse(
											sqlite_error_response(&takeover_err),
										),
									);
								}
							}

							match engine
								.get_pages(&req.actor_id, req.generation, req.pgnos)
								.await
							{
								Ok(pages) => {
									Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
										protocol::SqliteGetPagesOk {
											pages: pages
												.into_iter()
												.map(protocol_fetched_page)
												.collect(),
											meta: protocol_sqlite_meta(
												engine.load_meta(&req.actor_id).await?,
											),
										},
									))
								}
								Err(retry_err) => {
									Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
										sqlite_error_response(&retry_err),
									))
								}
							}
						} else {
							Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
								sqlite_error_response(&err),
							))
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.get_pages(req).await,
		}
	}

	async fn commit(
		&self,
		req: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_commit(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, hooks } => {
				if let Some(message) = hooks.take_commit_error() {
					return Err(anyhow::anyhow!(message));
				}

				match engine
					.commit(
						&req.actor_id,
						sqlite_storage::commit::CommitRequest {
							generation: req.generation,
							head_txid: req.expected_head_txid,
							db_size_pages: req.new_db_size_pages,
							dirty_pages: req
								.dirty_pages
								.into_iter()
								.map(storage_dirty_page)
								.collect(),
							now_ms: sqlite_now_ms()?,
						},
					)
					.await
				{
					Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
						protocol::SqliteCommitOk {
							new_head_txid: result.txid,
							meta: protocol_sqlite_meta(result.meta),
						},
					)),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteCommitResponse::SqliteFenceMismatch(
								protocol::SqliteFenceMismatch {
									actual_meta: protocol_sqlite_meta(
										engine.load_meta(&req.actor_id).await?,
									),
									reason: reason.clone(),
								},
							))
						} else if let Some(SqliteStorageError::CommitTooLarge {
							actual_size_bytes,
							max_size_bytes,
						}) = sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteCommitResponse::SqliteCommitTooLarge(
								protocol::SqliteCommitTooLarge {
									actual_size_bytes: *actual_size_bytes,
									max_size_bytes: *max_size_bytes,
								},
							))
						} else {
							Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
								sqlite_error_response(&err),
							))
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.commit(req).await,
		}
	}

	async fn commit_stage(
		&self,
		req: protocol::SqliteCommitStageRequest,
	) -> Result<protocol::SqliteCommitStageResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_commit_stage(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, .. } => {
				match engine
					.commit_stage(
						&req.actor_id,
						sqlite_storage::commit::CommitStageRequest {
							generation: req.generation,
							stage_id: req.stage_id,
							chunk_idx: req.chunk_idx,
							dirty_pages: req
								.dirty_pages
								.into_iter()
								.map(storage_dirty_page)
								.collect(),
							is_last: req.is_last,
						},
					)
					.await
				{
					Ok(result) => Ok(protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
						protocol::SqliteCommitStageOk {
							chunk_idx_committed: result.chunk_idx_committed,
						},
					)),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteCommitStageResponse::SqliteFenceMismatch(
								protocol::SqliteFenceMismatch {
									actual_meta: protocol_sqlite_meta(
										engine.load_meta(&req.actor_id).await?,
									),
									reason: reason.clone(),
								},
							))
						} else {
							Ok(protocol::SqliteCommitStageResponse::SqliteErrorResponse(
								sqlite_error_response(&err),
							))
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.commit_stage(req).await,
		}
	}

	fn queue_commit_stage(&self, req: protocol::SqliteCommitStageRequest) -> Result<bool> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => {
				handle.sqlite_commit_stage_fire_and_forget(req)?;
				Ok(true)
			}
			#[cfg(test)]
			SqliteTransportInner::Direct { .. } => Ok(false),
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => {
				protocol.queue_commit_stage(req);
				Ok(true)
			}
		}
	}

	async fn commit_finalize(
		&self,
		req: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_commit_finalize(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, .. } => {
				match engine
					.commit_finalize(
						&req.actor_id,
						sqlite_storage::commit::CommitFinalizeRequest {
							generation: req.generation,
							expected_head_txid: req.expected_head_txid,
							stage_id: req.stage_id,
							new_db_size_pages: req.new_db_size_pages,
							now_ms: sqlite_now_ms()?,
						},
					)
					.await
				{
					Ok(result) => Ok(
						protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
							protocol::SqliteCommitFinalizeOk {
								new_head_txid: result.new_head_txid,
								meta: protocol_sqlite_meta(result.meta),
							},
						),
					),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteCommitFinalizeResponse::SqliteFenceMismatch(
								protocol::SqliteFenceMismatch {
									actual_meta: protocol_sqlite_meta(
										engine.load_meta(&req.actor_id).await?,
									),
									reason: reason.clone(),
								},
							))
						} else if let Some(SqliteStorageError::StageNotFound { stage_id }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteCommitFinalizeResponse::SqliteStageNotFound(
								protocol::SqliteStageNotFound {
									stage_id: *stage_id,
								},
							))
						} else {
							Ok(protocol::SqliteCommitFinalizeResponse::SqliteErrorResponse(
								sqlite_error_response(&err),
							))
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.commit_finalize(req).await,
		}
	}
}

#[cfg(test)]
#[derive(Default)]
struct DirectTransportHooks {
	fail_next_commit: Mutex<Option<String>>,
}

#[cfg(test)]
impl DirectTransportHooks {
	fn fail_next_commit(&self, message: impl Into<String>) {
		*self.fail_next_commit.lock() = Some(message.into());
	}

	fn take_commit_error(&self) -> Option<String> {
		self.fail_next_commit.lock().take()
	}
}

#[cfg(test)]
fn protocol_sqlite_meta(meta: sqlite_storage::types::SqliteMeta) -> protocol::SqliteMeta {
	protocol::SqliteMeta {
		schema_version: meta.schema_version,
		generation: meta.generation,
		head_txid: meta.head_txid,
		materialized_txid: meta.materialized_txid,
		db_size_pages: meta.db_size_pages,
		page_size: meta.page_size,
		creation_ts_ms: meta.creation_ts_ms,
		max_delta_bytes: meta.max_delta_bytes,
	}
}

#[cfg(test)]
fn protocol_fetched_page(page: sqlite_storage::types::FetchedPage) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

#[cfg(test)]
fn storage_dirty_page(page: protocol::SqliteDirtyPage) -> sqlite_storage::types::DirtyPage {
	sqlite_storage::types::DirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

#[cfg(test)]
fn sqlite_storage_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
	err.downcast_ref::<SqliteStorageError>()
}

#[cfg(test)]
fn sqlite_error_reason(err: &anyhow::Error) -> String {
	err.chain()
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(": ")
}

#[cfg(test)]
fn sqlite_error_response(err: &anyhow::Error) -> protocol::SqliteErrorResponse {
	protocol::SqliteErrorResponse {
		message: sqlite_error_reason(err),
	}
}

#[cfg(test)]
fn sqlite_now_ms() -> Result<i64> {
	use std::time::{SystemTime, UNIX_EPOCH};

	Ok(SystemTime::now()
		.duration_since(UNIX_EPOCH)?
		.as_millis()
		.try_into()?)
}

#[cfg(test)]
struct MockProtocol {
	commit_response: protocol::SqliteCommitResponse,
	stage_response: protocol::SqliteCommitStageResponse,
	finalize_response: protocol::SqliteCommitFinalizeResponse,
	get_pages_response: protocol::SqliteGetPagesResponse,
	mirror_commit_meta: Mutex<bool>,
	commit_requests: Mutex<Vec<protocol::SqliteCommitRequest>>,
	stage_requests: Mutex<Vec<protocol::SqliteCommitStageRequest>>,
	awaited_stage_responses: Mutex<usize>,
	finalize_requests: Mutex<Vec<protocol::SqliteCommitFinalizeRequest>>,
	get_pages_requests: Mutex<Vec<protocol::SqliteGetPagesRequest>>,
	finalize_started: Notify,
	release_finalize: Notify,
}

#[cfg(test)]
impl MockProtocol {
	fn new(
		commit_response: protocol::SqliteCommitResponse,
		stage_response: protocol::SqliteCommitStageResponse,
		finalize_response: protocol::SqliteCommitFinalizeResponse,
	) -> Self {
		Self {
			commit_response,
			stage_response,
			finalize_response,
			get_pages_response: protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
				protocol::SqliteGetPagesOk {
					pages: vec![],
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
			mirror_commit_meta: Mutex::new(false),
			commit_requests: Mutex::new(Vec::new()),
			stage_requests: Mutex::new(Vec::new()),
			awaited_stage_responses: Mutex::new(0),
			finalize_requests: Mutex::new(Vec::new()),
			get_pages_requests: Mutex::new(Vec::new()),
			finalize_started: Notify::new(),
			release_finalize: Notify::new(),
		}
	}

	fn commit_requests(&self) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitRequest>> {
		self.commit_requests.lock()
	}

	fn stage_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStageRequest>> {
		self.stage_requests.lock()
	}

	fn awaited_stage_responses(&self) -> usize {
		*self.awaited_stage_responses.lock()
	}

	fn finalize_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitFinalizeRequest>> {
		self.finalize_requests.lock()
	}

	fn get_pages_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteGetPagesRequest>> {
		self.get_pages_requests.lock()
	}

	fn set_mirror_commit_meta(&self, enabled: bool) {
		*self.mirror_commit_meta.lock() = enabled;
	}

	fn queue_commit_stage(&self, req: protocol::SqliteCommitStageRequest) {
		self.stage_requests().push(req);
	}

	async fn get_pages(
		&self,
		req: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.get_pages_requests().push(req);
		Ok(self.get_pages_response.clone())
	}

	async fn commit(
		&self,
		req: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		let req = req.clone();
		self.commit_requests().push(req.clone());
		if *self.mirror_commit_meta.lock() {
			if let protocol::SqliteCommitResponse::SqliteCommitOk(ok) = &self.commit_response {
				let mut meta = ok.meta.clone();
				meta.head_txid = req.expected_head_txid + 1;
				meta.db_size_pages = req.new_db_size_pages;
				return Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
					protocol::SqliteCommitOk {
						new_head_txid: req.expected_head_txid + 1,
						meta,
					},
				));
			}
		}
		Ok(self.commit_response.clone())
	}

	async fn commit_stage(
		&self,
		req: protocol::SqliteCommitStageRequest,
	) -> Result<protocol::SqliteCommitStageResponse> {
		*self.awaited_stage_responses.lock() += 1;
		self.stage_requests().push(req);
		Ok(self.stage_response.clone())
	}

	async fn commit_finalize(
		&self,
		req: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		let req = req.clone();
		self.finalize_requests().push(req.clone());
		self.finalize_started.notify_one();
		self.release_finalize.notified().await;
		if *self.mirror_commit_meta.lock() {
			if let protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) =
				&self.finalize_response
			{
				let mut meta = ok.meta.clone();
				meta.head_txid = req.expected_head_txid + 1;
				meta.db_size_pages = req.new_db_size_pages;
				return Ok(
					protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
						protocol::SqliteCommitFinalizeOk {
							new_head_txid: req.expected_head_txid + 1,
							meta,
						},
					),
				);
			}
		}
		Ok(self.finalize_response.clone())
	}
}

#[cfg(test)]
fn sqlite_meta(max_delta_bytes: u64) -> protocol::SqliteMeta {
	protocol::SqliteMeta {
		schema_version: 2,
		generation: 7,
		head_txid: 12,
		materialized_txid: 12,
		db_size_pages: 1,
		page_size: 4096,
		creation_ts_ms: 1_700_000_000_000,
		max_delta_bytes,
	}
}

#[derive(Debug, Clone)]
pub struct VfsV2Config {
	pub cache_capacity_pages: u64,
	pub prefetch_depth: usize,
	pub max_prefetch_bytes: usize,
	pub max_pages_per_stage: usize,
}

impl Default for VfsV2Config {
	fn default() -> Self {
		Self {
			cache_capacity_pages: DEFAULT_CACHE_CAPACITY_PAGES,
			prefetch_depth: DEFAULT_PREFETCH_DEPTH,
			max_prefetch_bytes: DEFAULT_MAX_PREFETCH_BYTES,
			max_pages_per_stage: DEFAULT_MAX_PAGES_PER_STAGE,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitPath {
	Fast,
	Slow,
}

#[derive(Debug, Clone)]
pub struct BufferedCommitRequest {
	pub actor_id: String,
	pub generation: u64,
	pub expected_head_txid: u64,
	pub new_db_size_pages: u32,
	pub max_delta_bytes: u64,
	pub max_pages_per_stage: usize,
	pub dirty_pages: Vec<protocol::SqliteDirtyPage>,
}

#[derive(Debug, Clone)]
pub struct BufferedCommitOutcome {
	pub path: CommitPath,
	pub new_head_txid: u64,
	pub meta: protocol::SqliteMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitBufferError {
	FenceMismatch(String),
	StageNotFound(u64),
	Other(String),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteVfsMetricsSnapshot {
	pub request_build_ns: u64,
	pub serialize_ns: u64,
	pub transport_ns: u64,
	pub state_update_ns: u64,
	pub total_ns: u64,
	pub commit_count: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct CommitTransportMetrics {
	serialize_ns: u64,
	transport_ns: u64,
}

pub struct VfsV2Context {
	actor_id: String,
	runtime: Handle,
	transport: SqliteTransport,
	config: VfsV2Config,
	state: RwLock<VfsV2State>,
	aux_files: RwLock<BTreeMap<String, Arc<AuxFileState>>>,
	last_error: Mutex<Option<String>>,
	commit_atomic_count: AtomicU64,
	io_methods: Box<sqlite3_io_methods>,
	// Performance counters
	pub resolve_pages_total: AtomicU64,
	pub resolve_pages_cache_hits: AtomicU64,
	pub resolve_pages_fetches: AtomicU64,
	pub pages_fetched_total: AtomicU64,
	pub prefetch_pages_total: AtomicU64,
	pub commit_total: AtomicU64,
	pub commit_request_build_ns: AtomicU64,
	pub commit_serialize_ns: AtomicU64,
	pub commit_transport_ns: AtomicU64,
	pub commit_state_update_ns: AtomicU64,
	pub commit_duration_ns_total: AtomicU64,
}

#[derive(Debug, Clone)]
struct VfsV2State {
	generation: u64,
	head_txid: u64,
	db_size_pages: u32,
	page_size: usize,
	max_delta_bytes: u64,
	page_cache: Cache<u32, Vec<u8>>,
	write_buffer: WriteBuffer,
	predictor: PrefetchPredictor,
	dead: bool,
}

#[derive(Debug, Clone, Default)]
struct WriteBuffer {
	in_atomic_write: bool,
	saved_db_size: u32,
	dirty: BTreeMap<u32, Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
struct PrefetchPredictor {
	last_pgno: Option<u32>,
	last_delta: Option<i64>,
	stride_run_len: usize,
	// Inspired by mvSQLite's Markov + stride predictor design (Apache-2.0).
	transitions: HashMap<i64, HashMap<i64, u32>>,
}

#[derive(Debug)]
enum GetPagesError {
	FenceMismatch(String),
	Other(String),
}

#[repr(C)]
struct VfsV2File {
	base: sqlite3_file,
	ctx: *const VfsV2Context,
	aux: *mut AuxFileHandle,
}

#[derive(Default)]
struct AuxFileState {
	bytes: Mutex<Vec<u8>>,
}

struct AuxFileHandle {
	path: String,
	state: Arc<AuxFileState>,
	delete_on_close: bool,
}

unsafe impl Send for VfsV2Context {}
unsafe impl Sync for VfsV2Context {}

pub struct SqliteVfsV2 {
	vfs_ptr: *mut sqlite3_vfs,
	_name: CString,
	ctx_ptr: *mut VfsV2Context,
}

unsafe impl Send for SqliteVfsV2 {}
unsafe impl Sync for SqliteVfsV2 {}

pub struct NativeDatabaseV2 {
	db: *mut sqlite3,
	_vfs: SqliteVfsV2,
}

unsafe impl Send for NativeDatabaseV2 {}

impl PrefetchPredictor {
	fn record(&mut self, pgno: u32) {
		if let Some(last_pgno) = self.last_pgno {
			let delta = pgno as i64 - last_pgno as i64;
			if let Some(last_delta) = self.last_delta {
				self.transitions
					.entry(last_delta)
					.or_default()
					.entry(delta)
					.and_modify(|count| *count += 1)
					.or_insert(1);
				if delta == last_delta {
					self.stride_run_len += 1;
				} else {
					self.stride_run_len = 1;
				}
			} else {
				self.stride_run_len = 1;
			}
			self.last_delta = Some(delta);
		}
		self.last_pgno = Some(pgno);
	}

	fn multi_predict(&self, from_pgno: u32, depth: usize, db_size_pages: u32) -> Vec<u32> {
		if depth == 0 || db_size_pages == 0 {
			return Vec::new();
		}

		let mut seen = HashSet::new();
		let mut predicted = Vec::with_capacity(depth);

		if let Some(delta) = self.last_delta {
			if self.stride_run_len >= 2 && delta > 0 {
				let mut current = from_pgno as i64;
				for _ in 0..depth {
					current += delta;
					if !(1..=db_size_pages as i64).contains(&current) {
						break;
					}
					let pgno = current as u32;
					if seen.insert(pgno) {
						predicted.push(pgno);
					}
				}
				if predicted.len() >= depth {
					return predicted;
				}
			}

			let mut current_delta = delta;
			let mut current_pgno = from_pgno as i64;
			for _ in predicted.len()..depth {
				let Some(next_delta) = self
					.transitions
					.get(&current_delta)
					.and_then(|counts| counts.iter().max_by_key(|(_, count)| *count))
					.map(|(delta, _)| *delta)
				else {
					break;
				};

				current_pgno += next_delta;
				if !(1..=db_size_pages as i64).contains(&current_pgno) {
					break;
				}
				let pgno = current_pgno as u32;
				if seen.insert(pgno) {
					predicted.push(pgno);
				}
				current_delta = next_delta;
			}
		}

		predicted
	}
}

impl VfsV2State {
	fn new(config: &VfsV2Config, startup: &protocol::SqliteStartupData) -> Self {
		let page_cache = Cache::builder()
			.max_capacity(config.cache_capacity_pages)
			.build();
		for page in &startup.preloaded_pages {
			if let Some(bytes) = &page.bytes {
				page_cache.insert(page.pgno, bytes.clone());
			}
		}

		let mut state = Self {
			generation: startup.generation,
			head_txid: startup.meta.head_txid,
			db_size_pages: startup.meta.db_size_pages,
			page_size: startup.meta.page_size as usize,
			max_delta_bytes: startup.meta.max_delta_bytes,
			page_cache,
			write_buffer: WriteBuffer::default(),
			predictor: PrefetchPredictor::default(),
			dead: false,
		};
		if state.db_size_pages == 0 && !state.page_cache.contains_key(&1) {
			state.page_cache.insert(1, empty_db_page());
			state.db_size_pages = 1;
		}
		state
	}

	fn update_meta(&mut self, meta: &protocol::SqliteMeta) {
		self.generation = meta.generation;
		self.head_txid = meta.head_txid;
		self.db_size_pages = meta.db_size_pages;
		self.page_size = meta.page_size as usize;
		self.max_delta_bytes = meta.max_delta_bytes;
	}

	fn update_read_meta(&mut self, meta: &protocol::SqliteMeta) {
		self.max_delta_bytes = meta.max_delta_bytes;
	}
}

impl VfsV2Context {
	fn new(
		actor_id: String,
		runtime: Handle,
		transport: SqliteTransport,
		startup: protocol::SqliteStartupData,
		config: VfsV2Config,
		io_methods: sqlite3_io_methods,
	) -> Self {
		Self {
			actor_id,
			runtime,
			transport,
			config: config.clone(),
			state: RwLock::new(VfsV2State::new(&config, &startup)),
			aux_files: RwLock::new(BTreeMap::new()),
			last_error: Mutex::new(None),
			commit_atomic_count: AtomicU64::new(0),
			io_methods: Box::new(io_methods),
			resolve_pages_total: AtomicU64::new(0),
			resolve_pages_cache_hits: AtomicU64::new(0),
			resolve_pages_fetches: AtomicU64::new(0),
			pages_fetched_total: AtomicU64::new(0),
			prefetch_pages_total: AtomicU64::new(0),
			commit_total: AtomicU64::new(0),
			commit_request_build_ns: AtomicU64::new(0),
			commit_serialize_ns: AtomicU64::new(0),
			commit_transport_ns: AtomicU64::new(0),
			commit_state_update_ns: AtomicU64::new(0),
			commit_duration_ns_total: AtomicU64::new(0),
		}
	}

	fn clear_last_error(&self) {
		*self.last_error.lock() = None;
	}

	fn set_last_error(&self, message: String) {
		*self.last_error.lock() = Some(message);
	}

	fn clone_last_error(&self) -> Option<String> {
		self.last_error.lock().clone()
	}

	fn take_last_error(&self) -> Option<String> {
		self.last_error.lock().take()
	}

	fn add_commit_phase_metrics(
		&self,
		request_build_ns: u64,
		transport_metrics: CommitTransportMetrics,
		state_update_ns: u64,
		total_ns: u64,
	) {
		self.commit_request_build_ns
			.fetch_add(request_build_ns, Ordering::Relaxed);
		self.commit_serialize_ns
			.fetch_add(transport_metrics.serialize_ns, Ordering::Relaxed);
		self.commit_transport_ns
			.fetch_add(transport_metrics.transport_ns, Ordering::Relaxed);
		self.commit_state_update_ns
			.fetch_add(state_update_ns, Ordering::Relaxed);
		self.commit_duration_ns_total
			.fetch_add(total_ns, Ordering::Relaxed);
	}

	fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		SqliteVfsMetricsSnapshot {
			request_build_ns: self.commit_request_build_ns.load(Ordering::Relaxed),
			serialize_ns: self.commit_serialize_ns.load(Ordering::Relaxed),
			transport_ns: self.commit_transport_ns.load(Ordering::Relaxed),
			state_update_ns: self.commit_state_update_ns.load(Ordering::Relaxed),
			total_ns: self.commit_duration_ns_total.load(Ordering::Relaxed),
			commit_count: self.commit_total.load(Ordering::Relaxed),
		}
	}

	fn page_size(&self) -> usize {
		self.state.read().page_size.max(DEFAULT_PAGE_SIZE)
	}

	fn open_aux_file(&self, path: &str) -> Arc<AuxFileState> {
		if let Some(state) = self.aux_files.read().get(path) {
			return state.clone();
		}

		let mut aux_files = self.aux_files.write();
		aux_files
			.entry(path.to_string())
			.or_insert_with(|| Arc::new(AuxFileState::default()))
			.clone()
	}

	fn aux_file_exists(&self, path: &str) -> bool {
		self.aux_files.read().contains_key(path)
	}

	fn delete_aux_file(&self, path: &str) {
		self.aux_files.write().remove(path);
	}

	fn is_dead(&self) -> bool {
		self.state.read().dead
	}

	fn mark_dead(&self, message: String) {
		self.set_last_error(message);
		self.state.write().dead = true;
	}

	fn resolve_pages(
		&self,
		target_pgnos: &[u32],
		prefetch: bool,
	) -> std::result::Result<HashMap<u32, Option<Vec<u8>>>, GetPagesError> {
		use std::sync::atomic::Ordering::Relaxed;
		self.resolve_pages_total.fetch_add(1, Relaxed);

		let mut resolved = HashMap::new();
		let mut missing = Vec::new();
		let mut seen = HashSet::new();

		{
			let state = self.state.read();
			if state.dead {
				return Err(GetPagesError::Other(
					"sqlite v2 actor lost its fence".to_string(),
				));
			}

			for pgno in target_pgnos.iter().copied() {
				if !seen.insert(pgno) {
					continue;
				}
				if let Some(bytes) = state.write_buffer.dirty.get(&pgno) {
					resolved.insert(pgno, Some(bytes.clone()));
					continue;
				}
				if let Some(bytes) = state.page_cache.get(&pgno) {
					resolved.insert(pgno, Some(bytes));
					continue;
				}
				missing.push(pgno);
			}
		}

		if missing.is_empty() {
			self.resolve_pages_cache_hits
				.fetch_add(target_pgnos.len() as u64, Relaxed);
			return Ok(resolved);
		}
		self.resolve_pages_cache_hits
			.fetch_add((seen.len() - missing.len()) as u64, Relaxed);

		let (generation, to_fetch) = {
			let mut state = self.state.write();
			for pgno in target_pgnos.iter().copied() {
				state.predictor.record(pgno);
			}

			let mut to_fetch = missing.clone();
			if prefetch {
				let page_budget = (self.config.max_prefetch_bytes / state.page_size.max(1)).max(1);
				let prediction_budget = page_budget.saturating_sub(to_fetch.len());
				let seed_pgno = target_pgnos.last().copied().unwrap_or_default();
				for predicted in state.predictor.multi_predict(
					seed_pgno,
					prediction_budget.min(self.config.prefetch_depth),
					state.db_size_pages.max(seed_pgno),
				) {
					if resolved.contains_key(&predicted) || to_fetch.contains(&predicted) {
						continue;
					}
					to_fetch.push(predicted);
				}
			}
			(state.generation, to_fetch)
		};

		{
			let prefetch_count = to_fetch.len() - missing.len();
			self.resolve_pages_fetches.fetch_add(1, Relaxed);
			self.pages_fetched_total
				.fetch_add(to_fetch.len() as u64, Relaxed);
			self.prefetch_pages_total
				.fetch_add(prefetch_count as u64, Relaxed);
			tracing::debug!(
				missing = missing.len(),
				prefetch = prefetch_count,
				total_fetch = to_fetch.len(),
				"vfs get_pages fetch"
			);
		}

		let response = self
			.runtime
			.block_on(self.transport.get_pages(protocol::SqliteGetPagesRequest {
				actor_id: self.actor_id.clone(),
				generation,
				pgnos: to_fetch.clone(),
			}))
			.map_err(|err| GetPagesError::Other(err.to_string()))?;

		match response {
			protocol::SqliteGetPagesResponse::SqliteFenceMismatch(mismatch) => {
				Err(GetPagesError::FenceMismatch(mismatch.reason))
			}
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => {
				let mut state = self.state.write();
				state.update_read_meta(&ok.meta);
				for fetched in ok.pages {
					if let Some(bytes) = &fetched.bytes {
						state.page_cache.insert(fetched.pgno, bytes.clone());
					}
					resolved.insert(fetched.pgno, fetched.bytes);
				}
				for pgno in missing {
					resolved.entry(pgno).or_insert(None);
				}
				Ok(resolved)
			}
			protocol::SqliteGetPagesResponse::SqliteErrorResponse(error) => {
				Err(GetPagesError::Other(error.message))
			}
		}
	}

	fn flush_dirty_pages(
		&self,
	) -> std::result::Result<Option<BufferedCommitOutcome>, CommitBufferError> {
		let total_start = Instant::now();
		let request_build_start = Instant::now();
		let request = {
			let state = self.state.read();
			if state.dead {
				return Err(CommitBufferError::Other(
					"sqlite v2 actor lost its fence".to_string(),
				));
			}
			if state.write_buffer.in_atomic_write || state.write_buffer.dirty.is_empty() {
				return Ok(None);
			}

			BufferedCommitRequest {
				actor_id: self.actor_id.clone(),
				generation: state.generation,
				expected_head_txid: state.head_txid,
				new_db_size_pages: state.db_size_pages,
				max_delta_bytes: state.max_delta_bytes,
				max_pages_per_stage: self.config.max_pages_per_stage,
				dirty_pages: state
					.write_buffer
					.dirty
					.iter()
					.map(|(pgno, bytes)| protocol::SqliteDirtyPage {
						pgno: *pgno,
						bytes: bytes.clone(),
					})
					.collect(),
			}
		};
		let request_build_ns = request_build_start.elapsed().as_nanos() as u64;

		let (outcome, transport_metrics) = match self
			.runtime
			.block_on(commit_buffered_pages(&self.transport, request.clone()))
		{
			Ok(outcome) => outcome,
			Err(err) => {
				mark_dead_for_non_fence_commit_error(self, &err);
				return Err(err);
			}
		};
		self.commit_total
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		tracing::debug!(
			dirty_pages = request.dirty_pages.len(),
			path = ?outcome.path,
			new_head_txid = outcome.new_head_txid,
			request_build_ns,
			serialize_ns = transport_metrics.serialize_ns,
			transport_ns = transport_metrics.transport_ns,
			"vfs commit complete (flush)"
		);
		let state_update_start = Instant::now();
		let mut state = self.state.write();
		state.update_meta(&outcome.meta);
		state.db_size_pages = request.new_db_size_pages;
		for dirty_page in &request.dirty_pages {
			state
				.page_cache
				.insert(dirty_page.pgno, dirty_page.bytes.clone());
		}
		state.write_buffer.dirty.clear();
		let state_update_ns = state_update_start.elapsed().as_nanos() as u64;
		self.add_commit_phase_metrics(
			request_build_ns,
			transport_metrics,
			state_update_ns,
			total_start.elapsed().as_nanos() as u64,
		);
		Ok(Some(outcome))
	}

	fn commit_atomic_write(&self) -> std::result::Result<(), CommitBufferError> {
		let total_start = Instant::now();
		let request_build_start = Instant::now();
		let request = {
			let mut state = self.state.write();
			if state.dead {
				return Err(CommitBufferError::Other(
					"sqlite v2 actor lost its fence".to_string(),
				));
			}
			if !state.write_buffer.in_atomic_write {
				return Ok(());
			}
			if state.write_buffer.dirty.is_empty() {
				state.write_buffer.in_atomic_write = false;
				return Ok(());
			}

			BufferedCommitRequest {
				actor_id: self.actor_id.clone(),
				generation: state.generation,
				expected_head_txid: state.head_txid,
				new_db_size_pages: state.db_size_pages,
				max_delta_bytes: state.max_delta_bytes,
				max_pages_per_stage: self.config.max_pages_per_stage,
				dirty_pages: state
					.write_buffer
					.dirty
					.iter()
					.map(|(pgno, bytes)| protocol::SqliteDirtyPage {
						pgno: *pgno,
						bytes: bytes.clone(),
					})
					.collect(),
			}
		};
		let request_build_ns = request_build_start.elapsed().as_nanos() as u64;

		let (outcome, transport_metrics) = match self
			.runtime
			.block_on(commit_buffered_pages(&self.transport, request.clone()))
		{
			Ok(outcome) => outcome,
			Err(err) => {
				mark_dead_for_non_fence_commit_error(self, &err);
				return Err(err);
			}
		};
		self.commit_total
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		tracing::debug!(
			dirty_pages = request.dirty_pages.len(),
			path = ?outcome.path,
			new_head_txid = outcome.new_head_txid,
			request_build_ns,
			serialize_ns = transport_metrics.serialize_ns,
			transport_ns = transport_metrics.transport_ns,
			"vfs commit complete (atomic)"
		);
		self.set_last_error(format!(
			"post-commit atomic write succeeded: requested_db_size_pages={}, returned_db_size_pages={}, returned_head_txid={}",
			request.new_db_size_pages,
			outcome.meta.db_size_pages,
			outcome.meta.head_txid,
		));
		let state_update_start = Instant::now();
		let mut state = self.state.write();
		state.update_meta(&outcome.meta);
		state.db_size_pages = request.new_db_size_pages;
		for dirty_page in &request.dirty_pages {
			state
				.page_cache
				.insert(dirty_page.pgno, dirty_page.bytes.clone());
		}
		state.write_buffer.dirty.clear();
		state.write_buffer.in_atomic_write = false;
		let state_update_ns = state_update_start.elapsed().as_nanos() as u64;
		self.add_commit_phase_metrics(
			request_build_ns,
			transport_metrics,
			state_update_ns,
			total_start.elapsed().as_nanos() as u64,
		);
		Ok(())
	}

	fn truncate_main_file(&self, size: sqlite3_int64) {
		let page_size = self.page_size() as i64;
		let truncated_pages = ((size + page_size - 1) / page_size) as u32;
		let mut state = self.state.write();
		state.db_size_pages = truncated_pages;
		state
			.write_buffer
			.dirty
			.retain(|pgno, _| *pgno <= truncated_pages);
		state.page_cache.invalidate_all();
	}
}

fn cleanup_batch_atomic_probe(db: *mut sqlite3) {
	if let Err(err) = sqlite_exec(db, "DROP TABLE IF EXISTS __rivet_batch_probe;") {
		tracing::warn!(%err, "failed to clean up sqlite v2 batch atomic probe table");
	}
}

fn assert_batch_atomic_probe(
	db: *mut sqlite3,
	vfs: &SqliteVfsV2,
) -> std::result::Result<(), String> {
	let commit_atomic_before = vfs.commit_atomic_count();
	let probe_sql = "\
		BEGIN IMMEDIATE;\
		CREATE TABLE IF NOT EXISTS __rivet_batch_probe(x INTEGER);\
		INSERT INTO __rivet_batch_probe VALUES(1);\
		DELETE FROM __rivet_batch_probe;\
		DROP TABLE IF EXISTS __rivet_batch_probe;\
		COMMIT;\
	";

	if let Err(err) = sqlite_exec(db, probe_sql) {
		cleanup_batch_atomic_probe(db);
		return Err(format!("batch atomic probe failed: {err}"));
	}

	let commit_atomic_after = vfs.commit_atomic_count();
	if commit_atomic_after == commit_atomic_before {
		tracing::error!(
			"batch atomic writes not active for sqlite v2, SQLITE_ENABLE_BATCH_ATOMIC_WRITE may be missing"
		);
		cleanup_batch_atomic_probe(db);
		return Err(
			"batch atomic writes not active for sqlite v2, SQLITE_ENABLE_BATCH_ATOMIC_WRITE may be missing"
				.to_string(),
		);
	}

	Ok(())
}

fn mark_dead_for_non_fence_commit_error(ctx: &VfsV2Context, err: &CommitBufferError) {
	match err {
		CommitBufferError::FenceMismatch(_) => {}
		CommitBufferError::StageNotFound(stage_id) => {
			ctx.mark_dead(format!(
				"sqlite v2 stage {stage_id} missing during commit finalize"
			));
		}
		CommitBufferError::Other(message) => ctx.mark_dead(message.clone()),
	}
}

fn mark_dead_from_fence_commit_error(ctx: &VfsV2Context, err: &CommitBufferError) {
	if let CommitBufferError::FenceMismatch(reason) = err {
		ctx.mark_dead(reason.clone());
	}
}

fn dirty_pages_raw_bytes(dirty_pages: &[protocol::SqliteDirtyPage]) -> Result<u64> {
	dirty_pages.iter().try_fold(0u64, |total, dirty_page| {
		let page_len = u64::try_from(dirty_page.bytes.len())?;
		Ok(total + page_len)
	})
}

fn split_dirty_pages_by_size(
	dirty_pages: &[protocol::SqliteDirtyPage],
	max_delta_bytes: u64,
	max_pages_per_stage: usize,
) -> Result<Vec<Vec<protocol::SqliteDirtyPage>>> {
	let mut chunks = Vec::new();
	let mut chunk = Vec::new();
	let mut chunk_bytes = 0u64;

	for dirty_page in dirty_pages {
		let page_len = u64::try_from(dirty_page.bytes.len())?;
		let would_overflow_bytes = !chunk.is_empty() && chunk_bytes + page_len > max_delta_bytes;
		let would_overflow_pages = !chunk.is_empty() && chunk.len() >= max_pages_per_stage;
		if would_overflow_bytes || would_overflow_pages {
			chunks.push(chunk);
			chunk = Vec::new();
			chunk_bytes = 0;
		}

		chunk_bytes += page_len;
		chunk.push(dirty_page.clone());
	}

	if !chunk.is_empty() {
		chunks.push(chunk);
	}

	if chunks.is_empty() {
		chunks.push(Vec::new());
	}

	Ok(chunks)
}

fn next_stage_id() -> u64 {
	NEXT_STAGE_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_temp_aux_path() -> String {
	format!(
		"{TEMP_AUX_PATH_PREFIX}-{}",
		NEXT_TEMP_AUX_ID.fetch_add(1, Ordering::Relaxed)
	)
}

unsafe fn get_aux_state(file: &VfsV2File) -> Option<&AuxFileHandle> {
	(!file.aux.is_null()).then(|| &*file.aux)
}

async fn commit_buffered_pages(
	transport: &SqliteTransport,
	request: BufferedCommitRequest,
) -> std::result::Result<(BufferedCommitOutcome, CommitTransportMetrics), CommitBufferError> {
	let raw_dirty_bytes = dirty_pages_raw_bytes(&request.dirty_pages)
		.map_err(|err| CommitBufferError::Other(err.to_string()))?;
	let mut metrics = CommitTransportMetrics::default();

	if raw_dirty_bytes <= request.max_delta_bytes {
		let serialize_start = Instant::now();
		let fast_request = protocol::SqliteCommitRequest {
			actor_id: request.actor_id.clone(),
			generation: request.generation,
			expected_head_txid: request.expected_head_txid,
			dirty_pages: request.dirty_pages.clone(),
			new_db_size_pages: request.new_db_size_pages,
		};
		metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;
		let transport_start = Instant::now();
		match transport
			.commit(fast_request)
			.await
			.map_err(|err| CommitBufferError::Other(err.to_string()))?
		{
			protocol::SqliteCommitResponse::SqliteCommitOk(ok) => {
				metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
				return Ok((
					BufferedCommitOutcome {
						path: CommitPath::Fast,
						new_head_txid: ok.new_head_txid,
						meta: ok.meta,
					},
					metrics,
				));
			}
			protocol::SqliteCommitResponse::SqliteFenceMismatch(mismatch) => {
				return Err(CommitBufferError::FenceMismatch(mismatch.reason));
			}
			protocol::SqliteCommitResponse::SqliteCommitTooLarge(_) => {
				metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
			}
			protocol::SqliteCommitResponse::SqliteErrorResponse(error) => {
				return Err(CommitBufferError::Other(error.message));
			}
		}
	}

	let stage_id = next_stage_id();
	let staged_chunks = split_dirty_pages_by_size(
		&request.dirty_pages,
		request.max_delta_bytes,
		request.max_pages_per_stage,
	)
	.map_err(|err| CommitBufferError::Other(err.to_string()))?;

	for (chunk_idx, dirty_pages) in staged_chunks.iter().enumerate() {
		let serialize_start = Instant::now();
		let stage_request = protocol::SqliteCommitStageRequest {
			actor_id: request.actor_id.clone(),
			generation: request.generation,
			stage_id,
			chunk_idx: chunk_idx as u16,
			dirty_pages: dirty_pages.clone(),
			is_last: chunk_idx + 1 == staged_chunks.len(),
		};
		metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;
		if transport
			.queue_commit_stage(stage_request.clone())
			.map_err(|err| CommitBufferError::Other(err.to_string()))?
		{
			continue;
		}

		let transport_start = Instant::now();
		match transport
			.commit_stage(stage_request)
			.await
			.map_err(|err| CommitBufferError::Other(err.to_string()))?
		{
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(_) => {
				metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
			}
			protocol::SqliteCommitStageResponse::SqliteFenceMismatch(mismatch) => {
				return Err(CommitBufferError::FenceMismatch(mismatch.reason));
			}
			protocol::SqliteCommitStageResponse::SqliteErrorResponse(error) => {
				return Err(CommitBufferError::Other(error.message));
			}
		}
	}

	let serialize_start = Instant::now();
	let finalize_request = protocol::SqliteCommitFinalizeRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		expected_head_txid: request.expected_head_txid,
		stage_id,
		new_db_size_pages: request.new_db_size_pages,
	};
	metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;
	let transport_start = Instant::now();
	match transport
		.commit_finalize(finalize_request)
		.await
		.map_err(|err| CommitBufferError::Other(err.to_string()))?
	{
		protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) => {
			metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
			Ok((
				BufferedCommitOutcome {
					path: CommitPath::Slow,
					new_head_txid: ok.new_head_txid,
					meta: ok.meta,
				},
				metrics,
			))
		}
		protocol::SqliteCommitFinalizeResponse::SqliteFenceMismatch(mismatch) => {
			Err(CommitBufferError::FenceMismatch(mismatch.reason))
		}
		protocol::SqliteCommitFinalizeResponse::SqliteStageNotFound(not_found) => {
			Err(CommitBufferError::StageNotFound(not_found.stage_id))
		}
		protocol::SqliteCommitFinalizeResponse::SqliteErrorResponse(error) => {
			Err(CommitBufferError::Other(error.message))
		}
	}
}

unsafe fn get_file(p: *mut sqlite3_file) -> &'static mut VfsV2File {
	&mut *(p as *mut VfsV2File)
}

unsafe fn get_vfs_ctx(p: *mut sqlite3_vfs) -> &'static VfsV2Context {
	&*((*p).pAppData as *const VfsV2Context)
}

fn sqlite_error_message(db: *mut sqlite3) -> String {
	unsafe {
		if db.is_null() {
			"unknown sqlite error".to_string()
		} else {
			CStr::from_ptr(sqlite3_errmsg(db))
				.to_string_lossy()
				.into_owned()
		}
	}
}

fn sqlite_exec(db: *mut sqlite3, sql: &str) -> std::result::Result<(), String> {
	let c_sql = CString::new(sql).map_err(|err| err.to_string())?;
	let rc = unsafe { sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(format!(
			"`{sql}` failed with code {rc}: {}",
			sqlite_error_message(db)
		));
	}
	Ok(())
}

#[cfg(test)]
fn sqlite_step_statement(db: *mut sqlite3, sql: &str) -> std::result::Result<(), String> {
	let c_sql = CString::new(sql).map_err(|err| err.to_string())?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(format!(
			"`{sql}` prepare failed with code {rc}: {}",
			sqlite_error_message(db)
		));
	}
	if stmt.is_null() {
		return Ok(());
	}

	let result = loop {
		let step_rc = unsafe { sqlite3_step(stmt) };
		if step_rc == SQLITE_DONE {
			break Ok(());
		}
		if step_rc != SQLITE_ROW {
			break Err(format!(
				"`{sql}` step failed with code {step_rc}: {}",
				sqlite_error_message(db)
			));
		}
	};

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

fn page_span(offset: i64, length: usize, page_size: usize) -> std::result::Result<Vec<u32>, ()> {
	if offset < 0 {
		return Err(());
	}
	if length == 0 {
		return Ok(Vec::new());
	}

	let start = offset as usize / page_size + 1;
	let end = (offset as usize + length - 1) / page_size + 1;
	Ok((start as u32..=end as u32).collect())
}

unsafe extern "C" fn v2_io_close(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		if p_file.is_null() {
			return SQLITE_OK;
		}
		let file = get_file(p_file);
		let result = if !file.aux.is_null() {
			let aux = Box::from_raw(file.aux);
			if aux.delete_on_close {
				let ctx = &*file.ctx;
				ctx.delete_aux_file(&aux.path);
			}
			file.aux = ptr::null_mut();
			Ok(())
		} else {
			let ctx = &*file.ctx;
			let should_flush = {
				let state = ctx.state.read();
				state.write_buffer.in_atomic_write || !state.write_buffer.dirty.is_empty()
			};
			if should_flush {
				if ctx.state.read().write_buffer.in_atomic_write {
					ctx.commit_atomic_write().map(|_| ())
				} else {
					ctx.flush_dirty_pages().map(|_| ())
				}
			} else {
				Ok(())
			}
		};
		file.base.pMethods = ptr::null();
		match result {
			Ok(()) => SQLITE_OK,
			Err(err) => {
				let ctx = &*file.ctx;
				mark_dead_from_fence_commit_error(ctx, &err);
				SQLITE_IOERR
			}
		}
	})
}

unsafe extern "C" fn v2_io_read(
	p_file: *mut sqlite3_file,
	p_buf: *mut c_void,
	i_amt: c_int,
	i_offset: sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_READ, {
		if i_amt <= 0 {
			return SQLITE_OK;
		}

		let file = get_file(p_file);
		if let Some(aux) = get_aux_state(file) {
			if i_offset < 0 {
				return SQLITE_IOERR_READ;
			}

			let offset = i_offset as usize;
			let requested = i_amt as usize;
			let buf = slice::from_raw_parts_mut(p_buf.cast::<u8>(), requested);
			buf.fill(0);

			let bytes = aux.state.bytes.lock();
			if offset >= bytes.len() {
				return SQLITE_IOERR_SHORT_READ;
			}

			let copy_len = requested.min(bytes.len() - offset);
			buf[..copy_len].copy_from_slice(&bytes[offset..offset + copy_len]);
			return if copy_len < requested {
				SQLITE_IOERR_SHORT_READ
			} else {
				SQLITE_OK
			};
		}

		let ctx = &*file.ctx;
		if ctx.is_dead() {
			return SQLITE_IOERR_READ;
		}

		let buf = slice::from_raw_parts_mut(p_buf.cast::<u8>(), i_amt as usize);
		let requested_pages = match page_span(i_offset, i_amt as usize, ctx.page_size()) {
			Ok(pages) => pages,
			Err(_) => return SQLITE_IOERR_READ,
		};
		let page_size = ctx.page_size();
		let file_size = {
			let state = ctx.state.read();
			state.db_size_pages as usize * state.page_size
		};

		let resolved = match ctx.resolve_pages(&requested_pages, true) {
			Ok(pages) => pages,
			Err(GetPagesError::FenceMismatch(reason)) => {
				ctx.mark_dead(reason);
				return SQLITE_IOERR_READ;
			}
			Err(GetPagesError::Other(message)) => {
				ctx.mark_dead(message);
				return SQLITE_IOERR_READ;
			}
		};
		ctx.clear_last_error();

		buf.fill(0);
		for pgno in requested_pages {
			let Some(Some(bytes)) = resolved.get(&pgno) else {
				continue;
			};
			let page_start = (pgno as usize - 1) * page_size;
			let copy_start = page_start.max(i_offset as usize);
			let copy_end = (page_start + page_size).min(i_offset as usize + i_amt as usize);
			if copy_start >= copy_end {
				continue;
			}
			let page_offset = copy_start - page_start;
			let dest_offset = copy_start - i_offset as usize;
			let copy_len = copy_end - copy_start;
			buf[dest_offset..dest_offset + copy_len]
				.copy_from_slice(&bytes[page_offset..page_offset + copy_len]);
		}

		if i_offset as usize + i_amt as usize > file_size {
			return SQLITE_IOERR_SHORT_READ;
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn v2_io_write(
	p_file: *mut sqlite3_file,
	p_buf: *const c_void,
	i_amt: c_int,
	i_offset: sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_WRITE, {
		if i_amt <= 0 {
			return SQLITE_OK;
		}

		let file = get_file(p_file);
		if let Some(aux) = get_aux_state(file) {
			if i_offset < 0 {
				return SQLITE_IOERR_WRITE;
			}

			let offset = i_offset as usize;
			let source = slice::from_raw_parts(p_buf.cast::<u8>(), i_amt as usize);
			let mut bytes = aux.state.bytes.lock();
			let end = offset + source.len();
			if bytes.len() < end {
				bytes.resize(end, 0);
			}
			bytes[offset..end].copy_from_slice(source);
			return SQLITE_OK;
		}

		let ctx = &*file.ctx;
		if ctx.is_dead() {
			return SQLITE_IOERR_WRITE;
		}

		let page_size = ctx.page_size();
		let source = slice::from_raw_parts(p_buf.cast::<u8>(), i_amt as usize);
		let target_pages = match page_span(i_offset, i_amt as usize, page_size) {
			Ok(pages) => pages,
			Err(_) => return SQLITE_IOERR_WRITE,
		};

		// Fast path: for full-page aligned writes we don't need the existing
		// page data because we're overwriting every byte. Skip resolve_pages
		// to eliminate a round trip to the engine per page. Also, for pages
		// beyond db_size_pages (new allocations), there's nothing to fetch.
		let offset = i_offset as usize;
		let amt = i_amt as usize;
		let is_aligned_full_page = offset % page_size == 0 && amt % page_size == 0;

		let resolved = if is_aligned_full_page {
			HashMap::new()
		} else {
			let (db_size_pages, pages_to_resolve): (u32, Vec<u32>) = {
				let state = ctx.state.read();
				let known_max = state.db_size_pages;
				(
					known_max,
					target_pages
						.iter()
						.copied()
						.filter(|pgno| *pgno <= known_max)
						.collect(),
				)
			};

			let mut resolved = if pages_to_resolve.is_empty() {
				HashMap::new()
			} else {
				match ctx.resolve_pages(&pages_to_resolve, false) {
					Ok(pages) => pages,
					Err(GetPagesError::FenceMismatch(reason)) => {
						ctx.mark_dead(reason);
						return SQLITE_IOERR_WRITE;
					}
					Err(GetPagesError::Other(message)) => {
						ctx.mark_dead(message);
						return SQLITE_IOERR_WRITE;
					}
				}
			};
			for pgno in &target_pages {
				if *pgno > db_size_pages {
					resolved.entry(*pgno).or_insert(None);
				}
			}
			resolved
		};

		let mut dirty_pages = BTreeMap::new();
		for pgno in target_pages {
			let page_start = (pgno as usize - 1) * page_size;
			let patch_start = page_start.max(offset);
			let patch_end = (page_start + page_size).min(offset + amt);
			let Some(copy_len) = patch_end.checked_sub(patch_start) else {
				continue;
			};
			if copy_len == 0 {
				continue;
			}

			let mut page = if is_aligned_full_page {
				vec![0; page_size]
			} else {
				resolved
					.get(&pgno)
					.and_then(|bytes| bytes.clone())
					.unwrap_or_else(|| vec![0; page_size])
			};
			if page.len() < page_size {
				page.resize(page_size, 0);
			}

			let page_offset = patch_start - page_start;
			let source_offset = patch_start - offset;
			page[page_offset..page_offset + copy_len]
				.copy_from_slice(&source[source_offset..source_offset + copy_len]);
			dirty_pages.insert(pgno, page);
		}

		let mut state = ctx.state.write();
		for (pgno, bytes) in dirty_pages {
			state.write_buffer.dirty.insert(pgno, bytes);
		}
		let end_page = ((offset + amt) + page_size - 1) / page_size;
		state.db_size_pages = state.db_size_pages.max(end_page as u32);
		ctx.clear_last_error();
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_io_truncate(p_file: *mut sqlite3_file, size: sqlite3_int64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_TRUNCATE, {
		if size < 0 {
			return SQLITE_IOERR_TRUNCATE;
		}
		let file = get_file(p_file);
		if let Some(aux) = get_aux_state(file) {
			aux.state.bytes.lock().truncate(size as usize);
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		ctx.truncate_main_file(size);
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		if get_aux_state(file).is_some() {
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		match ctx.flush_dirty_pages() {
			Ok(_) => SQLITE_OK,
			Err(err) => {
				mark_dead_from_fence_commit_error(ctx, &err);
				SQLITE_IOERR_FSYNC
			}
		}
	})
}

unsafe extern "C" fn v2_io_file_size(
	p_file: *mut sqlite3_file,
	p_size: *mut sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSTAT, {
		let file = get_file(p_file);
		if let Some(aux) = get_aux_state(file) {
			*p_size = aux.state.bytes.lock().len() as sqlite3_int64;
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		let state = ctx.state.read();
		*p_size = (state.db_size_pages as usize * state.page_size) as sqlite3_int64;
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_io_lock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_LOCK, SQLITE_OK)
}

unsafe extern "C" fn v2_io_unlock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_UNLOCK, SQLITE_OK)
}

unsafe extern "C" fn v2_io_check_reserved_lock(
	_p_file: *mut sqlite3_file,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		*p_res_out = 0;
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_io_file_control(
	p_file: *mut sqlite3_file,
	op: c_int,
	_p_arg: *mut c_void,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		if get_aux_state(file).is_some() {
			return SQLITE_NOTFOUND;
		}
		let ctx = &*file.ctx;

		match op {
			SQLITE_FCNTL_BEGIN_ATOMIC_WRITE => {
				let mut state = ctx.state.write();
				state.write_buffer.in_atomic_write = true;
				state.write_buffer.saved_db_size = state.db_size_pages;
				state.write_buffer.dirty.clear();
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => match ctx.commit_atomic_write() {
				Ok(()) => {
					ctx.commit_atomic_count.fetch_add(1, Ordering::Relaxed);
					SQLITE_OK
				}
				Err(err) => {
					mark_dead_from_fence_commit_error(ctx, &err);
					SQLITE_IOERR
				}
			},
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				let mut state = ctx.state.write();
				state.write_buffer.dirty.clear();
				state.write_buffer.in_atomic_write = false;
				state.db_size_pages = state.write_buffer.saved_db_size;
				SQLITE_OK
			}
			_ => SQLITE_NOTFOUND,
		}
	})
}

unsafe extern "C" fn v2_io_sector_size(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(DEFAULT_PAGE_SIZE as c_int, DEFAULT_PAGE_SIZE as c_int)
}

unsafe extern "C" fn v2_io_device_characteristics(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(0, {
		let file = get_file(p_file);
		if get_aux_state(file).is_some() {
			0
		} else {
			SQLITE_IOCAP_BATCH_ATOMIC
		}
	})
}

unsafe extern "C" fn v2_vfs_open(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	p_file: *mut sqlite3_file,
	flags: c_int,
	p_out_flags: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_CANTOPEN, {
		let ctx = get_vfs_ctx(p_vfs);
		let delete_on_close = (flags & SQLITE_OPEN_DELETEONCLOSE) != 0;
		let path = if z_name.is_null() {
			if delete_on_close {
				next_temp_aux_path()
			} else {
				return SQLITE_CANTOPEN;
			}
		} else {
			match CStr::from_ptr(z_name).to_str() {
				Ok(path) => path.to_string(),
				Err(_) => return SQLITE_CANTOPEN,
			}
		};
		let is_main =
			path == ctx.actor_id && !delete_on_close && (flags & SQLITE_OPEN_MAIN_DB) != 0;

		let base = sqlite3_file {
			pMethods: ctx.io_methods.as_ref(),
		};
		let aux = if is_main {
			ptr::null_mut()
		} else {
			Box::into_raw(Box::new(AuxFileHandle {
				path: path.clone(),
				state: ctx.open_aux_file(&path),
				delete_on_close,
			}))
		};
		ptr::write(
			p_file.cast::<VfsV2File>(),
			VfsV2File {
				base,
				ctx: ctx as *const VfsV2Context,
				aux,
			},
		);

		if !p_out_flags.is_null() {
			*p_out_flags = flags;
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn v2_vfs_delete(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_sync_dir: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_DELETE, {
		if z_name.is_null() {
			return SQLITE_OK;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(path) => path,
			Err(_) => return SQLITE_OK,
		};
		if path != ctx.actor_id {
			ctx.delete_aux_file(path);
		}
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_vfs_access(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_flags: c_int,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_ACCESS, {
		if z_name.is_null() {
			*p_res_out = 0;
			return SQLITE_OK;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(path) => path,
			Err(_) => {
				*p_res_out = 0;
				return SQLITE_OK;
			}
		};

		*p_res_out = if path == ctx.actor_id || ctx.aux_file_exists(path) {
			1
		} else {
			0
		};
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_vfs_full_pathname(
	_p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	n_out: c_int,
	z_out: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		if z_name.is_null() || z_out.is_null() || n_out <= 0 {
			return SQLITE_IOERR;
		}

		let name = CStr::from_ptr(z_name);
		let bytes = name.to_bytes_with_nul();
		if bytes.len() >= n_out as usize {
			return SQLITE_IOERR;
		}

		ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), z_out, bytes.len());
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_vfs_randomness(
	_p_vfs: *mut sqlite3_vfs,
	n_byte: c_int,
	z_out: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(0, {
		let buf = slice::from_raw_parts_mut(z_out.cast::<u8>(), n_byte as usize);
		match getrandom::getrandom(buf) {
			Ok(()) => n_byte,
			Err(_) => 0,
		}
	})
}

unsafe extern "C" fn v2_vfs_sleep(_p_vfs: *mut sqlite3_vfs, microseconds: c_int) -> c_int {
	vfs_catch_unwind!(0, {
		std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
		microseconds
	})
}

unsafe extern "C" fn v2_vfs_current_time(_p_vfs: *mut sqlite3_vfs, p_time_out: *mut f64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default();
		*p_time_out = 2440587.5 + (now.as_secs_f64() / 86400.0);
		SQLITE_OK
	})
}

unsafe extern "C" fn v2_vfs_get_last_error(
	p_vfs: *mut sqlite3_vfs,
	n_byte: c_int,
	z_err_msg: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		if n_byte <= 0 || z_err_msg.is_null() {
			return 0;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let Some(message) = ctx.clone_last_error() else {
			*z_err_msg = 0;
			return 0;
		};

		let bytes = message.as_bytes();
		let max_len = (n_byte as usize).saturating_sub(1);
		let copy_len = bytes.len().min(max_len);
		let dst = z_err_msg.cast::<u8>();
		ptr::copy_nonoverlapping(bytes.as_ptr(), dst, copy_len);
		*dst.add(copy_len) = 0;
		0
	})
}

impl SqliteVfsV2 {
	pub fn register(
		name: &str,
		handle: EnvoyHandle,
		actor_id: String,
		runtime: Handle,
		startup: protocol::SqliteStartupData,
		config: VfsV2Config,
	) -> std::result::Result<Self, String> {
		Self::register_with_transport(
			name,
			SqliteTransport::from_envoy(handle),
			actor_id,
			runtime,
			startup,
			config,
		)
	}

	fn take_last_error(&self) -> Option<String> {
		unsafe { (*self.ctx_ptr).take_last_error() }
	}

	fn register_with_transport(
		name: &str,
		transport: SqliteTransport,
		actor_id: String,
		runtime: Handle,
		startup: protocol::SqliteStartupData,
		config: VfsV2Config,
	) -> std::result::Result<Self, String> {
		let mut io_methods: sqlite3_io_methods = unsafe { std::mem::zeroed() };
		io_methods.iVersion = 1;
		io_methods.xClose = Some(v2_io_close);
		io_methods.xRead = Some(v2_io_read);
		io_methods.xWrite = Some(v2_io_write);
		io_methods.xTruncate = Some(v2_io_truncate);
		io_methods.xSync = Some(v2_io_sync);
		io_methods.xFileSize = Some(v2_io_file_size);
		io_methods.xLock = Some(v2_io_lock);
		io_methods.xUnlock = Some(v2_io_unlock);
		io_methods.xCheckReservedLock = Some(v2_io_check_reserved_lock);
		io_methods.xFileControl = Some(v2_io_file_control);
		io_methods.xSectorSize = Some(v2_io_sector_size);
		io_methods.xDeviceCharacteristics = Some(v2_io_device_characteristics);

		let ctx = Box::new(VfsV2Context::new(
			actor_id, runtime, transport, startup, config, io_methods,
		));
		let ctx_ptr = Box::into_raw(ctx);
		let name_cstring = CString::new(name).map_err(|err| err.to_string())?;

		let mut vfs: sqlite3_vfs = unsafe { std::mem::zeroed() };
		vfs.iVersion = 1;
		vfs.szOsFile = std::mem::size_of::<VfsV2File>() as c_int;
		vfs.mxPathname = MAX_PATHNAME;
		vfs.zName = name_cstring.as_ptr();
		vfs.pAppData = ctx_ptr.cast::<c_void>();
		vfs.xOpen = Some(v2_vfs_open);
		vfs.xDelete = Some(v2_vfs_delete);
		vfs.xAccess = Some(v2_vfs_access);
		vfs.xFullPathname = Some(v2_vfs_full_pathname);
		vfs.xRandomness = Some(v2_vfs_randomness);
		vfs.xSleep = Some(v2_vfs_sleep);
		vfs.xCurrentTime = Some(v2_vfs_current_time);
		vfs.xGetLastError = Some(v2_vfs_get_last_error);

		let vfs_ptr = Box::into_raw(Box::new(vfs));
		let rc = unsafe { sqlite3_vfs_register(vfs_ptr, 0) };
		if rc != SQLITE_OK {
			unsafe {
				drop(Box::from_raw(vfs_ptr));
				drop(Box::from_raw(ctx_ptr));
			}
			return Err(format!("sqlite3_vfs_register failed with code {rc}"));
		}

		Ok(Self {
			vfs_ptr,
			_name: name_cstring,
			ctx_ptr,
		})
	}

	pub fn name_ptr(&self) -> *const c_char {
		self._name.as_ptr()
	}

	fn commit_atomic_count(&self) -> u64 {
		unsafe { (*self.ctx_ptr).commit_atomic_count.load(Ordering::Relaxed) }
	}
}

impl Drop for SqliteVfsV2 {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs_ptr);
			drop(Box::from_raw(self.vfs_ptr));
			drop(Box::from_raw(self.ctx_ptr));
		}
	}
}

impl NativeDatabaseV2 {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.db
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self._vfs.take_last_error()
	}

	pub fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		unsafe { (*self._vfs.ctx_ptr).sqlite_vfs_metrics() }
	}
}

impl Drop for NativeDatabaseV2 {
	fn drop(&mut self) {
		if !self.db.is_null() {
			unsafe {
				sqlite3_close(self.db);
			}
		}
	}
}

pub fn open_database(
	vfs: SqliteVfsV2,
	file_name: &str,
) -> std::result::Result<NativeDatabaseV2, String> {
	let c_name = CString::new(file_name).map_err(|err| err.to_string())?;
	let mut db: *mut sqlite3 = ptr::null_mut();

	let rc = unsafe {
		sqlite3_open_v2(
			c_name.as_ptr(),
			&mut db,
			SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
			vfs.name_ptr(),
		)
	};
	if rc != SQLITE_OK {
		let message = sqlite_error_message(db);
		if !db.is_null() {
			unsafe {
				sqlite3_close(db);
			}
		}
		return Err(format!("sqlite3_open_v2 failed with code {rc}: {message}"));
	}

	for pragma in &[
		"PRAGMA page_size = 4096;",
		"PRAGMA journal_mode = DELETE;",
		"PRAGMA synchronous = NORMAL;",
		"PRAGMA temp_store = MEMORY;",
		"PRAGMA auto_vacuum = NONE;",
		"PRAGMA locking_mode = EXCLUSIVE;",
	] {
		if let Err(err) = sqlite_exec(db, pragma) {
			unsafe {
				sqlite3_close(db);
			}
			return Err(err);
		}
	}

	if let Err(err) = assert_batch_atomic_probe(db, &vfs) {
		unsafe {
			sqlite3_close(db);
		}
		return Err(err);
	}

	Ok(NativeDatabaseV2 { db, _vfs: vfs })
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
	use std::sync::{Arc, Mutex as StdMutex};
	use std::thread;

	use tempfile::TempDir;
	use tokio::runtime::Builder;
	use universaldb::Subspace;

	use super::*;

	static TEST_ID: AtomicU64 = AtomicU64::new(1);

	fn dirty_pages(page_count: u32, fill: u8) -> Vec<protocol::SqliteDirtyPage> {
		(0..page_count)
			.map(|offset| protocol::SqliteDirtyPage {
				pgno: offset + 1,
				bytes: vec![fill; 4096],
			})
			.collect()
	}

	fn next_test_name(prefix: &str) -> String {
		let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
		format!("{prefix}-{id}")
	}

	fn random_hex() -> String {
		let mut bytes = [0u8; 8];
		getrandom::getrandom(&mut bytes).expect("random bytes should be available");
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}

	struct DirectEngineHarness {
		actor_id: String,
		db_dir: TempDir,
		subspace: Subspace,
	}

	impl DirectEngineHarness {
		fn new() -> Self {
			Self {
				actor_id: next_test_name("sqlite-v2-direct-actor"),
				db_dir: tempfile::tempdir().expect("temp dir should build"),
				subspace: Subspace::new(&("sqlite-v2-direct", random_hex())),
			}
		}

		async fn open_engine(&self) -> Arc<SqliteEngine> {
			let driver =
				universaldb::driver::RocksDbDatabaseDriver::new(self.db_dir.path().to_path_buf())
					.await
					.expect("rocksdb driver should build");
			let db = Arc::new(universaldb::Database::new(Arc::new(driver)));
			let (engine, _compaction_rx) = SqliteEngine::new(db, self.subspace.clone());

			Arc::new(engine)
		}

		async fn startup_data_for(
			&self,
			actor_id: &str,
			engine: &SqliteEngine,
		) -> protocol::SqliteStartupData {
			let takeover = engine
				.takeover(
					actor_id,
					sqlite_storage::takeover::TakeoverConfig::new(
						sqlite_now_ms().expect("startup time should resolve"),
					),
				)
				.await
				.expect("takeover should succeed");

			protocol::SqliteStartupData {
				generation: takeover.generation,
				meta: protocol_sqlite_meta(takeover.meta),
				preloaded_pages: takeover
					.preloaded_pages
					.into_iter()
					.map(protocol_fetched_page)
					.collect(),
			}
		}

		async fn startup_data(&self, engine: &SqliteEngine) -> protocol::SqliteStartupData {
			self.startup_data_for(&self.actor_id, engine).await
		}

		fn open_db_on_engine(
			&self,
			runtime: &tokio::runtime::Runtime,
			engine: Arc<SqliteEngine>,
			actor_id: &str,
			config: VfsV2Config,
		) -> NativeDatabaseV2 {
			let startup = runtime.block_on(self.startup_data_for(actor_id, &engine));
			let vfs = SqliteVfsV2::register_with_transport(
				&next_test_name("sqlite-v2-direct-vfs"),
				SqliteTransport::from_direct(engine),
				actor_id.to_string(),
				runtime.handle().clone(),
				startup,
				config,
			)
			.expect("v2 vfs should register");

			open_database(vfs, actor_id).expect("sqlite database should open")
		}

		fn open_db(&self, runtime: &tokio::runtime::Runtime) -> NativeDatabaseV2 {
			let engine = runtime.block_on(self.open_engine());
			self.open_db_on_engine(runtime, engine, &self.actor_id, VfsV2Config::default())
		}
	}

	fn direct_vfs_ctx(db: &NativeDatabaseV2) -> &VfsV2Context {
		unsafe { &*db._vfs.ctx_ptr }
	}

	fn sqlite_query_i64(db: *mut sqlite3, sql: &str) -> std::result::Result<i64, String> {
		let c_sql = CString::new(sql).map_err(|err| err.to_string())?;
		let mut stmt = ptr::null_mut();
		let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
		if rc != SQLITE_OK {
			return Err(format!(
				"`{sql}` prepare failed with code {rc}: {}",
				sqlite_error_message(db)
			));
		}
		if stmt.is_null() {
			return Err(format!("`{sql}` returned no statement"));
		}

		let result = match unsafe { sqlite3_step(stmt) } {
			SQLITE_ROW => Ok(unsafe { sqlite3_column_int64(stmt, 0) }),
			step_rc => Err(format!(
				"`{sql}` step failed with code {step_rc}: {}",
				sqlite_error_message(db)
			)),
		};

		unsafe {
			sqlite3_finalize(stmt);
		}

		result
	}

	fn sqlite_query_text(db: *mut sqlite3, sql: &str) -> std::result::Result<String, String> {
		let c_sql = CString::new(sql).map_err(|err| err.to_string())?;
		let mut stmt = ptr::null_mut();
		let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
		if rc != SQLITE_OK {
			return Err(format!(
				"`{sql}` prepare failed with code {rc}: {}",
				sqlite_error_message(db)
			));
		}
		if stmt.is_null() {
			return Err(format!("`{sql}` returned no statement"));
		}

		let result = match unsafe { sqlite3_step(stmt) } {
			SQLITE_ROW => {
				let text_ptr = unsafe { sqlite3_column_text(stmt, 0) };
				if text_ptr.is_null() {
					Ok(String::new())
				} else {
					Ok(unsafe { CStr::from_ptr(text_ptr.cast()) }
						.to_string_lossy()
						.into_owned())
				}
			}
			step_rc => Err(format!(
				"`{sql}` step failed with code {step_rc}: {}",
				sqlite_error_message(db)
			)),
		};

		unsafe {
			sqlite3_finalize(stmt);
		}

		result
	}

	fn sqlite_file_control(db: *mut sqlite3, op: c_int) -> std::result::Result<c_int, String> {
		let main = CString::new("main").map_err(|err| err.to_string())?;
		let rc = unsafe { sqlite3_file_control(db, main.as_ptr(), op, ptr::null_mut()) };
		if rc != SQLITE_OK {
			return Err(format!(
				"sqlite3_file_control op {op} failed with code {rc}: {}",
				sqlite_error_message(db)
			));
		}

		Ok(rc)
	}

	fn direct_runtime() -> tokio::runtime::Runtime {
		Builder::new_multi_thread()
			.worker_threads(2)
			.enable_all()
			.build()
			.expect("runtime should build")
	}

	#[test]
	fn predictor_prefers_stride_after_repeated_reads() {
		let mut predictor = PrefetchPredictor::default();
		for pgno in [5, 8, 11, 14] {
			predictor.record(pgno);
		}

		assert_eq!(predictor.multi_predict(14, 3, 30), vec![17, 20, 23]);
	}

	#[test]
	fn startup_data_populates_cache_without_protocol_calls() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		));
		let startup = protocol::SqliteStartupData {
			generation: 3,
			meta: sqlite_meta(8 * 1024 * 1024),
			preloaded_pages: vec![protocol::SqliteFetchedPage {
				pgno: 1,
				bytes: Some(vec![7; 4096]),
			}],
		};

		let ctx = VfsV2Context::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			startup,
			VfsV2Config::default(),
			unsafe { std::mem::zeroed() },
		);

		assert_eq!(ctx.state.read().page_cache.get(&1), Some(vec![7; 4096]));
		assert!(protocol.get_pages_requests().is_empty());
	}

	#[test]
	fn direct_engine_supports_create_insert_select_and_user_version() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		assert_eq!(
			sqlite_file_control(db.as_ptr(), SQLITE_FCNTL_BEGIN_ATOMIC_WRITE)
				.expect("batch atomic begin should succeed"),
			SQLITE_OK
		);
		assert_eq!(
			sqlite_file_control(db.as_ptr(), SQLITE_FCNTL_COMMIT_ATOMIC_WRITE)
				.expect("batch atomic commit should succeed"),
			SQLITE_OK
		);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1, 'alpha');",
		)
		.expect("insert should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA user_version = 42;")
			.expect("user_version pragma should succeed");

		assert_eq!(
			sqlite_query_text(db.as_ptr(), "SELECT value FROM items WHERE id = 1;")
				.expect("select should succeed"),
			"alpha"
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("count should succeed"),
			1
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "PRAGMA user_version;")
				.expect("user_version read should succeed"),
			42
		);
	}

	#[test]
	fn direct_engine_handles_large_rows_and_multi_page_growth() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE blobs (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

		for _ in 0..48 {
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO blobs (payload) VALUES (randomblob(3500));",
			)
			.expect("seed insert should succeed");
		}
		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO blobs (payload) VALUES (randomblob(9000));",
		)
		.expect("large row insert should succeed");

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM blobs;")
				.expect("count should succeed"),
			49
		);
		assert!(
			sqlite_query_i64(db.as_ptr(), "PRAGMA page_count;").expect("page_count should succeed")
				> 20
		);
		assert!(
			sqlite_query_i64(db.as_ptr(), "SELECT max(length(payload)) FROM blobs;")
				.expect("max payload length should succeed")
				>= 9000
		);
	}

	#[test]
	fn direct_engine_persists_data_across_close_and_reopen() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();

		{
			let db = harness.open_db(&runtime);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE events (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
			)
			.expect("create table should succeed");
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO events (id, value) VALUES (1, 'persisted');",
			)
			.expect("insert should succeed");
			sqlite_exec(db.as_ptr(), "PRAGMA user_version = 7;")
				.expect("user_version write should succeed");
		}

		let reopened = harness.open_db(&runtime);
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT COUNT(*) FROM events;")
				.expect("count after reopen should succeed"),
			1
		);
		assert_eq!(
			sqlite_query_text(reopened.as_ptr(), "SELECT value FROM events WHERE id = 1;")
				.expect("value after reopen should succeed"),
			"persisted"
		);
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "PRAGMA user_version;")
				.expect("user_version after reopen should succeed"),
			7
		);
	}

	#[test]
	fn direct_engine_handles_aux_files_and_truncate_then_regrow() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(db.as_ptr(), "PRAGMA temp_store = FILE;")
			.expect("temp_store pragma should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE blobs (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

		for _ in 0..32 {
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO blobs (payload) VALUES (randomblob(8192));",
			)
			.expect("growth insert should succeed");
		}
		let grown_pages = sqlite_query_i64(db.as_ptr(), "PRAGMA page_count;")
			.expect("grown page_count should succeed");
		assert!(grown_pages > 40);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TEMP TABLE scratch AS SELECT id FROM blobs ORDER BY id DESC;",
		)
		.expect("temp table should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM scratch;")
				.expect("temp table count should succeed"),
			32
		);

		sqlite_exec(db.as_ptr(), "DELETE FROM blobs;").expect("delete should succeed");
		sqlite_exec(db.as_ptr(), "VACUUM;").expect("vacuum should succeed");
		let shrunk_pages = sqlite_query_i64(db.as_ptr(), "PRAGMA page_count;")
			.expect("shrunk page_count should succeed");
		assert!(shrunk_pages < grown_pages);

		for _ in 0..8 {
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO blobs (payload) VALUES (randomblob(8192));",
			)
			.expect("regrow insert should succeed");
		}
		let regrown_pages = sqlite_query_i64(db.as_ptr(), "PRAGMA page_count;")
			.expect("regrown page_count should succeed");
		assert!(regrown_pages > shrunk_pages);
	}

	#[test]
	fn direct_engine_batch_atomic_probe_runs_on_open() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		assert!(
			db._vfs.commit_atomic_count() > 0,
			"open_database should run the sqlite v2 batch-atomic probe",
		);
	}

	#[test]
	fn direct_engine_keeps_head_txid_after_cache_miss_reads_between_commits() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let db = harness.open_db_on_engine(
			&runtime,
			engine,
			&harness.actor_id,
			VfsV2Config {
				cache_capacity_pages: 2,
				prefetch_depth: 0,
				max_prefetch_bytes: 0,
				..VfsV2Config::default()
			},
		);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(db.as_ptr(), "CREATE INDEX items_value_idx ON items(value);")
			.expect("create index should succeed");
		for i in 0..120 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO items (id, value) VALUES ({}, 'item-{i:03}');",
					i + 1
				),
			)
			.expect("seed insert should succeed");
		}

		let ctx = direct_vfs_ctx(&db);
		let head_after_first_phase = ctx.state.read().head_txid;

		ctx.state.write().page_cache.invalidate_all();
		assert_eq!(
			sqlite_query_text(
				db.as_ptr(),
				"SELECT value FROM items WHERE value = 'item-091';",
			)
			.expect("cache-miss read should succeed"),
			"item-091"
		);
		let head_after_cache_miss = ctx.state.read().head_txid;
		assert_eq!(
			head_after_cache_miss, head_after_first_phase,
			"cache-miss reads must not rewind head_txid",
		);

		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1000, 'after-cache-miss');",
		)
		.expect("commit after cache-miss read should succeed");
		assert!(
			ctx.state.read().head_txid > head_after_cache_miss,
			"head_txid should still advance after the follow-up commit",
		);
	}

	#[test]
	fn direct_engine_uses_slow_path_for_large_real_engine_commits() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let startup = runtime.block_on(harness.startup_data(&engine));
		let dirty_pages = (1..=2300u32)
			.map(|pgno| protocol::SqliteDirtyPage {
				pgno,
				bytes: vec![(pgno % 251) as u8; 4096],
			})
			.collect::<Vec<_>>();

		let outcome = runtime
			.block_on(commit_buffered_pages(
				&SqliteTransport::from_direct(Arc::clone(&engine)),
				BufferedCommitRequest {
					actor_id: harness.actor_id.clone(),
					generation: startup.generation,
					expected_head_txid: startup.meta.head_txid,
					new_db_size_pages: 2300,
					max_delta_bytes: startup.meta.max_delta_bytes,
					max_pages_per_stage: 256,
					dirty_pages,
				},
			))
			.expect("slow-path direct commit should succeed");
		let (outcome, metrics) = outcome;

		assert_eq!(outcome.path, CommitPath::Slow);
		assert_eq!(outcome.new_head_txid, startup.meta.head_txid + 1);
		assert!(metrics.serialize_ns > 0);
		assert!(metrics.transport_ns > 0);

		let pages = runtime
			.block_on(engine.get_pages(&harness.actor_id, startup.generation, vec![1, 1024, 2300]))
			.expect("pages should read back after slow-path commit");
		let expected_page_1 = vec![1u8; 4096];
		let expected_page_1024 = vec![(1024 % 251) as u8; 4096];
		let expected_page_2300 = vec![(2300 % 251) as u8; 4096];
		assert_eq!(pages.len(), 3);
		assert_eq!(pages[0].bytes.as_deref(), Some(expected_page_1.as_slice()));
		assert_eq!(
			pages[1].bytes.as_deref(),
			Some(expected_page_1024.as_slice())
		);
		assert_eq!(
			pages[2].bytes.as_deref(),
			Some(expected_page_2300.as_slice())
		);
	}

	#[test]
	fn direct_engine_marks_vfs_dead_after_transport_errors() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let startup = runtime.block_on(harness.startup_data(&engine));
		let transport = SqliteTransport::from_direct(engine);
		let hooks = transport
			.direct_hooks()
			.expect("direct transport should expose test hooks");
		let vfs = SqliteVfsV2::register_with_transport(
			&next_test_name("sqlite-v2-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsV2Config::default(),
		)
		.expect("v2 vfs should register");
		let db = open_database(vfs, &harness.actor_id).expect("sqlite database should open");

		hooks.fail_next_commit("InjectedTransportError: commit transport dropped");
		let err = sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE broken (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect_err("failing transport commit should surface as an IO error");
		assert!(
			err.contains("I/O") || err.contains("disk I/O"),
			"sqlite should surface transport failure as an IO error: {err}",
		);
		assert!(
			direct_vfs_ctx(&db).is_dead(),
			"transport error should kill the v2 VFS"
		);
		assert_eq!(
			db.take_last_kv_error().as_deref(),
			Some("InjectedTransportError: commit transport dropped"),
		);
		assert!(
			sqlite_query_i64(db.as_ptr(), "PRAGMA page_count;").is_err(),
			"subsequent reads should fail once the VFS is dead",
		);
	}

	#[test]
	fn flush_dirty_pages_marks_vfs_dead_after_transport_error() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let startup = runtime.block_on(harness.startup_data(&engine));
		let transport = SqliteTransport::from_direct(engine);
		let hooks = transport
			.direct_hooks()
			.expect("direct transport should expose test hooks");
		let vfs = SqliteVfsV2::register_with_transport(
			&next_test_name("sqlite-v2-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsV2Config::default(),
		)
		.expect("v2 vfs should register");
		let db = open_database(vfs, &harness.actor_id).expect("sqlite database should open");
		let ctx = direct_vfs_ctx(&db);

		{
			let mut state = ctx.state.write();
			state.write_buffer.dirty.insert(1, vec![0x7a; 4096]);
			state.db_size_pages = 1;
		}

		hooks.fail_next_commit("InjectedTransportError: flush transport dropped");
		let err = ctx
			.flush_dirty_pages()
			.expect_err("transport failure should bubble out of flush_dirty_pages");

		assert!(
			matches!(err, CommitBufferError::Other(ref message) if message.contains("InjectedTransportError")),
			"flush failure should surface as a transport error: {err:?}",
		);
		assert!(
			ctx.is_dead(),
			"flush transport failure should poison the VFS"
		);
		assert_eq!(
			db.take_last_kv_error().as_deref(),
			Some("InjectedTransportError: flush transport dropped"),
		);
	}

	#[test]
	fn commit_atomic_write_marks_vfs_dead_after_transport_error() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let startup = runtime.block_on(harness.startup_data(&engine));
		let transport = SqliteTransport::from_direct(engine);
		let hooks = transport
			.direct_hooks()
			.expect("direct transport should expose test hooks");
		let vfs = SqliteVfsV2::register_with_transport(
			&next_test_name("sqlite-v2-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsV2Config::default(),
		)
		.expect("v2 vfs should register");
		let db = open_database(vfs, &harness.actor_id).expect("sqlite database should open");
		let ctx = direct_vfs_ctx(&db);

		{
			let mut state = ctx.state.write();
			state.write_buffer.in_atomic_write = true;
			state.write_buffer.saved_db_size = state.db_size_pages;
			state.write_buffer.dirty.insert(1, vec![0x5c; 4096]);
			state.db_size_pages = 1;
		}

		hooks.fail_next_commit("InjectedTransportError: atomic transport dropped");
		let err = ctx
			.commit_atomic_write()
			.expect_err("transport failure should bubble out of commit_atomic_write");

		assert!(
			matches!(err, CommitBufferError::Other(ref message) if message.contains("InjectedTransportError")),
			"atomic-write failure should surface as a transport error: {err:?}",
		);
		assert!(
			ctx.is_dead(),
			"commit_atomic_write transport failure should poison the VFS",
		);
		assert_eq!(
			db.take_last_kv_error().as_deref(),
			Some("InjectedTransportError: atomic transport dropped"),
		);
	}

	#[test]
	fn direct_engine_handles_multithreaded_statement_churn() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = Arc::new(StdMutex::new(harness.open_db(&runtime)));

		{
			let db = db.lock().expect("db mutex should lock");
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL);",
			)
			.expect("create table should succeed");
		}

		let mut workers = Vec::new();
		for worker_id in 0..4 {
			let db = Arc::clone(&db);
			workers.push(thread::spawn(move || {
				for idx in 0..40 {
					let db = db.lock().expect("db mutex should lock");
					sqlite_step_statement(
						db.as_ptr(),
						&format!(
							"INSERT INTO items (value) VALUES ('worker-{worker_id}-row-{idx}');"
						),
					)
					.expect("threaded insert should succeed");
				}
			}));
		}
		for worker in workers {
			worker.join().expect("worker thread should finish");
		}

		let db = db.lock().expect("db mutex should lock");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("threaded row count should succeed"),
			160
		);
	}

	#[test]
	fn direct_engine_isolates_two_actors_on_one_shared_engine() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let actor_a = next_test_name("sqlite-v2-actor-a");
		let actor_b = next_test_name("sqlite-v2-actor-b");
		let db_a = harness.open_db_on_engine(
			&runtime,
			Arc::clone(&engine),
			&actor_a,
			VfsV2Config::default(),
		);
		let db_b = harness.open_db_on_engine(&runtime, engine, &actor_b, VfsV2Config::default());

		sqlite_exec(
			db_a.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("actor A create table should succeed");
		sqlite_exec(
			db_b.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("actor B create table should succeed");
		sqlite_step_statement(
			db_a.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1, 'alpha');",
		)
		.expect("actor A insert should succeed");
		sqlite_step_statement(
			db_b.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1, 'beta');",
		)
		.expect("actor B insert should succeed");

		assert_eq!(
			sqlite_query_text(db_a.as_ptr(), "SELECT value FROM items WHERE id = 1;")
				.expect("actor A select should succeed"),
			"alpha"
		);
		assert_eq!(
			sqlite_query_text(db_b.as_ptr(), "SELECT value FROM items WHERE id = 1;")
				.expect("actor B select should succeed"),
			"beta"
		);
	}

	#[test]
	fn direct_engine_hot_row_updates_survive_reopen() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();

		{
			let db = harness.open_db(&runtime);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE counters (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
			)
			.expect("create table should succeed");
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO counters (id, value) VALUES (1, 'v-0');",
			)
			.expect("seed row should succeed");
			for i in 1..=150 {
				sqlite_step_statement(
					db.as_ptr(),
					&format!("UPDATE counters SET value = 'v-{i}' WHERE id = 1;"),
				)
				.expect("hot-row update should succeed");
			}
		}

		let reopened = harness.open_db(&runtime);
		assert_eq!(
			sqlite_query_text(
				reopened.as_ptr(),
				"SELECT value FROM counters WHERE id = 1;"
			)
			.expect("final value should survive reopen"),
			"v-150"
		);
	}

	#[test]
	fn direct_engine_preserves_mixed_workload_across_sleep_wake() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();

		{
			let db = harness.open_db(&runtime);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL, status TEXT NOT NULL);",
			)
			.expect("create table should succeed");
			for id in 1..=50 {
				sqlite_step_statement(
					db.as_ptr(),
					&format!(
						"INSERT INTO items (id, value, status) VALUES ({id}, 'item-{id}', 'new');"
					),
				)
				.expect("seed insert should succeed");
			}
			for id in 1..=20 {
				sqlite_step_statement(
					db.as_ptr(),
					&format!(
						"UPDATE items SET status = 'updated', value = 'item-{id}-updated' WHERE id = {id};"
					),
				)
				.expect("update should succeed");
			}
			for id in 41..=50 {
				sqlite_step_statement(db.as_ptr(), &format!("DELETE FROM items WHERE id = {id};"))
					.expect("delete should succeed");
			}
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO items (id, value, status) VALUES (1000, 'disconnect-write', 'new');",
			)
			.expect("disconnect-style write before close should succeed");
		}

		let reopened = harness.open_db(&runtime);
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("row count after reopen should succeed"),
			41
		);
		assert_eq!(
			sqlite_query_i64(
				reopened.as_ptr(),
				"SELECT COUNT(*) FROM items WHERE status = 'updated';",
			)
			.expect("updated row count should succeed"),
			20
		);
		assert_eq!(
			sqlite_query_text(
				reopened.as_ptr(),
				"SELECT value FROM items WHERE id = 1000;",
			)
			.expect("disconnect write should survive reopen"),
			"disconnect-write"
		);
	}

	#[test]
	fn direct_engine_reopens_cleanly_after_failed_migration() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();

		{
			let db = harness.open_db(&runtime);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
			)
			.expect("create table should succeed");
			sqlite_exec(db.as_ptr(), "ALTER TABLE items ADD COLUMN;")
				.expect_err("broken migration should fail");
		}

		let reopened = harness.open_db(&runtime);
		sqlite_step_statement(
			reopened.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1, 'still-alive');",
		)
		.expect("reopened database should still accept writes after migration failure");
		assert_eq!(
			sqlite_query_text(reopened.as_ptr(), "SELECT value FROM items WHERE id = 1;")
				.expect("select after reopen should succeed"),
			"still-alive"
		);
	}

	#[test]
	fn direct_engine_reads_continue_while_compaction_runs() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let db = Arc::new(StdMutex::new(harness.open_db_on_engine(
			&runtime,
			Arc::clone(&engine),
			&harness.actor_id,
			VfsV2Config::default(),
		)));

		{
			let db = db.lock().expect("db mutex should lock");
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
			)
			.expect("create table should succeed");
			for id in 1..=48 {
				sqlite_step_statement(
					db.as_ptr(),
					&format!("INSERT INTO items (id, value) VALUES ({id}, 'row-{id}');"),
				)
				.expect("seed insert should succeed");
			}
		}

		let keep_reading = Arc::new(AtomicBool::new(true));
		let read_error = Arc::new(StdMutex::new(None::<String>));
		let db_for_reader = Arc::clone(&db);
		let keep_reading_for_thread = Arc::clone(&keep_reading);
		let read_error_for_thread = Arc::clone(&read_error);
		let reader = thread::spawn(move || {
			while keep_reading_for_thread.load(AtomicOrdering::Relaxed) {
				let db = db_for_reader.lock().expect("db mutex should lock");
				direct_vfs_ctx(&db)
					.state
					.write()
					.page_cache
					.invalidate_all();
				if let Err(err) =
					sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items WHERE id >= 1;")
				{
					*read_error_for_thread
						.lock()
						.expect("read error mutex should lock") = Some(err);
					break;
				}
			}
		});

		runtime
			.block_on(engine.compact_worker(&harness.actor_id, 8))
			.expect("compaction should succeed");
		keep_reading.store(false, AtomicOrdering::Relaxed);
		reader.join().expect("reader thread should finish");

		assert!(
			read_error
				.lock()
				.expect("read error mutex should lock")
				.is_none(),
			"reads should keep working while compaction folds deltas",
		);
		let db = db.lock().expect("db mutex should lock");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("final row count should succeed"),
			48
		);
	}

	#[test]
	fn open_database_supports_empty_db_schema_setup() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: protocol::SqliteMeta {
					db_size_pages: 2,
					..sqlite_meta(8 * 1024 * 1024)
				},
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: protocol::SqliteMeta {
						db_size_pages: 2,
						..sqlite_meta(8 * 1024 * 1024)
					},
				},
			),
		));
		protocol.set_mirror_commit_meta(true);

		let vfs = SqliteVfsV2::register_with_transport(
			"test-v2-empty-db",
			SqliteTransport::from_mock(protocol.clone()),
			"actor".to_string(),
			runtime.handle().clone(),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 0,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
		)
		.expect("vfs should register");
		let db = open_database(vfs, "actor").expect("db should open");

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("schema setup should succeed");
	}

	#[test]
	fn open_database_supports_insert_after_pragma_migration() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: protocol::SqliteMeta {
					db_size_pages: 32,
					..sqlite_meta(8 * 1024 * 1024)
				},
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: protocol::SqliteMeta {
						db_size_pages: 32,
						..sqlite_meta(8 * 1024 * 1024)
					},
				},
			),
		));

		let vfs = SqliteVfsV2::register_with_transport(
			"test-v2-pragma-migration",
			SqliteTransport::from_mock(protocol.clone()),
			"actor".to_string(),
			runtime.handle().clone(),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 0,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
		)
		.expect("vfs should register");
		let db = open_database(vfs, "actor").expect("db should open");

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'active';",
		)
		.expect("alter table should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA user_version = 2;").expect("pragma should succeed");
		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO items (name) VALUES ('test-item');",
		)
		.expect("insert after pragma migration should succeed");
	}

	#[test]
	fn open_database_supports_explicit_status_insert_after_pragma_migration() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: protocol::SqliteMeta {
					db_size_pages: 32,
					..sqlite_meta(8 * 1024 * 1024)
				},
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: protocol::SqliteMeta {
						db_size_pages: 32,
						..sqlite_meta(8 * 1024 * 1024)
					},
				},
			),
		));
		protocol.set_mirror_commit_meta(true);

		let vfs = SqliteVfsV2::register_with_transport(
			"test-v2-pragma-explicit",
			SqliteTransport::from_mock(protocol),
			"actor".to_string(),
			runtime.handle().clone(),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 0,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
		)
		.expect("vfs should register");
		let db = open_database(vfs, "actor").expect("db should open");

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'active';",
		)
		.expect("alter table should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA user_version = 2;").expect("pragma should succeed");
		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO items (name, status) VALUES ('done-item', 'completed');",
		)
		.expect("explicit status insert should succeed");
	}

	#[test]
	fn open_database_supports_hot_row_update_churn() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: protocol::SqliteMeta {
					db_size_pages: 128,
					..sqlite_meta(8 * 1024 * 1024)
				},
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: protocol::SqliteMeta {
						db_size_pages: 128,
						..sqlite_meta(8 * 1024 * 1024)
					},
				},
			),
		));
		protocol.set_mirror_commit_meta(true);

		let vfs = SqliteVfsV2::register_with_transport(
			"test-v2-hot-row-updates",
			SqliteTransport::from_mock(protocol),
			"actor".to_string(),
			runtime.handle().clone(),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 0,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
		)
		.expect("vfs should register");
		let db = open_database(vfs, "actor").expect("db should open");

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE test_data (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL, payload TEXT NOT NULL DEFAULT '', created_at INTEGER NOT NULL);",
		)
		.expect("create table should succeed");
		for i in 0..10 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO test_data (value, payload, created_at) VALUES ('init-{i}', '', 1);"
				),
			)
			.expect("seed insert should succeed");
		}
		for i in 0..240 {
			let row_id = i % 10 + 1;
			sqlite_step_statement(
				db.as_ptr(),
				&format!("UPDATE test_data SET value = 'v-{i}' WHERE id = {row_id};"),
			)
			.expect("hot-row update should succeed");
		}
	}

	#[test]
	fn open_database_supports_cross_thread_exec_sequence() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: protocol::SqliteMeta {
					db_size_pages: 32,
					..sqlite_meta(8 * 1024 * 1024)
				},
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: protocol::SqliteMeta {
						db_size_pages: 32,
						..sqlite_meta(8 * 1024 * 1024)
					},
				},
			),
		));
		protocol.set_mirror_commit_meta(true);

		let vfs = SqliteVfsV2::register_with_transport(
			"test-v2-cross-thread",
			SqliteTransport::from_mock(protocol),
			"actor".to_string(),
			runtime.handle().clone(),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 0,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
		)
		.expect("vfs should register");
		let db = Arc::new(StdMutex::new(
			open_database(vfs, "actor").expect("db should open"),
		));

		{
			let db = db.clone();
			thread::spawn(move || {
				let db = db.lock().expect("db mutex should lock");
				sqlite_exec(
					db.as_ptr(),
					"CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
				)
				.expect("create table should succeed");
				sqlite_exec(
					db.as_ptr(),
					"ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'active';",
				)
				.expect("alter table should succeed");
				sqlite_exec(db.as_ptr(), "PRAGMA user_version = 2;")
					.expect("pragma should succeed");
			})
			.join()
			.expect("migration thread should finish");
		}

		thread::spawn(move || {
			let db = db.lock().expect("db mutex should lock");
			sqlite_step_statement(
				db.as_ptr(),
				"INSERT INTO items (name) VALUES ('test-item');",
			)
			.expect("cross-thread insert should succeed");
		})
		.join()
		.expect("insert thread should finish");
	}

	#[test]
	fn aux_files_are_shared_by_path_until_deleted() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		));
		let ctx = VfsV2Context::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: sqlite_meta(8 * 1024 * 1024),
				preloaded_pages: Vec::new(),
			},
			VfsV2Config::default(),
			unsafe { std::mem::zeroed() },
		);

		let first = ctx.open_aux_file("actor-journal");
		first.bytes.lock().extend_from_slice(&[1, 2, 3, 4]);
		let second = ctx.open_aux_file("actor-journal");
		assert_eq!(*second.bytes.lock(), vec![1, 2, 3, 4]);
		assert!(ctx.aux_file_exists("actor-journal"));

		ctx.delete_aux_file("actor-journal");
		assert!(!ctx.aux_file_exists("actor-journal"));
		assert!(ctx.open_aux_file("actor-journal").bytes.lock().is_empty());
	}

	#[test]
	fn truncate_main_file_discards_pages_beyond_eof() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		));
		let ctx = VfsV2Context::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 4,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: vec![
					protocol::SqliteFetchedPage {
						pgno: 1,
						bytes: Some(vec![1; 4096]),
					},
					protocol::SqliteFetchedPage {
						pgno: 4,
						bytes: Some(vec![4; 4096]),
					},
				],
			},
			VfsV2Config::default(),
			unsafe { std::mem::zeroed() },
		);
		{
			let mut state = ctx.state.write();
			state.write_buffer.dirty.insert(3, vec![3; 4096]);
			state.write_buffer.dirty.insert(4, vec![4; 4096]);
		}

		ctx.truncate_main_file(2 * 4096);

		let state = ctx.state.read();
		assert_eq!(state.db_size_pages, 2);
		assert!(!state.write_buffer.dirty.contains_key(&3));
		assert!(!state.write_buffer.dirty.contains_key(&4));
		assert!(state.page_cache.get(&4).is_none());
	}

	#[test]
	fn resolve_pages_does_not_rewind_meta_on_stale_response() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let mut protocol = MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		);
		protocol.get_pages_response =
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(protocol::SqliteGetPagesOk {
				pages: vec![protocol::SqliteFetchedPage {
					pgno: 2,
					bytes: Some(vec![2; 4096]),
				}],
				meta: protocol::SqliteMeta {
					head_txid: 1,
					db_size_pages: 1,
					max_delta_bytes: 32 * 1024 * 1024,
					..sqlite_meta(8 * 1024 * 1024)
				},
			});
		let ctx = VfsV2Context::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(Arc::new(protocol)),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					head_txid: 3,
					db_size_pages: 3,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: vec![protocol::SqliteFetchedPage {
					pgno: 1,
					bytes: Some(vec![1; 4096]),
				}],
			},
			VfsV2Config::default(),
			unsafe { std::mem::zeroed() },
		);

		let resolved = ctx
			.resolve_pages(&[2], false)
			.expect("missing page should resolve");

		assert_eq!(resolved.get(&2), Some(&Some(vec![2; 4096])));
		let state = ctx.state.read();
		assert_eq!(state.head_txid, 3);
		assert_eq!(state.db_size_pages, 3);
		assert_eq!(state.max_delta_bytes, 32 * 1024 * 1024);
	}

	#[test]
	fn resolve_pages_does_not_shrink_db_size_pages_on_same_head_response() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let mut protocol = MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 13,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		);
		protocol.get_pages_response =
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(protocol::SqliteGetPagesOk {
				pages: vec![protocol::SqliteFetchedPage {
					pgno: 4,
					bytes: Some(vec![4; 4096]),
				}],
				meta: protocol::SqliteMeta {
					head_txid: 3,
					db_size_pages: 1,
					max_delta_bytes: 16 * 1024 * 1024,
					..sqlite_meta(8 * 1024 * 1024)
				},
			});
		let ctx = VfsV2Context::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(Arc::new(protocol)),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					head_txid: 3,
					db_size_pages: 4,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: vec![protocol::SqliteFetchedPage {
					pgno: 1,
					bytes: Some(vec![1; 4096]),
				}],
			},
			VfsV2Config::default(),
			unsafe { std::mem::zeroed() },
		);

		let resolved = ctx
			.resolve_pages(&[4], false)
			.expect("missing page should resolve");

		assert_eq!(resolved.get(&4), Some(&Some(vec![4; 4096])));
		let state = ctx.state.read();
		assert_eq!(state.head_txid, 3);
		assert_eq!(state.db_size_pages, 4);
		assert_eq!(state.max_delta_bytes, 16 * 1024 * 1024);
	}

	#[test]
	fn commit_buffered_pages_uses_fast_path() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitOk(protocol::SqliteCommitOk {
				new_head_txid: 13,
				meta: sqlite_meta(8 * 1024 * 1024),
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 14,
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		));

		let outcome = runtime
			.block_on(commit_buffered_pages(
				&SqliteTransport::from_mock(protocol.clone()),
				BufferedCommitRequest {
					actor_id: "actor".to_string(),
					generation: 7,
					expected_head_txid: 12,
					new_db_size_pages: 1,
					max_delta_bytes: 8 * 1024 * 1024,
					max_pages_per_stage: 4_000,
					dirty_pages: dirty_pages(1, 9),
				},
			))
			.expect("fast-path commit should succeed");
		let (outcome, metrics) = outcome;

		assert_eq!(outcome.path, CommitPath::Fast);
		assert_eq!(outcome.new_head_txid, 13);
		assert!(metrics.serialize_ns > 0);
		assert!(metrics.transport_ns > 0);
		assert_eq!(protocol.commit_requests().len(), 1);
		assert!(protocol.stage_requests().is_empty());
		assert!(protocol.finalize_requests().is_empty());
	}

	#[test]
	fn commit_buffered_pages_falls_back_to_slow_path() {
		let runtime = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let protocol = Arc::new(MockProtocol::new(
			protocol::SqliteCommitResponse::SqliteCommitTooLarge(protocol::SqliteCommitTooLarge {
				actual_size_bytes: 3 * 4096,
				max_size_bytes: 4096,
			}),
			protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
				protocol::SqliteCommitStageOk {
					chunk_idx_committed: 0,
				},
			),
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: 14,
					meta: sqlite_meta(4096),
				},
			),
		));

		let protocol_for_release = protocol.clone();
		let release = std::thread::spawn(move || {
			runtime.block_on(async {
				protocol_for_release.finalize_started.notified().await;
				assert_eq!(protocol_for_release.stage_requests().len(), 3);
				assert_eq!(protocol_for_release.awaited_stage_responses(), 0);
				protocol_for_release.release_finalize.notify_one();
			});
		});

		let outcome = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build")
			.block_on(commit_buffered_pages(
				&SqliteTransport::from_mock(protocol.clone()),
				BufferedCommitRequest {
					actor_id: "actor".to_string(),
					generation: 7,
					expected_head_txid: 12,
					new_db_size_pages: 3,
					max_delta_bytes: 4096,
					max_pages_per_stage: 1,
					dirty_pages: dirty_pages(3, 4),
				},
			))
			.expect("slow-path commit should succeed");
		let (outcome, metrics) = outcome;

		release.join().expect("release thread should finish");

		assert_eq!(outcome.path, CommitPath::Slow);
		assert_eq!(outcome.new_head_txid, 14);
		assert!(metrics.serialize_ns > 0);
		assert!(metrics.transport_ns > 0);
		assert!(protocol.commit_requests().is_empty());
		assert_eq!(protocol.stage_requests().len(), 3);
		assert_eq!(protocol.awaited_stage_responses(), 0);
		assert_eq!(protocol.finalize_requests().len(), 1);
	}

	#[test]
	fn vfs_records_commit_phase_durations() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE metrics_test (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");

		let relaxed = std::sync::atomic::Ordering::Relaxed;
		ctx.commit_request_build_ns.store(0, relaxed);
		ctx.commit_serialize_ns.store(0, relaxed);
		ctx.commit_transport_ns.store(0, relaxed);
		ctx.commit_state_update_ns.store(0, relaxed);
		ctx.commit_duration_ns_total.store(0, relaxed);
		ctx.commit_total.store(0, relaxed);

		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO metrics_test (id, value) VALUES (1, 'hello');",
		)
		.expect("insert should succeed");

		let metrics = db.sqlite_vfs_metrics();
		assert_eq!(metrics.commit_count, 1);
		assert!(metrics.request_build_ns > 0);
		assert!(metrics.serialize_ns > 0);
		assert!(metrics.transport_ns > 0);
		assert!(metrics.state_update_ns > 0);
		assert!(metrics.total_ns >= metrics.request_build_ns);
		assert!(metrics.request_build_ns + metrics.transport_ns + metrics.state_update_ns > 0);
	}

	#[test]
	fn profile_large_tx_insert_5mb() {
		// 5MB = 1280 rows x 4KB blobs in one transaction
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE bench (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

		let relaxed = std::sync::atomic::Ordering::Relaxed;
		ctx.resolve_pages_total.store(0, relaxed);
		ctx.resolve_pages_cache_hits.store(0, relaxed);
		ctx.resolve_pages_fetches.store(0, relaxed);
		ctx.pages_fetched_total.store(0, relaxed);
		ctx.prefetch_pages_total.store(0, relaxed);
		ctx.commit_total.store(0, relaxed);

		let start = std::time::Instant::now();
		sqlite_exec(db.as_ptr(), "BEGIN;").expect("begin");
		for i in 0..1280 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO bench (id, payload) VALUES ({}, randomblob(4096));",
					i
				),
			)
			.expect("insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "COMMIT;").expect("commit");
		let elapsed = start.elapsed();

		let resolve_total = ctx.resolve_pages_total.load(relaxed);
		let cache_hits = ctx.resolve_pages_cache_hits.load(relaxed);
		let fetches = ctx.resolve_pages_fetches.load(relaxed);
		let pages_fetched = ctx.pages_fetched_total.load(relaxed);
		let prefetch = ctx.prefetch_pages_total.load(relaxed);
		let commits = ctx.commit_total.load(relaxed);

		eprintln!("=== 5MB INSERT PROFILE (1280 rows x 4KB) ===");
		eprintln!("  wall clock:           {:?}", elapsed);
		eprintln!("  resolve_pages calls:  {}", resolve_total);
		eprintln!("  cache hits (pages):   {}", cache_hits);
		eprintln!("  engine fetches:       {}", fetches);
		eprintln!("  pages fetched total:  {}", pages_fetched);
		eprintln!("  prefetch pages:       {}", prefetch);
		eprintln!("  commits:              {}", commits);
		eprintln!("============================================");

		// In a single transaction, all 1280 row writes are to new pages.
		// Only the single commit at the end should hit the engine.
		assert_eq!(
			fetches, 0,
			"expected 0 engine fetches during 5MB insert transaction"
		);
		assert_eq!(
			commits, 1,
			"expected exactly 1 commit for transactional insert"
		);

		let count = sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM bench;")
			.expect("count should succeed");
		assert_eq!(count, 1280);
	}

	#[test]
	fn profile_hot_row_updates() {
		// 100 updates to the same row - this is the autocommit case
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE counter (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
		)
		.expect("create");
		sqlite_exec(db.as_ptr(), "INSERT INTO counter VALUES (1, 0);").expect("insert");

		let relaxed = std::sync::atomic::Ordering::Relaxed;
		ctx.resolve_pages_total.store(0, relaxed);
		ctx.resolve_pages_cache_hits.store(0, relaxed);
		ctx.resolve_pages_fetches.store(0, relaxed);
		ctx.pages_fetched_total.store(0, relaxed);
		ctx.prefetch_pages_total.store(0, relaxed);
		ctx.commit_total.store(0, relaxed);

		let start = std::time::Instant::now();
		for _ in 0..100 {
			sqlite_exec(
				db.as_ptr(),
				"UPDATE counter SET value = value + 1 WHERE id = 1;",
			)
			.expect("update");
		}
		let elapsed = start.elapsed();

		let fetches = ctx.resolve_pages_fetches.load(relaxed);
		let commits = ctx.commit_total.load(relaxed);

		eprintln!("=== 100 HOT ROW UPDATES (autocommit) ===");
		eprintln!("  wall clock:           {:?}", elapsed);
		eprintln!(
			"  resolve_pages calls:  {}",
			ctx.resolve_pages_total.load(relaxed)
		);
		eprintln!(
			"  cache hits (pages):   {}",
			ctx.resolve_pages_cache_hits.load(relaxed)
		);
		eprintln!("  engine fetches:       {}", fetches);
		eprintln!(
			"  pages fetched total:  {}",
			ctx.pages_fetched_total.load(relaxed)
		);
		eprintln!(
			"  prefetch pages:       {}",
			ctx.prefetch_pages_total.load(relaxed)
		);
		eprintln!("  commits:              {}", commits);
		eprintln!("=========================================");

		// Hot row updates: each update modifies the same page. Pages already
		// in write_buffer or cache should not need re-fetching. With the
		// counter's page(s) already warm, subsequent updates should be
		// 100% cache hits (0 fetches). Autocommit means 100 separate commits.
		assert_eq!(
			fetches, 0,
			"expected 0 engine fetches for 100 hot row updates"
		);
		assert_eq!(
			commits, 100,
			"expected 100 commits (autocommit per statement)"
		);
	}

	#[test]
	fn profile_large_tx_insert_1mb_preloaded() {
		// Same as the 1MB test but preload all pages first to see commit-only cost
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let actor_id = &harness.actor_id;

		// First pass: create and populate the table to generate pages
		let db1 =
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsV2Config::default());
		sqlite_exec(
			db1.as_ptr(),
			"CREATE TABLE bench (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(db1.as_ptr(), "BEGIN;").expect("begin");
		for i in 0..256 {
			sqlite_step_statement(
				db1.as_ptr(),
				&format!(
					"INSERT INTO bench (id, payload) VALUES ({}, randomblob(4096));",
					i
				),
			)
			.expect("insert should succeed");
		}
		sqlite_exec(db1.as_ptr(), "COMMIT;").expect("commit");
		drop(db1);

		// Second pass: reopen with warm cache (takeover preloads page 1, rest from reads)
		let db2 =
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsV2Config::default());
		let ctx = direct_vfs_ctx(&db2);

		// Warm the cache by reading everything
		sqlite_exec(db2.as_ptr(), "SELECT COUNT(*) FROM bench;").expect("count");

		// Reset counters
		let relaxed = std::sync::atomic::Ordering::Relaxed;
		ctx.resolve_pages_total.store(0, relaxed);
		ctx.resolve_pages_cache_hits.store(0, relaxed);
		ctx.resolve_pages_fetches.store(0, relaxed);
		ctx.pages_fetched_total.store(0, relaxed);
		ctx.prefetch_pages_total.store(0, relaxed);
		ctx.commit_total.store(0, relaxed);

		let start = std::time::Instant::now();
		sqlite_exec(db2.as_ptr(), "BEGIN;").expect("begin");
		for i in 256..512 {
			sqlite_step_statement(
				db2.as_ptr(),
				&format!(
					"INSERT INTO bench (id, payload) VALUES ({}, randomblob(4096));",
					i
				),
			)
			.expect("insert should succeed");
		}
		sqlite_exec(db2.as_ptr(), "COMMIT;").expect("commit");
		let elapsed = start.elapsed();

		let resolve_total = ctx.resolve_pages_total.load(relaxed);
		let cache_hits = ctx.resolve_pages_cache_hits.load(relaxed);
		let fetches = ctx.resolve_pages_fetches.load(relaxed);
		let pages_fetched = ctx.pages_fetched_total.load(relaxed);
		let prefetch = ctx.prefetch_pages_total.load(relaxed);
		let commits = ctx.commit_total.load(relaxed);

		eprintln!("=== 1MB INSERT PROFILE (WARM CACHE) ===");
		eprintln!("  wall clock:           {:?}", elapsed);
		eprintln!("  resolve_pages calls:  {}", resolve_total);
		eprintln!("  cache hits (pages):   {}", cache_hits);
		eprintln!("  engine fetches:       {}", fetches);
		eprintln!("  pages fetched total:  {}", pages_fetched);
		eprintln!("  prefetch pages:       {}", prefetch);
		eprintln!("  commits:              {}", commits);
		eprintln!("========================================");

		// Second 256-row transaction into the already-populated table.
		// All new pages are beyond db_size_pages, so no engine fetches.
		assert_eq!(
			fetches, 0,
			"expected 0 engine fetches during warm 1MB insert"
		);
		assert_eq!(
			commits, 1,
			"expected exactly 1 commit for transactional insert"
		);

		let count = sqlite_query_i64(db2.as_ptr(), "SELECT COUNT(*) FROM bench;")
			.expect("count should succeed");
		assert_eq!(count, 512);
	}

	#[test]
	fn profile_large_tx_insert_1mb() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE bench (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

		// Reset counters after schema setup
		ctx.resolve_pages_total
			.store(0, std::sync::atomic::Ordering::Relaxed);
		ctx.resolve_pages_cache_hits
			.store(0, std::sync::atomic::Ordering::Relaxed);
		ctx.resolve_pages_fetches
			.store(0, std::sync::atomic::Ordering::Relaxed);
		ctx.pages_fetched_total
			.store(0, std::sync::atomic::Ordering::Relaxed);
		ctx.prefetch_pages_total
			.store(0, std::sync::atomic::Ordering::Relaxed);
		ctx.commit_total
			.store(0, std::sync::atomic::Ordering::Relaxed);

		let start = std::time::Instant::now();

		sqlite_exec(db.as_ptr(), "BEGIN;").expect("begin should succeed");
		for i in 0..256 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO bench (id, payload) VALUES ({}, randomblob(4096));",
					i
				),
			)
			.expect("insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "COMMIT;").expect("commit should succeed");

		let elapsed = start.elapsed();
		let relaxed = std::sync::atomic::Ordering::Relaxed;

		let resolve_total = ctx.resolve_pages_total.load(relaxed);
		let cache_hits = ctx.resolve_pages_cache_hits.load(relaxed);
		let fetches = ctx.resolve_pages_fetches.load(relaxed);
		let pages_fetched = ctx.pages_fetched_total.load(relaxed);
		let prefetch = ctx.prefetch_pages_total.load(relaxed);
		let commits = ctx.commit_total.load(relaxed);

		eprintln!("=== 1MB INSERT PROFILE (256 rows x 4KB) ===");
		eprintln!("  wall clock:           {:?}", elapsed);
		eprintln!("  resolve_pages calls:  {}", resolve_total);
		eprintln!("  cache hits (pages):   {}", cache_hits);
		eprintln!("  engine fetches:       {}", fetches);
		eprintln!("  pages fetched total:  {}", pages_fetched);
		eprintln!("  prefetch pages:       {}", prefetch);
		eprintln!("  commits:              {}", commits);
		eprintln!("============================================");

		// Assert expected zero-fetch behavior: in a single transaction,
		// all writes are to new pages, so no engine fetches should happen.
		// Only the single commit at the end should hit the engine.
		assert_eq!(
			fetches, 0,
			"expected 0 engine fetches during 1MB insert transaction"
		);
		assert_eq!(
			commits, 1,
			"expected exactly 1 commit for transactional insert"
		);

		let count = sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM bench;")
			.expect("count should succeed");
		assert_eq!(count, 256);
	}

	// Regression test for fence mismatch during rapid autocommit inserts.
	// Each autocommit INSERT is its own transaction. This test drives many
	// sequential commits through the VFS and verifies they all succeed.
	#[test]
	fn autocommit_inserts_maintain_head_txid_consistency() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
		)
		.expect("create table should succeed");

		let relaxed = std::sync::atomic::Ordering::Relaxed;
		ctx.commit_total.store(0, relaxed);

		// 100 sequential autocommit inserts. If fence mismatch is the bug,
		// this will fail partway through with "commit head_txid X did not
		// match current head_txid X-1".
		for i in 0..100 {
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO t (id, v) VALUES ({i}, {});", i * 2),
			)
			.expect("autocommit insert should not fence-mismatch");
		}

		let commits = ctx.commit_total.load(relaxed);
		// Each autocommit INSERT = 1 commit. CREATE TABLE was 1 more.
		// We reset commit_total after CREATE, so expect 100.
		assert_eq!(commits, 100, "expected exactly 100 commits");

		let count =
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM t;").expect("count should succeed");
		assert_eq!(count, 100);

		// Verify the sum to make sure data is correct and not corrupted
		let sum =
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(v) FROM t;").expect("sum should succeed");
		assert_eq!(sum, (0..100).map(|i| i * 2).sum::<i64>());
	}

	// Regression test: 5 actors run 200 autocommits each on the same engine.
	// Compaction is triggered via the mpsc channel after each commit, so this
	// also exercises the commit-vs-compaction race that caused fence rewinds
	// before the tx_get_value_serializable fix.
	#[test]
	fn stress_concurrent_multi_actor_autocommits() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());

		let mut dbs = Vec::new();
		for i in 0..5 {
			let actor_id = format!("{}-stress-{}", harness.actor_id, i);
			let db = harness.open_db_on_engine(
				&runtime,
				engine.clone(),
				&actor_id,
				VfsV2Config::default(),
			);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
			)
			.expect("create");
			dbs.push(db);
		}

		// Interleave 200 autocommit inserts across all 5 actors
		for i in 0..200 {
			for db in &dbs {
				sqlite_exec(
					db.as_ptr(),
					&format!("INSERT INTO t (id, v) VALUES ({i}, {i});"),
				)
				.expect("insert");
			}
		}

		for db in &dbs {
			let count = sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM t;").expect("count");
			assert_eq!(count, 200);
		}
	}

	// Regression test: two actors run autocommits concurrently on the same
	// SqliteEngine. If anything in the engine (e.g., compaction) cross-contaminates
	// actors or races on shared state, we'd see fence mismatches.
	#[test]
	fn concurrent_multi_actor_autocommits() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());

		let actor_a = format!("{}-a", harness.actor_id);
		let actor_b = format!("{}-b", harness.actor_id);

		let db_a =
			harness.open_db_on_engine(&runtime, engine.clone(), &actor_a, VfsV2Config::default());
		let db_b =
			harness.open_db_on_engine(&runtime, engine.clone(), &actor_b, VfsV2Config::default());

		sqlite_exec(
			db_a.as_ptr(),
			"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
		)
		.expect("create a");
		sqlite_exec(
			db_b.as_ptr(),
			"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
		)
		.expect("create b");

		// Run 100 autocommits on each actor, interleaved.
		for i in 0..100 {
			sqlite_exec(
				db_a.as_ptr(),
				&format!("INSERT INTO t (id, v) VALUES ({i}, {i});"),
			)
			.expect("insert a");
			sqlite_exec(
				db_b.as_ptr(),
				&format!("INSERT INTO t (id, v) VALUES ({i}, {i});"),
			)
			.expect("insert b");
		}

		let count_a = sqlite_query_i64(db_a.as_ptr(), "SELECT COUNT(*) FROM t;").expect("count a");
		assert_eq!(count_a, 100);
		let count_b = sqlite_query_i64(db_b.as_ptr(), "SELECT COUNT(*) FROM t;").expect("count b");
		assert_eq!(count_b, 100);
	}

	// Same as above but across a close/reopen cycle to exercise takeover.
	#[test]
	fn autocommit_survives_close_reopen() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let actor_id = &harness.actor_id;

		{
			let db = harness.open_db_on_engine(
				&runtime,
				engine.clone(),
				actor_id,
				VfsV2Config::default(),
			);
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
			)
			.expect("create table");
			for i in 0..50 {
				sqlite_exec(
					db.as_ptr(),
					&format!("INSERT INTO t (id, v) VALUES ({i}, {});", i),
				)
				.expect("insert");
			}
		}

		// Reopen (triggers takeover which bumps generation)
		let db2 =
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsV2Config::default());
		for i in 50..100 {
			sqlite_exec(
				db2.as_ptr(),
				&format!("INSERT INTO t (id, v) VALUES ({i}, {});", i),
			)
			.expect("insert after reopen");
		}

		let count = sqlite_query_i64(db2.as_ptr(), "SELECT COUNT(*) FROM t;")
			.expect("count should succeed");
		assert_eq!(count, 100);
	}

	// Bench-parity tests. Each mirrors a workload in
	// examples/kitchen-sink/src/actors/testing/test-sqlite-bench.ts so
	// storage-layer regressions surface here without needing the full stack.

	fn open_bench_db(runtime: &tokio::runtime::Runtime) -> NativeDatabaseV2 {
		let harness = DirectEngineHarness::new();
		harness.open_db(runtime)
	}

	#[test]
	fn bench_insert_tx_x10000() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER);",
		)
		.unwrap();

		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..10_000 {
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO t (id, v) VALUES ({i}, {i});"),
			)
			.unwrap();
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM t;").unwrap(),
			10_000
		);
	}

	#[test]
	fn bench_large_tx_insert_500kb() {
		large_tx_insert(500 * 1024);
	}

	#[test]
	fn bench_large_tx_insert_10mb() {
		large_tx_insert(10 * 1024 * 1024);
	}

	#[test]
	fn bench_large_tx_insert_50mb() {
		// 50MB exercises the slow-path stage/finalize chunking that has
		// historically hit decode errors under certain transports.
		large_tx_insert(50 * 1024 * 1024);
	}

	fn large_tx_insert(target_bytes: usize) {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE large_tx (id INTEGER PRIMARY KEY AUTOINCREMENT, payload BLOB NOT NULL);",
		)
		.unwrap();

		let row_size = 4 * 1024;
		let rows = (target_bytes + row_size - 1) / row_size;
		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for _ in 0..rows {
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO large_tx (payload) VALUES (randomblob({row_size}));"),
			)
			.unwrap();
		}
		if let Err(err) = sqlite_exec(db.as_ptr(), "COMMIT") {
			let vfs_err = direct_vfs_ctx(&db).clone_last_error();
			panic!(
				"COMMIT failed for {} MiB: sqlite={}, vfs_last_error={:?}",
				target_bytes / (1024 * 1024),
				err,
				vfs_err,
			);
		}

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM large_tx;").unwrap(),
			rows as i64
		);
	}

	#[test]
	fn bench_churn_insert_delete_10x1000() {
		// Tests freelist reuse / space reclamation under heavy churn.
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE churn (id INTEGER PRIMARY KEY AUTOINCREMENT, payload BLOB NOT NULL);",
		)
		.unwrap();
		for _ in 0..10 {
			sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
			for _ in 0..1000 {
				sqlite_exec(
					db.as_ptr(),
					"INSERT INTO churn (payload) VALUES (randomblob(1024));",
				)
				.unwrap();
			}
			sqlite_exec(db.as_ptr(), "DELETE FROM churn;").unwrap();
			sqlite_exec(db.as_ptr(), "COMMIT").unwrap();
		}
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM churn;").unwrap(),
			0
		);
	}

	#[test]
	fn bench_mixed_oltp_large() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE mixed (id INTEGER PRIMARY KEY, v INTEGER NOT NULL, data BLOB NOT NULL);",
		)
		.unwrap();

		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..500 {
			sqlite_exec(
				db.as_ptr(),
				&format!(
					"INSERT INTO mixed (id, v, data) VALUES ({i}, {}, randomblob(1024));",
					i * 2
				),
			)
			.unwrap();
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..500 {
			sqlite_exec(
				db.as_ptr(),
				&format!(
					"INSERT INTO mixed (id, v, data) VALUES ({}, {}, randomblob(1024));",
					500 + i,
					i * 3
				),
			)
			.unwrap();
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE mixed SET v = v + 1 WHERE id = {i};"),
			)
			.unwrap();
			if i % 5 == 0 && i >= 50 {
				sqlite_exec(
					db.as_ptr(),
					&format!("DELETE FROM mixed WHERE id = {};", i - 50),
				)
				.unwrap();
			}
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		let count = sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM mixed;").unwrap();
		assert!(count > 900 && count < 1000);
	}

	#[test]
	fn bench_bulk_update_1000_rows() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE bulk (id INTEGER PRIMARY KEY, v INTEGER);",
		)
		.unwrap();
		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..1000 {
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO bulk (id, v) VALUES ({i}, {i});"),
			)
			.unwrap();
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..1000 {
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE bulk SET v = v + 1 WHERE id = {i};"),
			)
			.unwrap();
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(v) FROM bulk;").unwrap(),
			(0..1000).map(|i| i + 1).sum::<i64>()
		);
	}

	#[test]
	fn bench_truncate_and_regrow() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE regrow (id INTEGER PRIMARY KEY AUTOINCREMENT, payload BLOB NOT NULL);",
		)
		.unwrap();
		for _ in 0..2 {
			sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
			for _ in 0..500 {
				sqlite_exec(
					db.as_ptr(),
					"INSERT INTO regrow (payload) VALUES (randomblob(1024));",
				)
				.unwrap();
			}
			sqlite_exec(db.as_ptr(), "COMMIT").unwrap();
			sqlite_exec(db.as_ptr(), "DELETE FROM regrow;").unwrap();
		}
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM regrow;").unwrap(),
			0
		);
	}

	#[test]
	fn bench_many_small_tables() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..50 {
			sqlite_exec(
				db.as_ptr(),
				&format!("CREATE TABLE t_{i} (id INTEGER PRIMARY KEY, v INTEGER);"),
			)
			.unwrap();
			for j in 0..10 {
				sqlite_exec(
					db.as_ptr(),
					&format!("INSERT INTO t_{i} (id, v) VALUES ({j}, {});", i * j),
				)
				.unwrap();
			}
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		let total: i64 = (0..50)
			.map(|i| {
				sqlite_query_i64(db.as_ptr(), &format!("SELECT COUNT(*) FROM t_{i};")).unwrap()
			})
			.sum();
		assert_eq!(total, 500);
	}

	#[test]
	fn bench_index_creation_on_10k_rows() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE idx_test (id INTEGER PRIMARY KEY AUTOINCREMENT, k TEXT NOT NULL, v INTEGER NOT NULL);",
		)
		.unwrap();
		sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
		for i in 0..10_000 {
			sqlite_exec(
				db.as_ptr(),
				&format!(
					"INSERT INTO idx_test (k, v) VALUES ('key-{}-{i}', {i});",
					i % 1000
				),
			)
			.unwrap();
		}
		sqlite_exec(db.as_ptr(), "COMMIT").unwrap();

		sqlite_exec(db.as_ptr(), "CREATE INDEX idx_test_k ON idx_test(k);").unwrap();

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM idx_test;").unwrap(),
			10_000
		);
	}

	#[test]
	fn bench_growing_aggregation() {
		let runtime = direct_runtime();
		let db = open_bench_db(&runtime);
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE agg (id INTEGER PRIMARY KEY AUTOINCREMENT, v INTEGER NOT NULL);",
		)
		.unwrap();

		let batches = 20;
		let per_batch = 100;
		for batch in 0..batches {
			sqlite_exec(db.as_ptr(), "BEGIN").unwrap();
			for i in 0..per_batch {
				sqlite_exec(
					db.as_ptr(),
					&format!("INSERT INTO agg (v) VALUES ({});", batch * per_batch + i),
				)
				.unwrap();
			}
			sqlite_exec(db.as_ptr(), "COMMIT").unwrap();
			let expected_sum: i64 = (0..(batch + 1) * per_batch).map(|i| i as i64).sum();
			assert_eq!(
				sqlite_query_i64(db.as_ptr(), "SELECT SUM(v) FROM agg;").unwrap(),
				expected_sum
			);
		}
	}
}
