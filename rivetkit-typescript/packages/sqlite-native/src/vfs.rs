//! Custom SQLite VFS backed by KV operations over the KV channel.
//!
//! Keep this file behaviorally aligned with
//! `rivetkit-typescript/packages/sqlite-wasm/src/vfs.ts`.

use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use libsqlite3_sys::*;
use tokio::runtime::Handle;

use crate::kv;
use crate::sqlite_kv::{KvGetResult, SqliteKv};

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

/// File metadata version. Must match CURRENT_VERSION in the WASM VFS schema.
const META_VERSION: u16 = 1;

/// Encoded metadata size. This is 2 bytes of version plus 8 bytes of size.
const META_ENCODED_SIZE: usize = 10;

/// Maximum pathname length reported to SQLite.
const MAX_PATHNAME: c_int = 64;

/// Maximum number of keys accepted by a single KV put or delete request.
const KV_MAX_BATCH_KEYS: usize = 128;

/// First 108 bytes of a valid empty page-1 SQLite database.
///
/// This must match `HEADER_PREFIX` in
/// `rivetkit-typescript/packages/sqlite-vfs/src/generated/empty-db-page.ts`.
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

// MARK: VFS Metrics

/// Per-VFS-callback operation metrics for diagnosing native vs WASM performance.
pub struct VfsMetrics {
	pub xread_count: AtomicU64,
	pub xread_us: AtomicU64,
	pub xwrite_count: AtomicU64,
	pub xwrite_us: AtomicU64,
	pub xwrite_buffered_count: AtomicU64,
	pub xsync_count: AtomicU64,
	pub xsync_us: AtomicU64,
	pub commit_atomic_count: AtomicU64,
	pub commit_atomic_us: AtomicU64,
	pub commit_atomic_pages: AtomicU64,
}

impl VfsMetrics {
	pub fn new() -> Self {
		Self {
			xread_count: AtomicU64::new(0),
			xread_us: AtomicU64::new(0),
			xwrite_count: AtomicU64::new(0),
			xwrite_us: AtomicU64::new(0),
			xwrite_buffered_count: AtomicU64::new(0),
			xsync_count: AtomicU64::new(0),
			xsync_us: AtomicU64::new(0),
			commit_atomic_count: AtomicU64::new(0),
			commit_atomic_us: AtomicU64::new(0),
			commit_atomic_pages: AtomicU64::new(0),
		}
	}
}

// MARK: VFS Context

struct VfsContext {
	kv: Arc<dyn SqliteKv>,
	actor_id: String,
	main_file_name: String,
	rt_handle: Handle,
	io_methods: Box<sqlite3_io_methods>,
	vfs_metrics: Arc<VfsMetrics>,
}

impl VfsContext {
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

