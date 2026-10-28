//! Custom SQLite VFS backed by KV operations over the KV channel.
//!
//! This crate owns the KV-backed SQLite behavior used by `rivetkit-napi`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::page_buffer::{PageBudget, PageBuffer, PageTrace};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::{self, StreamExt};
use libsqlite3_sys::*;
use parking_lot::{Mutex, RwLock};
use rivet_envoy_protocol as protocol;
use sqlite_types::{PageBytes, is_head_fence_mismatch};
use tokio::runtime::Handle;
use tracing::Instrument;

use crate::optimization_flags::{
	SqliteOptimizationFlags, SqliteVfsPageCacheMode, sqlite_optimization_flags,
};
use crate::sqlite_page::{PageClass, classify};

const DEFAULT_MAX_PAGES_PER_STAGE: usize = 4_000;
const DEFAULT_PAGE_SIZE: usize = 4096;
const STAGE_PIPELINE_BYTE_BUDGET: usize = 8 * 1024 * 1024;
const STAGE_MAX_SEGMENT_BYTES: usize = sqlite_types::SHARD_SIZE as usize
	* sqlite_types::COMMIT_SEGMENT_MAX_SHARDS as usize
	* DEFAULT_PAGE_SIZE;
const STAGE_PIPELINE_DEPTH: usize = STAGE_PIPELINE_BYTE_BUDGET / STAGE_MAX_SEGMENT_BYTES;
const NATIVE_DATABASE_DROP_FLUSH_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_PATHNAME: c_int = 64;
const TEMP_AUX_PATH_PREFIX: &str = "__sqlite_temp__";
const SQLITE_HEADER_MAGIC: &[u8; 16] = b"SQLite format 3\0";
const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

static NEXT_TEMP_AUX_ID: AtomicU64 = AtomicU64::new(1);

unsafe extern "C" {
	fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
}

fn empty_db_page() -> Vec<u8> {
	let mut page = vec![0u8; DEFAULT_PAGE_SIZE];
	page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()].copy_from_slice(&EMPTY_DB_PAGE_HEADER_PREFIX);
	page
}

fn sqlite_header_page_size(page: &[u8]) -> Option<usize> {
	if page.len() < 100 || &page[..SQLITE_HEADER_MAGIC.len()] != SQLITE_HEADER_MAGIC {
		return None;
	}

	let raw = u16::from_be_bytes([page[16], page[17]]);
	let page_size = if raw == 1 { 65_536 } else { usize::from(raw) };

	if (512..=65_536).contains(&page_size) && page_size.is_power_of_two() {
		Some(page_size)
	} else {
		None
	}
}

fn sqlite_header_db_size_pages(page: &[u8]) -> Option<u32> {
	if page.len() < 100 || &page[..SQLITE_HEADER_MAGIC.len()] != SQLITE_HEADER_MAGIC {
		return None;
	}

	let db_size_pages = u32::from_be_bytes([page[28], page[29], page[30], page[31]]);
	Some(db_size_pages.max(1))
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
				tracing::error!(msg = panic_message(&panic), "sqlite callback panicked");
				$err_val
			}
		}
	};
}

#[async_trait]
pub trait SqliteTransport: Send + Sync {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse>;

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse>;

	/// Opens a staged commit, for a commit too large to send in one message.
	///
	/// Required rather than defaulted. A default made staging look optional, and the transport that
	/// runs `depot vacuum` in-process silently took it: every commit above the single-shot threshold
	/// failed there, which is exactly the commit shape a vacuum produces. A transport that genuinely
	/// cannot stage has to say so in its own body.
	async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse>;

	/// Stages one shard-aligned segment of an open staged commit.
	async fn commit_stage_segment(
		&self,
		request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse>;

	/// Publishes an open staged commit, making every staged segment visible at once.
	async fn commit_finalize(
		&self,
		request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse>;
}

pub type SqliteTransportHandle = Arc<dyn SqliteTransport>;
fn sqlite_now_ms() -> Result<i64> {
	use std::time::{SystemTime, UNIX_EPOCH};

	Ok(SystemTime::now()
		.duration_since(UNIX_EPOCH)?
		.as_millis()
		.try_into()?)
}

#[derive(Debug, Clone)]
pub struct VfsConfig {
	/// The process-wide budget this database's page buffers share. A database opened on a
	/// worker pool uses the pool's budget.
	pub page_budget: Arc<PageBudget>,
	pub page_cache_mode: SqliteVfsPageCacheMode,
	pub staging_cache_ttl_ms: u64,
	pub max_pages_per_stage: usize,
	/// Idle time after which the worker releases this database's caches. Zero disables.
	pub idle_release_ms: u64,
	/// The most pages one transaction may change. Values above Depot's own commit cap,
	/// `sqlite_types::MAX_COMMIT_DIRTY_PAGES`, are clamped to it.
	pub max_commit_dirty_pages: usize,
	
}

impl Default for VfsConfig {
	fn default() -> Self {
		Self::from_optimization_flags(*sqlite_optimization_flags())
	}
}

impl VfsConfig {
	/// The most pages one transaction may change, never above Depot's commit cap.
	pub fn commit_page_cap(&self) -> usize {
		self.max_commit_dirty_pages
			.min(sqlite_types::MAX_COMMIT_DIRTY_PAGES)
	}

	pub fn from_optimization_flags(flags: SqliteOptimizationFlags) -> Self {
		let caches_pages = flags.vfs_page_cache_mode.caches_any_pages();
		Self {
			page_budget: PageBudget::new(
				if caches_pages {
					flags.vfs_page_buffer_process_pages
				} else {
					0
				},
				if caches_pages {
					flags.vfs_staging_cache_ttl_ms
				} else {
					0
				},
				None,
			),
			page_cache_mode: flags.vfs_page_cache_mode,
			staging_cache_ttl_ms: if caches_pages {
				flags.vfs_staging_cache_ttl_ms
			} else {
				0
			},
			max_pages_per_stage: DEFAULT_MAX_PAGES_PER_STAGE,
			idle_release_ms: flags.idle_release_ms,
			max_commit_dirty_pages: sqlite_types::MAX_COMMIT_DIRTY_PAGES,
			
		}
	}
}

#[derive(Debug, Clone, Default)]
pub(crate) struct InitialPages {
	pub pages: Vec<(u32, PageBytes)>,
	pub head_txid: Option<u64>,
	pub requested_page_count: u32,
}

impl From<Vec<(u32, Vec<u8>)>> for InitialPages {
	fn from(pages: Vec<(u32, Vec<u8>)>) -> Self {
		let requested_page_count = pages.len().try_into().unwrap_or(u32::MAX);
		Self {
			pages: pages
				.into_iter()
				.map(|(pgno, bytes)| (pgno, bytes.into()))
				.collect(),
			head_txid: None,
			requested_page_count,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitPath {
	Fast,
	Slow,
	/// Written as several staged segments plus a finalize, because the commit was too large to send
	/// in one message.
	Staged,
}

#[derive(Debug, Clone)]
pub struct BufferedCommitRequest {
	pub actor_id: String,
	pub new_db_size_pages: u32,
	pub dirty_pages: Vec<BufferedDirtyPage>,
	pub expected_head_txid: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct BufferedDirtyPage {
	pub pgno: u32,
	pub bytes: PageBytes,
}

#[derive(Debug, Clone)]
pub struct BufferedCommitOutcome {
	pub path: CommitPath,
	pub db_size_pages: u32,
	pub head_txid: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitBufferError {
	FenceMismatch(String),
	Indeterminate(String),
	/// Storage refused the commit before running it, or its request timed out before it was
	/// sent. Nothing was committed, so the transaction rolls back and may run again.
	Storage(SqliteStorageError),
	Other(String),
}

/// A SQLite storage request that failed without changing anything, in a way the caller can act
/// on. The VFS reports it to SQLite as an I/O error, and the statement's command returns it
/// instead of SQLite's generic `disk I/O error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqliteStorageError {
	/// Storage refused the request before running it, as when the volume's writes are ahead of
	/// its tablet's apply. The request may be sent again after `retry_after_ms`.
	Overloaded {
		retry_after_ms: u64,
		message: String,
	},
	/// The request's deadline passed before storage answered a read, or before a write was
	/// sent. A write whose request was sent has an unknown outcome instead.
	Timeout { operation: String },
}

impl std::fmt::Display for SqliteStorageError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Overloaded {
				retry_after_ms,
				message,
			} => write!(
				f,
				"sqlite storage is overloaded, retry after {retry_after_ms} ms: {message}"
			),
			Self::Timeout { operation } => {
				write!(
					f,
					"sqlite storage {operation} timed out before storage answered"
				)
			}
		}
	}
}

impl std::error::Error for SqliteStorageError {}

/// A transport's report that a SQLite request's deadline passed before its response arrived,
/// for a read or for a write it never sent. A transport reports a sent write that timed out as
/// [`SqliteCommitIndeterminateError`] instead, since storage may still have committed it.
#[derive(Debug)]
pub struct SqliteStorageTimeoutError {
	pub operation: String,
}

impl std::fmt::Display for SqliteStorageTimeoutError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "sqlite {} request timed out", self.operation)
	}
}

impl std::error::Error for SqliteStorageTimeoutError {}

/// The group and code of a storage refusal that ran nothing and carries a retry hint.
const DEPOT_OVERLOADED_GROUP: &str = "depot";
const DEPOT_OVERLOADED_CODE: &str = "overloaded";

/// The hint a `depot.overloaded` response carries in its message, or this when it names none.
const DEFAULT_OVERLOADED_RETRY_AFTER_MS: u64 = 100;

/// The typed failure a storage error response stands for, if it is a refusal with a hint.
fn storage_refusal(error: &protocol::SqliteErrorResponse) -> Option<SqliteStorageError> {
	if error.group.as_str() != DEPOT_OVERLOADED_GROUP
		|| error.code.as_str() != DEPOT_OVERLOADED_CODE
	{
		return None;
	}
	let message = error.message.to_string();
	Some(SqliteStorageError::Overloaded {
		retry_after_ms: retry_after_hint(&message).unwrap_or(DEFAULT_OVERLOADED_RETRY_AFTER_MS),
		message,
	})
}

/// Reads the `retry_after_ms=<ms>` parameter Engine writes into a refusal's message.
fn retry_after_hint(message: &str) -> Option<u64> {
	let (_, rest) = message.split_once("retry_after_ms=")?;
	let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
	digits.parse().ok()
}

/// The typed failure a transport error stands for, if it is a timeout that changed nothing.
fn storage_timeout(error: &anyhow::Error) -> Option<SqliteStorageError> {
	error
		.downcast_ref::<SqliteStorageTimeoutError>()
		.map(|timeout| SqliteStorageError::Timeout {
			operation: timeout.operation.clone(),
		})
}

/// Marks a lost commit reply so the VFS can fail closed instead of using stale state.
#[derive(Debug)]
pub struct SqliteCommitIndeterminateError(pub String);

impl std::fmt::Display for SqliteCommitIndeterminateError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "sqlite {} result is indeterminate", self.0)
	}
}

impl std::error::Error for SqliteCommitIndeterminateError {}

/// A transaction tried to change more pages than one commit may carry.
///
/// The VFS refuses the write that would cross the cap, before any of the transaction reaches
/// Depot, and SQLite rolls the transaction back. The connection stays usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteTransactionTooLargeError {
	pub max_pages: usize,
	pub page_size: usize,
}

impl std::fmt::Display for SqliteTransactionTooLargeError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let max_kib = self.max_pages * self.page_size / 1024;
		let max_size = if max_kib >= 1024 {
			format!("{} MiB", max_kib / 1024)
		} else {
			format!("{max_kib} KiB")
		};
		write!(
			f,
			"sqlite transaction changes more than {} pages ({}), the most one commit can hold, \
			 so the statement failed and its changes were rolled back; split the work into smaller \
			 transactions. VACUUM rewrites the whole \
			 database in one transaction, so it only runs while the database's in-use pages fit under \
			 this limit",
			self.max_pages, max_size,
		)
	}
}

impl std::error::Error for SqliteTransactionTooLargeError {}

/// A file control opcode that reads this VFS's commit page cap into an `i64`. SQLite reserves
/// opcodes up to 100 for itself.
pub(crate) const FCNTL_COMMIT_PAGE_CAP: c_int = 0x5256_0001;

#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteVfsMetricsSnapshot {
	pub request_build_ns: u64,
	pub serialize_ns: u64,
	pub transport_ns: u64,
	pub state_update_ns: u64,
	pub total_ns: u64,
	pub commit_count: u64,
	pub page_cache_entries: u64,
	pub page_cache_weighted_size: u64,
	pub page_cache_capacity_pages: u64,
	pub write_buffer_dirty_pages: u64,
	pub db_size_pages: u64,
}

pub const MAX_PROFILED_GET_PAGES_REQUESTS: usize = 16;
pub const MAX_PROFILED_TRANSACTION_STATEMENTS: usize = 32;

/// Bounded details for one physical `get_pages` call made while a native SQLite
/// command is executing.
#[derive(Debug, Clone, Default)]
pub struct SqliteGetPagesProfile {
	pub ordinal: u64,
	pub duration_ns: u64,
	pub demand_requested: u64,
	pub response_present: u64,
	pub response_missing: u64,
	pub overflow_expansion_extra: u64,
	pub response_bytes: u64,
	pub success: bool,
}

/// Fixed-size, connection-local counters collected by VFS callbacks for one
/// native SQLite command. The worker installs and removes this context around
/// each command, so VFS activity cannot be attributed to a neighboring query.
#[derive(Debug, Clone, Default)]
pub struct SqliteOperationProfile {
	pub worker_wait_ns: u64,
	pub sqlite_execution_ns: u64,
	pub storage_ns: u64,
	pub bind_count: u64,
	pub bind_logical_bytes: u64,
	pub result_rows: u64,
	pub result_columns: u64,
	pub result_logical_bytes: u64,
	pub sqlite_requested_pages: u64,
	pub cache_hit_pages: u64,
	pub cache_miss_pages: u64,
	pub depot_demand_requested_pages: u64,
	pub response_present_pages: u64,
	pub response_missing_pages: u64,
	pub overflow_expansion_extra_pages: u64,
	pub btree_pages: u64,
	pub non_btree_pages: u64,
	pub prefetch_consumed_pages: u64,
	pub prefetch_unused_pages: u64,
	pub dirty_pages: u64,
	pub storage_response_bytes: u64,
	pub dirty_bytes: u64,
	pub get_pages_round_trips: u64,
	pub commit_round_trips: u64,
	pub get_pages_requests: [Option<SqliteGetPagesProfile>; MAX_PROFILED_GET_PAGES_REQUESTS],
	pub omitted_get_pages_requests: u64,
}

#[derive(Debug, Clone)]
pub struct SqliteOperationMetric {
	pub operation_type: &'static str,
	pub fingerprint: String,
	pub fingerprint_source: &'static str,
	pub transaction_mode: &'static str,
	pub storage_transport: &'static str,
	pub outcome: &'static str,
	pub sql_bytes: u64,
	pub total_ns: u64,
	pub transaction_wait_ns: u64,
	pub profile: SqliteOperationProfile,
}

