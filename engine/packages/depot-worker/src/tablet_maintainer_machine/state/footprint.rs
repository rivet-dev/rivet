//! What each tablet store operation and each queued store item touches.
//!
//! Store admission compares footprints instead of operation classes. A footprint has
//! a tablet-wide part, for the rules that still cover the whole store, and one access
//! per volume it reads or writes. Footprints are pure functions of an operation, or of
//! a queued item and the state it names, so the in-flight index can be recomputed from
//! the operations alone.
//!
//! Volumes are named by partition key hash and name rather than by `VolumeKey`, because
//! some work (sweeps, materialization, fork cleanup) identifies its volume only by a
//! storage scope, which carries the hash. A volume name is kept as a 64-bit digest, so
//! naming a volume never allocates. Two partition keys with the same 128-bit hash, or two
//! names with the same digest, only serialize more work than needed.

use super::{MaintainerState, Operation, filesystem::FlushStage, store::StoreWork};
use crate::tablet_maintainer_machine::types::OpId;
use depot_types::{PartitionKeyHash, VolumeKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One volume as store admission sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct StoreVolume {
	pub hash: PartitionKeyHash,
	/// Digest of the volume name, if the volume has one.
	pub name: Option<u64>,
}

impl StoreVolume {
	pub(crate) fn new(hash: PartitionKeyHash, name: Option<&[u8]>) -> Self {
		Self {
			hash,
			name: name.map(xxhash_rust::xxh3::xxh3_64),
		}
	}
}

/// Hashes a `StoreVolume` by folding its bytes, which are already a uniform hash and a
/// digest, so membership checks need no cryptographic hasher. Deterministic across runs.
#[derive(Default)]
pub(crate) struct StoreVolumeHasher(u64);

impl StoreVolumeHasher {
	fn add(&mut self, word: u64) {
		self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
	}
}

impl std::hash::Hasher for StoreVolumeHasher {
	fn write(&mut self, bytes: &[u8]) {
		for chunk in bytes.chunks(8) {
			let mut word = [0; 8];
			word[..chunk.len()].copy_from_slice(chunk);
			self.add(u64::from_le_bytes(word));
		}
	}

	fn write_u8(&mut self, value: u8) {
		self.add(u64::from(value));
	}

	fn write_u64(&mut self, value: u64) {
		self.add(value);
	}

	fn write_usize(&mut self, value: usize) {
		self.add(value as u64);
	}

	fn write_isize(&mut self, value: isize) {
		self.add(value as u64);
	}

	fn finish(&self) -> u64 {
		self.0
	}
}

/// A set of volumes for one admission scan, hashed rather than ordered because a scan only
/// asks whether a volume already waits. Equality, hashing and encoding use the sorted
/// members, so the set is deterministic state.
#[derive(Clone, Debug, Default)]
pub(crate) struct VolumeSet(
	std::collections::HashSet<StoreVolume, std::hash::BuildHasherDefault<StoreVolumeHasher>>,
);

impl VolumeSet {
	pub(crate) fn contains(&self, volume: &StoreVolume) -> bool {
		self.0.contains(volume)
	}

	fn sorted(&self) -> Vec<StoreVolume> {
		let mut volumes = self.0.iter().copied().collect::<Vec<_>>();
		volumes.sort_unstable();
		volumes
	}
}

impl Extend<StoreVolume> for VolumeSet {
	fn extend<I: IntoIterator<Item = StoreVolume>>(&mut self, volumes: I) {
		self.0.extend(volumes);
	}
}

impl PartialEq for VolumeSet {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl Eq for VolumeSet {}

impl std::hash::Hash for VolumeSet {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.sorted().hash(state);
	}
}

impl Serialize for VolumeSet {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.sorted().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for VolumeSet {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let volumes = Vec::<StoreVolume>::deserialize(deserializer)?;
		let mut set = Self::default();
		set.extend(volumes);
		Ok(set)
	}
}

/// Volumes to values, hashed rather than ordered because admission only looks volumes up.
/// Equality and hashing use the entries sorted by volume, so the map is deterministic state.
#[derive(Clone, Debug)]
pub(crate) struct VolumeMap<V>(
	std::collections::HashMap<StoreVolume, V, std::hash::BuildHasherDefault<StoreVolumeHasher>>,
);

impl<V> Default for VolumeMap<V> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<V> VolumeMap<V> {
	pub(crate) fn get(&self, volume: &StoreVolume) -> Option<&V> {
		self.0.get(volume)
	}

	pub(crate) fn get_mut(&mut self, volume: &StoreVolume) -> Option<&mut V> {
		self.0.get_mut(volume)
	}

	pub(crate) fn entry(
		&mut self,
		volume: StoreVolume,
	) -> std::collections::hash_map::Entry<'_, StoreVolume, V> {
		self.0.entry(volume)
	}

	pub(crate) fn remove(&mut self, volume: &StoreVolume) -> Option<V> {
		self.0.remove(volume)
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl<V: PartialEq> PartialEq for VolumeMap<V> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<V: Eq> Eq for VolumeMap<V> {}

impl<V: std::hash::Hash> std::hash::Hash for VolumeMap<V> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		let mut entries = self.0.iter().collect::<Vec<_>>();
		entries.sort_unstable_by_key(|(volume, _)| **volume);
		entries.hash(state);
	}
}

