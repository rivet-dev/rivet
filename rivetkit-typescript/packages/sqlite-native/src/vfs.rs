//! Custom SQLite VFS backed by KV operations via the channel.
//!
//! Maps SQLite VFS callbacks (xRead, xWrite, xTruncate, xDelete, etc.)
//! to KV get/put/delete/deleteRange operations. Uses the same 4 KiB chunk
//! layout and key encoding as the WASM VFS (`rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`).
//!
//! End-to-end tests are in the driver test suite:
//! `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::slice;
use std::sync::Arc;

use libsqlite3_sys::*;
use tokio::runtime::Handle;

use crate::channel::KvChannel;
use crate::kv;
use crate::protocol::*;

// MARK: Panic Guard

/// Extract a human-readable message from a panic payload.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
	if let Some(s) = payload.downcast_ref::<&str>() {
		s.to_string()
	} else if let Some(s) = payload.downcast_ref::<String>() {
		s.clone()
	} else {
		"unknown panic".to_string()
	}
}

/// Wrap a VFS callback body in `catch_unwind` to prevent panics from unwinding
/// through SQLite's C stack frames (which is undefined behavior).
///
/// On panic, logs the panic message via `tracing::error` and returns `$err_val`.
/// Uses `AssertUnwindSafe` because callback closures capture mutable state
/// (VFS file handles, context pointers).
macro_rules! vfs_catch_unwind {
	($err_val:expr, $body:expr) => {
		match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
			Ok(result) => result,
			Err(panic) => {
				tracing::error!(
					message = panic_message(&panic),
					"vfs callback panicked"
				);
				$err_val
			}
		}
	};
}

// MARK: Constants

/// File metadata version. Matches CURRENT_VERSION in
/// `rivetkit-typescript/packages/sqlite-vfs/schemas/file-meta/versioned.ts`.
/// Encoded as u16 LE (2 bytes), matching vbare's `serializeWithEmbeddedVersion` format.
const META_VERSION: u16 = 1;

/// Encoded metadata size: 2 bytes u16 LE version + 8 bytes u64 LE size.
const META_ENCODED_SIZE: usize = 10;

/// Maximum pathname length for this VFS.
const MAX_PATHNAME: c_int = 512;

// MARK: Metadata Encoding

/// Encode file size as versioned metadata.
///
/// Format: `[META_VERSION_u16_le, size_u64_le]`. Must be byte-identical to the WASM VFS
/// encoding in `rivetkit-typescript/packages/sqlite-vfs/schemas/file-meta/`, which uses
/// vbare's `serializeWithEmbeddedVersion` (2-byte u16 LE version prefix).
pub fn encode_file_meta(size: i64) -> Vec<u8> {
	let mut buf = Vec::with_capacity(META_ENCODED_SIZE);
	buf.extend_from_slice(&META_VERSION.to_le_bytes());
	buf.extend_from_slice(&(size as u64).to_le_bytes());
	buf
}

/// Decode file size from versioned metadata.
pub fn decode_file_meta(data: &[u8]) -> Option<i64> {
	if data.len() < META_ENCODED_SIZE {
		return None;
	}
	let version_bytes: [u8; 2] = data[0..2].try_into().ok()?;
	if u16::from_le_bytes(version_bytes) != META_VERSION {
		return None;
	}
	let bytes: [u8; 8] = data[2..10].try_into().ok()?;
	Some(u64::from_le_bytes(bytes) as i64)
}

// MARK: VFS Context

/// Shared state for a KV VFS instance. Stored in `sqlite3_vfs.pAppData`.
struct VfsContext {
	channel: Arc<KvChannel>,
	actor_id: String,
	main_file_name: String,
	rt_handle: Handle,
	/// IO methods table referenced by all open files. Lives as long as the VFS.
	io_methods: Box<sqlite3_io_methods>,
}