#[derive(Debug, Clone)]
pub struct SqliteTransactionMetric {
	pub fingerprint: String,
	pub fingerprint_source: &'static str,
	pub shape_fingerprint: String,
	pub statement_fingerprint_hashes: [Option<[u8; 16]>; MAX_PROFILED_TRANSACTION_STATEMENTS],
	pub omitted_statement_fingerprints: u64,
	pub storage_transport: &'static str,
	pub outcome: &'static str,
	pub total_ns: u64,
	pub transaction_wait_ns: u64,
	pub worker_wait_ns: u64,
	pub storage_ns: u64,
	pub local_work_ns: u64,
	pub application_time_ns: u64,
	pub commit_ns: u64,
	pub get_pages_round_trips: u64,
	pub statement_count: u64,
	pub dirty_pages: u64,
	pub dirty_bytes: u64,
}

impl SqliteOperationProfile {
	fn push_get_pages(&mut self, request: SqliteGetPagesProfile, request_limit: usize) {
		self.storage_ns = self.storage_ns.saturating_add(request.duration_ns);
		self.get_pages_round_trips = self.get_pages_round_trips.saturating_add(1);
		self.depot_demand_requested_pages = self
			.depot_demand_requested_pages
			.saturating_add(request.demand_requested);
		self.response_present_pages = self
			.response_present_pages
			.saturating_add(request.response_present);
		self.response_missing_pages = self
			.response_missing_pages
			.saturating_add(request.response_missing);
		self.overflow_expansion_extra_pages = self
			.overflow_expansion_extra_pages
			.saturating_add(request.overflow_expansion_extra);
		self.storage_response_bytes = self
			.storage_response_bytes
			.saturating_add(request.response_bytes);
		let index = self.get_pages_round_trips.saturating_sub(1) as usize;
		if index < request_limit
			&& let Some(slot) = self.get_pages_requests.get_mut(index)
		{
			*slot = Some(request);
		} else {
			self.omitted_get_pages_requests = self.omitted_get_pages_requests.saturating_add(1);
		}
	}
}

pub(crate) struct SqliteOperationProfileGuard<'a> {
	ctx: &'a VfsContext,
	active: bool,
}

impl SqliteOperationProfileGuard<'_> {
	pub(crate) fn finish(mut self) -> SqliteOperationProfile {
		self.active = false;
		self.ctx.finish_operation_profile()
	}
}

impl Drop for SqliteOperationProfileGuard<'_> {
	fn drop(&mut self) {
		if self.active {
			let _ = self.ctx.finish_operation_profile();
		}
	}
}

/// Cumulative count of network round trips the VFS has issued to the engine.
///
/// `get_pages` counts `SqliteGetPagesRequest` fetches and `commit` counts
/// `SqliteCommitRequest` commits. Diffing two snapshots gives the round trips
/// performed by the work that ran between them, such as a SQLite transaction.
#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteRoundTripCounts {
	pub get_pages: u64,
	pub commit: u64,
}

impl SqliteRoundTripCounts {
	pub fn total(&self) -> u64 {
		self.get_pages.saturating_add(self.commit)
	}

