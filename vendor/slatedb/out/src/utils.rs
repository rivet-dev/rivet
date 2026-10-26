use crate::block_iterator::BlockIterator;
use crate::block_iterator_v2::BlockIteratorV2;
use crate::cached_object_store::CachedObjectStore;
use crate::config::PreloadLevel;
use crate::db_state::SortedRun;
use crate::db_state::SsTableHandle;
use crate::error::SlateDBError;
use crate::format::sst::{SST_FORMAT_VERSION, SST_FORMAT_VERSION_V2};
use crate::iter::{IterationOrder, RowEntryIterator};
use crate::manifest::ManifestCore;
use crate::paths::PathResolver;
use crate::tablestore::TableStore;
use bytes::{Buf, BufMut, Bytes};
use futures::FutureExt;
use log::{error, warn};
use rand::{Rng, RngCore};
use slatedb_common::clock::SystemClock;
use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use ulid::Ulid;
use uuid::Uuid;

use futures::StreamExt;
use std::collections::VecDeque;

static EMPTY_KEY: Bytes = Bytes::new();

/// Whether the object store holds the main data path or the WAL.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ObjectStoreType {
    /// The primary object store (SSTs, manifests, compacted data).
    Main,
    /// The dedicated WAL object store, when configured separately.
    Wal,
}

impl ObjectStoreType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Wal => "wal",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WatchableOnceCell<T: Clone> {
    rx: tokio::sync::watch::Receiver<Option<T>>,
    tx: tokio::sync::watch::Sender<Option<T>>,
}

#[derive(Clone, Debug)]
pub(crate) struct WatchableOnceCellReader<T: Clone> {
    rx: tokio::sync::watch::Receiver<Option<T>>,
}

impl<T: Clone> WatchableOnceCell<T> {
    pub(crate) fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(None);
        Self { rx, tx }
    }

    /// Writes the value if not already set. Returns `true` if the value was
    /// written, `false` if a value was already present.
    pub(crate) fn write(&self, val: T) -> bool {
        self.tx.send_if_modified(|v| {
            if v.is_some() {
                return false;
            }
            v.replace(val);
            true
        })
    }

    pub(crate) fn reader(&self) -> WatchableOnceCellReader<T> {
        WatchableOnceCellReader {
            rx: self.rx.clone(),
        }
    }
}

impl<T: Clone> WatchableOnceCellReader<T> {
    pub(crate) fn read(&self) -> Option<T> {
        self.rx.borrow().clone()
    }

    pub(crate) async fn await_value(&mut self) -> T {
        self.rx
            .wait_for(|v| v.is_some())
            .await
            .expect("watch channel closed")
            .clone()
            .expect("no value found")
    }
}

/// Spawn a background tokio task. The task must return a Result<T, SlateDBError>.
/// When the task exits, the provided cleanup fn is called with a reference to the returned
/// result. If the task panics, the cleanup fn is called with Err(BackgroundTaskPanic).
pub(crate) fn spawn_bg_task<F, T, C>(
    name: String,
    handle: &tokio::runtime::Handle,
    cleanup_fn: C,
    future: F,
) -> tokio::task::JoinHandle<Result<T, SlateDBError>>
where
    F: Future<Output = Result<T, SlateDBError>> + Send + 'static,
    T: Send + 'static,
    C: FnOnce(&Result<T, SlateDBError>) + Send + 'static,
{
    // NOTE: It is critical that the future lives as long as the cleanup_fn.
    //       Otherwise, there is a gap where everything owned by the future is dropped
    //       before the cleanup_fn runs. Since our cleanup_fn's often set error states
    //       on the db, this would result in a gap where the db is not in an error state
    //       but resources such as channels have been dropped or closed. See #623 for
    //       details.
    let wrapped = AssertUnwindSafe(future).catch_unwind().map(move |outcome| {
        let result = match outcome {
            Ok(result) => result,
            Err(payload) => {
                error!(
                    "spawned task panicked. [name={}, panic={}]",
                    name,
                    panic_string(&payload)
                );
                Err(SlateDBError::BackgroundTaskPanic(name))
            }
        };
        cleanup_fn(&result);
        result
    });
    handle.spawn(wrapped)
}

/// Merge two options using the provided function.
pub(crate) fn merge_options<T>(
    current: Option<T>,
    next: Option<T>,
    f: impl Fn(T, T) -> T,
) -> Option<T> {
    match (current, next) {
        (Some(current), Some(next)) => Some(f(current, next)),
        (None, next) => next,
        (current, None) => current,
    }
}

/// Determines the last key and sequence number written by an output SST.
///
/// ## Arguments
/// - `table_store`: Table store for reading the SST index and blocks.
/// - `output_sst`: Output SST already written for a compaction being resumed.
/// - `segment`: Segment containing the output SST.
///
/// ## Returns
/// - `Ok(Some((Bytes, u64)))`: last key and sequence number from the final block.
/// - `Ok(None)`: when the SST contains no data blocks.
///
/// ## Errors
/// - `SlateDBError`: if reading the index or blocks fails.
pub(crate) async fn last_written_key_and_seq(
    table_store: Arc<TableStore>,
    output_sst: &SsTableHandle,
    segment: &Bytes,
) -> Result<Option<(Bytes, u64)>, SlateDBError> {
    let index = table_store
        .read_index(
            output_sst,
            false,
            Some(segment.clone()),
            &crate::reader::ReadTrace::new(None),
            None,
        )
        .await?;
    let num_blocks = index.borrow().block_meta().len();
    if num_blocks == 0 {
        return Ok(None);
    }
    let last_block_idx = num_blocks - 1;
    let mut blocks = table_store
        .read_blocks_using_index(
            output_sst,
            index,
            last_block_idx..last_block_idx + 1,
            false,
            Some(segment.clone()),
        )
        .await?;
    let Some(block) = blocks.pop_front() else {
        return Ok(None);
    };

    // Sort descending so we get the last row from the last block, which
    // should be the last written key/seq.
    let entry = match output_sst.format_version {
        SST_FORMAT_VERSION => {
            let mut block_iter = BlockIterator::new(block, IterationOrder::Descending);
            block_iter.init().await?;
            block_iter.next().await?
        }
        SST_FORMAT_VERSION_V2 => {
            let mut block_iter = BlockIteratorV2::new(block, IterationOrder::Descending);
            block_iter.init().await?;
            block_iter.next().await?
        }
        _ => {
            return Err(SlateDBError::InvalidVersion {
                format_name: "SST",
                supported_versions: vec![SST_FORMAT_VERSION, SST_FORMAT_VERSION_V2],
                actual_version: output_sst.format_version,
            });
        }
    };
    Ok(entry.map(|e| (e.key, e.seq)))
}

fn bytes_into_minimal_vec(bytes: &Bytes) -> Vec<u8> {
    let mut clamped = Vec::new();
    clamped.reserve_exact(bytes.len());
    clamped.put_slice(bytes.as_ref());
    clamped
}

pub(crate) fn clamp_allocated_size_bytes(bytes: &Bytes) -> Bytes {
    bytes_into_minimal_vec(bytes).into()
}