impl VfsContext {
	/// Resolve a file path to a file tag, or None if the path is unknown.
	fn resolve_file_tag(&self, path: &str) -> Option<u8> {
		if path == self.main_file_name {
			return Some(kv::FILE_TAG_MAIN);
		}
		let base = &self.main_file_name;
		if let Some(suffix) = path.strip_prefix(base) {
			match suffix {
				"-journal" => return Some(kv::FILE_TAG_JOURNAL),
				"-wal" => return Some(kv::FILE_TAG_WAL),
				"-shm" => return Some(kv::FILE_TAG_SHM),
				_ => {}
			}
		}
		None
	}

	/// Send a KV request synchronously by blocking on the tokio runtime.
	///
	/// Must not be called from within a tokio async context.
	fn send_sync(&self, data: RequestData) -> Result<ResponseData, String> {
		self.rt_handle
			.block_on(self.channel.send_request(&self.actor_id, data))
			.map_err(|e| e.to_string())
	}

	fn kv_get(&self, keys: Vec<Vec<u8>>) -> Result<KvGetResponse, String> {
		match self.send_sync(RequestData::KvGetRequest(KvGetRequest { keys }))? {
			ResponseData::KvGetResponse(r) => Ok(r),
			other => Err(format!("expected KvGetResponse, got {other:?}")),
		}
	}