	fn kv_get(&self, keys: Vec<Vec<u8>>) -> Result<KvGetResult, String> {
		let op_name = format!("get({}keys)", keys.len());
		let start = std::time::Instant::now();
		let result = self
			.rt_handle
			.block_on(self.kv.batch_get(&self.actor_id, keys))
			.map_err(|e| e.to_string());
		let elapsed = start.elapsed();
		if std::env::var("RIVET_TRACE_SQL").is_ok() {
			tracing::trace!(
				op = %op_name,
				duration_us = elapsed.as_micros() as u64,
				"sql trace kv round-trip"
			);
		}
		tracing::debug!(
			op = %op_name,
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_put(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<(), String> {
		let op_name = format!("put({}keys)", keys.len());
		let start = std::time::Instant::now();
		let result = self
			.rt_handle
			.block_on(self.kv.batch_put(&self.actor_id, keys, values))
			.map_err(|e| e.to_string());
		let elapsed = start.elapsed();
		if std::env::var("RIVET_TRACE_SQL").is_ok() {
			tracing::trace!(
				op = %op_name,
				duration_us = elapsed.as_micros() as u64,
				"sql trace kv round-trip"
			);
		}
		tracing::debug!(
			op = %op_name,
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_delete(&self, keys: Vec<Vec<u8>>) -> Result<(), String> {
		let op_name = format!("del({}keys)", keys.len());
		let start = std::time::Instant::now();
		let result = self
			.rt_handle
			.block_on(self.kv.batch_delete(&self.actor_id, keys))
			.map_err(|e| e.to_string());
		let elapsed = start.elapsed();
		if std::env::var("RIVET_TRACE_SQL").is_ok() {
			tracing::trace!(
				op = %op_name,
				duration_us = elapsed.as_micros() as u64,
				"sql trace kv round-trip"
			);
		}
		tracing::debug!(
			op = %op_name,
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> Result<(), String> {
		let op_name = "delRange";
		let start_time = std::time::Instant::now();
		let result = self
			.rt_handle
			.block_on(self.kv.delete_range(&self.actor_id, start, end))
			.map_err(|e| e.to_string());
		let elapsed = start_time.elapsed();
		if std::env::var("RIVET_TRACE_SQL").is_ok() {
			tracing::trace!(
				op = %op_name,
				duration_us = elapsed.as_micros() as u64,
				"sql trace kv round-trip"
			);
		}
		tracing::debug!(
			op = %op_name,
			duration_us = elapsed.as_micros() as u64,
			"kv round-trip"
		);
		result
	}

	fn delete_file(&self, file_tag: u8) -> Result<(), String> {
		let meta_key = kv::get_meta_key(file_tag);
		let resp = self.kv_get(vec![meta_key.to_vec()])?;
		let value_map = build_value_map(&resp);
		if !value_map.contains_key(meta_key.as_slice()) {
			return Ok(());
		}

		self.kv_delete_range(
			kv::get_chunk_key(file_tag, 0).to_vec(),
			kv::get_chunk_key_range_end(file_tag).to_vec(),
		)?;
		self.kv_delete(vec![meta_key.to_vec()])
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
	read_cache: HashMap<Vec<u8>, Vec<u8>>,
}

impl KvFileState {
	fn new() -> Self {
		Self {
			batch_mode: false,
			dirty_buffer: BTreeMap::new(),
			saved_file_size: 0,
			read_cache: HashMap::new(),
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
		let buf = slice::from_raw_parts_mut(p_buf as *mut u8, requested_length);

		if i_offset < 0 {
			return SQLITE_IOERR_READ;
		}

		let offset = i_offset as usize;
		let file_size = file.size as usize;
		if offset >= file_size {
			buf.fill(0);
			return SQLITE_IOERR_SHORT_READ;
		}

		let start_chunk = offset / kv::CHUNK_SIZE;
		let end_chunk = (offset + requested_length - 1) / kv::CHUNK_SIZE;

		let mut chunk_keys_to_fetch = Vec::new();
		let mut buffered_chunks: HashMap<usize, Vec<u8>> = HashMap::new();
		for chunk_idx in start_chunk..=end_chunk {
			// Check dirty buffer first (batch mode writes).
			if state.batch_mode {
				if let Some(buffered) = state.dirty_buffer.get(&(chunk_idx as u32)) {
					buffered_chunks.insert(chunk_idx, buffered.clone());
					continue;
				}
			}
			// Check read cache.
			let key = kv::get_chunk_key(file.file_tag, chunk_idx as u32);
			if let Some(cached) = state.read_cache.get(key.as_slice()) {
				buffered_chunks.insert(chunk_idx, cached.clone());
				continue;
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
				Ok(resp) => {
					// Populate read cache with fetched values.
					for (key, value) in resp.keys.iter().zip(resp.values.iter()) {
						if !value.is_empty() {
							state.read_cache.insert(key.clone(), value.clone());
						}
					}
					resp
				}
				Err(_) => return SQLITE_IOERR_READ,
			}
		};
		let value_map = build_value_map(&resp);

		for chunk_idx in start_chunk..=end_chunk {
			let chunk_data: Option<&[u8]> = buffered_chunks
				.get(&chunk_idx)
				.map(|v| v.as_slice())
				.or_else(|| {
					let key = kv::get_chunk_key(file.file_tag, chunk_idx as u32);
					value_map.get(key.as_slice()).copied()
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

		let actual_bytes = std::cmp::min(requested_length, file_size - offset);
		if actual_bytes < requested_length {
			buf[actual_bytes..].fill(0);
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
			let existing_chunk_index = if needs_existing {
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
			let existing_chunk = plan
				.existing_chunk_index
				.and_then(|idx| existing_chunks.get(idx))
				.and_then(|value| value.as_ref());

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

		// Update read cache with the entries we're about to write.
		{
			let state = get_file_state(file.state);
			for (key, value) in &entries_to_write {
				// Only cache chunk keys, not metadata.
				if key.len() == 8 {
					state.read_cache.insert(key.clone(), value.clone());
				}
			}
		}

		let (keys, values) = split_entries(entries_to_write);
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

		// Invalidate read cache entries for truncated chunks.
		{
			let state = get_file_state(file.state);
			let truncate_from_chunk = if size == 0 {
				0u32
			} else {
				(size as u32 / kv::CHUNK_SIZE as u32) + 1
			};
			state.read_cache.retain(|key, _| {
				// Chunk keys are 8 bytes: [prefix, version, CHUNK_PREFIX, file_tag, idx_be32]
				if key.len() == 8 && key[3] == file.file_tag {
					let chunk_idx = u32::from_be_bytes([key[4], key[5], key[6], key[7]]);
					chunk_idx < truncate_from_chunk
				} else {
					true
				}
			});
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
					if ctx
						.kv_put(
							vec![last_chunk_key.to_vec()],
							vec![last_chunk_data[..truncated_len].to_vec()],
						)
						.is_err()
					{
						return SQLITE_IOERR_TRUNCATE;
					}
				}
			}
		}

		let mut keys_to_delete = Vec::new();
		let mut chunk_idx = last_chunk_to_keep + 1;
		while chunk_idx <= last_existing_chunk {
			keys_to_delete.push(kv::get_chunk_key(file.file_tag, chunk_idx as u32).to_vec());
			chunk_idx += 1;
		}

		for chunk in keys_to_delete.chunks(KV_MAX_BATCH_KEYS) {
			if ctx.kv_delete(chunk.to_vec()).is_err() {
				return SQLITE_IOERR_TRUNCATE;
			}
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_FSYNC, {
		let file = get_file(p_file);
		if !file.meta_dirty {
			return SQLITE_OK;
		}

		let ctx = &*file.ctx;
		if ctx
			.kv_put(
				vec![file.meta_key.to_vec()],
				vec![encode_file_meta(file.size)],
			)
			.is_err()
		{
			return SQLITE_IOERR_FSYNC;
		}
		file.meta_dirty = false;

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
				state.saved_file_size = file.size;
				state.batch_mode = true;
				file.meta_dirty = false;
				state.dirty_buffer.clear();
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
				let ctx = &*file.ctx;
				let commit_start = std::time::Instant::now();
				let dirty_page_count = state.dirty_buffer.len() as u64;
				let max_dirty_pages = if file.meta_dirty {
					KV_MAX_BATCH_KEYS - 1
				} else {
					KV_MAX_BATCH_KEYS
				};

				if state.dirty_buffer.len() > max_dirty_pages {
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
				for (chunk_index, data) in flushed {
					let key = kv::get_chunk_key(file.file_tag, chunk_index);
					state.read_cache.insert(key.to_vec(), data);
				}
				file.meta_dirty = false;
				state.batch_mode = false;
				ctx.vfs_metrics
					.commit_atomic_count
					.fetch_add(1, Ordering::Relaxed);
				ctx.vfs_metrics
					.commit_atomic_pages
					.fetch_add(dirty_page_count, Ordering::Relaxed);
				ctx.vfs_metrics
					.commit_atomic_us
					.fetch_add(commit_start.elapsed().as_micros() as u64, Ordering::Relaxed);
				SQLITE_OK
			}
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				if !state.batch_mode {
					return SQLITE_OK;
				}
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

		let state = Box::into_raw(Box::new(KvFileState::new()));
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
	_p_vfs: *mut sqlite3_vfs,
	_n_byte: c_int,
	_z_err_msg: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, SQLITE_OK)
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
	pub fn register(
		name: &str,
		kv: Arc<dyn SqliteKv>,
		actor_id: String,
		rt_handle: Handle,
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
		let ctx = Box::new(VfsContext {
			kv,
			actor_id: actor_id.clone(),
			main_file_name: actor_id,
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
		if !db.is_null() {
			unsafe {
				sqlite3_close(db);
			}
		}
		return Err(format!("sqlite3_open_v2 failed with code {rc}"));
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
			unsafe {
				sqlite3_close(db);
			}
			return Err(format!("{pragma} failed with code {rc}"));
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
}