impl From<&VolumeKey> for StoreVolume {
	fn from(key: &VolumeKey) -> Self {
		Self::new(depot_types::partition_key_hash(key), key.name.as_deref())
	}
}

impl From<&depot_fs_core::keys::slatedb::Scope> for StoreVolume {
	fn from(scope: &depot_fs_core::keys::slatedb::Scope) -> Self {
		Self::new(scope.hash, scope.name.as_deref())
	}
}

impl From<&depot_fs_core::FsVolumeRecord> for StoreVolume {
	fn from(record: &depot_fs_core::FsVolumeRecord) -> Self {
		Self::from(&record.volume_key())
	}
}

/// How one store operation uses one volume. Two accesses to the same volume are
/// compatible only as listed on `compatible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) enum VolumeAccess {
	/// A session open of a caller that is still waiting. An open whose caller left keeps
	/// its entry but no longer blocks per-volume work.
	Read,
	/// A lifecycle row lookup. It trusts the cold floor, so it excludes every other use.
	Lookup,
	/// A filesystem apply or load.
	Keyed,
	/// A record, checkpoint, hold, fork or sweep writer, including a flush stage.
	Write,
	/// A staged flush record that is not durable yet.
	Unpublished,
}

impl VolumeAccess {
	/// The stronger of two accesses of one footprint to the same volume. `Write`
	/// excludes everything, so it dominates; otherwise the later access wins only when
	/// it excludes more.
	fn merge(self, other: VolumeAccess) -> VolumeAccess {
		if self == other {
			return self;
		}
		VolumeAccess::Write
	}
}

/// Whether two accesses of one volume may overlap.
pub(crate) fn compatible(a: VolumeAccess, b: VolumeAccess) -> bool {
	use VolumeAccess::{Keyed, Lookup, Read, Unpublished, Write};
	match (a, b) {
		(Read, Read) | (Read, Unpublished) | (Unpublished, Read) => true,
		(Keyed, Unpublished) | (Unpublished, Keyed) => true,
		(Read, Lookup | Keyed | Write)
		| (Lookup, Read | Lookup | Keyed | Write | Unpublished)
		| (Keyed, Read | Lookup | Keyed | Write)
		| (Write, Read | Lookup | Keyed | Write | Unpublished)
		| (Unpublished, Lookup | Write | Unpublished) => false,
	}
}

/// Rules that still cover the whole tablet store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) enum TabletAccess {
	None,
	/// Conflicts with every store operation.
	Exclusive,
	/// Conflicts only with `Exclusive`. Retired-store deletion.
	Independent,
	/// Pending-work discovery: conflicts with `Exclusive` and with writers of pending rows.
	PendingScan,
}

/// What one store operation or queued item touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoreFootprint {
	pub tablet: TabletAccess,
	pub volumes: FootprintVolumes,
}

/// The volumes of one footprint, ordered by volume. Most work touches a single volume,
/// which is kept inline so that judging a queued item allocates nothing; flushes and
/// passes touch many. A set of two or more volumes is always `Many`, so equal sets compare
/// equal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum FootprintVolumes {
	#[default]
	None,
	One(StoreVolume, VolumeAccess),
	Many(BTreeMap<StoreVolume, VolumeAccess>),
}

impl FootprintVolumes {
	pub(crate) fn is_empty(&self) -> bool {
		match self {
			FootprintVolumes::None => true,
			FootprintVolumes::One(..) => false,
			FootprintVolumes::Many(volumes) => volumes.is_empty(),
		}
	}

