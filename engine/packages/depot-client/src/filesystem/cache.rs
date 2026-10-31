//! Immutable overlay layers and bounded cold reads. Pins own their layers across trim.

use super::{
	Error, Result,
	transport::{ColdSnapshot, OpenPrefetch, PrefetchedRow},
};
use depot_fs_core::{
	CHUNK_BYTES, FsInode, FsKind,
	store::{Batch, ReadRequest, Row, RowKey, RowRange, RowStore},
};
use parking_lot::Mutex;
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
	},
};

/// Most chunks one prefetch request reads: 2 MiB, under a read's row and byte caps. A read
/// that needs more sends its requests together, so it still pays one round trip.
const PREFETCH_CHUNKS: u32 = 512;

/// The tier that held a row a filesystem read used, in the order a read looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadSource {
	/// An overlay row held in memory.
	Layer,
	/// A row of an index-only layer that the row cache still held.
	EntryCache,
	/// A row of an index-only layer whose log entry was read again from the log shards.
	Log,
	/// A row at the snapshot that the row cache held.
	ColdCache,
	/// A tablet maintainer read.
	Remote,
}

impl ReadSource {
	const ALL: [ReadSource; 5] = [
		ReadSource::Layer,
		ReadSource::EntryCache,
		ReadSource::Log,
		ReadSource::ColdCache,
		ReadSource::Remote,
	];

	fn label(self) -> &'static str {
		match self {
			ReadSource::Layer => "layer",
			ReadSource::EntryCache => "entry_cache",
			ReadSource::Log => "log",
			ReadSource::ColdCache => "cold_cache",
			ReadSource::Remote => "remote",
		}
	}
}

/// Value bytes of a row, the payload a read returns rather than its memory estimate.
fn value_bytes(row: &Option<Row>) -> u64 {
	row.as_ref().map_or(0, row_value_bytes)
}

fn row_value_bytes(row: &Row) -> u64 {
	match row {
		Row::Bytes(bytes) => bytes.len() as u64,
		Row::Inode(inode) => inode.inline.as_ref().map_or(0, |inline| inline.len()) as u64,
		Row::Format(_) | Row::Dentry(_) | Row::Unit => 0,
	}
}

/// Rows and bytes one view's reads used, by source. Fetches tally once, when the view
/// loads them. Overlay hits happen in the synchronous row store, which a read evaluates
/// again after every fetch, so they tally per evaluation and only the last one counts.
#[derive(Default)]
struct ReadTally {
	rows: [u64; 5],
	bytes: [u64; 5],
	layer_rows: AtomicU64,
	layer_bytes: AtomicU64,
}

impl ReadTally {
	fn add(&mut self, source: ReadSource, value: &Option<Row>) {
		self.rows[source as usize] += 1;
		self.bytes[source as usize] += value_bytes(value);
	}

	fn add_row(&mut self, source: ReadSource, row: &Row) {
		self.rows[source as usize] += 1;
		self.bytes[source as usize] += row_value_bytes(row);
	}

	fn add_layer_hit(&self, value: &Option<Row>) {
		self.layer_rows.fetch_add(1, Ordering::Relaxed);
		self.layer_bytes
			.fetch_add(value_bytes(value), Ordering::Relaxed);
	}

	fn start_evaluation(&self) {
		self.layer_rows.store(0, Ordering::Relaxed);
		self.layer_bytes.store(0, Ordering::Relaxed);
	}

	/// Records the tally with one counter increment per source, then clears it.
	fn record(&mut self) {
		self.rows[ReadSource::Layer as usize] += self.layer_rows.swap(0, Ordering::Relaxed);
		self.bytes[ReadSource::Layer as usize] += self.layer_bytes.swap(0, Ordering::Relaxed);
		for source in ReadSource::ALL {
			let index = source as usize;
			if self.rows[index] > 0 {
				crate::metrics::FS_READ_SERVED
					.with_label_values(&[source.label()])
					.inc_by(self.rows[index]);
			}
			if self.bytes[index] > 0 {
				crate::metrics::FS_READ_SERVED_BYTES
					.with_label_values(&[source.label()])
					.inc_by(self.bytes[index]);
			}
		}
		self.rows = [0; 5];
		self.bytes = [0; 5];
	}
}

pub(super) struct Budget {
	used: AtomicUsize,
	limit: usize,
	_capacity_metric: IntGaugeGuard,
}
pub(super) struct Charge {
	budget: Arc<Budget>,
	bytes: usize,
	_metric: IntGaugeGuard,
}
impl Budget {
	pub(crate) fn new(limit: usize) -> Arc<Self> {
		Arc::new(Self {
			used: AtomicUsize::new(0),
			limit,
			_capacity_metric: crate::metrics::FS_MEMORY_CAPACITY.add_guard(limit as i64),
		})
	}

	/// Bytes reserved against this budget.
	pub(crate) fn used(&self) -> usize {
		self.used.load(Ordering::Acquire)
	}

	/// Whether at least an eighth of the budget is in use.
	pub(crate) fn eighth_used(&self) -> bool {
		self.used.load(Ordering::Acquire) >= self.limit / 8
	}

	/// Whether at least half the budget is in use.
	pub(crate) fn half_used(&self) -> bool {
		self.used.load(Ordering::Acquire) >= self.limit / 2
	}

	/// Whether at least seven eighths of the budget are in use, so the next reservations are
	/// about to fail.
	pub(crate) fn nearly_full(&self) -> bool {
		self.used.load(Ordering::Acquire) >= self.limit / 8 * 7
	}

	pub(crate) fn reserve(self: &Arc<Self>, bytes: usize) -> Result<Charge> {
		self.reserve_up_to(bytes, self.limit).ok_or_else(|| {
			crate::metrics::FS_OVERLOADED
				.with_label_values(&["memory_budget"])
				.inc();
			Error::Overloaded
		})
	}
	/// A cache miss is optional; leave the rest of the memory budget for requests and indexes.
	fn reserve_up_to(self: &Arc<Self>, bytes: usize, ceiling: usize) -> Option<Charge> {
		self.used
			.try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
				used.checked_add(bytes)
					.filter(|next| *next <= self.limit.min(ceiling))
			})
			.ok()?;
		Some(Charge {
			budget: self.clone(),
			bytes,
			_metric: crate::metrics::FS_MEMORY_BYTES.add_guard(bytes as i64),
		})
	}
}
impl Drop for Charge {
	fn drop(&mut self) {
		self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
	}
}

/// A process-wide cap on bytes retained by filesystem caches, shared by every filesystem the
/// process opens. When it is full, an insert skips caching, as a full per-filesystem budget
/// does. Transient decode memory is not charged here.
#[derive(Debug)]
pub struct CacheBudget {
	used: AtomicUsize,
	capacity: usize,
	near_cap: AtomicBool,
	_capacity_metric: IntGaugeGuard,
}

/// Bytes reserved against a [`CacheBudget`], released on drop.
pub(super) struct ProcessCharge {
	budget: Arc<CacheBudget>,
	bytes: usize,
	_metric: IntGaugeGuard,
}

/// An adapter's retained derived data, charged to the filesystem process cache budget.
/// Dropping the reservation releases it; failure to reserve means the adapter skips caching.
pub struct CacheReservation {
	_charge: Option<ProcessCharge>,
}

impl CacheBudget {
	pub fn new(capacity: usize) -> Arc<Self> {
		Arc::new(Self {
			used: AtomicUsize::new(0),
			capacity,
			near_cap: AtomicBool::new(false),
			_capacity_metric: crate::metrics::FS_PROCESS_CACHE_CAPACITY.add_guard(capacity as i64),
		})
	}

