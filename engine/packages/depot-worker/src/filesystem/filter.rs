//! Per-compaction version retention, including checkpoint and active-session pins.
//!
//! Every pin is a cut over the volume's claim logs, and the cold position is one per log. A
//! row's versions are keyed by sequence alone and reach the filter newest first, so the filter
//! evaluates each pinned cut against the row's versions as they pass: a pin keeps the first
//! version inside it, and nothing older of that row for that pin. With one claim log this is
//! the scalar rule, and it stays right when a row's versions come from several logs, where a
//! per-log rule would drop the version a pin still returns.

use async_trait::async_trait;
use depot_fs_core::keys::slatedb::{
	Scope, decode_key, deleted_incarnation_prefix_len, fork_pin_prefix_len, row_stamp,
	scope_hold_prefix_len, versioned_row_prefix_len,
};
use parking_lot::Mutex;
use rivet_data::{
	ROOT_CLAIM_LOG,
	generated::{shared_depot_fs_v2::FsCut, shared_depot_types_v1::VolumeLogPosition},
};
use rivet_env::{Clock, capabilities::slatedb::StoragePolicy};
use rivet_keys::keys::{KeyAddress, PrefixAddress};
use slatedb::{
	CompactionFilter, CompactionFilterDecision as Decision, CompactionFilterError,
	CompactionFilterSupplier, CompactionJobContext, RowEntry, ValueDeletable,
};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

/// A pinned cut: the sequence each claim log is read at. A version is inside the pin when
/// the pin names its log and the version is at or below that log's sequence; a version of a
/// log the pin does not name is invisible at it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PinnedCut(pub BTreeMap<u16, u64>);

impl PinnedCut {
	/// A pin over claim log 0 alone: every pin of a volume with one claim.
	pub fn single(seq: u64) -> Self {
		Self(BTreeMap::from([(ROOT_CLAIM_LOG, seq)]))
	}

	/// The pin of a stored cut.
	pub fn from_cut(cut: &FsCut) -> Self {
		Self(
			cut.logs
				.iter()
				.map(|log| (log.claim_log, log.position.volume_seq))
				.collect(),
		)
	}

	/// Whether the version at `seq` written by `claim_log` is inside the pin.
	pub fn contains(&self, claim_log: u16, seq: u64) -> bool {
		self.0.get(&claim_log).is_some_and(|at| seq <= *at)
	}

	/// The sequence the pin reads claim log 0 at, if it names it.
	pub fn root_seq(&self) -> Option<u64> {
		self.0.get(&ROOT_CLAIM_LOG).copied()
	}
}

/// Unknown incarnations are retained in full until their record and holds have
/// been loaded. Catalog updates occur only after the corresponding store work.
#[derive(Clone, Debug, Default)]
pub struct Retention {
	/// Per claim log, the sequence through which its entries are cold. A log absent here has
	/// nothing cold, so every version it wrote is kept.
	pub cold: PerLog<u64>,
	/// Per claim log, the completed resolver boundary; open transactions prevent publishing a
	/// later cold record of that log.
	pub apply_cold: PerLog<VolumeLogPosition>,
	/// Internal SlateDB durable sequence covering this cold record and all rows
	/// preceding it. None conservatively prevents newest-tombstone removal.
	///
	/// It is read from the manifest after this flush's own memtable flush returned.
	/// The manifest's last L0 sequence only moves forward, so under overlapping flushes
	/// it is never older than this flush's cut; a newer value only makes the
	/// compaction filter's test harder to pass, which is conservative.
	pub cold_barrier: Option<u64>,
	/// The cuts checkpoints and fork holds pin.
	pub pins: BTreeSet<PinnedCut>,
	/// A missing child chunk falls through to these bases. Its Deleted row must
	/// continue masking them even when compaction reaches the oldest sorted run.
	pub has_base: bool,
	/// Session ID to the cut it reads at and its expiry in boot milliseconds.
	pub sessions: BTreeMap<uuid::Uuid, (PinnedCut, u64)>,
	/// Bumped each time the scope's checkpoints are loaded into the catalog. A flush
	/// that computed retention from an older snapshot recomputes it when this moved.
	pub loads: u64,
}

impl Retention {
	/// The sequence through which `claim_log`'s entries are cold, or nothing for a log the
	/// catalog does not know.
	pub fn cold_of(&self, claim_log: u16) -> u64 {
		self.cold.get(claim_log).copied().unwrap_or(0)
	}