	pub(crate) fn get(&self, volume: &StoreVolume) -> Option<VolumeAccess> {
		match self {
			FootprintVolumes::None => None,
			FootprintVolumes::One(only, access) => {
				if only == volume {
					Some(*access)
				} else {
					None
				}
			}
			FootprintVolumes::Many(volumes) => volumes.get(volume).copied(),
		}
	}

	

	/// Sets the access of `volume`, replacing an earlier one.
	fn insert(&mut self, volume: StoreVolume, access: VolumeAccess) {
		match self {
			FootprintVolumes::None => *self = FootprintVolumes::One(volume, access),
			FootprintVolumes::One(only, existing) if *only == volume => *existing = access,
			FootprintVolumes::One(only, existing) => {
				let volumes = BTreeMap::from([(*only, *existing), (volume, access)]);
				*self = FootprintVolumes::Many(volumes);
			}
			FootprintVolumes::Many(volumes) => {
				volumes.insert(volume, access);
			}
		}
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = (&StoreVolume, &VolumeAccess)> {
		let (one, many) = match self {
			FootprintVolumes::None => (None, None),
			FootprintVolumes::One(volume, access) => (Some((volume, access)), None),
			FootprintVolumes::Many(volumes) => (None, Some(volumes.iter())),
		};
		one.into_iter().chain(many.into_iter().flatten())
	}

	pub(crate) fn keys(&self) -> impl Iterator<Item = &StoreVolume> {
		self.iter().map(|(volume, _)| volume)
	}

	pub(crate) fn into_keys(self) -> impl Iterator<Item = StoreVolume> {
		self.into_iter().map(|(volume, _)| volume)
	}
}

impl IntoIterator for FootprintVolumes {
	type Item = (StoreVolume, VolumeAccess);
	type IntoIter = std::iter::Chain<
		std::option::IntoIter<(StoreVolume, VolumeAccess)>,
		std::iter::Flatten<
			std::option::IntoIter<std::collections::btree_map::IntoIter<StoreVolume, VolumeAccess>>,
		>,
	>;

	fn into_iter(self) -> Self::IntoIter {
		let (one, many) = match self {
			FootprintVolumes::None => (None, None),
			FootprintVolumes::One(volume, access) => (Some((volume, access)), None),
			FootprintVolumes::Many(volumes) => (None, Some(volumes.into_iter())),
		};
		one.into_iter().chain(many.into_iter().flatten())
	}
}

impl StoreFootprint {
	pub(crate) fn none() -> Self {
		Self {
			tablet: TabletAccess::None,
			volumes: FootprintVolumes::None,
		}
	}

	pub(crate) fn tablet(tablet: TabletAccess) -> Self {
		Self {
			tablet,
			volumes: FootprintVolumes::None,
		}
	}

	pub(crate) fn volume(volume: impl Into<StoreVolume>, access: VolumeAccess) -> Self {
		Self::none().with(volume, access)
	}

	pub(crate) fn with(mut self, volume: impl Into<StoreVolume>, access: VolumeAccess) -> Self {
		let volume = volume.into();
		let merged = match self.volumes.get(&volume) {
			Some(existing) => existing.merge(access),
			None => access,
		};
		self.volumes.insert(volume, merged);
		self
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.tablet == TabletAccess::None && self.volumes.is_empty()
	}
}

/// Which capacity limit a queued item consumes when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkSlot {
	/// A read slot shared with session reads.
	Read,
	/// A keyed slot.
	Keyed,
	/// The flush slot: at most one flush is in flight.
	Flush,
	/// The pass slot: at most one cold write pass is in flight.
	Pass,
	/// No capacity limit beyond its footprint.
	None,
}

pub(crate) fn work_slot(work: &StoreWork) -> WorkSlot {
	match work {
		StoreWork::FilesystemQuery { .. }
		| StoreWork::ReadRow { .. }
		| StoreWork::ReadUnloadRow { .. }
		| StoreWork::ReadDeleteRow { .. } => WorkSlot::Read,
		StoreWork::FilesystemLoad { .. } | StoreWork::FilesystemApply { .. } => WorkSlot::Keyed,
		StoreWork::FilesystemFlush => WorkSlot::Flush,
		StoreWork::Materialize { .. }
		| StoreWork::ForkCleanup { .. }
		| StoreWork::Fork { .. }
		| StoreWork::ForkReference { .. }
		| StoreWork::PendingWork
		| StoreWork::SweepPending { .. }
		| StoreWork::Checkpoint { .. }
		| StoreWork::SweepDeleted { .. }
		| StoreWork::PrepareSplit
		| StoreWork::DeleteRetired { .. } => WorkSlot::None,
		StoreWork::WriteToCold { .. } => WorkSlot::Pass,
	}
}