	fn kv_put(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<(), String> {
		match self.send_sync(RequestData::KvPutRequest(KvPutRequest { keys, values }))? {
			ResponseData::KvPutResponse => Ok(()),
			other => Err(format!("expected KvPutResponse, got {other:?}")),
		}
	}

	fn kv_delete(&self, keys: Vec<Vec<u8>>) -> Result<(), String> {
		match self.send_sync(RequestData::KvDeleteRequest(KvDeleteRequest { keys }))? {
			ResponseData::KvDeleteResponse => Ok(()),
			other => Err(format!("expected KvDeleteResponse, got {other:?}")),
		}
	}

	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> Result<(), String> {
		match self.send_sync(RequestData::KvDeleteRangeRequest(KvDeleteRangeRequest {
			start,
			end,
		}))? {
			ResponseData::KvDeleteResponse => Ok(()),
			other => Err(format!("expected KvDeleteResponse, got {other:?}")),
		}
	}
}

// MARK: KvFile

/// Per-file state extending `sqlite3_file`. The `base` field must be first
/// because SQLite casts its `sqlite3_file*` to our struct.
#[repr(C)]
struct KvFile {
	base: sqlite3_file,
	ctx: *const VfsContext,
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

unsafe fn get_vfs_ctx(p: *mut sqlite3_vfs) -> &'static VfsContext {
	&*((*p).pAppData as *const VfsContext)
}

/// Build a lookup map from a KvGetResponse for efficient chunk access.
fn build_value_map(resp: &KvGetResponse) -> HashMap<&[u8], &[u8]> {
	resp.keys
		.iter()
		.zip(resp.values.iter())
		.filter(|(_, v)| !v.is_empty())
		.map(|(k, v)| (k.as_slice(), v.as_slice()))
		.collect()
}

// MARK: IO Callbacks

unsafe extern "C" fn kv_io_close(p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;

		// Delete-on-close: remove the file entirely (used for journal files).
		if file.flags & SQLITE_OPEN_DELETEONCLOSE != 0 {
			let tag = file.file_tag;
			if let Err(err) = ctx.kv_delete_range(
				kv::get_chunk_key(tag, 0).to_vec(),
				kv::get_chunk_key_range_end(tag).to_vec(),
			) {
				tracing::error!(%err, file_tag = tag, "failed to delete chunks on close");
				return SQLITE_IOERR;
			}
			if let Err(err) = ctx.kv_delete(vec![kv::get_meta_key(tag).to_vec()]) {
				tracing::error!(%err, file_tag = tag, "failed to delete metadata on close");
				return SQLITE_IOERR;
			}
			return SQLITE_OK;
		}

		// Flush dirty metadata before closing.
		if file.meta_dirty {
			let meta = encode_file_meta(file.size);
			if let Err(err) = ctx.kv_put(vec![file.meta_key.to_vec()], vec![meta]) {
				tracing::error!(%err, file_tag = file.file_tag, "failed to flush metadata on close");
				return SQLITE_IOERR;
			}
			file.meta_dirty = false;
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_read(
	p_file: *mut sqlite3_file,
	p_buf: *mut c_void,
	i_amt: c_int,
	i_offset: sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_READ, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let amt = i_amt as usize;
		let buf = slice::from_raw_parts_mut(p_buf as *mut u8, amt);

		if i_offset < 0 {
			return SQLITE_IOERR_READ;
		}

		// Past EOF: zero-fill and return short read.
		if i_offset >= file.size {
			buf.fill(0);
			return SQLITE_IOERR_SHORT_READ;
		}

		let offset = i_offset as usize;
		let file_size = file.size as usize;
		let start_chunk = offset / kv::CHUNK_SIZE;
		let end_chunk = (offset + amt - 1) / kv::CHUNK_SIZE;

		// Batch-fetch all needed chunk keys.
		let chunk_keys: Vec<Vec<u8>> = (start_chunk..=end_chunk)
			.map(|i| kv::get_chunk_key(file.file_tag, i as u32).to_vec())
			.collect();

		let resp = match ctx.kv_get(chunk_keys) {
			Ok(r) => r,
			Err(_) => return SQLITE_IOERR_READ,
		};
		let value_map = build_value_map(&resp);

		// Copy chunk data to output buffer.
		for i in start_chunk..=end_chunk {
			let chunk_offset = i * kv::CHUNK_SIZE;
			let read_start = offset.saturating_sub(chunk_offset);
			let read_end = std::cmp::min(kv::CHUNK_SIZE, offset + amt - chunk_offset);
			let dest_start = chunk_offset + read_start - offset;

			let key = kv::get_chunk_key(file.file_tag, i as u32);
			if let Some(chunk) = value_map.get(key.as_slice()) {
				let src_end = std::cmp::min(read_end, chunk.len());
				if src_end > read_start {
					let dest_end = dest_start + (src_end - read_start);
					buf[dest_start..dest_end].copy_from_slice(&chunk[read_start..src_end]);
				}
				// Zero-fill if chunk is shorter than expected.
				if src_end < read_end {
					let zero_start = dest_start + src_end.saturating_sub(read_start);
					let zero_end = dest_start + (read_end - read_start);
					buf[zero_start..zero_end].fill(0);
				}
			} else {
				// Missing chunk: zero-fill.
				let dest_end = dest_start + (read_end - read_start);
				buf[dest_start..dest_end].fill(0);
			}
		}

		// Short read if requested range extends past file size.
		let actual = std::cmp::min(amt, file_size - offset);
		if actual < amt {
			buf[actual..].fill(0);
			return SQLITE_IOERR_SHORT_READ;
		}

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
		let file = get_file(p_file);
		let ctx = &*file.ctx;
		let amt = i_amt as usize;
		let data = slice::from_raw_parts(p_buf as *const u8, amt);

		if i_offset < 0 {
			return SQLITE_IOERR_WRITE;
		}

		let offset = i_offset as usize;
		let write_end = offset + amt;
		let file_size = file.size as usize;

		let start_chunk = offset / kv::CHUNK_SIZE;
		let end_chunk = (offset + amt - 1) / kv::CHUNK_SIZE;

		// Identify chunks that need existing data prefetched (partial overwrites).
		struct ChunkPlan {
			chunk_idx: usize,
			w_start: usize,
			w_end: usize,
			prefetch_idx: Option<usize>,
		}

		let mut prefetch_keys: Vec<Vec<u8>> = Vec::new();
		let mut plans: Vec<ChunkPlan> = Vec::new();

		for i in start_chunk..=end_chunk {
			let chunk_offset = i * kv::CHUNK_SIZE;
			let w_start = offset.saturating_sub(chunk_offset);
			let w_end = std::cmp::min(kv::CHUNK_SIZE, write_end - chunk_offset);
			let existing_in_chunk = if file_size > chunk_offset {
				std::cmp::min(kv::CHUNK_SIZE, file_size - chunk_offset)
			} else {
				0
			};
			let needs_existing = w_start > 0 || existing_in_chunk > w_end;
			let prefetch_idx = if needs_existing {
				let idx = prefetch_keys.len();
				prefetch_keys.push(kv::get_chunk_key(file.file_tag, i as u32).to_vec());
				Some(idx)
			} else {
				None
			};
			plans.push(ChunkPlan {
				chunk_idx: i,
				w_start,
				w_end,
				prefetch_idx,
			});
		}

		// Prefetch existing chunks that need partial preservation.
		let prefetched: Vec<Option<Vec<u8>>> = if !prefetch_keys.is_empty() {
			match ctx.kv_get(prefetch_keys.clone()) {
				Ok(r) => {
					let map = build_value_map(&r);
					prefetch_keys
						.iter()
						.map(|k| map.get(k.as_slice()).map(|v| v.to_vec()))
						.collect()
				}
				Err(_) => return SQLITE_IOERR_WRITE,
			}
		} else {
			Vec::new()
		};

		// Build the write batch.
		let mut put_keys: Vec<Vec<u8>> = Vec::new();
		let mut put_values: Vec<Vec<u8>> = Vec::new();

		for plan in &plans {
			let existing = plan
				.prefetch_idx
				.and_then(|idx| prefetched.get(idx))
				.and_then(|v| v.as_ref());

			let new_chunk = if let Some(existing_chunk) = existing {
				let new_len = std::cmp::max(existing_chunk.len(), plan.w_end);
				let mut chunk = vec![0u8; new_len];
				chunk[..existing_chunk.len()].copy_from_slice(existing_chunk);
				let src_start = plan.chunk_idx * kv::CHUNK_SIZE + plan.w_start - offset;
				let src_len = plan.w_end - plan.w_start;
				chunk[plan.w_start..plan.w_end]
					.copy_from_slice(&data[src_start..src_start + src_len]);
				chunk
			} else {
				let mut chunk = vec![0u8; plan.w_end];
				let src_start = plan.chunk_idx * kv::CHUNK_SIZE + plan.w_start - offset;
				let src_len = plan.w_end - plan.w_start;
				chunk[plan.w_start..plan.w_end]
					.copy_from_slice(&data[src_start..src_start + src_len]);
				chunk
			};

			put_keys.push(kv::get_chunk_key(file.file_tag, plan.chunk_idx as u32).to_vec());
			put_values.push(new_chunk);
		}

		// Update file size if we wrote past the end.
		let new_size = std::cmp::max(file.size, write_end as i64);
		if new_size != file.size {
			file.size = new_size;
			file.meta_dirty = true;
		}

		// Include metadata in the write batch if dirty.
		if file.meta_dirty {
			put_keys.push(file.meta_key.to_vec());
			put_values.push(encode_file_meta(file.size));
		}

		if ctx.kv_put(put_keys, put_values).is_err() {
			return SQLITE_IOERR_WRITE;
		}

		if file.meta_dirty {
			file.meta_dirty = false;
		}

		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_truncate(
	p_file: *mut sqlite3_file,
	size: sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_TRUNCATE, {
		let file = get_file(p_file);
		let ctx = &*file.ctx;

		if size < 0 {
			return SQLITE_IOERR_TRUNCATE;
		}

		// Truncating to a larger size: just update metadata.
		if size >= file.size {
			if size > file.size {
				file.size = size;
				file.meta_dirty = true;
				let meta = encode_file_meta(file.size);
				if ctx
					.kv_put(vec![file.meta_key.to_vec()], vec![meta])
					.is_err()
				{
					return SQLITE_IOERR_TRUNCATE;
				}
				file.meta_dirty = false;
			}
			return SQLITE_OK;
		}

		// Delete chunks beyond the new size using O(1) range delete.
		// Chunk keys are lexicographically contiguous per file tag due to the
		// fixed prefix + big-endian chunk index layout.
		let new_size = size as usize;
		if new_size == 0 {
			// Delete all chunks.
			if ctx
				.kv_delete_range(
					kv::get_chunk_key(file.file_tag, 0).to_vec(),
					kv::get_chunk_key_range_end(file.file_tag).to_vec(),
				)
				.is_err()
			{
				return SQLITE_IOERR_TRUNCATE;
			}
		} else {
			let first_delete = ((new_size - 1) / kv::CHUNK_SIZE + 1) as u32;
			if ctx
				.kv_delete_range(
					kv::get_chunk_key(file.file_tag, first_delete).to_vec(),
					kv::get_chunk_key_range_end(file.file_tag).to_vec(),
				)
				.is_err()
			{
				return SQLITE_IOERR_TRUNCATE;
			}

			// Truncate the last kept chunk if it has trailing data beyond the new size.
			if new_size % kv::CHUNK_SIZE != 0 {
				let last_idx = first_delete - 1;
				let last_key = kv::get_chunk_key(file.file_tag, last_idx);
				match ctx.kv_get(vec![last_key.to_vec()]) {
					Ok(resp) => {
						let map = build_value_map(&resp);
						if let Some(chunk) = map.get(last_key.as_slice()) {
							if chunk.len() > new_size % kv::CHUNK_SIZE {
								let truncated = chunk[..new_size % kv::CHUNK_SIZE].to_vec();
								if let Err(err) = ctx.kv_put(vec![last_key.to_vec()], vec![truncated]) {
									tracing::error!(%err, file_tag = file.file_tag, "failed to write truncated chunk");
									return SQLITE_IOERR_TRUNCATE;
								}
							}
						}
					}
					Err(err) => {
						tracing::error!(%err, file_tag = file.file_tag, "failed to read chunk for truncation");
						return SQLITE_IOERR_TRUNCATE;
					}
				}
			}
		}

		// Update metadata.
		file.size = size;
		file.meta_dirty = true;
		let meta = encode_file_meta(file.size);
		if ctx
			.kv_put(vec![file.meta_key.to_vec()], vec![meta])
			.is_err()
		{
			return SQLITE_IOERR_TRUNCATE;
		}
		file.meta_dirty = false;

		SQLITE_OK
	})
}

/// Flush dirty metadata via KvPutRequest.
unsafe extern "C" fn kv_io_sync(
	p_file: *mut sqlite3_file,
	_flags: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		if !file.meta_dirty {
			return SQLITE_OK;
		}
		let ctx = &*file.ctx;
		let meta = encode_file_meta(file.size);
		if ctx
			.kv_put(vec![file.meta_key.to_vec()], vec![meta])
			.is_err()
		{
			return SQLITE_IOERR;
		}
		file.meta_dirty = false;
		SQLITE_OK
	})
}

/// Return cached file size (read from metadata on xOpen).
unsafe extern "C" fn kv_io_file_size(
	p_file: *mut sqlite3_file,
	p_size: *mut sqlite3_int64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let file = get_file(p_file);
		*p_size = file.size;
		SQLITE_OK
	})
}

