//! Custom SQLite VFS backed by KV operations over the KV channel.
//!
//! This crate now owns the KV-backed SQLite behavior used by `rivetkit-napi`.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::Result;
use libsqlite3_sys::*;
use moka::sync::Cache;
use parking_lot::{Mutex, RwLock};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use sqlite_storage::ltx::{LtxHeader, encode_ltx_v3};
#[cfg(test)]
use sqlite_storage::{engine::SqliteEngine, error::SqliteStorageError};
use tokio::runtime::Handle;
#[cfg(test)]
use tokio::sync::Notify;

use crate::optimization_flags::{SqliteOptimizationFlags, sqlite_optimization_flags};

const DEFAULT_PREFETCH_DEPTH: usize = 64;
const DEFAULT_MAX_PREFETCH_BYTES: usize = 256 * 1024;
const DEFAULT_ADAPTIVE_PREFETCH_DEPTH: usize = 256;
const DEFAULT_ADAPTIVE_MAX_PREFETCH_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_PAGES_PER_STAGE: usize = 4_000;
const DEFAULT_RECENT_HINT_PAGE_BUDGET: usize = 128;
const DEFAULT_RECENT_HINT_RANGE_BUDGET: usize = 16;
const DEFAULT_PAGE_SIZE: usize = 4096;
const MIN_RECENT_SCAN_RANGE_PAGES: u32 = 8;
const SCAN_SCORE_THRESHOLD: i32 = 6;
const SCAN_SCORE_MAX: i32 = 12;
const SCAN_GAP_TOLERANCE: u32 = 8;
const MAX_PATHNAME: c_int = 64;
const TEMP_AUX_PATH_PREFIX: &str = "__sqlite_temp__";
const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

#[cfg(test)]
static NEXT_STAGE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_AUX_ID: AtomicU64 = AtomicU64::new(1);