	/// Whether a checkpoint or fork hold pins claim log 0 exactly at `seq`.
	pub fn pins_root_at(&self, seq: u64) -> bool {
		self.pins.iter().any(|pin| pin.root_seq() == Some(seq))
	}
}

pub type Catalog = Arc<Mutex<BTreeMap<Scope, Retention>>>;

/// One value per claim log. Nearly every volume has one claim and only ever writes claim log
/// 0, so that log's value is held inline and a map is built only once a second log appears.
/// The catalog holds one of these per loaded or applied scope, so the inline form keeps a
/// scope's entry small.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PerLog<T> {
	Empty,
	Root(T),
	Logs(BTreeMap<u16, T>),
}

impl<T> Default for PerLog<T> {
	fn default() -> Self {
		PerLog::Empty
	}
}

impl<T> PerLog<T> {
	pub fn get(&self, claim_log: u16) -> Option<&T> {
		match self {
			PerLog::Empty => None,
			PerLog::Root(value) => match claim_log == ROOT_CLAIM_LOG {
				true => Some(value),
				false => None,
			},
			PerLog::Logs(values) => values.get(&claim_log),
		}
	}

	pub fn insert(&mut self, claim_log: u16, value: T) {
		match (std::mem::take(self), claim_log == ROOT_CLAIM_LOG) {
			(PerLog::Empty | PerLog::Root(_), true) => *self = PerLog::Root(value),
			(PerLog::Empty, false) => *self = PerLog::Logs(BTreeMap::from([(claim_log, value)])),
			(PerLog::Root(root), false) => {
				*self = PerLog::Logs(BTreeMap::from([(ROOT_CLAIM_LOG, root), (claim_log, value)]))
			}
			(PerLog::Logs(mut values), true | false) => {
				values.insert(claim_log, value);
				*self = PerLog::Logs(values);
			}
		}
	}
}

pub struct VersionFilterSupplier {
	pub catalog: Catalog,
	pub clock: Arc<dyn Clock>,
	pub hash_range: Option<depot_types::PartitionKeyHashRange>,
}
#[async_trait]
impl CompactionFilterSupplier for VersionFilterSupplier {
	fn requires_full_input(&self) -> bool {
		true
	}
	async fn create_compaction_filter(
		&self,
		context: &CompactionJobContext,
	) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
		let now = self
			.clock
			.observe()
			.map_err(|e| CompactionFilterError::CreationError(Box::new(e)))?
			.boot_ms;

		Ok(Box::new(VersionFilter {
			catalog: self.catalog.clone(),
			captured_at_boot_ms: now,
			retention: None,
			ownership: self.hash_range.clone().map(OwnedRange::new),
			bottom: context.is_dest_last_run,
			complete_input_through: context.complete_input_through,
			previous: None,
			satisfied: Satisfied::From(0),
			generation: 0,
		}))
	}
}

/// A scope's retention as one compaction sees it: the live sessions folded into the pins.
struct Loaded {
	cold: PerLog<u64>,
	cold_barrier: Option<u64>,
	has_base: bool,
	pins: LoadedPins,
}

impl Loaded {
	fn cold_of(&self, claim_log: u16) -> u64 {
		self.cold.get(claim_log).copied().unwrap_or(0)
	}

	/// Whether every pin reads the version at `seq` of `claim_log`.
	fn every_pin_contains(&self, claim_log: u16, seq: u64) -> bool {
		match &self.pins {
			LoadedPins::Root(seqs) => {
				seqs.is_empty() || (claim_log == ROOT_CLAIM_LOG && seqs[0] >= seq)
			}
			LoadedPins::Cuts(cuts) => cuts.iter().all(|pin| pin.contains(claim_log, seq)),
		}
	}
}

/// The pins of a scope as one compaction holds them.
enum LoadedPins {
	/// Every pin reads claim log 0 alone, as every pin of a volume with one claim does: their
	/// sequences, ascending and distinct, so a version finds the pins it satisfies by binary
	/// search and a scope switch copies one integer per pin.
	Root(Vec<u64>),
	/// Pins over several claim logs, checked one by one.
	Cuts(Vec<PinnedCut>),
}