	pub fn used(&self) -> usize {
		self.used.load(Ordering::Acquire)
	}

	fn reserve(self: &Arc<Self>, bytes: usize) -> Option<ProcessCharge> {
		let reserved = self
			.used
			.try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
				used.checked_add(bytes)
					.filter(|next| *next <= self.capacity)
			});
		let Ok(previous) = reserved else {
			crate::metrics::FS_OVERLOADED
				.with_label_values(&["process_cache_budget"])
				.inc();
			return None;
		};
		self.observe(previous.saturating_add(bytes));
		Some(ProcessCharge {
			budget: self.clone(),
			bytes,
			_metric: crate::metrics::FS_PROCESS_CACHE_BYTES.add_guard(bytes as i64),
		})
	}

	/// Warns once when usage crosses 90% of capacity and once when it falls back below.
	fn observe(&self, used: usize) {
		let near = used.saturating_mul(10) >= self.capacity.saturating_mul(9);
		if self.near_cap.swap(near, Ordering::AcqRel) != near {
			tracing::warn!(
				used,
				capacity = self.capacity,
				near,
				"filesystem process cache budget threshold crossed; raise filesystem.max_process_cache_bytes if caches thrash"
			);
		}
	}
}

impl Drop for ProcessCharge {
	fn drop(&mut self) {
		let previous = self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
		self.budget.observe(previous.saturating_sub(self.bytes));
	}
}

/// Shared by one filesystem's row cache: the optional process budget and whether the cache
/// holds rows since its last release.
pub(super) struct CacheShare {
	process: Option<Arc<CacheBudget>>,
	/// False after a release until the next insert.
	filled: AtomicBool,
}

impl CacheShare {
	pub(crate) fn new(process: Option<Arc<CacheBudget>>) -> Arc<Self> {
		Arc::new(Self {
			process,
			filled: AtomicBool::new(true),
		})
	}

	/// Reserves process bytes for one insert, or `None` when the process budget is full.
	fn insert(&self, bytes: usize) -> Option<Option<ProcessCharge>> {
		let charge = match &self.process {
			Some(process) => Some(process.reserve(bytes)?),
			None => None,
		};
		self.filled.store(true, Ordering::Release);
		Some(charge)
	}

	pub(crate) fn reserve_adapter(&self, bytes: usize) -> Option<CacheReservation> {
		self.insert(bytes)
			.map(|charge| CacheReservation { _charge: charge })
	}

	pub(crate) fn filled(&self) -> bool {
		self.filled.load(Ordering::Acquire)
	}

	pub(crate) fn mark_released(&self) {
		self.filled.store(false, Ordering::Release);
	}
}

pub(super) fn row_bytes(key: &RowKey, row: &Option<Row>) -> usize {
	let key_bytes = match key {
		RowKey::Dentry(_, x) | RowKey::Link(_, _, x) | RowKey::Xattr(_, x) | RowKey::Kv(_, x) => {
			x.len()
		}
		_ => 0,
	};
	192usize
		.saturating_add(key_bytes)
		.saturating_add(match row {
			Some(Row::Bytes(x)) => x.len(),
			Some(Row::Inode(x)) => {
				256 + x.inline.as_ref().map_or(0, |inline| inline.len()) + x.target.len()
			}
			_ => 64,
		})
}
pub(super) fn batch_bytes(batch: &Batch) -> usize {
	batch
		.rows
		.iter()
		.fold(256usize, |n, (k, v)| n.saturating_add(row_bytes(k, v)))
		.saturating_add(
			batch
				.checkpoints
				.keys()
				.map(|(_, name)| 128 + name.len())
				.sum::<usize>(),
		)
}
pub(super) struct Layer {
	pub seq: u64,
	/// Logical inode version, independent of a staged layer's visibility at sequence zero.
	pub inode_change_seq: u64,
	pub batch: Batch,
	pub ranges: Vec<RowRange>,
	pub references: BTreeMap<RowKey, Arc<LogRows>>,
	_charge: Charge,
	_index_charge: Option<Charge>,
}
impl Layer {
	/// Staged format rows are visible before admission but retain their eventual log version.
	pub(crate) fn staged(
		seq: u64,
		plan: depot_fs_core::apply::ApplyPlan,
		budget: &Arc<Budget>,
	) -> Result<Arc<Self>> {
		let mut layer = Self::new(0, plan, budget)?;
		let value = Arc::get_mut(&mut layer).expect("new layer");
		value.inode_change_seq = seq;
		Ok(layer)
	}

	pub(crate) fn new(
		seq: u64,
		plan: depot_fs_core::apply::ApplyPlan,
		budget: &Arc<Budget>,
	) -> Result<Arc<Self>> {
		let charge = Self::charge(&plan, budget)?;
		Ok(Self::with_charge(seq, plan, charge))
	}

	/// Reserves the budget a layer holding `plan` needs.
	fn charge(plan: &depot_fs_core::apply::ApplyPlan, budget: &Arc<Budget>) -> Result<Charge> {
		budget.reserve(Self::plan_bytes(plan))
	}

	/// Keeps cached postimages within both the cache allowance and half the memory budget.
	pub(super) fn charge_cached(
		plan: &depot_fs_core::apply::ApplyPlan,
		budget: &Arc<Budget>,
		cache_limit: usize,
	) -> Option<Charge> {
		budget.reserve_up_to(Self::plan_bytes(plan), cache_limit.min(budget.limit / 2))
	}

	fn plan_bytes(plan: &depot_fs_core::apply::ApplyPlan) -> usize {
		let range_bytes = plan
			.ranges
			.iter()
			.map(|range| match range {
				RowRange::Kv { start, end, .. } => {
					128 + start.len() + end.as_ref().map_or(0, Vec::len)
				}
				_ => 128,
			})
			.sum::<usize>();
		batch_bytes(&plan.final_rows).saturating_add(range_bytes)
	}

	/// Builds a layer from `plan` with a charge from [`charge`](Self::charge) for that plan.
	pub(super) fn with_charge(
		seq: u64,
		plan: depot_fs_core::apply::ApplyPlan,
		charge: Charge,
	) -> Arc<Self> {
		Arc::new(Self {
			seq,
			inode_change_seq: seq,
			batch: plan.final_rows,
			ranges: plan.ranges,
			references: BTreeMap::new(),
			_index_charge: None,
			_charge: charge,
		})
	}
}

/// The index holds no page bytes. A miss re-reads the bounded source entry.
pub(super) struct LogRows {
	seq: u64,
	journal: Arc<dyn super::transport::Journal>,
}
impl Layer {
	pub(crate) fn indexed(
		seq: u64,
		mut plan: depot_fs_core::apply::ApplyPlan,
		budget: &Arc<Budget>,
		journal: Arc<dyn super::transport::Journal>,
	) -> Result<Arc<Self>> {
		let source = Arc::new(LogRows { seq, journal });
		let mut references = BTreeMap::new();
		for (key, row) in &mut plan.final_rows.rows {
			if matches!(row, Some(Row::Bytes(_))) {
				references.insert(key.clone(), source.clone());
				*row = None;
			}
		}
		let mut layer = Self::new(seq, plan, budget)?;
		let value = Arc::get_mut(&mut layer).expect("new layer");
		value._index_charge =
			Some(budget.reserve(references.keys().map(|k| row_bytes(k, &None)).sum())?);
		value.references = references;
		Ok(layer)
	}