/// Computes the "index key" (lowest bound) for an SST index block, ie a key that's greater
/// than all keys in the previous block and less than or equal to all keys in the new block
pub(crate) fn compute_index_key(
    prev_block_last_key: Option<Bytes>,
    this_block_first_key: &Bytes,
) -> Bytes {
    if let Some(prev_key) = prev_block_last_key {
        compute_lower_bound(&prev_key, this_block_first_key)
    } else {
        EMPTY_KEY.clone()
    }
}

fn compute_lower_bound(prev_block_last_key: &Bytes, this_block_first_key: &Bytes) -> Bytes {
    assert!(!prev_block_last_key.is_empty() && !this_block_first_key.is_empty());

    for i in 0..prev_block_last_key.len() {
        if prev_block_last_key[i] != this_block_first_key[i] {
            return this_block_first_key.slice(..i + 1);
        }
    }

    // if the keys are equal, just use the full key
    if prev_block_last_key.len() == this_block_first_key.len() {
        return this_block_first_key.clone();
    }

    // if we didn't find a mismatch yet then the prev block's key must be shorter,
    // so just use the common prefix plus the next byte in this block's key
    this_block_first_key.slice(..prev_block_last_key.len() + 1)
}

/// Trait for generating UUIDs and ULIDs from a random number generator.
pub(crate) trait IdGenerator {
    fn gen_uuid(&mut self) -> Uuid;
    fn gen_ulid(&mut self, clock: &dyn SystemClock) -> Ulid;
}

impl<R: RngCore> IdGenerator for R {
    /// Generates a random UUID using the provided RNG.
    fn gen_uuid(&mut self) -> Uuid {
        let mut bytes = [0u8; 16];
        self.fill_bytes(&mut bytes);
        // set version = 4
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        // set variant = RFC4122
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes)
    }

    /// Generates a random ULID using the provided RNG. The clock is used to generate
    /// the timestamp component of the ULID.
    fn gen_ulid(&mut self, clock: &dyn SystemClock) -> Ulid {
        let now = u64::try_from(clock.now().timestamp_millis())
            .expect("timestamp outside u64 range in gen_ulid");
        let random_bytes = self.random::<u128>();
        Ulid::from_parts(now, random_bytes)
    }
}

/// A simple bit-level writer that packs bits into a `[u8]`
pub(crate) struct BitWriter {
    buf: Vec<u8>,
    cur: u8, // the currently assembling byte
    n: u8,   // the number of "filled" bytes in `cur`
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        BitWriter {
            buf: Vec::new(),
            cur: 0,
            n: 0,
        }
    }

    /// Push a single bit into the buffer
    pub(crate) fn push(&mut self, bit: bool) {
        if bit {
            self.cur |= 1 << (7 - self.n);
        }
        self.n += 1;
        if self.n == 8 {
            self.flush_byte();
        }
    }

    /// Push up to a u32 into the buffer, only considering
    /// the first `bits` number of bits
    pub(crate) fn push32(&mut self, value: u32, bits: u8) {
        // writes the lowest `bits` bits from `value` into
        // the current buffer (most significant bits first)
        for i in (0..bits).rev() {
            let bit = ((value >> i) & 1) != 0;
            self.push(bit);
        }
    }

    /// Push up to a u32 into the buffer, only considering
    /// the first `bits` number of bits
    pub(crate) fn push64(&mut self, value: u64, bits: u8) {
        for i in (0..bits).rev() {
            let bit = ((value >> i) & 1) != 0;
            self.push(bit);
        }
    }

    fn flush_byte(&mut self) {
        self.buf.push(self.cur);
        self.cur = 0;
        self.n = 0;
    }

    /// Extract the finalized buffer from the writer
    pub(crate) fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.buf.push(self.cur);
        }
        self.buf
    }
}

pub(crate) struct BitReader<'a> {
    buf: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0..8; next bit to read is at (7 - bit_pos)
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        BitReader {
            buf,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Read one bit, or None if we've exhausted the buffer.
    pub(crate) fn read_bit(&mut self) -> Option<bool> {
        if self.byte_pos >= self.buf.len() {
            return None;
        }
        let byte = self.buf[self.byte_pos];
        let bit = ((byte >> (7 - self.bit_pos)) & 1) != 0;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Some(bit)
    }

    /// Read `bits` bits, MSB first, returning them as the low `bits` of a u32.
    pub(crate) fn read32(&mut self, bits: u8) -> Option<u32> {
        let mut val = 0u32;
        for _ in 0..bits {
            val <<= 1;
            match self.read_bit() {
                Some(true) => val |= 1,
                Some(false) => (),
                None => return None,
            }
        }
        Some(val)
    }

    /// Read `bits` bits, MSB first, returning them as the low `bits` of a u64.
    pub(crate) fn read64(&mut self, bits: u8) -> Option<u64> {
        let mut val = 0u64;
        for _ in 0..bits {
            val <<= 1;
            match self.read_bit() {
                Some(true) => val |= 1,
                Some(false) => (),
                None => return None,
            }
        }
        Some(val)
    }
}

/// Sign‐extend the low `bits` of `val` into a full i32.
pub(crate) fn sign_extend(val: u32, bits: u8) -> i32 {
    let shift = 32 - bits;
    ((val << shift) as i32) >> shift
}

/// Compute a bounded concurrency for iterator construction.
///
/// Arguments:
/// - `l0_count`: number of L0 SSTables to scan
/// - `srs`: slice of sorted runs to scan; each run’s SST count is summed
/// - `cap`: hard ceiling for concurrency (minimum effective value is 1)
///
/// Returns:
/// - The effective max parallelism.
pub(crate) fn compute_max_parallel(l0_count: usize, srs: &[SortedRun], cap: usize) -> usize {
    let total_ssts = l0_count + srs.iter().map(|sr| sr.sst_views().len()).sum::<usize>();
    total_ssts.min(cap).max(1)
}

/// Estimate the total number of bytes before `key` across sorted runs.
///
/// This is a best-effort estimate based on SST boundaries and metadata-only
/// size estimates; it does not read SST contents. For exmple, if we have:
///
/// - Run 1: SSTs [a (10 bytes), k (20 bytes), z (30 bytes)]
/// - Run 2: SSTs [b (40 bytes), f (50 bytes)]
///
/// and we call `estimate_bytes_before_key` with `key = "m"`, the result will be:
///
/// - From Run 1: SST "a" (10 bytes) is before "m", SST "k" and "z" are after because
///   k < m < z.
/// - From Run 2: SST "b" (40 bytes) is before "m", SST "f" is after because
///   f < m and it's the last SST.
pub(crate) fn estimate_bytes_before_key(sorted_runs: &[SortedRun], key: &Bytes) -> u64 {
    sorted_runs
        .iter()
        .map(|sorted_run| {
            let Some(idx) = sorted_run.find_last_sst_with_range_covering_key(key) else {
                return 0;
            };
            sorted_run
                .sst_views()
                .iter()
                .take(idx)
                .map(|sst| sst.estimate_size())
                .sum::<u64>()
        })
        .sum()
}

