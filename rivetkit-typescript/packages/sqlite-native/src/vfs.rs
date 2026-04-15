//! Custom SQLite VFS backed by KV operations over the KV channel.
//!
//! This crate now owns the KV-backed SQLite behavior used by `rivetkit-native`.

use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use libsqlite3_sys::*;
use serde::Serialize;
use tokio::runtime::Handle;

use crate::kv;
use crate::sqlite_kv::{
	KvGetResult, SqliteFastPathFence, SqliteKv, SqliteKvError, SqlitePageUpdate,
	SqliteWriteBatchRequest,
};

// MARK: Panic Guard

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
	if let Some(s) = payload.downcast_ref::<&str>() {
		s.to_string()
	} else if let Some(s) = payload.downcast_ref::<String>() {
		s.clone()
	} else {
		"unknown panic".to_string()
	}
}

macro_rules! vfs_catch_unwind {
	($err_val:expr, $body:expr) => {
		match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
			Ok(result) => result,
			Err(panic) => {
				tracing::error!(message = panic_message(&panic), "vfs callback panicked");
				$err_val
			}
		}
	};
}

// MARK: Constants

/// File metadata version for KV-backed SQLite storage.
const META_VERSION: u16 = 1;

/// Encoded metadata size. This is 2 bytes of version plus 8 bytes of size.
const META_ENCODED_SIZE: usize = 10;

/// Maximum pathname length reported to SQLite.
const MAX_PATHNAME: c_int = 64;

/// Maximum number of keys accepted by a single KV put or delete request.
const KV_MAX_BATCH_KEYS: usize = 128;

/// Opt-in flag for the native read cache. Disabled by default to match the WASM VFS.
const READ_CACHE_ENV_VAR: &str = "RIVETKIT_SQLITE_NATIVE_READ_CACHE";

/// First 108 bytes of a valid empty page-1 SQLite database.
///
/// This is the canonical empty page-1 header for the KV-backed SQLite VFS.
const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

fn empty_db_page() -> Vec<u8> {
	let mut page = vec![0u8; kv::CHUNK_SIZE];
	page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()].copy_from_slice(&EMPTY_DB_PAGE_HEADER_PREFIX);
	page
}

// MARK: Metadata Encoding

pub fn encode_file_meta(size: i64) -> Vec<u8> {
	let mut buf = Vec::with_capacity(META_ENCODED_SIZE);
	buf.extend_from_slice(&META_VERSION.to_le_bytes());
	buf.extend_from_slice(&(size as u64).to_le_bytes());
	buf
}

pub fn decode_file_meta(data: &[u8]) -> Option<i64> {
	if data.len() < META_ENCODED_SIZE {
		return None;
	}
	let version_bytes: [u8; 2] = data[0..2].try_into().ok()?;
	if u16::from_le_bytes(version_bytes) != META_VERSION {
		return None;
	}
	let size_bytes: [u8; 8] = data[2..10].try_into().ok()?;
	let size = u64::from_le_bytes(size_bytes);
	if size > i64::MAX as u64 {
		return None;
	}
	Some(size as i64)
}

fn is_valid_file_size(size: i64) -> bool {
	size >= 0 && (size as u64) <= kv::MAX_FILE_SIZE
}

fn read_cache_enabled() -> bool {
	static READ_CACHE_ENABLED: OnceLock<bool> = OnceLock::new();

	*READ_CACHE_ENABLED.get_or_init(|| {
		std::env::var(READ_CACHE_ENV_VAR)
			.map(|value| {
				matches!(
					value.to_ascii_lowercase().as_str(),
					"1" | "true" | "yes" | "on"
				)
			})
			.unwrap_or(false)
	})
}

type StartupPreloadEntries = Vec<(Vec<u8>, Vec<u8>)>;

fn sort_startup_preload(entries: &mut StartupPreloadEntries) {
	entries.sort_by(|a, b| a.0.cmp(&b.0));
}

fn startup_preload_search(entries: &StartupPreloadEntries, key: &[u8]) -> Result<usize, usize> {
	entries.binary_search_by(|(candidate, _)| candidate.as_slice().cmp(key))
}

fn startup_preload_get<'a>(entries: &'a StartupPreloadEntries, key: &[u8]) -> Option<&'a [u8]> {
	startup_preload_search(entries, key)
		.ok()
		.map(|idx| entries[idx].1.as_slice())
}

fn startup_preload_put(entries: &mut StartupPreloadEntries, key: &[u8], value: &[u8]) {
	if let Ok(idx) = startup_preload_search(entries, key) {
		entries[idx].1 = value.to_vec();
	}
}

fn startup_preload_delete(entries: &mut StartupPreloadEntries, key: &[u8]) {
	if let Ok(idx) = startup_preload_search(entries, key) {
		entries.remove(idx);
	}
}

fn startup_preload_delete_range(entries: &mut StartupPreloadEntries, start: &[u8], end: &[u8]) {
	entries.retain(|(key, _)| key.as_slice() < start || key.as_slice() >= end);
}