	/// This layer with its page bytes released: every byte row it holds becomes a reference to
	/// its own log entry, which a miss reads again from the log shards. Returns `None` when the
	/// layer holds no byte rows or already references log entries, since a merged transaction
	/// layer's rows belong to other entries than its own sequence.
	pub(crate) fn index_only(
		&self,
		budget: &Arc<Budget>,
		journal: Arc<dyn super::transport::Journal>,
	) -> Result<Option<Arc<Self>>> {
		if !self.references.is_empty()
			|| !self
				.batch
				.rows
				.values()
				.any(|row| matches!(row, Some(Row::Bytes(_))))
		{
			return Ok(None);
		}
		let plan = depot_fs_core::apply::ApplyPlan {
			ranges: self.ranges.clone(),
			final_rows: self.batch.clone(),
		};
		let mut layer = Self::indexed(self.seq, plan, budget, journal)?;
		let value = Arc::get_mut(&mut layer).expect("new layer");
		value.inode_change_seq = self.inode_change_seq;
		Ok(Some(layer))
	}

	pub(crate) fn merge(
		seq: u64,
		layers: &[Arc<Layer>],
		budget: &Arc<Budget>,
	) -> Result<Arc<Self>> {
		let mut plan = depot_fs_core::apply::ApplyPlan {
			ranges: Vec::new(),
			final_rows: Batch::default(),
		};
		let mut references = BTreeMap::new();
		for layer in layers {
			for range in &layer.ranges {
				plan.final_rows.rows.retain(|key, _| !range.contains(key));
				references.retain(|key, _| !range.contains(key));
			}
			for key in layer.batch.rows.keys() {
				references.remove(key);
			}
			plan.ranges.extend(layer.ranges.clone());
			plan.final_rows.rows.extend(layer.batch.rows.clone());
			plan.final_rows.checkpoints.extend(
				layer
					.batch
					.checkpoints
					.iter()
					.map(|((_, name), value)| ((seq, name.clone()), value.clone())),
			);
			references.extend(layer.references.clone());
		}
		let mut layer = Self::new(seq, plan, budget)?;
		let value = Arc::get_mut(&mut layer).expect("new layer");
		value._index_charge =
			Some(budget.reserve(references.keys().map(|k| row_bytes(k, &None)).sum())?);
		value.references = references;
		Ok(layer)
	}
}
/// Which version of a row a cached value holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Version {
	/// The row at the filesystem's current snapshot.
	Snapshot,
	/// The row as log entry `seq` wrote it, which an index-only layer above the snapshot names.
	Entry(u64),
}

type CacheKey = (RowKey, Version);

struct Cached {
	value: Option<Row>,
	/// For an inode row at the snapshot, the sequence of its latest change, when known. Rows
	/// cached by a plain row read leave it unset, so they cannot answer an inode read.
	change_seq: Option<u64>,
	used: u64,
	_charge: Charge,
	_process: Option<ProcessCharge>,
}

/// Byte-bounded least-recently-used rows. Every operation is O(log n), so the guard around it
/// never runs a scan proportional to the cache size.
#[derive(Default)]
struct Lru {
	values: BTreeMap<CacheKey, Cached>,
	/// Recency index keyed by `Cached::used`. Ticks are unique, so the first entry is the least
	/// recently used row. It holds exactly one entry per row in `values`.
	order: BTreeMap<u64, CacheKey>,
	bytes: usize,
	tick: u64,
}

impl Lru {
	fn next_tick(&mut self) -> u64 {
		self.tick = self.tick.wrapping_add(1);
		self.tick
	}

	/// Returns a cached row and marks it most recently used.
	fn get(&mut self, key: &CacheKey) -> Option<Option<Row>> {
		self.get_versioned(key).map(|(row, _)| row)
	}

	/// Returns a cached row with its change sequence and marks it most recently used.
	fn get_versioned(&mut self, key: &CacheKey) -> Option<(Option<Row>, Option<u64>)> {
		let tick = self.next_tick();
		let cached = self.values.get_mut(key)?;
		self.order.remove(&cached.used);
		cached.used = tick;
		self.order.insert(tick, key.clone());
		Some((cached.value.clone(), cached.change_seq))
	}

	/// Removes a cached row and returns it with its charges.
	fn take(&mut self, key: &CacheKey) -> Option<Cached> {
		let old = self.values.remove(key)?;
		self.order.remove(&old.used);
		self.bytes -= old._charge.bytes;
		Some(old)
	}

	fn remove(&mut self, key: &CacheKey) {
		self.take(key);
	}

	/// Evicts the least recently used row; returns false when the cache is empty.
	fn evict_one(&mut self) -> bool {
		let Some((_, victim)) = self.order.pop_first() else {
			return false;
		};
		if let Some(old) = self.values.remove(&victim) {
			self.bytes -= old._charge.bytes;
		}
		true
	}

	/// Puts back a row taken with [`take`](Self::take) under `key`, keeping its charges.
	fn insert_cached(&mut self, key: CacheKey, mut cached: Cached) {
		self.remove(&key);
		cached.used = self.next_tick();
		self.order.insert(cached.used, key.clone());
		self.bytes += cached._charge.bytes;
		self.values.insert(key, cached);
	}

	/// Removes the snapshot row of `key` and every row of it from an entry at or below
	/// `through`.
	fn remove_through(&mut self, key: &RowKey, through: u64) {
		let doomed: Vec<CacheKey> = self
			.values
			.range((key.clone(), Version::Snapshot)..=(key.clone(), Version::Entry(through)))
			.map(|(key, _)| key.clone())
			.collect();
		for key in doomed {
			self.remove(&key);
		}
	}

	/// Removes the rows in `range` that [`remove_through`](Self::remove_through) would remove
	/// for each of their keys, except keys `kept` selects. The range's keys are contiguous in
	/// key order, so this visits only the cached rows inside it.
	fn remove_range_through(
		&mut self,
		range: &RowRange,
		through: u64,
		kept: impl Fn(&RowKey) -> bool,
	) {
		let start = match range_start(range) {
			Some(key) => std::ops::Bound::Included((key, Version::Snapshot)),
			None => std::ops::Bound::Unbounded,
		};
		let doomed: Vec<CacheKey> = self
			.values
			.range((start, std::ops::Bound::Unbounded))
			.map(|(key, _)| key)
			.take_while(|(key, _)| range.contains(key))
			.filter(|(_, version)| match version {
				Version::Snapshot => true,
				Version::Entry(seq) => *seq <= through,
			})
			.filter(|(key, _)| !kept(key))
			.cloned()
			.collect();
		for key in doomed {
			self.remove(&key);
		}
	}

	/// Evicts least recently used rows until `bytes` more fit under `max_bytes`, then caches the
	/// row if the shared budget has room. Callers reject rows larger than `max_bytes` first.
	/// A row without a change sequence never replaces the same row cached with one: both hold
	/// the one value the row has at that version.
	#[allow(
		clippy::too_many_arguments,
		reason = "the cached row with its accounting inputs"
	)]
	fn remember(
		&mut self,
		key: CacheKey,
		value: Option<Row>,
		change_seq: Option<u64>,
		bytes: usize,
		max_bytes: usize,
		budget: &Arc<Budget>,
		share: &CacheShare,
	) {
		if change_seq.is_none()
			&& self
				.values
				.get(&key)
				.is_some_and(|cached| cached.change_seq.is_some())
		{
			return;
		}
		self.remove(&key);
		while self.bytes.saturating_add(bytes) > max_bytes {
			if !self.evict_one() {
				break;
			}
		}
		let Ok(charge) = budget.reserve(bytes) else {
			return;
		};
		let Some(process) = share.insert(bytes) else {
			return;
		};
		let used = self.next_tick();
		self.order.insert(used, key.clone());
		self.values.insert(
			key,
			Cached {
				value,
				change_seq,
				used,
				_charge: charge,
				_process: process,
			},
		);
		self.bytes += bytes;
	}
}