impl LoadedPins {
	/// The pins of `retention` with its sessions live at `now_boot_ms`.
	fn of(retention: &Retention, now_boot_ms: u64) -> Self {
		let live_sessions = || {
			retention
				.sessions
				.values()
				.filter(|(_, expires)| now_boot_ms < *expires)
				.map(|(pin, _)| pin)
		};
		let all_root = retention
			.pins
			.iter()
			.chain(live_sessions())
			.all(|pin| pin.0.len() == 1 && pin.root_seq().is_some());
		match all_root {
			true => {
				let mut seqs = retention
					.pins
					.iter()
					.chain(live_sessions())
					.filter_map(PinnedCut::root_seq)
					.collect::<Vec<_>>();
				seqs.sort_unstable();
				seqs.dedup();
				LoadedPins::Root(seqs)
			}
			false => LoadedPins::Cuts(
				retention
					.pins
					.iter()
					.chain(live_sessions())
					.cloned()
					.collect(),
			),
		}
	}

	fn len(&self) -> usize {
		match self {
			LoadedPins::Root(seqs) => seqs.len(),
			LoadedPins::Cuts(cuts) => cuts.len(),
		}
	}
}

/// Which pins of the captured scope the current physical row already satisfied: a pin is
/// satisfied once a version inside it passed, since a read at it returns that version.
enum Satisfied {
	/// For [`LoadedPins::Root`]: versions pass newest first, so the satisfied pins are always
	/// those at or above some index, and this is that index.
	From(usize),
	/// For [`LoadedPins::Cuts`]: per pin, the generation of the physical row in which a
	/// version inside it was last seen.
	Generations(Vec<u64>),
}

/// The version of the current physical row that passed just before this one.
struct Previous {
	scope: Scope,
	row: depot_fs_core::store::RowKey,
	seq: u64,
	claim_log: u16,
	dropped: bool,
	protect: bool,
}

struct VersionFilter {
	catalog: Catalog,
	captured_at_boot_ms: u64,
	/// Sorted input keeps each scope contiguous. Capture only scopes that input visits.
	retention: Option<(Scope, Option<Loaded>)>,
	ownership: Option<OwnedRange>,
	bottom: bool,
	complete_input_through: Option<u64>,
	previous: Option<Previous>,
	/// The pins of the captured scope the current physical row already satisfied.
	satisfied: Satisfied,
	/// Counts physical rows, so `Satisfied::Generations` resets by comparison instead of by
	/// clearing.
	generation: u64,
}

impl VersionFilter {
	/// Starts a new physical row: no pin is satisfied yet.
	fn start_row(&mut self) {
		self.generation = self.generation.wrapping_add(1);
		if let (Satisfied::From(from), Some((_, Some(loaded)))) =
			(&mut self.satisfied, &self.retention)
		{
			*from = loaded.pins.len();
		}
	}

	/// Marks every pin the version at `seq` of `claim_log` is the first inside of, and says
	/// whether there was one: a read at such a pin returns this version.
	fn satisfy(&mut self, claim_log: u16, seq: u64) -> bool {
		let Some((_, Some(loaded))) = &self.retention else {
			return false;
		};
		match (&loaded.pins, &mut self.satisfied) {
			(LoadedPins::Root(seqs), Satisfied::From(from)) => {
				if claim_log != ROOT_CLAIM_LOG {
					return false;
				}
				// The pins at or above `seq` contain this version; those from `from` up were
				// satisfied by a newer version of the row.
				let first = seqs.partition_point(|pin| *pin < seq);
				let needed = first < *from;
				*from = (*from).min(first);
				needed
			}
			(LoadedPins::Cuts(cuts), Satisfied::Generations(generations)) => {
				let mut needed = false;
				for (index, pin) in cuts.iter().enumerate() {
					if generations[index] != self.generation && pin.contains(claim_log, seq) {
						generations[index] = self.generation;
						needed = true;
					}
				}
				needed
			}
			(LoadedPins::Root(_), Satisfied::Generations(_))
			| (LoadedPins::Cuts(_), Satisfied::From(_)) => {
				unreachable!("the satisfied pins are reset with the scope's pins")
			}
		}
	}
}