	pub fn since(&self, earlier: SqliteRoundTripCounts) -> SqliteRoundTripCounts {
		SqliteRoundTripCounts {
			get_pages: self.get_pages.saturating_sub(earlier.get_pages),
			commit: self.commit.saturating_sub(earlier.commit),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum SqliteOpenPhase {
	InitialPreload,
	VfsRegister,
	WorkerReady,
	Total,
}

impl SqliteOpenPhase {
	pub fn as_label(self) -> &'static str {
		match self {
			SqliteOpenPhase::InitialPreload => "initial_preload",
			SqliteOpenPhase::VfsRegister => "vfs_register",
			SqliteOpenPhase::WorkerReady => "worker_ready",
			SqliteOpenPhase::Total => "total",
		}
	}
}

pub trait SqliteVfsMetrics: Send + Sync {
	fn profiling_enabled(&self) -> bool;

	fn max_profiled_get_pages_requests(&self) -> usize;

	/// Records an operation and returns whether its selected fingerprint was admitted.
	fn observe_operation_profile(&self, profile: &SqliteOperationMetric) -> bool;

	/// Records a transaction and returns whether its selected fingerprint was admitted.
	fn observe_transaction_profile(&self, profile: &SqliteTransactionMetric) -> bool;

	/// Selects a diagnostic before its caller computes SQL fingerprints.
	fn operation_diagnostic_selected(&self, _profile: &SqliteOperationMetric) -> bool {
		true
	}

	fn transaction_diagnostic_selected(&self, _profile: &SqliteTransactionMetric) -> bool {
		true
	}

	/// Emits an event already selected by operation_diagnostic_selected.
	fn emit_operation_diagnostic_event(
		&self,
		actor_id: &str,
		generation: Option<u64>,
		profile: &SqliteOperationMetric,
	);

	fn emit_transaction_diagnostic_event(
		&self,
		actor_id: &str,
		generation: Option<u64>,
		profile: &SqliteTransactionMetric,
	);

	fn record_fingerprint_catalog(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
		identity: &str,
		format_version: u8,
	);

	fn record_resolve_pages(&self, requested_pages: u64);

	fn record_resolve_cache_hits(&self, pages: u64);

	fn record_resolve_cache_misses(&self, pages: u64);

	fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64);

	fn observe_get_pages_duration(&self, duration_ns: u64);

	fn observe_open_phase(&self, phase: SqliteOpenPhase, outcome: &'static str, duration_ns: u64);

	fn record_startup_preload_pages(&self, kind: &'static str, pages: u64);

	/// Records one successful commit and the dirty pages it wrote.
	fn record_commit(&self, dirty_pages: u64);

	fn observe_commit_phases(
		&self,
		request_build_ns: u64,
		serialize_ns: u64,
		transport_ns: u64,
		state_update_ns: u64,
		total_ns: u64,
	);

	fn set_worker_queue_depth(&self, depth: u64);

	fn set_worker_active(&self, active: bool);

	fn set_worker_inflight(&self, active: bool);

	/// Reports the bytes this database's VFS page caches hold, after each command, after an
	/// idle release and as zero when the worker stops.
	fn set_vfs_cached_bytes(&self, bytes: u64);
	/// Prefetched pages discarded before their first use, including close and lazy TTL expiry.
	fn record_prefetch_unused_pages(&self, _pages: u64) {}

	fn set_coordinator_queue_depth(&self, depth: u64);

	fn record_worker_queue_overload(&self);

	fn observe_worker_command_duration(
		&self,
		operation: &'static str,
		in_tx: bool,
		stmt_kind: &'static str,
		duration_ns: u64,
	);

	fn observe_transaction_round_trips(&self, get_pages_round_trips: u64, commit_round_trips: u64);

	fn record_worker_command_error(&self, operation: &'static str, code: &'static str);

	fn observe_worker_close_duration(&self, duration_ns: u64);

	fn record_worker_close_timeout(&self);

	fn record_worker_crash(&self);

	fn record_worker_unclean_close(&self);
}

#[derive(Debug, Clone, Copy, Default)]
struct CommitTransportMetrics {
	serialize_ns: u64,
	transport_ns: u64,
}

enum CommitWait<T> {
	Completed(T),
	TimedOut,
}

pub struct VfsContext {
	actor_id: String,
	generation: Option<u64>,
	runtime: Handle,
	transport: SqliteTransportHandle,
	config: VfsConfig,
	state: RwLock<VfsState>,
	aux_files: RwLock<BTreeMap<String, Arc<AuxFileState>>>,
	last_error: Mutex<Option<String>>,
	indeterminate_result: Mutex<Option<String>>,
	transient_commit_error: Mutex<Option<String>>,
	/// Set when a write was refused for crossing the commit page cap, until the worker reports it.
	transaction_too_large: Mutex<Option<SqliteTransactionTooLargeError>>,
	/// The typed storage failure behind the I/O error SQLite last saw, for the command that
	/// ran the statement to return instead of SQLite's generic message.
	storage_error: Mutex<Option<SqliteStorageError>>,
	fatal_error: RwLock<Option<String>>,
	
	
	
	
	commit_atomic_count: AtomicU64,
	/// Diagnostic attempts, distinct even when a refused/timed-out commit did not advance commit_total.
	trace_commit_attempts: AtomicU64,
	io_methods: Box<sqlite3_io_methods>,
	// Performance counters
	pub resolve_pages_total: AtomicU64,
	pub resolve_pages_cache_hits: AtomicU64,
	pub resolve_pages_fetches: AtomicU64,
	pub pages_fetched_total: AtomicU64,
	pub prefetch_pages_total: AtomicU64,
	/// Pages SQLite read from what the server sent beyond the requested pages.
	pub prefetch_consumed_total: AtomicU64,
	pub commit_total: AtomicU64,
	pub commit_request_build_ns: AtomicU64,
	pub commit_serialize_ns: AtomicU64,
	pub commit_transport_ns: AtomicU64,
	pub commit_state_update_ns: AtomicU64,
	pub commit_duration_ns_total: AtomicU64,
	// SQLite invokes VFS callbacks synchronously, so this profiling context cannot
	// use an async mutex. The native worker releases each guard before returning
	// to async code.
	operation_profile_active: AtomicBool,
	operation_profile: Mutex<Option<SqliteOperationProfile>>,
	operation_prefetch_unused_start: AtomicU64,
	reported_prefetch_unused: AtomicU64,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
}

#[derive(Debug, Clone)]
struct VfsState {
	db_size_pages: u32,
	/// Size depot last acknowledged. `db_size_pages` moving away from this is a pending size change
	/// that has to reach depot even when no page bytes are dirty, because SQLite reports a shrink
	/// through `xTruncate` alone.
	committed_db_size_pages: u32,
	head_txid: Option<u64>,
	page_size: usize,
	/// Pages fetched ahead of SQLite's requests. A page leaves when SQLite reads it.
	prefetch_buffer: PageBuffer,
	/// Pages this database committed in the last `staging_cache_ttl_ms`, for re-reads after
	/// SQLite's pager evicts them. A page leaves when SQLite reads it back.
	committed_buffer: PageBuffer,
	prefetch_unused_total: Arc<AtomicU64>,
	write_buffer: WriteBuffer,
	/// Pages of transactions SQLite committed while commits are corked, not yet sent to depot.
	/// `Some` while corked: each commit adds its pages here instead of committing, and uncorking
	/// sends them all as one commit. Reads see these pages before any cache.
	corked: Option<BTreeMap<u32, PageBytes>>,
	dead: bool,
	/// Database size after the last transaction moved into the cork, which a commit of the cork
	/// alone carries.
	corked_db_size_pages: u32,
	/// Set when a main database write was refused for crossing the commit page cap or a commit
	/// of the write buffer failed, until SQLite's rollback commits or a new transaction starts.
	/// Until then the write buffer holds pages of a transaction SQLite is abandoning, which a
	/// close must discard, not commit.
	refused_write_pending: bool,
	/// Set by `NativeDatabase::Drop` once it has published every committed transaction, just
	/// before SQLite closes. Main file writes after that come from SQLite rolling back a
	/// transaction left open at close, which restores the state Depot and the cork already
	/// hold, so commits discard them instead of sending them. A `SqliteVfs` serves one
	/// connection for its lifetime, so the flag never outlives the connection that set it.
	closing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CorkOutcome {
	NotCorked,
	Corked,
	Overflow,
}

#[derive(Debug, Clone, Default)]
struct WriteBuffer {
	in_atomic_write: bool,
	saved_db_size: u32,
	dirty: BTreeMap<u32, PageBytes>,
}

impl WriteBuffer {
	fn snapshot(&self) -> Vec<BufferedDirtyPage> {
		self.dirty
			.iter()
			.map(|(pgno, bytes)| BufferedDirtyPage {
				pgno: *pgno,
				bytes: bytes.clone(),
			})
			.collect()
	}
}

#[derive(Debug)]
enum GetPagesError {
	FenceMismatch(String),
	Storage(SqliteStorageError),
	Other(String),
}

#[repr(C)]
struct VfsFile {
	base: sqlite3_file,
	ctx: *const VfsContext,
	aux: *mut AuxFileHandle,
}

#[derive(Default)]
struct AuxFileState {
	bytes: Mutex<Vec<u8>>,
	/// Page records written to this file since it was last emptied, counted only for the main
	/// database's rollback journal.
	journal_records: AtomicUsize,
}

struct AuxFileHandle {
	path: String,
	state: Arc<AuxFileState>,
	delete_on_close: bool,
	/// Whether this is the main database's rollback journal, whose size the commit cap bounds.
	is_main_journal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageCacheInsertKind {
	Target,
	PartialTarget,
	Prefetch,
	Startup,
}

unsafe impl Send for VfsContext {}
unsafe impl Sync for VfsContext {}

pub struct SqliteVfs {
	_registration: SqliteVfsRegistration,
	_name: CString,
	ctx: Box<VfsContext>,
}

unsafe impl Send for SqliteVfs {}
unsafe impl Sync for SqliteVfs {}

struct SqliteVfsRegistration {
	vfs_ptr: *mut sqlite3_vfs,
}

pub struct NativeDatabase {
	db: *mut sqlite3,
	_vfs: NativeVfsHandle,
}

unsafe impl Send for NativeDatabase {}

pub type NativeVfsHandle = Arc<SqliteVfs>;
pub type NativeConnection = NativeDatabase;

impl VfsState {
	/// The database size every committed transaction left, including corked ones.
	fn committed_size_pages_before_transaction(&self) -> u32 {
		if self.corked.is_some() {
			self.corked_db_size_pages
		} else {
			self.committed_db_size_pages
		}
	}

	/// Drops the write buffer of a transaction SQLite has not committed and restores the size
	/// from before it. Depot and the cork already hold every committed transaction: each
	/// commit syncs the main file (connections cannot turn `synchronous` off), and the sync
	/// sends or corks its pages, so the write buffer holds only the unfinished transaction.
	fn discard_uncommitted_pages(&mut self) {
		self.write_buffer.dirty.clear();
		self.write_buffer.in_atomic_write = false;
		self.db_size_pages = self.committed_size_pages_before_transaction();
		self.refused_write_pending = false;
	}

	fn new(config: &VfsConfig, _profiling_enabled: bool) -> Self {
		let prefetch_unused_total = Arc::new(AtomicU64::new(0));
		let prefetch_buffer =
			PageBuffer::with_unused_counter(&config.page_budget, prefetch_unused_total.clone());
		let committed_buffer = build_page_buffer(config);
		let mut state = Self {
			db_size_pages: 1,
			committed_db_size_pages: 1,
			head_txid: None,
			page_size: DEFAULT_PAGE_SIZE,
			prefetch_buffer,
			committed_buffer,
			prefetch_unused_total,
			write_buffer: WriteBuffer::default(),
			corked: None,
			dead: false,
			corked_db_size_pages: 0,
			closing: false,
			refused_write_pending: false,
		};
		state.cache_page(
			config,
			PageCacheInsertKind::Startup,
			1,
			empty_db_page().into(),
		);
		state
	}

	fn cache_page(
		&mut self,
		config: &VfsConfig,
		kind: PageCacheInsertKind,
		pgno: u32,
		bytes: PageBytes,
	) {
		if !should_cache_page(config, kind, pgno) {
			return;
		}
		cache_page(config, &self.prefetch_buffer, kind, pgno, bytes);
	}

	/// Hands a buffered page to SQLite: the page leaves the buffer, because SQLite's pager owns
	/// every page it has read. Page 1 stays, because SQLite also reads its header directly,
	/// outside the pager. So does a page SQLite reads only part of (`partial`), such as the
	/// last page of an overflow chain read directly, bypassing the pager: SQLite reads that page
	/// again whole through the pager.
	fn take_buffered_page(
		&self,
		config: &VfsConfig,
		pgno: u32,
		partial: bool,
	) -> Option<(PageBytes, bool)> {
		if !can_read_cached_page(config, pgno) {
			return None;
		}
		if pgno == 1 || partial {
			let bytes = self
				.committed_buffer
				.get(&pgno)
				.or_else(|| self.prefetch_buffer.get(&pgno))?;
			return Some((bytes, false));
		}

		// Dropping the page here relies on SQLite 3.46.0 as bundled by libsqlite3-sys 0.30.1; line
		// numbers are in that crate's `sqlite3/sqlite3.c` amalgamation.
		// With `SQLITE_DIRECT_OVERFLOW_READ=0` (set in `.cargo/config.toml`), `journal_mode =
		// DELETE` (so no WAL reads of the database file) and no memory map (`iVersion` 1 has no
		// `xFetch`), the only whole-page `xRead` of the main database is `readDbPage`
		// (sqlite3.c:60138, pager.c), which reads
		// into a page-cache slot that the pager holds pinned while it is in use. The other
		// main-database reads are partial and only touch page 1: the 100-byte header in
		// `sqlite3PagerReadFileheader` (sqlite3.c:60994) and the 16-byte change counter in
		// `pagerSharedLock` (sqlite3.c:62490). The direct overflow read in `accessPayload`
		// (sqlite3.c:75763) would read every overflow page but the last of a chain whole, past
		// the cache, but that flag compiles it out.
		//
		// SQLite does not keep the page forever. Once unpinned, it leaves the cache when the
		// process-wide cache in `pager_cache.rs` evicts it under its byte budget (`PRAGMA
		// cache_size` does not bound it, since `x_cachesize` ignores it), on `xShrink` from
		// `sqlite3_db_release_memory`, and on `pager_reset`: an I/O error at unlock
		// (sqlite3.c:59012), a page size change (60888), close (61281), VACUUM's
		// `sqlite3PagerClearCache` (64577) or hot-journal playback (60009). The change counter
		// check that resets the cache when another connection wrote (62499) does not run here,
		// because `locking_mode = EXCLUSIVE` keeps the pager in its read state between
		// transactions, and the VFS serves exactly one connection per database. A page that
		// left both caches costs one depot fetch on its next read and never returns wrong data,
		// because each buffered page is a copy depot already holds: fetched from it, or written
		// by a commit it acknowledged.
		if let Some(bytes) = self.committed_buffer.take(&pgno) {
			// A neighboring miss may have prefetched this page after its commit.
			// The authoritative committed copy wins; its redundant speculative copy
			// was not consumed and must leave the buffer (and count as unused).
			self.prefetch_buffer.invalidate(&pgno);
			Some((bytes, false))
		} else {
			self.prefetch_buffer.take_with_status(&pgno)
		}
	}

	

	/// A page a corked commit holds, which no cache may stand in for.
	fn corked_page(&self, pgno: u32) -> Option<&PageBytes> {
		self.corked.as_ref().and_then(|corked| corked.get(&pgno))
	}

	/// While corked, moves the finished transaction's pages into the cork instead of
	/// committing them.
	///
	/// The cork is sent as one commit, so it holds at most `max_pages` pages. When the
	/// transaction would take it past that, nothing moves and the result is `Overflow`: the
	/// caller commits the cork on its own first, then corks the transaction.
	fn cork_dirty_pages(&mut self, max_pages: usize) -> CorkOutcome {
		let Some(corked) = self.corked.as_mut() else {
			return CorkOutcome::NotCorked;
		};
		let added_pages = self
			.write_buffer
			.dirty
			.keys()
			.filter(|pgno| !corked.contains_key(pgno))
			.count();
		if corked.len() + added_pages > max_pages {
			return CorkOutcome::Overflow;
		}
		corked.append(&mut self.write_buffer.dirty);
		self.corked_db_size_pages = self.db_size_pages;
		CorkOutcome::Corked
	}

	/// Keeps a committed page for a re-read after SQLite's pager evicts it, including under another
	/// connection's pressure on the process-wide pager cache, and drops any older prefetched copy. SQLite's pager holds the page right after the commit, so read-ahead
	/// skips it.
	fn note_committed_page(&mut self, config: &VfsConfig, pgno: u32, bytes: &PageBytes) {
		self.prefetch_buffer.invalidate(&pgno);
		if pgno != 1 {}
		if config.staging_cache_ttl_ms == 0 || !config.page_cache_mode.caches_any_pages() {
			return;
		}
		self.committed_buffer.insert(pgno, bytes.clone());
	}

	fn seed_page(
		&mut self,
		config: &VfsConfig,
		kind: PageCacheInsertKind,
		pgno: u32,
		page: PageBytes,
	) {
		if pgno == 1 {
			self.seed_main_page(config, kind, page);
		} else {
			self.cache_page(config, kind, pgno, page);
		}
	}

	fn seed_main_page(&mut self, config: &VfsConfig, kind: PageCacheInsertKind, page: PageBytes) {
		if let Some(page_size) = sqlite_header_page_size(&page) {
			self.page_size = page_size;
		}
		if let Some(db_size_pages) = sqlite_header_db_size_pages(&page) {
			self.db_size_pages = db_size_pages;
			self.committed_db_size_pages = db_size_pages;
		}
		self.cache_page(config, kind, 1, page);
	}

	fn invalidate_page_buffers(&mut self) {
		self.prefetch_buffer.invalidate_all();
		self.committed_buffer.invalidate_all();
	}
}

fn build_page_buffer(config: &VfsConfig) -> PageBuffer {
	PageBuffer::new(&config.page_budget)
}

fn cache_page(
	config: &VfsConfig,
	prefetch_buffer: &PageBuffer,
	kind: PageCacheInsertKind,
	pgno: u32,
	bytes: PageBytes,
) {
	if !should_cache_page(config, kind, pgno) {
		return;
	}
	if kind == PageCacheInsertKind::Prefetch || (kind == PageCacheInsertKind::Startup && pgno != 1)
	{
		prefetch_buffer.insert_prefetched(pgno, bytes);
	} else {
		prefetch_buffer.insert(pgno, bytes);
	}
}

fn should_cache_page(config: &VfsConfig, kind: PageCacheInsertKind, pgno: u32) -> bool {
	match kind {
		PageCacheInsertKind::Target => false,
		PageCacheInsertKind::PartialTarget => true,
		PageCacheInsertKind::Prefetch => {
			config.staging_cache_ttl_ms > 0 && config.page_cache_mode.caches_prefetched_pages()
		}
		PageCacheInsertKind::Startup => {
			pgno == 1
				|| (config.staging_cache_ttl_ms > 0
					&& config.page_cache_mode.caches_startup_preloaded_pages())
		}
	}
}

fn can_read_cached_page(config: &VfsConfig, pgno: u32) -> bool {
	pgno == 1 || config.page_cache_mode.caches_any_pages()
}

impl VfsContext {
	fn new(
		actor_id: String,
		generation: Option<u64>,
		runtime: Handle,
		transport: SqliteTransportHandle,
		config: VfsConfig,
		io_methods: sqlite3_io_methods,
		initial_pages: impl Into<InitialPages>,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> std::result::Result<Self, String> {
		let profiling_enabled = metrics
			.as_ref()
			.is_some_and(|metrics| metrics.profiling_enabled());
		let mut state = VfsState::new(&config, profiling_enabled);
		let initial_pages = initial_pages.into();
		state.head_txid = initial_pages.head_txid;
		for (pgno, page) in initial_pages.pages {
			state.seed_page(&config, PageCacheInsertKind::Startup, pgno, page);
		}

		Ok(Self {
			actor_id,
			generation,
			runtime,
			transport,
			config: config.clone(),
			state: RwLock::new(state),
			aux_files: RwLock::new(BTreeMap::new()),
			last_error: Mutex::new(None),
			indeterminate_result: Mutex::new(None),
			transient_commit_error: Mutex::new(None),
			transaction_too_large: Mutex::new(None),
			storage_error: Mutex::new(None),
			fatal_error: RwLock::new(None),
			
			
			
			
			commit_atomic_count: AtomicU64::new(0),
			trace_commit_attempts: AtomicU64::new(0),
			io_methods: Box::new(io_methods),
			resolve_pages_total: AtomicU64::new(0),
			resolve_pages_cache_hits: AtomicU64::new(0),
			resolve_pages_fetches: AtomicU64::new(0),
			pages_fetched_total: AtomicU64::new(0),
			prefetch_pages_total: AtomicU64::new(0),
			prefetch_consumed_total: AtomicU64::new(0),
			commit_total: AtomicU64::new(0),
			commit_request_build_ns: AtomicU64::new(0),
			commit_serialize_ns: AtomicU64::new(0),
			commit_transport_ns: AtomicU64::new(0),
			commit_state_update_ns: AtomicU64::new(0),
			commit_duration_ns_total: AtomicU64::new(0),
			operation_profile_active: AtomicBool::new(false),
			operation_profile: Mutex::new(None),
			operation_prefetch_unused_start: AtomicU64::new(0),
			reported_prefetch_unused: AtomicU64::new(0),
			metrics,
		})
	}

	fn begin_operation_profile(&self) {
		if self
			.metrics
			.as_ref()
			.is_some_and(|metrics| metrics.profiling_enabled())
		{
			let unused = self
				.state
				.read()
				.prefetch_unused_total
				.load(Ordering::Relaxed);
			self.operation_prefetch_unused_start
				.store(unused, Ordering::Relaxed);
			*self.operation_profile.lock() = Some(SqliteOperationProfile::default());
			self.operation_profile_active.store(true, Ordering::Release);
		}
	}

	fn report_prefetch_unused(&self) {
		let total = self
			.state
			.read()
			.prefetch_unused_total
			.load(Ordering::Relaxed);
		let previous = self
			.reported_prefetch_unused
			.fetch_max(total, Ordering::Relaxed);
		if total > previous
			&& let Some(metrics) = &self.metrics
		{
			metrics.record_prefetch_unused_pages(total - previous);
		}
	}

	fn finish_operation_profile(&self) -> SqliteOperationProfile {
		self.report_prefetch_unused();
		if !self.operation_profile_active.swap(false, Ordering::AcqRel) {
			return SqliteOperationProfile::default();
		}
		let mut profile = self.operation_profile.lock().take().unwrap_or_default();
		let unused = self
			.state
			.read()
			.prefetch_unused_total
			.load(Ordering::Relaxed)
			.saturating_sub(self.operation_prefetch_unused_start.load(Ordering::Relaxed));
		profile.prefetch_unused_pages = unused;
		profile
	}

	fn update_operation_profile(&self, update: impl FnOnce(&mut SqliteOperationProfile)) {
		if !self.operation_profile_active.load(Ordering::Relaxed) {
			return;
		}
		if let Some(profile) = self.operation_profile.lock().as_mut() {
			update(profile);
		}
	}

	fn profile_request_limit(&self) -> usize {
		self.metrics
			.as_ref()
			.map_or(0, |metrics| metrics.max_profiled_get_pages_requests())
			.min(MAX_PROFILED_GET_PAGES_REQUESTS)
	}

	fn clear_last_error(&self) {
		*self.last_error.lock() = None;
	}

	fn set_last_error(&self, message: String) {
		*self.last_error.lock() = Some(message);
	}

	fn mark_indeterminate(&self, operation: String) {
		*self.indeterminate_result.lock() = Some(operation.clone());
		self.mark_fatal(SqliteCommitIndeterminateError(operation).to_string());
	}

	fn take_indeterminate_result(&self) -> Option<String> {
		self.indeterminate_result.lock().take()
	}

	fn clone_last_error(&self) -> Option<String> {
		self.last_error.lock().clone()
	}

	fn clone_fatal_error(&self) -> Option<String> {
		self.fatal_error.read().clone()
	}

	fn defer_transient_commit_error(&self, message: String) {
		*self.transient_commit_error.lock() = Some(message);
	}

	fn take_transient_commit_error(&self) -> Option<String> {
		self.transient_commit_error.lock().take()
	}

	fn transaction_too_large_error(&self) -> SqliteTransactionTooLargeError {
		SqliteTransactionTooLargeError {
			max_pages: self.config.commit_page_cap(),
			page_size: self.page_size(),
		}
	}

	/// Refuses a write that would take the transaction past the commit page cap. SQLite treats
	/// `SQLITE_FULL` like a full disk: it fails the statement and rolls back, restoring the
	/// pages from its journal, so nothing over the cap is ever held or sent.
	fn refuse_oversize_transaction(&self) -> c_int {
		let error = self.transaction_too_large_error();
		tracing::warn!(
			actor_id = %self.actor_id,
			max_pages = error.max_pages,
			"sqlite transaction exceeds the commit page cap; the application must split it into smaller transactions"
		);
		self.set_last_error(error.to_string());
		*self.transaction_too_large.lock() = Some(error);
		SQLITE_FULL
	}

	/// A commit refused for crossing the cap. The write cap keeps every transaction under it,
	/// so this is only a backstop.
	fn transaction_too_large_commit_error(&self) -> CommitBufferError {
		CommitBufferError::Other(self.transaction_too_large_error().to_string())
	}

	/// Commits the transactions held in the cork as one commit and leaves the cork empty, for
	/// when the next transaction would take the cork past one commit. Corked transactions were
	/// never promised to be atomic together, only to become durable, and each commit still
	/// holds whole transactions.
	fn commit_cork_alone(&self) -> std::result::Result<(), CommitBufferError> {
		let request = {
			let state = self.state.read();
			let Some(corked) = state.corked.as_ref() else {
				return Ok(());
			};
			BufferedCommitRequest {
				actor_id: self.actor_id.clone(),
				new_db_size_pages: state.corked_db_size_pages,
				expected_head_txid: state.head_txid,
				dirty_pages: corked
					.iter()
					.map(|(pgno, bytes)| BufferedDirtyPage {
						pgno: *pgno,
						bytes: bytes.clone(),
					})
					.collect(),
			}
		};

		let outcome = match self.block_on_buffered_commit(request.clone(), None) {
			Ok(CommitWait::Completed((outcome, _))) => outcome,
			Ok(CommitWait::TimedOut) => {
				return Err(CommitBufferError::Other(
					"sqlite commit timed out".to_string(),
				));
			}
			Err(err) => {
				tracing::error!(
					actor_id = %self.actor_id,
					dirty_pages = request.dirty_pages.len(),
					?err,
					"sqlite commit of a full cork failed"
				);
				handle_non_finalize_commit_error(self, &err);
				return Err(err);
			}
		};
		self.commit_total.fetch_add(1, Ordering::Relaxed);
		if let Some(metrics) = &self.metrics {
			metrics.record_commit(request.dirty_pages.len() as u64);
		}

		let mut state = self.state.write();
		state.committed_db_size_pages = outcome.db_size_pages;
		state.head_txid = outcome
			.head_txid
			.or_else(|| state.head_txid.map(|head_txid| head_txid.saturating_add(1)));
		for dirty_page in &request.dirty_pages {
			state.note_committed_page(&self.config, dirty_page.pgno, &dirty_page.bytes);
		}
		if let Some(corked) = state.corked.as_mut() {
			corked.clear();
		}
		Ok(())
	}

	pub(crate) fn take_transaction_too_large(&self) -> Option<SqliteTransactionTooLargeError> {
		self.transaction_too_large.lock().take()
	}

	/// Records a typed storage failure SQLite is about to see as an I/O error.
	fn set_storage_error(&self, error: SqliteStorageError) {
		self.set_last_error(error.to_string());
		*self.storage_error.lock() = Some(error);
	}

	pub(crate) fn take_storage_error(&self) -> Option<SqliteStorageError> {
		self.storage_error.lock().take()
	}

	/// Discards the pages of a transaction whose write was refused, when the database closes
	/// before SQLite has rolled it back. Depot and the cork still hold the state from before
	/// that transaction, which is exactly what the rollback would restore.
	fn discard_refused_transaction(&self) {
		let mut state = self.state.write();
		if !state.refused_write_pending {
			return;
		}
		tracing::debug!(
			actor_id = %self.actor_id,
			dirty_pages = state.write_buffer.dirty.len(),
			"discarding the pages of an abandoned sqlite transaction at close"
		);
		state.discard_uncommitted_pages();
	}

	/// Marks the write buffer as holding a transaction whose commit failed. SQLite reports the
	/// failure and rolls the transaction back on its next access, which rewrites the journaled
	/// pages and commits them; a close before that access must discard the buffer instead.
	fn abandon_write_buffer(&self) {
		self.state.write().refused_write_pending = true;
	}

	/// Discards the pages of a transaction still open when the database closes. SQLite never
	/// committed it, so none of its pages may reach Depot.
	fn discard_open_transaction(&self) {
		let mut state = self.state.write();
		if state.write_buffer.dirty.is_empty()
			&& !state.write_buffer.in_atomic_write
			&& state.db_size_pages == state.committed_size_pages_before_transaction()
		{
			return;
		}
		tracing::debug!(
			actor_id = %self.actor_id,
			dirty_pages = state.write_buffer.dirty.len(),
			"discarding the pages of an uncommitted sqlite transaction at close"
		);
		state.discard_uncommitted_pages();
	}

	/// Drops every page a closing connection failed to send, including the commits SQLite
	/// completed while corked. Those commits are lost for this connection, so the close no
	/// longer holds them and the next open reads whatever Depot accepted. Returns how many
	/// pages were dropped.
	fn discard_unsent_commits(&self) -> usize {
		let mut state = self.state.write();
		let dirty_pages = state.write_buffer.dirty.len();
		state.discard_uncommitted_pages();
		dirty_pages
	}

	

	pub(crate) fn take_last_error(&self) -> Option<String> {
		self.last_error.lock().take()
	}

	fn add_commit_phase_metrics(
		&self,
		request_build_ns: u64,
		transport_metrics: CommitTransportMetrics,
		state_update_ns: u64,
		total_ns: u64,
	) {
		self.update_operation_profile(|profile| {
			profile.storage_ns = profile
				.storage_ns
				.saturating_add(transport_metrics.transport_ns);
			profile.commit_round_trips = profile.commit_round_trips.saturating_add(1);
		});
		if let Some(metrics) = &self.metrics {
			metrics.observe_commit_phases(
				request_build_ns,
				transport_metrics.serialize_ns,
				transport_metrics.transport_ns,
				state_update_ns,
				total_ns,
			);
		}
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
		let state = self.state.read();

		SqliteVfsMetricsSnapshot {
			request_build_ns: self.commit_request_build_ns.load(Ordering::Relaxed),
			serialize_ns: self.commit_serialize_ns.load(Ordering::Relaxed),
			transport_ns: self.commit_transport_ns.load(Ordering::Relaxed),
			state_update_ns: self.commit_state_update_ns.load(Ordering::Relaxed),
			total_ns: self.commit_duration_ns_total.load(Ordering::Relaxed),
			commit_count: self.commit_total.load(Ordering::Relaxed),
			page_cache_entries: state
				.prefetch_buffer
				.entry_count()
				.saturating_add(state.committed_buffer.entry_count()),
			page_cache_weighted_size: state.prefetch_buffer.weighted_size(),
			page_cache_capacity_pages: self.config.page_budget.capacity_pages(),
			write_buffer_dirty_pages: state.write_buffer.dirty.len() as u64,
			db_size_pages: state.db_size_pages as u64,
		}
	}

	fn block_on_buffered_commit(
		&self,
		request: BufferedCommitRequest,
		timeout: Option<Duration>,
	) -> std::result::Result<
		CommitWait<(BufferedCommitOutcome, CommitTransportMetrics)>,
		CommitBufferError,
	> {
		let trace_pages =
			tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE).then(|| {
				(
					request
						.dirty_pages
						.iter()
						.map(|page| page.pgno)
						.collect::<Vec<_>>(),
					request.new_db_size_pages,
				)
			});
		let attempt = trace_pages
			.as_ref()
			.map(|_| self.trace_commit_attempts.fetch_add(1, Ordering::Relaxed));
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "native_sqlite_commit",
            stream_id = %format_args!("sqlite-{:p}", self), actor_id = %self.actor_id,
            generation = self.generation, commit_attempt = attempt,
            local_request_id = %format_args!("c{}", self.commit_total.load(Ordering::Relaxed)));
		let commit =
			commit_buffered_pages(&*self.transport, request, self.config.commit_page_cap())
				.instrument(span.clone());
		let result = if let Some(timeout) = timeout {
			match self
				.runtime
				.block_on(async { tokio::time::timeout(timeout, commit).await })
			{
				Ok(result) => result,
				Err(_) => {
					span.in_scope(|| tracing::trace!(target: "sqlite_prefetch::trace", event = "commit_result", outcome = "timeout"));
					return Ok(CommitWait::TimedOut);
				}
			}
		} else {
			self.runtime.block_on(commit)
		};

		let _span = span.enter();
		tracing::trace!(target: "sqlite_prefetch::trace", event = "commit_result",
            outcome = if result.is_ok() { "acknowledged" } else { "error" });

		if result.is_ok()
			&& let Some((pages, size_pages)) = trace_pages
		{
			let request_id = self.commit_total.load(Ordering::Relaxed);
			for pgno in pages {
				tracing::trace!(target: "sqlite_prefetch::trace", event = "access", op = "write",
                    stream_id = %format_args!("sqlite-{:p}", self),
                    pgno, hit = false, request_id = %format_args!("c{request_id}"));
			}
			tracing::trace!(target: "sqlite_prefetch::trace", event = "commit",
                stream_id = %format_args!("sqlite-{:p}", self),
                request_id = %format_args!("c{request_id}"), size_pages);
		}
		result.map(CommitWait::Completed)
	}

	fn round_trip_counts(&self) -> SqliteRoundTripCounts {
		SqliteRoundTripCounts {
			get_pages: self.resolve_pages_fetches.load(Ordering::Relaxed),
			commit: self.commit_total.load(Ordering::Relaxed),
		}
	}

	fn page_size(&self) -> usize {
		self.state.read().page_size.max(DEFAULT_PAGE_SIZE)
	}

	fn open_aux_file(&self, path: &str) -> Arc<AuxFileState> {
		let mut aux_files = self.aux_files.write();
		aux_files
			.entry(path.to_string())
			.or_insert_with(|| Arc::new(AuxFileState::default()))
			.clone()
	}

	fn aux_file_exists(&self, path: &str) -> bool {
		self.aux_files.read().contains_key(path)
	}

	fn read_aux_file(&self, path: &str) -> Option<Vec<u8>> {
		let state = self.aux_files.read().get(path).cloned()?;
		let bytes = state.bytes.lock().clone();
		Some(bytes)
	}

	fn delete_aux_file(&self, path: &str) {
		self.aux_files.write().remove(path);
	}

	

	

	

	

	fn is_dead(&self) -> bool {
		self.state.read().dead
	}

	fn mark_fatal(&self, message: String) {
		self.set_last_error(message.clone());
		self.state.write().dead = true;
		let mut fatal_error = self.fatal_error.write();
		if fatal_error.is_none() {
			*fatal_error = Some(message);
		}
	}

	/// Resolves `target_pgnos` from the buffers or the transport. `partial` marks a read of
	/// less than a whole page, which leaves buffered pages in place; see `take_buffered_page`.
	fn resolve_pages(
		&self,
		target_pgnos: &[u32],
		partial: bool,
	) -> std::result::Result<HashMap<u32, Option<PageBytes>>, GetPagesError> {
		use std::sync::atomic::Ordering::Relaxed;
		let request_id = self.resolve_pages_total.fetch_add(1, Relaxed);
		self.update_operation_profile(|profile| {
			profile.sqlite_requested_pages = profile
				.sqlite_requested_pages
				.saturating_add(target_pgnos.len() as u64);
		});
		if let Some(metrics) = &self.metrics {
			metrics.record_resolve_pages(target_pgnos.len() as u64);
		}

		let mut resolved = HashMap::new();
		let mut missing = Vec::new();
		let mut seen = HashSet::new();
		let mut prefetch_consumed = 0_u64;

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
				if let Some(bytes) = state.corked_page(pgno) {
					resolved.insert(pgno, Some(bytes.clone()));
					continue;
				}
				if let Some((bytes, was_prefetched)) =
					state.take_buffered_page(&self.config, pgno, partial)
				{
					prefetch_consumed = prefetch_consumed.saturating_add(u64::from(was_prefetched));
					resolved.insert(pgno, Some(bytes));
					continue;
				}
				missing.push(pgno);
			}
		}
		if tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE) {
			for pgno in target_pgnos {
				tracing::trace!(target: "sqlite_prefetch::trace", event = "access", op = "read",
                    stream_id = %format_args!("sqlite-{:p}", self),
                    pgno, hit = resolved.contains_key(pgno), request_id);
			}
		}
		if prefetch_consumed > 0 {
			self.prefetch_consumed_total
				.fetch_add(prefetch_consumed, Relaxed);
			self.update_operation_profile(|profile| {
				profile.prefetch_consumed_pages = profile
					.prefetch_consumed_pages
					.saturating_add(prefetch_consumed);
			});
		}

		if missing.is_empty() {
			self.resolve_pages_cache_hits
				.fetch_add(target_pgnos.len() as u64, Relaxed);
			if let Some(metrics) = &self.metrics {
				metrics.record_resolve_cache_hits(target_pgnos.len() as u64);
			}
			self.update_operation_profile(|profile| {
				profile.cache_hit_pages = profile
					.cache_hit_pages
					.saturating_add(target_pgnos.len() as u64);
			});
			return Ok(resolved);
		}
		self.resolve_pages_cache_hits
			.fetch_add((seen.len() - missing.len()) as u64, Relaxed);
		if let Some(metrics) = &self.metrics {
			metrics.record_resolve_cache_hits((seen.len() - missing.len()) as u64);
			metrics.record_resolve_cache_misses(missing.len() as u64);
		}
		self.update_operation_profile(|profile| {
			profile.cache_hit_pages = profile
				.cache_hit_pages
				.saturating_add((seen.len() - missing.len()) as u64);
			profile.cache_miss_pages = profile
				.cache_miss_pages
				.saturating_add(missing.len() as u64);
		});

		let (to_fetch, page_size, db_size_pages, expected_head_txid) = {
			let state = self.state.read();
			// The page server plans every prefetch; the client asks for what SQLite missed.
			let to_fetch = missing.clone();
			(
				to_fetch,
				state.page_size.max(1),
				state.db_size_pages,
				state.head_txid,
			)
		};

		{
			self.resolve_pages_fetches.fetch_add(1, Relaxed);
			self.pages_fetched_total
				.fetch_add(to_fetch.len() as u64, Relaxed);
			tracing::debug!(
				actor_id = %self.actor_id,
				generation = ?self.generation,
				requested_pages = ?target_pgnos,
				missing_pages = ?missing,
				total_fetch_pages = to_fetch.len(),
				total_fetch_bytes = to_fetch.len().saturating_mul(page_size),
				db_size_pages,
				expected_head_txid,
				"vfs get_pages fetch"
			);
		}

		let trace_span = tracing::trace_span!(target: "sqlite_prefetch::trace", "native_sqlite_read",
            stream_id = %format_args!("sqlite-{:p}", self), actor_id = %self.actor_id,
            generation = self.generation, local_request_id = request_id);
		let get_pages_start = Instant::now();
		// Transport rejection, including envoy shutdown while a VFS callback is
		// active, becomes GetPagesError here. The SQLite callback maps that to
		// SQLITE_IOERR_* because VFS has no richer async transport error channel.
		let response = self.runtime.block_on(
			self.transport
				.get_pages(protocol::SqliteGetPagesRequest {
					actor_id: self.actor_id.clone().into(),
					pgnos: to_fetch.clone(),
					expected_generation: None,
					expected_head_txid,
				})
				.instrument(trace_span.clone()),
		);
		// Only synchronous response handling holds this guard; the transport future above
		// carries its span across polls instead of entering a span across an await.
		let _trace_guard = trace_span.enter();
		let get_pages_duration_ns = get_pages_start.elapsed().as_nanos() as u64;
		if let Some(metrics) = &self.metrics {
			metrics.observe_get_pages_duration(get_pages_duration_ns);
		}
		let profile_active = self.operation_profile_active.load(Ordering::Relaxed);
		let ordinal = if profile_active {
			self.operation_profile
				.lock()
				.as_ref()
				.map_or(1, |profile| profile.get_pages_round_trips.saturating_add(1))
		} else {
			1
		};
		let response = match response {
			Ok(response) => response,
			Err(err) => {
				self.update_operation_profile(|profile| {
					profile.push_get_pages(
						SqliteGetPagesProfile {
							ordinal,
							duration_ns: get_pages_duration_ns,
							demand_requested: missing.len() as u64,
							..Default::default()
						},
						self.profile_request_limit(),
					);
				});
				if let Some(error) = storage_timeout(&err) {
					return Err(GetPagesError::Storage(error));
				}
				return Err(GetPagesError::Other(err.to_string()));
			}
		};

		match response {
			protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => {
				if profile_active {
					let requested = to_fetch.iter().copied().collect::<HashSet<_>>();
					let mut request_profile = SqliteGetPagesProfile {
						ordinal,
						duration_ns: get_pages_duration_ns,
						demand_requested: missing.len() as u64,
						success: true,
						..Default::default()
					};
					let mut btree_pages = 0_u64;
					let mut non_btree_pages = 0_u64;
					for fetched in &ok.pages {
						if !requested.contains(&fetched.pgno) {
							request_profile.overflow_expansion_extra =
								request_profile.overflow_expansion_extra.saturating_add(1);
						}
						if let Some(bytes) = &fetched.bytes {
							request_profile.response_present =
								request_profile.response_present.saturating_add(1);
							request_profile.response_bytes = request_profile
								.response_bytes
								.saturating_add(bytes.len() as u64);
							match classify(fetched.pgno, bytes) {
								PageClass::Btree => btree_pages = btree_pages.saturating_add(1),
								PageClass::Overflow => {
									non_btree_pages = non_btree_pages.saturating_add(1)
								}
							}
						} else {
							request_profile.response_missing =
								request_profile.response_missing.saturating_add(1);
						}
					}
					self.update_operation_profile(|profile| {
						profile.btree_pages = profile.btree_pages.saturating_add(btree_pages);
						profile.non_btree_pages =
							profile.non_btree_pages.saturating_add(non_btree_pages);
						profile.push_get_pages(request_profile, self.profile_request_limit());
					});
				}
				let response_head_txid = ok.head_txid;
				if let Some(head_txid) = response_head_txid {
					self.state.write().head_txid = Some(head_txid);
				}
				let missing_pages = missing.iter().copied().collect::<HashSet<_>>();
				let prefetch_buffer = self.state.read().prefetch_buffer.clone();
				#[cfg(debug_assertions)]
				let mut returned_pgnos = HashSet::new();
				#[cfg(debug_assertions)]
				let mut returned_missing_pages = Vec::new();
				#[cfg(debug_assertions)]
				let mut returned_missing_in_range_pages = Vec::new();
				let mut server_prefetched = 0_u64;
				for (ordinal, fetched) in ok.pages.into_iter().enumerate() {
					tracing::trace!(target: "sqlite_prefetch::trace", event = "page_received",
                        ordinal, pgno = fetched.pgno, present = fetched.bytes.is_some(),
                        bytes = fetched.bytes.as_ref().map_or(0, |bytes| bytes.len()),
                        demand = missing_pages.contains(&fetched.pgno));
					// Missing pages and the synthetic empty header are not delivered data.
					let delivered = fetched
						.bytes
						.as_ref()
						.and_then(|bytes| PageTrace::capture(ordinal, fetched.pgno, bytes.len()));
					#[cfg(debug_assertions)]
					{
						returned_pgnos.insert(fetched.pgno);
						if fetched.bytes.is_none() {
							returned_missing_pages.push(fetched.pgno);
							if fetched.pgno <= db_size_pages {
								returned_missing_in_range_pages.push(fetched.pgno);
							}
						}
					}
					let bytes = if fetched.bytes.is_none()
						&& self.commit_total.load(Relaxed) == 0
						&& missing_pages.contains(&fetched.pgno)
						&& fetched.pgno == 1
					{
						// An empty file can already have a nonzero inode version. Its
						// synthetic SQLite page must retain the storage response's fence.
						self.state.write().head_txid = Some(response_head_txid.unwrap_or(0));
						Some(PageBytes::from(empty_db_page()))
					} else {
						fetched.bytes.map(PageBytes::from)
					};
					if let Some(bytes) = &bytes {
						// A page fetched for a partial read is read again whole next, so it
						// stays buffered like a read-ahead page for that read.
						let kind = if missing_pages.contains(&fetched.pgno) {
							if partial {
								PageCacheInsertKind::PartialTarget
							} else {
								PageCacheInsertKind::Target
							}
						} else {
							PageCacheInsertKind::Prefetch
						};
						if kind == PageCacheInsertKind::Prefetch {
							server_prefetched += 1;
							if should_cache_page(&self.config, kind, fetched.pgno) {
								prefetch_buffer.insert_prefetched_traced(
									fetched.pgno,
									bytes.clone(),
									delivered,
								);
							} else if let Some(trace) = delivered {
								trace.disposition("unused");
							}
						} else {
							// A partial read already consumed these bytes even though a
							// copy remains for a later whole-page read.
							if let Some(trace) = delivered {
								trace.disposition("consumed");
							}
							cache_page(
								&self.config,
								&prefetch_buffer,
								kind,
								fetched.pgno,
								bytes.clone(),
							);
						}
					}
					resolved.insert(fetched.pgno, bytes);
				}
				self.prefetch_pages_total
					.fetch_add(server_prefetched, Relaxed);
				if let Some(metrics) = &self.metrics {
					metrics.record_get_pages_request(
						to_fetch.len() as u64,
						server_prefetched,
						page_size as u64,
					);
				}
				#[cfg(debug_assertions)]
				{
					let absent_response_pages = to_fetch
						.iter()
						.copied()
						.filter(|pgno| !returned_pgnos.contains(pgno))
						.collect::<Vec<_>>();
					let absent_in_range_pages = absent_response_pages
						.iter()
						.copied()
						.filter(|pgno| *pgno <= db_size_pages)
						.collect::<Vec<_>>();
					if !returned_missing_in_range_pages.is_empty()
						|| !absent_in_range_pages.is_empty()
					{
						tracing::warn!(
							actor_id = %self.actor_id,
							requested_pages = ?target_pgnos,
							missing_pages = ?missing,
							fetch_pages = ?to_fetch,
							db_size_pages,
							returned_missing_pages = ?returned_missing_pages,
							returned_missing_in_range_pages = ?returned_missing_in_range_pages,
							absent_response_pages = ?absent_response_pages,
							absent_in_range_pages = ?absent_in_range_pages,
							"sqlite get_pages returned missing pages within declared db size"
						);
					}
				}
				for pgno in missing {
					resolved.entry(pgno).or_insert(None);
				}
				Ok(resolved)
			}
			protocol::SqliteGetPagesResponse::SqliteErrorResponse(error) => {
				self.update_operation_profile(|profile| {
					profile.push_get_pages(
						SqliteGetPagesProfile {
							ordinal,
							duration_ns: get_pages_duration_ns,
							demand_requested: missing.len() as u64,
							..Default::default()
						},
						self.profile_request_limit(),
					);
				});
				if self.commit_total.load(Relaxed) == 0
					&& missing.contains(&1)
					&& is_initial_main_page_missing(&error.message)
				{
					for pgno in missing {
						let bytes = if pgno == 1 {
							Some(PageBytes::from(empty_db_page()))
						} else {
							None
						};
						resolved.entry(pgno).or_insert(bytes);
					}
					return Ok(resolved);
				}
				if is_head_fence_mismatch_response(&error) {
					return Err(GetPagesError::FenceMismatch(error.message.to_string()));
				}
				if let Some(refusal) = storage_refusal(&error) {
					return Err(GetPagesError::Storage(refusal));
				}
				Err(GetPagesError::Other(error.message.to_string()))
			}
		}
	}

	fn flush_dirty_pages(
		&self,
	) -> std::result::Result<Option<BufferedCommitOutcome>, CommitBufferError> {
		match self.flush_dirty_pages_with_timeout(None)? {
			CommitWait::Completed(outcome) => Ok(outcome),
			CommitWait::TimedOut => Err(CommitBufferError::Other(
				"sqlite commit timed out".to_string(),
			)),
		}
	}

	fn flush_dirty_pages_with_timeout(
		&self,
		timeout: Option<Duration>,
	) -> std::result::Result<CommitWait<Option<BufferedCommitOutcome>>, CommitBufferError> {
		let total_start = Instant::now();
		let request_build_start = Instant::now();
		let request = {
			let mut state = self.state.write();
			if state.dead {
				return Err(CommitBufferError::Other(
					"sqlite actor lost its fence".to_string(),
				));
			}
			if state.closing {
				state.discard_uncommitted_pages();
				return Ok(CommitWait::Completed(None));
			}
			if state.write_buffer.in_atomic_write
				|| (state.write_buffer.dirty.is_empty()
					&& state.db_size_pages == state.committed_db_size_pages)
			{
				return Ok(CommitWait::Completed(None));
			}
			// A corked commit is complete for SQLite once its pages are in the cork.
			match state.cork_dirty_pages(self.config.commit_page_cap()) {
				CorkOutcome::NotCorked => {}
				CorkOutcome::Corked => {
					state.refused_write_pending = false;
					return Ok(CommitWait::Completed(None));
				}
				CorkOutcome::Overflow => {
					drop(state);
					self.commit_cork_alone()?;
					let mut state = self.state.write();
					return match state.cork_dirty_pages(self.config.commit_page_cap()) {
						CorkOutcome::Corked | CorkOutcome::NotCorked => {
							state.refused_write_pending = false;
							Ok(CommitWait::Completed(None))
						}
						CorkOutcome::Overflow => Err(self.transaction_too_large_commit_error()),
					};
				}
			}

			BufferedCommitRequest {
				actor_id: self.actor_id.clone(),
				new_db_size_pages: state.db_size_pages,
				expected_head_txid: state.head_txid,
				dirty_pages: state.write_buffer.snapshot(),
			}
		};
		let request_build_ns = request_build_start.elapsed().as_nanos() as u64;
		self.update_operation_profile(|profile| {
			profile.dirty_pages = profile
				.dirty_pages
				.saturating_add(request.dirty_pages.len() as u64);
			profile.dirty_bytes = profile.dirty_bytes.saturating_add(
				request
					.dirty_pages
					.iter()
					.map(|page| page.bytes.len() as u64)
					.sum::<u64>(),
			);
		});

		let (outcome, transport_metrics) =
			// Transport rejection, including envoy shutdown while a VFS callback is
			// active, becomes CommitBufferError here. xSync and xClose then surface
			// it to SQLite as SQLITE_IOERR_*.
			match self.block_on_buffered_commit(request.clone(), timeout) {
				Ok(CommitWait::Completed(outcome)) => outcome,
				Ok(CommitWait::TimedOut) => return Ok(CommitWait::TimedOut),
				Err(err) => {
					tracing::error!(
						actor_id = %self.actor_id,
						new_db_size_pages = request.new_db_size_pages,
						dirty_pages = request.dirty_pages.len(),
						?err,
						"sqlite flush commit failed"
					);
					handle_non_finalize_commit_error(self, &err);
					return Err(err);
				}
			};
		self.commit_total
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		if let Some(metrics) = &self.metrics {
			metrics.record_commit(request.dirty_pages.len() as u64);
		}
		tracing::debug!(
			dirty_pages = request.dirty_pages.len(),
			path = ?outcome.path,
			requested_db_size_pages = request.new_db_size_pages,
			db_size_pages = outcome.db_size_pages,
			request_build_ns,
			serialize_ns = transport_metrics.serialize_ns,
			transport_ns = transport_metrics.transport_ns,
			"vfs commit complete (flush)"
		);
		#[cfg(debug_assertions)]
		{
			if outcome.db_size_pages != request.new_db_size_pages {
				tracing::warn!(
					actor_id = %self.actor_id,
					dirty_pages = request.dirty_pages.len(),
					path = ?outcome.path,
					requested_db_size_pages = request.new_db_size_pages,
					outcome_db_size_pages = outcome.db_size_pages,
					"sqlite flush commit returned db size different from request"
				);
			}
		}
		let state_update_start = Instant::now();
		let mut state = self.state.write();
		state.db_size_pages = outcome.db_size_pages;
		state.committed_db_size_pages = outcome.db_size_pages;
		state.head_txid = outcome
			.head_txid
			.or_else(|| state.head_txid.map(|head_txid| head_txid.saturating_add(1)));
		for dirty_page in &request.dirty_pages {
			state.note_committed_page(&self.config, dirty_page.pgno, &dirty_page.bytes);
		}
		state.write_buffer.dirty.clear();
		state.refused_write_pending = false;
		let state_update_ns = state_update_start.elapsed().as_nanos() as u64;
		self.add_commit_phase_metrics(
			request_build_ns,
			transport_metrics,
			state_update_ns,
			total_start.elapsed().as_nanos() as u64,
		);
		Ok(CommitWait::Completed(Some(outcome)))
	}

	fn commit_atomic_write(&self) -> std::result::Result<(), CommitBufferError> {
		match self.commit_atomic_write_with_timeout(None)? {
			CommitWait::Completed(()) => Ok(()),
			CommitWait::TimedOut => Err(CommitBufferError::Other(
				"sqlite commit timed out".to_string(),
			)),
		}
	}

	fn commit_atomic_write_with_timeout(
		&self,
		timeout: Option<Duration>,
	) -> std::result::Result<CommitWait<()>, CommitBufferError> {
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
				return Ok(CommitWait::Completed(()));
			}
			if state.closing {
				state.discard_uncommitted_pages();
				return Ok(CommitWait::Completed(()));
			}
			if state.write_buffer.dirty.is_empty()
				&& state.db_size_pages == state.committed_db_size_pages
			{
				state.write_buffer.in_atomic_write = false;
				return Ok(CommitWait::Completed(()));
			}
			match state.cork_dirty_pages(self.config.commit_page_cap()) {
				CorkOutcome::NotCorked => {}
				CorkOutcome::Corked => {
					state.write_buffer.in_atomic_write = false;
					state.refused_write_pending = false;
					return Ok(CommitWait::Completed(()));
				}
				CorkOutcome::Overflow => {
					drop(state);
					self.commit_cork_alone()?;
					let mut state = self.state.write();
					return match state.cork_dirty_pages(self.config.commit_page_cap()) {
						CorkOutcome::Corked | CorkOutcome::NotCorked => {
							state.write_buffer.in_atomic_write = false;
							state.refused_write_pending = false;
							Ok(CommitWait::Completed(()))
						}
						CorkOutcome::Overflow => Err(self.transaction_too_large_commit_error()),
					};
				}
			}

			BufferedCommitRequest {
				actor_id: self.actor_id.clone(),
				new_db_size_pages: state.db_size_pages,
				expected_head_txid: state.head_txid,
				dirty_pages: state.write_buffer.snapshot(),
			}
		};
		let request_build_ns = request_build_start.elapsed().as_nanos() as u64;
		self.update_operation_profile(|profile| {
			profile.dirty_pages = profile
				.dirty_pages
				.saturating_add(request.dirty_pages.len() as u64);
			profile.dirty_bytes = profile.dirty_bytes.saturating_add(
				request
					.dirty_pages
					.iter()
					.map(|page| page.bytes.len() as u64)
					.sum::<u64>(),
			);
		});