/// The first key `range` can contain, or `None` when it starts at the first key. Every range
/// is contiguous in key order from this key.
fn range_start(range: &RowRange) -> Option<RowKey> {
	match range {
		RowRange::All | RowRange::MetadataAll => None,
		RowRange::ChunksAll => Some(RowKey::Chunk(0, 0)),
		RowRange::Metadata(ino) => Some(RowKey::Inode(*ino)),
		RowRange::Chunks(ino) => Some(RowKey::Chunk(*ino, 0)),
		RowRange::ChunkSuffix { ino, first } => Some(RowKey::Chunk(*ino, *first)),
		RowRange::Directory(ino) => Some(RowKey::Dentry(*ino, Vec::new())),
		RowRange::Xattrs(ino) => Some(RowKey::Xattr(*ino, Vec::new())),
		RowRange::Kv { ino, start, .. } => Some(RowKey::Kv(*ino, start.clone())),
		RowRange::Orphans => Some(RowKey::Orphan(0)),
	}
}

struct RowCacheState {
	lru: Lru,
	/// The snapshot whose rows `Version::Snapshot` entries hold.
	snapshot: VolumeLogPosition,
	/// Goes up each time a snapshot is installed, including a renewal at the same position.
	/// Only a [`Cold`] built for the current version reads or fills snapshot rows: a view on an
	/// older snapshot cannot tell whether a row changed between the two, and a view whose
	/// snapshot expired must not keep reading through the cache.
	snapshot_version: u64,
	/// Changes when recovery replaces the log history, so a log read that started before
	/// cannot cache an entry the new history may not contain.
	history: u64,
}

/// One filesystem's cache of rows, for both sides of a view's cut: rows a maintainer read
/// returned at the current snapshot, and rows of the log entries that index-only layers above
/// it name. Installing a newer snapshot carries the cache over instead of dropping it. A row
/// that none of the released layers wrote is still the row at the new snapshot, and the
/// newest released write of a key becomes that key's snapshot row, so the rows a filesystem
/// just wrote stay cached after it releases their layers.
pub(super) struct RowCache {
	state: Mutex<RowCacheState>,
	/// Serializes log reads, so simultaneous misses on one entry read it once.
	loading: tokio::sync::Mutex<()>,
	budget: Arc<Budget>,
	max_bytes: usize,
	share: Arc<CacheShare>,
}

impl RowCache {
	pub(crate) fn new(
		snapshot: VolumeLogPosition,
		budget: Arc<Budget>,
		max_bytes: usize,
		share: Arc<CacheShare>,
	) -> Arc<Self> {
		Arc::new(Self {
			state: Mutex::new(RowCacheState {
				lru: Lru::default(),
				snapshot,
				snapshot_version: 0,
				history: 0,
			}),
			loading: tokio::sync::Mutex::new(()),
			budget,
			max_bytes,
			share,
		})
	}

	/// The snapshot version a [`Cold`] built now serves.
	fn snapshot_version(&self) -> u64 {
		self.state.lock().snapshot_version
	}

	/// The row of `key` at the current snapshot, when `version` is current and the row is
	/// cached.
	fn snapshot_get(&self, version: u64, key: &RowKey) -> Option<Option<Row>> {
		let mut state = self.state.lock();
		if state.snapshot_version != version {
			return None;
		}
		let row = state.lru.get(&(key.clone(), Version::Snapshot));
		if row.is_some() {}
		row
	}

	/// The inode `ino` at the current snapshot, when `version` is current and the inode was
	/// cached with its change sequence.
	fn snapshot_get_inode(&self, version: u64, ino: u64) -> Option<Option<super::Inode>> {
		let mut state = self.state.lock();
		if state.snapshot_version != version {
			return None;
		}
		let (row, change_seq) = state
			.lru
			.get_versioned(&(RowKey::Inode(ino), Version::Snapshot))?;
		let change_seq = change_seq?;
		match row {
			Some(Row::Inode(inode)) => Some(Some(super::Inode { inode, change_seq })),
			None => Some(None),
			// Any other row under an inode key is corrupt; reading cold reports it.
			Some(Row::Format(_) | Row::Dentry(_) | Row::Bytes(_) | Row::Unit) => None,
		}
	}

	/// Caches the row of `key` a maintainer read returned, when `version` is current. An inode
	/// row carries its change sequence so it can answer inode reads.
	fn snapshot_remember(
		&self,
		version: u64,
		key: RowKey,
		value: Option<Row>,
		change_seq: Option<u64>,
	) {
		let bytes = row_bytes(&key, &value);
		if bytes > self.max_bytes {
			return;
		}
		let mut state = self.state.lock();
		if state.snapshot_version != version {
			return;
		}
		state.lru.remember(
			(key, Version::Snapshot),
			value,
			change_seq,
			bytes,
			self.max_bytes,
			&self.budget,
			&self.share,
		);
	}

	/// Moves the cache to `snapshot` after the filesystem released `released`, its layers at
	/// or below it, in ascending order. The work is proportional to the keys those layers
	/// wrote and the cached rows inside their deleted ranges. Build the snapshot's [`Cold`]
	/// after this call.
	///
	/// Every committed entry between the two snapshots is one of `released`, because reads
	/// overlay exactly those layers on the old snapshot. So a key's newest value among them is
	/// its value at `snapshot`, and the rows a released layer held in memory become snapshot
	/// rows, with an inode's change sequence being the layer that wrote it. Reads right after
	/// the release, such as the next wake of a recently parked actor database, then need no
	/// cold round trip.
	pub(crate) fn install(&self, snapshot: &VolumeLogPosition, released: &[Arc<Layer>]) {
		let mut state = self.state.lock();
		state.snapshot_version = state.snapshot_version.wrapping_add(1);
		if &state.snapshot == snapshot {
			return;
		}
		let through = snapshot.volume_seq;
		let mut written = std::collections::BTreeSet::new();
		let mut deleted: Vec<RowRange> = Vec::new();

		// Newest layer first, so the first layer naming a key holds its newest version. A layer's
		// rows shadow its own ranges, as in `View::overlay`.
		let mut seeded = Vec::new();
		for layer in released.iter().rev() {
			for (key, value) in &layer.batch.rows {
				if written.contains(key) || deleted.iter().any(|range| range.contains(key)) {
					continue;
				}
				written.insert(key.clone());
				let reference = layer.references.get(key);
				let newest = reference
					.and_then(|source| state.lru.take(&(key.clone(), Version::Entry(source.seq))));
				state.lru.remove_through(key, through);
				if let Some(cached) = newest {
					state
						.lru
						.insert_cached((key.clone(), Version::Snapshot), cached);
				} else if reference.is_none() {
					seeded.push((key, value, layer.seq));
				}
			}
			for range in &layer.ranges {
				state.lru.remove_range_through(range, through, |key| {
					written.contains(key) || deleted.iter().any(|range| range.contains(key))
				});
				deleted.push(range.clone());
			}
		}

		// Oldest first, so the newest rows are the most recently used when the bound evicts.
		for (key, value, seq) in seeded.into_iter().rev() {
			let bytes = row_bytes(key, value);
			if bytes > self.max_bytes {
				continue;
			}
			let change_seq = if matches!(key, RowKey::Inode(_)) {
				Some(seq)
			} else {
				None
			};
			state.lru.remember(
				(key.clone(), Version::Snapshot),
				value.clone(),
				change_seq,
				bytes,
				self.max_bytes,
				&self.budget,
				&self.share,
			);
		}
		state.snapshot = snapshot.clone();
	}