// MARK: VFS Metrics

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsReadTelemetry {
	pub count: u64,
	pub duration_us: u64,
	pub requested_bytes: u64,
	pub returned_bytes: u64,
	pub short_read_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsWriteTelemetry {
	pub count: u64,
	pub duration_us: u64,
	pub input_bytes: u64,
	pub buffered_count: u64,
	pub buffered_bytes: u64,
	pub immediate_kv_put_count: u64,
	pub immediate_kv_put_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsSyncTelemetry {
	pub count: u64,
	pub duration_us: u64,
	pub metadata_flush_count: u64,
	pub metadata_flush_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsAtomicWriteTelemetry {
	pub begin_count: u64,
	pub commit_attempt_count: u64,
	pub commit_success_count: u64,
	pub commit_duration_us: u64,
	pub committed_dirty_pages_total: u64,
	pub max_committed_dirty_pages: u64,
	pub committed_buffered_bytes_total: u64,
	pub rollback_count: u64,
	pub fast_path_attempt_count: u64,
	pub fast_path_success_count: u64,
	pub fast_path_fallback_count: u64,
	pub fast_path_failure_count: u64,
	pub batch_cap_failure_count: u64,
	pub commit_kv_put_failure_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsKvTelemetry {
	pub get_count: u64,
	pub get_duration_us: u64,
	pub get_key_count: u64,
	pub get_bytes: u64,
	pub put_count: u64,
	pub put_duration_us: u64,
	pub put_key_count: u64,
	pub put_bytes: u64,
	pub delete_count: u64,
	pub delete_duration_us: u64,
	pub delete_key_count: u64,
	pub delete_range_count: u64,
	pub delete_range_duration_us: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsTelemetrySnapshot {
	pub reads: VfsReadTelemetry,
	pub writes: VfsWriteTelemetry,
	pub syncs: VfsSyncTelemetry,
	pub atomic_write: VfsAtomicWriteTelemetry,
	pub kv: VfsKvTelemetry,
}

fn update_max(counter: &AtomicU64, value: u64) {
	let mut current = counter.load(Ordering::Relaxed);
	while value > current {
		match counter.compare_exchange(current, value, Ordering::Relaxed, Ordering::Relaxed) {
			Ok(_) => break,
			Err(previous) => current = previous,
		}
	}
}

fn reset_counter(counter: &AtomicU64) {
	counter.store(0, Ordering::Relaxed);
}

/// Per-VFS-callback operation metrics for diagnosing native SQLite VFS performance.
pub struct VfsMetrics {
	pub xread_count: AtomicU64,
	pub xread_us: AtomicU64,
	pub xread_requested_bytes: AtomicU64,
	pub xread_returned_bytes: AtomicU64,
	pub xread_short_read_count: AtomicU64,
	pub xwrite_count: AtomicU64,
	pub xwrite_us: AtomicU64,
	pub xwrite_input_bytes: AtomicU64,
	pub xwrite_buffered_count: AtomicU64,
	pub xwrite_buffered_bytes: AtomicU64,
	pub xwrite_immediate_kv_put_count: AtomicU64,
	pub xwrite_immediate_kv_put_bytes: AtomicU64,
	pub xsync_count: AtomicU64,
	pub xsync_us: AtomicU64,
	pub xsync_metadata_flush_count: AtomicU64,
	pub xsync_metadata_flush_bytes: AtomicU64,
	pub begin_atomic_count: AtomicU64,
	pub commit_atomic_attempt_count: AtomicU64,
	pub commit_atomic_success_count: AtomicU64,
	pub commit_atomic_us: AtomicU64,
	pub commit_atomic_pages: AtomicU64,
	pub commit_atomic_max_pages: AtomicU64,
	pub commit_atomic_bytes: AtomicU64,
	pub rollback_atomic_count: AtomicU64,
	pub commit_atomic_fast_path_attempt_count: AtomicU64,
	pub commit_atomic_fast_path_success_count: AtomicU64,
	pub commit_atomic_fast_path_fallback_count: AtomicU64,
	pub commit_atomic_fast_path_failure_count: AtomicU64,
	pub commit_atomic_batch_cap_failure_count: AtomicU64,
	pub commit_atomic_kv_put_failure_count: AtomicU64,
	pub kv_get_count: AtomicU64,
	pub kv_get_us: AtomicU64,
	pub kv_get_keys: AtomicU64,
	pub kv_get_bytes: AtomicU64,
	pub kv_put_count: AtomicU64,
	pub kv_put_us: AtomicU64,
	pub kv_put_keys: AtomicU64,
	pub kv_put_bytes: AtomicU64,
	pub kv_delete_count: AtomicU64,
	pub kv_delete_us: AtomicU64,
	pub kv_delete_keys: AtomicU64,
	pub kv_delete_range_count: AtomicU64,
	pub kv_delete_range_us: AtomicU64,
}

impl VfsMetrics {
	pub fn new() -> Self {
		Self {
			xread_count: AtomicU64::new(0),
			xread_us: AtomicU64::new(0),
			xread_requested_bytes: AtomicU64::new(0),
			xread_returned_bytes: AtomicU64::new(0),
			xread_short_read_count: AtomicU64::new(0),
			xwrite_count: AtomicU64::new(0),
			xwrite_us: AtomicU64::new(0),
			xwrite_input_bytes: AtomicU64::new(0),
			xwrite_buffered_count: AtomicU64::new(0),
			xwrite_buffered_bytes: AtomicU64::new(0),
			xwrite_immediate_kv_put_count: AtomicU64::new(0),
			xwrite_immediate_kv_put_bytes: AtomicU64::new(0),
			xsync_count: AtomicU64::new(0),
			xsync_us: AtomicU64::new(0),
			xsync_metadata_flush_count: AtomicU64::new(0),
			xsync_metadata_flush_bytes: AtomicU64::new(0),
			begin_atomic_count: AtomicU64::new(0),
			commit_atomic_attempt_count: AtomicU64::new(0),
			commit_atomic_success_count: AtomicU64::new(0),
			commit_atomic_us: AtomicU64::new(0),
			commit_atomic_pages: AtomicU64::new(0),
			commit_atomic_max_pages: AtomicU64::new(0),
			commit_atomic_bytes: AtomicU64::new(0),
			rollback_atomic_count: AtomicU64::new(0),
			commit_atomic_fast_path_attempt_count: AtomicU64::new(0),
			commit_atomic_fast_path_success_count: AtomicU64::new(0),
			commit_atomic_fast_path_fallback_count: AtomicU64::new(0),
			commit_atomic_fast_path_failure_count: AtomicU64::new(0),
			commit_atomic_batch_cap_failure_count: AtomicU64::new(0),
			commit_atomic_kv_put_failure_count: AtomicU64::new(0),
			kv_get_count: AtomicU64::new(0),
			kv_get_us: AtomicU64::new(0),
			kv_get_keys: AtomicU64::new(0),
			kv_get_bytes: AtomicU64::new(0),
			kv_put_count: AtomicU64::new(0),
			kv_put_us: AtomicU64::new(0),
			kv_put_keys: AtomicU64::new(0),
			kv_put_bytes: AtomicU64::new(0),
			kv_delete_count: AtomicU64::new(0),
			kv_delete_us: AtomicU64::new(0),
			kv_delete_keys: AtomicU64::new(0),
			kv_delete_range_count: AtomicU64::new(0),
			kv_delete_range_us: AtomicU64::new(0),
		}
	}

	pub fn snapshot(&self) -> VfsTelemetrySnapshot {
		VfsTelemetrySnapshot {
			reads: VfsReadTelemetry {
				count: self.xread_count.load(Ordering::Relaxed),
				duration_us: self.xread_us.load(Ordering::Relaxed),
				requested_bytes: self.xread_requested_bytes.load(Ordering::Relaxed),
				returned_bytes: self.xread_returned_bytes.load(Ordering::Relaxed),
				short_read_count: self.xread_short_read_count.load(Ordering::Relaxed),
			},
			writes: VfsWriteTelemetry {
				count: self.xwrite_count.load(Ordering::Relaxed),
				duration_us: self.xwrite_us.load(Ordering::Relaxed),
				input_bytes: self.xwrite_input_bytes.load(Ordering::Relaxed),
				buffered_count: self.xwrite_buffered_count.load(Ordering::Relaxed),
				buffered_bytes: self.xwrite_buffered_bytes.load(Ordering::Relaxed),
				immediate_kv_put_count: self.xwrite_immediate_kv_put_count.load(Ordering::Relaxed),
				immediate_kv_put_bytes: self.xwrite_immediate_kv_put_bytes.load(Ordering::Relaxed),
			},
			syncs: VfsSyncTelemetry {
				count: self.xsync_count.load(Ordering::Relaxed),
				duration_us: self.xsync_us.load(Ordering::Relaxed),
				metadata_flush_count: self.xsync_metadata_flush_count.load(Ordering::Relaxed),
				metadata_flush_bytes: self.xsync_metadata_flush_bytes.load(Ordering::Relaxed),
			},
			atomic_write: VfsAtomicWriteTelemetry {
				begin_count: self.begin_atomic_count.load(Ordering::Relaxed),
				commit_attempt_count: self.commit_atomic_attempt_count.load(Ordering::Relaxed),
				commit_success_count: self.commit_atomic_success_count.load(Ordering::Relaxed),
				commit_duration_us: self.commit_atomic_us.load(Ordering::Relaxed),
				committed_dirty_pages_total: self.commit_atomic_pages.load(Ordering::Relaxed),
				max_committed_dirty_pages: self.commit_atomic_max_pages.load(Ordering::Relaxed),
				committed_buffered_bytes_total: self.commit_atomic_bytes.load(Ordering::Relaxed),
				rollback_count: self.rollback_atomic_count.load(Ordering::Relaxed),
				fast_path_attempt_count: self
					.commit_atomic_fast_path_attempt_count
					.load(Ordering::Relaxed),
				fast_path_success_count: self
					.commit_atomic_fast_path_success_count
					.load(Ordering::Relaxed),
				fast_path_fallback_count: self
					.commit_atomic_fast_path_fallback_count
					.load(Ordering::Relaxed),
				fast_path_failure_count: self
					.commit_atomic_fast_path_failure_count
					.load(Ordering::Relaxed),
				batch_cap_failure_count: self
					.commit_atomic_batch_cap_failure_count
					.load(Ordering::Relaxed),
				commit_kv_put_failure_count: self
					.commit_atomic_kv_put_failure_count
					.load(Ordering::Relaxed),
			},
			kv: VfsKvTelemetry {
				get_count: self.kv_get_count.load(Ordering::Relaxed),
				get_duration_us: self.kv_get_us.load(Ordering::Relaxed),
				get_key_count: self.kv_get_keys.load(Ordering::Relaxed),
				get_bytes: self.kv_get_bytes.load(Ordering::Relaxed),
				put_count: self.kv_put_count.load(Ordering::Relaxed),
				put_duration_us: self.kv_put_us.load(Ordering::Relaxed),
				put_key_count: self.kv_put_keys.load(Ordering::Relaxed),
				put_bytes: self.kv_put_bytes.load(Ordering::Relaxed),
				delete_count: self.kv_delete_count.load(Ordering::Relaxed),
				delete_duration_us: self.kv_delete_us.load(Ordering::Relaxed),
				delete_key_count: self.kv_delete_keys.load(Ordering::Relaxed),
				delete_range_count: self.kv_delete_range_count.load(Ordering::Relaxed),
				delete_range_duration_us: self.kv_delete_range_us.load(Ordering::Relaxed),
			},
		}
	}

	pub fn reset(&self) {
		reset_counter(&self.xread_count);
		reset_counter(&self.xread_us);
		reset_counter(&self.xread_requested_bytes);
		reset_counter(&self.xread_returned_bytes);
		reset_counter(&self.xread_short_read_count);
		reset_counter(&self.xwrite_count);
		reset_counter(&self.xwrite_us);
		reset_counter(&self.xwrite_input_bytes);
		reset_counter(&self.xwrite_buffered_count);
		reset_counter(&self.xwrite_buffered_bytes);
		reset_counter(&self.xwrite_immediate_kv_put_count);
		reset_counter(&self.xwrite_immediate_kv_put_bytes);
		reset_counter(&self.xsync_count);
		reset_counter(&self.xsync_us);
		reset_counter(&self.xsync_metadata_flush_count);
		reset_counter(&self.xsync_metadata_flush_bytes);
		reset_counter(&self.begin_atomic_count);
		reset_counter(&self.commit_atomic_attempt_count);
		reset_counter(&self.commit_atomic_success_count);
		reset_counter(&self.commit_atomic_us);
		reset_counter(&self.commit_atomic_pages);
		reset_counter(&self.commit_atomic_max_pages);
		reset_counter(&self.commit_atomic_bytes);
		reset_counter(&self.rollback_atomic_count);
		reset_counter(&self.commit_atomic_fast_path_attempt_count);
		reset_counter(&self.commit_atomic_fast_path_success_count);
		reset_counter(&self.commit_atomic_fast_path_fallback_count);
		reset_counter(&self.commit_atomic_fast_path_failure_count);
		reset_counter(&self.commit_atomic_batch_cap_failure_count);
		reset_counter(&self.commit_atomic_kv_put_failure_count);
		reset_counter(&self.kv_get_count);
		reset_counter(&self.kv_get_us);
		reset_counter(&self.kv_get_keys);
		reset_counter(&self.kv_get_bytes);
		reset_counter(&self.kv_put_count);
		reset_counter(&self.kv_put_us);
		reset_counter(&self.kv_put_keys);
		reset_counter(&self.kv_put_bytes);
		reset_counter(&self.kv_delete_count);
		reset_counter(&self.kv_delete_us);
		reset_counter(&self.kv_delete_keys);
		reset_counter(&self.kv_delete_range_count);
		reset_counter(&self.kv_delete_range_us);
	}
}

// MARK: VFS Context

#[derive(Clone, Copy, Debug, Default)]
struct SqliteFastPathFenceTracker {
	last_committed_fence: Option<u64>,
	next_request_fence: u64,
}

struct VfsContext {
	kv: Arc<dyn SqliteKv>,
	actor_id: String,
	main_file_name: String,
	// Bounded startup entries shipped with actor start. This is not the opt-in read cache.
	startup_preload: Mutex<Option<StartupPreloadEntries>>,
	fast_path_fences: Mutex<BTreeMap<u8, SqliteFastPathFenceTracker>>,
	read_cache_enabled: bool,
	last_error: Mutex<Option<String>>,
	rt_handle: Handle,
	io_methods: Box<sqlite3_io_methods>,
	vfs_metrics: Arc<VfsMetrics>,
}

impl VfsContext {
	fn clear_last_error(&self) {
		match self.last_error.lock() {
			Ok(mut last_error) => {
				*last_error = None;
			}
			Err(err) => {
				tracing::warn!(%err, "native sqlite last_error mutex poisoned");
			}
		}
	}

	fn set_last_error(&self, message: String) {
		match self.last_error.lock() {
			Ok(mut last_error) => {
				*last_error = Some(message);
			}
			Err(err) => {
				tracing::warn!(%err, "native sqlite last_error mutex poisoned");
			}
		}
	}

	fn clone_last_error(&self) -> Option<String> {
		match self.last_error.lock() {
			Ok(last_error) => last_error.clone(),
			Err(err) => {
				tracing::warn!(%err, "native sqlite last_error mutex poisoned");
				None
			}
		}
	}

	fn take_last_error(&self) -> Option<String> {
		match self.last_error.lock() {
			Ok(mut last_error) => last_error.take(),
			Err(err) => {
				tracing::warn!(%err, "native sqlite last_error mutex poisoned");
				None
			}
		}
	}

	fn report_kv_error(&self, err: SqliteKvError) -> String {
		let message = err.to_string();
		self.set_last_error(message.clone());
		self.kv.on_error(&self.actor_id, &err);
		message
	}

	fn snapshot_vfs_telemetry(&self) -> VfsTelemetrySnapshot {
		self.vfs_metrics.snapshot()
	}

	fn reset_vfs_telemetry(&self) {
		self.vfs_metrics.reset();
		self.clear_last_error();
	}

	fn resolve_file_tag(&self, path: &str) -> Option<u8> {
		if path == self.main_file_name {
			return Some(kv::FILE_TAG_MAIN);
		}

		if let Some(suffix) = path.strip_prefix(&self.main_file_name) {
			match suffix {
				"-journal" => Some(kv::FILE_TAG_JOURNAL),
				"-wal" => Some(kv::FILE_TAG_WAL),
				"-shm" => Some(kv::FILE_TAG_SHM),
				_ => None,
			}
		} else {
			None
		}
	}

	fn update_startup_preload(&self, f: impl FnOnce(&mut StartupPreloadEntries)) {
		if let Ok(mut guard) = self.startup_preload.lock() {
			if let Some(entries) = guard.as_mut() {
				f(entries);
			}
		}
	}

	fn sqlite_write_batch_fast_path_supported(&self) -> bool {
		match self
			.rt_handle
			.block_on(self.kv.sqlite_fast_path_capability(&self.actor_id))
		{
			Ok(Some(capability)) => capability.supports_write_batch,
			Ok(None) => false,
			Err(err) => {
				tracing::warn!(%err, "failed to resolve sqlite fast path capability");
				false
			}
		}
	}

	fn reserve_sqlite_fast_path_fence(&self, file_tag: u8) -> SqliteFastPathFence {
		let mut fences = self
			.fast_path_fences
			.lock()
			.expect("sqlite fast path fence mutex poisoned");
		let tracker = fences
			.entry(file_tag)
			.or_insert_with(|| SqliteFastPathFenceTracker {
				last_committed_fence: None,
				next_request_fence: 1,
			});
		let request_fence = tracker.next_request_fence.max(1);
		tracker.next_request_fence = request_fence.saturating_add(1);
		SqliteFastPathFence {
			expected_fence: tracker.last_committed_fence,
			request_fence,
		}
	}

	fn mark_sqlite_fast_path_committed(&self, file_tag: u8, request_fence: u64) {
		let mut fences = self
			.fast_path_fences
			.lock()
			.expect("sqlite fast path fence mutex poisoned");
		let tracker = fences.entry(file_tag).or_default();
		tracker.last_committed_fence = Some(request_fence);
		if tracker.next_request_fence <= request_fence {
			tracker.next_request_fence = request_fence.saturating_add(1);
		}
	}

	fn clear_sqlite_fast_path_fence(&self, file_tag: u8) {
		if let Ok(mut fences) = self.fast_path_fences.lock() {
			fences.remove(&file_tag);
		}
	}

	fn kv_get(&self, keys: Vec<Vec<u8>>) -> Result<KvGetResult, String> {
		let key_count = keys.len();
		let start = std::time::Instant::now();
		let (preloaded_keys, preloaded_values, miss_keys) =
			if let Ok(guard) = self.startup_preload.lock() {
				if let Some(entries) = guard.as_ref() {
					let mut hit_keys = Vec::new();
					let mut hit_values = Vec::new();
					let mut misses = Vec::new();
					for key in keys {
						if let Some(value) = startup_preload_get(entries, key.as_slice()) {
							hit_keys.push(key);
							hit_values.push(value.to_vec());
						} else {
							misses.push(key);
						}
					}
					(hit_keys, hit_values, misses)
				} else {
					(Vec::new(), Vec::new(), keys)
				}
			} else {
				(Vec::new(), Vec::new(), keys)
			};
		let remote_fetch = !miss_keys.is_empty();
		let remote_key_count = miss_keys.len() as u64;
		if remote_fetch {
			self.vfs_metrics
				.kv_get_count
				.fetch_add(1, Ordering::Relaxed);
			self.vfs_metrics
				.kv_get_keys
				.fetch_add(remote_key_count, Ordering::Relaxed);
		}
		let result = if miss_keys.is_empty() {
			Ok(KvGetResult {
				keys: preloaded_keys,
				values: preloaded_values,
			})
		} else {
			self.rt_handle
				.block_on(self.kv.batch_get(&self.actor_id, miss_keys))
				.map(|mut result| {
					let fetched_bytes = result
						.values
						.iter()
						.map(|value| value.len() as u64)
						.sum::<u64>();
					self.vfs_metrics
						.kv_get_bytes
						.fetch_add(fetched_bytes, Ordering::Relaxed);
					result.keys.extend(preloaded_keys);
					result.values.extend(preloaded_values);
					result
				})
				.map_err(|err| self.report_kv_error(err))
		};
		if result.is_ok() {
			self.clear_last_error();
		}
		let elapsed = start.elapsed();
		if remote_fetch {
			self.vfs_metrics
				.kv_get_us
				.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		}
		tracing::debug!(
			op = %format_args!("get({key_count}keys)"),
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_put(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<(), String> {
		let key_count = keys.len();
		let start = std::time::Instant::now();
		let put_bytes = values.iter().map(|value| value.len() as u64).sum::<u64>();
		self.vfs_metrics
			.kv_put_count
			.fetch_add(1, Ordering::Relaxed);
		self.vfs_metrics
			.kv_put_keys
			.fetch_add(key_count as u64, Ordering::Relaxed);
		self.vfs_metrics
			.kv_put_bytes
			.fetch_add(put_bytes, Ordering::Relaxed);
		let result = self
			.rt_handle
			.block_on(
				self.kv
					.batch_put(&self.actor_id, keys.clone(), values.clone()),
			)
			.map_err(|err| self.report_kv_error(err));
		if result.is_ok() {
			self.clear_last_error();
			self.update_startup_preload(|entries| {
				for (key, value) in keys.iter().zip(values.iter()) {
					startup_preload_put(entries, key.as_slice(), value.as_slice());
				}
			});
		}
		let elapsed = start.elapsed();
		self.vfs_metrics
			.kv_put_us
			.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		tracing::debug!(
			op = %format_args!("put({key_count}keys)"),
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_delete(&self, keys: Vec<Vec<u8>>) -> Result<(), String> {
		let key_count = keys.len();
		let start = std::time::Instant::now();
		self.vfs_metrics
			.kv_delete_count
			.fetch_add(1, Ordering::Relaxed);
		self.vfs_metrics
			.kv_delete_keys
			.fetch_add(key_count as u64, Ordering::Relaxed);
		let result = self
			.rt_handle
			.block_on(self.kv.batch_delete(&self.actor_id, keys.clone()))
			.map_err(|err| self.report_kv_error(err));
		if result.is_ok() {
			self.clear_last_error();
			self.update_startup_preload(|entries| {
				for key in &keys {
					startup_preload_delete(entries, key.as_slice());
				}
			});
		}
		let elapsed = start.elapsed();
		self.vfs_metrics
			.kv_delete_us
			.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		tracing::debug!(
			op = %format_args!("del({key_count}keys)"),
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> Result<(), String> {
		let start_time = std::time::Instant::now();
		let preload_start = start.clone();
		let preload_end = end.clone();
		self.vfs_metrics
			.kv_delete_range_count
			.fetch_add(1, Ordering::Relaxed);
		let result = self
			.rt_handle
			.block_on(self.kv.delete_range(&self.actor_id, start, end))
			.map_err(|err| self.report_kv_error(err));
		if result.is_ok() {
			self.clear_last_error();
			self.update_startup_preload(|entries| {
				startup_preload_delete_range(
					entries,
					preload_start.as_slice(),
					preload_end.as_slice(),
				);
			});
		}
		let elapsed = start_time.elapsed();
		self.vfs_metrics
			.kv_delete_range_us
			.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		tracing::debug!(
			op = "delRange",
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn delete_file(&self, file_tag: u8) -> Result<(), String> {
		let meta_key = kv::get_meta_key(file_tag);
		self.kv_delete(vec![meta_key.to_vec()])?;
		self.kv_delete_range(
			kv::get_chunk_key(file_tag, 0).to_vec(),
			kv::get_chunk_key_range_end(file_tag).to_vec(),
		)?;
		self.clear_sqlite_fast_path_fence(file_tag);
		Ok(())
	}
}

// MARK: File State

struct AtomicWriteSnapshot {
	file_size: i64,
	meta_dirty: bool,
	dirty_buffer: BTreeMap<u32, Vec<u8>>,
	pending_delete_start: Option<u32>,
}

struct BufferedFlushResult {
	dirty_page_count: u64,
	dirty_buffer_bytes: u64,
}

struct KvFileState {
	batch_mode: bool,
	dirty_buffer: BTreeMap<u32, Vec<u8>>,
	pending_delete_start: Option<u32>,
	atomic_snapshot: Option<AtomicWriteSnapshot>,
	/// Read cache: maps chunk keys to their data. Populated on KV gets,
	/// updated on writes, cleared on truncate/delete. This avoids
	/// redundant KV round-trips for pages SQLite reads multiple times.
	read_cache: Option<HashMap<Vec<u8>, Vec<u8>>>,
}

impl KvFileState {
	fn new(read_cache_enabled: bool) -> Self {
		Self {
			batch_mode: false,
			dirty_buffer: BTreeMap::new(),
			pending_delete_start: None,
			atomic_snapshot: None,
			read_cache: read_cache_enabled.then(HashMap::new),
		}
	}
}

#[repr(C)]
struct KvFile {
	base: sqlite3_file,
	ctx: *const VfsContext,
	state: *mut KvFileState,
	file_tag: u8,
	meta_key: [u8; 4],
	size: i64,
	meta_dirty: bool,
	flags: c_int,
}

// MARK: Helpers

unsafe fn get_file(p: *mut sqlite3_file) -> &'static mut KvFile {
	&mut *(p as *mut KvFile)
}

unsafe fn get_file_state(state: *mut KvFileState) -> &'static mut KvFileState {
	&mut *state
}

unsafe fn free_file_state(file: &mut KvFile) {
	if !file.state.is_null() {
		drop(Box::from_raw(file.state));
		file.state = ptr::null_mut();
	}
}

unsafe fn get_vfs_ctx(p: *mut sqlite3_vfs) -> &'static VfsContext {
	&*((*p).pAppData as *const VfsContext)
}

fn build_value_map(resp: &KvGetResult) -> HashMap<&[u8], &[u8]> {
	resp.keys
		.iter()
		.zip(resp.values.iter())
		.filter(|(_, value)| !value.is_empty())
		.map(|(key, value)| (key.as_slice(), value.as_slice()))
		.collect()
}

fn split_entries(entries: Vec<(Vec<u8>, Vec<u8>)>) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
	let mut keys = Vec::with_capacity(entries.len());
	let mut values = Vec::with_capacity(entries.len());
	for (key, value) in entries {
		keys.push(key);
		values.push(value);
	}
	(keys, values)
}

fn build_sqlite_page_updates(state: &KvFileState) -> Vec<SqlitePageUpdate> {
	state
		.dirty_buffer
		.iter()
		.map(|(chunk_index, data)| SqlitePageUpdate {
			chunk_index: *chunk_index,
			data: data.clone(),
		})
		.collect()
}

fn chunk_is_logically_deleted(state: &KvFileState, chunk_idx: u32) -> bool {
	state
		.pending_delete_start
		.map(|start| chunk_idx >= start)
		.unwrap_or(false)
}

fn logical_chunk_len(file: &KvFile, state: &KvFileState, chunk_idx: u32) -> usize {
	if let Some(buffered) = state.dirty_buffer.get(&chunk_idx) {
		return buffered.len();
	}
	if chunk_is_logically_deleted(state, chunk_idx) {
		return 0;
	}

	let chunk_offset = chunk_idx as usize * kv::CHUNK_SIZE;
	let file_size = file.size.max(0) as usize;
	if file_size <= chunk_offset {
		0
	} else {
		std::cmp::min(kv::CHUNK_SIZE, file_size - chunk_offset)
	}
}

fn trim_read_cache_for_truncate(file: &KvFile, state: &mut KvFileState, delete_start_chunk: u32) {
	if let Some(read_cache) = state.read_cache.as_mut() {
		read_cache.retain(|key, _| {
			if key.len() == 8 && key[3] == file.file_tag {
				let chunk_idx = u32::from_be_bytes([key[4], key[5], key[6], key[7]]);
				chunk_idx < delete_start_chunk
			} else {
				true
			}
		});
	}
}

fn load_visible_chunk(
	file: &KvFile,
	state: &KvFileState,
	ctx: &VfsContext,
	chunk_idx: u32,
) -> Result<Option<Vec<u8>>, String> {
	if let Some(buffered) = state.dirty_buffer.get(&chunk_idx) {
		return Ok(Some(buffered.clone()));
	}
	if chunk_is_logically_deleted(state, chunk_idx) {
		return Ok(None);
	}

	let chunk_key = kv::get_chunk_key(file.file_tag, chunk_idx);
	if let Some(read_cache) = state.read_cache.as_ref() {
		if let Some(cached) = read_cache.get(chunk_key.as_slice()) {
			return Ok(Some(cached.clone()));
		}
	}

	let resp = ctx.kv_get(vec![chunk_key.to_vec()])?;
	let value_map = build_value_map(&resp);
	Ok(value_map
		.get(chunk_key.as_slice())
		.map(|value| value.to_vec()))
}

fn apply_flush_to_startup_preload(file: &KvFile, state: &KvFileState, ctx: &VfsContext) {
	let meta_value = encode_file_meta(file.size);
	ctx.update_startup_preload(|entries| {
		if let Some(delete_start_chunk) = state.pending_delete_start {
			startup_preload_delete_range(
				entries,
				kv::get_chunk_key(file.file_tag, delete_start_chunk).as_slice(),
				kv::get_chunk_key_range_end(file.file_tag).as_slice(),
			);
		}
		for (chunk_index, data) in &state.dirty_buffer {
			let chunk_key = kv::get_chunk_key(file.file_tag, *chunk_index);
			startup_preload_put(entries, chunk_key.as_slice(), data.as_slice());
		}
		startup_preload_put(entries, file.meta_key.as_slice(), meta_value.as_slice());
	});
}

fn apply_flush_to_read_cache(file: &KvFile, state: &mut KvFileState) {
	if let Some(read_cache) = state.read_cache.as_mut() {
		if let Some(delete_start_chunk) = state.pending_delete_start {
			read_cache.retain(|key, _| {
				if key.len() == 8 && key[3] == file.file_tag {
					let chunk_idx = u32::from_be_bytes([key[4], key[5], key[6], key[7]]);
					chunk_idx < delete_start_chunk
				} else {
					true
				}
			});
		}
		for (chunk_index, data) in &state.dirty_buffer {
			let key = kv::get_chunk_key(file.file_tag, *chunk_index);
			read_cache.insert(key.to_vec(), data.clone());
		}
	}
}

fn finish_buffered_flush(
	file: &mut KvFile,
	state: &mut KvFileState,
	ctx: &VfsContext,
	dirty_page_count: u64,
	dirty_buffer_bytes: u64,
) -> BufferedFlushResult {
	apply_flush_to_startup_preload(file, state, ctx);
	apply_flush_to_read_cache(file, state);
	state.dirty_buffer.clear();
	state.pending_delete_start = None;
	file.meta_dirty = false;

	BufferedFlushResult {
		dirty_page_count,
		dirty_buffer_bytes,
	}
}

fn try_flush_buffered_file_fast_path(
	file: &mut KvFile,
	state: &mut KvFileState,
	ctx: &VfsContext,
	dirty_page_count: u64,
	dirty_buffer_bytes: u64,
) -> Result<Option<BufferedFlushResult>, String> {
	if dirty_page_count == 0
		|| state.pending_delete_start.is_some()
		|| !ctx.sqlite_write_batch_fast_path_supported()
	{
		return Ok(None);
	}

	let fence = ctx.reserve_sqlite_fast_path_fence(file.file_tag);
	ctx.vfs_metrics
		.commit_atomic_fast_path_attempt_count
		.fetch_add(1, Ordering::Relaxed);
	let request = SqliteWriteBatchRequest {
		file_tag: file.file_tag,
		meta_value: encode_file_meta(file.size),
		page_updates: build_sqlite_page_updates(state),
		fence,
	};

	if let Err(err) = ctx
		.rt_handle
		.block_on(ctx.kv.sqlite_write_batch(&ctx.actor_id, request))
	{
		ctx.vfs_metrics
			.commit_atomic_fast_path_failure_count
			.fetch_add(1, Ordering::Relaxed);
		return Err(ctx.report_kv_error(err));
	}

	ctx.clear_last_error();
	ctx.mark_sqlite_fast_path_committed(file.file_tag, fence.request_fence);
	ctx.vfs_metrics
		.commit_atomic_fast_path_success_count
		.fetch_add(1, Ordering::Relaxed);
	Ok(Some(finish_buffered_flush(
		file,
		state,
		ctx,
		dirty_page_count,
		dirty_buffer_bytes,
	)))
}

fn flush_buffered_file(
	file: &mut KvFile,
	state: &mut KvFileState,
	ctx: &VfsContext,
) -> Result<BufferedFlushResult, String> {
	let dirty_page_count = state.dirty_buffer.len() as u64;
	let dirty_buffer_bytes = state
		.dirty_buffer
		.values()
		.map(|value| value.len() as u64)
		.sum::<u64>();

	if let Some(result) =
		try_flush_buffered_file_fast_path(file, state, ctx, dirty_page_count, dirty_buffer_bytes)?
	{
		return Ok(result);
	}
	if dirty_page_count > 0 {
		ctx.vfs_metrics
			.commit_atomic_fast_path_fallback_count
			.fetch_add(1, Ordering::Relaxed);
	}

	if let Some(delete_start_chunk) = state.pending_delete_start {
		ctx.kv_delete_range(
			kv::get_chunk_key(file.file_tag, delete_start_chunk).to_vec(),
			kv::get_chunk_key_range_end(file.file_tag).to_vec(),
		)?;
	}

	let flushed_entries: Vec<_> = state
		.dirty_buffer
		.iter()
		.map(|(chunk_index, data)| {
			(
				kv::get_chunk_key(file.file_tag, *chunk_index).to_vec(),
				data.clone(),
			)
		})
		.collect();
	for chunk in flushed_entries.chunks(KV_MAX_BATCH_KEYS) {
		let (keys, values) = split_entries(chunk.to_vec());
		ctx.kv_put(keys, values)?;
	}

	if file.meta_dirty {
		ctx.kv_put(
			vec![file.meta_key.to_vec()],
			vec![encode_file_meta(file.size)],
		)?;
	}

	ctx.clear_sqlite_fast_path_fence(file.file_tag);

	Ok(finish_buffered_flush(
		file,
		state,
		ctx,
		dirty_page_count,
		dirty_buffer_bytes,
	))
}

// MARK: IO Callbacks

unsafe extern "C" fn kv_io_close(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let state = get_file_state(file.state);

		let result = if file.flags & SQLITE_OPEN_DELETEONCLOSE != 0 {
			ctx.delete_file(file.file_tag)
		} else if file.meta_dirty
			|| state.pending_delete_start.is_some()
			|| !state.dirty_buffer.is_empty()
		{
			flush_buffered_file(file, state, ctx).map(|_| ())
		} else {
			Ok(())
		};

		free_file_state(file);

		match result {
			Ok(()) => SQLITE_OK,
			Err(err) => {
				tracing::error!(%err, file_tag = file.file_tag, "failed to close file");
				SQLITE_IOERR
			}
		}
	})
}

unsafe extern "C" fn kv_io_read(
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
		let state = get_file_state(file.state);
		let ctx = &*file.ctx;
		let read_start = std::time::Instant::now();
		ctx.vfs_metrics.xread_count.fetch_add(1, Ordering::Relaxed);
		let requested_length = i_amt as usize;
		ctx.vfs_metrics
			.xread_requested_bytes
			.fetch_add(requested_length as u64, Ordering::Relaxed);
		let buf = slice::from_raw_parts_mut(p_buf as *mut u8, requested_length);

		if i_offset < 0 {
			return SQLITE_IOERR_READ;
		}

		let offset = i_offset as usize;
		let file_size = file.size as usize;
		if offset >= file_size {
			buf.fill(0);
			ctx.vfs_metrics
				.xread_short_read_count
				.fetch_add(1, Ordering::Relaxed);
			ctx.vfs_metrics
				.xread_us
				.fetch_add(read_start.elapsed().as_micros() as u64, Ordering::Relaxed);
			return SQLITE_IOERR_SHORT_READ;
		}

		let start_chunk = offset / kv::CHUNK_SIZE;
		let end_chunk = (offset + requested_length - 1) / kv::CHUNK_SIZE;

		let mut chunk_keys_to_fetch = Vec::new();
		let mut buffered_chunks: HashMap<usize, &[u8]> = HashMap::new();
		// Skip fetching chunks already present in the dirty buffer or read cache.
		for chunk_idx in start_chunk..=end_chunk {
			if let Some(buffered) = state.dirty_buffer.get(&(chunk_idx as u32)) {
				buffered_chunks.insert(chunk_idx, buffered.as_slice());
				continue;
			}
			if chunk_is_logically_deleted(state, chunk_idx as u32) {
				continue;
			}
			let key = kv::get_chunk_key(file.file_tag, chunk_idx as u32);
			if let Some(read_cache) = state.read_cache.as_ref() {
				if let Some(cached) = read_cache.get(key.as_slice()) {
					buffered_chunks.insert(chunk_idx, cached.as_slice());
					continue;
				}
			}
			chunk_keys_to_fetch.push(key.to_vec());
		}

		let resp = if chunk_keys_to_fetch.is_empty() {
			KvGetResult {
				keys: Vec::new(),
				values: Vec::new(),
			}
		} else {
			match ctx.kv_get(chunk_keys_to_fetch) {
				Ok(resp) => resp,
				Err(_) => return SQLITE_IOERR_READ,
			}
		};
		let value_map = build_value_map(&resp);

		for chunk_idx in start_chunk..=end_chunk {
			let chunk_data = buffered_chunks.get(&chunk_idx).copied().or_else(|| {
				let chunk_key = kv::get_chunk_key(file.file_tag, chunk_idx as u32);
				value_map.get(chunk_key.as_slice()).copied()
			});
			let chunk_offset = chunk_idx * kv::CHUNK_SIZE;
			let read_start = offset.saturating_sub(chunk_offset);
			let read_end = std::cmp::min(kv::CHUNK_SIZE, offset + requested_length - chunk_offset);
			let dest_start = chunk_offset + read_start - offset;

			if let Some(chunk_data) = chunk_data {
				let source_end = std::cmp::min(read_end, chunk_data.len());
				if source_end > read_start {
					let dest_end = dest_start + (source_end - read_start);
					buf[dest_start..dest_end].copy_from_slice(&chunk_data[read_start..source_end]);
				}
				if source_end < read_end {
					let zero_start = dest_start + (source_end - read_start);
					let zero_end = dest_start + (read_end - read_start);
					buf[zero_start..zero_end].fill(0);
				}
			} else {
				let dest_end = dest_start + (read_end - read_start);
				buf[dest_start..dest_end].fill(0);
			}
		}

		// `resp` is empty when every chunk was served from the dirty buffer,
		// logical truncate state, or read cache.
		// In that case this loop is a no-op.
		if let Some(read_cache) = state.read_cache.as_mut() {
			for (key, value) in resp.keys.iter().zip(resp.values.iter()) {
				if !value.is_empty() {
					read_cache.insert(key.clone(), value.clone());
				}
			}
		}

		let actual_bytes = std::cmp::min(requested_length, file_size - offset);
		ctx.vfs_metrics
			.xread_returned_bytes
			.fetch_add(actual_bytes as u64, Ordering::Relaxed);
		if actual_bytes < requested_length {
			buf[actual_bytes..].fill(0);
			ctx.vfs_metrics
				.xread_short_read_count
				.fetch_add(1, Ordering::Relaxed);
			ctx.vfs_metrics
				.xread_us
				.fetch_add(read_start.elapsed().as_micros() as u64, Ordering::Relaxed);
			return SQLITE_IOERR_SHORT_READ;
		}

		ctx.vfs_metrics
			.xread_us
			.fetch_add(read_start.elapsed().as_micros() as u64, Ordering::Relaxed);
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_write(
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
		let write_start = std::time::Instant::now();
		ctx.vfs_metrics.xwrite_count.fetch_add(1, Ordering::Relaxed);
		let data = slice::from_raw_parts(p_buf as *const u8, i_amt as usize);
		ctx.vfs_metrics
			.xwrite_input_bytes
			.fetch_add(data.len() as u64, Ordering::Relaxed);

		if i_offset < 0 {
			return SQLITE_IOERR_WRITE;
		}

		let offset = i_offset as usize;
		let write_length = i_amt as usize;
		let write_end_offset = match offset.checked_add(write_length) {
			Some(end) => end,
			None => return SQLITE_IOERR_WRITE,
		};
		if write_end_offset as u64 > kv::MAX_FILE_SIZE {
			return SQLITE_IOERR_WRITE;
		}

		let start_chunk = offset / kv::CHUNK_SIZE;
		let end_chunk = (offset + write_length - 1) / kv::CHUNK_SIZE;

		struct WritePlan {
			chunk_index: u32,
			chunk_offset: usize,
			write_start: usize,
			write_end: usize,
			buffered_chunk: Option<Vec<u8>>,
			cached_chunk: Option<Vec<u8>>,
			existing_chunk_index: Option<usize>,
		}

		let mut plans = Vec::new();
		let mut chunk_keys_to_fetch = Vec::new();
		let state = get_file_state(file.state);
		for chunk_idx in start_chunk..=end_chunk {
			let chunk_offset = chunk_idx * kv::CHUNK_SIZE;
			let write_start = offset.saturating_sub(chunk_offset);
			let write_end = std::cmp::min(kv::CHUNK_SIZE, offset + write_length - chunk_offset);
			let chunk_index = chunk_idx as u32;
			let buffered_chunk = state.dirty_buffer.get(&chunk_index).cloned();
			let logically_deleted = chunk_is_logically_deleted(state, chunk_index);
			let existing_bytes_in_chunk = logical_chunk_len(file, state, chunk_index);
			let needs_existing = write_start > 0 || existing_bytes_in_chunk > write_end;
			let chunk_key = kv::get_chunk_key(file.file_tag, chunk_index).to_vec();
			let cached_chunk = if needs_existing && buffered_chunk.is_none() && !logically_deleted {
				state
					.read_cache
					.as_ref()
					.and_then(|read_cache| read_cache.get(chunk_key.as_slice()).cloned())
			} else {
				None
			};
			let existing_chunk_index = if needs_existing
				&& buffered_chunk.is_none()
				&& cached_chunk.is_none()
				&& !logically_deleted
			{
				let idx = chunk_keys_to_fetch.len();
				chunk_keys_to_fetch.push(chunk_key.clone());
				Some(idx)
			} else {
				None
			};

			plans.push(WritePlan {
				chunk_index,
				chunk_offset,
				write_start,
				write_end,
				buffered_chunk,
				cached_chunk,
				existing_chunk_index,
			});
		}

		let existing_chunks = if chunk_keys_to_fetch.is_empty() {
			Vec::new()
		} else {
			match ctx.kv_get(chunk_keys_to_fetch.clone()) {
				Ok(resp) => {
					let value_map = build_value_map(&resp);
					chunk_keys_to_fetch
						.iter()
						.map(|key| value_map.get(key.as_slice()).map(|value| value.to_vec()))
						.collect::<Vec<_>>()
				}
				Err(_) => return SQLITE_IOERR_WRITE,
			}
		};

		let mut buffered_writes = Vec::with_capacity(plans.len());
		for plan in &plans {
			let existing_chunk = plan
				.buffered_chunk
				.as_deref()
				.or(plan.cached_chunk.as_deref())
				.or_else(|| {
					plan.existing_chunk_index
						.and_then(|idx| existing_chunks.get(idx))
						.and_then(|value| value.as_deref())
				});

			let mut new_chunk = if let Some(existing_chunk) = existing_chunk {
				let mut chunk = vec![0u8; std::cmp::max(existing_chunk.len(), plan.write_end)];
				chunk[..existing_chunk.len()].copy_from_slice(existing_chunk);
				chunk
			} else {
				vec![0u8; plan.write_end]
			};

			let source_start = plan.chunk_offset + plan.write_start - offset;
			let source_end = source_start + (plan.write_end - plan.write_start);
			new_chunk[plan.write_start..plan.write_end]
				.copy_from_slice(&data[source_start..source_end]);

			buffered_writes.push((plan.chunk_index, new_chunk));
		}

		let new_size = std::cmp::max(file.size, write_end_offset as i64);
		if new_size != file.size {
			file.size = new_size;
			file.meta_dirty = true;
		}

		for (chunk_index, new_chunk) in buffered_writes {
			state.dirty_buffer.insert(chunk_index, new_chunk);
		}

		ctx.vfs_metrics
			.xwrite_buffered_count
			.fetch_add(1, Ordering::Relaxed);
		ctx.vfs_metrics
			.xwrite_buffered_bytes
			.fetch_add(data.len() as u64, Ordering::Relaxed);

		ctx.vfs_metrics
			.xwrite_us
			.fetch_add(write_start.elapsed().as_micros() as u64, Ordering::Relaxed);
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_truncate(p_file: *mut sqlite3_file, size: sqlite3_int64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_TRUNCATE, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let state = get_file_state(file.state);

		if size < 0 || size as u64 > kv::MAX_FILE_SIZE {
			return SQLITE_IOERR_TRUNCATE;
		}

		if size >= file.size {
			if size > file.size {
				file.size = size;
				file.meta_dirty = true;
			}
			return SQLITE_OK;
		}

		let delete_start_chunk = (size as usize / kv::CHUNK_SIZE) as u32;
		let truncated_tail = if size > 0 && size as usize % kv::CHUNK_SIZE != 0 {
			let truncated_len = size as usize % kv::CHUNK_SIZE;
			match load_visible_chunk(file, state, ctx, delete_start_chunk) {
				Ok(existing_chunk) => {
					let mut truncated_chunk =
						existing_chunk.unwrap_or_else(|| vec![0u8; truncated_len]);
					truncated_chunk.truncate(truncated_len);
					Some((delete_start_chunk, truncated_chunk))
				}
				Err(_) => return SQLITE_IOERR_TRUNCATE,
			}
		} else {
			None
		};

		trim_read_cache_for_truncate(file, state, delete_start_chunk);
		state
			.dirty_buffer
			.retain(|chunk_index, _| *chunk_index < delete_start_chunk);
		if let Some((chunk_index, truncated_chunk)) = truncated_tail {
			state.dirty_buffer.insert(chunk_index, truncated_chunk);
		}
		state.pending_delete_start = Some(
			state
				.pending_delete_start
				.map(|existing| existing.min(delete_start_chunk))
				.unwrap_or(delete_start_chunk),
		);
		file.size = size;
		file.meta_dirty = true;

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let state = get_file_state(file.state);
		let sync_start = std::time::Instant::now();
		ctx.vfs_metrics.xsync_count.fetch_add(1, Ordering::Relaxed);
		if !file.meta_dirty && state.pending_delete_start.is_none() && state.dirty_buffer.is_empty()
		{
			ctx.vfs_metrics
				.xsync_us
				.fetch_add(sync_start.elapsed().as_micros() as u64, Ordering::Relaxed);
			return SQLITE_OK;
		}

		ctx.vfs_metrics
			.xsync_metadata_flush_count
			.fetch_add(1, Ordering::Relaxed);
		ctx.vfs_metrics
			.xsync_metadata_flush_bytes
			.fetch_add(META_ENCODED_SIZE as u64, Ordering::Relaxed);
		if flush_buffered_file(file, state, ctx).is_err() {
			ctx.vfs_metrics
				.xsync_us
				.fetch_add(sync_start.elapsed().as_micros() as u64, Ordering::Relaxed);
			return SQLITE_IOERR_FSYNC;
		}
		ctx.vfs_metrics
			.xsync_us
			.fetch_add(sync_start.elapsed().as_micros() as u64, Ordering::Relaxed);

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_file_size(
	p_file: *mut sqlite3_file,
	p_size: *mut sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSTAT, {
		let file = get_file(p_file);
		*p_size = file.size;
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_lock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_LOCK, SQLITE_OK)
}

unsafe extern "C" fn kv_io_unlock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_UNLOCK, SQLITE_OK)
}

unsafe extern "C" fn kv_io_check_reserved_lock(
	_p_file: *mut sqlite3_file,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		*p_res_out = 0;
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_file_control(
	p_file: *mut sqlite3_file,
	op: c_int,
	_p_arg: *mut c_void,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		if file.state.is_null() {
			return SQLITE_NOTFOUND;
		}
		let state = get_file_state(file.state);

		match op {
			SQLITE_FCNTL_BEGIN_ATOMIC_WRITE => {
				let ctx = &*file.ctx;
				ctx.vfs_metrics
					.begin_atomic_count
					.fetch_add(1, Ordering::Relaxed);
				state.atomic_snapshot = Some(AtomicWriteSnapshot {
					file_size: file.size,
					meta_dirty: file.meta_dirty,
					dirty_buffer: state.dirty_buffer.clone(),
					pending_delete_start: state.pending_delete_start,
				});
				state.batch_mode = true;
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
				let ctx = &*file.ctx;
				let commit_start = std::time::Instant::now();
				ctx.vfs_metrics
					.commit_atomic_attempt_count
					.fetch_add(1, Ordering::Relaxed);
				let flush_result = flush_buffered_file(file, state, ctx);
				let BufferedFlushResult {
					dirty_page_count,
					dirty_buffer_bytes,
				} = match flush_result {
					Ok(result) => result,
					Err(_) => {
						ctx.vfs_metrics
							.commit_atomic_kv_put_failure_count
							.fetch_add(1, Ordering::Relaxed);
						ctx.vfs_metrics.commit_atomic_us.fetch_add(
							commit_start.elapsed().as_micros() as u64,
							Ordering::Relaxed,
						);
						return SQLITE_IOERR;
					}
				};
				state.batch_mode = false;
				state.atomic_snapshot = None;
				ctx.vfs_metrics
					.commit_atomic_success_count
					.fetch_add(1, Ordering::Relaxed);
				ctx.vfs_metrics
					.commit_atomic_pages
					.fetch_add(dirty_page_count, Ordering::Relaxed);
				update_max(&ctx.vfs_metrics.commit_atomic_max_pages, dirty_page_count);
				ctx.vfs_metrics
					.commit_atomic_bytes
					.fetch_add(dirty_buffer_bytes, Ordering::Relaxed);
				ctx.vfs_metrics
					.commit_atomic_us
					.fetch_add(commit_start.elapsed().as_micros() as u64, Ordering::Relaxed);
				SQLITE_OK
			}
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				if !state.batch_mode {
					return SQLITE_OK;
				}
				let ctx = &*file.ctx;
				ctx.vfs_metrics
					.rollback_atomic_count
					.fetch_add(1, Ordering::Relaxed);
				if let Some(snapshot) = state.atomic_snapshot.take() {
					state.dirty_buffer = snapshot.dirty_buffer;
					state.pending_delete_start = snapshot.pending_delete_start;
					file.size = snapshot.file_size;
					file.meta_dirty = snapshot.meta_dirty;
				}
				state.batch_mode = false;
				SQLITE_OK
			}
			_ => SQLITE_NOTFOUND,
		}
	})
}

unsafe extern "C" fn kv_io_sector_size(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(kv::CHUNK_SIZE as c_int, kv::CHUNK_SIZE as c_int)
}

unsafe extern "C" fn kv_io_device_characteristics(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(0, SQLITE_IOCAP_BATCH_ATOMIC)
}

// MARK: VFS Callbacks

unsafe extern "C" fn kv_vfs_open(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	p_file: *mut sqlite3_file,
	flags: c_int,
	p_out_flags: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_CANTOPEN, {
		if z_name.is_null() {
			return SQLITE_CANTOPEN;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(path) => path,
			Err(_) => return SQLITE_CANTOPEN,
		};
		let file_tag = match ctx.resolve_file_tag(path) {
			Some(file_tag) => file_tag,
			None => return SQLITE_CANTOPEN,
		};
		let meta_key = kv::get_meta_key(file_tag);

		let resp = match ctx.kv_get(vec![meta_key.to_vec()]) {
			Ok(resp) => resp,
			Err(_) => return SQLITE_CANTOPEN,
		};
		let value_map = build_value_map(&resp);

		let size = if let Some(size_data) = value_map.get(meta_key.as_slice()) {
			let size = match decode_file_meta(size_data) {
				Some(size) => size,
				None => return SQLITE_IOERR,
			};
			if !is_valid_file_size(size) {
				return SQLITE_IOERR;
			}
			size
		} else if flags & SQLITE_OPEN_CREATE != 0 {
			if file_tag == kv::FILE_TAG_MAIN {
				let size = kv::CHUNK_SIZE as i64;
				let entries = vec![
					(kv::get_chunk_key(file_tag, 0).to_vec(), empty_db_page()),
					(meta_key.to_vec(), encode_file_meta(size)),
				];
				let (keys, values) = split_entries(entries);
				if ctx.kv_put(keys, values).is_err() {
					return SQLITE_CANTOPEN;
				}
				size
			} else {
				let size = 0i64;
				if ctx
					.kv_put(vec![meta_key.to_vec()], vec![encode_file_meta(size)])
					.is_err()
				{
					return SQLITE_CANTOPEN;
				}
				size
			}
		} else {
			return SQLITE_CANTOPEN;
		};

		let state = Box::into_raw(Box::new(KvFileState::new(ctx.read_cache_enabled)));
		let base = sqlite3_file {
			pMethods: ctx.io_methods.as_ref() as *const sqlite3_io_methods,
		};
		ptr::write(
			p_file as *mut KvFile,
			KvFile {
				base,
				ctx: ctx as *const VfsContext,
				state,
				file_tag,
				meta_key,
				size,
				meta_dirty: false,
				flags,
			},
		);

		if !p_out_flags.is_null() {
			*p_out_flags = flags;
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_vfs_delete(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_sync_dir: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_DELETE, {
		if z_name.is_null() {
			return SQLITE_IOERR_DELETE;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(path) => path,
			Err(_) => return SQLITE_IOERR_DELETE,
		};
		let file_tag = match ctx.resolve_file_tag(path) {
			Some(file_tag) => file_tag,
			None => return SQLITE_IOERR_DELETE,
		};

		match ctx.delete_file(file_tag) {
			Ok(()) => SQLITE_OK,
			Err(_) => SQLITE_IOERR_DELETE,
		}
	})
}

unsafe extern "C" fn kv_vfs_access(
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
		let file_tag = match ctx.resolve_file_tag(path) {
			Some(file_tag) => file_tag,
			None => {
				*p_res_out = 0;
				return SQLITE_OK;
			}
		};
		let meta_key = kv::get_meta_key(file_tag);
		let resp = match ctx.kv_get(vec![meta_key.to_vec()]) {
			Ok(resp) => resp,
			Err(_) => return SQLITE_IOERR_ACCESS,
		};
		let value_map = build_value_map(&resp);
		*p_res_out = if value_map.contains_key(meta_key.as_slice()) {
			1
		} else {
			0
		};

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_vfs_full_pathname(
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

		ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, z_out, bytes.len());
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_vfs_randomness(
	_p_vfs: *mut sqlite3_vfs,
	n_byte: c_int,
	z_out: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(0, {
		let buf = slice::from_raw_parts_mut(z_out as *mut u8, n_byte as usize);
		match getrandom::getrandom(buf) {
			Ok(()) => n_byte,
			Err(_) => 0,
		}
	})
}

unsafe extern "C" fn kv_vfs_sleep(_p_vfs: *mut sqlite3_vfs, microseconds: c_int) -> c_int {
	vfs_catch_unwind!(0, {
		std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
		microseconds
	})
}

unsafe extern "C" fn kv_vfs_current_time(_p_vfs: *mut sqlite3_vfs, p_time_out: *mut f64) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default();
		*p_time_out = 2440587.5 + (now.as_secs_f64() / 86400.0);
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_vfs_get_last_error(
	p_vfs: *mut sqlite3_vfs,
	n_byte: c_int,
	z_err_msg: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		if n_byte <= 0 || z_err_msg.is_null() {
			return 0;
		}

		let ctx = get_vfs_ctx(p_vfs);
		let last_error = ctx.clone_last_error();
		let Some(message) = last_error else {
			*z_err_msg = 0;
			return 0;
		};

		let bytes = message.as_bytes();
		let max_len = (n_byte as usize).saturating_sub(1);
		let copy_len = bytes.len().min(max_len);
		let dst = z_err_msg.cast::<u8>();
		ptr::copy_nonoverlapping(bytes.as_ptr(), dst, copy_len);
		*dst.add(copy_len) = 0u8;
		0
	})
}

// MARK: KvVfs

pub struct KvVfs {
	vfs_ptr: *mut sqlite3_vfs,
	_name: CString,
	ctx_ptr: *mut VfsContext,
}

unsafe impl Send for KvVfs {}
unsafe impl Sync for KvVfs {}

impl KvVfs {
	fn take_last_kv_error(&self) -> Option<String> {
		unsafe { (*self.ctx_ptr).take_last_error() }
	}

	pub fn snapshot_vfs_telemetry(&self) -> VfsTelemetrySnapshot {
		unsafe { (*self.ctx_ptr).snapshot_vfs_telemetry() }
	}

	pub fn reset_vfs_telemetry(&self) {
		unsafe {
			(*self.ctx_ptr).reset_vfs_telemetry();
		}
	}

	pub fn register(
		name: &str,
		kv: Arc<dyn SqliteKv>,
		actor_id: String,
		rt_handle: Handle,
		mut startup_preload: StartupPreloadEntries,
	) -> Result<Self, String> {
		let mut io_methods: sqlite3_io_methods = unsafe { std::mem::zeroed() };
		io_methods.iVersion = 1;
		io_methods.xClose = Some(kv_io_close);
		io_methods.xRead = Some(kv_io_read);
		io_methods.xWrite = Some(kv_io_write);
		io_methods.xTruncate = Some(kv_io_truncate);
		io_methods.xSync = Some(kv_io_sync);
		io_methods.xFileSize = Some(kv_io_file_size);
		io_methods.xLock = Some(kv_io_lock);
		io_methods.xUnlock = Some(kv_io_unlock);
		io_methods.xCheckReservedLock = Some(kv_io_check_reserved_lock);
		io_methods.xFileControl = Some(kv_io_file_control);
		io_methods.xSectorSize = Some(kv_io_sector_size);
		io_methods.xDeviceCharacteristics = Some(kv_io_device_characteristics);

		let vfs_metrics = Arc::new(VfsMetrics::new());
		sort_startup_preload(&mut startup_preload);
		let ctx = Box::new(VfsContext {
			kv,
			actor_id: actor_id.clone(),
			main_file_name: actor_id,
			startup_preload: Mutex::new((!startup_preload.is_empty()).then_some(startup_preload)),
			fast_path_fences: Mutex::new(BTreeMap::new()),
			read_cache_enabled: read_cache_enabled(),
			last_error: Mutex::new(None),
			rt_handle,
			io_methods: Box::new(io_methods),
			vfs_metrics,
		});
		let ctx_ptr = Box::into_raw(ctx);

		let name_cstring = CString::new(name).map_err(|err| err.to_string())?;

		let mut vfs: sqlite3_vfs = unsafe { std::mem::zeroed() };
		vfs.iVersion = 1;
		vfs.szOsFile = std::mem::size_of::<KvFile>() as c_int;
		vfs.mxPathname = MAX_PATHNAME;
		vfs.zName = name_cstring.as_ptr();
		vfs.pAppData = ctx_ptr as *mut c_void;
		vfs.xOpen = Some(kv_vfs_open);
		vfs.xDelete = Some(kv_vfs_delete);
		vfs.xAccess = Some(kv_vfs_access);
		vfs.xFullPathname = Some(kv_vfs_full_pathname);
		vfs.xRandomness = Some(kv_vfs_randomness);
		vfs.xSleep = Some(kv_vfs_sleep);
		vfs.xCurrentTime = Some(kv_vfs_current_time);
		vfs.xGetLastError = Some(kv_vfs_get_last_error);

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
}

impl Drop for KvVfs {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs_ptr);
			drop(Box::from_raw(self.vfs_ptr));
			drop(Box::from_raw(self.ctx_ptr));
		}
	}
}

// MARK: NativeDatabase

pub struct NativeDatabase {
	db: *mut sqlite3,
	_vfs: KvVfs,
}

unsafe impl Send for NativeDatabase {}

impl NativeDatabase {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.db
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self._vfs.take_last_kv_error()
	}

	pub fn snapshot_vfs_telemetry(&self) -> VfsTelemetrySnapshot {
		self._vfs.snapshot_vfs_telemetry()
	}

	pub fn reset_vfs_telemetry(&self) {
		self._vfs.reset_vfs_telemetry();
	}
}

impl Drop for NativeDatabase {
	fn drop(&mut self) {
		if !self.db.is_null() {
			unsafe {
				sqlite3_close(self.db);
			}
		}
	}
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

pub fn open_database(vfs: KvVfs, file_name: &str) -> Result<NativeDatabase, String> {
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
		let c_sql = CString::new(*pragma).map_err(|err| err.to_string())?;
		let rc =
			unsafe { sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut()) };
		if rc != SQLITE_OK {
			let message = sqlite_error_message(db);
			unsafe {
				sqlite3_close(db);
			}
			return Err(format!("{pragma} failed with code {rc}: {message}"));
		}
	}

	Ok(NativeDatabase { db, _vfs: vfs })
}

// MARK: Tests

#[cfg(test)]
mod tests {
	use super::*;
	use async_trait::async_trait;

	#[test]
	fn encode_decode_round_trip() {
		for size in [0i64, 1, 4096, 1_000_000, i64::MAX / 2] {
			let encoded = encode_file_meta(size);
			assert_eq!(encoded.len(), META_ENCODED_SIZE);
			assert_eq!(&encoded[0..2], &META_VERSION.to_le_bytes());
			let decoded = decode_file_meta(&encoded).unwrap();
			assert_eq!(decoded, size);
		}
	}

	#[test]
	fn encode_zero_size() {
		let encoded = encode_file_meta(0);
		assert_eq!(encoded, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	}

	#[test]
	fn encode_known_size() {
		let encoded = encode_file_meta(4096);
		assert_eq!(
			encoded,
			[1, 0, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
		);
	}

	#[test]
	fn decode_invalid_version() {
		let data = [2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
		assert!(decode_file_meta(&data).is_none());
	}

	#[test]
	fn decode_too_short() {
		assert!(decode_file_meta(&[]).is_none());
		assert!(decode_file_meta(&[1]).is_none());
		assert!(decode_file_meta(&[1, 0]).is_none());
		assert!(decode_file_meta(&[1, 0, 0, 0, 0]).is_none());
	}

	#[test]
	fn kv_file_struct_is_larger_than_sqlite3_file() {
		assert!(std::mem::size_of::<KvFile>() > std::mem::size_of::<sqlite3_file>());
	}

	#[test]
	fn meta_encoded_size_constant() {
		assert_eq!(META_ENCODED_SIZE, 10);
	}

	#[test]
	fn meta_version_matches_wasm_vfs() {
		assert_eq!(META_VERSION, 1);
	}

	#[test]
	fn encode_matches_vbare_format() {
		let encoded = encode_file_meta(42);
		assert_eq!(encoded[0], 0x01);
		assert_eq!(encoded[1], 0x00);
		assert_eq!(&encoded[2..], &42u64.to_le_bytes());
	}

	#[test]
	fn empty_db_page_matches_generated_prefix() {
		let page = empty_db_page();
		assert_eq!(page.len(), kv::CHUNK_SIZE);
		assert_eq!(
			&page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()],
			&EMPTY_DB_PAGE_HEADER_PREFIX
		);
		assert!(page[EMPTY_DB_PAGE_HEADER_PREFIX.len()..]
			.iter()
			.all(|byte| *byte == 0));
	}

	#[test]
	fn startup_preload_helpers_use_exact_key_matches() {
		let mut entries = vec![
			(vec![3], vec![30]),
			(vec![1], vec![10]),
			(vec![2], vec![20]),
		];
		sort_startup_preload(&mut entries);

		assert_eq!(startup_preload_get(&entries, &[1]), Some(&[10][..]));
		assert_eq!(startup_preload_get(&entries, &[2]), Some(&[20][..]));
		assert_eq!(startup_preload_get(&entries, &[4]), None);
	}

	#[test]
	fn startup_preload_helpers_update_without_growing() {
		let mut entries = vec![(vec![1], vec![10]), (vec![2], vec![20])];
		sort_startup_preload(&mut entries);

		startup_preload_put(&mut entries, &[2], &[99]);
		startup_preload_put(&mut entries, &[3], &[30]);
		startup_preload_delete(&mut entries, &[1]);
		startup_preload_delete(&mut entries, &[7]);

		assert_eq!(entries, vec![(vec![2], vec![99])]);
	}

	#[test]
	fn startup_preload_helpers_delete_range_is_half_open() {
		let mut entries = vec![
			(vec![1], vec![10]),
			(vec![2], vec![20]),
			(vec![3], vec![30]),
			(vec![4], vec![40]),
		];
		sort_startup_preload(&mut entries);

		startup_preload_delete_range(&mut entries, &[2], &[4]);

		assert_eq!(entries, vec![(vec![1], vec![10]), (vec![4], vec![40])]);
	}

	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
	enum FailureOperation {
		BatchPut,
		BatchDelete,
		DeleteRange,
		SqliteWriteBatch,
	}

	struct InjectedFailure {
		op: FailureOperation,
		file_tag: Option<u8>,
		message: String,
	}

	#[derive(Default)]
	struct MemoryKv {
		store: Mutex<BTreeMap<Vec<u8>, Vec<u8>>>,
		failures: Mutex<Vec<InjectedFailure>>,
		sqlite_fast_path_capability: Option<crate::sqlite_kv::SqliteFastPathCapability>,
		sqlite_write_batches: Mutex<Vec<SqliteWriteBatchRequest>>,
	}

	impl MemoryKv {
		fn with_sqlite_write_batch_fast_path() -> Self {
			Self {
				store: Mutex::new(BTreeMap::new()),
				failures: Mutex::new(Vec::new()),
				sqlite_fast_path_capability: Some(crate::sqlite_kv::SqliteFastPathCapability {
					supports_write_batch: true,
					supports_truncate: false,
				}),
				sqlite_write_batches: Mutex::new(Vec::new()),
			}
		}

		fn fail_next_batch_put(&self, message: impl Into<String>) {
			self.failures
				.lock()
				.expect("memory kv failures mutex poisoned")
				.push(InjectedFailure {
					op: FailureOperation::BatchPut,
					file_tag: None,
					message: message.into(),
				});
		}

		fn fail_next_sqlite_write_batch(&self, message: impl Into<String>) {
			self.failures
				.lock()
				.expect("memory kv failures mutex poisoned")
				.push(InjectedFailure {
					op: FailureOperation::SqliteWriteBatch,
					file_tag: None,
					message: message.into(),
				});
		}

		fn recorded_sqlite_write_batches(&self) -> Vec<SqliteWriteBatchRequest> {
			self.sqlite_write_batches
				.lock()
				.expect("memory kv write batch mutex poisoned")
				.clone()
		}

		fn clear_recorded_sqlite_write_batches(&self) {
			self.sqlite_write_batches
				.lock()
				.expect("memory kv write batch mutex poisoned")
				.clear();
		}

		fn maybe_fail_keys(
			&self,
			op: FailureOperation,
			keys: &[Vec<u8>],
		) -> Result<(), SqliteKvError> {
			let mut failures = self
				.failures
				.lock()
				.expect("memory kv failures mutex poisoned");
			if let Some(idx) = failures.iter().position(|failure| {
				failure.op == op
					&& failure.file_tag.map_or(true, |file_tag| {
						keys.iter().any(|key| {
							key.get(3)
								.map(|key_file_tag| *key_file_tag == file_tag)
								.unwrap_or(false)
						})
					})
			}) {
				return Err(SqliteKvError::new(failures.remove(idx).message));
			}
			Ok(())
		}

		fn maybe_fail_range(
			&self,
			op: FailureOperation,
			start: &[u8],
		) -> Result<(), SqliteKvError> {
			let mut failures = self
				.failures
				.lock()
				.expect("memory kv failures mutex poisoned");
			if let Some(idx) = failures.iter().position(|failure| {
				failure.op == op
					&& failure.file_tag.map_or(true, |file_tag| {
						start
							.get(3)
							.map(|start_file_tag| *start_file_tag == file_tag)
							.unwrap_or(false)
					})
			}) {
				return Err(SqliteKvError::new(failures.remove(idx).message));
			}
			Ok(())
		}

		fn maybe_fail_file_tag(
			&self,
			op: FailureOperation,
			file_tag: u8,
		) -> Result<(), SqliteKvError> {
			let mut failures = self
				.failures
				.lock()
				.expect("memory kv failures mutex poisoned");
			if let Some(idx) = failures.iter().position(|failure| {
				failure.op == op
					&& failure
						.file_tag
						.map_or(true, |expected| expected == file_tag)
			}) {
				return Err(SqliteKvError::new(failures.remove(idx).message));
			}
			Ok(())
		}
	}

	#[async_trait]
	impl SqliteKv for MemoryKv {
		async fn sqlite_fast_path_capability(
			&self,
			_actor_id: &str,
		) -> Result<Option<crate::sqlite_kv::SqliteFastPathCapability>, SqliteKvError> {
			Ok(self.sqlite_fast_path_capability)
		}

		async fn batch_get(
			&self,
			_actor_id: &str,
			keys: Vec<Vec<u8>>,
		) -> Result<KvGetResult, SqliteKvError> {
			let store = self.store.lock().expect("memory kv mutex poisoned");
			let mut found_keys = Vec::new();
			let mut found_values = Vec::new();
			for key in keys {
				if let Some(value) = store.get(&key) {
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
			_actor_id: &str,
			keys: Vec<Vec<u8>>,
			values: Vec<Vec<u8>>,
		) -> Result<(), SqliteKvError> {
			self.maybe_fail_keys(FailureOperation::BatchPut, &keys)?;
			let mut store = self.store.lock().expect("memory kv mutex poisoned");
			for (key, value) in keys.into_iter().zip(values.into_iter()) {
				store.insert(key, value);
			}
			Ok(())
		}

		async fn sqlite_write_batch(
			&self,
			_actor_id: &str,
			request: SqliteWriteBatchRequest,
		) -> Result<(), SqliteKvError> {
			self.maybe_fail_file_tag(FailureOperation::SqliteWriteBatch, request.file_tag)?;
			let mut store = self.store.lock().expect("memory kv mutex poisoned");
			for page in &request.page_updates {
				store.insert(
					kv::get_chunk_key(request.file_tag, page.chunk_index).to_vec(),
					page.data.clone(),
				);
			}
			store.insert(
				kv::get_meta_key(request.file_tag).to_vec(),
				request.meta_value.clone(),
			);
			drop(store);
			self.sqlite_write_batches
				.lock()
				.expect("memory kv write batch mutex poisoned")
				.push(request);
			Ok(())
		}

		async fn batch_delete(
			&self,
			_actor_id: &str,
			keys: Vec<Vec<u8>>,
		) -> Result<(), SqliteKvError> {
			self.maybe_fail_keys(FailureOperation::BatchDelete, &keys)?;
			let mut store = self.store.lock().expect("memory kv mutex poisoned");
			for key in keys {
				store.remove(&key);
			}
			Ok(())
		}

		async fn delete_range(
			&self,
			_actor_id: &str,
			start: Vec<u8>,
			end: Vec<u8>,
		) -> Result<(), SqliteKvError> {
			self.maybe_fail_range(FailureOperation::DeleteRange, &start)?;
			let mut store = self.store.lock().expect("memory kv mutex poisoned");
			store.retain(|key, _| {
				key.as_slice() < start.as_slice() || key.as_slice() >= end.as_slice()
			});
			Ok(())
		}
	}

	static NEXT_TEST_VFS_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

	fn open_database_with_kv(
		file_name: &str,
		kv: Arc<dyn SqliteKv>,
	) -> (tokio::runtime::Runtime, NativeDatabase) {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let vfs_id = NEXT_TEST_VFS_ID.fetch_add(1, Ordering::Relaxed);
		let vfs = KvVfs::register(
			&format!("test-vfs-{file_name}-{vfs_id}"),
			kv,
			file_name.to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let db = open_database(vfs, file_name).expect("open test database");
		(runtime, db)
	}

	fn open_memory_database(
		file_name: &str,
	) -> (tokio::runtime::Runtime, Arc<MemoryKv>, NativeDatabase) {
		let kv = Arc::new(MemoryKv::default());
		let (runtime, db) = open_database_with_kv(file_name, kv.clone());
		(runtime, kv, db)
	}

	fn exec_sql_result(db: &NativeDatabase, sql: &str) -> Result<(), (c_int, String)> {
		let c_sql = CString::new(sql).expect("sql without nul");
		let mut err_msg: *mut c_char = ptr::null_mut();
		let rc = unsafe {
			sqlite3_exec(
				db.as_ptr(),
				c_sql.as_ptr(),
				None,
				ptr::null_mut(),
				&mut err_msg,
			)
		};
		if rc == SQLITE_OK {
			return Ok(());
		}

		let message = if err_msg.is_null() {
			sqlite_error_message(db.as_ptr())
		} else {
			let message = unsafe { CStr::from_ptr(err_msg) }
				.to_string_lossy()
				.into_owned();
			unsafe {
				sqlite3_free(err_msg.cast());
			}
			message
		};
		Err((rc, message))
	}

	fn exec_sql(db: &NativeDatabase, sql: &str) {
		let result = exec_sql_result(db, sql);
		assert_eq!(result, Ok(()), "sql failed: {sql}");
	}

	fn primary_result_code(rc: c_int) -> c_int {
		rc & 0xff
	}

	fn query_single_i64(db: &NativeDatabase, sql: &str) -> i64 {
		let c_sql = CString::new(sql).expect("sql without nul");
		let mut stmt = ptr::null_mut();
		let rc = unsafe {
			sqlite3_prepare_v2(db.as_ptr(), c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut())
		};
		assert_eq!(rc, SQLITE_OK, "prepare failed: {sql}");
		assert!(!stmt.is_null(), "statement pointer missing for {sql}");
		let step_rc = unsafe { sqlite3_step(stmt) };
		assert_eq!(step_rc, SQLITE_ROW, "query returned no row: {sql}");
		let value = unsafe { sqlite3_column_int64(stmt, 0) };
		let final_rc = unsafe { sqlite3_finalize(stmt) };
		assert_eq!(final_rc, SQLITE_OK, "finalize failed: {sql}");
		value
	}

	fn query_single_text(db: &NativeDatabase, sql: &str) -> String {
		let c_sql = CString::new(sql).expect("sql without nul");
		let mut stmt = ptr::null_mut();
		let rc = unsafe {
			sqlite3_prepare_v2(db.as_ptr(), c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut())
		};
		assert_eq!(rc, SQLITE_OK, "prepare failed: {sql}");
		assert!(!stmt.is_null(), "statement pointer missing for {sql}");
		let step_rc = unsafe { sqlite3_step(stmt) };
		assert_eq!(step_rc, SQLITE_ROW, "query returned no row: {sql}");
		let value = unsafe {
			CStr::from_ptr(sqlite3_column_text(stmt, 0).cast())
				.to_string_lossy()
				.into_owned()
		};
		let final_rc = unsafe { sqlite3_finalize(stmt) };
		assert_eq!(final_rc, SQLITE_OK, "finalize failed: {sql}");
		value
	}

	fn assert_integrity_check_ok(db: &NativeDatabase) {
		assert_eq!(query_single_text(db, "PRAGMA integrity_check;"), "ok");
	}

	fn open_raw_main_file(vfs: &KvVfs, file_name: &str) -> (Vec<u8>, *mut sqlite3_file) {
		let mut file_storage = vec![0u8; unsafe { (*vfs.vfs_ptr).szOsFile as usize }];
		let p_file = file_storage.as_mut_ptr().cast::<sqlite3_file>();
		let c_name = CString::new(file_name).expect("file name without nul");
		let mut out_flags = 0;
		let rc = unsafe {
			kv_vfs_open(
				vfs.vfs_ptr,
				c_name.as_ptr(),
				p_file,
				SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_MAIN_DB,
				&mut out_flags,
			)
		};
		assert_eq!(rc, SQLITE_OK, "open raw sqlite file");
		(file_storage, p_file)
	}

	#[test]
	fn transaction_writes_buffer_until_sync_boundary() {
		let (_runtime, _kv, db) = open_memory_database("buffered-write.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		db.reset_vfs_telemetry();
		exec_sql(&db, "BEGIN;");
		for idx in 0..64 {
			let payload = format!("item-{idx}-{}", "x".repeat(512));
			exec_sql(
				&db,
				&format!("INSERT INTO items (payload) VALUES ('{payload}');"),
			);
		}
		exec_sql(&db, "COMMIT;");

		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 64);

		let telemetry = db.snapshot_vfs_telemetry();
		assert!(telemetry.writes.buffered_count > 0);
		assert!(telemetry.syncs.count > 0);
		assert!(telemetry.kv.put_count > 0);
		assert_eq!(telemetry.writes.immediate_kv_put_count, 0);
	}

	#[test]
	fn supported_fast_path_routes_buffered_commits_through_sqlite_write_batch() {
		let kv = Arc::new(MemoryKv::with_sqlite_write_batch_fast_path());
		let (runtime, db) = open_database_with_kv("fast-path-write-batch.db", kv.clone());

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		kv.clear_recorded_sqlite_write_batches();
		db.reset_vfs_telemetry();

		for idx in 0..2 {
			exec_sql(&db, "BEGIN;");
			exec_sql(
				&db,
				&format!("INSERT INTO items (payload) VALUES ('fast-path-{idx}');"),
			);
			exec_sql(&db, "COMMIT;");
		}

		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 2);
		assert_integrity_check_ok(&db);

		let main_write_batches: Vec<_> = kv
			.recorded_sqlite_write_batches()
			.into_iter()
			.filter(|request| request.file_tag == kv::FILE_TAG_MAIN)
			.collect();
		assert!(
			main_write_batches.len() >= 2,
			"expected at least two main-file fast-path commits"
		);
		assert!(main_write_batches[0].fence.request_fence > 0);
		for window in main_write_batches.windows(2) {
			assert!(
				window[1].fence.request_fence > window[0].fence.request_fence,
				"expected strictly increasing fences"
			);
			assert_eq!(
				window[1].fence.expected_fence,
				Some(window[0].fence.request_fence)
			);
		}

		let telemetry = db.snapshot_vfs_telemetry();
		assert!(telemetry.atomic_write.fast_path_success_count > 0);
		assert_eq!(telemetry.atomic_write.fast_path_failure_count, 0);

		drop(db);
		drop(runtime);

		let (_reopen_runtime, reopened_db) = open_database_with_kv("fast-path-write-batch.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			2
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn committed_rows_survive_reopen_after_commit() {
		let (runtime, kv, db) = open_memory_database("commit-durable.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		exec_sql(&db, "BEGIN;");
		exec_sql(
			&db,
			"INSERT INTO items (id, payload) VALUES (1, 'committed');",
		);
		exec_sql(&db, "COMMIT;");

		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 1);
		assert_integrity_check_ok(&db);

		drop(db);
		drop(runtime);

		let (_reopen_runtime, reopened_db) = open_database_with_kv("commit-durable.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			1
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn rollback_discards_buffered_writes_before_commit_boundary() {
		let (runtime, kv, db) = open_memory_database("rollback-buffered.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		db.reset_vfs_telemetry();
		exec_sql(&db, "BEGIN;");
		exec_sql(
			&db,
			"INSERT INTO items (id, payload) VALUES (1, 'rolled-back');",
		);
		exec_sql(&db, "ROLLBACK;");

		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 0);

		drop(db);
		drop(runtime);

		let (_reopen_runtime, reopened_db) = open_database_with_kv("rollback-buffered.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			0
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn sync_failure_returns_sqlite_ioerr_without_false_success() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let kv = Arc::new(MemoryKv::default());
		let vfs = KvVfs::register(
			"test-vfs-sync-failure",
			kv.clone(),
			"sync-failure.db".to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let (_file_storage, p_file) = open_raw_main_file(&vfs, "sync-failure.db");
		let ctx = unsafe { &*vfs.ctx_ptr };
		let file = unsafe { get_file(p_file) };
		let state = unsafe { get_file_state(file.state) };

		let original_page = empty_db_page();
		let mut updated_page = original_page.clone();
		updated_page[128] = 0x7f;

		let write_rc = unsafe {
			kv_io_write(
				p_file,
				updated_page.as_ptr().cast(),
				updated_page.len() as c_int,
				0,
			)
		};
		assert_eq!(write_rc, SQLITE_OK);
		kv.fail_next_batch_put("simulated timeout during commit flush");

		let sync_rc = unsafe { kv_io_sync(p_file, 0) };
		assert_eq!(primary_result_code(sync_rc), SQLITE_IOERR);
		assert_eq!(
			ctx.take_last_error().as_deref(),
			Some("simulated timeout during commit flush")
		);
		assert_eq!(state.dirty_buffer.get(&0), Some(&updated_page));
		assert_eq!(
			kv.store
				.lock()
				.expect("memory kv mutex poisoned")
				.get(kv::get_chunk_key(kv::FILE_TAG_MAIN, 0).as_slice()),
			Some(&original_page)
		);
	}

	#[test]
	fn missing_fast_path_capability_falls_back_to_generic_sync_flush() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let kv = Arc::new(MemoryKv::default());
		let vfs = KvVfs::register(
			"test-vfs-fast-path-fallback",
			kv,
			"fast-path-fallback.db".to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let (_file_storage, p_file) = open_raw_main_file(&vfs, "fast-path-fallback.db");

		let mut updated_page = empty_db_page();
		updated_page[64] = 0x4f;
		let write_rc = unsafe {
			kv_io_write(
				p_file,
				updated_page.as_ptr().cast(),
				updated_page.len() as c_int,
				0,
			)
		};
		assert_eq!(write_rc, SQLITE_OK);
		assert_eq!(unsafe { kv_io_sync(p_file, 0) }, SQLITE_OK);
		assert_eq!(unsafe { kv_io_close(p_file) }, SQLITE_OK);

		let telemetry = vfs.snapshot_vfs_telemetry();
		assert_eq!(telemetry.atomic_write.fast_path_success_count, 0);
		assert_eq!(telemetry.atomic_write.fast_path_fallback_count, 1);
		assert!(telemetry.kv.put_count > 0);
	}

	#[test]
	fn actor_stop_during_buffered_write_rolls_back_uncommitted_pages() {
		let (runtime, kv, db) = open_memory_database("actor-stop-buffered.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		exec_sql(&db, "BEGIN;");
		exec_sql(
			&db,
			"INSERT INTO items (id, payload) VALUES (1, 'stopped');",
		);
		drop(db);
		drop(runtime);

		let (_reopen_runtime, reopened_db) = open_database_with_kv("actor-stop-buffered.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			0
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn process_death_before_commit_drops_only_buffered_state() {
		let (runtime, kv, db) = open_memory_database("process-death-before-commit.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		exec_sql(&db, "BEGIN;");
		exec_sql(
			&db,
			"INSERT INTO items (id, payload) VALUES (1, 'lost-on-crash');",
		);
		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 1);

		std::mem::forget(db);
		std::mem::forget(runtime);

		let (_reopen_runtime, reopened_db) =
			open_database_with_kv("process-death-before-commit.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			0
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn process_death_after_commit_ack_keeps_rows_durable() {
		let (runtime, kv, db) = open_memory_database("process-death-after-commit.db");

		exec_sql(
			&db,
			"CREATE TABLE items (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
		);
		exec_sql(&db, "BEGIN;");
		exec_sql(
			&db,
			"INSERT INTO items (id, payload) VALUES (1, 'durable');",
		);
		exec_sql(&db, "COMMIT;");
		assert_eq!(query_single_i64(&db, "SELECT COUNT(*) FROM items;"), 1);

		std::mem::forget(db);
		std::mem::forget(runtime);

		let (_reopen_runtime, reopened_db) =
			open_database_with_kv("process-death-after-commit.db", kv);
		assert_eq!(
			query_single_i64(&reopened_db, "SELECT COUNT(*) FROM items;"),
			1
		);
		assert_integrity_check_ok(&reopened_db);
	}

	#[test]
	fn retry_after_timeout_flushes_buffered_pages_on_next_sync() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let kv = Arc::new(MemoryKv::default());
		let vfs = KvVfs::register(
			"test-vfs-sync-retry",
			kv.clone(),
			"sync-retry.db".to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let (_file_storage, p_file) = open_raw_main_file(&vfs, "sync-retry.db");
		let file = unsafe { get_file(p_file) };
		let state = unsafe { get_file_state(file.state) };

		let mut updated_page = empty_db_page();
		updated_page[256] = 0x55;
		let write_rc = unsafe {
			kv_io_write(
				p_file,
				updated_page.as_ptr().cast(),
				updated_page.len() as c_int,
				0,
			)
		};
		assert_eq!(write_rc, SQLITE_OK);
		kv.fail_next_batch_put("simulated timeout during commit flush");

		let failed_sync_rc = unsafe { kv_io_sync(p_file, 0) };
		assert_eq!(primary_result_code(failed_sync_rc), SQLITE_IOERR);
		assert!(!state.dirty_buffer.is_empty());

		let retry_sync_rc = unsafe { kv_io_sync(p_file, 0) };
		assert_eq!(retry_sync_rc, SQLITE_OK);
		assert!(state.dirty_buffer.is_empty());
		assert_eq!(
			kv.store
				.lock()
				.expect("memory kv mutex poisoned")
				.get(kv::get_chunk_key(kv::FILE_TAG_MAIN, 0).as_slice()),
			Some(&updated_page)
		);
		assert_eq!(unsafe { kv_io_close(p_file) }, SQLITE_OK);
	}

	#[test]
	fn fast_path_write_batch_failure_returns_sqlite_ioerr() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let kv = Arc::new(MemoryKv::with_sqlite_write_batch_fast_path());
		let vfs = KvVfs::register(
			"test-vfs-fast-path-failure",
			kv.clone(),
			"fast-path-failure.db".to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let (_file_storage, p_file) = open_raw_main_file(&vfs, "fast-path-failure.db");
		let ctx = unsafe { &*vfs.ctx_ptr };
		let state = unsafe { get_file_state(get_file(p_file).state) };

		let mut updated_page = empty_db_page();
		updated_page[512] = 0x3c;
		let write_rc = unsafe {
			kv_io_write(
				p_file,
				updated_page.as_ptr().cast(),
				updated_page.len() as c_int,
				0,
			)
		};
		assert_eq!(write_rc, SQLITE_OK);
		kv.fail_next_sqlite_write_batch("simulated fast-path failure");

		let sync_rc = unsafe { kv_io_sync(p_file, 0) };
		assert_eq!(primary_result_code(sync_rc), SQLITE_IOERR);
		assert_eq!(
			ctx.take_last_error().as_deref(),
			Some("simulated fast-path failure")
		);
		assert_eq!(state.dirty_buffer.get(&0), Some(&updated_page));
		assert!(kv.recorded_sqlite_write_batches().is_empty());

		let telemetry = vfs.snapshot_vfs_telemetry();
		assert_eq!(telemetry.atomic_write.fast_path_attempt_count, 1);
		assert_eq!(telemetry.atomic_write.fast_path_failure_count, 1);
		assert_eq!(telemetry.atomic_write.fast_path_success_count, 0);
	}

	#[test]
	fn load_visible_chunk_skips_remote_chunks_past_pending_delete_boundary() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("create tokio runtime");
		let kv = Arc::new(MemoryKv::default());
		let stale_chunk = vec![7u8; kv::CHUNK_SIZE];
		kv.store.lock().expect("memory kv mutex poisoned").insert(
			kv::get_chunk_key(kv::FILE_TAG_MAIN, 1).to_vec(),
			stale_chunk,
		);

		let vfs = KvVfs::register(
			"test-vfs-logical-delete-read",
			kv,
			"logical-delete-read.db".to_string(),
			runtime.handle().clone(),
			Vec::new(),
		)
		.expect("register test vfs");
		let ctx = unsafe { &*vfs.ctx_ptr };
		let state = Box::new(KvFileState::new(false));
		let state_ref = Box::leak(state);
		state_ref.pending_delete_start = Some(1);
		let file = KvFile {
			base: sqlite3_file {
				pMethods: ctx.io_methods.as_ref() as *const sqlite3_io_methods,
			},
			ctx: ctx as *const VfsContext,
			state: state_ref as *mut KvFileState,
			file_tag: kv::FILE_TAG_MAIN,
			meta_key: kv::get_meta_key(kv::FILE_TAG_MAIN),
			size: (kv::CHUNK_SIZE * 2) as i64,
			meta_dirty: true,
			flags: 0,
		};

		assert_eq!(
			load_visible_chunk(&file, state_ref, ctx, 1).expect("load visible chunk"),
			None
		);
	}
}