/// Physical files remain shared at split. Ordinary compaction discards foreign data
/// and indexes as it rewrites those files, without a separate sweep or split-time copy.
struct OwnedRange {
	hashes: depot_types::PartitionKeyHashRange,
	volumes: (std::ops::Bound<bytes::Bytes>, std::ops::Bound<bytes::Bytes>),
	pending: (std::ops::Bound<bytes::Bytes>, std::ops::Bound<bytes::Bytes>),
	volume_prefix: bytes::Bytes,
	pending_prefix: bytes::Bytes,
	alarm_prefix: bytes::Bytes,
}
impl OwnedRange {
	fn new(hashes: depot_types::PartitionKeyHashRange) -> Self {
		Self {
			volumes: depot_types::keys::slatedb::tablet_range(hashes.clone()),
			pending: depot_types::keys::slatedb::tablet_pending_range(hashes.clone()),
			hashes,
			volume_prefix: depot_types::keys::slatedb::VolumeDataPrefix::new().pack(),
			pending_prefix: depot_fs_core::keys::slatedb::PendingWorkPrefix::new().pack(),
			alarm_prefix: depot_types::keys::slatedb::AlarmTimePrefix::new().pack(),
		}
	}
	fn contains(&self, key: &bytes::Bytes) -> bool {
		use std::ops::RangeBounds;
		if key.starts_with(&self.volume_prefix) && !self.volumes.contains(key) {
			// Preserve ancestor split proofs until bootstrap, even for a right child.
			// Their number is bounded by split depth; each proof is a single small row.
			return depot_types::keys::slatedb::SplitSnapshotKey::unpack(key).is_ok();
		}
		if key.starts_with(&self.pending_prefix) {
			return self.pending.contains(key);
		}
		if key.starts_with(&self.alarm_prefix) {
			return depot_types::keys::slatedb::AlarmTimeKey::unpack(key)
				.map_or(true, |alarm| self.hashes.contains(alarm.hash));
		}
		true
	}
}
#[async_trait]
impl CompactionFilter for VersionFilter {
	async fn filter(&mut self, entry: &RowEntry) -> Result<Decision, CompactionFilterError> {
		if self
			.ownership
			.as_ref()
			.is_some_and(|range| !range.contains(&entry.key))
		{
			self.previous = None;
			return Ok(Decision::Drop);
		}
		let Ok((scope, row, seq)) = decode_key(&entry.key) else {
			self.previous = None;
			return Ok(Decision::Keep);
		};
		if self
			.retention
			.as_ref()
			.is_none_or(|(current, _)| current != &scope)
		{
			let loaded = self.catalog.lock().get(&scope).map(|retention| Loaded {
				cold: retention.cold.clone(),
				cold_barrier: retention.cold_barrier,
				has_base: retention.has_base,
				pins: LoadedPins::of(retention, self.captured_at_boot_ms),
			});
			self.satisfied = match loaded.as_ref().map(|loaded| &loaded.pins) {
				Some(LoadedPins::Cuts(cuts)) => Satisfied::Generations(vec![0; cuts.len()]),
				Some(LoadedPins::Root(_)) | None => Satisfied::From(0),
			};
			self.retention = Some((scope.clone(), loaded));
		}
		let newer = self
			.previous
			.as_ref()
			.filter(|previous| previous.scope == scope && previous.row == row)
			.map(|previous| {
				(
					previous.seq,
					previous.claim_log,
					previous.dropped,
					previous.protect,
				)
			});
		if newer.is_none() {
			self.start_row();
		}
		// The stamp and tombstone tag are read in place; the row itself is never decoded.
		let (claim_log, deleted) = match &entry.value {
			ValueDeletable::Value(bytes) => {
				row_stamp(&row, bytes.clone()).unwrap_or((ROOT_CLAIM_LOG, false))
			}
			_ => (ROOT_CLAIM_LOG, false),
		};
		let protect = newer.map_or(!self.bottom && deleted, |(_, _, _, protect)| protect);
		let mut drop = false;
		if self
			.retention
			.as_ref()
			.is_some_and(|(_, loaded)| loaded.is_some())
		{
			// Pins this version is the first inside of are satisfied by it whether it is kept
			// or dropped: a dropped bottom tombstone reads as absent there, which no older
			// version may replace.
			let needed = self.satisfy(claim_log, seq);
			let Some((_, Some(loaded))) = &self.retention else {
				unreachable!("checked above");
			};
			if let Some((newer_seq, newer_log, was_dropped, _)) = newer {
				// A physical key can have internal versions from a partially
				// staged range op. If its newest copy was dropped, all its older
				// copies must follow it or a live copy could be resurrected.
				drop = !protect
					&& ((newer_seq == seq && was_dropped)
						|| (newer_seq > seq && newer_seq <= loaded.cold_of(newer_log) && !needed));
			} else if self.bottom
				&& loaded
					.cold_barrier
					.zip(self.complete_input_through)
					.is_some_and(|(cold, input)| cold <= input)
				&& seq <= loaded.cold_of(claim_log)
				&& !loaded.has_base
				&& loaded.every_pin_contains(claim_log, seq)
			{
				// Every pin reads this tombstone, so every pin reads the row as absent, which
				// is what removing the tombstone leaves.
				drop = deleted;
			}
		}
		self.previous = Some(Previous {
			scope,
			row,
			seq,
			claim_log,
			dropped: drop,
			protect,
		});
		Ok(if drop { Decision::Drop } else { Decision::Keep })
	}
	async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
		Ok(())
	}
}