		let (outcome, transport_metrics) =
			match self.block_on_buffered_commit(request.clone(), timeout) {
				Ok(CommitWait::Completed(outcome)) => outcome,
				Ok(CommitWait::TimedOut) => return Ok(CommitWait::TimedOut),
				Err(err) => {
					tracing::error!(
						actor_id = %self.actor_id,
						new_db_size_pages = request.new_db_size_pages,
						dirty_pages = request.dirty_pages.len(),
						?err,
						"sqlite atomic commit failed"
					);
					handle_non_finalize_commit_error(self, &err);
					return Err(err);
				}
			};
		self.commit_total
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		if let Some(metrics) = &self.metrics {
			metrics.record_commit(request.dirty_pages.len() as u64);
		}
		tracing::debug!(
			dirty_pages = request.dirty_pages.len(),
			path = ?outcome.path,
			requested_db_size_pages = request.new_db_size_pages,
			db_size_pages = outcome.db_size_pages,
			request_build_ns,
			serialize_ns = transport_metrics.serialize_ns,
			transport_ns = transport_metrics.transport_ns,
			"vfs commit complete (atomic)"
		);
		#[cfg(debug_assertions)]
		{
			if outcome.db_size_pages != request.new_db_size_pages {
				tracing::warn!(
					actor_id = %self.actor_id,
					dirty_pages = request.dirty_pages.len(),
					path = ?outcome.path,
					requested_db_size_pages = request.new_db_size_pages,
					outcome_db_size_pages = outcome.db_size_pages,
					"sqlite atomic commit returned db size different from request"
				);
			}
		}
		self.clear_last_error();
		let state_update_start = Instant::now();
		let mut state = self.state.write();
		state.db_size_pages = outcome.db_size_pages;
		state.committed_db_size_pages = outcome.db_size_pages;
		state.head_txid = outcome
			.head_txid
			.or_else(|| state.head_txid.map(|head_txid| head_txid.saturating_add(1)));
		for dirty_page in &request.dirty_pages {
			state.note_committed_page(&self.config, dirty_page.pgno, &dirty_page.bytes);
		}
		state.write_buffer.dirty.clear();
		state.write_buffer.in_atomic_write = false;
		state.refused_write_pending = false;
		let state_update_ns = state_update_start.elapsed().as_nanos() as u64;
		self.add_commit_phase_metrics(
			request_build_ns,
			transport_metrics,
			state_update_ns,
			total_start.elapsed().as_nanos() as u64,
		);
		Ok(CommitWait::Completed(()))
	}

	/// Returns true when the truncate left behind a size change that only a commit can carry to
	/// depot.
	///
	/// SQLite's batch-atomic commit path truncates *after* `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` and
	/// does not sync afterwards, so a shrink recorded only in memory here never reaches depot. When
	/// the write buffer is empty the caller has to issue a size-only commit. When it is not, the
	/// buffered pages carry the new size on the commit that follows, and committing here would
	/// publish a transaction SQLite has not finished.
	fn truncate_main_file(&self, size: sqlite3_int64) -> bool {
		let page_size = self.page_size() as i64;
		let truncated_pages = ((size + page_size - 1) / page_size) as u32;
		let mut state = self.state.write();
		state.db_size_pages = truncated_pages;
		state
			.write_buffer
			.dirty
			.retain(|pgno, _| *pgno <= truncated_pages);
		let between_transactions =
			!state.write_buffer.in_atomic_write && state.write_buffer.dirty.is_empty();
		if let Some(corked) = state.corked.as_mut() {
			corked.retain(|pgno, _| *pgno <= truncated_pages);
		}
		// A shrink between transactions belongs to the last corked one.
		if state.corked.is_some() && between_transactions {
			state.corked_db_size_pages = truncated_pages;
		}
		state.invalidate_page_buffers();
		// A corked size change rides the uncorking commit.
		state.corked.is_none()
			&& !state.write_buffer.in_atomic_write
			&& state.write_buffer.dirty.is_empty()
			&& state.db_size_pages != state.committed_db_size_pages
	}

	/// Corks commits: until `uncork`, each transaction SQLite commits stays in memory instead
	/// of going to depot. SQLite and its readers see the transactions as committed; none is
	/// durable until uncorking commits them all at once.
	fn cork(&self) {
		let mut state = self.state.write();
		if state.corked.is_none() {
			state.corked = Some(BTreeMap::new());
			// A cork inside a transaction starts from the committed size, not the size the
			// open transaction's spilled pages reached, so a close that discards the
			// transaction does not publish its growth.
			state.corked_db_size_pages = state.committed_db_size_pages;
		}
	}

	/// Sends every transaction committed while corked to depot as one commit, then commits
	/// directly again. It must run between transactions, and fails without uncorking inside
	/// one. On a failed commit the pages are not durable and the database is unusable, as
	/// after any failed commit.
	fn uncork(&self) -> std::result::Result<(), CommitBufferError> {
		{
			let mut state = self.state.write();
			if state.corked.is_none() {
				return Ok(());
			}
			if state.write_buffer.in_atomic_write || !state.write_buffer.dirty.is_empty() {
				return Err(CommitBufferError::Other(
					"cannot uncork sqlite commits inside a transaction".to_string(),
				));
			}
			state.write_buffer.dirty = state.corked.take().unwrap_or_default();
		}
		self.flush_dirty_pages().map(|_| ())
	}
}