	/// Caches the byte rows a layer held in memory as the rows of its log entry, before the
	/// layer keeps only its index. A miss on one of them then finds it here instead of reading
	/// the entry from the log, within the cache's bound.
	pub(crate) fn remember_entry_rows(&self, layer: &Layer) {
		let mut state = self.state.lock();
		for (key, value) in &layer.batch.rows {
			if !matches!(value, Some(Row::Bytes(_))) {
				continue;
			}
			let bytes = row_bytes(key, value);
			if bytes > self.max_bytes {
				continue;
			}
			state.lru.remember(
				(key.clone(), Version::Entry(layer.seq)),
				value.clone(),
				None,
				bytes,
				self.max_bytes,
				&self.budget,
				&self.share,
			);
		}
	}

	/// Drops every cached row for a log history that recovery replaced, and moves to the
	/// snapshot recovery opened. Build that snapshot's [`Cold`] after this call.
	pub(crate) async fn reset(&self, snapshot: VolumeLogPosition) {
		let _loading = self.loading.lock().await;
		let mut state = self.state.lock();
		state.lru = Lru::default();
		state.snapshot = snapshot;
		state.snapshot_version = state.snapshot_version.wrapping_add(1);
		state.history = state.history.wrapping_add(1);
	}

	/// Drops every cached row unless a log read is running; returns whether it cleared. Rows
	/// are durable copies, so a later read fetches them again.
	pub(crate) fn try_clear(&self) -> bool {
		let Ok(_loading) = self.loading.try_lock() else {
			return false;
		};
		self.state.lock().lru = Lru::default();
		true
	}

	/// Returns the row of `key` that `source` wrote, and whether the cache held it or the log
	/// was read again. A log read caches every byte row of the entry, so a read of a
	/// neighboring key in the same commit finds it.
	async fn load(&self, key: &RowKey, source: Arc<LogRows>) -> Result<(Option<Row>, ReadSource)> {
		let cache_key = (key.clone(), Version::Entry(source.seq));
		if let Some(row) = self.state.lock().lru.get(&cache_key) {
			return Ok((row, ReadSource::EntryCache));
		}
		let _loading = self.loading.lock().await;
		let history = {
			let mut state = self.state.lock();
			if let Some(row) = state.lru.get(&cache_key) {
				return Ok((row, ReadSource::EntryCache));
			}
			state.history
		};
		let status = source.journal.status()?;
		if status.committed < source.seq && !source.journal.reads_admitted() {
			source.journal.barrier()?.await.map_err(Error::Log)?;
		}
		let page = source
			.journal
			.read(source.seq, status.max_payload_bytes.saturating_add(128))
			.await?;
		if page.cold.volume_seq >= source.seq {
			return Err(Error::TooOld);
		}
		let entry = page
			.entries
			.into_iter()
			.next()
			.filter(|e| e.volume_seq == source.seq)
			.ok_or_else(|| corrupt!())?;
		let rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(payload) =
			entry.body
		else {
			return Err(corrupt!());
		};

		// Evict cached rows before reporting budget pressure. Transient decode memory is
		// charged only while the entry decodes.
		let transient = self.reserve_evicting(payload.payload.len().saturating_mul(2))?;
		let raw = depot_types::payload_codec::decode(&payload).map_err(|_| corrupt!())?;
		let decoded = <rivet_data::versioned::DepotFsEntryData as vbare::Versioned>::deserialize_with_embedded_version(&raw).map_err(|_| corrupt!())?;
		let batch = depot_fs_core::apply::plan(source.seq, &decoded)?.final_rows;
		drop(decoded);
		drop(raw);
		drop(payload);
		drop(transient);
		let row = batch.rows.get(key).cloned().ok_or_else(|| corrupt!())?;

		let mut state = self.state.lock();
		if state.history == history {
			for (row_key, value) in batch.rows {
				if !matches!(value, Some(Row::Bytes(_))) {
					continue;
				}
				let bytes = row_bytes(&row_key, &value);
				if bytes > self.max_bytes {
					continue;
				}
				state.lru.remember(
					(row_key, Version::Entry(source.seq)),
					value,
					None,
					bytes,
					self.max_bytes,
					&self.budget,
					&self.share,
				);
			}
		}
		Ok((row, ReadSource::Log))
	}

	/// Reserves `bytes` from the filesystem budget, evicting cached rows while it is full.
	fn reserve_evicting(&self, bytes: usize) -> Result<Charge> {
		loop {
			match self.budget.reserve(bytes) {
				Ok(charge) => return Ok(charge),
				Err(error) => {
					if !self.state.lock().lru.evict_one() {
						return Err(error);
					}
				}
			}
		}
	}
}

/// A cold snapshot and the filesystem's row cache that serves it. It reads and fills the
/// cache's snapshot rows only while it is the filesystem's current snapshot.
pub(super) struct Cold {
	pub snapshot: Arc<dyn ColdSnapshot>,
	rows: Arc<RowCache>,
	snapshot_version: u64,
	/// First scan pages the open's prefetch returned, each taken by the first scan of its
	/// range. A scan merges the page with the layers above the snapshot as it would a page
	/// the snapshot served, so the rows are only ever the snapshot's.
	prefetched_scans: Mutex<Vec<(RowRange, Vec<(RowKey, Row)>)>>,
	/// KV rows the open's prefetch returned, kept for this snapshot's lifetime so point reads
	/// and scans inside each covered range are answered here, absent rows included. Bounded
	/// by one page per prefetch, like the orphan page above, so they are not charged.
	prefetched_kv: Mutex<Vec<PrefetchedKv>>,
}

/// The rows of one KV inode an open's prefetch returned, and the key range they answer for.
struct PrefetchedKv {
	ino: u64,
	start: Vec<u8>,
	/// The requested exclusive end, unbounded when `None`.
	end: Option<Vec<u8>>,
	/// Set when the page was full: the rows answer only for keys through this one, because
	/// the maintainer may hold more rows past it.
	last_inclusive: Option<Vec<u8>>,
	rows: BTreeMap<Vec<u8>, Row>,
}

impl PrefetchedKv {
	/// Whether the rows answer for `key` on `ino`.
	fn covers(&self, ino: u64, key: &[u8]) -> bool {
		ino == self.ino
			&& key >= self.start.as_slice()
			&& self.end.as_ref().is_none_or(|end| key < end.as_slice())
			&& self
				.last_inclusive
				.as_ref()
				.is_none_or(|last| key <= last.as_slice())
	}

	/// Whether the rows answer for every key of `ino` from `start` to `end`, so a scan of
	/// that range can take its first page here.
	fn covers_range(&self, ino: u64, start: &[u8], end: Option<&[u8]>) -> bool {
		if ino != self.ino || start < self.start.as_slice() {
			return false;
		}
		let within_end = match (end, &self.end) {
			(_, None) => true,
			(None, Some(_)) => false,
			(Some(end), Some(own)) => end <= own.as_slice(),
		};
		let within_last = match (end, &self.last_inclusive) {
			(_, None) => true,
			(None, Some(_)) => false,
			(Some(end), Some(last)) => end <= last.as_slice(),
		};
		within_end && within_last
	}
}

impl Cold {
	/// Builds the filesystem's current snapshot, after [`RowCache::install`] or
	/// [`RowCache::reset`] moved `rows` to it.
	pub(crate) fn new(snapshot: Arc<dyn ColdSnapshot>, rows: Arc<RowCache>) -> Arc<Self> {
		let snapshot_version = rows.snapshot_version();
		Arc::new(Self {
			snapshot,
			rows,
			snapshot_version,
			prefetched_scans: Mutex::new(Vec::new()),
			prefetched_kv: Mutex::new(Vec::new()),
		})
	}