/// Concurrently build items with a bounded level of parallelism.
///
/// This function maps each input to an async task using the provided factory `f`,
/// runs up to `max_parallel` tasks at once, and collects all successful results.
/// If any task returns `Err`, the first error is returned.
///
/// Arguments:
/// - `inputs`: the items to process
/// - `max_parallel`: maximum number of in-flight tasks (values <= 0 are clamped to 1)
/// - `f`: per-item async factory that returns `Result<Option<T>, SlateDBError>`
///
/// Returns:
/// - `Ok(VecDeque<T>)` containing all successfully built items (in completion order)
/// - `Err(SlateDBError)` if any task fails (short-circuits on the first error observed)
///
/// Concurrency & ordering:
/// - Tasks are polled concurrently up to `max_parallel` using `buffer_unordered`.
/// - Completion order is not guaranteed; results are pushed as tasks finish.
#[allow(clippy::redundant_closure)]
pub(crate) async fn build_concurrent<I, T, F, Fut>(
    inputs: I,
    max_parallel: usize,
    f: F,
) -> Result<VecDeque<T>, SlateDBError>
where
    I: IntoIterator,
    I::Item: Send,
    T: Send,
    F: Fn(I::Item) -> Fut + Send,
    Fut: Future<Output = Result<Option<T>, SlateDBError>> + Send,
{
    let mut out = VecDeque::new();

    let results = futures::stream::iter(inputs.into_iter().map(move |it| f(it)))
        .buffer_unordered(max_parallel.max(1))
        .collect::<Vec<_>>()
        .await;

    for r in results {
        match r {
            Ok(Some(t)) => out.push_back(t),
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// Returns a string representation of a panic. The following panic types are
/// converted to their string representation:
///
/// - Result<(), SlateDBError>
/// - SlateDBError
/// - Box<dyn std::error::Error>
/// - String
/// - &'static str
///
/// Other panic types are handled by printing a generic message with the type name.
pub(crate) fn panic_string(panic: &Box<dyn Any + Send>) -> String {
    if let Some(result) = panic.downcast_ref::<Result<(), SlateDBError>>() {
        match result {
            Ok(()) => "ok".to_string(),
            Err(e) => e.to_string(),
        }
    } else if let Some(err) = panic.downcast_ref::<SlateDBError>() {
        err.to_string()
    } else if let Some(err) = panic.downcast_ref::<Box<dyn std::error::Error>>() {
        err.to_string()
    } else if let Some(err) = panic.downcast_ref::<String>() {
        err.clone()
    } else if let Some(err) = panic.downcast_ref::<&str>() {
        err.to_string()
    } else {
        format!(
            "task panicked with unknown type [type_id=`{:?}`]",
            (**panic).type_id()
        )
    }
}

/// Splits a `catch_unwind` result
/// (`Result<Result<(), SlateDBError>, Box<dyn std::any::Any + Send>>`) into a
/// `Result<(), SlateDBError>` and an optional `Box<dyn std::any::Any + Send>`
/// containing the panic payload.
///
/// # Arguments
///
/// * `name`: The name of the task
/// * `unwind_result`: The result of the catch_unwind
///
/// # Returns
///
/// - (Ok(()), None) if the task completed successfully
/// - (Err(SlateDBError:: .. ), None) if the task completed with an error
/// - (Err(SlateDBError::BackgroundTaskPanic), Some(payload)) if the task panicked
pub(crate) fn split_unwind_result(
    name: String,
    unwind_result: Result<Result<(), SlateDBError>, Box<dyn Any + Send>>,
) -> (Result<(), SlateDBError>, Option<Box<dyn Any + Send>>) {
    match unwind_result {
        Ok(result) => (result, None),
        Err(payload) => (Err(SlateDBError::BackgroundTaskPanic(name)), Some(payload)),
    }
}

/// Splits a `join` result
/// (`Result<Result<(), SlateDBError>, tokio::task::JoinError>`) into a
/// `Result<(), SlateDBError>` and an optional `Box<dyn std::any::Any + Send>`
/// containing the panic payload.
///
/// # Arguments
///
/// * `name`: The name of the task
/// * `join_result`: The result of the join handle
///
/// # Returns
///
/// - (Ok(()), None) if the task completed successfully
/// - (Err(SlateDBError:: .. ), None) if the task completed with an error
/// - (Err(SlateDBError::BackgroundTaskCancelled), None) if the task was cancelled
/// - (Err(SlateDBError::BackgroundTaskPanic), Some(payload)) if the task panicked
pub(crate) fn split_join_result(
    name: String,
    join_result: Result<Result<(), SlateDBError>, tokio::task::JoinError>,
) -> (Result<(), SlateDBError>, Option<Box<dyn Any + Send>>) {
    match join_result {
        Ok(task_result) => (task_result, None),
        Err(join_error) => {
            if join_error.is_cancelled() {
                (Err(SlateDBError::BackgroundTaskCancelled(name)), None)
            } else {
                let payload = join_error.into_panic();
                (Err(SlateDBError::BackgroundTaskPanic(name)), Some(payload))
            }
        }
    }
}

/// Formats a byte count as a human-readable string using SI units (KB, MB, GB, etc.).
///
/// Uses decimal (SI) prefixes where 1 KB = 1000 bytes.
///
/// # Examples
/// ```ignore
/// use slatedb::format_bytes_si;
///
/// assert_eq!(format_bytes_si(0), "0 B");
/// assert_eq!(format_bytes_si(999), "999 B");
/// assert_eq!(format_bytes_si(1000), "1.00 KB");
/// assert_eq!(format_bytes_si(1500), "1.50 KB");
/// assert_eq!(format_bytes_si(1_000_000), "1.00 MB");
/// ```
pub(crate) fn format_bytes_si(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    const FACTOR: f64 = 1000.0;

    if bytes < 1000 {
        return format!("{} B", bytes);
    }

    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= FACTOR && unit_index < UNITS.len() - 1 {
        value /= FACTOR;
        unit_index += 1;
    }

    format!("{:.2} {}", value, UNITS[unit_index])
}

// ============================================================================
// Varint encoding (LEB128)
// ============================================================================

/// Encode a u32 value using LEB128 varint encoding.
///
/// LEB128 (Little Endian Base 128) encodes integers in 7-bit groups,
/// using the high bit as a continuation flag. This allows small values
/// to be encoded in fewer bytes (e.g., values < 128 use only 1 byte).
#[allow(dead_code)]
pub(crate) fn encode_varint(buf: &mut Vec<u8>, mut value: u32) {
    while value >= 0x80 {
        buf.push((value as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

/// Decode a u32 value using LEB128 varint encoding.
///
/// Reads bytes from the buffer, advancing the slice, until a byte
/// without the continuation bit (high bit = 0) is found.
#[allow(dead_code)]
pub(crate) fn decode_varint(buf: &mut impl Buf) -> u32 {
    let mut result = 0u32;
    let mut shift = 0;
    loop {
        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

/// Calculate the encoded length of a u32 varint without actually encoding it.
#[allow(dead_code)]
pub(crate) fn varint_len(mut value: u32) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

/// Preload SST files into the disk cache based on the configured [`PreloadLevel`].
pub(crate) async fn preload_cache_from_manifest(
    core: &ManifestCore,
    cached_obj_store: &CachedObjectStore,
    path_resolver: &PathResolver,
    preload_level: Option<PreloadLevel>,
    max_cache_size: usize,
) -> Result<(), SlateDBError> {
    match preload_level {
        Some(PreloadLevel::AllSst) => {
            let all_sst_paths: Vec<object_store::path::Path> = core
                .all_sst_views()
                .map(|view| path_resolver.sst_path(&view.sst.id))
                .collect();
            if !all_sst_paths.is_empty() {
                if let Err(e) = cached_obj_store
                    .load_files_to_cache(all_sst_paths, max_cache_size)
                    .await
                {
                    warn!("Failed to preload all SSTs to cache: {:?}", e);
                }
            }
        }
        Some(PreloadLevel::L0Sst) => {
            let l0_sst_paths: Vec<object_store::path::Path> = core
                .trees()
                .flat_map(|tree| tree.l0.iter())
                .map(|view| path_resolver.sst_path(&view.sst.id))
                .collect();
            if !l0_sst_paths.is_empty() {
                if let Err(e) = cached_obj_store
                    .load_files_to_cache(l0_sst_paths, max_cache_size)
                    .await
                {
                    warn!("Failed to preload L0 SSTs to cache: {:?}", e);
                }
            }
        }
        None => {
            // No preloading
        }
    }
    Ok(())
}

/// A channel sender that checks the DB's closed result when the underlying
/// channel is closed, converting the raw channel error into the appropriate
/// [`SlateDBError`].
///
/// Use [`SafeSender::unbounded_channel`] to construct a channel wired to a
/// [`crate::db_status::ClosedResultWriter::result_reader`].
pub(crate) struct SafeSender<T> {
    tx: async_channel::Sender<T>,
    closed: WatchableOnceCellReader<Result<(), SlateDBError>>,
}

impl<T> SafeSender<T> {
    pub(crate) fn new(
        tx: async_channel::Sender<T>,
        closed: WatchableOnceCellReader<Result<(), SlateDBError>>,
    ) -> Self {
        Self { tx, closed }
    }

    pub(crate) fn unbounded_channel(
        closed: WatchableOnceCellReader<Result<(), SlateDBError>>,
    ) -> (Self, async_channel::Receiver<T>) {
        let (tx, rx) = async_channel::unbounded();
        (Self::new(tx, closed), rx)
    }

    /// Returns the DB's terminal error for a failed channel send.
    ///
    /// Panics if the channel is closed before the DB close result is recorded,
    /// which indicates a lifecycle ordering bug.
    #[allow(clippy::panic)]
    fn closed_send_error(&self, channel_error: String) -> SlateDBError {
        if let Some(result) = self.closed.read() {
            match result {
                Ok(()) => SlateDBError::Closed,
                Err(err) => err,
            }
        } else {
            panic!(
                "Failed to send message to unbounded channel: {}",
                channel_error
            );
        }
    }

    /// Attempts to send a message. If the channel is closed, returns the
    /// DB's closed result error, or [`SlateDBError::Closed`] if it was a
    /// clean shutdown. Panics if the channel is closed but no closed result
    /// has been set (indicates a bug).
    #[inline]
    pub(crate) fn send(&self, message: T) -> Result<(), SlateDBError> {
        match self.tx.try_send(message) {
            Ok(_) => Ok(()),
            Err(e) => Err(self.closed_send_error(e.to_string())),
        }
    }

    /// Like [`Self::send`], but invokes `on_closed` with the rejected message
    /// and closed result so callers can fail embedded response channels
    /// explicitly.
    #[inline]
    pub(crate) fn send_or_handle_closed<F>(
        &self,
        message: T,
        on_closed: F,
    ) -> Result<(), SlateDBError>
    where
        F: FnOnce(T, &SlateDBError),
    {
        match self.tx.try_send(message) {
            Ok(_) => Ok(()),
            Err(e) => {
                let channel_error = e.to_string();
                let message = e.into_inner();
                let err = self.closed_send_error(channel_error);
                on_closed(message, &err);
                Err(err)
            }
        }
    }
}

impl<T> Clone for SafeSender<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            closed: self.closed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use slatedb_common::MockSystemClock;

    use crate::clock::MonotonicClock;
    use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::error::SlateDBError;
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::sst_builder::BlockFormat;
    use crate::types::RowEntry;
    use crate::utils::{
        build_concurrent, bytes_into_minimal_vec, clamp_allocated_size_bytes, compute_index_key,
        compute_max_parallel, estimate_bytes_before_key, format_bytes_si, last_written_key_and_seq,
        panic_string, spawn_bg_task, BitReader, BitWriter, WatchableOnceCell,
    };
    use crate::Db;
    use bytes::{BufMut, Bytes, BytesMut};
    use object_store::memory::InMemory;
    use parking_lot::Mutex;
    use std::any::Any;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use ulid::Ulid;

    struct ResultCaptor<T: Clone> {
        error: Mutex<Option<Result<T, SlateDBError>>>,
    }

    impl<T: Clone> ResultCaptor<T> {
        fn new() -> Self {
            Self {
                error: Mutex::new(None),
            }
        }

        fn capture(&self, result: &Result<T, SlateDBError>) {
            let mut guard = self.error.lock();
            let prev = guard.replace(result.clone());
            assert!(prev.is_none());
        }

        fn captured(&self) -> Option<Result<T, SlateDBError>> {
            self.error.lock().clone()
        }
    }

    fn make_sst_view(start_key: &str, size: u64) -> SsTableView {
        let info = SsTableInfo {
            first_entry: Some(Bytes::from(start_key.as_bytes().to_vec())),
            index_offset: size.saturating_sub(1),
            index_len: 1,
            ..Default::default()
        };
        SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            info,
        ))
    }

    #[test]
    fn test_should_return_empty_for_index_of_first_block() {
        let this_block_first_key = Bytes::from(vec![0x01, 0x02, 0x03]);
        let result = compute_index_key(None, &this_block_first_key);

        assert_eq!(result, &Bytes::new());
    }

    #[rstest]
    #[case(Some("aaaac"), "abaaa", "ab")]
    #[case(Some("ababc"), "abacd", "abac")]
    #[case(Some("cc"), "ccccccc", "ccc")]
    #[case(Some("eed"), "eee", "eee")]
    #[case(Some("abcdef"), "abcdef", "abcdef")]
    fn test_should_compute_index_key(
        #[case] prev_block_last_key: Option<&'static str>,
        #[case] this_block_first_key: &'static str,
        #[case] expected_index_key: &'static str,
    ) {
        assert_eq!(
            compute_index_key(
                prev_block_last_key.map(|s| Bytes::from(s.to_string())),
                &Bytes::from(this_block_first_key.to_string())
            ),
            Bytes::from_static(expected_index_key.as_bytes())
        );
    }

    #[rstest]
    #[case(Some(""), "a")]
    #[case(Some("a"), "")]
    #[should_panic]
    fn test_should_panic_on_empty_keys(
        #[case] prev_block_last_key: Option<&'static str>,
        #[case] this_block_first_key: &'static str,
    ) {
        compute_index_key(
            prev_block_last_key.map(|s| Bytes::from(s.to_string())),
            &Bytes::from(this_block_first_key.to_string()),
        );
    }

    #[tokio::test]
    async fn test_should_cleanup_when_task_exits_with_error() {
        let captor = Arc::new(ResultCaptor::new());
        let handle = tokio::runtime::Handle::current();
        let captor2 = captor.clone();

        let task = spawn_bg_task(
            "test".to_string(),
            &handle,
            move |err| captor2.capture(err),
            async { Err(SlateDBError::Fenced) },
        );

        let result: Result<(), SlateDBError> = task.await.expect("join failure");
        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert!(matches!(captor.captured(), Some(Err(SlateDBError::Fenced))));
    }

    #[tokio::test]
    async fn test_should_cleanup_when_task_panics() {
        let monitored = async {
            panic!("oops");
        };
        let captor = Arc::new(ResultCaptor::new());
        let handle = tokio::runtime::Handle::current();
        let captor2 = captor.clone();

        let task = spawn_bg_task(
            "test".to_string(),
            &handle,
            move |err| captor2.capture(err),
            monitored,
        );

        let result: Result<(), SlateDBError> = task.await.expect("join failure");
        assert!(matches!(result, Err(SlateDBError::BackgroundTaskPanic(_))));
        assert!(matches!(
            captor.captured(),
            Some(Err(SlateDBError::BackgroundTaskPanic(_)))
        ));
    }

    #[tokio::test]
    async fn test_should_cleanup_when_task_exits() {
        let captor = Arc::new(ResultCaptor::new());
        let handle = tokio::runtime::Handle::current();
        let captor2 = captor.clone();

        let task = spawn_bg_task(
            "test".to_string(),
            &handle,
            move |err| captor2.capture(err),
            async { Ok(()) },
        );

        let result: Result<(), SlateDBError> = task.await.expect("join failure");
        assert!(matches!(result, Ok(())));
        assert!(matches!(captor.captured(), Some(Ok(()))));
    }

    #[tokio::test]
    async fn test_should_only_write_register_once() {
        let register = WatchableOnceCell::new();
        let reader = register.reader();
        assert_eq!(reader.read(), None);
        register.write(123);
        assert_eq!(reader.read(), Some(123));
        register.write(456);
        assert_eq!(reader.read(), Some(123));
    }

    #[tokio::test]
    async fn test_should_return_on_await_written_register() {
        let register = WatchableOnceCell::new();
        let mut reader = register.reader();
        let h = tokio::spawn(async move {
            assert_eq!(reader.await_value().await, 123);
            assert_eq!(reader.await_value().await, 123);
        });
        register.write(123);
        h.await.unwrap();
    }

    #[tokio::test]
    async fn test_monotonicity_enforcement_on_mono_clock() {
        // Given:
        let clock = Arc::new(MockSystemClock::new());
        let mono_clock = MonotonicClock::new(clock.clone(), 0);

        // When:
        clock.set(10);
        mono_clock.now().await.unwrap();
        clock.set(5);

        // Then:
        if let Err(SlateDBError::InvalidClockTick {
            last_tick,
            next_tick,
        }) = mono_clock.now().await
        {
            assert_eq!(last_tick, 10);
            assert_eq!(next_tick, 5);
        } else {
            panic!("Expected InvalidClockTick from mono_clock")
        }
    }

    #[tokio::test]
    async fn test_monotonicity_enforcement_on_mono_clock_set_tick() {
        // Given:
        let clock = Arc::new(MockSystemClock::new());
        let mono_clock = MonotonicClock::new(clock.clone(), 0);

        // When:
        clock.set(10);
        mono_clock.now().await.unwrap();

        // Then:
        if let Err(SlateDBError::InvalidClockTick {
            last_tick,
            next_tick,
        }) = mono_clock.set_last_tick(5)
        {
            assert_eq!(last_tick, 10);
            assert_eq!(next_tick, 5);
        } else {
            panic!("Expected InvalidClockTick from mono_clock")
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_await_valid_tick() {
        // the delegate clock is behind the mono clock by 100ms
        let delegate_clock = Arc::new(MockSystemClock::new());
        let mono_clock = MonotonicClock::new(delegate_clock.clone(), 100);

        tokio::spawn({
            let delegate_clock = delegate_clock.clone();
            async move {
                // wait for half the time it would wait for
                tokio::time::sleep(Duration::from_millis(50)).await;
                delegate_clock.set(101);
            }
        });

        let tick_future = mono_clock.now();
        tokio::time::advance(Duration::from_millis(100)).await;

        let result = tick_future.await;
        assert_eq!(result.unwrap(), 101);
    }

    #[tokio::test(start_paused = true)]
    async fn test_await_valid_tick_failure() {
        // the delegate clock is behind the mono clock by 100ms
        let delegate_clock = Arc::new(MockSystemClock::new());
        let mono_clock = MonotonicClock::new(delegate_clock.clone(), 100);

        // wait for 10ms after the maximum time it should accept to wait
        let tick_future = mono_clock.now();
        tokio::time::advance(Duration::from_millis(110)).await;

        let result = tick_future.await;
        assert!(result.is_err());
    }

    #[test]
    fn test_should_clamp_bytes_to_minimal_vec() {
        let mut bytes = BytesMut::with_capacity(2048);
        bytes.put_bytes(0u8, 2048);
        let bytes = bytes.freeze();
        let slice = bytes.slice(100..1124);

        let clamped = bytes_into_minimal_vec(&slice);

        assert_eq!(slice.as_ref(), clamped.as_slice());
        assert_eq!(clamped.capacity(), 1024);
    }

    #[test]
    fn test_should_clamp_bytes_and_preserve_data() {
        let mut bytes = BytesMut::with_capacity(2048);
        bytes.put_bytes(0u8, 2048);
        let bytes = bytes.freeze();
        let slice = bytes.slice(100..1124);

        let clamped = clamp_allocated_size_bytes(&slice);

        assert_eq!(clamped, slice);
        // It doesn't seem to be possible to assert that the clamped block's data is actually
        // a buffer of the minimal size, as Bytes doesn't expose the underlying buffer's
        // capacity. The best we can do is assert it allocated a new buffer.
        assert_ne!(clamped.as_ptr(), slice.as_ptr());
    }

    #[tokio::test]
    async fn test_last_written_key_and_seq_from_output_sst() {
        let os = Arc::new(InMemory::new());
        let path = "testdb-last-written".to_string();
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, os.clone())
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();
        let table_store = db.inner.table_store.clone();

        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_value(b"a", b"1", 1))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"b", b"2", 2))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let _sst1 = table_store
            .write_sst(
                &SsTableId::from(Ulid::new()),
                &encoded_sst,
                Some(Bytes::new()),
            )
            .await
            .unwrap();

        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_value(b"m", b"3", 3))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"z", b"4", 4))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let sst2 = table_store
            .write_sst(
                &SsTableId::from(Ulid::new()),
                &encoded_sst,
                Some(Bytes::new()),
            )
            .await
            .unwrap();

        let (last_key, last_seq) =
            last_written_key_and_seq(table_store.clone(), &sst2, &Bytes::new())
                .await
                .unwrap()
                .expect("missing last entry");
        assert_eq!(last_key, Bytes::from(b"z".as_slice()));
        assert_eq!(last_seq, 4);
    }

    #[tokio::test]
    async fn should_get_last_written_key_and_seq_from_v1_sst() {
        // given: an SST built with V1 block format
        let os = Arc::new(InMemory::new());
        let path = "testdb-last-written-v1".to_string();
        let clock = Arc::new(MockSystemClock::new());
        let db = Db::builder(path, os.clone())
            .with_system_clock(clock.clone())
            .build()
            .await
            .unwrap();
        let table_store = db.inner.table_store.clone();

        let mut sst_builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V1);
        sst_builder
            .add(RowEntry::new_value(b"aaa", b"1", 10))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"zzz", b"2", 20))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let sst = table_store
            .write_sst(
                &SsTableId::from(Ulid::new()),
                &encoded_sst,
                Some(Bytes::new()),
            )
            .await
            .unwrap();

        // when: getting last written key and seq
        let (last_key, last_seq) =
            last_written_key_and_seq(table_store.clone(), &sst, &Bytes::new())
                .await
                .unwrap()
                .expect("missing last entry");

        // then: should return the last key and seq from the V1 formatted SST
        assert_eq!(last_key, Bytes::from(b"zzz".as_slice()));
        assert_eq!(last_seq, 20);
    }

    #[rstest]
    #[case("alternating_bits", vec![true, false, true, false, true, false, true, false], vec![], vec![], vec![0xAA])]
    #[case("u64_value", vec![], vec![(0xAB, 8)], vec![], vec![0xAB])]
    #[case("partial_and_multiple_bytes", vec![true, false], vec![(0x3F, 6), (0xCD, 8)], vec![], vec![0xBF, 0xCD])]
    #[case("empty_writer", vec![], vec![], vec![], vec![])]
    #[case("single_bit_true", vec![true], vec![], vec![], vec![0x80])]
    #[case("single_bit_false", vec![false], vec![], vec![], vec![0x00])]
    #[case("all_zeros", vec![false, false, false, false, false, false, false, false], vec![], vec![], vec![0x00])]
    #[case("all_ones", vec![true, true, true, true, true, true, true, true], vec![], vec![], vec![0xFF])]
    #[case("partial_byte_padding", vec![true, false, true], vec![], vec![], vec![0xA0])]
    #[case("push32_single_bit", vec![], vec![(1, 1)], vec![], vec![0x80])]
    #[case("push32_zero_bits", vec![], vec![(0xFF, 0)], vec![], vec![])]
    #[case("push32_max_bits", vec![], vec![(0xDEADBEEF, 32)], vec![], vec![0xDE, 0xAD, 0xBE, 0xEF])]
    #[case("push64_operations", vec![], vec![], vec![(0x123456789ABCDEF0, 64)], vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0])]
    #[case("push64_partial", vec![], vec![], vec![(0xABCD, 12)], vec![0xBC, 0xD0])]
    #[case("mixed_operations", vec![true], vec![(0x7F, 7)], vec![], vec![0xFF])]
    #[case("boundary_crossing", vec![true, false, true, false], vec![(0xF0, 4)], vec![], vec![0xA0])]
    #[case("multiple_partial_bytes", vec![true], vec![(0x5, 3), (0x2, 2), (0x1, 2)], vec![], vec![0xD9])]
    fn test_bit_writer(
        #[case] _description: &str,
        #[case] individual_bits: Vec<bool>,
        #[case] push32_operations: Vec<(u32, u8)>,
        #[case] push64_operations: Vec<(u64, u8)>,
        #[case] expected: Vec<u8>,
    ) {
        // Given: a new BitWriter
        let mut writer = BitWriter::new();

        // When: we perform the specified operations
        // Push individual bits first
        for bit in individual_bits {
            writer.push(bit);
        }
        // Then push32 operations
        for (value, bits) in push32_operations {
            writer.push32(value, bits);
        }
        // Finally push64 operations
        for (value, bits) in push64_operations {
            writer.push64(value, bits);
        }
        let result = writer.finish();

        // Then: it should return the expected result
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("alternating_bits", vec![0xAA], vec![true, false, true, false, true, false, true, false], vec![], vec![])]
    #[case("u64_value", vec![0xAB], vec![], vec![(0xAB, 8)], vec![])]
    #[case("partial_and_multiple_bytes", vec![0xBF, 0xCD], vec![true, false], vec![(0x3F, 6), (0xCD, 8)], vec![])]
    #[case("empty_reader", vec![], vec![], vec![], vec![])]
    #[case("single_bit_true", vec![0x80], vec![true], vec![], vec![])]
    #[case("single_bit_false", vec![0x00], vec![false], vec![], vec![])]
    #[case("all_zeros", vec![0x00], vec![false, false, false, false, false, false, false, false], vec![], vec![])]
    #[case("all_ones", vec![0xFF], vec![true, true, true, true, true, true, true, true], vec![], vec![])]
    #[case("partial_byte_padding", vec![0xA0], vec![true, false, true], vec![], vec![])]
    #[case("push32_single_bit", vec![0x80], vec![], vec![(1, 1)], vec![])]
    #[case("push32_zero_bits", vec![], vec![], vec![], vec![])]
    #[case("push32_max_bits", vec![0xDE, 0xAD, 0xBE, 0xEF], vec![], vec![(0xDEADBEEF, 32)], vec![])]
    #[case("push64_operations", vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0], vec![], vec![], vec![(0x123456789ABCDEF0, 64)])]
    #[case("push64_partial", vec![0xBC, 0xD0], vec![], vec![], vec![(0xBCD, 12)])]
    #[case("mixed_operations", vec![0xFF], vec![true], vec![(0x7F, 7)], vec![])]
    #[case("boundary_crossing", vec![0xA0], vec![true, false, true, false], vec![(0x0, 4)], vec![])]
    #[case("multiple_partial_bytes", vec![0xD9], vec![true], vec![(0x5, 3), (0x2, 2), (0x1, 2)], vec![])]
    fn test_bit_reader(
        #[case] _description: &str,
        #[case] input_bytes: Vec<u8>,
        #[case] expected_individual_bits: Vec<bool>,
        #[case] expected_read32_operations: Vec<(u32, u8)>,
        #[case] expected_read64_operations: Vec<(u64, u8)>,
    ) {
        // Given: a BitReader with the input bytes

        let mut reader = BitReader::new(&input_bytes);

        // When: we read individual bits
        for expected_bit in expected_individual_bits {
            let actual_bit = reader.read_bit();
            assert_eq!(actual_bit, Some(expected_bit));
        }

        // Then: read32 operations
        for (expected_value, bits) in expected_read32_operations {
            let actual_value = reader.read32(bits);
            assert_eq!(actual_value, Some(expected_value));
        }

        // Finally: read64 operations
        for (expected_value, bits) in expected_read64_operations {
            let actual_value = reader.read64(bits);
            assert_eq!(actual_value, Some(expected_value));
        }

        // Verify we've consumed all bits for non-empty inputs
        if !input_bytes.is_empty() {
            // For partial bytes, there might be padding bits we should be able to read as false
            let next_bit = reader.read_bit();
            if let Some(bit) = next_bit {
                // If there are remaining bits, they should be padding (false)
                assert!(!bit);
            }
        }
    }

    #[test]
    fn test_bit_reader_exhaustion() {
        // Test that BitReader properly returns None when exhausted
        let bytes = vec![0xFF]; // Single byte with all bits set
        let mut reader = BitReader::new(&bytes);

        // Read all 8 bits
        for _ in 0..8 {
            assert_eq!(reader.read_bit(), Some(true));
        }

        // Next read should return None
        assert_eq!(reader.read_bit(), None);
        assert_eq!(reader.read32(1), None);
        assert_eq!(reader.read64(1), None);
    }

    #[rstest]
    #[case(0, "0 B")]
    #[case(1, "1 B")]
    #[case(999, "999 B")]
    #[case(1_000, "1.00 KB")]
    #[case(1_500, "1.50 KB")]
    #[case(1_000_000, "1.00 MB")]
    #[case(1_500_000, "1.50 MB")]
    #[case(1_000_000_000, "1.00 GB")]
    #[case(1_000_000_000_000, "1.00 TB")]
    #[case(1_000_000_000_000_000, "1.00 PB")]
    #[case(1_000_000_000_000_000_000, "1.00 EB")]
    #[case(u64::MAX, "18.45 EB")]
    fn test_format_bytes_si(#[case] bytes: u64, #[case] expected: &str) {
        assert_eq!(format_bytes_si(bytes), expected);
    }

    #[test]
    fn test_compute_max_parallel_min_and_cap() {
        // No SRs; total = l0_count
        assert_eq!(compute_max_parallel(5, &[], 8), 5);
        // Cap applies
        assert_eq!(compute_max_parallel(10, &[], 8), 8);
        // Clamp to at least 1 even when cap = 0
        assert_eq!(compute_max_parallel(0, &[], 0), 1);
    }

    #[test]
    fn test_estimate_bytes_before_key() {
        let run1 = SortedRun::new(
            1,
            [
                make_sst_view("a", 10),
                make_sst_view("k", 20), // k < m < z, so only "a" counts
                make_sst_view("z", 30),
            ],
        );
        let run2 = SortedRun::new(
            2,
            // f < m < ..., so only "b" counts
            [make_sst_view("b", 40), make_sst_view("f", 50)],
        );

        let key = Bytes::from("m");
        let total = estimate_bytes_before_key(&[run1, run2], &key);

        assert_eq!(total, 10 + 40);
    }

    // Filters out None; collects only Some(T)
    #[tokio::test]
    async fn test_build_iters_concurrent_option_filters_none() {
        let inputs = 0..10usize;
        let out: VecDeque<usize> = build_concurrent(inputs, 4, |x| async move {
            // Keep evens, drop odds
            if x % 2 == 0 {
                Ok(Some(x * 3))
            } else {
                Ok(None)
            }
        })
        .await
        .expect("should succeed");

        // Expect evens only (0,2,4,6,8) multiplied by 3
        let mut got: Vec<_> = out.into_iter().collect();
        got.sort_unstable();
        assert_eq!(got, vec![0, 6, 12, 18, 24]);
    }

    // Propagates first error
    #[tokio::test]
    async fn test_build_iters_concurrent_option_error() {
        let inputs = 0..10usize;

        let res: Result<VecDeque<usize>, SlateDBError> =
            build_concurrent(inputs, 3, |x| async move {
                if x == 5 {
                    Err(SlateDBError::Fenced)
                } else {
                    Ok(Some(x))
                }
            })
            .await;

        assert!(res.is_err());
    }

    // Respects max_parallel bound
    #[tokio::test]
    async fn test_build_iters_concurrent_option_respects_max_parallel() {
        let inputs = 0..16usize;
        let max_parallel = 4;
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let res: Result<VecDeque<()>, SlateDBError> = build_concurrent(inputs, max_parallel, {
            let in_flight = in_flight.clone();
            let peak = peak.clone();
            move |_x| {
                let in_flight = in_flight.clone();
                let peak = peak.clone();
                async move {
                    let cur = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(cur, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(Some(()))
                }
            }
        })
        .await;

        assert!(res.is_ok());
        let observed_peak = peak.load(Ordering::SeqCst);
        assert!(
            observed_peak <= max_parallel,
            "observed peak {} exceeds max_parallel {}",
            observed_peak,
            max_parallel
        );
    }

    #[test]
    fn panic_string_handles_slatedb_error() {
        // Given a SlateDBError payload
        let err = SlateDBError::InvalidDBState;
        let payload: Box<dyn Any + Send> = Box::new(err.clone());

        // When
        let msg = panic_string(&payload);

        // Then: it should stringify the exact error
        assert_eq!(msg, err.to_string());
    }

    #[test]
    fn panic_string_handles_slatedb_result() {
        // Given a SlateDBError payload
        let payload: Box<Result<(), SlateDBError>> = Box::new(Err(SlateDBError::InvalidDBState));

        // When
        let msg = panic_string(&(payload as Box<dyn Any + Send>));

        // Then: it should stringify the exact error
        assert_eq!(msg, SlateDBError::InvalidDBState.to_string());
    }

    #[test]
    fn panic_string_handles_string() {
        let s: Box<dyn Any + Send> = Box::new(String::from("hello"));
        let msg = panic_string(&s);
        assert_eq!(msg, "hello");
    }

    #[test]
    fn panic_string_handles_static_str() {
        let s: Box<dyn Any + Send> = Box::new("boom");
        let msg = panic_string(&s);
        assert_eq!(msg, "boom");
    }

    #[test]
    fn panic_string_falls_back_for_boxed_error_trait_object() {
        // The function attempts to downcast to `Box<dyn std::error::Error>`.
        // However, because `panic_string` requires `Send`, a realistic panic payload
        // would be `Box<dyn std::error::Error + Send + Sync>`, which does not
        // match the downcast target exactly and therefore takes the fallback path.
        let err_box: Box<dyn Any + Send> = Box::new(std::io::Error::other("oh no"));

        let msg = panic_string(&err_box);
        assert!(msg.contains("task panicked with unknown type"));
    }

    #[test]
    fn panic_string_falls_back_for_unknown_type() {
        #[derive(Clone, Debug)]
        struct MyType;

        let msg = panic_string(&(Box::new(MyType) as Box<dyn Any + Send>));
        assert!(msg.contains("task panicked with unknown type"));
    }

    #[test]
    fn test_split_unwind_result_ok_ok() {
        // Given: a successful unwind result
        let unwind_result: Result<Result<(), SlateDBError>, Box<dyn Any + Send>> = Ok(Ok(()));

        // When: we split the result
        let (result, payload) = super::split_unwind_result("test".to_string(), unwind_result);

        // Then: result should be Ok and payload should be None
        assert!(result.is_ok());
        assert!(payload.is_none());
    }

    #[test]
    fn test_split_unwind_result_ok_error() {
        // Given: an unwind result with a task error
        let unwind_result: Result<Result<(), SlateDBError>, Box<dyn Any + Send>> =
            Ok(Err(SlateDBError::Fenced));

        // When: we split the result
        let (result, payload) = super::split_unwind_result("test".to_string(), unwind_result);

        // Then: result should be the error and payload should be None
        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert!(payload.is_none());
    }

    #[test]
    fn test_split_unwind_result_panic() {
        // Given: an unwind result that panicked with a non-SlateDBError (e.g., a string)
        let panic_msg = "something went wrong";
        let unwind_result: Result<Result<(), SlateDBError>, Box<dyn Any + Send>> =
            Err(Box::new(panic_msg));

        // When: we split the result
        let (result, payload) = super::split_unwind_result("test_task".to_string(), unwind_result);

        // Then: result should be BackgroundTaskPanic and payload should contain the original panic
        assert!(matches!(
            result,
            Err(SlateDBError::BackgroundTaskPanic(ref name)) if name == "test_task"
        ));
        assert!(payload.is_some());
        // Verify the payload contains the original panic message
        if let Some(p) = payload {
            if let Some(msg) = p.downcast_ref::<&str>() {
                assert_eq!(msg, &panic_msg);
            } else {
                panic!("expected &str, got {:?}", p);
            }
        }
    }

    #[test]
    fn test_split_join_result_ok_ok() {
        // Given: a successful join result
        let join_result: Result<Result<(), SlateDBError>, tokio::task::JoinError> = Ok(Ok(()));

        // When: we split the result
        let (result, payload) = super::split_join_result("test".to_string(), join_result);

        // Then: result should be Ok and payload should be None
        assert!(result.is_ok());
        assert!(payload.is_none());
    }

    #[test]
    fn test_split_join_result_ok_error() {
        // Given: a join result with a task error
        let join_result: Result<Result<(), SlateDBError>, tokio::task::JoinError> =
            Ok(Err(SlateDBError::Fenced));

        // When: we split the result
        let (result, payload) = super::split_join_result("test".to_string(), join_result);

        // Then: result should be the error and payload should be None
        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert!(payload.is_none());
    }

    #[tokio::test]
    async fn test_split_join_result_cancelled() {
        // Given: a join result from a cancelled task
        let handle = tokio::spawn(async {
            // Wait forever
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Cancel the task
        handle.abort();
        let join_result = handle.await;

        // When: we split the result
        let (result, payload) = super::split_join_result("test_task".to_string(), join_result);

        // Then: result should be BackgroundTaskCancelled and payload should be None
        assert!(matches!(
            result,
            Err(SlateDBError::BackgroundTaskCancelled(ref name)) if name == "test_task"
        ));
        assert!(payload.is_none());
    }

    #[tokio::test]
    async fn test_split_join_result_panic() {
        // Given: a join result from a task that panicked with a non-SlateDBError
        let handle = tokio::spawn(async {
            panic!("something went wrong");
        });

        let join_result = handle.await;

        // When: we split the result
        let (result, payload) = super::split_join_result("test_task".to_string(), join_result);

        // Then: result should be BackgroundTaskPanic and payload should contain the original panic
        assert!(matches!(
            result,
            Err(SlateDBError::BackgroundTaskPanic(ref name)) if name == "test_task"
        ));
        assert!(payload.is_some());
        if let Some(p) = payload {
            if let Some(msg) = p.downcast_ref::<&str>() {
                assert_eq!(msg, &"something went wrong");
            } else {
                panic!("expected &str, got {:?}", p);
            }
        }
    }

    // ============================================================================
    // Varint (LEB128) encoding tests
    // ============================================================================

    #[rstest]
    #[case(0, 1)] // 0 fits in 1 byte
    #[case(1, 1)] // 1 fits in 1 byte
    #[case(127, 1)] // max value for 1 byte (0x7F)
    #[case(128, 2)] // min value requiring 2 bytes (0x80)
    #[case(16383, 2)] // max value for 2 bytes (0x3FFF)
    #[case(16384, 3)] // min value requiring 3 bytes (0x4000)
    #[case(2097151, 3)] // max value for 3 bytes (0x1FFFFF)
    #[case(2097152, 4)] // min value requiring 4 bytes (0x200000)
    #[case(268435455, 4)] // max value for 4 bytes (0xFFFFFFF)
    #[case(268435456, 5)] // min value requiring 5 bytes (0x10000000)
    #[case(u32::MAX, 5)] // max u32 requires 5 bytes
    fn should_calculate_varint_len(#[case] value: u32, #[case] expected_len: usize) {
        // when: calculating the varint length
        let len = super::varint_len(value);

        // then: the length matches expected
        assert_eq!(len, expected_len);
    }

    #[rstest]
    #[case(0, vec![0x00])]
    #[case(1, vec![0x01])]
    #[case(127, vec![0x7F])]
    #[case(128, vec![0x80, 0x01])]
    #[case(255, vec![0xFF, 0x01])]
    #[case(300, vec![0xAC, 0x02])]
    #[case(16384, vec![0x80, 0x80, 0x01])]
    #[case(u32::MAX, vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F])]
    fn should_encode_varint(#[case] value: u32, #[case] expected: Vec<u8>) {
        // given: an empty buffer
        let mut buf = Vec::new();

        // when: encoding the value
        super::encode_varint(&mut buf, value);

        // then: the encoded bytes match expected
        assert_eq!(buf, expected);
    }

    #[rstest]
    #[case(vec![0x00], 0)]
    #[case(vec![0x01], 1)]
    #[case(vec![0x7F], 127)]
    #[case(vec![0x80, 0x01], 128)]
    #[case(vec![0xFF, 0x01], 255)]
    #[case(vec![0xAC, 0x02], 300)]
    #[case(vec![0x80, 0x80, 0x01], 16384)]
    #[case(vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F], u32::MAX)]
    fn should_decode_varint(#[case] bytes: Vec<u8>, #[case] expected: u32) {
        // given: a buffer with encoded varint
        let mut buf: &[u8] = &bytes;

        // when: decoding the value
        let value = super::decode_varint(&mut buf);

        // then: the decoded value matches expected and buffer is consumed
        assert_eq!(value, expected);
        assert!(buf.is_empty());
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(127)]
    #[case(128)]
    #[case(255)]
    #[case(16383)]
    #[case(16384)]
    #[case(2097151)]
    #[case(2097152)]
    #[case(268435455)]
    #[case(268435456)]
    #[case(u32::MAX)]
    fn should_roundtrip_varint(#[case] value: u32) {
        // given: an encoded varint
        let mut buf = Vec::new();
        super::encode_varint(&mut buf, value);

        // when: decoding it back
        let mut slice: &[u8] = &buf;
        let decoded = super::decode_varint(&mut slice);

        // then: the value matches and encoded length is correct
        assert_eq!(decoded, value);
        assert!(slice.is_empty());
        assert_eq!(buf.len(), super::varint_len(value));
    }

    #[test]
    fn should_decode_varint_with_trailing_data() {
        // given: a buffer with varint followed by extra data
        let bytes = vec![0xAC, 0x02, 0xFF, 0xAB, 0xCD];
        let mut buf: &[u8] = &bytes;

        // when: decoding the varint
        let value = super::decode_varint(&mut buf);

        // then: only the varint bytes are consumed
        assert_eq!(value, 300);
        assert_eq!(buf, &[0xFF, 0xAB, 0xCD]);
    }

    #[test]
    fn should_decode_multiple_varints() {
        // given: a buffer with multiple varints
        let mut buf = Vec::new();
        super::encode_varint(&mut buf, 1);
        super::encode_varint(&mut buf, 300);
        super::encode_varint(&mut buf, 16384);
        super::encode_varint(&mut buf, u32::MAX);

        // when: decoding them sequentially
        let mut slice: &[u8] = &buf;
        let v1 = super::decode_varint(&mut slice);
        let v2 = super::decode_varint(&mut slice);
        let v3 = super::decode_varint(&mut slice);
        let v4 = super::decode_varint(&mut slice);

        // then: all values are correctly decoded
        assert_eq!(v1, 1);
        assert_eq!(v2, 300);
        assert_eq!(v3, 16384);
        assert_eq!(v4, u32::MAX);
        assert!(slice.is_empty());
    }
}