/// The footprint of an in-flight operation.
pub(crate) fn operation_footprint(operation: &Operation) -> StoreFootprint {
	use super::filesystem_reads::QueryOrder;
	match operation {
		Operation::FilesystemFlush {
			records,
			stage: FlushStage::Staging,
			..
		} => records
			.iter()
			.fold(StoreFootprint::none(), |footprint, record| {
				footprint.with(record, VolumeAccess::Write)
			}),
		Operation::PendingWork { volume } => pending_scan_footprint(volume.as_ref()),
		Operation::OpenCold | Operation::PrepareSplit { .. } => {
			StoreFootprint::tablet(TabletAccess::Exclusive)
		}
		Operation::SweepPending { scope } | Operation::MaterializeStore { scope } => {
			StoreFootprint::volume(scope, VolumeAccess::Write)
		}
		Operation::ForkCleanupStore { work } => {
			StoreFootprint::volume(&work.scope, VolumeAccess::Write)
		}
		Operation::SweepDeleted { key, .. } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Write)
		}
		Operation::Checkpoint { volume_key, .. } => {
			StoreFootprint::volume(volume_key, VolumeAccess::Write)
		}
		// A replayed op writes its volume's rows like the live operation it stands for.
		Operation::ReplayOp { volume_key, .. } => match volume_key {
			Some(volume_key) => StoreFootprint::volume(volume_key, VolumeAccess::Write),
			None => StoreFootprint::none(),
		},
		Operation::ForkReference { volumes, .. } => volumes
			.iter()
			.fold(StoreFootprint::none(), |footprint, volume| {
				footprint.with(volume, VolumeAccess::Write)
			}),
		Operation::ForkStore {
			volume_key,
			parent_volume_key,
		} => parent_volume_key.iter().fold(
			StoreFootprint::volume(volume_key, VolumeAccess::Write),
			|footprint, parent| footprint.with(parent, VolumeAccess::Write),
		),
		// Retired-store deletion uses only the factory and another tablet's prefix.
		Operation::DeleteRetired { .. } => StoreFootprint::tablet(TabletAccess::Independent),
		Operation::WriteToCold { rows, .. } => {
			rows.keys().fold(StoreFootprint::none(), |footprint, key| {
				footprint.with(&key.0.volume_key, VolumeAccess::Write)
			})
		}
		Operation::FilesystemFlush {
			records,
			stage: FlushStage::Uploading,
			..
		} => records
			.iter()
			.fold(StoreFootprint::none(), |footprint, record| {
				footprint.with(record, VolumeAccess::Unpublished)
			}),
		Operation::FilesystemLoad { key, .. } | Operation::FilesystemApply { key, .. } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Keyed)
		}
		Operation::ReadOpenKeys { .. } | Operation::Repair(_) => StoreFootprint::none(),
		Operation::ReadRow { volume_key } => {
			StoreFootprint::volume(volume_key, VolumeAccess::Lookup)
		}
		Operation::ReadUnloadRow { key } | Operation::ReadDeleteRow { key } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Lookup)
		}
		Operation::FilesystemQuery {
			order, volume_key, ..
		} => match (order, volume_key) {
			(QueryOrder::Ordered, Some(volume_key)) => {
				StoreFootprint::volume(volume_key, VolumeAccess::Read)
			}
			(QueryOrder::Ordered, None)
			| (QueryOrder::Session | QueryOrder::Claim, _)
			| (QueryOrder::HeldImage, _) => StoreFootprint::none(),
		},
		Operation::Alarm(
			super::alarms::Attempt::Running(request)
			| super::alarms::Attempt::Cancelling(request)
			| super::alarms::Attempt::Obsolete(request),
		) if request.uses_store() => match request.key() {
			Some(key) => StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Keyed),
			None => StoreFootprint::none(),
		},
		Operation::MaterializePeer { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::ForkPeer { .. }
		| Operation::FilesystemRead { .. }
		| Operation::PromotionList { .. }
		| Operation::CatchUp { .. }
		| Operation::DeleteReplica { .. }
		| Operation::DrainClaim { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::RecoverVolume { .. }
		| Operation::ForgetVolumes { .. }
		| Operation::Trim { .. }
		| Operation::TrimBatch { .. }
		| Operation::Poll { .. }
		| Operation::List { .. }
		| Operation::Load { .. }
		| Operation::ForgetOrphan { .. }
		| Operation::MaintainerLog { .. }
		| Operation::Alarm(_) => StoreFootprint::none(),
	}
}