impl Drop for VfsContext {
	fn drop(&mut self) {
		self.state.write().invalidate_page_buffers();
		self.report_prefetch_unused();
		let state = self.state.read();
		let page_cache_entries = state
			.prefetch_buffer
			.entry_count()
			.saturating_add(state.committed_buffer.entry_count());
		let page_cache_weighted_size = state.prefetch_buffer.weighted_size();
		tracing::debug!(
			actor_id = %self.actor_id,
			generation = ?self.generation,
			resolve_pages_calls = self.resolve_pages_total.load(Ordering::Relaxed),
			resolve_pages_cache_hits = self.resolve_pages_cache_hits.load(Ordering::Relaxed),
			get_pages_round_trips = self.resolve_pages_fetches.load(Ordering::Relaxed),
			pages_fetched_total = self.pages_fetched_total.load(Ordering::Relaxed),
			prefetch_pages_total = self.prefetch_pages_total.load(Ordering::Relaxed),
			prefetch_consumed_total = self.prefetch_consumed_total.load(Ordering::Relaxed),
			commit_count = self.commit_total.load(Ordering::Relaxed),
			db_size_pages = state.db_size_pages,
			head_txid = state.head_txid,
			page_cache_entries,
			page_cache_weighted_size,
			page_cache_capacity_pages = self.config.page_budget.capacity_pages(),
			"sqlite vfs close summary"
		);
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

/// Ephemeral commit sink used only by the process-startup batch-atomic probe.
///
/// The probe transaction is intentionally small enough to use a single-shot commit. Pages only
/// need to remain visible inside that transaction, where the VFS write buffer owns them, so the
/// sink does not need to retain committed data.
struct BatchAtomicProbeTransport;

#[async_trait]
impl SqliteTransport for BatchAtomicProbeTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
			protocol::SqliteGetPagesOk {
				pages: request
					.pgnos
					.into_iter()
					.map(|pgno| protocol::SqliteFetchedPage { pgno, bytes: None })
					.collect(),
				head_txid: Some(0),
			},
		))
	}

	async fn commit(
		&self,
		_request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
			protocol::SqliteCommitOk { head_txid: None },
		))
	}

	async fn commit_stage_begin(
		&self,
		_request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		anyhow::bail!("batch-atomic startup probe unexpectedly attempted a staged commit")
	}

	async fn commit_stage_segment(
		&self,
		_request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse> {
		anyhow::bail!("batch-atomic startup probe unexpectedly attempted a staged commit")
	}

	async fn commit_finalize(
		&self,
		_request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		anyhow::bail!("batch-atomic startup probe unexpectedly attempted a staged commit")
	}
}