/// Hashes each versioned row's address and each scope's checkpoint and fork-hold prefixes into
/// the SST bloom filters, beside every whole key. Point reads and row scans test row addresses,
/// and flush staging and retention scan those scope prefixes for every flushed record; most
/// have no such rows, so the filter lets a miss skip every SST without reading a data block.
///
/// This is the rule and name that 3.0.7 wrote into every SST, and both must stay as they are:
/// a reader drops each SST filter whose name no configured policy has, and a filter is only
/// right for the prefixes it hashed. A prefix this rule does not hash goes in its own policy,
/// such as [`ForkPrefixExtractor`], so SSTs written under this rule keep their filters.
pub struct RowPrefixExtractor;
impl slatedb::PrefixExtractor for RowPrefixExtractor {
	fn name(&self) -> &str {
		"depot-fs-formal-row-v2"
	}
	fn prefix_len(&self, target: &slatedb::PrefixTarget) -> Option<usize> {
		match target {
			slatedb::PrefixTarget::Point(bytes) | slatedb::PrefixTarget::Prefix(bytes) => {
				versioned_row_prefix_len(bytes).or_else(|| scope_hold_prefix_len(bytes))
			}
		}
	}
}

/// Hashes each scope's fork pin prefix and each logical volume key's fork index prefix into a
/// second SST bloom filter, without whole keys. Pin maintenance scans a scope's pins, and every
/// lazy fork that finds no current incarnation of its key scans the key's fork index; most have
/// no such rows, so the filter lets those scans skip every SST without reading a data block.
/// It answers nothing about any other key, which [`RowPrefixExtractor`]'s filter covers. SSTs
/// written before this policy have no such filter and are read without one for these scans.
/// The name changes whenever the extracted prefixes change, so filters built under an older
/// rule are never asked about prefixes they did not hash.
pub struct ForkPrefixExtractor;
impl slatedb::PrefixExtractor for ForkPrefixExtractor {
	fn name(&self) -> &str {
		"depot-fs-fork-v1"
	}
	fn prefix_len(&self, target: &slatedb::PrefixTarget) -> Option<usize> {
		match target {
			slatedb::PrefixTarget::Point(bytes) | slatedb::PrefixTarget::Prefix(bytes) => {
				fork_pin_prefix_len(bytes).or_else(|| deleted_incarnation_prefix_len(bytes))
			}
		}
	}
}

/// Policies shared by filesystem SST writers and retained-checkpoint readers. A read is
/// skipped when any filter of the SST rules its key or prefix out.
pub(crate) fn sst_filters() -> Vec<Arc<dyn slatedb::FilterPolicy>> {
	vec![
		Arc::new(
			slatedb::BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(RowPrefixExtractor)),
		),
		Arc::new(
			slatedb::BloomFilterPolicy::new(10)
				.with_whole_key_filtering(false)
				.with_prefix_extractor(Arc::new(ForkPrefixExtractor)),
		),
	]
}

/// The tablet database's storage policy: 4 KiB SST blocks, so each page keeps its own block
/// and index entry, a bloom filter over row and scope prefixes, and a block cache that also
/// takes each memtable flush's data blocks.
pub fn storage_policy(catalog: Catalog, clock: Arc<dyn Clock>) -> StoragePolicy {
	storage_policy_in_range(catalog, clock, None)
}

pub(crate) fn storage_policy_in_range(
	catalog: Catalog,
	clock: Arc<dyn Clock>,
	hash_range: Option<depot_types::PartitionKeyHashRange>,
) -> StoragePolicy {
	StoragePolicy {
		sst_block_size: Some(slatedb::SstBlockSize::Block4Kib),
		block_cache: slatedb::BlockCachePolicy::default(),
		filters: Some(sst_filters()),
		compaction_filter: Some(Arc::new(VersionFilterSupplier {
			catalog,
			clock,
			hash_range,
		})),
		scan: slatedb::config::ScanOptions::default()
			.with_cache_blocks(true)
			.with_read_ahead_bytes(4096),
	}
}