/// No-op. Single-writer per actor is enforced by the KV channel lock.
unsafe extern "C" fn kv_io_lock(
	_p_file: *mut sqlite3_file,
	_level: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_LOCK, SQLITE_OK)
}

/// No-op. Single-writer per actor is enforced by the KV channel lock.
unsafe extern "C" fn kv_io_unlock(
	_p_file: *mut sqlite3_file,
	_level: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR_UNLOCK, SQLITE_OK)
}

unsafe extern "C" fn kv_io_check_reserved_lock(
	_p_file: *mut sqlite3_file,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		// Actor-scoped with one writer, no external reserved lock to report.
		*p_res_out = 0;
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_io_file_control(
	_p_file: *mut sqlite3_file,
	_op: c_int,
	_p_arg: *mut c_void,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, SQLITE_NOTFOUND)
}

unsafe extern "C" fn kv_io_sector_size(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(kv::CHUNK_SIZE as c_int, kv::CHUNK_SIZE as c_int)
}

unsafe extern "C" fn kv_io_device_characteristics(_p_file: *mut sqlite3_file) -> c_int {
	vfs_catch_unwind!(0, 0)
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
		let ctx = get_vfs_ctx(p_vfs);

		if z_name.is_null() {
			return SQLITE_CANTOPEN;
		}
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(s) => s,
			Err(_) => return SQLITE_CANTOPEN,
		};

		let file_tag = match ctx.resolve_file_tag(path) {
			Some(tag) => tag,
			None => return SQLITE_CANTOPEN,
		};

		let meta_key = kv::get_meta_key(file_tag);

		// Check if the file exists by reading its metadata key.
		let size = match ctx.kv_get(vec![meta_key.to_vec()]) {
			Ok(resp) => {
				let map = build_value_map(&resp);
				if let Some(data) = map.get(meta_key.as_slice()) {
					match decode_file_meta(data) {
						Some(s) if s >= 0 => s,
						_ => return SQLITE_CANTOPEN,
					}
				} else if flags & SQLITE_OPEN_CREATE != 0 {
					// File does not exist. Create it with size 0.
					let meta = encode_file_meta(0);
					if ctx
						.kv_put(vec![meta_key.to_vec()], vec![meta])
						.is_err()
					{
						return SQLITE_CANTOPEN;
					}
					0i64
				} else {
					return SQLITE_CANTOPEN;
				}
			}
			Err(_) => return SQLITE_CANTOPEN,
		};

		// Initialize our file struct.
		let file = get_file(p_file);
		file.base.pMethods = ctx.io_methods.as_ref() as *const sqlite3_io_methods;
		file.ctx = ctx as *const VfsContext;
		file.file_tag = file_tag;
		file.meta_key = meta_key;
		file.size = size;
		file.meta_dirty = false;
		file.flags = flags;

		if !p_out_flags.is_null() {
			*p_out_flags = flags;
		}

		SQLITE_OK
	})
}

