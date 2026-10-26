//! Cold-store addresses contain a kind, the big-endian partition hash and an optional name.
//! Raw partition keys are stored once in the volume record. Incarnations isolate filesystem histories.

use crate::{PartitionKeyHash, VolumeKey};
use vbare::Versioned;

rivet_keys::slatedb_key! {
	pub AlarmCurrentKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: uuid::Uuid) => [VOLUME, {hash}, {name}, {incarnation}, ALARM_CURRENT];
	value: vbare(rivet_data::versioned::DepotAlarmCurrentData, rivet_data::versioned::DepotAlarmCurrentData::LATEST);
}
rivet_keys::slatedb_key! {
	pub AlarmTimeKey(deadline: u64, hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: uuid::Uuid) => [ALARM_TIME, {deadline}, {hash}, {name}, {incarnation}];
	value: vbare(rivet_data::versioned::DepotAlarmTimeData, rivet_data::versioned::DepotAlarmTimeData::LATEST);
}
rivet_keys::slatedb_prefix! { pub AlarmTimePrefix() => [ALARM_TIME]; }

// This marker survives snapshot inheritance, unlike each child's bootstrap cursor.
rivet_keys::slatedb_key! {
	pub SplitSnapshotKey(hash: PartitionKeyHash, checkpoint: uuid::Uuid) => [VOLUME, {hash}, FS_SPLIT_SNAPSHOT, {checkpoint}];
	value: vbare(rivet_data::versioned::DepotSplitSnapshotData, rivet_data::versioned::DepotSplitSnapshotData::LATEST);
}

/// Share all data/index families while excluding the parent's bootstrap cursors.
/// Consumers and compaction enforce each child's logical hash range.
pub fn split_snapshot_range() -> (std::ops::Bound<bytes::Bytes>, std::ops::Bound<bytes::Bytes>) {
	use rivet_keys::keys::PrefixAddress;
	let mut end = AlarmTimePrefix::new().pack().to_vec();
	*end.last_mut().expect("nonempty alarm prefix") += 1;
	(
		std::ops::Bound::Included(VolumeDataPrefix::new().pack()),
		std::ops::Bound::Excluded(end.into()),
	)
}
rivet_keys::slatedb_key! {
	pub VolumeRecordKey(partition_key_hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {partition_key_hash}, {name}, RECORD];
	value: vbare_selected(rivet_data::versioned::DepotFsVolumeRecordData);
}

impl VolumeRecordKey {
	pub fn for_volume(key: &VolumeKey) -> Self {
		Self::new(
			crate::partition_key_hash(key),
			key.name.as_deref().map(<[u8]>::to_vec),
		)
	}

	/// Reject a corrupt row or an unexpected hash collision before using its identity.
	pub fn matches(&self, key: &VolumeKey) -> bool {
		self.partition_key_hash == crate::partition_key_hash(key)
			&& self.name.as_deref() == key.name.as_deref()
	}
}

rivet_keys::slatedb_prefix! {
	pub TabletHashBoundary(partition_key_hash: PartitionKeyHash) => [VOLUME, {partition_key_hash}];
}

rivet_keys::slatedb_prefix! {
	pub VolumeDataPrefix() => [VOLUME];
}

rivet_keys::slatedb_prefix! {
	pub TabletPendingHashBoundary(partition_key_hash: PartitionKeyHash) => [FS_PENDING, {partition_key_hash}];
}

// The family bound for tablet ranges; `depot-fs-core` owns the pending row keys.
rivet_keys::slatedb_prefix! {
	TabletPendingFamily() => [FS_PENDING];
}

type KeyRange = (std::ops::Bound<bytes::Bytes>, std::ops::Bound<bytes::Bytes>);

/// Bounds one tablet's volume family, including the largest hash when end is absent.
pub fn tablet_range(range: crate::PartitionKeyHashRange) -> KeyRange {
	use rivet_keys::keys::PrefixAddress;
	family_range(range, VolumeDataPrefix::new().pack(), |hash| {
		TabletHashBoundary::new(hash).pack()
	})
}

/// Bounds one tablet's pending work family, `[FS_PENDING, hash, ...]`.
pub fn tablet_pending_range(range: crate::PartitionKeyHashRange) -> KeyRange {
	use rivet_keys::keys::PrefixAddress;
	family_range(range, TabletPendingFamily::new().pack(), |hash| {
		TabletPendingHashBoundary::new(hash).pack()
	})
}

/// Hash-ordered families for a tablet, disjoint and in key order. Timer keys are ordered
/// by deadline instead; zero-copy splits inherit them through `split_snapshot_range`
/// and enforce logical ownership when consuming them.
pub fn tablet_ranges(range: crate::PartitionKeyHashRange) -> Vec<KeyRange> {
	let mut ranges = vec![tablet_range(range.clone()), tablet_pending_range(range)];
	ranges.sort_by(|a, b| range_start(a).cmp(range_start(b)));
	ranges
}

fn range_start(range: &KeyRange) -> &[u8] {
	match &range.0 {
		std::ops::Bound::Included(start) | std::ops::Bound::Excluded(start) => start,
		std::ops::Bound::Unbounded => &[],
	}
}

/// Bounds `[family, start)..[family, end)`, or to the end of the family when end is absent.
fn family_range(
	range: crate::PartitionKeyHashRange,
	family: bytes::Bytes,
	boundary: impl Fn(PartitionKeyHash) -> bytes::Bytes,
) -> KeyRange {
	use std::ops::Bound;
	let end = range.end.map(&boundary).unwrap_or_else(|| {
		let mut prefix = family.to_vec();
		while prefix.last() == Some(&255) {
			prefix.pop();
		}
		*prefix.last_mut().expect("nonempty key family prefix") += 1;
		prefix.into()
	});
	(Bound::Included(boundary(range.start)), Bound::Excluded(end))
}

rivet_keys::slatedb_key! {
	pub AlarmBootstrapKey() => [ALARM_BOOTSTRAP];
	value: vbare(rivet_data::versioned::DepotAlarmIndexProgressData, rivet_data::versioned::DepotAlarmIndexProgressData::LATEST);
}

// A split child's progress copying its pending work rows from the parent checkpoint. It is
// outside every tablet range, so a later split never carries it into a grandchild.
rivet_keys::slatedb_key! {
	pub PendingImportKey() => [FS_PENDING_IMPORT];
	value: vbare(rivet_data::versioned::DepotAlarmIndexProgressData, rivet_data::versioned::DepotAlarmIndexProgressData::LATEST);
}