unsafe extern "C" {
	fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
}

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
				tracing::error!(message = panic_message(&panic), "sqlite callback panicked");
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
						Ok(result) => Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
							protocol::SqliteGetPagesOk {
								pages: result
									.pages
									.into_iter()
									.map(protocol_fetched_page)
									.collect(),
								meta: protocol_sqlite_meta(result.meta),
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
								.open(
									&req.actor_id,
									sqlite_storage::open::OpenConfig::new(1),
								)
								.await
							{
								Ok(_) => {}
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
									Ok(result) => {
										Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
											protocol::SqliteGetPagesOk {
												pages: result
													.pages
													.into_iter()
													.map(protocol_fetched_page)
													.collect(),
												meta: protocol_sqlite_meta(result.meta),
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

	async fn get_page_range(
		&self,
		req: protocol::SqliteGetPageRangeRequest,
	) -> Result<protocol::SqliteGetPageRangeResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_get_page_range(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, .. } => {
				match engine
					.get_page_range(
						&req.actor_id,
						req.generation,
						req.start_pgno,
						req.max_pages,
						req.max_bytes,
					)
					.await
				{
						Ok(result) => Ok(protocol::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(
							protocol::SqliteGetPageRangeOk {
								start_pgno: req.start_pgno,
								pages: result
									.pages
									.into_iter()
									.map(protocol_fetched_page)
									.collect(),
								meta: protocol_sqlite_meta(result.meta),
							},
						)),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(protocol::SqliteGetPageRangeResponse::SqliteFenceMismatch(
								protocol::SqliteFenceMismatch {
									actual_meta: protocol_sqlite_meta(
										engine.load_meta(&req.actor_id).await?,
									),
									reason: reason.clone(),
								},
							))
						} else {
							Ok(protocol::SqliteGetPageRangeResponse::SqliteErrorResponse(
								sqlite_error_response(&err),
							))
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.get_page_range(req).await,
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

	async fn commit_stage_begin(
		&self,
		req: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		match &*self.inner {
			SqliteTransportInner::Envoy(handle) => handle.sqlite_commit_stage_begin(req).await,
			#[cfg(test)]
			SqliteTransportInner::Direct { engine, .. } => {
				match engine
					.commit_stage_begin(
						&req.actor_id,
						sqlite_storage::commit::CommitStageBeginRequest {
							generation: req.generation,
						},
					)
					.await
				{
					Ok(result) => Ok(
						protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
							protocol::SqliteCommitStageBeginOk { txid: result.txid },
						),
					),
					Err(err) => {
						if let Some(SqliteStorageError::FenceMismatch { reason }) =
							sqlite_storage_error(&err)
						{
							Ok(
								protocol::SqliteCommitStageBeginResponse::SqliteFenceMismatch(
									protocol::SqliteFenceMismatch {
										actual_meta: protocol_sqlite_meta(
											engine.load_meta(&req.actor_id).await?,
										),
										reason: reason.clone(),
									},
								),
							)
						} else {
							Ok(
								protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(
									sqlite_error_response(&err),
								),
							)
						}
					}
				}
			}
			#[cfg(test)]
			SqliteTransportInner::Test(protocol) => protocol.commit_stage_begin(req).await,
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
							txid: req.txid,
							chunk_idx: req.chunk_idx,
							bytes: req.bytes,
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
							txid: req.txid,
							new_db_size_pages: req.new_db_size_pages,
							now_ms: sqlite_now_ms()?,
							origin_override: None,
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
	get_page_range_response: protocol::SqliteGetPageRangeResponse,
	mirror_commit_meta: AtomicBool,
	commit_requests: Mutex<Vec<protocol::SqliteCommitRequest>>,
	stage_requests: Mutex<Vec<protocol::SqliteCommitStageRequest>>,
	awaited_stage_responses: AtomicUsize,
	stage_response_awaited: Notify,
	finalize_requests: Mutex<Vec<protocol::SqliteCommitFinalizeRequest>>,
	get_pages_requests: Mutex<Vec<protocol::SqliteGetPagesRequest>>,
	get_page_range_requests: Mutex<Vec<protocol::SqliteGetPageRangeRequest>>,
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
			get_page_range_response: protocol::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(
				protocol::SqliteGetPageRangeOk {
					start_pgno: 1,
					pages: vec![],
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
			mirror_commit_meta: AtomicBool::new(false),
			commit_requests: Mutex::new(Vec::new()),
			stage_requests: Mutex::new(Vec::new()),
			awaited_stage_responses: AtomicUsize::new(0),
			stage_response_awaited: Notify::new(),
			finalize_requests: Mutex::new(Vec::new()),
			get_pages_requests: Mutex::new(Vec::new()),
			get_page_range_requests: Mutex::new(Vec::new()),
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
		self.awaited_stage_responses.load(Ordering::SeqCst)
	}

	async fn wait_for_stage_responses(&self, expected: usize) {
		use std::time::Duration;

		tokio::time::timeout(Duration::from_secs(1), async {
			while self.awaited_stage_responses() < expected {
				self.stage_response_awaited.notified().await;
			}
		})
		.await
		.expect("stage response await count should reach expected value");
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

	fn get_page_range_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteGetPageRangeRequest>> {
		self.get_page_range_requests.lock()
	}

	fn set_mirror_commit_meta(&self, enabled: bool) {
		self.mirror_commit_meta.store(enabled, Ordering::SeqCst);
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

	async fn get_page_range(
		&self,
		req: protocol::SqliteGetPageRangeRequest,
	) -> Result<protocol::SqliteGetPageRangeResponse> {
		self.get_page_range_requests().push(req);
		Ok(self.get_page_range_response.clone())
	}

	async fn commit(
		&self,
		req: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		let req = req.clone();
		self.commit_requests().push(req.clone());
		if self.mirror_commit_meta.load(Ordering::SeqCst) {
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

	async fn commit_stage_begin(
		&self,
		_req: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		Ok(
			protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				protocol::SqliteCommitStageBeginOk {
					txid: next_stage_id(),
				},
			),
		)
	}

	async fn commit_stage(
		&self,
		req: protocol::SqliteCommitStageRequest,
	) -> Result<protocol::SqliteCommitStageResponse> {
		self.awaited_stage_responses.fetch_add(1, Ordering::SeqCst);
		self.stage_response_awaited.notify_one();
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
		if self.mirror_commit_meta.load(Ordering::SeqCst) {
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
pub struct VfsConfig {
	pub cache_capacity_pages: u64,
	pub cache_fetched_pages: bool,
	pub cache_prefetched_pages: bool,
	pub cache_startup_preloaded_pages: bool,
	pub scan_resistant_cache: bool,
	pub protected_cache_pages: usize,
	pub prefetch_depth: usize,
	pub adaptive_prefetch_depth: usize,
	pub max_prefetch_bytes: usize,
	pub adaptive_max_prefetch_bytes: usize,
	pub max_pages_per_stage: usize,
	pub recent_hint_page_budget: usize,
	pub recent_hint_range_budget: usize,
	pub cache_hit_predictor_training: bool,
	pub recent_page_hints: bool,
	pub adaptive_read_ahead: bool,
	pub range_reads: bool,
}

impl Default for VfsConfig {
	fn default() -> Self {
		Self::from_optimization_flags(*sqlite_optimization_flags())
	}
}

impl VfsConfig {
	pub fn from_optimization_flags(flags: SqliteOptimizationFlags) -> Self {
		Self {
			cache_capacity_pages: flags.vfs_page_cache_capacity_pages,
			cache_fetched_pages: flags.vfs_cache_fetched_pages,
			cache_prefetched_pages: flags.vfs_cache_prefetched_pages,
			cache_startup_preloaded_pages: flags.vfs_cache_startup_preloaded_pages,
			scan_resistant_cache: flags.vfs_scan_resistant_cache,
			protected_cache_pages: flags.vfs_protected_cache_pages,
			prefetch_depth: if flags.read_ahead {
				DEFAULT_PREFETCH_DEPTH
			} else {
				0
			},
			adaptive_prefetch_depth: DEFAULT_ADAPTIVE_PREFETCH_DEPTH,
			max_prefetch_bytes: DEFAULT_MAX_PREFETCH_BYTES,
			adaptive_max_prefetch_bytes: DEFAULT_ADAPTIVE_MAX_PREFETCH_BYTES,
			max_pages_per_stage: DEFAULT_MAX_PAGES_PER_STAGE,
			recent_hint_page_budget: if flags.recent_page_hints {
				DEFAULT_RECENT_HINT_PAGE_BUDGET
			} else {
				0
			},
			recent_hint_range_budget: if flags.recent_page_hints {
				DEFAULT_RECENT_HINT_RANGE_BUDGET
			} else {
				0
			},
			cache_hit_predictor_training: flags.cache_hit_predictor_training,
			recent_page_hints: flags.recent_page_hints,
			adaptive_read_ahead: flags.adaptive_read_ahead,
			range_reads: flags.range_reads,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsPreloadHintRange {
	pub start_pgno: u32,
	pub page_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VfsPreloadHintSnapshot {
	pub pgnos: Vec<u32>,
	pub ranges: Vec<VfsPreloadHintRange>,
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

pub trait SqliteVfsMetrics: Send + Sync {
	fn record_resolve_pages(&self, _requested_pages: u64) {}

	fn record_resolve_cache_hits(&self, _pages: u64) {}

	fn record_resolve_cache_misses(&self, _pages: u64) {}

	fn record_get_pages_request(&self, _pages: u64, _prefetch_pages: u64, _page_size: u64) {}

	fn observe_get_pages_duration(&self, _duration_ns: u64) {}

	fn record_commit(&self) {}

	fn observe_commit_phases(
		&self,
		_request_build_ns: u64,
		_serialize_ns: u64,
		_transport_ns: u64,
		_state_update_ns: u64,
		_total_ns: u64,
	) {
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct CommitTransportMetrics {
	serialize_ns: u64,
	transport_ns: u64,
}

pub struct VfsContext {
	actor_id: String,
	runtime: Handle,
	transport: SqliteTransport,
	config: VfsConfig,
	state: RwLock<VfsState>,
	aux_files: RwLock<BTreeMap<String, Arc<AuxFileState>>>,
	aux_file_roles: RwLock<BTreeMap<String, VfsFileRole>>,
	last_error: Mutex<Option<String>>,
	#[cfg(test)]
	fail_next_aux_open: Mutex<Option<String>>,
	#[cfg(test)]
	fail_next_aux_delete: Mutex<Option<String>>,
	commit_atomic_count: AtomicU64,
	io_methods: Box<sqlite3_io_methods>,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
}

#[derive(Debug, Clone)]
struct VfsState {
	generation: u64,
	head_txid: u64,
	db_size_pages: u32,
	page_size: usize,
	max_delta_bytes: u64,
	page_cache: Cache<u32, Vec<u8>>,
	protected_page_cache: ProtectedPageCache,
	write_buffer: WriteBuffer,
	predictor: PrefetchPredictor,
	read_ahead: AdaptiveReadAhead,
	recent_pages: RecentPageTracker,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadAheadMode {
	Bounded,
	ForwardScan,
	BackwardScan,
}

#[derive(Debug, Clone, Copy)]
struct ReadAheadPlan {
	mode: ReadAheadMode,
	depth: usize,
	max_bytes: usize,
	seed_pgno: Option<u32>,
}

#[derive(Debug, Clone, Default)]
struct AdaptiveReadAhead {
	last_pgno: Option<u32>,
	scan_tip_pgno: Option<u32>,
	scan_direction: Option<ScanDirection>,
	score: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanDirection {
	Forward,
	Backward,
}

#[derive(Debug, Clone)]
struct RecentPageTracker {
	page_budget: usize,
	range_budget: usize,
	hot_pages: HashMap<u32, RecentPageAccess>,
	ranges: VecDeque<VfsPreloadHintRange>,
	active_scan_start: Option<u32>,
	active_scan_end: u32,
	last_pgno: Option<u32>,
	access_seq: u64,
}

#[derive(Debug, Clone, Copy)]
struct RecentPageAccess {
	count: u32,
	last_access_seq: u64,
}

#[derive(Debug, Clone)]
struct ProtectedPageCache {
	page_budget: usize,
	early_page_budget: usize,
	access_budget: usize,
	pages: HashMap<u32, Vec<u8>>,
	order: VecDeque<u32>,
	access_counts: HashMap<u32, u32>,
	access_order: VecDeque<u32>,
	early_pages_seen: usize,
}

#[derive(Debug)]
enum GetPagesError {
	FenceMismatch(String),
	Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageFetchTransport {
	PageList,
	Range {
		start_pgno: u32,
		max_pages: u32,
		max_bytes: u64,
	},
}

#[repr(C)]
struct VfsFile {
	base: sqlite3_file,
	ctx: *const VfsContext,
	aux: *mut AuxFileHandle,
	role: VfsFileRole,
}

#[derive(Default)]
struct AuxFileState {
	bytes: Mutex<Vec<u8>>,
}

struct AuxFileHandle {
	path: String,
	state: Arc<AuxFileState>,
	delete_on_close: bool,
	role: VfsFileRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VfsFileRole {
	Reader,
	Writer,
}

impl VfsFileRole {
	fn from_open_flags(flags: c_int) -> Self {
		if (flags & SQLITE_OPEN_READWRITE) != 0 {
			Self::Writer
		} else {
			Self::Reader
		}
	}

	fn out_flags(self, flags: c_int) -> c_int {
		match self {
			Self::Reader => (flags | SQLITE_OPEN_READONLY) & !SQLITE_OPEN_READWRITE,
			Self::Writer => (flags | SQLITE_OPEN_READWRITE) & !SQLITE_OPEN_READONLY,
		}
	}

	fn is_reader(self) -> bool {
		matches!(self, Self::Reader)
	}
}

unsafe impl Send for VfsContext {}
unsafe impl Sync for VfsContext {}

struct SqliteVfsInner {
	vfs_ptr: *mut sqlite3_vfs,
	_name: CString,
	ctx_ptr: *mut VfsContext,
}

unsafe impl Send for SqliteVfsInner {}
unsafe impl Sync for SqliteVfsInner {}

#[derive(Clone)]
pub struct NativeVfsHandle {
	inner: Arc<SqliteVfsInner>,
}

pub type SqliteVfs = NativeVfsHandle;

pub struct NativeDatabase {
	connection: NativeConnection,
	vfs: NativeVfsHandle,
}

pub struct NativeConnection {
	db: *mut sqlite3,
	_vfs: NativeVfsHandle,
}

unsafe impl Send for NativeDatabase {}
unsafe impl Send for NativeConnection {}

fn select_page_fetch_transport(
	to_fetch: &[u32],
	prefetch: bool,
	read_ahead_plan: ReadAheadPlan,
	config: &VfsConfig,
) -> PageFetchTransport {
	if !config.range_reads
		|| !prefetch
		|| !matches!(
			read_ahead_plan.mode,
			ReadAheadMode::ForwardScan | ReadAheadMode::BackwardScan
		)
		|| to_fetch.len() <= config.prefetch_depth
	{
		return PageFetchTransport::PageList;
	}

	let Some(start_pgno) = contiguous_page_run_start(to_fetch) else {
		return PageFetchTransport::PageList;
	};

	PageFetchTransport::Range {
		start_pgno,
		max_pages: to_fetch.len().try_into().unwrap_or(u32::MAX),
		max_bytes: read_ahead_plan.max_bytes.try_into().unwrap_or(u64::MAX),
	}
}

fn contiguous_page_run_start(pgnos: &[u32]) -> Option<u32> {
	let first = *pgnos.first()?;
	if pgnos.len() == 1 {
		return Some(first);
	}

	if pgnos
		.windows(2)
		.all(|window| window[1] == window[0].saturating_add(1))
	{
		return Some(first);
	}

	if pgnos
		.windows(2)
		.all(|window| window[0] == window[1].saturating_add(1))
	{
		return pgnos.last().copied();
	}

	None
}

fn sqlite_get_page_range_response_to_get_pages_response(
	response: protocol::SqliteGetPageRangeResponse,
) -> protocol::SqliteGetPagesResponse {
	match response {
		protocol::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(ok) => {
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(protocol::SqliteGetPagesOk {
				pages: ok.pages,
				meta: ok.meta,
			})
		}
		protocol::SqliteGetPageRangeResponse::SqliteFenceMismatch(mismatch) => {
			protocol::SqliteGetPagesResponse::SqliteFenceMismatch(mismatch)
		}
		protocol::SqliteGetPageRangeResponse::SqliteErrorResponse(error) => {
			protocol::SqliteGetPagesResponse::SqliteErrorResponse(error)
		}
	}
}

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
			if self.stride_run_len >= 2 && delta != 0 {
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

impl AdaptiveReadAhead {
	fn record_and_plan(&mut self, pgnos: &[u32], config: &VfsConfig) -> ReadAheadPlan {
		let mut scan_seed_pgno = None;
		let mut scan_direction = None;
		for pgno in pgnos.iter().copied() {
			if let Some(direction) = self.record(pgno) {
				scan_seed_pgno = Some(pgno);
				scan_direction = Some(direction);
			}
		}

		if config.adaptive_read_ahead
			&& self.score >= SCAN_SCORE_THRESHOLD
			&& scan_seed_pgno.is_some()
			&& scan_direction.is_some()
		{
			let depth = if self.score >= SCAN_SCORE_THRESHOLD + 4 {
				config.adaptive_prefetch_depth
			} else {
				config
					.adaptive_prefetch_depth
					.min(config.prefetch_depth.saturating_mul(2))
			};
			ReadAheadPlan {
				mode: match scan_direction.expect("scan direction checked above") {
					ScanDirection::Forward => ReadAheadMode::ForwardScan,
					ScanDirection::Backward => ReadAheadMode::BackwardScan,
				},
				depth,
				max_bytes: config.adaptive_max_prefetch_bytes,
				seed_pgno: scan_seed_pgno,
			}
		} else {
			ReadAheadPlan {
				mode: ReadAheadMode::Bounded,
				depth: config.prefetch_depth,
				max_bytes: config.max_prefetch_bytes,
				seed_pgno: pgnos.last().copied(),
			}
		}
	}

	fn record(&mut self, pgno: u32) -> Option<ScanDirection> {
		let direction_from_last = self
			.last_pgno
			.and_then(|last_pgno| scan_direction_between(last_pgno, pgno));
		let direction_from_scan_tip = self
			.scan_tip_pgno
			.and_then(|tip_pgno| scan_direction_between(tip_pgno, pgno));
		let repeated = self.last_pgno == Some(pgno);
		let observed_direction = direction_from_last.or(direction_from_scan_tip);

		if let Some(direction) = observed_direction {
			if self.scan_direction == Some(direction)
				|| self.scan_direction.is_none()
				|| self.score < SCAN_SCORE_THRESHOLD
			{
				self.score = (self.score + 2).min(SCAN_SCORE_MAX);
				self.scan_tip_pgno = Some(pgno);
				self.scan_direction = Some(direction);
				self.last_pgno = Some(pgno);
				return Some(direction);
			}

			self.score = (self.score - 4).max(0);
			self.scan_tip_pgno = Some(pgno);
			self.scan_direction = Some(direction);
		} else if !repeated {
			if self.score >= SCAN_SCORE_THRESHOLD && self.scan_tip_pgno.is_some() {
				self.score = (self.score - 1).max(0);
			} else {
				self.score = (self.score - 4).max(0);
				self.scan_tip_pgno = Some(pgno);
				self.scan_direction = None;
			}
		}

		self.last_pgno = Some(pgno);
		None
	}
}

fn scan_direction_between(previous: u32, current: u32) -> Option<ScanDirection> {
	if let Some(delta) = current.checked_sub(previous) {
		if (1..=SCAN_GAP_TOLERANCE).contains(&delta) {
			return Some(ScanDirection::Forward);
		}
	}
	if previous.checked_sub(current) == Some(1) {
		return Some(ScanDirection::Backward);
	}
	None
}

impl VfsPreloadHintRange {
	fn new(start_pgno: u32, end_pgno: u32) -> Self {
		Self {
			start_pgno,
			page_count: end_pgno.saturating_sub(start_pgno).saturating_add(1),
		}
	}

	fn end_pgno(&self) -> u32 {
		self.start_pgno
			.saturating_add(self.page_count)
			.saturating_sub(1)
	}

	fn contains(&self, pgno: u32) -> bool {
		(self.start_pgno..=self.end_pgno()).contains(&pgno)
	}
}

impl RecentPageTracker {
	fn new(page_budget: usize, range_budget: usize) -> Self {
		Self {
			page_budget,
			range_budget,
			hot_pages: HashMap::new(),
			ranges: VecDeque::new(),
			active_scan_start: None,
			active_scan_end: 0,
			last_pgno: None,
			access_seq: 0,
		}
	}

	fn record_pages(&mut self, pgnos: impl IntoIterator<Item = u32>) {
		for pgno in pgnos {
			self.record_page(pgno);
		}
	}

	fn record_page(&mut self, pgno: u32) {
		self.access_seq = self.access_seq.saturating_add(1);
		self.record_hot_page(pgno);
		self.record_scan_page(pgno);
	}

	fn record_hot_page(&mut self, pgno: u32) {
		if self.page_budget == 0 {
			return;
		}

		if let Some(access) = self.hot_pages.get_mut(&pgno) {
			access.count = access.count.saturating_add(1);
			access.last_access_seq = self.access_seq;
			return;
		}

		if self.hot_pages.len() >= self.page_budget {
			if let Some(evict_pgno) = self
				.hot_pages
				.iter()
				.min_by_key(|(_, access)| (access.count, access.last_access_seq))
				.map(|(pgno, _)| *pgno)
			{
				self.hot_pages.remove(&evict_pgno);
			}
		}

		self.hot_pages.insert(
			pgno,
			RecentPageAccess {
				count: 1,
				last_access_seq: self.access_seq,
			},
		);
	}

	fn record_scan_page(&mut self, pgno: u32) {
		match self.last_pgno {
			Some(last_pgno) if pgno == last_pgno.saturating_add(1) => {
				if self.active_scan_start.is_none() {
					self.active_scan_start = Some(last_pgno);
				}
				self.active_scan_end = pgno;
			}
			Some(last_pgno) if pgno == last_pgno => {}
			Some(_) | None => {
				self.finish_active_scan();
				self.active_scan_start = None;
				self.active_scan_end = 0;
			}
		}
		self.last_pgno = Some(pgno);
	}

	fn finish_active_scan(&mut self) {
		let Some(start_pgno) = self.active_scan_start else {
			return;
		};
		if self.active_scan_end < start_pgno {
			return;
		}
		let page_count = self.active_scan_end - start_pgno + 1;
		if page_count < MIN_RECENT_SCAN_RANGE_PAGES {
			return;
		}
		self.push_range(VfsPreloadHintRange::new(start_pgno, self.active_scan_end));
	}

	fn push_range(&mut self, range: VfsPreloadHintRange) {
		if self.range_budget == 0 || range.page_count == 0 {
			return;
		}
		push_coalesced_range(&mut self.ranges, range);
		while self.ranges.len() > self.range_budget {
			self.ranges.pop_front();
		}
	}

	fn snapshot(&self) -> VfsPreloadHintSnapshot {
		let mut ranges = self.ranges.clone();
		if let Some(start_pgno) = self.active_scan_start {
			if self.active_scan_end >= start_pgno {
				let page_count = self.active_scan_end - start_pgno + 1;
				if page_count >= MIN_RECENT_SCAN_RANGE_PAGES {
					push_coalesced_range(
						&mut ranges,
						VfsPreloadHintRange::new(start_pgno, self.active_scan_end),
					);
				}
			}
		}
		while ranges.len() > self.range_budget {
			ranges.pop_front();
		}

		let mut scored_pages = self
			.hot_pages
			.iter()
			.filter(|(pgno, _)| !ranges.iter().any(|range| range.contains(**pgno)))
			.map(|(pgno, access)| (*pgno, *access))
			.collect::<Vec<_>>();
		scored_pages.sort_by(|(left_pgno, left), (right_pgno, right)| {
			right
				.count
				.cmp(&left.count)
				.then_with(|| right.last_access_seq.cmp(&left.last_access_seq))
				.then_with(|| left_pgno.cmp(right_pgno))
		});

		let mut pgnos = scored_pages
			.into_iter()
			.take(self.page_budget)
			.map(|(pgno, _)| pgno)
			.collect::<Vec<_>>();
		pgnos.sort_unstable();

		VfsPreloadHintSnapshot {
			pgnos,
			ranges: ranges.into_iter().collect(),
		}
	}
}

impl ProtectedPageCache {
	fn new(page_budget: usize) -> Self {
		let early_page_budget = page_budget.min(64);
		let access_budget = page_budget.saturating_mul(4).max(page_budget).min(4096);
		Self {
			page_budget,
			early_page_budget,
			access_budget,
			pages: HashMap::new(),
			order: VecDeque::new(),
			access_counts: HashMap::new(),
			access_order: VecDeque::new(),
			early_pages_seen: 0,
		}
	}

	fn get(&self, pgno: &u32) -> Option<Vec<u8>> {
		self.pages.get(pgno).cloned()
	}

	fn clear(&mut self) {
		self.pages.clear();
		self.order.clear();
		self.access_counts.clear();
		self.access_order.clear();
		self.early_pages_seen = 0;
	}

	fn protect_startup_page(&mut self, pgno: u32, bytes: Vec<u8>) {
		self.protect_page(pgno, bytes);
	}

	fn update_if_protected(&mut self, pgno: u32, bytes: Vec<u8>) {
		if let Some(existing) = self.pages.get_mut(&pgno) {
			*existing = bytes;
		}
	}

	fn record_target_access(&mut self, pgno: u32, bytes: Vec<u8>) {
		if self.page_budget == 0 {
			return;
		}

		let is_new_access = !self.access_counts.contains_key(&pgno);
		if is_new_access {
			self.trim_access_counts_for_new_page();
			self.access_order.push_back(pgno);
		}
		let count = self.access_counts.entry(pgno).or_insert(0);
		*count = count.saturating_add(1);

		if self.pages.contains_key(&pgno) {
			self.protect_page(pgno, bytes);
			return;
		}

		if self.early_pages_seen < self.early_page_budget {
			self.early_pages_seen += 1;
			self.protect_page(pgno, bytes);
			return;
		}

		if *count >= 2 {
			self.protect_page(pgno, bytes);
		}
	}

	fn protect_page(&mut self, pgno: u32, bytes: Vec<u8>) {
		if self.page_budget == 0 {
			return;
		}

		if self.pages.insert(pgno, bytes).is_none() {
			self.order.push_back(pgno);
		}
		self.evict_over_budget();
	}

	fn trim_access_counts_for_new_page(&mut self) {
		while self.access_counts.len() >= self.access_budget && self.access_budget > 0 {
			let Some(candidate) = self.access_order.pop_front() else {
				self.access_counts.clear();
				return;
			};
			if self.pages.contains_key(&candidate) {
				continue;
			}
			self.access_counts.remove(&candidate);
			return;
		}
	}

	fn evict_over_budget(&mut self) {
		while self.pages.len() > self.page_budget {
			let Some(candidate) = self.order.pop_front() else {
				return;
			};
			if self.pages.remove(&candidate).is_some() {
				self.access_counts.remove(&candidate);
			}
		}
	}
}

fn push_coalesced_range(ranges: &mut VecDeque<VfsPreloadHintRange>, range: VfsPreloadHintRange) {
	let mut start_pgno = range.start_pgno;
	let mut end_pgno = range.end_pgno();
	let mut retained = VecDeque::new();
	while let Some(existing) = ranges.pop_front() {
		let existing_end = existing.end_pgno();
		if existing.start_pgno <= end_pgno.saturating_add(1)
			&& start_pgno <= existing_end.saturating_add(1)
		{
			start_pgno = start_pgno.min(existing.start_pgno);
			end_pgno = end_pgno.max(existing_end);
		} else {
			retained.push_back(existing);
		}
	}
	retained.push_back(VfsPreloadHintRange::new(start_pgno, end_pgno));
	*ranges = retained;
}

impl VfsState {
	fn new(config: &VfsConfig, startup: &protocol::SqliteStartupData) -> Self {
		let page_cache = Cache::builder()
			.max_capacity(config.cache_capacity_pages)
			.build();
		let mut protected_page_cache = ProtectedPageCache::new(if config.scan_resistant_cache {
			config.protected_cache_pages
		} else {
			0
		});
		if config.cache_startup_preloaded_pages {
			for page in &startup.preloaded_pages {
				if let Some(bytes) = &page.bytes {
					page_cache.insert(page.pgno, bytes.clone());
					protected_page_cache.protect_startup_page(page.pgno, bytes.clone());
				}
			}
		}

		let mut state = Self {
			generation: startup.generation,
			head_txid: startup.meta.head_txid,
			db_size_pages: startup.meta.db_size_pages,
			page_size: startup.meta.page_size as usize,
			max_delta_bytes: startup.meta.max_delta_bytes,
			page_cache,
			protected_page_cache,
			write_buffer: WriteBuffer::default(),
			predictor: PrefetchPredictor::default(),
			read_ahead: AdaptiveReadAhead::default(),
			recent_pages: RecentPageTracker::new(
				config.recent_hint_page_budget,
				config.recent_hint_range_budget,
			),
			dead: false,
		};
		if state.db_size_pages == 0 && !state.page_cache.contains_key(&1) {
			state.page_cache.insert(1, empty_db_page());
			state.db_size_pages = 1;
		}
		state
	}

	fn cache_fetched_page(
		&mut self,
		pgno: u32,
		bytes: Vec<u8>,
		target_page: bool,
		config: &VfsConfig,
	) {
		let should_cache = if target_page {
			config.cache_fetched_pages
		} else {
			config.cache_prefetched_pages
		};
		if should_cache {
			self.page_cache.insert(pgno, bytes.clone());
		}
		if target_page && config.cache_fetched_pages && config.scan_resistant_cache {
			self.protected_page_cache.record_target_access(pgno, bytes);
		}
	}

	fn record_target_cache_access(&mut self, pgno: u32, bytes: Vec<u8>, config: &VfsConfig) {
		if config.cache_fetched_pages && config.scan_resistant_cache {
			self.protected_page_cache.record_target_access(pgno, bytes);
		}
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

impl VfsContext {
	fn new(
		actor_id: String,
		runtime: Handle,
		transport: SqliteTransport,
		startup: protocol::SqliteStartupData,
		config: VfsConfig,
		io_methods: sqlite3_io_methods,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Self {
		Self {
			actor_id,
			runtime,
			transport,
			config: config.clone(),
			state: RwLock::new(VfsState::new(&config, &startup)),
			aux_files: RwLock::new(BTreeMap::new()),
			aux_file_roles: RwLock::new(BTreeMap::new()),
			last_error: Mutex::new(None),
			#[cfg(test)]
			fail_next_aux_open: Mutex::new(None),
			#[cfg(test)]
			fail_next_aux_delete: Mutex::new(None),
			commit_atomic_count: AtomicU64::new(0),
			io_methods: Box::new(io_methods),
			metrics,
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
		if let Some(metrics) = &self.metrics {
			metrics.observe_commit_phases(
				request_build_ns,
				transport_metrics.serialize_ns,
				transport_metrics.transport_ns,
				state_update_ns,
				total_ns,
			);
		}
	}

	fn page_size(&self) -> usize {
		self.state.read().page_size.max(DEFAULT_PAGE_SIZE)
	}

	fn open_aux_file(&self, path: &str, role: VfsFileRole) -> Arc<AuxFileState> {
		let mut aux_files = self.aux_files.write();
		let state = aux_files
			.entry(path.to_string())
			.or_insert_with(|| Arc::new(AuxFileState::default()))
			.clone();
		self.aux_file_roles
			.write()
			.entry(path.to_string())
			.or_insert(role);
		state
	}

	fn aux_file_exists(&self, path: &str) -> bool {
		self.aux_files.read().contains_key(path)
	}

	fn aux_file_role(&self, path: &str) -> Option<VfsFileRole> {
		self.aux_file_roles.read().get(path).copied()
	}

	fn delete_aux_file(&self, path: &str) {
		self.aux_files.write().remove(path);
		self.aux_file_roles.write().remove(path);
	}

	#[cfg(test)]
	fn fail_next_aux_open(&self, message: impl Into<String>) {
		*self.fail_next_aux_open.lock() = Some(message.into());
	}

	#[cfg(test)]
	fn take_aux_open_error(&self) -> Option<String> {
		self.fail_next_aux_open.lock().take()
	}

	#[cfg(test)]
	fn fail_next_aux_delete(&self, message: impl Into<String>) {
		*self.fail_next_aux_delete.lock() = Some(message.into());
	}

	#[cfg(test)]
	fn take_aux_delete_error(&self) -> Option<String> {
		self.fail_next_aux_delete.lock().take()
	}

	fn is_dead(&self) -> bool {
		self.state.read().dead
	}

	fn mark_dead(&self, message: String) {
		self.set_last_error(message);
		self.state.write().dead = true;
	}

	fn snapshot_preload_hints(&self) -> VfsPreloadHintSnapshot {
		if !self.config.recent_page_hints {
			return VfsPreloadHintSnapshot::default();
		}
		self.state.read().recent_pages.snapshot()
	}

	fn resolve_pages(
		&self,
		target_pgnos: &[u32],
		prefetch: bool,
	) -> std::result::Result<HashMap<u32, Option<Vec<u8>>>, GetPagesError> {
		if let Some(metrics) = &self.metrics {
			metrics.record_resolve_pages(target_pgnos.len() as u64);
		}

		let mut resolved = HashMap::new();
		let mut missing = Vec::new();
		let mut seen = HashSet::new();

		{
			let state = self.state.read();
			if state.dead {
				return Err(GetPagesError::Other(
					"sqlite actor lost its fence".to_string(),
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
				if let Some(bytes) = state.protected_page_cache.get(&pgno) {
					resolved.insert(pgno, Some(bytes));
					continue;
				}
				missing.push(pgno);
			}
		}

		if missing.is_empty() {
			let mut state = self.state.write();
			if self.config.cache_hit_predictor_training {
				for pgno in target_pgnos.iter().copied() {
					state.predictor.record(pgno);
				}
			}
			state.read_ahead.record_and_plan(target_pgnos, &self.config);
			if self.config.recent_page_hints {
				state.recent_pages.record_pages(target_pgnos.iter().copied());
			}
			for pgno in target_pgnos.iter().copied() {
				if let Some(Some(bytes)) = resolved.get(&pgno) {
					state.record_target_cache_access(pgno, bytes.clone(), &self.config);
				}
			}
			if let Some(metrics) = &self.metrics {
				metrics.record_resolve_cache_hits(target_pgnos.len() as u64);
			}
			return Ok(resolved);
		}
		if let Some(metrics) = &self.metrics {
			metrics.record_resolve_cache_hits((seen.len() - missing.len()) as u64);
			metrics.record_resolve_cache_misses(missing.len() as u64);
		}

		let (
			generation,
			to_fetch,
			page_size,
			read_ahead_mode,
			read_ahead_depth,
			read_ahead_max_bytes,
			seed_pgno,
			prediction_budget,
			predicted_pgnos,
			fetch_transport,
		) = {
			let mut state = self.state.write();
			if self.config.cache_hit_predictor_training {
				for pgno in target_pgnos.iter().copied() {
					state.predictor.record(pgno);
				}
			}
			let read_ahead_plan = state.read_ahead.record_and_plan(target_pgnos, &self.config);
			if self.config.recent_page_hints {
				state.recent_pages.record_pages(target_pgnos.iter().copied());
			}

			let mut to_fetch = missing.clone();
			let seed_pgno = read_ahead_plan.seed_pgno;
			let mut prediction_budget = 0;
			let mut predicted_pgnos = Vec::new();
			if prefetch {
				let page_budget = (read_ahead_plan.max_bytes / state.page_size.max(1)).max(1);
				prediction_budget = page_budget.saturating_sub(to_fetch.len());
				let seed = seed_pgno.unwrap_or_default();
				predicted_pgnos = state.predictor.multi_predict(
					seed,
					prediction_budget.min(read_ahead_plan.depth),
					state.db_size_pages.max(seed),
				);
				if read_ahead_plan.mode == ReadAheadMode::Bounded {
					predicted_pgnos.retain(|predicted| *predicted > seed);
				} else if read_ahead_plan.mode == ReadAheadMode::BackwardScan {
					let mut next_pgno = seed.checked_sub(1);
					predicted_pgnos.retain(|predicted| match next_pgno {
						Some(expected_pgno) if *predicted == expected_pgno => {
							next_pgno = predicted.checked_sub(1);
							true
						}
						_ => {
							next_pgno = None;
							false
						}
					});
				}
				for predicted in predicted_pgnos.iter().copied() {
					if resolved.contains_key(&predicted) || to_fetch.contains(&predicted) {
						continue;
					}
					to_fetch.push(predicted);
				}
			}
			let fetch_transport =
				select_page_fetch_transport(&to_fetch, prefetch, read_ahead_plan, &self.config);
			(
				state.generation,
				to_fetch,
				state.page_size.max(1),
				read_ahead_plan.mode,
				read_ahead_plan.depth,
				read_ahead_plan.max_bytes,
				seed_pgno,
				prediction_budget,
				predicted_pgnos,
				fetch_transport,
			)
		};

		{
			let prefetch_count = to_fetch.len() - missing.len();
			if let Some(metrics) = &self.metrics {
				metrics.record_get_pages_request(
					to_fetch.len() as u64,
					prefetch_count as u64,
					page_size as u64,
				);
			}
			tracing::debug!(
				requested_pages = ?target_pgnos,
				missing_pages = ?missing,
				read_ahead_mode = ?read_ahead_mode,
				read_ahead_depth,
				read_ahead_max_bytes,
				fetch_transport = ?fetch_transport,
				prediction_budget,
				predicted_pages = ?predicted_pgnos,
				prefetch_pages = prefetch_count,
				total_fetch_pages = to_fetch.len(),
				total_fetch_bytes = to_fetch.len().saturating_mul(page_size),
				seed_pgno,
				"vfs page fetch"
			);
		}

		let get_pages_start = Instant::now();
		let response = match fetch_transport {
			PageFetchTransport::PageList => self.runtime.block_on(self.transport.get_pages(
				protocol::SqliteGetPagesRequest {
					actor_id: self.actor_id.clone(),
					generation,
					pgnos: to_fetch.clone(),
				},
			)),
			PageFetchTransport::Range {
				start_pgno,
				max_pages,
				max_bytes,
			} => self
				.runtime
				.block_on(self.transport.get_page_range(protocol::SqliteGetPageRangeRequest {
					actor_id: self.actor_id.clone(),
					generation,
					start_pgno,
					max_pages,
					max_bytes,
				}))
				.map(sqlite_get_page_range_response_to_get_pages_response),
		};
		if let Some(metrics) = &self.metrics {
			metrics.observe_get_pages_duration(get_pages_start.elapsed().as_nanos() as u64);
		}
		let response = response.map_err(|err| GetPagesError::Other(err.to_string()))?;

		match response {
			protocol::SqliteGetPagesResponse::SqliteFenceMismatch(mismatch) => {
				Err(GetPagesError::FenceMismatch(mismatch.reason))
			}
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => {
				let mut state = self.state.write();
				state.update_read_meta(&ok.meta);
				let missing_set = missing.iter().copied().collect::<HashSet<_>>();
				for pgno in target_pgnos.iter().copied() {
					if let Some(Some(bytes)) = resolved.get(&pgno) {
						state.record_target_cache_access(pgno, bytes.clone(), &self.config);
					}
				}
				for fetched in ok.pages {
					if let Some(bytes) = &fetched.bytes {
						state.cache_fetched_page(
							fetched.pgno,
							bytes.clone(),
							missing_set.contains(&fetched.pgno),
							&self.config,
						);
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
					"sqlite actor lost its fence".to_string(),
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
				tracing::error!(
					actor_id = %self.actor_id,
					generation = request.generation,
					expected_head_txid = request.expected_head_txid,
					new_db_size_pages = request.new_db_size_pages,
					dirty_pages = request.dirty_pages.len(),
					?err,
					"sqlite flush commit failed"
				);
				mark_dead_for_non_fence_commit_error(self, &err);
				return Err(err);
			}
		};
			if let Some(metrics) = &self.metrics {
				metrics.record_commit();
			}
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
			state
				.protected_page_cache
				.update_if_protected(dirty_page.pgno, dirty_page.bytes.clone());
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
					"sqlite actor lost its fence".to_string(),
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
				tracing::error!(
					actor_id = %self.actor_id,
					generation = request.generation,
					expected_head_txid = request.expected_head_txid,
					new_db_size_pages = request.new_db_size_pages,
					dirty_pages = request.dirty_pages.len(),
					?err,
					"sqlite atomic commit failed"
				);
				mark_dead_for_non_fence_commit_error(self, &err);
				return Err(err);
			}
		};
			if let Some(metrics) = &self.metrics {
				metrics.record_commit();
			}
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
			state
				.protected_page_cache
				.update_if_protected(dirty_page.pgno, dirty_page.bytes.clone());
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
		state.protected_page_cache.clear();
	}
}

fn cleanup_batch_atomic_probe(db: *mut sqlite3) {
	if let Err(err) = sqlite_exec(db, "DROP TABLE IF EXISTS __rivet_batch_probe;") {
		tracing::warn!(%err, "failed to clean up sqlite batch atomic probe table");
	}
}

fn assert_batch_atomic_probe(db: *mut sqlite3, vfs: &SqliteVfs) -> std::result::Result<(), String> {
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
		let last_error = vfs.clone_last_error();
		tracing::error!(
			%err,
			last_error = ?last_error,
			commit_atomic_before,
			"sqlite batch atomic probe failed"
		);
		cleanup_batch_atomic_probe(db);
		if let Some(last_error) = last_error {
			return Err(format!(
				"batch atomic probe failed: {err}; vfs last_error: {last_error}"
			));
		}
		return Err(format!("batch atomic probe failed: {err}"));
	}

	let commit_atomic_after = vfs.commit_atomic_count();
	if commit_atomic_after == commit_atomic_before {
		tracing::error!(
			commit_atomic_before,
			commit_atomic_after,
			last_error = ?vfs.clone_last_error(),
			"batch atomic writes not active for sqlite, SQLITE_ENABLE_BATCH_ATOMIC_WRITE may be missing"
		);
		cleanup_batch_atomic_probe(db);
		return Err(
			"batch atomic writes not active for sqlite, SQLITE_ENABLE_BATCH_ATOMIC_WRITE may be missing"
				.to_string(),
		);
	}

	Ok(())
}

fn mark_dead_for_non_fence_commit_error(ctx: &VfsContext, err: &CommitBufferError) {
	match err {
		CommitBufferError::FenceMismatch(_) => {}
		CommitBufferError::StageNotFound(stage_id) => {
			ctx.mark_dead(format!(
				"sqlite stage {stage_id} missing during commit finalize"
			));
		}
		CommitBufferError::Other(message) => ctx.mark_dead(message.clone()),
	}
}

fn mark_dead_from_fence_commit_error(ctx: &VfsContext, err: &CommitBufferError) {
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

fn split_bytes(bytes: &[u8], max_chunk_bytes: usize) -> Vec<Vec<u8>> {
	if bytes.is_empty() || max_chunk_bytes == 0 {
		return vec![bytes.to_vec()];
	}

	bytes
		.chunks(max_chunk_bytes)
		.map(|chunk| chunk.to_vec())
		.collect()
}

#[cfg(test)]
fn next_stage_id() -> u64 {
	NEXT_STAGE_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_temp_aux_path() -> String {
	format!(
		"{TEMP_AUX_PATH_PREFIX}-{}",
		NEXT_TEMP_AUX_ID.fetch_add(1, Ordering::Relaxed)
	)
}

unsafe fn get_aux_state(file: &VfsFile) -> Option<&AuxFileHandle> {
	(!file.aux.is_null()).then(|| &*file.aux)
}

fn reject_reader_mutation(ctx: &VfsContext, operation: &str) {
	ctx.set_last_error(format!(
		"reader sqlite VFS handle attempted mutating operation {operation}"
	));
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

	let serialize_start = Instant::now();
	let stage_begin_request = protocol::SqliteCommitStageBeginRequest {
		actor_id: request.actor_id.clone(),
		generation: request.generation,
	};
	metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;
	let transport_start = Instant::now();
	let txid = match transport
		.commit_stage_begin(stage_begin_request)
		.await
		.map_err(|err| CommitBufferError::Other(err.to_string()))?
	{
		protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(ok) => {
			metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
			ok.txid
		}
		protocol::SqliteCommitStageBeginResponse::SqliteFenceMismatch(mismatch) => {
			return Err(CommitBufferError::FenceMismatch(mismatch.reason));
		}
		protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(error) => {
			return Err(CommitBufferError::Other(error.message));
		}
	};

	let serialize_start = Instant::now();
	let encoded_delta = encode_ltx_v3(
		LtxHeader::delta(
			txid,
			request.new_db_size_pages,
			sqlite_now_ms().map_err(|err| CommitBufferError::Other(err.to_string()))?,
		),
		&request
			.dirty_pages
			.iter()
			.map(|dirty_page| sqlite_storage::types::DirtyPage {
				pgno: dirty_page.pgno,
				bytes: dirty_page.bytes.clone(),
			})
			.collect::<Vec<_>>(),
	)
	.map_err(|err| CommitBufferError::Other(err.to_string()))?;
	let staged_chunks = split_bytes(
		&encoded_delta,
		request.max_delta_bytes.try_into().map_err(|_| {
			CommitBufferError::Other("sqlite max_delta_bytes exceeded usize".to_string())
		})?,
	);
	metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;

	for (chunk_idx, chunk_bytes) in staged_chunks.iter().enumerate() {
		let serialize_start = Instant::now();
		let stage_request = protocol::SqliteCommitStageRequest {
			actor_id: request.actor_id.clone(),
			generation: request.generation,
			txid,
			chunk_idx: chunk_idx as u32,
			bytes: chunk_bytes.clone(),
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
		txid,
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

unsafe fn get_file(p: *mut sqlite3_file) -> &'static mut VfsFile {
	&mut *(p as *mut VfsFile)
}

unsafe fn get_vfs_ctx(p: *mut sqlite3_vfs) -> &'static VfsContext {
	&*((*p).pAppData as *const VfsContext)
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

#[cfg(test)]
fn sqlite_prepare_statement(
	db: *mut sqlite3,
	sql: &str,
) -> std::result::Result<*mut sqlite3_stmt, String> {
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

	Ok(stmt)
}

#[cfg(test)]
fn sqlite_bind_text_bytes(
	db: *mut sqlite3,
	stmt: *mut sqlite3_stmt,
	index: c_int,
	bytes: &[u8],
	sql: &str,
) -> std::result::Result<(), String> {
	let rc = unsafe {
		sqlite3_bind_text(
			stmt,
			index,
			bytes.as_ptr().cast(),
			bytes.len() as c_int,
			None,
		)
	};
	if rc != SQLITE_OK {
		return Err(format!(
			"`{sql}` bind text index {index} failed with code {rc}: {}",
			sqlite_error_message(db)
		));
	}

	Ok(())
}

#[cfg(test)]
fn sqlite_bind_i64(
	db: *mut sqlite3,
	stmt: *mut sqlite3_stmt,
	index: c_int,
	value: i64,
	sql: &str,
) -> std::result::Result<(), String> {
	let rc = unsafe { sqlite3_bind_int64(stmt, index, value) };
	if rc != SQLITE_OK {
		return Err(format!(
			"`{sql}` bind int index {index} failed with code {rc}: {}",
			sqlite_error_message(db)
		));
	}

	Ok(())
}

#[cfg(test)]
fn sqlite_step_prepared(
	db: *mut sqlite3,
	stmt: *mut sqlite3_stmt,
	sql: &str,
) -> std::result::Result<(), String> {
	let step_rc = unsafe { sqlite3_step(stmt) };
	if step_rc != SQLITE_DONE {
		return Err(format!(
			"`{sql}` step failed with code {step_rc}: {}",
			sqlite_error_message(db)
		));
	}

	Ok(())
}

#[cfg(test)]
fn sqlite_reset_prepared(stmt: *mut sqlite3_stmt, sql: &str) -> std::result::Result<(), String> {
	let rc = unsafe { sqlite3_reset(stmt) };
	if rc != SQLITE_OK {
		return Err(format!("`{sql}` reset failed with code {rc}"));
	}

	Ok(())
}

#[cfg(test)]
fn sqlite_clear_bindings(stmt: *mut sqlite3_stmt, sql: &str) -> std::result::Result<(), String> {
	let rc = unsafe { sqlite3_clear_bindings(stmt) };
	if rc != SQLITE_OK {
		return Err(format!("`{sql}` clear bindings failed with code {rc}"));
	}

	Ok(())
}

#[cfg(test)]
fn sqlite_insert_text_with_int_value(
	db: *mut sqlite3,
	sql: &str,
	text_bytes: &[u8],
	int_value: i64,
) -> std::result::Result<(), String> {
	let stmt = sqlite_prepare_statement(db, sql)?;
	let result = (|| {
		sqlite_bind_text_bytes(db, stmt, 1, text_bytes, sql)?;
		sqlite_bind_i64(db, stmt, 2, int_value, sql)?;
		sqlite_step_prepared(db, stmt, sql)
	})();
	unsafe {
		sqlite3_finalize(stmt);
	}
	result
}

#[cfg(test)]
fn sqlite_query_i64_bind_text(
	db: *mut sqlite3,
	sql: &str,
	text_bytes: &[u8],
) -> std::result::Result<i64, String> {
	let stmt = sqlite_prepare_statement(db, sql)?;
	let result = (|| {
		sqlite_bind_text_bytes(db, stmt, 1, text_bytes, sql)?;
		match unsafe { sqlite3_step(stmt) } {
			SQLITE_ROW => Ok(unsafe { sqlite3_column_int64(stmt, 0) }),
			step_rc => Err(format!(
				"`{sql}` step failed with code {step_rc}: {}",
				sqlite_error_message(db)
			)),
		}
	})();
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

unsafe extern "C" fn io_close(p_file: *mut sqlite3_file) -> c_int {
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
				if file.role.is_reader() {
					reject_reader_mutation(ctx, "dirty xClose");
					return SQLITE_IOERR;
				}
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

unsafe extern "C" fn io_read(
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

unsafe extern "C" fn io_write(
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
		let ctx = &*file.ctx;
		let role = get_aux_state(file)
			.map(|aux| aux.role)
			.unwrap_or(file.role);
		if role.is_reader() {
			reject_reader_mutation(ctx, "xWrite");
			return SQLITE_IOERR_WRITE;
		}
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

unsafe extern "C" fn io_truncate(p_file: *mut sqlite3_file, size: sqlite3_int64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_TRUNCATE, {
		if size < 0 {
			return SQLITE_IOERR_TRUNCATE;
		}
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let role = get_aux_state(file)
			.map(|aux| aux.role)
			.unwrap_or(file.role);
		if role.is_reader() {
			reject_reader_mutation(ctx, "xTruncate");
			return SQLITE_IOERR_TRUNCATE;
		}
		if let Some(aux) = get_aux_state(file) {
			aux.state.bytes.lock().truncate(size as usize);
			return SQLITE_OK;
		}
		ctx.truncate_main_file(size);
		SQLITE_OK
	})
}

unsafe extern "C" fn io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		if get_aux_state(file).is_some() {
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		if file.role.is_reader() {
			let state = ctx.state.read();
			if state.write_buffer.in_atomic_write || !state.write_buffer.dirty.is_empty() {
				drop(state);
				reject_reader_mutation(ctx, "dirty xSync");
				return SQLITE_IOERR_FSYNC;
			}
			return SQLITE_OK;
		}
		match ctx.flush_dirty_pages() {
			Ok(_) => SQLITE_OK,
			Err(err) => {
				tracing::error!(
					actor_id = %ctx.actor_id,
					last_error = ?ctx.clone_last_error(),
					?err,
					"sqlite sync failed"
				);
				mark_dead_from_fence_commit_error(ctx, &err);
				SQLITE_IOERR_FSYNC
			}
		}
	})
}

unsafe extern "C" fn io_file_size(p_file: *mut sqlite3_file, p_size: *mut sqlite3_int64) -> c_int {
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

unsafe extern "C" fn io_lock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_LOCK, SQLITE_OK)
}

unsafe extern "C" fn io_unlock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_UNLOCK, SQLITE_OK)
}

unsafe extern "C" fn io_check_reserved_lock(
	_p_file: *mut sqlite3_file,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		*p_res_out = 0;
		SQLITE_OK
	})
}

unsafe extern "C" fn io_file_control(
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
				if file.role.is_reader() {
					reject_reader_mutation(ctx, "begin atomic write file-control");
					return SQLITE_READONLY;
				}
				let mut state = ctx.state.write();
				state.write_buffer.in_atomic_write = true;
				state.write_buffer.saved_db_size = state.db_size_pages;
				state.write_buffer.dirty.clear();
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
				if file.role.is_reader() {
					reject_reader_mutation(ctx, "commit atomic write file-control");
					return SQLITE_READONLY;
				}
				match ctx.commit_atomic_write() {
				Ok(()) => {
					ctx.commit_atomic_count.fetch_add(1, Ordering::Relaxed);
					SQLITE_OK
				}
				Err(err) => {
					tracing::error!(
						actor_id = %ctx.actor_id,
						last_error = ?ctx.clone_last_error(),
						?err,
						"sqlite atomic write file control failed"
					);
					mark_dead_from_fence_commit_error(ctx, &err);
					SQLITE_IOERR
				}
				}
			}
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				if file.role.is_reader() {
					reject_reader_mutation(ctx, "rollback atomic write file-control");
					return SQLITE_READONLY;
				}
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

unsafe extern "C" fn io_sector_size(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(DEFAULT_PAGE_SIZE as c_int, DEFAULT_PAGE_SIZE as c_int)
}

unsafe extern "C" fn io_device_characteristics(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(0, {
		let file = get_file(p_file);
		if file.role.is_reader() || get_aux_state(file).is_some() {
			0
		} else {
			SQLITE_IOCAP_BATCH_ATOMIC
		}
	})
}

unsafe extern "C" fn vfs_open(
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
		let role = VfsFileRole::from_open_flags(flags);

		if !is_main && role.is_reader() && !ctx.aux_file_exists(&path) {
			// Reader auxiliary files are not safe yet. A reader connection may only
			// open an existing auxiliary path without creating new mutable state.
			ctx.set_last_error(format!(
				"reader sqlite VFS handle attempted auxiliary file creation for {path}"
			));
			return SQLITE_CANTOPEN;
		}

		#[cfg(test)]
		if !is_main {
			if let Some(message) = ctx.take_aux_open_error() {
				ctx.set_last_error(message);
				return SQLITE_CANTOPEN;
			}
		}

		let base = sqlite3_file {
			pMethods: ctx.io_methods.as_ref(),
		};
		let aux = if is_main {
			ptr::null_mut()
		} else {
			Box::into_raw(Box::new(AuxFileHandle {
				path: path.clone(),
				state: ctx.open_aux_file(&path, role),
				delete_on_close,
				role,
			}))
		};
		ptr::write(
			p_file.cast::<VfsFile>(),
			VfsFile {
				base,
				ctx: ctx as *const VfsContext,
				aux,
				role,
			},
		);

		if !p_out_flags.is_null() {
			*p_out_flags = role.out_flags(flags);
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_delete(
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
			if matches!(ctx.aux_file_role(path), Some(VfsFileRole::Reader)) {
				reject_reader_mutation(ctx, "xDelete");
				return SQLITE_READONLY;
			}
			#[cfg(test)]
			if let Some(message) = ctx.take_aux_delete_error() {
				ctx.set_last_error(message);
				return SQLITE_IOERR_DELETE;
			}
			ctx.delete_aux_file(path);
		}
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_access(
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

unsafe extern "C" fn vfs_full_pathname(
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

unsafe extern "C" fn vfs_randomness(
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

unsafe extern "C" fn vfs_sleep(_p_vfs: *mut sqlite3_vfs, microseconds: c_int) -> c_int {
	vfs_catch_unwind!(0, {
		std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
		microseconds
	})
}

unsafe extern "C" fn vfs_current_time(_p_vfs: *mut sqlite3_vfs, p_time_out: *mut f64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default();
		*p_time_out = 2440587.5 + (now.as_secs_f64() / 86400.0);
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_get_last_error(
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

impl NativeVfsHandle {
	pub fn register(
		name: &str,
		handle: EnvoyHandle,
		actor_id: String,
		runtime: Handle,
		startup: protocol::SqliteStartupData,
		config: VfsConfig,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> std::result::Result<Self, String> {
		Self::register_with_transport(
			name,
			SqliteTransport::from_envoy(handle),
			actor_id,
			runtime,
			startup,
			config,
			metrics,
		)
	}

	fn take_last_error(&self) -> Option<String> {
		unsafe { (*self.inner.ctx_ptr).take_last_error() }
	}

	fn clone_last_error(&self) -> Option<String> {
		unsafe { (*self.inner.ctx_ptr).clone_last_error() }
	}

	fn snapshot_preload_hints(&self) -> VfsPreloadHintSnapshot {
		unsafe { (*self.inner.ctx_ptr).snapshot_preload_hints() }
	}

	fn register_with_transport(
		name: &str,
		transport: SqliteTransport,
		actor_id: String,
		runtime: Handle,
		startup: protocol::SqliteStartupData,
		config: VfsConfig,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> std::result::Result<Self, String> {
		let mut io_methods: sqlite3_io_methods = unsafe { std::mem::zeroed() };
		io_methods.iVersion = 1;
		io_methods.xClose = Some(io_close);
		io_methods.xRead = Some(io_read);
		io_methods.xWrite = Some(io_write);
		io_methods.xTruncate = Some(io_truncate);
		io_methods.xSync = Some(io_sync);
		io_methods.xFileSize = Some(io_file_size);
		io_methods.xLock = Some(io_lock);
		io_methods.xUnlock = Some(io_unlock);
		io_methods.xCheckReservedLock = Some(io_check_reserved_lock);
		io_methods.xFileControl = Some(io_file_control);
		io_methods.xSectorSize = Some(io_sector_size);
		io_methods.xDeviceCharacteristics = Some(io_device_characteristics);

		let ctx = Box::new(VfsContext::new(
			actor_id, runtime, transport, startup, config, io_methods, metrics,
		));
		let ctx_ptr = Box::into_raw(ctx);
		let name_cstring = CString::new(name).map_err(|err| err.to_string())?;

		let mut vfs: sqlite3_vfs = unsafe { std::mem::zeroed() };
		vfs.iVersion = 1;
		vfs.szOsFile = std::mem::size_of::<VfsFile>() as c_int;
		vfs.mxPathname = MAX_PATHNAME;
		vfs.zName = name_cstring.as_ptr();
		vfs.pAppData = ctx_ptr.cast::<c_void>();
		vfs.xOpen = Some(vfs_open);
		vfs.xDelete = Some(vfs_delete);
		vfs.xAccess = Some(vfs_access);
		vfs.xFullPathname = Some(vfs_full_pathname);
		vfs.xRandomness = Some(vfs_randomness);
		vfs.xSleep = Some(vfs_sleep);
		vfs.xCurrentTime = Some(vfs_current_time);
		vfs.xGetLastError = Some(vfs_get_last_error);

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
			inner: Arc::new(SqliteVfsInner {
			vfs_ptr,
			_name: name_cstring,
			ctx_ptr,
			}),
		})
	}

	pub fn name_ptr(&self) -> *const c_char {
		self.inner._name.as_ptr()
	}

	fn commit_atomic_count(&self) -> u64 {
		unsafe {
			(*self.inner.ctx_ptr)
				.commit_atomic_count
				.load(Ordering::Relaxed)
		}
	}
}

impl Drop for SqliteVfsInner {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs_ptr);
			drop(Box::from_raw(self.vfs_ptr));
			drop(Box::from_raw(self.ctx_ptr));
		}
	}
}

impl NativeDatabase {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.connection.as_ptr()
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self.vfs.take_last_error()
	}

	pub fn snapshot_preload_hints(&self) -> VfsPreloadHintSnapshot {
		self.vfs.snapshot_preload_hints()
	}

	pub fn vfs_handle(&self) -> NativeVfsHandle {
		self.vfs.clone()
	}
}

impl NativeConnection {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.db
	}
}

impl Drop for NativeConnection {
	fn drop(&mut self) {
		if !self.db.is_null() {
			let rc = unsafe { sqlite3_close_v2(self.db) };
			if rc != SQLITE_OK {
				tracing::warn!(
					rc,
					error = sqlite_error_message(self.db),
					"failed to close sqlite database"
				);
			}
			self.db = ptr::null_mut();
		}
	}
}

pub fn open_connection(
	vfs: NativeVfsHandle,
	file_name: &str,
	flags: c_int,
) -> std::result::Result<NativeConnection, String> {
	let c_name = CString::new(file_name).map_err(|err| err.to_string())?;
	let mut db: *mut sqlite3 = ptr::null_mut();

	let rc = unsafe {
		sqlite3_open_v2(
			c_name.as_ptr(),
			&mut db,
			flags,
			vfs.name_ptr(),
		)
	};
	if rc != SQLITE_OK {
		let message = sqlite_error_message(db);
		tracing::error!(
			file_name,
			rc,
			%message,
			last_error = ?vfs.clone_last_error(),
			"failed to open sqlite database with custom VFS"
		);
		if !db.is_null() {
			unsafe {
				sqlite3_close(db);
			}
		}
		return Err(format!("sqlite3_open_v2 failed with code {rc}: {message}"));
	}

	Ok(NativeConnection { db, _vfs: vfs })
}

pub fn open_database(
	vfs: NativeVfsHandle,
	file_name: &str,
) -> std::result::Result<NativeDatabase, String> {
	let connection = open_connection(
		vfs.clone(),
		file_name,
		SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
	)?;

	for pragma in &[
		"PRAGMA page_size = 4096;",
		"PRAGMA journal_mode = DELETE;",
		"PRAGMA synchronous = NORMAL;",
		"PRAGMA temp_store = MEMORY;",
		"PRAGMA auto_vacuum = NONE;",
		"PRAGMA locking_mode = EXCLUSIVE;",
	] {
		if let Err(err) = sqlite_exec(connection.as_ptr(), pragma) {
			tracing::error!(
				file_name,
				pragma,
				%err,
				last_error = ?vfs.clone_last_error(),
				"failed to configure sqlite database"
			);
			return Err(err);
		}
	}

	if let Err(err) = assert_batch_atomic_probe(connection.as_ptr(), &vfs) {
		tracing::error!(
			file_name,
			%err,
			last_error = ?vfs.clone_last_error(),
			"failed to verify sqlite batch atomic writes"
		);
		return Err(err);
	}

	Ok(NativeDatabase { connection, vfs })
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
	use std::sync::{Arc, Barrier};
	use std::thread;

	use parking_lot::Mutex as SyncMutex;
	use tempfile::TempDir;
	use tokio::runtime::Builder;
	use universaldb::Subspace;

	use super::*;

	static TEST_ID: AtomicU64 = AtomicU64::new(1);

	#[derive(Default)]
	struct TestVfsMetrics {
		resolve_pages_total: AtomicU64,
		resolve_pages_requested_total: AtomicU64,
		resolve_pages_cache_hits_total: AtomicU64,
		resolve_pages_cache_misses_total: AtomicU64,
		get_pages_total: AtomicU64,
		pages_fetched_total: AtomicU64,
		prefetch_pages_total: AtomicU64,
		bytes_fetched_total: AtomicU64,
		prefetch_bytes_total: AtomicU64,
		get_pages_duration_ns_total: AtomicU64,
		get_pages_duration_count: AtomicU64,
		commit_total: AtomicU64,
		commit_request_build_ns_total: AtomicU64,
		commit_serialize_ns_total: AtomicU64,
		commit_transport_ns_total: AtomicU64,
		commit_state_update_ns_total: AtomicU64,
		commit_duration_ns_total: AtomicU64,
	}

	#[allow(dead_code)]
	#[derive(Default)]
	struct TestVfsMetricsSnapshot {
		resolve_pages_total: u64,
		resolve_pages_requested_total: u64,
		resolve_pages_cache_hits_total: u64,
		resolve_pages_cache_misses_total: u64,
		get_pages_total: u64,
		pages_fetched_total: u64,
		prefetch_pages_total: u64,
		bytes_fetched_total: u64,
		prefetch_bytes_total: u64,
		get_pages_duration_ns_total: u64,
		get_pages_duration_count: u64,
		commit_total: u64,
		commit_request_build_ns_total: u64,
		commit_serialize_ns_total: u64,
		commit_transport_ns_total: u64,
		commit_state_update_ns_total: u64,
		commit_duration_ns_total: u64,
	}

	impl TestVfsMetrics {
		fn new() -> Arc<Self> {
			Arc::new(Self::default())
		}

		fn reset(&self) {
			let relaxed = AtomicOrdering::Relaxed;
			self.resolve_pages_total.store(0, relaxed);
			self.resolve_pages_requested_total.store(0, relaxed);
			self.resolve_pages_cache_hits_total.store(0, relaxed);
			self.resolve_pages_cache_misses_total.store(0, relaxed);
			self.get_pages_total.store(0, relaxed);
			self.pages_fetched_total.store(0, relaxed);
			self.prefetch_pages_total.store(0, relaxed);
			self.bytes_fetched_total.store(0, relaxed);
			self.prefetch_bytes_total.store(0, relaxed);
			self.get_pages_duration_ns_total.store(0, relaxed);
			self.get_pages_duration_count.store(0, relaxed);
			self.commit_total.store(0, relaxed);
			self.commit_request_build_ns_total.store(0, relaxed);
			self.commit_serialize_ns_total.store(0, relaxed);
			self.commit_transport_ns_total.store(0, relaxed);
			self.commit_state_update_ns_total.store(0, relaxed);
			self.commit_duration_ns_total.store(0, relaxed);
		}

		fn snapshot(&self) -> TestVfsMetricsSnapshot {
			let relaxed = AtomicOrdering::Relaxed;
			TestVfsMetricsSnapshot {
				resolve_pages_total: self.resolve_pages_total.load(relaxed),
				resolve_pages_requested_total: self.resolve_pages_requested_total.load(relaxed),
				resolve_pages_cache_hits_total: self.resolve_pages_cache_hits_total.load(relaxed),
				resolve_pages_cache_misses_total: self
					.resolve_pages_cache_misses_total
					.load(relaxed),
				get_pages_total: self.get_pages_total.load(relaxed),
				pages_fetched_total: self.pages_fetched_total.load(relaxed),
				prefetch_pages_total: self.prefetch_pages_total.load(relaxed),
				bytes_fetched_total: self.bytes_fetched_total.load(relaxed),
				prefetch_bytes_total: self.prefetch_bytes_total.load(relaxed),
				get_pages_duration_ns_total: self.get_pages_duration_ns_total.load(relaxed),
				get_pages_duration_count: self.get_pages_duration_count.load(relaxed),
				commit_total: self.commit_total.load(relaxed),
				commit_request_build_ns_total: self.commit_request_build_ns_total.load(relaxed),
				commit_serialize_ns_total: self.commit_serialize_ns_total.load(relaxed),
				commit_transport_ns_total: self.commit_transport_ns_total.load(relaxed),
				commit_state_update_ns_total: self.commit_state_update_ns_total.load(relaxed),
				commit_duration_ns_total: self.commit_duration_ns_total.load(relaxed),
			}
		}
	}

	impl SqliteVfsMetrics for TestVfsMetrics {
		fn record_resolve_pages(&self, requested_pages: u64) {
			let relaxed = AtomicOrdering::Relaxed;
			self.resolve_pages_total.fetch_add(1, relaxed);
			self.resolve_pages_requested_total
				.fetch_add(requested_pages, relaxed);
		}

		fn record_resolve_cache_hits(&self, pages: u64) {
			self.resolve_pages_cache_hits_total
				.fetch_add(pages, AtomicOrdering::Relaxed);
		}

		fn record_resolve_cache_misses(&self, pages: u64) {
			self.resolve_pages_cache_misses_total
				.fetch_add(pages, AtomicOrdering::Relaxed);
		}

		fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64) {
			let relaxed = AtomicOrdering::Relaxed;
			self.get_pages_total.fetch_add(1, relaxed);
			self.pages_fetched_total.fetch_add(pages, relaxed);
			self.prefetch_pages_total.fetch_add(prefetch_pages, relaxed);
			self.bytes_fetched_total
				.fetch_add(pages.saturating_mul(page_size), relaxed);
			self.prefetch_bytes_total
				.fetch_add(prefetch_pages.saturating_mul(page_size), relaxed);
		}

		fn observe_get_pages_duration(&self, duration_ns: u64) {
			let relaxed = AtomicOrdering::Relaxed;
			self.get_pages_duration_ns_total
				.fetch_add(duration_ns, relaxed);
			self.get_pages_duration_count.fetch_add(1, relaxed);
		}

		fn record_commit(&self) {
			self.commit_total.fetch_add(1, AtomicOrdering::Relaxed);
		}

		fn observe_commit_phases(
			&self,
			request_build_ns: u64,
			serialize_ns: u64,
			transport_ns: u64,
			state_update_ns: u64,
			total_ns: u64,
		) {
			let relaxed = AtomicOrdering::Relaxed;
			self.commit_request_build_ns_total
				.fetch_add(request_build_ns, relaxed);
			self.commit_serialize_ns_total
				.fetch_add(serialize_ns, relaxed);
			self.commit_transport_ns_total
				.fetch_add(transport_ns, relaxed);
			self.commit_state_update_ns_total
				.fetch_add(state_update_ns, relaxed);
			self.commit_duration_ns_total.fetch_add(total_ns, relaxed);
		}
	}

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
				actor_id: next_test_name("sqlite-direct-actor"),
				db_dir: tempfile::tempdir().expect("temp dir should build"),
				subspace: Subspace::new(&("sqlite-direct", random_hex())),
			}
		}

		async fn open_engine(&self) -> Arc<SqliteEngine> {
			let mut attempts = 0;
			let driver = loop {
				match universaldb::driver::RocksDbDatabaseDriver::new(
					self.db_dir.path().to_path_buf(),
				)
				.await
				{
					Ok(driver) => break driver,
					Err(_err) if attempts < 50 => {
						attempts += 1;
						std::thread::sleep(std::time::Duration::from_millis(10));
					}
					Err(err) => panic!("rocksdb driver should build: {err:#}"),
				}
			};
			let db = universaldb::Database::new(Arc::new(driver));
			let (engine, _compaction_rx) = SqliteEngine::new(db, self.subspace.clone());

			Arc::new(engine)
		}

		async fn startup_data_for(
			&self,
			actor_id: &str,
			engine: &SqliteEngine,
		) -> protocol::SqliteStartupData {
			let takeover = engine
				.open(
					actor_id,
					sqlite_storage::open::OpenConfig::new(
						sqlite_now_ms().expect("startup time should resolve"),
					),
				)
				.await
				.expect("open should succeed");

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
				config: VfsConfig,
			) -> NativeDatabase {
				self.open_db_on_engine_with_metrics(runtime, engine, actor_id, config, None)
			}

			fn open_vfs_on_engine_with_metrics(
				&self,
				runtime: &tokio::runtime::Runtime,
				engine: Arc<SqliteEngine>,
				actor_id: &str,
				name: &str,
				config: VfsConfig,
				metrics: Option<Arc<dyn SqliteVfsMetrics>>,
			) -> NativeVfsHandle {
				let startup = runtime.block_on(self.startup_data_for(actor_id, &engine));
				SqliteVfs::register_with_transport(
					name,
					SqliteTransport::from_direct(engine),
					actor_id.to_string(),
					runtime.handle().clone(),
					startup,
					config,
					metrics,
				)
				.expect("v2 vfs should register")
			}

			fn open_db_on_engine_with_metrics(
				&self,
				runtime: &tokio::runtime::Runtime,
				engine: Arc<SqliteEngine>,
				actor_id: &str,
				config: VfsConfig,
				metrics: Option<Arc<dyn SqliteVfsMetrics>>,
			) -> NativeDatabase {
				let vfs_name = next_test_name("sqlite-direct-vfs");
				let vfs = self.open_vfs_on_engine_with_metrics(
					runtime, engine, actor_id, &vfs_name, config, metrics,
				);

				open_database(vfs, actor_id).expect("sqlite database should open")
			}

			fn open_db(&self, runtime: &tokio::runtime::Runtime) -> NativeDatabase {
				let engine = runtime.block_on(self.open_engine());
				self.open_db_on_engine(runtime, engine, &self.actor_id, VfsConfig::default())
			}

			fn open_db_with_metrics(
				&self,
				runtime: &tokio::runtime::Runtime,
				metrics: Arc<TestVfsMetrics>,
			) -> NativeDatabase {
				let engine = runtime.block_on(self.open_engine());
				self.open_db_on_engine_with_metrics(
					runtime,
					engine,
					&self.actor_id,
					VfsConfig::default(),
					Some(metrics),
				)
			}
		}

	fn direct_vfs_ctx(db: &NativeDatabase) -> &VfsContext {
		unsafe { &*db.vfs.inner.ctx_ptr }
	}

	fn direct_connection_vfs_ctx(connection: &NativeConnection) -> &VfsContext {
		unsafe { &*connection._vfs.inner.ctx_ptr }
	}

	fn sqlite_vfs_registered(name: &str) -> bool {
		let name = CString::new(name).expect("vfs name should not contain NUL");
		unsafe { !sqlite3_vfs_find(name.as_ptr()).is_null() }
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

	fn test_vfs_file(ctx: &VfsContext, role: VfsFileRole) -> Box<VfsFile> {
		Box::new(VfsFile {
			base: sqlite3_file {
				pMethods: ctx.io_methods.as_ref(),
			},
			ctx: ctx as *const VfsContext,
			aux: ptr::null_mut(),
			role,
		})
	}

	fn test_vfs_file_ptr(file: &mut VfsFile) -> *mut sqlite3_file {
		(file as *mut VfsFile).cast()
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
	fn predictor_prefers_reverse_stride_after_repeated_reads() {
		let mut predictor = PrefetchPredictor::default();
		for pgno in [20, 17, 14, 11] {
			predictor.record(pgno);
		}

		assert_eq!(predictor.multi_predict(11, 3, 30), vec![8, 5, 2]);
	}

	#[test]
	fn adaptive_read_ahead_tolerates_jumps_and_decays() {
		let config = VfsConfig::default();
		let mut read_ahead = AdaptiveReadAhead::default();

		for pgno in 10..=12 {
			assert_eq!(
				read_ahead.record_and_plan(&[pgno], &config).mode,
				ReadAheadMode::Bounded
			);
		}

		let scan_plan = read_ahead.record_and_plan(&[13], &config);
		assert_eq!(scan_plan.mode, ReadAheadMode::ForwardScan);
		assert!(scan_plan.depth > DEFAULT_PREFETCH_DEPTH);

		let jump_plan = read_ahead.record_and_plan(&[2], &config);
		assert_eq!(jump_plan.mode, ReadAheadMode::Bounded);

		let resumed_scan_plan = read_ahead.record_and_plan(&[14], &config);
		assert_eq!(resumed_scan_plan.mode, ReadAheadMode::ForwardScan);

		for pgno in [500, 200, 700, 50, 900, 30] {
			assert_eq!(
				read_ahead.record_and_plan(&[pgno], &config).mode,
				ReadAheadMode::Bounded
			);
		}

		let scattered_followup = read_ahead.record_and_plan(&[31], &config);
		assert_eq!(scattered_followup.mode, ReadAheadMode::Bounded);
	}

	#[test]
	fn adaptive_read_ahead_detects_backward_scans_and_decays() {
		let config = VfsConfig::default();
		let mut read_ahead = AdaptiveReadAhead::default();

		for pgno in (98..=100).rev() {
			assert_eq!(
				read_ahead.record_and_plan(&[pgno], &config).mode,
				ReadAheadMode::Bounded
			);
		}

		let scan_plan = read_ahead.record_and_plan(&[97], &config);
		assert_eq!(scan_plan.mode, ReadAheadMode::BackwardScan);
		assert!(scan_plan.depth > DEFAULT_PREFETCH_DEPTH);

		for pgno in [500, 200, 700, 50, 900, 30] {
			assert_eq!(
				read_ahead.record_and_plan(&[pgno], &config).mode,
				ReadAheadMode::Bounded
			);
		}
	}

	#[test]
	fn recent_page_tracker_keeps_full_scan_as_range_from_start() {
		let mut tracker = RecentPageTracker::new(4, 2);
		tracker.record_pages(1..=100);

		let snapshot = tracker.snapshot();
		assert!(snapshot.pgnos.len() <= 4);
		assert_eq!(
			snapshot.ranges,
			vec![VfsPreloadHintRange {
				start_pgno: 1,
				page_count: 100,
			}]
		);
	}

	#[test]
	fn recent_page_tracker_keeps_hot_pages_bounded() {
		let mut tracker = RecentPageTracker::new(3, 2);
		for pgno in [10, 20, 30, 40, 20, 30, 20] {
			tracker.record_page(pgno);
		}

		let snapshot = tracker.snapshot();
		assert!(snapshot.ranges.is_empty());
		assert_eq!(snapshot.pgnos.len(), 3);
		assert!(snapshot.pgnos.contains(&20));
		assert!(snapshot.pgnos.contains(&30));
	}

	#[test]
	fn resolve_pages_records_recent_page_hint_snapshot() {
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig {
				recent_hint_page_budget: 4,
				recent_hint_range_budget: 2,
				..VfsConfig::default()
			},
			unsafe { std::mem::zeroed() },
			None,
		);

		for pgno in 50..=65 {
			ctx.resolve_pages(&[pgno], true)
				.expect("page should resolve");
		}

		assert_eq!(
			ctx.snapshot_preload_hints().ranges,
			vec![VfsPreloadHintRange {
				start_pgno: 50,
				page_count: 16,
			}]
		);
	}

	#[test]
	fn default_vfs_config_prefetches_one_shard_for_forward_scan() {
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::default(),
			unsafe { std::mem::zeroed() },
			None,
		);

		for pgno in [10, 11, 12] {
			ctx.resolve_pages(&[pgno], true)
				.expect("page should resolve");
		}

		let requests = protocol.get_pages_requests();
		let shard_fetch = requests.last().expect("scan should fetch missing pages");
		let expected = (12..76).collect::<Vec<_>>();
		assert_eq!(shard_fetch.pgnos, expected);
	}

	#[test]
	fn default_vfs_config_uses_range_for_backward_scan() {
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::default(),
			unsafe { std::mem::zeroed() },
			None,
		);

		for pgno in [75, 74, 73, 72] {
			ctx.resolve_pages(&[pgno], true)
				.expect("page should resolve");
		}

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 3);
		assert_eq!(requests[2].pgnos, vec![73]);
		let range_requests = protocol.get_page_range_requests();
		assert_eq!(range_requests.len(), 1);
		assert_eq!(range_requests[0].start_pgno, 1);
		assert_eq!(range_requests[0].max_pages, 72);
		assert_eq!(range_requests[0].max_bytes, 1024 * 1024);
	}

	#[test]
	fn disabled_read_ahead_flag_disables_bounded_prefetch() {
		let config = VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
			read_ahead: false,
			..SqliteOptimizationFlags::default()
		});

		assert_eq!(config.prefetch_depth, 0);
	}

	#[test]
	fn cache_hit_reads_train_forward_scan_prefetch() {
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
				pages: (10..76)
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(vec![(pgno % 251) as u8; 4096]),
					})
					.collect(),
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		protocol.get_page_range_response =
			protocol::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(
				protocol::SqliteGetPageRangeOk {
					start_pgno: 76,
					pages: (76..332)
						.map(|pgno| protocol::SqliteFetchedPage {
							pgno,
							bytes: Some(vec![(pgno % 251) as u8; 4096]),
						})
						.collect(),
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			);
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 400,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::default(),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[10], true)
			.expect("first missing page should resolve");
		for pgno in 11..76 {
			ctx.resolve_pages(&[pgno], true)
				.expect("cache-hit page should resolve");
		}
		ctx.resolve_pages(&[76], true)
			.expect("next missing page should resolve");

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 1);
		assert_eq!(requests[0].pgnos, vec![10]);
		let range_requests = protocol.get_page_range_requests();
		assert_eq!(range_requests.len(), 1);
		assert_eq!(range_requests[0].start_pgno, 76);
		assert_eq!(range_requests[0].max_pages, 256);
		assert_eq!(range_requests[0].max_bytes, 1024 * 1024);
	}

	#[test]
	fn disabled_range_reads_keep_forward_scan_on_get_pages() {
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
				pages: (10..76)
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(vec![(pgno % 251) as u8; 4096]),
					})
					.collect(),
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 400,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				range_reads: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[10], true)
			.expect("first missing page should resolve");
		for pgno in 11..76 {
			ctx.resolve_pages(&[pgno], true)
				.expect("cache-hit page should resolve");
		}
		ctx.resolve_pages(&[76], true)
			.expect("next missing page should resolve");

		assert!(protocol.get_page_range_requests().is_empty());
		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 2);
		assert_eq!(requests[1].pgnos, (76..332).collect::<Vec<_>>());
	}

	#[test]
	fn cache_hit_reads_train_backward_scan_range_prefetch() {
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
				pages: (95..=350)
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(vec![(pgno % 251) as u8; 4096]),
					})
					.collect(),
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		protocol.get_page_range_response =
			protocol::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(
				protocol::SqliteGetPageRangeOk {
					start_pgno: 1,
					pages: (1..95)
						.map(|pgno| protocol::SqliteFetchedPage {
							pgno,
							bytes: Some(vec![(pgno % 251) as u8; 4096]),
						})
						.collect(),
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			);
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 400,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::default(),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[350], true)
			.expect("first missing page should resolve");
		for pgno in (95..350).rev() {
			ctx.resolve_pages(&[pgno], true)
				.expect("cache-hit page should resolve");
		}
		ctx.resolve_pages(&[94], true)
			.expect("next missing page should resolve");

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 1);
		assert_eq!(requests[0].pgnos, vec![350]);
		let range_requests = protocol.get_page_range_requests();
		assert_eq!(range_requests.len(), 1);
		assert_eq!(range_requests[0].start_pgno, 1);
		assert_eq!(range_requests[0].max_pages, 94);
		assert_eq!(range_requests[0].max_bytes, 1024 * 1024);
	}

	#[test]
	fn disabled_adaptive_read_ahead_keeps_forward_scan_to_one_shard() {
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
				pages: (10..76)
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(vec![(pgno % 251) as u8; 4096]),
					})
					.collect(),
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 400,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				adaptive_read_ahead: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[10], true)
			.expect("first missing page should resolve");
		for pgno in 11..76 {
			ctx.resolve_pages(&[pgno], true)
				.expect("cache-hit page should resolve");
		}
		ctx.resolve_pages(&[76], true)
			.expect("next missing page should resolve");

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 2);
		assert_eq!(requests[1].pgnos, (76..140).collect::<Vec<_>>());
	}

	#[test]
	fn disabled_cache_hit_training_bypasses_hit_path_predictor_updates() {
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
				pages: (10..76)
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: Some(vec![(pgno % 251) as u8; 4096]),
					})
					.collect(),
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				cache_hit_predictor_training: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[10], true)
			.expect("first missing page should resolve");
		for pgno in 11..76 {
			ctx.resolve_pages(&[pgno], true)
				.expect("cache-hit page should resolve");
		}
		ctx.resolve_pages(&[76], true)
			.expect("next missing page should resolve");

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 2);
		assert_eq!(requests[1].pgnos, vec![76]);
	}

	#[test]
	fn disabled_recent_page_hints_return_empty_snapshot() {
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				recent_page_hints: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		for pgno in 50..=65 {
			ctx.resolve_pages(&[pgno], true)
				.expect("page should resolve");
		}

		assert_eq!(ctx.snapshot_preload_hints(), VfsPreloadHintSnapshot::default());
	}

	#[test]
	fn default_vfs_config_keeps_point_reads_bounded() {
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 200,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::default(),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[90], true)
			.expect("point read should resolve");

		let requests = protocol.get_pages_requests();
		assert_eq!(requests.len(), 1);
		assert_eq!(requests[0].pgnos, vec![90]);
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

		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
				startup,
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
			);

		assert_eq!(ctx.state.read().page_cache.get(&1), Some(vec![7; 4096]));
		assert!(protocol.get_pages_requests().is_empty());
	}

	#[test]
	fn disabled_startup_preloaded_page_cache_fetches_on_first_read() {
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
					pgno: 1,
					bytes: Some(vec![9; 4096]),
				}],
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 3,
				meta: sqlite_meta(8 * 1024 * 1024),
				preloaded_pages: vec![protocol::SqliteFetchedPage {
					pgno: 1,
					bytes: Some(vec![7; 4096]),
				}],
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				vfs_cache_startup_preloaded_pages: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		assert!(ctx.state.read().page_cache.get(&1).is_none());
		let resolved = ctx
			.resolve_pages(&[1], false)
			.expect("disabled startup cache should fetch page");

		assert_eq!(resolved.get(&1), Some(&Some(vec![9; 4096])));
		assert_eq!(protocol.get_pages_requests().len(), 1);
	}

	#[test]
	fn disabled_fetched_page_cache_re_fetches_target_reads() {
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
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 3,
				meta: protocol::SqliteMeta {
					db_size_pages: 4,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				vfs_cache_fetched_pages: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[2], false)
			.expect("first target read should fetch");
		ctx.resolve_pages(&[2], false)
			.expect("disabled fetched cache should fetch again");

		assert_eq!(protocol.get_pages_requests().len(), 2);
	}

	#[test]
	fn disabled_prefetched_page_cache_re_fetches_prefetch_hits() {
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
				pages: vec![
					protocol::SqliteFetchedPage {
						pgno: 2,
						bytes: Some(vec![2; 4096]),
					},
					protocol::SqliteFetchedPage {
						pgno: 3,
						bytes: Some(vec![3; 4096]),
					},
				],
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 3,
				meta: protocol::SqliteMeta {
					db_size_pages: 4,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				vfs_cache_prefetched_pages: false,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[2], false)
			.expect("first target read should fetch target and prefetch page");
		ctx.resolve_pages(&[3], false)
			.expect("disabled prefetch cache should fetch prefetched page again");

		assert_eq!(protocol.get_pages_requests().len(), 2);
	}

	#[test]
	fn scan_resistant_cache_preserves_startup_and_early_pages() {
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
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 3,
				meta: protocol::SqliteMeta {
					db_size_pages: 100,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: vec![protocol::SqliteFetchedPage {
					pgno: 1,
					bytes: Some(vec![1; 4096]),
				}],
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				vfs_page_cache_capacity_pages: 2,
				vfs_protected_cache_pages: 4,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[2], false)
			.expect("early page read should resolve");
		{
			let state = ctx.state.write();
			for pgno in 10..80 {
				state.page_cache.insert(pgno, vec![(pgno % 251) as u8; 4096]);
			}
			state.page_cache.invalidate_all();
		}

		ctx.resolve_pages(&[1], false)
			.expect("startup page should survive scan churn");
		ctx.resolve_pages(&[2], false)
			.expect("early page should survive scan churn");

		assert_eq!(protocol.get_pages_requests().len(), 1);
	}

	#[test]
	fn scan_resistant_cache_preserves_hot_pages_after_scan_churn() {
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
				pages: vec![
					protocol::SqliteFetchedPage {
						pgno: 2,
						bytes: Some(vec![2; 4096]),
					},
					protocol::SqliteFetchedPage {
						pgno: 3,
						bytes: Some(vec![3; 4096]),
					},
				],
				meta: sqlite_meta(8 * 1024 * 1024),
			});
		let protocol = Arc::new(protocol);
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol.clone()),
			protocol::SqliteStartupData {
				generation: 3,
				meta: protocol::SqliteMeta {
					db_size_pages: 100,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: Vec::new(),
			},
			VfsConfig::from_optimization_flags(SqliteOptimizationFlags {
				vfs_page_cache_capacity_pages: 2,
				vfs_protected_cache_pages: 1,
				..SqliteOptimizationFlags::default()
			}),
			unsafe { std::mem::zeroed() },
			None,
		);

		ctx.resolve_pages(&[2], false)
			.expect("early page read should resolve");
		ctx.resolve_pages(&[3], false)
			.expect("first hot page read should hit normal cache");
		ctx.resolve_pages(&[3], false)
			.expect("second hot page read should promote protection");
		{
			let state = ctx.state.write();
			for pgno in 10..80 {
				state.page_cache.insert(pgno, vec![(pgno % 251) as u8; 4096]);
			}
			state.page_cache.invalidate_all();
		}

		ctx.resolve_pages(&[3], false)
			.expect("hot page should survive scan churn");

		assert_eq!(protocol.get_pages_requests().len(), 1);
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
	fn direct_engine_accepts_actual_nul_text_when_bound_with_explicit_length() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE nul_texts (payload TEXT PRIMARY KEY, marker INTEGER NOT NULL);",
		)
		.expect("create table should succeed");

		let payload = b"actual\0nul-text";
		sqlite_insert_text_with_int_value(
			db.as_ptr(),
			"INSERT INTO nul_texts (payload, marker) VALUES (?, ?);",
			payload,
			7,
		)
		.expect("explicit-length text bind should preserve embedded nul");

		assert_eq!(
			sqlite_query_i64_bind_text(
				db.as_ptr(),
				"SELECT marker FROM nul_texts WHERE payload = ?;",
				payload,
			)
			.expect("lookup by embedded-nul text should succeed"),
			7
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "SELECT hex(payload) FROM nul_texts;")
				.expect("hex query should succeed"),
			"61637475616C006E756C2D74657874"
		);
	}

	#[test]
	fn direct_engine_handles_boundary_primary_keys() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE boundary_keys (key_text TEXT PRIMARY KEY, value INTEGER NOT NULL);",
		)
		.expect("create table should succeed");

		let mut keys = vec![
			Vec::from("".as_bytes()),
			Vec::from(" ".as_bytes()),
			Vec::from("slash/key".as_bytes()),
			Vec::from("comma,key".as_bytes()),
			Vec::from("percent%key".as_bytes()),
			Vec::from("CaseKey".as_bytes()),
			Vec::from("casekey".as_bytes()),
			vec![b'k'; 2048],
		];
		for i in 0..256 {
			keys.push(format!("seq-{i:04}").into_bytes());
		}

		for (index, key) in keys.iter().enumerate() {
			sqlite_insert_text_with_int_value(
				db.as_ptr(),
				"INSERT INTO boundary_keys (key_text, value) VALUES (?, ?);",
				key,
				index as i64,
			)
			.expect("boundary key insert should succeed");
		}

		for (index, key) in keys.iter().enumerate() {
			assert_eq!(
				sqlite_query_i64_bind_text(
					db.as_ptr(),
					"SELECT value FROM boundary_keys WHERE key_text = ?;",
					key,
				)
				.expect("boundary key lookup should succeed"),
				index as i64
			);
		}

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM boundary_keys;")
				.expect("count should succeed"),
			keys.len() as i64
		);
	}

	#[test]
	fn direct_engine_keeps_shadow_checksum_transaction_consistent() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
		)
		.expect("create items should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE shadow_checksums (name TEXT PRIMARY KEY, total INTEGER NOT NULL);",
		)
		.expect("create shadow table should succeed");

		let expected_total = (1..=128).sum::<i64>();
		sqlite_exec(db.as_ptr(), "BEGIN").expect("begin should succeed");
		for i in 1..=128 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!("INSERT INTO items (id, value) VALUES ({i}, {i});"),
			)
			.expect("item insert should succeed");
		}
		sqlite_step_statement(
			db.as_ptr(),
			&format!(
				"INSERT INTO shadow_checksums (name, total) VALUES ('items', {expected_total});"
			),
		)
		.expect("shadow insert should succeed");
		sqlite_exec(db.as_ptr(), "COMMIT").expect("commit should succeed");

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(value) FROM items;")
				.expect("item sum should succeed"),
			expected_total
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT total FROM shadow_checksums WHERE name = 'items';",
			)
			.expect("shadow total should succeed"),
			expected_total
		);
	}

	#[test]
	fn direct_engine_mixed_row_model_preserves_invariants() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (item_key TEXT PRIMARY KEY, value TEXT NOT NULL, version INTEGER NOT NULL);",
		)
		.expect("create table should succeed");

		for i in 0..64 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO items (item_key, value, version) VALUES ('item-{i:02}', 'insert-{i}', 1);"
				),
			)
			.expect("seed insert should succeed");
		}

		for i in 0..32 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"UPDATE items SET value = 'update-{i}', version = version + 1 WHERE item_key = 'item-{i:02}';"
				),
			)
			.expect("update should succeed");
		}

		for i in 16..24 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!("DELETE FROM items WHERE item_key = 'item-{i:02}';"),
			)
			.expect("delete should succeed");
		}

		for i in 0..16 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO items (item_key, value, version) VALUES ('item-{i:02}', 'upsert-{i}', 99)
					ON CONFLICT(item_key) DO UPDATE SET value = excluded.value, version = excluded.version;"
				),
			)
			.expect("upsert should succeed");
		}

		for i in 0..1000 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"UPDATE items SET version = version + 1, value = 'hot-{i}' WHERE item_key = 'item-00';"
				),
			)
			.expect("hot-row update should succeed");
		}

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("count should succeed"),
			56
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT version FROM items WHERE item_key = 'item-00';"
			)
			.expect("hot-row version should succeed"),
			1099
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "PRAGMA quick_check;")
				.expect("quick_check should succeed"),
			"ok"
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "PRAGMA integrity_check;")
				.expect("integrity_check should succeed"),
			"ok"
		);
	}

	#[test]
	fn direct_engine_runs_deterministic_nasty_script() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE nasty_edge (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create nasty_edge should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE nasty_counter (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
		)
		.expect("create nasty_counter should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE nasty_rows (n INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create nasty_rows should succeed");

		for i in 0..256 {
			let size = 1 + ((131072 - 1) * i / 255);
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT OR REPLACE INTO nasty_edge (id, payload) VALUES (1, randomblob({size}));"
				),
			)
			.expect("grow-row write should succeed");
		}
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT length(payload) FROM nasty_edge WHERE id = 1;"
			)
			.expect("grown row length should succeed"),
			131072
		);

		sqlite_step_statement(
			db.as_ptr(),
			"INSERT INTO nasty_counter (id, value) VALUES (1, 0);",
		)
		.expect("seed counter should succeed");
		sqlite_exec(db.as_ptr(), "BEGIN").expect("counter begin should succeed");
		for _ in 0..10_000 {
			sqlite_step_statement(
				db.as_ptr(),
				"UPDATE nasty_counter SET value = value + 1 WHERE id = 1;",
			)
			.expect("counter update should succeed");
		}
		sqlite_exec(db.as_ptr(), "COMMIT").expect("counter commit should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT value FROM nasty_counter WHERE id = 1;")
				.expect("counter read should succeed"),
			10_000
		);

		sqlite_exec(
			db.as_ptr(),
			"CREATE INDEX idx_nasty_rows_payload ON nasty_rows(payload);",
		)
		.expect("create index should succeed");
		sqlite_exec(db.as_ptr(), "BEGIN").expect("bulk begin should succeed");
		for i in 0..10_000 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!("INSERT INTO nasty_rows (n, payload) VALUES ({i}, randomblob(64));"),
			)
			.expect("bulk insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "DELETE FROM nasty_rows WHERE n % 2 = 0;")
			.expect("bulk delete should succeed");
		sqlite_exec(db.as_ptr(), "COMMIT").expect("bulk commit should succeed");
		sqlite_exec(db.as_ptr(), "DROP INDEX idx_nasty_rows_payload;")
			.expect("drop index should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM nasty_rows;")
				.expect("remaining row count should succeed"),
			5000
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_nasty_rows_payload';",
			)
			.expect("index absence should succeed"),
			0
		);

		sqlite_exec(db.as_ptr(), "BEGIN").expect("rollback begin should succeed");
		for i in 0..1000 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO nasty_rows (n, payload) VALUES ({}, randomblob(8));",
					20_000 + i
				),
			)
			.expect("rollback insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "ROLLBACK").expect("rollback should succeed");
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM nasty_rows WHERE n >= 20000;"
			)
			.expect("rollback count should succeed"),
			0
		);
	}

	#[test]
	fn direct_engine_handles_page_boundary_payloads_and_text_roundtrip() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE payload_matrix (
				id INTEGER PRIMARY KEY,
				blob_payload BLOB NOT NULL,
				unicode_text TEXT NOT NULL,
				escaped_text TEXT NOT NULL
			);",
		)
		.expect("create table should succeed");

		let sizes = [
			1, 4095, 4096, 4097, 8191, 8192, 8193, 32768, 65535, 65536, 98304, 131072,
		];
		for (index, size) in sizes.iter().enumerate() {
			let id = index + 1;
			let unicode_text = format!("snowman-{id}-こんにちは-ß-🧪");
			let escaped_text = format!("escaped\\\\0-row-{id}");
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO payload_matrix (id, blob_payload, unicode_text, escaped_text)
					VALUES ({id}, zeroblob({size}), '{}', '{}');",
					unicode_text.replace('\'', "''"),
					escaped_text.replace('\'', "''"),
				),
			)
			.expect("boundary payload insert should succeed");
		}

		for (index, size) in sizes.iter().enumerate() {
			let id = index + 1;
			assert_eq!(
				sqlite_query_i64(
					db.as_ptr(),
					&format!("SELECT length(blob_payload) FROM payload_matrix WHERE id = {id};"),
				)
				.expect("blob length query should succeed"),
				*size as i64
			);
		}

		assert_eq!(
			sqlite_query_text(
				db.as_ptr(),
				"SELECT unicode_text FROM payload_matrix WHERE id = 4;",
			)
			.expect("unicode text should roundtrip"),
			"snowman-4-こんにちは-ß-🧪"
		);
		assert_eq!(
			sqlite_query_text(
				db.as_ptr(),
				"SELECT escaped_text FROM payload_matrix WHERE id = 4;",
			)
			.expect("escaped nul text should roundtrip"),
			"escaped\\\\0-row-4"
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "PRAGMA quick_check;")
				.expect("quick_check should succeed"),
			"ok"
		);
	}

	#[test]
	fn direct_engine_enforces_constraints_savepoints_and_relational_invariants() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(db.as_ptr(), "PRAGMA foreign_keys = ON;")
			.expect("foreign_keys pragma should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE users (
				id INTEGER PRIMARY KEY,
				email TEXT NOT NULL UNIQUE,
				name TEXT NOT NULL
			);",
		)
		.expect("create users should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE inventory (
				sku TEXT PRIMARY KEY,
				stock INTEGER NOT NULL CHECK(stock >= 0)
			);",
		)
		.expect("create inventory should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE orders (
				id INTEGER PRIMARY KEY,
				user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
				total_cents INTEGER NOT NULL CHECK(total_cents >= 0),
				status TEXT NOT NULL
			);",
		)
		.expect("create orders should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE order_items (
				order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
				sku TEXT NOT NULL REFERENCES inventory(sku),
				qty INTEGER NOT NULL CHECK(qty > 0),
				price_cents INTEGER NOT NULL CHECK(price_cents >= 0),
				PRIMARY KEY(order_id, sku)
			) WITHOUT ROWID;",
		)
		.expect("create order_items should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE payments (
				id INTEGER PRIMARY KEY,
				order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
				amount_cents INTEGER NOT NULL CHECK(amount_cents >= 0),
				status TEXT NOT NULL
			);",
		)
		.expect("create payments should succeed");

		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO users (id, email, name) VALUES (1, 'alice@example.com', 'Alice');",
		)
		.expect("seed user should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO inventory (sku, stock) VALUES ('sku-red', 10), ('sku-blue', 8);",
		)
		.expect("seed inventory should succeed");

		assert!(
			sqlite_exec(
				db.as_ptr(),
				"INSERT INTO users (id, email, name) VALUES (2, 'alice@example.com', 'Again');",
			)
			.expect_err("unique violation should fail")
			.contains("UNIQUE constraint failed")
		);
		assert!(
			sqlite_exec(
				db.as_ptr(),
				"INSERT INTO users (id, email, name) VALUES (3, 'null@example.com', NULL);",
			)
			.expect_err("not-null violation should fail")
			.contains("NOT NULL constraint failed")
		);
		assert!(
			sqlite_exec(
				db.as_ptr(),
				"UPDATE inventory SET stock = -1 WHERE sku = 'sku-red';"
			)
			.expect_err("check violation should fail")
			.contains("CHECK constraint failed")
		);
		assert!(
			sqlite_exec(
				db.as_ptr(),
				"INSERT INTO orders (id, user_id, total_cents, status) VALUES (9, 999, 100, 'pending');",
			)
			.expect_err("foreign-key violation should fail")
			.contains("FOREIGN KEY constraint failed")
		);

		sqlite_exec(db.as_ptr(), "BEGIN").expect("begin should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO orders (id, user_id, total_cents, status) VALUES (1, 1, 700, 'paid');",
		)
		.expect("order insert should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO order_items (order_id, sku, qty, price_cents)
			VALUES (1, 'sku-red', 2, 150), (1, 'sku-blue', 1, 400);",
		)
		.expect("order items insert should succeed");
		sqlite_exec(
			db.as_ptr(),
			"UPDATE inventory
			SET stock = stock - CASE sku
				WHEN 'sku-red' THEN 2
				WHEN 'sku-blue' THEN 1
				ELSE 0
			END
			WHERE sku IN ('sku-red', 'sku-blue');",
		)
		.expect("inventory update should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO payments (id, order_id, amount_cents, status) VALUES (1, 1, 700, 'captured');",
		)
		.expect("payment insert should succeed");
		sqlite_exec(db.as_ptr(), "COMMIT").expect("commit should succeed");

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT total_cents FROM orders WHERE id = 1;",)
				.expect("order total should succeed"),
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT SUM(qty * price_cents) FROM order_items WHERE order_id = 1;",
			)
			.expect("item sum should succeed")
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT SUM(amount_cents) FROM payments WHERE order_id = 1 AND status = 'captured';",
			)
			.expect("captured payment sum should succeed"),
			700
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT SUM(stock) FROM inventory WHERE sku IN ('sku-red', 'sku-blue');",
			)
			.expect("inventory sum should succeed"),
			15
		);

		sqlite_exec(db.as_ptr(), "SAVEPOINT sp_order_rollback;").expect("savepoint should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO orders (id, user_id, total_cents, status) VALUES (2, 1, 123, 'draft');",
		)
		.expect("draft order insert should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO order_items (order_id, sku, qty, price_cents) VALUES (2, 'sku-red', 1, 123);",
		)
		.expect("draft item insert should succeed");
		sqlite_exec(db.as_ptr(), "ROLLBACK TO sp_order_rollback;")
			.expect("rollback to savepoint should succeed");
		sqlite_exec(db.as_ptr(), "RELEASE sp_order_rollback;")
			.expect("release savepoint should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM orders WHERE id = 2;")
				.expect("rolled-back order should be absent"),
			0
		);

		sqlite_exec(db.as_ptr(), "SAVEPOINT sp_order_release;")
			.expect("release savepoint should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO orders (id, user_id, total_cents, status) VALUES (3, 1, 50, 'pending');",
		)
		.expect("released order insert should succeed");
		sqlite_exec(db.as_ptr(), "RELEASE sp_order_release;")
			.expect("release savepoint should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM orders WHERE id = 3;")
				.expect("released order should exist"),
			1
		);

		sqlite_exec(db.as_ptr(), "BEGIN").expect("rollback begin should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO payments (id, order_id, amount_cents, status) VALUES (2, 3, 50, 'captured');",
		)
		.expect("rollback payment insert should succeed");
		sqlite_exec(db.as_ptr(), "ROLLBACK").expect("rollback should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM payments WHERE id = 2;")
				.expect("rolled-back payment should be absent"),
			0
		);

		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO users (id, email, name) VALUES (7, 'idempotent@example.com', 'Replay')
			ON CONFLICT(id) DO NOTHING;",
		)
		.expect("idempotent insert should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO users (id, email, name) VALUES (7, 'idempotent@example.com', 'Replay')
			ON CONFLICT(id) DO NOTHING;",
		)
		.expect("idempotent replay should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM users WHERE id = 7;")
				.expect("idempotent user count should succeed"),
			1
		);

		sqlite_exec(db.as_ptr(), "DELETE FROM orders WHERE id = 1;")
			.expect("cascade delete should succeed");
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM order_items WHERE order_id = 1;"
			)
			.expect("cascaded order items should be removed"),
			0
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM payments WHERE order_id = 1;"
			)
			.expect("cascaded payments should be removed"),
			0
		);
	}

	#[test]
	fn direct_engine_handles_schema_churn_index_parity_and_pragmas() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		assert_eq!(
			sqlite_query_text(db.as_ptr(), "PRAGMA journal_mode = DELETE;")
				.expect("journal_mode pragma should succeed"),
			"delete"
		);
		sqlite_exec(db.as_ptr(), "PRAGMA synchronous = NORMAL;")
			.expect("synchronous pragma should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA cache_size = -2000;")
			.expect("cache_size pragma should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA foreign_keys = ON;")
			.expect("foreign_keys pragma should succeed");
		sqlite_exec(db.as_ptr(), "PRAGMA auto_vacuum = NONE;")
			.expect("auto_vacuum pragma should succeed");
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "PRAGMA cache_size;")
				.expect("cache_size read should succeed"),
			-2000
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "PRAGMA foreign_keys;")
				.expect("foreign_keys read should succeed"),
			1
		);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (
				id INTEGER PRIMARY KEY,
				bucket INTEGER NOT NULL,
				key_text TEXT NOT NULL,
				value INTEGER NOT NULL
			);",
		)
		.expect("create items should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE item_ops (
				seq INTEGER PRIMARY KEY AUTOINCREMENT,
				kind TEXT NOT NULL,
				item_id INTEGER NOT NULL
			);",
		)
		.expect("create item_ops should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE ledger (
				account_id TEXT NOT NULL,
				entry_id INTEGER NOT NULL,
				amount INTEGER NOT NULL,
				PRIMARY KEY(account_id, entry_id)
			) WITHOUT ROWID;",
		)
		.expect("create ledger should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE VIEW active_items AS
			SELECT id, bucket, key_text, value FROM items WHERE value >= 0;",
		)
		.expect("create view should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TRIGGER items_ai AFTER INSERT ON items
			BEGIN
				INSERT INTO item_ops (kind, item_id) VALUES ('insert', NEW.id);
			END;",
		)
		.expect("create insert trigger should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TRIGGER items_ad AFTER DELETE ON items
			BEGIN
				INSERT INTO item_ops (kind, item_id) VALUES ('delete', OLD.id);
			END;",
		)
		.expect("create delete trigger should succeed");
		sqlite_exec(
			db.as_ptr(),
			"ALTER TABLE items ADD COLUMN tag TEXT NOT NULL DEFAULT 'base';",
		)
		.expect("alter table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE INDEX idx_items_bucket_key ON items(bucket, key_text);",
		)
		.expect("create compound index should succeed");

		sqlite_exec(db.as_ptr(), "BEGIN").expect("begin should succeed");
		for i in 0..128 {
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO items (id, bucket, key_text, value, tag)
					VALUES ({i}, {}, 'k-{i:03}', {}, 'tag-{}');",
					i % 8,
					i * 2,
					i % 5,
				),
			)
			.expect("items insert should succeed");
			sqlite_step_statement(
				db.as_ptr(),
				&format!(
					"INSERT INTO ledger (account_id, entry_id, amount) VALUES ('acct-{}', {i}, {});",
					i % 4,
					(i as i64) - 32,
				),
			)
			.expect("ledger insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "COMMIT").expect("commit should succeed");
		sqlite_exec(db.as_ptr(), "DELETE FROM items WHERE id % 9 = 0;")
			.expect("delete should succeed");

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM active_items;")
				.expect("view query should succeed"),
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items WHERE value >= 0;")
				.expect("table query should succeed")
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT
					SUM(CASE WHEN kind = 'insert' THEN 1 ELSE 0 END) -
					SUM(CASE WHEN kind = 'delete' THEN 1 ELSE 0 END)
				FROM item_ops;",
			)
			.expect("op log delta should succeed"),
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items;")
				.expect("live item count should succeed")
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM items WHERE bucket = 3 AND key_text >= 'k-040';",
			)
			.expect("indexed scan should succeed"),
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM items NOT INDEXED WHERE bucket = 3 AND key_text >= 'k-040';",
			)
			.expect("not-indexed scan should succeed")
		);
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM ledger WHERE account_id = 'acct-2';"
			)
			.expect("without-rowid query should succeed"),
			32
		);

		sqlite_exec(db.as_ptr(), "DROP INDEX idx_items_bucket_key;")
			.expect("drop index should succeed");
		assert_eq!(
			sqlite_query_i64(
				db.as_ptr(),
				"SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_items_bucket_key';",
			)
			.expect("dropped index should be absent"),
			0
		);
	}

	#[test]
	fn direct_engine_handles_prepared_statement_churn() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE prepared_items (
				id INTEGER PRIMARY KEY,
				value TEXT NOT NULL,
				counter INTEGER NOT NULL
			);",
		)
		.expect("create table should succeed");

		for i in 1..=128 {
			let sql = format!(
				"INSERT INTO prepared_items (id, value, counter) VALUES ({i}, 'seed-{i}', 0); -- unique-sql-{i}"
			);
			let stmt = sqlite_prepare_statement(db.as_ptr(), &sql)
				.expect("unique prepared statement should prepare");
			sqlite_step_prepared(db.as_ptr(), stmt, &sql)
				.expect("unique prepared statement should execute");
			unsafe {
				sqlite3_finalize(stmt);
			}
		}

		let update_sql = "UPDATE prepared_items SET counter = counter + ?, value = ? WHERE id = ?;";
		let stmt = sqlite_prepare_statement(db.as_ptr(), update_sql)
			.expect("reused prepared statement should prepare");
		for i in 0..4000 {
			sqlite_reset_prepared(stmt, update_sql).expect("statement reset should succeed");
			sqlite_clear_bindings(stmt, update_sql).expect("binding clear should succeed");
			sqlite_bind_i64(db.as_ptr(), stmt, 1, 1, update_sql)
				.expect("increment bind should succeed");
			let value = format!("value-{i}");
			sqlite_bind_text_bytes(db.as_ptr(), stmt, 2, value.as_bytes(), update_sql)
				.expect("text bind should succeed");
			sqlite_bind_i64(db.as_ptr(), stmt, 3, (i % 128 + 1) as i64, update_sql)
				.expect("id bind should succeed");
			sqlite_step_prepared(db.as_ptr(), stmt, update_sql)
				.expect("reused prepared statement should execute");
		}
		unsafe {
			sqlite3_finalize(stmt);
		}

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM prepared_items;")
				.expect("row count should succeed"),
			128
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(counter) FROM prepared_items;")
				.expect("counter sum should succeed"),
			4000
		);
		assert_eq!(
			sqlite_query_text(
				db.as_ptr(),
				"SELECT value FROM prepared_items WHERE id = 1;",
			)
			.expect("final prepared value should succeed"),
			"value-3968"
		);
	}

	#[test]
	fn direct_engine_preserves_transaction_balance_and_fragmentation_invariants() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE accounts (
				id INTEGER PRIMARY KEY,
				balance INTEGER NOT NULL
			);",
		)
		.expect("create accounts should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE transfer_log (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				from_id INTEGER NOT NULL,
				to_id INTEGER NOT NULL,
				amount INTEGER NOT NULL
			);",
		)
		.expect("create transfer_log should succeed");
		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE frag (
				id INTEGER PRIMARY KEY,
				payload BLOB NOT NULL
			);",
		)
		.expect("create frag should succeed");

		for id in 1..=8 {
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO accounts (id, balance) VALUES ({id}, 1000);"),
			)
			.expect("seed account should succeed");
		}

		sqlite_exec(db.as_ptr(), "BEGIN").expect("transfer begin should succeed");
		for step in 0..500 {
			let from_id = step % 8 + 1;
			let to_id = (step * 5 + 3) % 8 + 1;
			let amount = step % 17 + 1;
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE accounts SET balance = balance - {amount} WHERE id = {from_id};"),
			)
			.expect("debit should succeed");
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE accounts SET balance = balance + {amount} WHERE id = {to_id};"),
			)
			.expect("credit should succeed");
			sqlite_exec(
				db.as_ptr(),
				&format!(
					"INSERT INTO transfer_log (from_id, to_id, amount) VALUES ({from_id}, {to_id}, {amount});"
				),
			)
			.expect("transfer log insert should succeed");
		}
		sqlite_exec(db.as_ptr(), "COMMIT").expect("transfer commit should succeed");

		sqlite_exec(db.as_ptr(), "BEGIN").expect("rollback begin should succeed");
		sqlite_exec(
			db.as_ptr(),
			"UPDATE accounts SET balance = balance - 777 WHERE id = 1;",
		)
		.expect("rollback debit should succeed");
		sqlite_exec(
			db.as_ptr(),
			"UPDATE accounts SET balance = balance + 777 WHERE id = 2;",
		)
		.expect("rollback credit should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO transfer_log (from_id, to_id, amount) VALUES (1, 2, 777);",
		)
		.expect("rollback transfer log insert should succeed");
		sqlite_exec(db.as_ptr(), "ROLLBACK").expect("rollback should succeed");

		for id in 0..512 {
			let size = ((id * 541) % 16384) + 1;
			sqlite_exec(
				db.as_ptr(),
				&format!("INSERT INTO frag (id, payload) VALUES ({id}, randomblob({size}));"),
			)
			.expect("fragmentation insert should succeed");
		}
		for id in (0..512).filter(|id| (id * 17 + 11) % 5 == 0) {
			sqlite_exec(db.as_ptr(), &format!("DELETE FROM frag WHERE id = {id};"))
				.expect("fragmentation delete should succeed");
		}
		for id in (0..512).filter(|id| id % 3 == 0) {
			let shrink_size = ((id * 13) % 256) + 1;
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE frag SET payload = randomblob({shrink_size}) WHERE id = {id};"),
			)
			.expect("fragmentation shrink should succeed");
		}
		for id in (0..512).filter(|id| id % 7 == 0) {
			let grow_size = 16384 + ((id * 97) % 8192);
			sqlite_exec(
				db.as_ptr(),
				&format!("UPDATE frag SET payload = randomblob({grow_size}) WHERE id = {id};"),
			)
			.expect("fragmentation grow should succeed");
		}
		sqlite_exec(db.as_ptr(), "VACUUM;").expect("vacuum should succeed");

		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(balance) FROM accounts;")
				.expect("balance sum should succeed"),
			8000
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM transfer_log;")
				.expect("transfer log count should succeed"),
			500
		);
		assert_eq!(
			sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM frag;")
				.expect("frag count should succeed"),
			410
		);
		assert!(
			sqlite_query_i64(db.as_ptr(), "SELECT SUM(length(payload)) FROM frag;")
				.expect("frag payload sum should succeed")
				> 0
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "PRAGMA integrity_check;")
				.expect("integrity_check should succeed"),
			"ok"
		);
	}

	#[test]
	fn direct_engine_batch_atomic_probe_runs_on_open() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);

		assert!(
			db.vfs.commit_atomic_count() > 0,
			"open_database should run the sqlite batch-atomic probe",
		);
	}

	#[test]
	fn reader_vfs_file_rejects_mutating_callbacks() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);
		let mut file = test_vfs_file(ctx, VfsFileRole::Reader);
		let p_file = test_vfs_file_ptr(&mut file);
		let bytes = vec![0x5a; DEFAULT_PAGE_SIZE];

		assert_eq!(
			unsafe { io_write(p_file, bytes.as_ptr().cast(), bytes.len() as c_int, 0) },
			SQLITE_IOERR_WRITE
		);
		assert_eq!(unsafe { io_truncate(p_file, 0) }, SQLITE_IOERR_TRUNCATE);
		{
			let mut state = ctx.state.write();
			state.write_buffer.dirty.insert(1, vec![0x7a; DEFAULT_PAGE_SIZE]);
		}
		assert_eq!(unsafe { io_sync(p_file, 0) }, SQLITE_IOERR_FSYNC);
		{
			let mut state = ctx.state.write();
			state.write_buffer.dirty.clear();
			state.write_buffer.in_atomic_write = false;
		}
		assert_eq!(
			unsafe { io_file_control(p_file, SQLITE_FCNTL_BEGIN_ATOMIC_WRITE, ptr::null_mut()) },
			SQLITE_READONLY
		);
		assert!(
			ctx.clone_last_error()
				.expect("reader mutation should set last error")
				.contains("reader sqlite VFS handle attempted mutating operation")
		);
	}

	#[test]
	fn writer_vfs_file_supports_write_callback() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);
		let mut file = test_vfs_file(ctx, VfsFileRole::Writer);
		let p_file = test_vfs_file_ptr(&mut file);
		let bytes = vec![0x5a; DEFAULT_PAGE_SIZE];

		assert_eq!(
			unsafe { io_write(p_file, bytes.as_ptr().cast(), bytes.len() as c_int, 0) },
			SQLITE_OK
		);
		{
			let mut state = ctx.state.write();
			assert!(state.write_buffer.dirty.contains_key(&1));
			state.write_buffer.dirty.clear();
		}
	}

	#[test]
	fn vfs_open_sets_role_flags_and_denies_reader_aux_creation() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);
		let actor = CString::new(harness.actor_id.as_str()).expect("actor id should be valid");
		let mut reader_out_flags = 0;
		let mut reader_file = std::mem::MaybeUninit::<VfsFile>::uninit();

		let rc = unsafe {
			vfs_open(
				db.vfs.inner.vfs_ptr,
				actor.as_ptr(),
				reader_file.as_mut_ptr().cast(),
				SQLITE_OPEN_MAIN_DB | SQLITE_OPEN_READONLY,
				&mut reader_out_flags,
			)
		};
		assert_eq!(rc, SQLITE_OK);
		assert_ne!(reader_out_flags & SQLITE_OPEN_READONLY, 0);
		assert_eq!(reader_out_flags & SQLITE_OPEN_READWRITE, 0);
		let mut reader_file = unsafe { reader_file.assume_init() };
		assert_eq!(reader_file.role, VfsFileRole::Reader);
		assert_eq!(
			unsafe { io_close(test_vfs_file_ptr(&mut reader_file)) },
			SQLITE_OK
		);

		let aux_path = CString::new("reader-scratch").expect("aux path should be valid");
		let mut aux_out_flags = 0;
		let mut aux_file = std::mem::MaybeUninit::<VfsFile>::uninit();
		let rc = unsafe {
			vfs_open(
				db.vfs.inner.vfs_ptr,
				aux_path.as_ptr(),
				aux_file.as_mut_ptr().cast(),
				SQLITE_OPEN_CREATE | SQLITE_OPEN_READONLY,
				&mut aux_out_flags,
			)
		};
		assert_eq!(rc, SQLITE_CANTOPEN);
		assert!(
			ctx.clone_last_error()
				.expect("reader aux create should set last error")
				.contains("auxiliary file creation")
		);
	}

	#[test]
	fn reader_owned_aux_files_reject_delete() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let ctx = direct_vfs_ctx(&db);
		ctx.open_aux_file("reader-owned-journal", VfsFileRole::Reader);
		let path = CString::new("reader-owned-journal").expect("aux path should be valid");

		assert_eq!(
			unsafe { vfs_delete(db.vfs.inner.vfs_ptr, path.as_ptr(), 0) },
			SQLITE_READONLY
		);
		assert!(ctx.aux_file_exists("reader-owned-journal"));
	}

	#[test]
	fn native_vfs_handle_opens_multiple_connections_against_one_context() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let db = harness.open_db(&runtime);
		let second_connection = open_connection(
			db.vfs_handle(),
			&harness.actor_id,
			SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
		)
		.expect("second sqlite connection should open");

		assert_ne!(db.as_ptr(), second_connection.as_ptr());
		assert!(
			ptr::eq(direct_vfs_ctx(&db), direct_connection_vfs_ctx(&second_connection)),
			"connections opened from one NativeVfsHandle should share one VfsContext",
		);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE shared_connections (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO shared_connections (id, value) VALUES (1, 'visible');",
		)
		.expect("insert should succeed");
		assert_eq!(
			sqlite_query_text(
				second_connection.as_ptr(),
				"SELECT value FROM shared_connections WHERE id = 1;",
			)
			.expect("second connection should read through shared VFS"),
			"visible"
		);

		drop(db);
		assert_eq!(
			sqlite_query_text(
				second_connection.as_ptr(),
				"SELECT value FROM shared_connections WHERE id = 1;",
			)
			.expect("connection should keep shared VFS alive after manager drop"),
			"visible"
		);
	}

	#[test]
	fn native_vfs_handle_unregisters_after_last_connection_closes() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let name = next_test_name("sqlite-shared-vfs");
		let startup = runtime.block_on(harness.startup_data_for(&harness.actor_id, &engine));
		let vfs = SqliteVfs::register_with_transport(
			&name,
			SqliteTransport::from_direct(Arc::clone(&engine)),
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsConfig::default(),
			None,
		)
		.expect("vfs should register");
		let connection = open_connection(
			vfs.clone(),
			&harness.actor_id,
			SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
		)
		.expect("sqlite connection should open");
		assert!(sqlite_vfs_registered(&name));

		drop(vfs);
		assert!(
			sqlite_vfs_registered(&name),
			"an open connection should keep the VFS registered",
		);

		drop(connection);
		assert!(
			!sqlite_vfs_registered(&name),
			"VFS should unregister after the last connection closes",
		);
		let replacement_startup =
			runtime.block_on(harness.startup_data_for(&harness.actor_id, &engine));
		SqliteVfs::register_with_transport(
			&name,
			SqliteTransport::from_direct(engine),
			harness.actor_id.clone(),
			runtime.handle().clone(),
			replacement_startup,
			VfsConfig::default(),
			None,
		)
		.expect("VFS name should be reusable after the last connection closes");
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
			VfsConfig {
				cache_capacity_pages: 2,
				prefetch_depth: 0,
				max_prefetch_bytes: 0,
				..VfsConfig::default()
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
			.expect("pages should read back after slow-path commit")
			.pages;
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
		let vfs = SqliteVfs::register_with_transport(
			&next_test_name("sqlite-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsConfig::default(),
			None,
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
		let vfs = SqliteVfs::register_with_transport(
			&next_test_name("sqlite-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsConfig::default(),
			None,
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
		let vfs = SqliteVfs::register_with_transport(
			&next_test_name("sqlite-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsConfig::default(),
			None,
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
		// Forced-sync: this test shares one SQLite handle across std::thread workers.
		let db = Arc::new(SyncMutex::new(harness.open_db(&runtime)));

		{
			let db = db.lock();
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
					let db = db.lock();
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

		let db = db.lock();
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
		let actor_a = next_test_name("sqlite-actor-a");
		let actor_b = next_test_name("sqlite-actor-b");
		let db_a = harness.open_db_on_engine(
			&runtime,
			Arc::clone(&engine),
			&actor_a,
			VfsConfig::default(),
		);
		let db_b = harness.open_db_on_engine(&runtime, engine, &actor_b, VfsConfig::default());

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
	fn direct_engine_repeated_close_reopen_cycles_preserve_state() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let actor_id = &harness.actor_id;

		for cycle in 0..20 {
			let db =
				harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsConfig::default());
			sqlite_exec(
				db.as_ptr(),
				"CREATE TABLE IF NOT EXISTS reopen_cycles (
					id INTEGER PRIMARY KEY,
					cycle INTEGER NOT NULL,
					value INTEGER NOT NULL
				);",
			)
			.expect("create table should succeed");

			let start = cycle * 25;
			for id in start..start + 25 {
				sqlite_exec(
					db.as_ptr(),
					&format!(
						"INSERT INTO reopen_cycles (id, cycle, value) VALUES ({id}, {cycle}, {})
						ON CONFLICT(id) DO UPDATE SET cycle = excluded.cycle, value = excluded.value;",
						id * 3
					),
				)
				.expect("insert across reopen cycle should succeed");
			}

			assert_eq!(
				sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM reopen_cycles;")
					.expect("count during reopen cycle should succeed"),
				((cycle + 1) * 25) as i64
			);
		}

		let reopened =
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsConfig::default());
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT COUNT(*) FROM reopen_cycles;")
				.expect("final reopen count should succeed"),
			500
		);
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT SUM(value) FROM reopen_cycles;")
				.expect("final reopen sum should succeed"),
			(0..500).map(|id| (id * 3) as i64).sum::<i64>()
		);
		assert_eq!(
			sqlite_query_text(reopened.as_ptr(), "PRAGMA integrity_check;")
				.expect("integrity_check after reopen loop should succeed"),
			"ok"
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
	fn direct_engine_fresh_reopen_recovers_after_poisoned_handle() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		let startup = runtime.block_on(harness.startup_data_for(&harness.actor_id, &engine));
		let transport = SqliteTransport::from_direct(engine.clone());
		let hooks = transport
			.direct_hooks()
			.expect("direct transport should expose test hooks");
		let vfs = SqliteVfs::register_with_transport(
			&next_test_name("sqlite-direct-vfs"),
			transport,
			harness.actor_id.clone(),
			runtime.handle().clone(),
			startup,
			VfsConfig::default(),
			None,
		)
		.expect("v2 vfs should register");
		let db = open_database(vfs, &harness.actor_id).expect("sqlite database should open");

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE stable_rows (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO stable_rows (id, value) VALUES (1, 'committed-before-failure');",
		)
		.expect("seed write should succeed");

		hooks.fail_next_commit("InjectedTransportError: reopen recovery transport dropped");
		let err = sqlite_exec(
			db.as_ptr(),
			"INSERT INTO stable_rows (id, value) VALUES (2, 'should-not-commit');",
		)
		.expect_err("failing transport commit should surface as an IO error");
		assert!(
			err.contains("I/O") || err.contains("disk I/O"),
			"sqlite should surface transport failure as an IO error: {err}",
		);
		assert!(
			direct_vfs_ctx(&db).is_dead(),
			"transport error should kill the live VFS",
		);

		drop(db);

		let reopened = harness.open_db_on_engine(
			&runtime,
			engine.clone(),
			&harness.actor_id,
			VfsConfig::default(),
		);
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT COUNT(*) FROM stable_rows;")
				.expect("reopened count should succeed"),
			1
		);
		assert_eq!(
			sqlite_query_text(
				reopened.as_ptr(),
				"SELECT value FROM stable_rows WHERE id = 1;"
			)
			.expect("committed row should survive reopen"),
			"committed-before-failure"
		);
		assert_eq!(
			sqlite_query_i64(
				reopened.as_ptr(),
				"SELECT COUNT(*) FROM stable_rows WHERE id = 2;",
			)
			.expect("failed row should stay absent"),
			0
		);

		sqlite_exec(
			reopened.as_ptr(),
			"INSERT INTO stable_rows (id, value) VALUES (3, 'after-reopen');",
		)
		.expect("fresh reopen should accept new writes");
		assert_eq!(
			sqlite_query_i64(reopened.as_ptr(), "SELECT COUNT(*) FROM stable_rows;")
				.expect("final count should succeed"),
			2
		);
	}

		#[test]
		fn direct_engine_aux_open_failure_surfaces_without_poisoning_main_db() {
			let runtime = direct_runtime();
			let harness = DirectEngineHarness::new();
			let db = harness.open_db(&runtime);
			let ctx = direct_vfs_ctx(&db);

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");
		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO items (id, value) VALUES (1, 'still-works');",
		)
		.expect("seed write should succeed");

		ctx.fail_next_aux_open("InjectedAuxOpenError: attached db open failed");
		let err = sqlite_exec(db.as_ptr(), "ATTACH 'scratch-aux.db' AS scratch;")
			.expect_err("attach should surface aux open failure");
		assert!(
			err.contains("open") || err.contains("I/O") || err.contains("disk I/O"),
			"sqlite should surface aux open failure: {err}",
		);
		assert_eq!(
			db.take_last_kv_error().as_deref(),
			Some("InjectedAuxOpenError: attached db open failed"),
		);
		assert!(
			!ctx.is_dead(),
			"aux open failure should not poison the main db handle",
		);
		assert_eq!(
			sqlite_query_text(db.as_ptr(), "SELECT value FROM items WHERE id = 1;")
				.expect("main db should remain queryable"),
			"still-works"
		);
	}

		#[test]
		fn vfs_delete_surfaces_aux_delete_failure() {
			let runtime = direct_runtime();
			let harness = DirectEngineHarness::new();
			let db = harness.open_db(&runtime);
			let ctx = direct_vfs_ctx(&db);

		ctx.open_aux_file("actor-journal", VfsFileRole::Writer);
		ctx.fail_next_aux_delete("InjectedAuxDeleteError: delete failed");
		let path = CString::new("actor-journal").expect("cstring should build");

		let rc = unsafe { vfs_delete(db.vfs.inner.vfs_ptr, path.as_ptr(), 0) };
		assert_eq!(rc, SQLITE_IOERR_DELETE);
		assert_eq!(
			db.take_last_kv_error().as_deref(),
			Some("InjectedAuxDeleteError: delete failed"),
		);
		assert!(
			ctx.aux_file_exists("actor-journal"),
			"failed delete should leave aux state intact",
		);
	}

	#[test]
	fn direct_engine_reads_continue_while_compaction_runs() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let engine = runtime.block_on(harness.open_engine());
		// Forced-sync: the reader is a std::thread exercising SQLite VFS callbacks.
		let db = Arc::new(SyncMutex::new(harness.open_db_on_engine(
			&runtime,
			Arc::clone(&engine),
			&harness.actor_id,
			VfsConfig::default(),
		)));

		{
			let db = db.lock();
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
		// Forced-sync: error capture is written from a std::thread and read after join.
		let read_error = Arc::new(SyncMutex::new(None::<String>));
		let db_for_reader = Arc::clone(&db);
		let keep_reading_for_thread = Arc::clone(&keep_reading);
		let read_error_for_thread = Arc::clone(&read_error);
		let reader = thread::spawn(move || {
			while keep_reading_for_thread.load(AtomicOrdering::Relaxed) {
				let db = db_for_reader.lock();
				direct_vfs_ctx(&db)
					.state
					.write()
					.page_cache
					.invalidate_all();
				if let Err(err) =
					sqlite_query_i64(db.as_ptr(), "SELECT COUNT(*) FROM items WHERE id >= 1;")
				{
					*read_error_for_thread.lock() = Some(err);
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
			read_error.lock().is_none(),
			"reads should keep working while compaction folds deltas",
		);
		let db = db.lock();
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

		let vfs = SqliteVfs::register_with_transport(
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
				VfsConfig::default(),
				None,
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

		let vfs = SqliteVfs::register_with_transport(
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
				VfsConfig::default(),
				None,
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

		let vfs = SqliteVfs::register_with_transport(
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
				VfsConfig::default(),
				None,
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

		let vfs = SqliteVfs::register_with_transport(
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
				VfsConfig::default(),
				None,
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

		let vfs = SqliteVfs::register_with_transport(
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
				VfsConfig::default(),
				None,
			)
		.expect("vfs should register");
		// Forced-sync: this test moves one SQLite handle between std::thread workers.
		let db = Arc::new(SyncMutex::new(
			open_database(vfs, "actor").expect("db should open"),
		));

		{
			let db = db.clone();
			thread::spawn(move || {
				let db = db.lock();
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
			let db = db.lock();
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
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: sqlite_meta(8 * 1024 * 1024),
				preloaded_pages: Vec::new(),
				},
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
			);

		let first = ctx.open_aux_file("actor-journal", VfsFileRole::Writer);
		first.bytes.lock().extend_from_slice(&[1, 2, 3, 4]);
		let second = ctx.open_aux_file("actor-journal", VfsFileRole::Writer);
		assert_eq!(*second.bytes.lock(), vec![1, 2, 3, 4]);
		assert!(ctx.aux_file_exists("actor-journal"));

		ctx.delete_aux_file("actor-journal");
		assert!(!ctx.aux_file_exists("actor-journal"));
		assert!(
			ctx.open_aux_file("actor-journal", VfsFileRole::Writer)
				.bytes
				.lock()
				.is_empty()
		);
	}

	#[test]
	fn concurrent_aux_file_opens_share_single_state() {
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
		let ctx = Arc::new(VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(protocol),
			protocol::SqliteStartupData {
				generation: 7,
				meta: sqlite_meta(8 * 1024 * 1024),
				preloaded_pages: Vec::new(),
				},
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
			));
		let barrier = Arc::new(Barrier::new(2));

		let first = {
			let ctx = ctx.clone();
			let barrier = barrier.clone();
			thread::spawn(move || {
				barrier.wait();
				ctx.open_aux_file("actor-journal", VfsFileRole::Writer)
			})
		};
		let second = {
			let ctx = ctx.clone();
			let barrier = barrier.clone();
			thread::spawn(move || {
				barrier.wait();
				ctx.open_aux_file("actor-journal", VfsFileRole::Writer)
			})
		};

		let first = first.join().expect("first open should complete");
		let second = second.join().expect("second open should complete");
		assert!(Arc::ptr_eq(&first, &second));
		assert_eq!(ctx.aux_files.read().len(), 1);
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
		let ctx = VfsContext::new(
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
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
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
		let ctx = VfsContext::new(
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
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
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
		let ctx = VfsContext::new(
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
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
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
	fn resolve_pages_surfaces_read_path_error_response() {
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
			protocol::SqliteGetPagesResponse::SqliteErrorResponse(protocol::SqliteErrorResponse {
				message: "InjectedGetPagesError: read path dropped".to_string(),
			});
		let ctx = VfsContext::new(
			"actor".to_string(),
			runtime.handle().clone(),
			SqliteTransport::from_mock(Arc::new(protocol)),
			protocol::SqliteStartupData {
				generation: 7,
				meta: protocol::SqliteMeta {
					db_size_pages: 4,
					..sqlite_meta(8 * 1024 * 1024)
				},
				preloaded_pages: vec![protocol::SqliteFetchedPage {
					pgno: 1,
					bytes: Some(vec![1; 4096]),
				}],
				},
				VfsConfig::default(),
				unsafe { std::mem::zeroed() },
				None,
			);

		let err = ctx
			.resolve_pages(&[2], false)
			.expect_err("read-path error response should surface");
		assert!(matches!(
			err,
			GetPagesError::Other(ref message)
				if message.contains("InjectedGetPagesError")
		));
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
	fn mock_protocol_notifies_stage_response_awaits() {
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
					meta: sqlite_meta(8 * 1024 * 1024),
				},
			),
		));

		runtime.block_on(async {
			let wait = protocol.wait_for_stage_responses(1);
			let stage = protocol.commit_stage(protocol::SqliteCommitStageRequest {
				actor_id: "actor".to_string(),
				generation: 7,
				txid: 1,
				chunk_idx: 0,
				bytes: vec![1, 2, 3],
				is_last: true,
			});
			let ((), response) = tokio::join!(wait, stage);
			assert!(matches!(
				response.expect("stage response should succeed"),
				protocol::SqliteCommitStageResponse::SqliteCommitStageOk(_)
			));
		});
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
		assert!(!protocol.stage_requests().is_empty());
		assert!(
			protocol
				.stage_requests()
				.iter()
				.enumerate()
				.all(|(chunk_idx, request)| request.chunk_idx as usize == chunk_idx)
		);
		assert!(
			protocol
				.stage_requests()
				.last()
				.is_some_and(|request| request.is_last)
		);
		assert_eq!(protocol.awaited_stage_responses(), 0);
		assert_eq!(protocol.finalize_requests().len(), 1);
	}

	#[test]
	fn commit_buffered_pages_surfaces_finalize_stage_not_found() {
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
			protocol::SqliteCommitFinalizeResponse::SqliteStageNotFound(
				protocol::SqliteStageNotFound { stage_id: 99 },
			),
		));

		let protocol_for_release = Arc::clone(&protocol);
		let release = std::thread::spawn(move || {
			runtime.block_on(async {
				protocol_for_release.finalize_started.notified().await;
				protocol_for_release.release_finalize.notify_one();
			});
		});

		let err = Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("runtime should build")
			.block_on(commit_buffered_pages(
				&SqliteTransport::from_mock(Arc::clone(&protocol)),
				BufferedCommitRequest {
					actor_id: "actor".to_string(),
					generation: 7,
					expected_head_txid: 12,
					new_db_size_pages: 3,
					max_delta_bytes: 4096,
					max_pages_per_stage: 1,
					dirty_pages: dirty_pages(3, 9),
				},
			))
			.expect_err("stage-not-found finalize should fail");

		release.join().expect("release thread should finish");

		assert!(matches!(err, CommitBufferError::StageNotFound(99)));
	}

	#[test]
	fn vfs_records_commit_phase_durations() {
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let metrics = TestVfsMetrics::new();
		let db = harness.open_db_with_metrics(&runtime, Arc::clone(&metrics));

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE metrics_test (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
		)
		.expect("create table should succeed");

		metrics.reset();

		sqlite_exec(
			db.as_ptr(),
			"INSERT INTO metrics_test (id, value) VALUES (1, 'hello');",
		)
		.expect("insert should succeed");

		let snapshot = metrics.snapshot();
		assert_eq!(snapshot.commit_total, 1);
		assert!(snapshot.commit_request_build_ns_total > 0);
		assert!(snapshot.commit_serialize_ns_total > 0);
		assert!(snapshot.commit_transport_ns_total > 0);
		assert!(snapshot.commit_state_update_ns_total > 0);
		assert!(snapshot.commit_duration_ns_total >= snapshot.commit_request_build_ns_total);
		assert!(
			snapshot.commit_request_build_ns_total
				+ snapshot.commit_transport_ns_total
				+ snapshot.commit_state_update_ns_total
				> 0
		);
	}

	#[test]
	fn profile_large_tx_insert_5mb() {
		// 5MB = 1280 rows x 4KB blobs in one transaction
		let runtime = direct_runtime();
		let harness = DirectEngineHarness::new();
		let metrics = TestVfsMetrics::new();
		let db = harness.open_db_with_metrics(&runtime, Arc::clone(&metrics));

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE bench (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

		metrics.reset();

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

		let snapshot = metrics.snapshot();
		let resolve_total = snapshot.resolve_pages_total;
		let cache_hits = snapshot.resolve_pages_cache_hits_total;
		let fetches = snapshot.get_pages_total;
		let pages_fetched = snapshot.pages_fetched_total;
		let prefetch = snapshot.prefetch_pages_total;
		let commits = snapshot.commit_total;

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
		let metrics = TestVfsMetrics::new();
		let db = harness.open_db_with_metrics(&runtime, Arc::clone(&metrics));

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE counter (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);",
		)
		.expect("create");
		sqlite_exec(db.as_ptr(), "INSERT INTO counter VALUES (1, 0);").expect("insert");

		metrics.reset();

		let start = std::time::Instant::now();
		for _ in 0..100 {
			sqlite_exec(
				db.as_ptr(),
				"UPDATE counter SET value = value + 1 WHERE id = 1;",
			)
			.expect("update");
		}
		let elapsed = start.elapsed();

		let snapshot = metrics.snapshot();
		let fetches = snapshot.get_pages_total;
		let commits = snapshot.commit_total;

		eprintln!("=== 100 HOT ROW UPDATES (autocommit) ===");
		eprintln!("  wall clock:           {:?}", elapsed);
		eprintln!(
			"  resolve_pages calls:  {}",
			snapshot.resolve_pages_total
		);
		eprintln!(
			"  cache hits (pages):   {}",
			snapshot.resolve_pages_cache_hits_total
		);
		eprintln!("  engine fetches:       {}", fetches);
		eprintln!(
			"  pages fetched total:  {}",
			snapshot.pages_fetched_total
		);
		eprintln!(
			"  prefetch pages:       {}",
			snapshot.prefetch_pages_total
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
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsConfig::default());
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
			let metrics = TestVfsMetrics::new();
			let db2 = harness.open_db_on_engine_with_metrics(
				&runtime,
				engine.clone(),
				actor_id,
				VfsConfig::default(),
				Some(metrics.clone()),
			);

		// Warm the cache by reading everything
		sqlite_exec(db2.as_ptr(), "SELECT COUNT(*) FROM bench;").expect("count");

			metrics.reset();

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

			let snapshot = metrics.snapshot();
			let resolve_total = snapshot.resolve_pages_total;
			let cache_hits = snapshot.resolve_pages_cache_hits_total;
			let fetches = snapshot.get_pages_total;
			let pages_fetched = snapshot.pages_fetched_total;
			let prefetch = snapshot.prefetch_pages_total;
			let commits = snapshot.commit_total;

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
			let metrics = TestVfsMetrics::new();
			let db = harness.open_db_with_metrics(&runtime, Arc::clone(&metrics));

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE bench (id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
		)
		.expect("create table should succeed");

			metrics.reset();

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
			let snapshot = metrics.snapshot();
			let resolve_total = snapshot.resolve_pages_total;
			let cache_hits = snapshot.resolve_pages_cache_hits_total;
			let fetches = snapshot.get_pages_total;
			let pages_fetched = snapshot.pages_fetched_total;
			let prefetch = snapshot.prefetch_pages_total;
			let commits = snapshot.commit_total;

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
			let metrics = TestVfsMetrics::new();
			let db = harness.open_db_with_metrics(&runtime, Arc::clone(&metrics));

		sqlite_exec(
			db.as_ptr(),
			"CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER NOT NULL);",
		)
		.expect("create table should succeed");

			metrics.reset();

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

			let commits = metrics.snapshot().commit_total;
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
				VfsConfig::default(),
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
			harness.open_db_on_engine(&runtime, engine.clone(), &actor_a, VfsConfig::default());
		let db_b =
			harness.open_db_on_engine(&runtime, engine.clone(), &actor_b, VfsConfig::default());

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
			let db =
				harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsConfig::default());
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
			harness.open_db_on_engine(&runtime, engine.clone(), actor_id, VfsConfig::default());
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

	fn open_bench_db(runtime: &tokio::runtime::Runtime) -> NativeDatabase {
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

	#[test]
	fn bench_large_tx_insert_100mb() {
		large_tx_insert(100 * 1024 * 1024);
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