fn verify_batch_atomic_writes_at_process_startup_blocking(
	runtime: Handle,
	config: VfsConfig,
) -> std::result::Result<(), String> {
	let probe_id = NEXT_TEMP_AUX_ID.fetch_add(1, Ordering::Relaxed);
	let file_name = format!("batch-atomic-startup-probe-{probe_id}.db");
	let vfs = SqliteVfs::register_with_transport_and_initial_pages(
		&format!("batch-atomic-startup-probe-{probe_id}"),
		Arc::new(BatchAtomicProbeTransport),
		file_name.clone(),
		runtime,
		config,
		InitialPages::default(),
		None,
	)?;
	let connection = open_connection(
		Arc::new(vfs),
		&file_name,
		SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
	)?;
	configure_connection_for_database(connection.as_ptr(), &connection._vfs, &file_name)
		.map_err(|error| error.to_string())?;
	assert_batch_atomic_probe(connection.as_ptr(), &connection._vfs)
}

/// Verifies the linked SQLite pager and the production VFS batch-atomic callbacks before this
/// process becomes eligible to run actors.
pub async fn verify_batch_atomic_writes_at_process_startup() -> Result<()> {
	verify_batch_atomic_writes_at_process_startup_with_config(VfsConfig::default()).await
}

async fn verify_batch_atomic_writes_at_process_startup_with_config(
	config: VfsConfig,
) -> Result<()> {
	let runtime = Handle::try_current().map_err(|err| {
		anyhow::anyhow!("batch-atomic startup probe requires a Tokio runtime: {err}")
	})?;
	let result = tokio::task::spawn_blocking(move || {
		verify_batch_atomic_writes_at_process_startup_blocking(runtime, config)
	})
	.await
	.map_err(|err| anyhow::anyhow!("batch-atomic startup probe task failed: {err}"))?;
	result.map_err(anyhow::Error::msg)
}

fn handle_non_finalize_commit_error(ctx: &VfsContext, err: &CommitBufferError) {
	match err {
		CommitBufferError::FenceMismatch(message) => ctx.mark_fatal(message.clone()),
		CommitBufferError::Indeterminate(operation) => {
			ctx.mark_indeterminate(operation.clone());
		}
		CommitBufferError::Storage(error) => ctx.set_storage_error(error.clone()),
		CommitBufferError::Other(message) => ctx.set_last_error(message.clone()),
	}
}

fn handle_finalize_fence_error(ctx: &VfsContext, err: &CommitBufferError) {
	if let CommitBufferError::FenceMismatch(reason) = err {
		ctx.mark_fatal(reason.clone());
	}
}



pub(crate) async fn fetch_initial_pages_for_registration(
	transport: SqliteTransportHandle,
	actor_id: &str,
	generation: u64,
	_config: &VfsConfig,
) -> std::result::Result<InitialPages, String> {
	fetch_initial_pages(transport, actor_id.to_string(), generation, 1).await
}

async fn fetch_initial_pages(
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	page_count: u32,
) -> std::result::Result<InitialPages, String> {
	tracing::debug!(
		actor_id = %actor_id,
		generation,
		page_count,
		"sqlite initial page preload request"
	);

	let request_actor_id = actor_id.clone();
	let response = transport
		.get_pages(protocol::SqliteGetPagesRequest {
			actor_id: request_actor_id.into(),
			pgnos: (1..=page_count).collect(),
			expected_generation: None,
			expected_head_txid: None,
		})
		.await;

	match response {
		Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok)) => {
			let head_txid = ok.head_txid;
			let pages: Vec<_> = ok
				.pages
				.into_iter()
				.filter_map(|page| page.bytes.map(|bytes| (page.pgno, bytes.into())))
				.collect();
			tracing::debug!(
				actor_id = %actor_id,
				generation,
				page_count,
				loaded_pages = pages.len(),
				head_txid,
				"sqlite initial page preload result"
			);
			Ok(InitialPages {
				pages,
				head_txid,
				requested_page_count: page_count,
			})
		}
		Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(error)) => {
			if !is_initial_main_page_missing(&error.message) {
				return Err(format!(
					"sqlite initial page fetch failed: {}",
					error.message
				));
			}
			tracing::info!(
				actor_id = %actor_id,
				generation,
				page_count,
				error = %error.message,
				"sqlite initial page fetch did not find persisted data"
			);
			Ok(InitialPages {
				pages: Vec::new(),
				head_txid: Some(0),
				requested_page_count: page_count,
			})
		}
		Err(err) => Err(format!("sqlite initial page fetch failed: {err}")),
	}
}

fn is_initial_main_page_missing(message: &str) -> bool {
	message.contains("sqlite database was not found in this bucket branch")
		|| message.contains("sqlite meta missing for get_pages")
		|| message == "actor does not exist"
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

/// Dirty pages above which a commit is staged in segments rather than sent as one message.
///
/// Matches the engine's single-shot cap. A commit at or under it fits one FDB transaction with its
/// page bytes included, so it takes the single round trip; anything larger has to be staged because
/// the engine would reject it, not merely because staging is faster.
///
/// Kept as its own value rather than imported, since the engine is free to lower its cap and this
/// side must keep working against an engine on either side of that change. Sending a single-shot
/// commit the engine refuses fails the commit, while staging one it would have accepted only costs
/// extra round trips.
const COMMIT_STAGE_THRESHOLD_PAGES: usize = 320;

async fn commit_buffered_pages(
	transport: &dyn SqliteTransport,
	request: BufferedCommitRequest,
	max_dirty_pages: usize,
) -> std::result::Result<(BufferedCommitOutcome, CommitTransportMetrics), CommitBufferError> {
	// Refused before sending rather than discovered partway through. The engine enforces the same
	// cap and is the authority, but finding out from it means staging up to 128 MiB of segments
	// first, and the bytes already staged are only reclaimed when some later commit reopens the txid.
	if request.dirty_pages.len() > max_dirty_pages {
		return Err(CommitBufferError::Other(format!(
			"commit of {} dirty pages exceeds the {} page maximum",
			request.dirty_pages.len(),
			max_dirty_pages,
		)));
	}

	if request.dirty_pages.len() > COMMIT_STAGE_THRESHOLD_PAGES {
		return commit_staged_pages(transport, request).await;
	}

	let mut metrics = CommitTransportMetrics::default();
	let serialize_start = Instant::now();
	let BufferedCommitRequest {
		actor_id,
		new_db_size_pages,
		dirty_pages,
		expected_head_txid,
	} = request;
	let commit_request = protocol::SqliteCommitRequest {
		actor_id: actor_id.into(),
		dirty_pages: dirty_pages
			.into_iter()
			.map(|page| protocol::SqliteDirtyPage {
				pgno: page.pgno,
				bytes: page.bytes.into_bytes(),
			})
			.collect(),
		db_size_pages: new_db_size_pages,
		now_ms: sqlite_now_ms().map_err(|err| CommitBufferError::Other(err.to_string()))?,
		expected_generation: None,
		expected_head_txid,
	};
	metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;
	let transport_start = Instant::now();
	match transport
		.commit(commit_request)
		.await
		.map_err(commit_transport_error)?
	{
		protocol::SqliteCommitResponse::SqliteCommitOk(ok) => {
			metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;
			Ok((
				BufferedCommitOutcome {
					path: CommitPath::Fast,
					db_size_pages: new_db_size_pages,
					head_txid: ok.head_txid,
				},
				metrics,
			))
		}
		protocol::SqliteCommitResponse::SqliteErrorResponse(error) => {
			if is_head_fence_mismatch_response(&error) {
				Err(CommitBufferError::FenceMismatch(error.message.to_string()))
			} else if let Some(refusal) = storage_refusal(&error) {
				Err(CommitBufferError::Storage(refusal))
			} else {
				Err(CommitBufferError::Other(error.message.to_string()))
			}
		}
	}
}

/// Commits pages too numerous for one message by staging them in shard-aligned segments.
///
/// The whole commit fails on any error, exactly as the single-shot path does: SQLite sees one failed
/// commit, never a partial one. Nothing staged is visible until finalize, so an abandoned attempt
/// leaves the database exactly as it was, and the engine reclaims the staged bytes when the next
/// attempt reopens the same txid.
async fn commit_staged_pages(
	transport: &dyn SqliteTransport,
	request: BufferedCommitRequest,
) -> std::result::Result<(BufferedCommitOutcome, CommitTransportMetrics), CommitBufferError> {
	let mut metrics = CommitTransportMetrics::default();
	let serialize_start = Instant::now();
	let mut dirty_pages = request.dirty_pages;
	// Cutting segments needs ascending pages, and sorting once here keeps the per-segment work to a
	// slice.
	dirty_pages.sort_by_key(|page| page.pgno);
	metrics.serialize_ns += serialize_start.elapsed().as_nanos() as u64;

	let transport_start = Instant::now();
	let txid = match transport
		.commit_stage_begin(protocol::SqliteCommitStageBeginRequest {
			actor_id: request.actor_id.clone().into(),
			expected_generation: None,
			expected_head_txid: request.expected_head_txid,
		})
		.await
		.map_err(commit_transport_error)?
	{
		protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(ok) => ok.txid,
		protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(error) => {
			return Err(staged_commit_error(error));
		}
	};

	let segments = sqlite_types::cut_page_segments(&dirty_pages, |page| page.pgno);
	let segment_first_pgnos = segments.iter().map(|(first_pgno, _)| *first_pgno).collect();
	// Each segment spans at most five shards. Buffered requests cap their combined page bytes,
	// while ordered responses preserve the first failure and finalize waits for every admission.
	let responses = stream::iter(segments)
		.map(|(first_pgno, segment)| {
			let dirty_pages = segment
				.iter()
				.map(|page| protocol::SqliteDirtyPage {
					pgno: page.pgno,
					bytes: page.bytes.clone().into_bytes(),
				})
				.collect();
			let actor_id = request.actor_id.clone();
			async move {
				match transport
					.commit_stage_segment(protocol::SqliteCommitStageSegmentRequest {
						actor_id: actor_id.into(),
						expected_generation: None,
						txid,
						first_pgno,
						dirty_pages,
					})
					.await
					.map_err(commit_transport_error)?
				{
					protocol::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(_) => {
						Ok(())
					}
					protocol::SqliteCommitStageSegmentResponse::SqliteErrorResponse(error) => {
						Err(staged_commit_error(error))
					}
				}
			}
		})
		.buffered(STAGE_PIPELINE_DEPTH)
		.collect::<Vec<_>>()
		.await;
	for response in responses {
		response?;
	}

	let head_txid = match transport
		.commit_finalize(protocol::SqliteCommitFinalizeRequest {
			actor_id: request.actor_id.clone().into(),
			expected_generation: None,
			txid,
			new_db_size_pages: request.new_db_size_pages,
			now_ms: sqlite_now_ms().map_err(|err| CommitBufferError::Other(err.to_string()))?,
			segment_first_pgnos,
		})
		.await
		.map_err(commit_transport_error)?
	{
		protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) => ok.head_txid,
		protocol::SqliteCommitFinalizeResponse::SqliteErrorResponse(error) => {
			return Err(staged_commit_error(error));
		}
	};
	metrics.transport_ns += transport_start.elapsed().as_nanos() as u64;

	Ok((
		BufferedCommitOutcome {
			path: CommitPath::Staged,
			db_size_pages: request.new_db_size_pages,
			head_txid,
		},
		metrics,
	))
}