/// Delete a file by removing all its chunks and metadata.
/// Uses KvDeleteRangeRequest for O(1) chunk deletion.
unsafe extern "C" fn kv_vfs_delete(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_sync_dir: c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let ctx = get_vfs_ctx(p_vfs);

		if z_name.is_null() {
			return SQLITE_OK;
		}
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(s) => s,
			Err(_) => return SQLITE_IOERR,
		};

		let file_tag = match ctx.resolve_file_tag(path) {
			Some(tag) => tag,
			None => return SQLITE_OK,
		};

		let meta_key = kv::get_meta_key(file_tag);

		// Check if the file exists.
		let exists = match ctx.kv_get(vec![meta_key.to_vec()]) {
			Ok(resp) => {
				let map = build_value_map(&resp);
				map.contains_key(meta_key.as_slice())
			}
			Err(_) => return SQLITE_IOERR,
		};

		if !exists {
			return SQLITE_OK;
		}

		// Delete all chunks via range delete.
		if ctx
			.kv_delete_range(
				kv::get_chunk_key(file_tag, 0).to_vec(),
				kv::get_chunk_key_range_end(file_tag).to_vec(),
			)
			.is_err()
		{
			return SQLITE_IOERR;
		}

		// Delete the metadata key.
		if ctx.kv_delete(vec![meta_key.to_vec()]).is_err() {
			return SQLITE_IOERR;
		}

		SQLITE_OK
	})
}