/// The footprint a queued item is admitted by. Work whose spec is gone gets an empty
/// footprint and dispatches as a no-op.
pub(crate) fn work_footprint(work: &StoreWork, state: &MaintainerState) -> StoreFootprint {
	match work {
		StoreWork::FilesystemQuery { request_id } => state
			.pending_fs_reads
			.get(request_id)
			.map_or_else(StoreFootprint::none, |request| {
				StoreFootprint::volume(request.volume_key(), VolumeAccess::Read)
			}),
		StoreWork::ReadRow { volume_key } => {
			StoreFootprint::volume(volume_key, VolumeAccess::Lookup)
		}
		StoreWork::ReadUnloadRow { key } | StoreWork::ReadDeleteRow { key } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Lookup)
		}
		StoreWork::FilesystemLoad { key, .. } | StoreWork::FilesystemApply { key } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Keyed)
		}
		// A queued flush selects its records at dispatch. Its ordering footprint is
		// writing the volumes an earlier flush skipped, so their later work queues
		// behind it and they drain before it starts.
		StoreWork::FilesystemFlush => state
			.filesystem
			.owed
			.iter()
			.fold(StoreFootprint::none(), |footprint, key| {
				footprint.with(&key.0.volume_key, VolumeAccess::Write)
			}),
		StoreWork::PendingWork => {
			let volume = match &state.pending_work {
				super::pending_work::Discovery::Queued(super::pending_work::Scan::Volume {
					volume,
					..
				}) => Some(volume),
				super::pending_work::Discovery::Queued(super::pending_work::Scan::All {
					..
				})
				| super::pending_work::Discovery::Waiting(_)
				| super::pending_work::Discovery::Reading(_)
				| super::pending_work::Discovery::Done => None,
			};
			pending_scan_footprint(volume)
		}
		StoreWork::PrepareSplit => StoreFootprint::tablet(TabletAccess::Exclusive),
		StoreWork::Materialize { scope } | StoreWork::SweepPending { scope } => {
			StoreFootprint::volume(scope, VolumeAccess::Write)
		}
		StoreWork::ForkCleanup { work } => StoreFootprint::volume(&work.scope, VolumeAccess::Write),
		StoreWork::SweepDeleted { key } => {
			StoreFootprint::volume(&key.0.volume_key, VolumeAccess::Write)
		}
		StoreWork::Checkpoint { request_id } => state
			.pending_checkpoints
			.get(request_id)
			.map_or_else(StoreFootprint::none, |pending| {
				StoreFootprint::volume(pending.volume_key(), VolumeAccess::Write)
			}),
		StoreWork::ForkReference { request_id } => state
			.pending_fork_references
			.get(request_id)
			.map_or_else(StoreFootprint::none, |pending| {
				super::fork_references::fork_volumes(state, pending.spec())
					.iter()
					.fold(StoreFootprint::none(), |footprint, volume| {
						footprint.with(volume, VolumeAccess::Write)
					})
			}),
		StoreWork::Fork { volume_key } => {
			let footprint = StoreFootprint::volume(volume_key, VolumeAccess::Write);
			match state.forks.jobs.get(volume_key) {
				Some(job) if super::in_tablet(state, &job.spec.parent_volume_key) => {
					footprint.with(&job.spec.parent_volume_key, VolumeAccess::Write)
				}
				Some(_) | None => footprint,
			}
		}
		StoreWork::DeleteRetired { .. } => StoreFootprint::tablet(TabletAccess::Independent),
		// A queued pass selects its rows at dispatch. Its ordering footprint is writing
		// the rows an earlier pass deferred.
		StoreWork::WriteToCold { .. } => state
			.pass_deferred
			.iter()
			.fold(StoreFootprint::none(), |footprint, key| {
				footprint.with(&key.0.volume_key, VolumeAccess::Write)
			}),
	}
}

