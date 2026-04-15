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
use crate::sqlite_kv::{KvGetResult, SqliteKv, SqliteKvError};

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

struct VfsContext {
	kv: Arc<dyn SqliteKv>,
	actor_id: String,
	main_file_name: String,
	// Bounded startup entries shipped with actor start. This is not the opt-in read cache.
	startup_preload: Mutex<Option<StartupPreloadEntries>>,
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
		)
	}
}

// MARK: File State

struct KvFileState {
	batch_mode: bool,
	dirty_buffer: BTreeMap<u32, Vec<u8>>,
	saved_file_size: i64,
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
			saved_file_size: 0,
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

// MARK: IO Callbacks

unsafe extern "C" fn kv_io_close(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;

		let result = if file.flags & SQLITE_OPEN_DELETEONCLOSE != 0 {
			ctx.delete_file(file.file_tag)
		} else if file.meta_dirty {
			ctx.kv_put(
				vec![file.meta_key.to_vec()],
				vec![encode_file_meta(file.size)],
			)
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
		// Skip fetching chunks already present in the dirty buffer (batch mode) or read cache.
		for chunk_idx in start_chunk..=end_chunk {
			if state.batch_mode {
				if state.dirty_buffer.contains_key(&(chunk_idx as u32)) {
					continue;
				}
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
			let chunk_data = if state.batch_mode {
				state
					.dirty_buffer
					.get(&(chunk_idx as u32))
					.map(|buffered| buffered.as_slice())
			} else {
				None
			}
			.or_else(|| buffered_chunks.get(&chunk_idx).copied())
			.or_else(|| {
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

		// `resp` is empty when every chunk was served from the dirty buffer or read cache.
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

		{
			let state = get_file_state(file.state);
			if state.batch_mode {
				for chunk_idx in start_chunk..=end_chunk {
					let chunk_offset = chunk_idx * kv::CHUNK_SIZE;
					let source_start =
						std::cmp::max(0isize, chunk_offset as isize - offset as isize) as usize;
					let source_end =
						std::cmp::min(write_length, chunk_offset + kv::CHUNK_SIZE - offset);
					state
						.dirty_buffer
						.insert(chunk_idx as u32, data[source_start..source_end].to_vec());
				}

				let new_size = std::cmp::max(file.size, write_end_offset as i64);
				if new_size != file.size {
					file.size = new_size;
					file.meta_dirty = true;
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
				return SQLITE_OK;
			}
		}

		struct WritePlan {
			chunk_key: Vec<u8>,
			chunk_offset: usize,
			write_start: usize,
			write_end: usize,
			cached_chunk: Option<Vec<u8>>,
			existing_chunk_index: Option<usize>,
		}

		let mut plans = Vec::new();
		let mut chunk_keys_to_fetch = Vec::new();
		for chunk_idx in start_chunk..=end_chunk {
			let chunk_offset = chunk_idx * kv::CHUNK_SIZE;
			let write_start = offset.saturating_sub(chunk_offset);
			let write_end = std::cmp::min(kv::CHUNK_SIZE, offset + write_length - chunk_offset);
			let existing_bytes_in_chunk = if file.size as usize > chunk_offset {
				std::cmp::min(kv::CHUNK_SIZE, file.size as usize - chunk_offset)
			} else {
				0
			};
			let needs_existing = write_start > 0 || existing_bytes_in_chunk > write_end;
			let chunk_key = kv::get_chunk_key(file.file_tag, chunk_idx as u32).to_vec();
			let cached_chunk = if needs_existing && ctx.read_cache_enabled {
				let state = get_file_state(file.state);
				state
					.read_cache
					.as_ref()
					.and_then(|read_cache| read_cache.get(chunk_key.as_slice()).cloned())
			} else {
				None
			};
			let existing_chunk_index = if needs_existing && cached_chunk.is_none() {
				let idx = chunk_keys_to_fetch.len();
				chunk_keys_to_fetch.push(chunk_key.clone());
				Some(idx)
			} else {
				None
			};

			plans.push(WritePlan {
				chunk_key,
				chunk_offset,
				write_start,
				write_end,
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

		let mut entries_to_write = Vec::with_capacity(plans.len() + 1);
		for plan in &plans {
			let existing_chunk = plan.cached_chunk.as_deref().or_else(|| {
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

			entries_to_write.push((plan.chunk_key.clone(), new_chunk));
		}

		let previous_size = file.size;
		let previous_meta_dirty = file.meta_dirty;
		let new_size = std::cmp::max(file.size, write_end_offset as i64);
		if new_size != previous_size {
			file.size = new_size;
			file.meta_dirty = true;
		}
		if file.meta_dirty {
			entries_to_write.push((file.meta_key.to_vec(), encode_file_meta(file.size)));
		}

		if let Some(read_cache) = get_file_state(file.state).read_cache.as_mut() {
			for (key, value) in &entries_to_write {
				// Only cache chunk keys here. Metadata keys are read on open/access
				// and should not be mixed into the per-page cache.
				if key.len() == 8 {
					read_cache.insert(key.clone(), value.clone());
				}
			}
		}

		let (keys, values) = split_entries(entries_to_write);
		ctx.vfs_metrics
			.xwrite_immediate_kv_put_count
			.fetch_add(1, Ordering::Relaxed);
		ctx.vfs_metrics
			.xwrite_immediate_kv_put_bytes
			.fetch_add(data.len() as u64, Ordering::Relaxed);
		if ctx.kv_put(keys, values).is_err() {
			file.size = previous_size;
			file.meta_dirty = previous_meta_dirty;
			return SQLITE_IOERR_WRITE;
		}
		file.meta_dirty = false;

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

		if size < 0 || size as u64 > kv::MAX_FILE_SIZE {
			return SQLITE_IOERR_TRUNCATE;
		}

		if size >= file.size {
			if size > file.size {
				let previous_size = file.size;
				let previous_meta_dirty = file.meta_dirty;
				file.size = size;
				file.meta_dirty = true;
				if ctx
					.kv_put(
						vec![file.meta_key.to_vec()],
						vec![encode_file_meta(file.size)],
					)
					.is_err()
				{
					file.size = previous_size;
					file.meta_dirty = previous_meta_dirty;
					return SQLITE_IOERR_TRUNCATE;
				}
				file.meta_dirty = false;
			}
			return SQLITE_OK;
		}

		let last_chunk_to_keep = if size == 0 {
			-1
		} else {
			(size - 1) / kv::CHUNK_SIZE as i64
		};
		let last_existing_chunk = if file.size == 0 {
			-1
		} else {
			(file.size - 1) / kv::CHUNK_SIZE as i64
		};

		if let Some(read_cache) = get_file_state(file.state).read_cache.as_mut() {
			// The read cache stores only chunk keys. Keep entries strictly before
			// the truncation boundary so reads cannot serve bytes from removed chunks.
			read_cache.retain(|key, _| {
				// Chunk keys are 8 bytes: [prefix, version, CHUNK_PREFIX, file_tag, idx_be32]
				if key.len() == 8 && key[3] == file.file_tag {
					let chunk_idx = u32::from_be_bytes([key[4], key[5], key[6], key[7]]);
					(chunk_idx as i64) <= last_chunk_to_keep
				} else {
					true
				}
			});
		}

		let previous_size = file.size;
		let previous_meta_dirty = file.meta_dirty;
		file.size = size;
		file.meta_dirty = true;
		if ctx
			.kv_put(
				vec![file.meta_key.to_vec()],
				vec![encode_file_meta(file.size)],
			)
			.is_err()
		{
			file.size = previous_size;
			file.meta_dirty = previous_meta_dirty;
			return SQLITE_IOERR_TRUNCATE;
		}
		file.meta_dirty = false;

		if size > 0 && size as usize % kv::CHUNK_SIZE != 0 {
			let last_chunk_key = kv::get_chunk_key(file.file_tag, last_chunk_to_keep as u32);
			let resp = match ctx.kv_get(vec![last_chunk_key.to_vec()]) {
				Ok(resp) => resp,
				Err(_) => return SQLITE_IOERR_TRUNCATE,
			};
			let value_map = build_value_map(&resp);
			if let Some(last_chunk_data) = value_map.get(last_chunk_key.as_slice()) {
				let truncated_len = size as usize % kv::CHUNK_SIZE;
				if last_chunk_data.len() > truncated_len {
					let truncated_chunk = last_chunk_data[..truncated_len].to_vec();
					if ctx
						.kv_put(vec![last_chunk_key.to_vec()], vec![truncated_chunk.clone()])
						.is_err()
					{
						return SQLITE_IOERR_TRUNCATE;
					}
					if let Some(read_cache) = get_file_state(file.state).read_cache.as_mut() {
						read_cache.insert(last_chunk_key.to_vec(), truncated_chunk);
					}
				}
			}
		}

		if last_chunk_to_keep < last_existing_chunk {
			if ctx
				.kv_delete_range(
					kv::get_chunk_key(file.file_tag, (last_chunk_to_keep + 1) as u32).to_vec(),
					kv::get_chunk_key_range_end(file.file_tag).to_vec(),
				)
				.is_err()
			{
				return SQLITE_IOERR_TRUNCATE;
			}
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let sync_start = std::time::Instant::now();
		ctx.vfs_metrics.xsync_count.fetch_add(1, Ordering::Relaxed);
		if !file.meta_dirty {
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
		if ctx
			.kv_put(
				vec![file.meta_key.to_vec()],
				vec![encode_file_meta(file.size)],
			)
			.is_err()
		{
			ctx.vfs_metrics
				.xsync_us
				.fetch_add(sync_start.elapsed().as_micros() as u64, Ordering::Relaxed);
			return SQLITE_IOERR_FSYNC;
		}
		file.meta_dirty = false;
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
				state.saved_file_size = file.size;
				state.batch_mode = true;
				file.meta_dirty = false;
				state.dirty_buffer.clear();
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
				let ctx = &*file.ctx;
				let commit_start = std::time::Instant::now();
				ctx.vfs_metrics
					.commit_atomic_attempt_count
					.fetch_add(1, Ordering::Relaxed);
				let dirty_page_count = state.dirty_buffer.len() as u64;
				let dirty_buffer_bytes = state
					.dirty_buffer
					.values()
					.map(|value| value.len() as u64)
					.sum::<u64>();
				let max_dirty_pages = if file.meta_dirty {
					KV_MAX_BATCH_KEYS - 1
				} else {
					KV_MAX_BATCH_KEYS
				};

				if state.dirty_buffer.len() > max_dirty_pages {
					ctx.vfs_metrics
						.commit_atomic_batch_cap_failure_count
						.fetch_add(1, Ordering::Relaxed);
					ctx.vfs_metrics
						.commit_atomic_us
						.fetch_add(commit_start.elapsed().as_micros() as u64, Ordering::Relaxed);
					state.dirty_buffer.clear();
					file.size = state.saved_file_size;
					file.meta_dirty = false;
					state.batch_mode = false;
					return SQLITE_IOERR;
				}

				let mut entries = Vec::with_capacity(state.dirty_buffer.len() + 1);
				for (chunk_index, data) in &state.dirty_buffer {
					entries.push((
						kv::get_chunk_key(file.file_tag, *chunk_index).to_vec(),
						data.clone(),
					));
				}
				if file.meta_dirty {
					entries.push((file.meta_key.to_vec(), encode_file_meta(file.size)));
				}

				let (keys, values) = split_entries(entries);
				if ctx.kv_put(keys, values).is_err() {
					ctx.vfs_metrics
						.commit_atomic_kv_put_failure_count
						.fetch_add(1, Ordering::Relaxed);
					ctx.vfs_metrics
						.commit_atomic_us
						.fetch_add(commit_start.elapsed().as_micros() as u64, Ordering::Relaxed);
					state.dirty_buffer.clear();
					file.size = state.saved_file_size;
					file.meta_dirty = false;
					state.batch_mode = false;
					return SQLITE_IOERR;
				}

				// Move dirty buffer entries into the read cache so subsequent
				// reads can serve them without a KV round-trip.
				let flushed: Vec<_> = std::mem::take(&mut state.dirty_buffer)
					.into_iter()
					.collect();
				if let Some(read_cache) = state.read_cache.as_mut() {
					// Only chunk pages belong in the read cache. The metadata write above
					// still goes through KV, but should not be cached as a page.
					for (chunk_index, data) in flushed {
						let key = kv::get_chunk_key(file.file_tag, chunk_index);
						read_cache.insert(key.to_vec(), data);
					}
				}
				file.meta_dirty = false;
				state.batch_mode = false;
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
				state.dirty_buffer.clear();
				file.size = state.saved_file_size;
				file.meta_dirty = false;
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
}