/// Check whether a file exists by checking its metadata key via KvGetRequest.
unsafe extern "C" fn kv_vfs_access(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_flags: c_int,
	p_res_out: *mut c_int,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		let ctx = get_vfs_ctx(p_vfs);

		if z_name.is_null() {
			*p_res_out = 0;
			return SQLITE_OK;
		}
		let path = match CStr::from_ptr(z_name).to_str() {
			Ok(s) => s,
			Err(_) => {
				*p_res_out = 0;
				return SQLITE_OK;
			}
		};

		let file_tag = match ctx.resolve_file_tag(path) {
			Some(tag) => tag,
			None => {
				*p_res_out = 0;
				return SQLITE_OK;
			}
		};

		let meta_key = kv::get_meta_key(file_tag);
		let exists = match ctx.kv_get(vec![meta_key.to_vec()]) {
			Ok(resp) => {
				let map = build_value_map(&resp);
				map.contains_key(meta_key.as_slice())
			}
			Err(_) => {
				*p_res_out = 0;
				return SQLITE_OK;
			}
		};

		*p_res_out = if exists { 1 } else { 0 };
		SQLITE_OK
	})
}

unsafe extern "C" fn kv_vfs_full_pathname(
	_p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	n_out: c_int,
	z_out: *mut c_char,
) -> c_int {
	vfs_catch_unwind!(SQLITE_CANTOPEN, {
		if z_name.is_null() {
			return SQLITE_CANTOPEN;
		}
		let name = CStr::from_ptr(z_name);
		let bytes = name.to_bytes_with_nul();
		if bytes.len() > n_out as usize {
			return SQLITE_CANTOPEN;
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
		let seed = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos();
		// LCG PRNG, adequate for SQLite's randomness needs.
		let mut state = seed as u64;
		for byte in buf.iter_mut() {
			state = state
				.wrapping_mul(6364136223846793005)
				.wrapping_add(1442695040888963407);
			*byte = (state >> 33) as u8;
		}
		n_byte
	})
}

unsafe extern "C" fn kv_vfs_sleep(
	_p_vfs: *mut sqlite3_vfs,
	microseconds: c_int,
) -> c_int {
	vfs_catch_unwind!(0, {
		std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
		microseconds
	})
}

unsafe extern "C" fn kv_vfs_current_time(
	_p_vfs: *mut sqlite3_vfs,
	p_time_out: *mut f64,
) -> c_int {
	vfs_catch_unwind!(SQLITE_IOERR, {
		// Julian day number. Unix epoch = Julian day 2440587.5.
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

/// A registered SQLite VFS backed by KV operations over a WebSocket channel.
///
/// Each actor gets its own VFS instance parameterized by actor ID.
/// The VFS is automatically unregistered when dropped.
pub struct KvVfs {
	vfs_ptr: *mut sqlite3_vfs,
	_name: CString,
	ctx_ptr: *mut VfsContext,
}

unsafe impl Send for KvVfs {}
unsafe impl Sync for KvVfs {}

impl KvVfs {
	/// Register a new KV VFS with SQLite.
	///
	/// `name` must be unique across all registered VFS instances (e.g., `kv-{actor_id}`).
	/// `rt_handle` must point to a running tokio runtime. VFS callbacks block on this
	/// runtime for async KV channel operations, so they must not be called from within
	/// a tokio async context.
	pub fn register(
		name: &str,
		channel: Arc<KvChannel>,
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

		let ctx = Box::new(VfsContext {
			channel,
			actor_id: actor_id.clone(),
			main_file_name: actor_id,
			rt_handle,
			io_methods: Box::new(io_methods),
		});
		let ctx_ptr = Box::into_raw(ctx);

		let name_cstring = CString::new(name).map_err(|e| e.to_string())?;

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
				let _ = Box::from_raw(vfs_ptr);
				let _ = Box::from_raw(ctx_ptr);
			}
			return Err(format!("sqlite3_vfs_register failed with code {rc}"));
		}

		Ok(KvVfs {
			vfs_ptr,
			_name: name_cstring,
			ctx_ptr,
		})
	}

	/// The VFS name as a C string pointer. Valid for the lifetime of this handle.
	pub fn name_ptr(&self) -> *const c_char {
		self._name.as_ptr()
	}
}

impl Drop for KvVfs {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs_ptr);
			let _ = Box::from_raw(self.vfs_ptr);
			let _ = Box::from_raw(self.ctx_ptr);
		}
	}
}