	/// Caches what an open's prefetch of `requested` found, so the opening's first reads need
	/// no round trip. Every row the open asked for and did not get is absent at the snapshot,
	/// as far as each path resolved: a missing step ends its path, and nothing past it is
	/// known. The orphan rows become the first page of the next orphan scan, and each KV
	/// prefetch's rows answer the reads inside its range.
	pub(crate) fn remember_prefetched(&self, requested: &OpenPrefetch, rows: Vec<PrefetchedRow>) {
		let mut found: BTreeMap<RowKey, PrefetchedRow> = BTreeMap::new();
		let mut orphans = Vec::new();
		let mut kv_rows: BTreeMap<u64, BTreeMap<Vec<u8>, Row>> = BTreeMap::new();
		for row in rows {
			match row.key {
				RowKey::Orphan(_) => orphans.push((row.key, row.row)),
				RowKey::Kv(ino, key) => {
					kv_rows.entry(ino).or_default().insert(key, row.row);
				}
				RowKey::Format
				| RowKey::Inode(_)
				| RowKey::Dentry(..)
				| RowKey::Link(..)
				| RowKey::Xattr(..)
				| RowKey::Chunk(..) => {
					found.insert(row.key.clone(), row);
				}
			}
		}
		self.remember_prefetched_inode(&found, depot_fs_core::ROOT_INO);
		self.remember(
			RowKey::Format,
			found.get(&RowKey::Format).map(|row| row.row.clone()),
		);
		self.prefetched_scans
			.lock()
			.push((RowRange::Orphans, orphans));

		if let Some(ino) = self.remember_prefetched_path(&found, &requested.path) {
			for name in &requested.xattrs {
				let key = RowKey::Xattr(ino, name.clone());
				self.remember(key.clone(), found.get(&key).map(|row| row.row.clone()));
			}
		}
		for kv in &requested.kv {
			let Some(ino) = self.remember_prefetched_path(&found, &kv.path) else {
				continue;
			};
			let rows: BTreeMap<Vec<u8>, Row> = kv_rows
				.get(&ino)
				.into_iter()
				.flatten()
				.filter(|(key, _)| {
					key.as_slice() >= kv.start.as_slice()
						&& kv
							.end
							.as_ref()
							.is_none_or(|end| key.as_slice() < end.as_slice())
				})
				// Rows from another prefetch of this inode may follow this request's full
				// first page. They cannot prove absence in the unreturned gap.
				.take(depot_fs_core::wire::PREFETCH_KV_ROWS)
				.map(|(key, row)| (key.clone(), row.clone()))
				.collect();
			// A full page may stop short of the range's end; the rows answer only through
			// the last one.
			let last_inclusive = if rows.len() >= depot_fs_core::wire::PREFETCH_KV_ROWS {
				rows.keys().next_back().cloned()
			} else {
				None
			};
			self.prefetched_kv.lock().push(PrefetchedKv {
				ino,
				start: kv.start.clone(),
				end: kv.end.clone(),
				last_inclusive,
				rows,
			});
		}
	}

	/// Caches each step of `path` from the root as the prefetch found it, the dentry and the
	/// child's inode, absent included, and returns the final inode when every step resolved.
	fn remember_prefetched_path(
		&self,
		found: &BTreeMap<RowKey, PrefetchedRow>,
		path: &[Vec<u8>],
	) -> Option<u64> {
		let mut parent = depot_fs_core::ROOT_INO;
		for name in path {
			let key = RowKey::Dentry(parent, name.clone());
			let Some(PrefetchedRow {
				row: Row::Dentry(dentry),
				..
			}) = found.get(&key)
			else {
				self.remember(key, None);
				return None;
			};
			let child = dentry.child;
			self.remember(key, Some(Row::Dentry(dentry.clone())));
			if !self.remember_prefetched_inode(found, child) {
				return None;
			}
			parent = child;
		}
		Some(parent)
	}

	/// Caches the inode `ino` as the prefetch found it, absent included. Returns whether it
	/// exists.
	fn remember_prefetched_inode(&self, found: &BTreeMap<RowKey, PrefetchedRow>, ino: u64) -> bool {
		let inode = match found.get(&RowKey::Inode(ino)) {
			Some(PrefetchedRow {
				row: Row::Inode(inode),
				change_seq: Some(change_seq),
				..
			}) => Some(super::Inode {
				inode: inode.clone(),
				change_seq: *change_seq,
			}),
			// A row of another kind or without its change sequence is left for a read to
			// report, never cached.
			Some(_) => return true,
			None => None,
		};
		let exists = inode.is_some();
		self.remember_inode(ino, &inode);
		exists
	}

	/// The first page of a scan of `range` that the open prefetched: an orphan page once, or
	/// the rows of a KV range inside one a KV prefetch covers, as often as they are asked for.
	fn prefetched_scan(&self, range: &RowRange) -> Option<Vec<(RowKey, Row)>> {
		if let RowRange::Kv { ino, start, end } = range {
			let prefetched = self.prefetched_kv.lock();
			let covering = prefetched
				.iter()
				.find(|kv| kv.covers_range(*ino, start, end.as_deref()))?;
			let page = covering
				.rows
				.range::<[u8], _>((
					std::ops::Bound::Included(start.as_slice()),
					end.as_deref()
						.map_or(std::ops::Bound::Unbounded, std::ops::Bound::Excluded),
				))
				.map(|(key, row)| (RowKey::Kv(*ino, key.clone()), row.clone()))
				.collect();
			return Some(page);
		}
		let mut scans = self.prefetched_scans.lock();
		let index = scans.iter().position(|(scanned, _)| scanned == range)?;
		Some(scans.swap_remove(index).1)
	}

	/// A cached row of the snapshot, or a KV row inside a range the open's prefetch covers,
	/// which is absent when the prefetch did not return it.
	fn get(&self, key: &RowKey) -> Option<Option<Row>> {
		if let Some(value) = self.rows.snapshot_get(self.snapshot_version, key) {
			return Some(value);
		}
		let RowKey::Kv(ino, kv_key) = key else {
			return None;
		};
		let value = self
			.prefetched_kv
			.lock()
			.iter()
			.find(|kv| kv.covers(*ino, kv_key))
			.map(|kv| kv.rows.get(kv_key).cloned())?;
		self.remember(key.clone(), value.clone());
		Some(value)
	}

	fn remember(&self, key: RowKey, value: Option<Row>) {
		self.rows
			.snapshot_remember(self.snapshot_version, key, value, None);
	}

	fn get_inode(&self, ino: u64) -> Option<Option<super::Inode>> {
		self.rows.snapshot_get_inode(self.snapshot_version, ino)
	}

	fn remember_inode(&self, ino: u64, inode: &Option<super::Inode>) {
		// A missing inode has no change of its own; any sequence marks the answer as complete.
		let change_seq = inode.as_ref().map_or(0, |inode| inode.change_seq);
		let row = inode.as_ref().map(|inode| Row::Inode(inode.inode.clone()));
		self.rows.snapshot_remember(
			self.snapshot_version,
			RowKey::Inode(ino),
			row,
			Some(change_seq),
		);
	}
}

