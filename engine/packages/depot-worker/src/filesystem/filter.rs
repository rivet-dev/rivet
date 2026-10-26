//! Per-compaction version retention, including checkpoint and active-session pins.

use async_trait::async_trait;
use depot_fs_core::keys::slatedb::{
	Scope, decode_key, row_is_deleted, scope_hold_prefix_len, versioned_row_prefix_len,
};
use parking_lot::Mutex;
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

/// Unknown incarnations are retained in full until their record and holds have
/// been loaded. Catalog updates occur only after the corresponding store work.
#[derive(Clone, Debug, Default)]
pub struct Retention {
	pub cold: u64,
	/// Completed resolver boundary; open transactions prevent publishing a later cold record.
	pub apply_cold: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition>,
	/// Internal SlateDB durable sequence covering this cold record and all rows
	/// preceding it. None conservatively prevents newest-tombstone removal.
	///
	/// It is read from the manifest after this flush's own memtable flush returned.
	/// The manifest's last L0 sequence only moves forward, so under overlapping flushes
	/// it is never older than this flush's cut; a newer value only makes the
	/// compaction filter's test harder to pass, which is conservative.
	pub cold_barrier: Option<u64>,
	pub checkpoints: BTreeSet<u64>,
	/// A missing child chunk falls through to these bases. Its Deleted row must
	/// continue masking them even when compaction reaches the oldest sorted run.
	pub has_base: bool,
	pub sessions: BTreeMap<uuid::Uuid, (u64, u64)>, // session ID -> (as_of, expiry_boot_ms)
	/// Bumped each time the scope's checkpoints are loaded into the catalog. A flush
	/// that computed retention from an older snapshot recomputes it when this moved.
	pub loads: u64,
}
pub type Catalog = Arc<Mutex<BTreeMap<Scope, Retention>>>;

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
		}))
	}
}
struct VersionFilter {
	catalog: Catalog,
	captured_at_boot_ms: u64,
	/// Sorted input keeps each scope contiguous. Capture only scopes that input visits.
	retention: Option<(Scope, Option<Retention>)>,
	ownership: Option<OwnedRange>,
	bottom: bool,
	complete_input_through: Option<u64>,
	previous: Option<(Scope, depot_fs_core::store::RowKey, u64, bool, bool)>,
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
			let mut retention = self.catalog.lock().get(&scope).cloned();
			if let Some(retention) = &mut retention {
				for (seq, expires) in retention.sessions.values() {
					if self.captured_at_boot_ms < *expires {
						retention.checkpoints.insert(*seq);
					}
				}
				retention.sessions.clear();
			}
			self.retention = Some((scope.clone(), retention));
		}
		let newer = self
			.previous
			.as_ref()
			.filter(|(s, r, _, _, _)| s == &scope && r == &row)
			.map(|(_, _, seq, dropped, protect)| (*seq, *dropped, *protect));
		// The tombstone tag is read in place; the row itself is never decoded.
		let deleted = matches!(&entry.value, ValueDeletable::Value(bytes) if matches!(row_is_deleted(&row, bytes.clone()), Ok(true)));
		let protect = newer.map_or(!self.bottom && deleted, |(_, _, protect)| protect);
		let mut drop = false;
		if let Some(retention) = self
			.retention
			.as_ref()
			.and_then(|(_, retention)| retention.as_ref())
		{
			if let Some((newer, was_dropped, _)) = newer {
				// A physical key can have internal versions from a partially
				// staged range op. If its newest copy was dropped, all its older
				// copies must follow it or a live copy could be resurrected.
				drop = !protect
					&& ((newer == seq && was_dropped)
						|| (newer > seq
							&& newer <= retention.cold
							&& retention.checkpoints.range(seq..newer).next().is_none()));
			} else if self.bottom
				&& retention
					.cold_barrier
					.zip(self.complete_input_through)
					.is_some_and(|(cold, input)| cold <= input)
				&& seq <= retention.cold
				&& !retention.has_base
				&& retention.checkpoints.range(..seq).next().is_none()
			{
				drop = deleted;
			}
		}
		self.previous = Some((scope, row, seq, drop, protect));
		Ok(if drop { Decision::Drop } else { Decision::Keep })
	}
	async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
		Ok(())
	}
}

/// Hashes each versioned row's address and each scope's checkpoint and fork-hold
/// prefixes into the SST bloom filters. Flush staging and retention scan those scope
/// prefixes for every flushed record, and most scopes have no such rows, so the filter
/// lets the scan skip every SST without reading a data block. The name changes whenever
/// the extracted prefixes change, so filters built under an older rule are never asked
/// about prefixes they did not hash.
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
	let filters: Vec<Arc<dyn slatedb::FilterPolicy>> = vec![Arc::new(
		slatedb::BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(RowPrefixExtractor)),
	)];

	StoragePolicy {
		sst_block_size: Some(slatedb::SstBlockSize::Block4Kib),
		block_cache: slatedb::BlockCachePolicy::default(),
		filters: Some(filters),
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