/// Whether an in-flight access still constrains its volume. An ordered open whose
/// caller left keeps running but stops blocking.
pub(crate) fn live(state: &MaintainerState, id: &OpId, access: VolumeAccess) -> bool {
	if access != VolumeAccess::Read {
		return true;
	}
	match state.operations.get(id) {
		Some(Operation::FilesystemQuery { request_id, .. }) => {
			state.pending_fs_reads.contains_key(request_id)
		}
		_ => true,
	}
}

/// Whether in-flight work of `volume` conflicts with a new `access` of it.
pub(crate) fn volume_conflict(
	state: &MaintainerState,
	volume: &StoreVolume,
	access: VolumeAccess,
) -> bool {
	state
		.operations
		.index()
		.store_volumes
		.get(volume)
		.is_some_and(|holders| {
			holders
				.iter()
				.any(|(id, held)| !compatible(*held, access) && live(state, id, *held))
		})
}

/// The tablet-wide accesses in flight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TabletInFlight {
	pub exclusive: bool,
	pub independent: bool,
	pub pending_scan: bool,
}

impl TabletInFlight {
	pub(crate) fn of(state: &MaintainerState) -> Self {
		let mut tablet = Self::default();
		for access in state.operations.index().store_tablet.values() {
			match access {
				TabletAccess::None => {}
				TabletAccess::Exclusive => tablet.exclusive = true,
				TabletAccess::Independent => tablet.independent = true,
				TabletAccess::PendingScan => tablet.pending_scan = true,
			}
		}
		tablet
	}
}

/// Whether a queued flush must wait for one in flight. Up to `max_flush_staging` flushes
/// may stage and `max_flush_uploads` be in flight in all.
pub(crate) fn flush_slot_full(state: &MaintainerState) -> bool {
	let index = state.operations.index();
	index.store.staging >= state.config.max_flush_staging as usize
		|| index.counts.filesystem_flush >= state.config.max_flush_uploads
}