/// One operation's dependency cache is separate from the LRU: eviction cannot
/// turn a retry of a pure rule into a loop fetching the same dependencies.
pub(super) struct View {
	pub seq: u64,
	pub cold: Arc<Cold>,
	pub layers: Vec<Arc<Layer>>,
	gets: BTreeMap<RowKey, Option<Row>>,
	inodes: BTreeMap<u64, Option<super::Inode>>,
	scans: Vec<(ReadRequest, Vec<(RowKey, Row)>)>,
	charges: Vec<Charge>,
	bytes: usize,
	max_bytes: usize,
	budget: Arc<Budget>,
	tally: ReadTally,
}
impl View {
	pub(crate) fn new(
		seq: u64,
		cold: Arc<Cold>,
		layers: Vec<Arc<Layer>>,
		budget: Arc<Budget>,
		max_bytes: usize,
	) -> Self {
		Self {
			seq,
			cold,
			layers,
			gets: BTreeMap::new(),
			inodes: BTreeMap::new(),
			scans: Vec::new(),
			charges: Vec::new(),
			bytes: 0,
			max_bytes,
			budget,
			tally: ReadTally::default(),
		}
	}

	/// Clears the overlay hits of an evaluation that is about to run again.
	pub(crate) fn start_evaluation(&self) {
		self.tally.start_evaluation();
	}

	/// Records where the rows of a completed read came from.
	pub(crate) fn record_served(&mut self) {
		self.tally.record();
	}

	pub(crate) fn fork(&self) -> Self {
		Self::new(
			self.seq,
			self.cold.clone(),
			self.layers.clone(),
			self.budget.clone(),
			self.max_bytes,
		)
	}
	pub(crate) fn forget_fetched(&mut self) {
		self.gets.clear();
		self.inodes.clear();
		self.scans.clear();
		self.charges.clear();
		self.bytes = 0;
	}

	fn charge(&mut self, bytes: usize) -> Result<()> {
		let next = self
			.bytes
			.checked_add(bytes)
			.filter(|n| *n <= self.max_bytes)
			.ok_or(Error::TooLarge)?;
		self.charges.push(self.budget.reserve(bytes)?);
		self.bytes = next;
		Ok(())
	}
	fn overlay(&self, key: &RowKey, seq: u64) -> Option<&Option<Row>> {
		self.layers
			.iter()
			.rev()
			.filter(|l| l.seq <= seq)
			.find_map(|l| {
				l.batch.rows.get(key).or_else(|| {
					l.ranges
						.iter()
						.any(|range| range.contains(key))
						.then_some(&None)
				})
			})
	}
	fn reference(&self, key: &RowKey, seq: u64) -> Option<Arc<LogRows>> {
		for layer in self.layers.iter().rev().filter(|l| l.seq <= seq) {
			if layer.batch.rows.contains_key(key) {
				return layer.references.get(key).cloned();
			}
			if layer.ranges.iter().any(|r| r.contains(key)) {
				return None;
			}
		}
		None
	}
	async fn load_reference(&mut self, key: &RowKey, source: Arc<LogRows>) -> Result<Option<Row>> {
		let (row, served) = self.cold.rows.load(key, source).await?;
		self.tally.add(served, &row);
		Ok(row)
	}

	pub(crate) fn inode(&self, ino: u64) -> depot_fs_core::Result<super::Inode> {
		let key = RowKey::Inode(ino);
		for layer in self.layers.iter().rev().filter(|l| l.seq <= self.seq) {
			if let Some(row) = layer.batch.rows.get(&key) {
				self.tally.add_layer_hit(row);
				return match row {
					Some(Row::Inode(inode)) => Ok(super::Inode {
						inode: inode.clone(),
						change_seq: layer.inode_change_seq,
					}),
					None => Err(depot_fs_core::Error::NotFound),
					_ => Err(depot_fs_core::Error::Corrupt),
				};
			}
			if layer.ranges.iter().any(|r| r.contains(&key)) {
				return Err(depot_fs_core::Error::NotFound);
			}
		}
		self.inodes
			.get(&ino)
			.cloned()
			.ok_or(depot_fs_core::Error::ReadRequired(ReadRequest::GetInode(
				ino,
			)))?
			.ok_or(depot_fs_core::Error::NotFound)
	}

	/// The inode `ino` at this view's sequence when the view can resolve it without a request:
	/// from a layer, an earlier read or the row cache. `Some(None)` means it is known absent.
	/// Returns `None` when only the cold snapshot can answer, and for a row a layer holds that
	/// is not an inode, which a read then reports as corrupt.
	fn known_inode(&self, ino: u64) -> Option<Option<FsInode>> {
		let key = RowKey::Inode(ino);
		for layer in self.layers.iter().rev().filter(|l| l.seq <= self.seq) {
			if let Some(row) = layer.batch.rows.get(&key) {
				return match row {
					Some(Row::Inode(inode)) => Some(Some(inode.clone())),
					None => Some(None),
					Some(Row::Format(_) | Row::Dentry(_) | Row::Bytes(_) | Row::Unit) => None,
				};
			}
			if layer.ranges.iter().any(|r| r.contains(&key)) {
				return Some(None);
			}
		}
		let fetched = match self.inodes.get(&ino) {
			Some(fetched) => fetched.clone(),
			None => self.cold.get_inode(ino)?,
		};
		Some(fetched.map(|inode| inode.inode))
	}

	/// Loads the chunks of `ino` in `first..first + count` that no layer holds and no earlier
	/// read fetched: from the row cache when it has them, and otherwise from the cold
	/// snapshot in one request per contiguous run. A file read then evaluates once instead of
	/// once per chunk, each after its own round trip.
	///
	/// When the inode is already known without a request, only chunks a read of it can use are
	/// loaded: none for an inline, absent or non-file inode, and none past its size. Those rows
	/// may not exist, so asking for them would cost a request whose answer the read ignores,
	/// and would make a writer cut off from its maintainer wait out its deadline for them.
	pub(crate) async fn prefetch_chunks(
		&mut self,
		ino: u64,
		spans: &[(u64, u64)],
		deadline_boot_ms: u64,
	) -> Result<()> {
		let readable_chunks = match self.known_inode(ino) {
			Some(Some(inode)) if inode.kind == FsKind::File && inode.inline.is_none() => {
				inode.size.div_ceil(CHUNK_BYTES as u64)
			}
			Some(_) => 0,
			None => u64::MAX,
		};

		// Every chunk no layer holds and no cache has, as contiguous runs, in file order.
		let mut runs: Vec<(u64, u32)> = Vec::new();
		let mut wanted: Vec<u64> = spans
			.iter()
			.flat_map(|&(first, count)| first..first.saturating_add(count).min(readable_chunks))
			.collect();
		wanted.sort_unstable();
		wanted.dedup();
		for index in wanted {
			let key = RowKey::Chunk(ino, index);
			if self.gets.contains_key(&key)
				|| self.reference(&key, self.seq).is_some()
				|| self.overlay(&key, self.seq).is_some()
			{
				continue;
			}
			if let Some(value) = self.cold.get(&key) {
				self.charge(row_bytes(&key, &value))?;
				self.tally.add(ReadSource::ColdCache, &value);
				self.gets.insert(key, value);
				continue;
			}
			match runs.last_mut() {
				Some((start, len)) if *start + u64::from(*len) == index => {
					*len += 1;
				}
				Some(_) | None => runs.push((index, 1)),
			}
		}
		if runs.is_empty() {
			return Ok(());
		}

		// One request carries every run, up to a request's chunk cap; a read past the cap
		// sends its requests together, so it still waits one round trip.
		let mut requests: Vec<Vec<(u64, u32)>> = Vec::new();
		let mut request_chunks = 0u32;
		for (start, len) in runs {
			let mut start = start;
			let mut len = len;
			while len > 0 {
				if requests.is_empty() || request_chunks >= PREFETCH_CHUNKS {
					requests.push(Vec::new());
					request_chunks = 0;
				}
				let take = len.min(PREFETCH_CHUNKS - request_chunks);
				if let Some(request) = requests.last_mut() {
					request.push((start, take));
				}
				request_chunks += take;
				start += u64::from(take);
				len -= take;
			}
		}
		let snapshot = self.cold.snapshot.clone();
		let through = self.seq;
		let replies = futures_util::future::try_join_all(
			requests
				.iter()
				.map(|ranges| snapshot.chunks(ino, ranges.clone(), through, deadline_boot_ms)),
		)
		.await?;
		for (ranges, rows) in requests.iter().zip(replies) {
			let mut found = rows.into_iter().peekable();
			for &(start, len) in ranges {
				for index in start..start + u64::from(len) {
					let key = RowKey::Chunk(ino, index);
					let value = match found.next_if(|(found, _)| found == &key) {
						Some((_, row)) => Some(row),
						None => None,
					};
					self.charge(row_bytes(&key, &value))?;
					self.tally.add(ReadSource::Remote, &value);
					self.cold.remember(key.clone(), value.clone());
					self.gets.insert(key, value);
				}
			}
			if found.next().is_some() {
				return Err(corrupt!());
			}
		}
		Ok(())
	}