fn commit_transport_error(error: anyhow::Error) -> CommitBufferError {
	if let Some(timeout) = storage_timeout(&error) {
		return CommitBufferError::Storage(timeout);
	}
	match error.downcast::<SqliteCommitIndeterminateError>() {
		Ok(error) => CommitBufferError::Indeterminate(error.0),
		Err(error) => CommitBufferError::Other(error.to_string()),
	}
}

fn staged_commit_error(error: protocol::SqliteErrorResponse) -> CommitBufferError {
	if is_head_fence_mismatch_response(&error) {
		CommitBufferError::FenceMismatch(error.message.to_string())
	} else if let Some(refusal) = storage_refusal(&error) {
		CommitBufferError::Storage(refusal)
	} else {
		CommitBufferError::Other(error.message.to_string())
	}
}

fn is_head_fence_mismatch_response(error: &protocol::SqliteErrorResponse) -> bool {
	is_head_fence_mismatch(&error.group, &error.code)
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

pub(crate) fn sqlite_exec(db: *mut sqlite3, sql: &str) -> std::result::Result<(), String> {
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
			
			ctx.discard_refused_transaction();
			let should_flush = {
				let state = ctx.state.read();
				state.write_buffer.in_atomic_write
					|| !state.write_buffer.dirty.is_empty()
					|| state.db_size_pages != state.committed_db_size_pages
					|| state.corked.is_some()
			};
			if should_flush {
				if ctx.state.read().write_buffer.in_atomic_write {
					ctx.commit_atomic_write().map(|_| ())
				} else {
					ctx.uncork()
						.and_then(|()| ctx.flush_dirty_pages().map(|_| ()))
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
				handle_finalize_fence_error(ctx, &err);
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
		let (file_size, db_size_pages) = {
			let state = ctx.state.read();
			(
				state.db_size_pages as usize * state.page_size,
				state.db_size_pages,
			)
		};

		let partial = i_offset as usize % page_size != 0 || i_amt as usize % page_size != 0;
		let resolved = match ctx.resolve_pages(&requested_pages, partial) {
			Ok(pages) => pages,
			Err(GetPagesError::FenceMismatch(message)) => {
				tracing::error!(
					actor_id = %ctx.actor_id,
					requested_pages = ?requested_pages,
					error = %message,
					"sqlite xRead hit fatal sqlite error"
				);
				ctx.mark_fatal(message);
				return SQLITE_IOERR_READ;
			}
			Err(GetPagesError::Storage(error)) => {
				tracing::warn!(
					actor_id = %ctx.actor_id,
					requested_pages = ?requested_pages,
					%error,
					"sqlite xRead failed without changing storage; the statement fails with a typed storage error"
				);
				ctx.set_storage_error(error);
				return SQLITE_IOERR_READ;
			}
			Err(GetPagesError::Other(message)) => {
				tracing::error!(
					actor_id = %ctx.actor_id,
					requested_pages = ?requested_pages,
					error = %message,
					"sqlite xRead failed to resolve pages"
				);
				ctx.set_last_error(message);
				return SQLITE_IOERR_READ;
			}
		};
		ctx.clear_last_error();

		#[cfg(debug_assertions)]
		{
			let missing_in_range_pages = requested_pages
				.iter()
				.copied()
				.filter(|pgno| *pgno <= db_size_pages)
				.filter(|pgno| !matches!(resolved.get(pgno), Some(Some(_))))
				.collect::<Vec<_>>();
			if !missing_in_range_pages.is_empty() {
				tracing::warn!(
					actor_id = %ctx.actor_id,
					offset = i_offset,
					amount = i_amt,
					page_size,
					db_size_pages,
					file_size,
					requested_pages = ?requested_pages,
					missing_in_range_pages = ?missing_in_range_pages,
					"sqlite xRead would zero-fill pages within declared db size"
				);
			}
		}

		buf.fill(0);
		for pgno in requested_pages.iter().copied() {
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

/// How many page records a write adds to the main rollback journal.
///
/// A rollback replays every journal record, so the journal holds at most as many records as
/// one commit may carry. SQLite writes a record as a 4 byte page number, the page at an offset
/// that is never sector-aligned, and a 4 byte checksum; headers are page-sized but
/// sector-aligned. When it moves an in-memory journal to the file, it copies it in chunks of
/// any size, so other writes that extend the file count every record they could hold. That
/// overcounts the headers, which only makes the cap refuse a little earlier.
fn journal_records_added(offset: usize, len: usize, file_len: usize, page_size: usize) -> usize {
	if len == page_size {
		return usize::from(offset % DEFAULT_PAGE_SIZE != 0);
	}
	if len == 4 {
		return 0;
	}
	let appended = (offset + len).saturating_sub(file_len.max(offset));
	appended.div_ceil(page_size + 8)
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
		if let Some(aux) = get_aux_state(file) {
			if i_offset < 0 {
				return SQLITE_IOERR_WRITE;
			}

			let offset = i_offset as usize;
			let source = slice::from_raw_parts(p_buf.cast::<u8>(), i_amt as usize);
			let end = offset + source.len();
			let mut bytes = aux.state.bytes.lock();
			if aux.is_main_journal {
				let ctx = &*file.ctx;
				let added =
					journal_records_added(offset, source.len(), bytes.len(), ctx.page_size());
				let records = aux.state.journal_records.load(Ordering::Relaxed);
				if added > 0 && records + added > ctx.config.commit_page_cap() {
					drop(bytes);
					return ctx.refuse_oversize_transaction();
				}
				aux.state
					.journal_records
					.store(records + added, Ordering::Relaxed);
			}
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

		// Bound the transaction's dirty pages before holding them, not when committing them.
		// Pages the transaction already changed stay writable, so SQLite's rollback, which
		// rewrites journaled pages, always fits.
		{
			let state = ctx.state.read();
			let new_pages = target_pages
				.iter()
				.filter(|pgno| !state.write_buffer.dirty.contains_key(pgno))
				.count();
			if state.write_buffer.dirty.len() + new_pages > ctx.config.commit_page_cap() {
				drop(state);
				ctx.state.write().refused_write_pending = true;
				return ctx.refuse_oversize_transaction();
			}
		}

		// Fast path: for full-page aligned writes we don't need the existing
		// page data because we're overwriting every byte. Skip resolve_pages
		// to eliminate a round trip to the engine per page. Also, for pages
		// beyond db_size_pages (new allocations), there's nothing to fetch.
		let offset = i_offset as usize;
		let amt = i_amt as usize;
		let is_aligned_full_page = offset % page_size == 0 && amt % page_size == 0;

		let mut resolved = HashMap::new();
		if !is_aligned_full_page {
			let (db_size_pages, pages_to_resolve): (u32, Vec<u32>) = {
				let state = ctx.state.read();
				let known_max = state.db_size_pages;
				(
					known_max,
					target_pages
						.iter()
						.copied()
						.filter(|pgno| {
							*pgno <= known_max
								&& !state.write_buffer.dirty.contains_key(pgno)
								&& state.corked_page(*pgno).is_none()
						})
						.collect(),
				)
			};

			if !pages_to_resolve.is_empty() {
				resolved = match ctx.resolve_pages(&pages_to_resolve, false) {
					Ok(pages) => pages,
					Err(GetPagesError::FenceMismatch(message)) => {
						ctx.mark_fatal(message);
						return SQLITE_IOERR_WRITE;
					}
					Err(GetPagesError::Storage(error)) => {
						ctx.set_storage_error(error);
						return SQLITE_IOERR_WRITE;
					}
					Err(GetPagesError::Other(message)) => {
						ctx.set_last_error(message);
						return SQLITE_IOERR_WRITE;
					}
				};
			}
			for pgno in &target_pages {
				if *pgno > db_size_pages {
					resolved.entry(*pgno).or_insert(None);
				}
			}
			#[cfg(debug_assertions)]
			{
				let missing_existing_pages = target_pages
					.iter()
					.copied()
					.filter(|pgno| *pgno <= db_size_pages)
					.filter(|pgno| pages_to_resolve.contains(pgno))
					.filter(|pgno| !matches!(resolved.get(pgno), Some(Some(_))))
					.collect::<Vec<_>>();
				if !missing_existing_pages.is_empty() {
					tracing::warn!(
						actor_id = %ctx.actor_id,
						offset = i_offset,
						amount = i_amt,
						page_size,
						db_size_pages,
						target_pages = ?target_pages,
						missing_existing_pages = ?missing_existing_pages,
						"sqlite xWrite partial update would synthesize existing pages from zeros"
					);
				}
			}
		}

		let mut state = ctx.state.write();
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

			let page_offset = patch_start - page_start;
			let source_offset = patch_start - offset;
			if is_aligned_full_page {
				state.write_buffer.dirty.insert(
					pgno,
					source[source_offset..source_offset + copy_len]
						.to_vec()
						.into(),
				);
				continue;
			}

			let mut page = state
				.write_buffer
				.dirty
				.get_mut(&pgno)
				.map(std::mem::take)
				.map(PageBytes::into_vec)
				.unwrap_or_else(|| {
					resolved
						.remove(&pgno)
						.flatten()
						.map(PageBytes::into_vec)
						.unwrap_or_else(|| vec![0; page_size])
				});
			if page.len() < page_size {
				page.resize(page_size, 0);
			}
			page[page_offset..page_offset + copy_len]
				.copy_from_slice(&source[source_offset..source_offset + copy_len]);
			state.write_buffer.dirty.insert(pgno, page.into());
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
		if let Some(aux) = get_aux_state(file) {
			let mut bytes = aux.state.bytes.lock();
			bytes.truncate(size as usize);
			bytes.shrink_to_fit();
			if size == 0 {
				aux.state.journal_records.store(0, Ordering::Relaxed);
			}
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		if !ctx.truncate_main_file(size) {
			return SQLITE_OK;
		}
		match ctx.flush_dirty_pages() {
			Ok(_) => SQLITE_OK,
			Err(err) => {
				tracing::error!(
					actor_id = %ctx.actor_id,
					last_error = ?ctx.clone_last_error(),
					?err,
					"sqlite truncate commit failed"
				);
				handle_finalize_fence_error(ctx, &err);
				ctx.abandon_write_buffer();
				SQLITE_IOERR_TRUNCATE
			}
		}
	})
}

/// xSync returns once `ctx.flush_dirty_pages()` resolves. Durability of those
/// bytes is delegated to depot's `sqlite_commit` reply. If pegboard-envoy ever
/// pre-acks before the FDB tx commit, xSync's durability contract is broken.
unsafe extern "C" fn io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		if get_aux_state(file).is_some() {
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		if let Some(message) = ctx.take_transient_commit_error() {
			ctx.set_last_error(message);
			ctx.abandon_write_buffer();
			return SQLITE_IOERR_FSYNC;
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
				handle_finalize_fence_error(ctx, &err);
				ctx.abandon_write_buffer();
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

// Lock callbacks are intentional no-ops. Pegboard guarantees a single actor
// process per actor_id, the database is opened with `locking_mode=EXCLUSIVE`,
// and only one SQLite connection runs against it, so SQLite's internal lock
// state machine is single-party and has nothing to coordinate with. Flipping
// to a non-EXCLUSIVE locking mode or a multi-connection setup would require
// implementing a real lock state ladder here.
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
	p_arg: *mut c_void,
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
				state.refused_write_pending = false;
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => match ctx.commit_atomic_write() {
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
					match &err {
						CommitBufferError::Other(message) => {
							ctx.defer_transient_commit_error(message.clone());
						}
						CommitBufferError::Storage(error) => {
							ctx.defer_transient_commit_error(error.to_string());
						}
						CommitBufferError::FenceMismatch(_)
						| CommitBufferError::Indeterminate(_) => {}
					}
					handle_finalize_fence_error(ctx, &err);
					SQLITE_IOERR
				}
			},
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				let mut state = ctx.state.write();
				state.write_buffer.dirty.clear();
				state.write_buffer.in_atomic_write = false;
				state.db_size_pages = state.write_buffer.saved_db_size;
				state.refused_write_pending = false;
				SQLITE_OK
			}
			// VACUUM announces the size of the database it is about to write over this one,
			// after building the copy and before writing any of it.
			SQLITE_FCNTL_OVERWRITE => {
				if p_arg.is_null() {
					return SQLITE_NOTFOUND;
				}
				let bytes = *(p_arg as *const i64);
				let pages = (bytes.max(0) as usize).div_ceil(ctx.page_size());
				if pages > ctx.config.commit_page_cap() {
					ctx.refuse_oversize_transaction()
				} else {
					SQLITE_NOTFOUND
				}
			}
			FCNTL_COMMIT_PAGE_CAP => {
				if p_arg.is_null() {
					return SQLITE_MISUSE;
				}
				*(p_arg as *mut i64) = ctx.config.commit_page_cap() as i64;
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
		if get_aux_state(file).is_some() {
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

		

		let base = sqlite3_file {
			pMethods: ctx.io_methods.as_ref(),
		};
		
		let aux = if is_main {
			ptr::null_mut()
		} else {
			Box::into_raw(Box::new(AuxFileHandle {
				is_main_journal: (flags & SQLITE_OPEN_MAIN_JOURNAL) != 0
					&& path.strip_suffix("-journal") == Some(ctx.actor_id.as_str()),
				path: path.clone(),
				state: ctx.open_aux_file(&path),
				delete_on_close,
			}))
		};
		ptr::write(
			p_file.cast::<VfsFile>(),
			VfsFile {
				base,
				ctx: ctx as *const VfsContext,
				aux,
			},
		);

		if !p_out_flags.is_null() {
			*p_out_flags = flags;
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
		if path == ctx.actor_id {
			// Main database deletion is unsupported because xDelete cannot remove persisted depot state.
			ctx.set_last_error("main database deletion is unsupported".to_string());
			return SQLITE_IOERR_DELETE;
		} else {
			
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

impl SqliteVfs {
	pub(crate) fn take_last_error(&self) -> Option<String> {
		self.ctx.take_last_error()
	}

	pub(crate) fn take_indeterminate_result(&self) -> Option<String> {
		self.ctx.take_indeterminate_result()
	}

	pub(crate) fn take_transaction_too_large(&self) -> Option<SqliteTransactionTooLargeError> {
		self.ctx.take_transaction_too_large()
	}

	pub(crate) fn take_storage_error(&self) -> Option<SqliteStorageError> {
		self.ctx.take_storage_error()
	}

	fn clone_last_error(&self) -> Option<String> {
		self.ctx.clone_last_error()
	}

	pub fn clone_fatal_error(&self) -> Option<String> {
		self.ctx.clone_fatal_error()
	}

	/// Reads back an auxiliary file this VFS holds in memory. `VACUUM INTO` and other statements
	/// that name an output file resolve it through the connection's VFS, so the only way to get
	/// those bytes onto the host filesystem is to copy them out here.
	pub(crate) fn read_aux_file(&self, path: &str) -> Option<Vec<u8>> {
		self.ctx.read_aux_file(path)
	}

	pub(crate) fn delete_aux_file(&self, path: &str) {
		self.ctx.delete_aux_file(path);
	}

	pub(crate) fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		self.ctx.sqlite_vfs_metrics()
	}

	

	

	pub(crate) fn register_with_transport_and_initial_pages(
		name: &str,
		transport: SqliteTransportHandle,
		actor_id: String,
		runtime: Handle,
		config: VfsConfig,
		initial_pages: InitialPages,
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

		let generation = name
			.rsplit_once("-g")
			.and_then(|(_, generation)| generation.parse::<u64>().ok());
		let mut ctx = Box::new(VfsContext::new(
			actor_id,
			generation,
			runtime,
			transport,
			config,
			io_methods,
			initial_pages,
			metrics,
		)?);
		tracing::trace!(target: "sqlite_prefetch::trace", event = "open",
            stream_id = %format_args!("sqlite-{:p}", &*ctx), actor_id = %ctx.actor_id, generation = ctx.generation);
		let ctx_ptr = (&mut *ctx) as *mut VfsContext;
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

		let registration = SqliteVfsRegistration::register(vfs)?;

		Ok(Self {
			_registration: registration,
			_name: name_cstring,
			ctx,
		})
	}

	pub fn name_ptr(&self) -> *const c_char {
		self._name.as_ptr()
	}

	/// Bytes held by the VFS prefetch buffer and committed-page buffer, which hold only durable
	/// copies.
	pub fn cached_page_bytes(&self) -> u64 {
		let state = self.ctx.state.read();
		state
			.prefetch_buffer
			.entry_count()
			.saturating_add(state.committed_buffer.entry_count())
			.saturating_mul(state.page_size as u64)
	}

	/// How long the worker waits without a command before releasing this database's caches.
	pub fn idle_release(&self) -> Option<std::time::Duration> {
		match self.ctx.config.idle_release_ms {
			0 => None,
			ms => Some(std::time::Duration::from_millis(ms)),
		}
	}

	

	fn ctx(&self) -> &VfsContext {
		&self.ctx
	}

	fn commit_atomic_count(&self) -> u64 {
		self.ctx.commit_atomic_count.load(Ordering::Relaxed)
	}
}

impl SqliteVfsRegistration {
	fn register(vfs: sqlite3_vfs) -> std::result::Result<Self, String> {
		let vfs_ptr = Box::into_raw(Box::new(vfs));
		let rc = unsafe { sqlite3_vfs_register(vfs_ptr, 0) };
		if rc != SQLITE_OK {
			unsafe {
				drop(Box::from_raw(vfs_ptr));
			}
			return Err(format!("sqlite3_vfs_register failed with code {rc}"));
		}

		Ok(Self { vfs_ptr })
	}
}

impl Drop for SqliteVfsRegistration {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs_ptr);
			drop(Box::from_raw(self.vfs_ptr));
		}
	}
}

impl NativeDatabase {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.db
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self._vfs.take_last_error()
	}

	pub fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		self._vfs.ctx.sqlite_vfs_metrics()
	}

	pub fn round_trip_counts(&self) -> SqliteRoundTripCounts {
		self._vfs.ctx.round_trip_counts()
	}

	pub(crate) fn begin_operation_profile(&self) -> SqliteOperationProfileGuard<'_> {
		self._vfs.ctx.begin_operation_profile();
		SqliteOperationProfileGuard {
			ctx: &self._vfs.ctx,
			active: true,
		}
	}

	/// Bytes held by the VFS page buffers, which hold only durable copies.
	pub fn cached_page_bytes(&self) -> u64 {
		self._vfs.cached_page_bytes()
	}

	/// Frees the pager's unpinned pages and the VFS page buffers for an idle database. Returns
	/// false without releasing while a write is buffered; the caller must also skip the release
	/// while an explicit transaction is open. Every released page is a durable copy that the
	/// next read fetches again.
	/// Holds every later commit in memory until `uncork_commits`, which sends them to depot as
	/// one commit. The transactions are committed for this connection meanwhile, but none is
	/// durable, so nothing may report them done until uncorking succeeds.
	pub fn cork_commits(&self) {
		self._vfs.ctx().cork();
	}

	/// Sends the transactions committed since `cork_commits` to depot as one commit. Must run
	/// outside a transaction.
	/// Sends every commit made since `cork_commits` to depot as one commit.
	///
	/// Fails while the connection has an open transaction, because its pages
	/// may still sit in SQLite's page cache where the VFS cannot see them.
	pub fn uncork_commits(&self) -> Result<()> {
		// SAFETY: `as_ptr` is a live connection owned by this database.
		let autocommit = unsafe { sqlite3_get_autocommit(self.as_ptr()) };
		if autocommit == 0 {
			anyhow::bail!("cannot uncork sqlite commits inside a transaction");
		}
		self._vfs
			.ctx()
			.uncork()
			.map_err(|error| anyhow::anyhow!("uncork sqlite commits: {error:?}"))
	}

	pub fn release_idle_memory(&self) -> bool {
		{
			let state = self._vfs.ctx.state.read();
			if state.write_buffer.in_atomic_write
				|| !state.write_buffer.dirty.is_empty()
				|| state.corked.is_some()
			{
				return false;
			}
		}
		unsafe {
			sqlite3_db_release_memory(self.db);
		}
		self._vfs.ctx.state.write().invalidate_page_buffers();
		self._vfs.ctx.report_prefetch_unused();
		true
	}
}

impl Drop for NativeDatabase {
	fn drop(&mut self) {
		if !self.db.is_null() {
			let ctx = self._vfs.ctx();
			ctx.discard_refused_transaction();
			// A transaction still open at close never committed, so the flush below must carry
			// none of its spilled pages. SQLite rolls it back as the connection closes.
			if write_transaction_open(self.db) {
				ctx.discard_open_transaction();
			}
			// Corked pages join the write buffer, so the flush below carries them.
			{
				let mut state = ctx.state.write();
				if let Some(mut corked) = state.corked.take() {
					corked.append(&mut state.write_buffer.dirty);
					state.write_buffer.dirty = corked;
				}
			}
			let should_flush = {
				let state = ctx.state.read();
				state.write_buffer.in_atomic_write
					|| !state.write_buffer.dirty.is_empty()
					|| state.db_size_pages != state.committed_db_size_pages
			};
			if should_flush {
				let result = if ctx.state.read().write_buffer.in_atomic_write {
					ctx.commit_atomic_write_with_timeout(Some(NATIVE_DATABASE_DROP_FLUSH_TIMEOUT))
				} else {
					ctx.flush_dirty_pages_with_timeout(Some(NATIVE_DATABASE_DROP_FLUSH_TIMEOUT))
						.map(|wait| match wait {
							CommitWait::Completed(_) => CommitWait::Completed(()),
							CommitWait::TimedOut => CommitWait::TimedOut,
						})
				};
				// A flush that fails or runs out of time leaves the connection with commits it
				// cannot send. The connection still closes: keeping it open would leak SQLite's
				// connection and its share of the process-wide page cache, and the close below
				// must not try to send the same pages again.
				match result {
					Ok(CommitWait::Completed(())) => {}
					Ok(CommitWait::TimedOut) => {
						let discarded_pages = ctx.discard_unsent_commits();
						tracing::error!(
							actor_id = %ctx.actor_id,
							timeout_ms = NATIVE_DATABASE_DROP_FLUSH_TIMEOUT.as_millis(),
							discarded_pages,
							"timed out flushing sqlite database before close, discarding its unsent commits"
						);
					}
					Err(err) => {
						handle_non_finalize_commit_error(ctx, &err);
						let discarded_pages = ctx.discard_unsent_commits();
						tracing::error!(
							actor_id = %ctx.actor_id,
							?err,
							discarded_pages,
							"failed to flush sqlite database before close, discarding its unsent commits"
						);
					}
				}
			}

			// Every committed transaction is now in Depot or discarded. SQLite's rollback of an
			// open transaction during close only restores that state, so nothing it writes is
			// sent. Every statement the connection's callers prepare is finalized before their
			// call returns, so the close is never deferred by a statement nobody owns.
			ctx.state.write().closing = true;
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

/// Whether the connection holds a write transaction SQLite has not committed: an explicit
/// transaction after `BEGIN`, or a statement whose implicit transaction has not finished.
fn write_transaction_open(db: *mut sqlite3) -> bool {
	// SAFETY: `db` is a live connection owned by the caller.
	let autocommit = unsafe { sqlite3_get_autocommit(db) };
	let txn_state = unsafe { sqlite3_txn_state(db, ptr::null()) };
	autocommit == 0 || txn_state == SQLITE_TXN_WRITE
}

pub fn open_database(vfs: SqliteVfs, file_name: &str) -> Result<NativeDatabase> {
	open_connection(
		Arc::new(vfs),
		file_name,
		SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
	)
	.map_err(anyhow::Error::msg)
	.and_then(|connection| {
		configure_connection_for_database(connection.as_ptr(), &connection._vfs, file_name)?;
		Ok(connection)
	})
}

pub fn open_connection(
	vfs: NativeVfsHandle,
	file_name: &str,
	flags: c_int,
) -> std::result::Result<NativeConnection, String> {
	let c_name = CString::new(file_name).map_err(|err| err.to_string())?;
	let mut db: *mut sqlite3 = ptr::null_mut();

	let rc = unsafe { sqlite3_open_v2(c_name.as_ptr(), &mut db, flags, vfs.name_ptr()) };
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

	Ok(NativeDatabase { db, _vfs: vfs })
}

#[derive(rivet_error::RivetError)]
#[error(
	"sqlite",
	"invalid_compile_options",
	"SQLite requires SQLITE_ENABLE_BATCH_ATOMIC_WRITE and SQLITE_DIRECT_OVERFLOW_READ=0. Rebuild with LIBSQLITE3_FLAGS=\"SQLITE_ENABLE_BATCH_ATOMIC_WRITE SQLITE_DIRECT_OVERFLOW_READ=0\"."
)]
struct SqliteInvalidCompileOptions;

pub fn configure_connection_for_database(
	db: *mut sqlite3,
	vfs: &SqliteVfs,
	file_name: &str,
) -> Result<()> {
	// SAFETY: SQLite only reads its static compile-option table using these C strings.
	let batch_atomic = unsafe { sqlite3_compileoption_used(c"ENABLE_BATCH_ATOMIC_WRITE".as_ptr()) };
	let direct_overflow = unsafe { sqlite3_compileoption_used(c"DIRECT_OVERFLOW_READ".as_ptr()) };
	if batch_atomic == 0 || direct_overflow != 0 {
		return Err(SqliteInvalidCompileOptions.build());
	}

	// SQLite interprets a negative cache_size as a KiB budget instead of a page count. With the
	// process-wide pager cache installed, cache_size reaches the cache only through `xCachesize`,
	// which ignores it: the process budget bounds the pages, and SQLite spills a transaction's
	// dirty pages only when the cache refuses a `createFlag` 1 fetch. cache_size then matters only
	// for SQLite's temporary-file flush heuristic, so it keeps its per-connection value.
	let cache_size_kib = sqlite_optimization_flags().pager_cache_size_kib;
	let cache_size_pragma = format!("PRAGMA cache_size = -{cache_size_kib};");

	let pragmas = [
		"PRAGMA page_size = 4096;",
		"PRAGMA journal_mode = DELETE;",
		"PRAGMA synchronous = NORMAL;",
		"PRAGMA temp_store = MEMORY;",
		"PRAGMA auto_vacuum = NONE;",
		"PRAGMA locking_mode = EXCLUSIVE;",
		// An exclusive connection keeps its journal between transactions. Truncating it after
		// each one releases the memory a large transaction's journal held.
		"PRAGMA journal_size_limit = 0;",
		cache_size_pragma.as_str(),
	];
	for pragma in &pragmas {
		if let Err(err) = sqlite_exec(db, pragma) {
			tracing::error!(
				file_name,
				pragma,
				%err,
				last_error = ?vfs.clone_last_error(),
				"failed to configure sqlite database"
			);
			return Err(anyhow::Error::msg(err));
		}
	}

	let rc = unsafe { sqlite3_set_authorizer(db, Some(authorize_journal_mode), ptr::null_mut()) };
	if rc != SQLITE_OK {
		anyhow::bail!("failed to install the sqlite authorizer: {rc}");
	}

	Ok(())
}

/// Keeps every connection on the `DELETE` rollback journal the VFS depends on. Commits reach
/// Depot only through the main file's sync, so a WAL journal would not be durable, a `MEMORY`
/// journal could roll back more pages than one commit carries, and `OFF` would let a failed
/// transaction's pages be committed. It also pins `journal_size_limit`, and keeps `synchronous`
/// at a level that syncs the main file on every commit, so a commit's pages never wait in the
/// write buffer where a close could not tell them from a later open transaction's. Reading any
/// of them stays allowed. A guarded connection replaces this authorizer with one that refuses
/// every journal mode change.
extern "C" fn authorize_journal_mode(
	_context: *mut c_void,
	action: c_int,
	first: *const c_char,
	second: *const c_char,
	_database: *const c_char,
	_trigger: *const c_char,
) -> c_int {
	if action != SQLITE_PRAGMA || first.is_null() || second.is_null() {
		return SQLITE_OK;
	}
	let name = unsafe { CStr::from_ptr(first) }.to_string_lossy();
	// The journal's record count resets when the journal is truncated after each transaction,
	// which this limit configures.
	if name.eq_ignore_ascii_case("journal_size_limit") {
		return SQLITE_DENY;
	}
	let value = unsafe { CStr::from_ptr(second) }.to_string_lossy();
	let value = value.trim_matches(|c| matches!(c, '"' | '\''));
	if name.eq_ignore_ascii_case("synchronous") {
		let syncs_every_commit = [
			"normal", "full", "extra", "1", "2", "3", "on", "yes", "true",
		]
		.iter()
		.any(|level| value.eq_ignore_ascii_case(level));
		return if syncs_every_commit {
			SQLITE_OK
		} else {
			SQLITE_DENY
		};
	}
	if !name.eq_ignore_ascii_case("journal_mode") {
		return SQLITE_OK;
	}
	if value.eq_ignore_ascii_case("delete") {
		SQLITE_OK
	} else {
		SQLITE_DENY
	}
}