// MARK: NativeDatabase

/// An open SQLite database backed by KV storage.
///
/// Owns the underlying VFS registration. The database is closed and VFS
/// unregistered when this struct is dropped.
pub struct NativeDatabase {
	db: *mut sqlite3,
	_vfs: KvVfs,
}

unsafe impl Send for NativeDatabase {}

impl NativeDatabase {
	/// Raw sqlite3 pointer for FFI usage.
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

/// Open a SQLite database backed by the given KV VFS and apply PRAGMA settings.
///
/// PRAGMA settings (keep in sync with `rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`
/// and `docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md`):
/// - `page_size = 4096`: matches CHUNK_SIZE for page-aligned writes.
/// - `busy_timeout = 5000`: wait 5 seconds for locked databases.
pub fn open_database(vfs: KvVfs, file_name: &str) -> Result<NativeDatabase, String> {
	let c_name = CString::new(file_name).map_err(|e| e.to_string())?;
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

	// PRAGMA settings for KV-backed SQLite. Keep in sync with WASM VFS
	// (rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts) and spec doc
	// (docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md).
	// page_size=4096 matches CHUNK_SIZE. busy_timeout=5000 matches the
	// file-system driver default. WAL mode is not enabled.
	for pragma in &[
		"PRAGMA page_size = 4096;",
		"PRAGMA busy_timeout = 5000;",
	] {
		let c_sql = CString::new(*pragma).unwrap();
		let rc = unsafe {
			sqlite3_exec(
				db,
				c_sql.as_ptr(),
				None,
				ptr::null_mut(),
				ptr::null_mut(),
			)
		};
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
			// First 2 bytes are u16 LE version
			assert_eq!(&encoded[0..2], &META_VERSION.to_le_bytes());
			let decoded = decode_file_meta(&encoded).unwrap();
			assert_eq!(decoded, size);
		}
	}