/// Whether an in-flight operation may write rows under the pending-work prefix. A
/// pending-work scan never overlaps one: a page could otherwise return the row of a job
/// that finished after its snapshot, and discovery would start the job again.
pub(crate) fn writes_pending_rows(operation: &Operation) -> bool {
	match operation {
		Operation::MaterializeStore { .. }
		| Operation::ForkCleanupStore { .. }
		| Operation::ForkStore { .. }
		| Operation::ForkReference { .. }
		| Operation::Checkpoint { .. }
		| Operation::SweepPending { .. }
		| Operation::SweepDeleted { .. }
		| Operation::ReplayOp { .. } => true,
		Operation::WriteToCold { rows, .. } => rows
			.values()
			.any(|row| row.status == depot_fs_core::FsVolumeStatus::Deleted),
		Operation::FilesystemFlush { records, .. } => records
			.iter()
			.any(|record| record.status == depot_fs_core::FsVolumeStatus::Deleted),
		Operation::MaterializePeer { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::ForkPeer { .. }
		| Operation::PendingWork { .. }
		| Operation::FilesystemQuery { .. }
		| Operation::FilesystemLoad { .. }
		| Operation::FilesystemRead { .. }
		| Operation::FilesystemApply { .. }
		| Operation::PromotionList { .. }
		| Operation::CatchUp { .. }
		| Operation::ReadDeleteRow { .. }
		| Operation::DeleteReplica { .. }
		| Operation::DrainClaim { .. }
		| Operation::PrepareSplit { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::DeleteRetired { .. }
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::RecoverVolume { .. }
		| Operation::ReadUnloadRow { .. }
		| Operation::ForgetVolumes { .. }
		| Operation::Trim { .. }
		| Operation::TrimBatch { .. }
		| Operation::Poll { .. }
		| Operation::List { .. }
		| Operation::Load { .. }
		| Operation::ForgetOrphan { .. }
		| Operation::MaintainerLog { .. }
		| Operation::ReadRow { .. }
		| Operation::ReadOpenKeys { .. }
		| Operation::Repair(_)
		| Operation::OpenCold
		| Operation::Alarm(_) => false,
	}
}

/// Queued work that may write pending rows once it starts. A queued pass may carry a
/// deleted record, so it counts; a flush never carries a deleting volume's record.
pub(crate) fn work_writes_pending_rows(work: &StoreWork) -> bool {
	match work {
		StoreWork::Materialize { .. }
		| StoreWork::ForkCleanup { .. }
		| StoreWork::Fork { .. }
		| StoreWork::ForkReference { .. }
		| StoreWork::SweepPending { .. }
		| StoreWork::Checkpoint { .. }
		| StoreWork::SweepDeleted { .. }
		| StoreWork::WriteToCold { .. } => true,
		StoreWork::FilesystemFlush
		| StoreWork::PendingWork
		| StoreWork::FilesystemQuery { .. }
		| StoreWork::FilesystemLoad { .. }
		| StoreWork::FilesystemApply { .. }
		| StoreWork::ReadDeleteRow { .. }
		| StoreWork::PrepareSplit
		| StoreWork::DeleteRetired { .. }
		| StoreWork::ReadUnloadRow { .. }
		| StoreWork::ReadRow { .. } => false,
	}
}

/// The footprint of a pending-work page. The pass over every pending row excludes every
/// writer of pending rows; a scan of one volume key's rows conflicts only with work on that
/// volume, so it does not stall forks and checkpoints of other volumes. A writer that
/// finishes beside a scan of its own volume marks the volume again either way.
fn pending_scan_footprint(volume: Option<&depot_fs_core::work::PendingVolume>) -> StoreFootprint {
	match volume {
		None => StoreFootprint::tablet(TabletAccess::PendingScan),
		Some(volume) => StoreFootprint::none().with(
			StoreVolume::new(volume.hash, volume.name.as_deref()),
			VolumeAccess::Write,
		),
	}
}

/// Whether a queued item and the in-flight work disagree about the pending-work scan:
/// a scan waits for in-flight writers of pending rows, and such a writer waits for an
/// in-flight scan.
pub(crate) fn pending_scan_conflict(
	state: &MaintainerState,
	work: &StoreWork,
	footprint: &StoreFootprint,
) -> bool {
	let index = state.operations.index();
	let scanning = index
		.store_tablet
		.values()
		.any(|access| *access == TabletAccess::PendingScan);
	if footprint.tablet == TabletAccess::PendingScan {
		return scanning || !index.pending_writers.is_empty();
	}
	scanning && work_writes_pending_rows(work)
}

/// Whether no store operation with a footprint is in flight.
pub(crate) fn store_idle(state: &MaintainerState) -> bool {
	let index = state.operations.index();
	index.store_tablet.is_empty() && index.store_volumes.is_empty()
}

/// Whether two in-flight tablet accesses may overlap.
fn tablets_compatible(a: TabletAccess, b: TabletAccess) -> bool {
	use TabletAccess::{Exclusive, Independent, None, PendingScan};
	match (a, b) {
		(None, _) | (_, None) => true,
		(Exclusive, _) | (_, Exclusive) => false,
		(Independent, Independent | PendingScan) | (PendingScan, Independent) => true,
		(PendingScan, PendingScan) => false,
	}
}

/// Checks that no two in-flight footprints conflict. A full pairwise pass for debug
/// validation only.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	let footprints = state
		.operations
		.iter()
		.map(|(id, operation)| (*id, operation_footprint(operation)))
		.filter(|(_, footprint)| !footprint.is_empty())
		.collect::<Vec<_>>();
	for (index, (a_id, a)) in footprints.iter().enumerate() {
		for (b_id, b) in &footprints[index + 1..] {
			anyhow::ensure!(
				tablets_compatible(a.tablet, b.tablet),
				"store operations {a_id:?} and {b_id:?} overlap tablet-wide"
			);
			anyhow::ensure!(
				!(a.tablet == TabletAccess::Exclusive || b.tablet == TabletAccess::Exclusive),
				"exclusive store operation {a_id:?} or {b_id:?} overlaps other store work"
			);
			for (scan, other) in [(a, b_id), (b, a_id)] {
				if scan.tablet == TabletAccess::PendingScan {
					anyhow::ensure!(
						!state.operations.get(other).is_some_and(writes_pending_rows),
						"a pending-work scan overlaps {other:?}, a writer of pending rows"
					);
				}
			}
			for (volume, a_access) in a.volumes.iter() {
				let Some(b_access) = b.volumes.get(volume) else {
					continue;
				};
				anyhow::ensure!(
					compatible(*a_access, b_access)
						|| !live(state, a_id, *a_access)
						|| !live(state, b_id, b_access),
					"store operations {a_id:?} ({a_access:?}) and {b_id:?} ({b_access:?}) overlap on one volume"
				);
			}
		}
	}
	Ok(())
}