	pub(crate) async fn load(&mut self, request: ReadRequest, deadline_boot_ms: u64) -> Result<()> {
		match &request {
			ReadRequest::GetInode(ino) => {
				let (inode, source) = match self.cold.get_inode(*ino) {
					Some(inode) => (inode, ReadSource::ColdCache),
					None => {
						let inode = self
							.cold
							.snapshot
							.get_inode(*ino, self.seq, deadline_boot_ms)
							.await?;
						self.cold.remember_inode(*ino, &inode);
						(inode, ReadSource::Remote)
					}
				};
				let row = inode.as_ref().map(|v| Row::Inode(v.inode.clone()));
				self.charge(row_bytes(&RowKey::Inode(*ino), &row))?;
				self.tally.add(source, &row);
				self.gets.insert(RowKey::Inode(*ino), row);
				self.inodes.insert(*ino, inode);
			}
			ReadRequest::Get(key) => {
				let reference = self.reference(key, self.seq);
				let value = if let Some(source) = reference.as_ref() {
					self.load_reference(key, source.clone()).await?
				} else {
					match self.cold.get(key) {
						Some(value) => {
							self.tally.add(ReadSource::ColdCache, &value);
							value
						}
						None => {
							let value = self
								.cold
								.snapshot
								.get(key.clone(), self.seq, deadline_boot_ms)
								.await?;
							self.tally.add(ReadSource::Remote, &value);
							value
						}
					}
				};
				self.charge(row_bytes(key, &value))?;
				if reference.is_none() {
					self.cold.remember(key.clone(), value.clone());
				}
				self.gets.insert(key.clone(), value);
			}
			ReadRequest::Scan {
				range,
				after,
				limit,
				as_of,
			} => {
				if *limit == 0 {
					self.scans.push((request, Vec::new()));
					return Ok(());
				}
				if *limit > self.max_bytes / 64 {
					return Err(Error::TooLarge);
				}
				let seq = as_of.unwrap_or(self.seq).min(self.seq);
				// Retain only candidates that can appear in the bounded result.
				// A range tombstone does not expand into one overlay row per key.
				let mut rows = BTreeMap::<RowKey, (Row, Charge)>::new();
				let mut scan_bytes = 0usize;
				let budget = self.budget.clone();
				let available = self.max_bytes.saturating_sub(self.bytes);
				let mut insert = |rows: &mut BTreeMap<RowKey, (Row, Charge)>,
				                  key: RowKey,
				                  value: Row|
				 -> Result<()> {
					if rows.contains_key(&key)
						|| (rows.len() >= *limit
							&& rows.last_key_value().is_some_and(|(last, _)| &key >= last))
					{
						return Ok(());
					}
					if rows.len() >= *limit {
						let (_, (_, charge)) = rows.pop_last().expect("nonempty bounded scan");
						scan_bytes -= charge.bytes;
					}
					let bytes = row_bytes(&key, &Some(value.clone()));
					scan_bytes = scan_bytes
						.checked_add(bytes)
						.filter(|n| *n <= available)
						.ok_or(Error::TooLarge)?;
					rows.insert(key, (value, budget.reserve(bytes)?));
					Ok(())
				};
				let layers = self.layers.clone();
				for layer in layers.iter().filter(|l| l.seq <= seq) {
					for key in layer.batch.rows.keys() {
						if range.contains(key)
							&& after.as_ref().is_none_or(|a| key > a)
							// A retained candidate already contains the newest visible value.
							// Heartbeats can overwrite the same key in many overlay layers.
							&& !rows.contains_key(key)
						{
							let value = if let Some(source) = self.reference(key, seq) {
								self.load_reference(key, source).await?
							} else {
								let value = self.overlay(key, seq).cloned().flatten();
								self.tally.add(ReadSource::Layer, &value);
								value
							};
							if let Some(value) = value {
								insert(&mut rows, key.clone(), value)?;
							}
						}
					}
				}
				let mut cursor = after.clone();
				loop {
					let prefetched = match cursor {
						None => self.cold.prefetched_scan(range),
						Some(_) => None,
					};
					let page = match prefetched {
						Some(page) => page,
						None => {
							self.cold
								.snapshot
								.scan(range.clone(), cursor.clone(), 256, seq, deadline_boot_ms)
								.await?
						}
					};
					let end = page.len() < 256;
					if page.len() > 256 {
						return Err(corrupt!());
					}
					for (key, value) in page {
						if !range.contains(&key) || cursor.as_ref().is_some_and(|prev| &key <= prev)
						{
							return Err(corrupt!());
						}
						cursor = Some(key.clone());
						if self.overlay(&key, seq).is_none() {
							self.tally.add_row(ReadSource::Remote, &value);
							insert(&mut rows, key, value)?;
						}
					}
					// Overlay rows may sort after unseen cold rows. Stop only once
					// the cold cursor has passed the last row we will return.
					if end
						|| rows
							.keys()
							.nth(*limit - 1)
							.is_some_and(|last| cursor.as_ref().is_some_and(|c| last <= c))
					{
						break;
					}
				}
				self.bytes += scan_bytes;
				let rows = rows
					.into_iter()
					.map(|(key, (value, charge))| {
						self.charges.push(charge);
						(key, value)
					})
					.collect();
				self.scans.push((request, rows));
			}
		}
		Ok(())
	}
}
impl RowStore for View {
	fn get(&self, key: &RowKey, as_of: Option<u64>) -> depot_fs_core::Result<Option<Row>> {
		let seq = as_of.unwrap_or(self.seq).min(self.seq);
		if seq < self.cold.snapshot.position().volume_seq {
			return Err(depot_fs_core::Error::Corrupt);
		}
		if self.reference(key, seq).is_some() {
			return self
				.gets
				.get(key)
				.cloned()
				.ok_or_else(|| depot_fs_core::Error::ReadRequired(ReadRequest::Get(key.clone())));
		}
		if let Some(value) = self.overlay(key, seq) {
			self.tally.add_layer_hit(value);
			return Ok(value.clone());
		}
		self.gets
			.get(key)
			.cloned()
			.ok_or_else(|| depot_fs_core::Error::ReadRequired(ReadRequest::Get(key.clone())))
	}
	fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
		as_of: Option<u64>,
	) -> depot_fs_core::Result<Vec<(RowKey, Row)>> {
		let request = ReadRequest::Scan {
			range: range.clone(),
			after: after.cloned(),
			limit,
			as_of,
		};
		self.scans
			.iter()
			.find(|(r, _)| r == &request)
			.map(|(_, rows)| rows.clone())
			.ok_or(depot_fs_core::Error::ReadRequired(request))
	}
}