	#[test]
	fn encode_zero_size() {
		let encoded = encode_file_meta(0);
		// 2-byte u16 LE version (1) + 8-byte u64 LE size (0)
		assert_eq!(encoded, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	}

	#[test]
	fn encode_known_size() {
		// 4096 = 0x1000, LE bytes: [0x00, 0x10, 0x00, ...]
		let encoded = encode_file_meta(4096);
		// 2-byte u16 LE version (1) + 8-byte u64 LE size (4096)
		assert_eq!(
			encoded,
			[1, 0, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
		);
	}

	#[test]
	fn decode_invalid_version() {
		// Version 2 encoded as u16 LE
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
		assert!(
			std::mem::size_of::<KvFile>() > std::mem::size_of::<sqlite3_file>()
		);
	}

	#[test]
	fn meta_encoded_size_constant() {
		// 2 bytes u16 LE version + 8 bytes u64 LE size
		assert_eq!(META_ENCODED_SIZE, 10);
	}

	#[test]
	fn meta_version_matches_wasm_vfs() {
		// Must match CURRENT_VERSION in sqlite-vfs/schemas/file-meta/versioned.ts.
		assert_eq!(META_VERSION, 1);
	}

	#[test]
	fn encode_matches_vbare_format() {
		// Verify byte layout matches vbare's serializeWithEmbeddedVersion:
		// 2-byte u16 LE version prefix + BARE-encoded payload
		let encoded = encode_file_meta(42);
		// Version 1 as u16 LE = [0x01, 0x00]
		assert_eq!(encoded[0], 0x01);
		assert_eq!(encoded[1], 0x00);
		// Size 42 as u64 LE = [42, 0, 0, 0, 0, 0, 0, 0]
		assert_eq!(&encoded[2..], &42u64.to_le_bytes());
	}
}
