//! Admission to the tablet's cold store. Queued work has no I/O deadline yet; an
//! operation's deadline starts only when it is dispatched.
//!
//! Store work is admitted by footprint (`footprint.rs`): a tablet-wide access and one
//! access per volume it touches. Two pieces of work may overlap only when their
//! footprints are compatible, and each volume keeps its queue order.
//!
//! Tablet-wide (`Exclusive`) work is every mutation other than keyed work and passes:
//! flushes while staging, materialize, fork, checkpoint, split, sweeps, retired
//! deletion and pending-work scans. It starts only on an idle store, in queue order, and
//! nothing queued behind it starts first.
//!
//! Ordered reads are session opens (`Read`) and lifecycle row lookups (`Lookup`), both
//! the opening lookups and the background ones for unloading and deletion. They resolve
//! current records and positions, so each observes every mutation of its volume queued
//! before it. Opens of one volume overlap each other; a lookup excludes every other use
//! of its volume, so lifecycle steps see lookups complete in order. Filesystem applies
//! and loads (`Keyed`) exclude other keyed work and ordered reads of their volume.
//!
//! A cold write pass writes only the records of the volumes it carries (`Write`). An
//! uploading flush's records are written but not durable (`Unpublished`): opens, applies
//! and loads of its volumes may run beside it, and lookups of them may not.
//!
//! Session reads (get, scan, chunks and close on an existing session) read only the
//! snapshot pinned when the session opened. That snapshot had to stay valid while
//! arbitrary mutations ran between two reads of the session, so overlapping a
//! mutation changes nothing a session read can observe. They bypass the store queue
//! entirely and wait only for a read slot. Every completion still rechecks current
//! lifecycle facts before replying.
//!
//! A queued open waits for its own volume's load. When the queue starts with loads
//! and opens only, the work of volumes an open is waiting on moves ahead of loads
//! that no open needs, so a foreground read never waits behind unrelated recovery.
//! Loads read only their own volume's record and opens observe only their own
//! volume, so this reordering keeps every volume's own order and changes nothing
//! either observes.
//!
//! Ordered and session reads share the client window, `max_cold_reads`. When a slot frees, the
//! store queue is driven first so an open at its head cannot starve behind a steady
//! stream of session reads. Held-image source reads are bounded at admission by the
//! separate background window, `max_background_cold_reads`. Nothing here runs outside the `Serving` and `Frozen` phases, and fencing
//! clears both queues.

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::types::{OpId, RequestId};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum StoreWork {
	Materialize {
		scope: depot_fs_core::keys::slatedb::Scope,
	},
	ForkCleanup {
		work: depot_fs_core::forks::cleanup::Work,
	},
	Fork {
		volume_key: depot_types::VolumeKey,
	},
	ForkReference {
		request_id: crate::tablet_maintainer_machine::types::RequestId,
	},
	PendingWork,
	SweepPending {
		scope: depot_fs_core::keys::slatedb::Scope,
	},
	Checkpoint {
		request_id: crate::tablet_maintainer_machine::types::RequestId,
	},
	FilesystemQuery {
		request_id: crate::tablet_maintainer_machine::types::RequestId,
	},
	FilesystemLoad {
		key: depot_types::VolumeLogKey,
	},
	FilesystemApply {
		key: depot_types::VolumeLogKey,
	},
	FilesystemFlush,

	ReadDeleteRow {
		key: depot_types::VolumeLogKey,
	},
	SweepDeleted {
		key: depot_types::VolumeLogKey,
	},
	PrepareSplit,
	DeleteRetired {
		tablet_id: [u8; 16],
	},
	ReadUnloadRow {
		key: depot_types::VolumeLogKey,
	},

	ReadRow {
		volume_key: depot_types::VolumeKey,
	},
	WriteToCold {
		/// Operator-triggered passes capture a finite read page after the barrier
		/// status round of this generation finishes.
		after_observations: Option<u64>,
		reads: Option<BTreeSet<crate::tablet_maintainer_machine::types::OpId>>,
	},
}

/// The ordered store queue with an index of what it holds.
///
/// The deque is the authority and keeps admission order. Every mutation goes through
/// this type, which withdraws the old items' contributions and adds the new ones, so
/// membership checks and counts by kind never scan the queue. `version` changes on
/// every mutation; store admission reruns only when it or its other inputs change.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct StoreQueue {
	work: VecDeque<StoreWork>,
	index: StoreQueueIndex,
	version: u64,
	/// Changes on every mutation except an append at the back. While it is unchanged,
	/// the items an earlier admission scan already judged are still the same items in
	/// the same order, so a resumed scan only has to look at the appended tail.
	edits: u64,
}

/// Queued work by kind and by the key the drives look it up by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct StoreQueueIndex {
	counts: QueueSample,
	loads: BTreeMap<depot_types::VolumeLogKey, u32>,
	applies: BTreeMap<depot_types::VolumeLogKey, u32>,
	unload_reads: BTreeMap<depot_types::VolumeLogKey, u32>,
	checkpoints: BTreeMap<RequestId, u32>,
	fork_references: BTreeMap<RequestId, u32>,
	queries: BTreeMap<RequestId, u32>,
	row_reads: BTreeMap<depot_types::VolumeKey, u32>,
	sweeps: BTreeMap<depot_types::VolumeLogKey, u32>,
	retired: BTreeMap<[u8; 16], u32>,
	prepare_split: u32,
	flushes: u32,
	/// Operator passes waiting for fresh observations before they capture their rows.
	waiting_passes: u32,
}

impl Serialize for StoreQueue {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.work.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for StoreQueue {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let work = VecDeque::<StoreWork>::deserialize(deserializer)?;
		let index = StoreQueueIndex::recompute(&work);
		Ok(Self {
			work,
			index,
			version: 0,
			edits: 0,
		})
	}
}

impl std::ops::Index<usize> for StoreQueue {
	type Output = StoreWork;

	fn index(&self, index: usize) -> &StoreWork {
		&self.work[index]
	}
}

impl StoreQueue {
	pub(crate) fn len(&self) -> usize {
		self.work.len()
	}

	

	pub(crate) fn iter(&self) -> std::collections::vec_deque::Iter<'_, StoreWork> {
		self.work.iter()
	}

	pub(crate) fn front(&self) -> Option<&StoreWork> {
		self.work.front()
	}

	/// Queued work by kind, for metrics.
	pub(crate) fn counts(&self) -> QueueSample {
		self.index.counts
	}

	/// Whether `work` is queued. Kinds the index tracks by key answer without a scan.
	pub(crate) fn contains(&self, work: &StoreWork) -> bool {
		match work {
			StoreWork::FilesystemLoad { key } => self.index.loads.contains_key(key),
			StoreWork::FilesystemApply { key } => self.index.applies.contains_key(key),
			StoreWork::ReadUnloadRow { key } => self.index.unload_reads.contains_key(key),
			StoreWork::Checkpoint { request_id } => self.index.checkpoints.contains_key(request_id),
			StoreWork::ForkReference { request_id } => {
				self.index.fork_references.contains_key(request_id)
			}
			StoreWork::FilesystemQuery { request_id } => {
				self.index.queries.contains_key(request_id)
			}
			StoreWork::ReadRow { volume_key } => self.index.row_reads.contains_key(volume_key),
			StoreWork::SweepDeleted { key } => self.index.sweeps.contains_key(key),
			StoreWork::DeleteRetired { tablet_id } => self.index.retired.contains_key(tablet_id),
			StoreWork::PrepareSplit => self.index.prepare_split > 0,
			StoreWork::FilesystemFlush => self.index.flushes > 0,
			StoreWork::Materialize { .. }
			| StoreWork::ForkCleanup { .. }
			| StoreWork::Fork { .. }
			| StoreWork::PendingWork
			| StoreWork::SweepPending { .. }
			| StoreWork::ReadDeleteRow { .. }
			| StoreWork::WriteToCold { .. } => self.work.contains(work),
		}
	}

	/// Whether any filesystem load is queued.
	pub(crate) fn has_loads(&self) -> bool {
		!self.index.loads.is_empty()
	}

	/// Whether a filesystem apply or load of any incarnation of `volume_key` is queued.
	pub(crate) fn has_keyed_work(&self, volume_key: &depot_types::VolumeKey) -> bool {
		let incarnations = super::incarnations(volume_key);
		self.index
			.loads
			.range(incarnations.clone())
			.next()
			.is_some()
			|| self.index.applies.range(incarnations).next().is_some()
	}

	/// Whether a cold write pass is queued.
	pub(crate) fn has_pass(&self) -> bool {
		self.index.counts.passes > 0
	}

	/// Whether the queued pass waits for a fresh round of observations first.
	pub(crate) fn has_waiting_pass(&self) -> bool {
		self.index.waiting_passes > 0
	}

	pub(crate) fn push_back(&mut self, work: StoreWork) {
		self.index.apply(&work, true);
		self.work.push_back(work);
		self.version = self.version.wrapping_add(1);
	}

	pub(crate) fn remove(&mut self, index: usize) -> Option<StoreWork> {
		let work = self.work.remove(index)?;
		self.index.apply(&work, false);
		self.changed();
		Some(work)
	}

	pub(crate) fn retain(&mut self, mut keep: impl FnMut(&StoreWork) -> bool) {
		let index = &mut self.index;
		let before = self.work.len();
		self.work.retain(|work| {
			let kept = keep(work);
			if !kept {
				index.apply(work, false);
			}
			kept
		});
		if self.work.len() != before {
			self.changed();
		}
	}

	/// Removes every queued item equal to one of `items`. The index answers first, so a call
	/// for work that is not queued costs a lookup per item rather than a scan of the queue.
	pub(crate) fn remove_all(&mut self, items: &[StoreWork]) {
		if !items.iter().any(|item| self.contains(item)) {
			return;
		}
		self.retain(|work| !items.contains(work));
	}

	pub(crate) fn clear(&mut self) {
		self.work.clear();
		self.index = StoreQueueIndex::default();
		self.changed();
	}

	/// Replaces the run starting at `start` with a permutation of the same items.
	pub(crate) fn reorder(&mut self, start: usize, items: Vec<StoreWork>) {
		for (offset, work) in items.into_iter().enumerate() {
			let slot = &mut self.work[start + offset];
			self.index.apply(slot, false);
			self.index.apply(&work, true);
			*slot = work;
		}
		self.changed();
	}

	/// Records an operator intent to wait for fresh observations on the queued pass,
	/// or queues a new pass. Returns nothing; at most one pass is ever queued.
	fn queue_pass(&mut self, wait: Option<u64>) {
		if self.has_pass() {
			if let Some(generation) = wait
				&& let Some(position) = self
					.work
					.iter()
					.position(|work| matches!(work, StoreWork::WriteToCold { .. }))
			{
				self.modify(position, |work| {
					if let StoreWork::WriteToCold {
						after_observations, ..
					} = work
					{
						*after_observations = (*after_observations).max(Some(generation));
					}
				});
			}
			return;
		}
		self.push_back(StoreWork::WriteToCold {
			after_observations: wait,
			reads: None,
		});
	}

	/// Fixes the reads an operator pass at the head of the queue waits for.
	fn capture_front_pass_reads(&mut self, captured: BTreeSet<OpId>) {
		if matches!(self.work.front(), Some(StoreWork::WriteToCold { .. })) {
			self.modify(0, |work| {
				if let StoreWork::WriteToCold { reads, .. } = work {
					*reads = Some(captured);
				}
			});
		}
	}

	/// Changes one queued item in place, keeping the index exact across the change.
	fn modify(&mut self, position: usize, change: impl FnOnce(&mut StoreWork)) {
		let work = &mut self.work[position];
		self.index.apply(work, false);
		change(work);
		self.index.apply(work, true);
		self.changed();
	}

	fn changed(&mut self) {
		self.version = self.version.wrapping_add(1);
		self.edits = self.edits.wrapping_add(1);
	}

	/// Records that an admission input outside the queue changed: a waiting filesystem
	/// caller left, which can unblock queued work of its volume.
	pub(crate) fn touch(&mut self) {
		self.changed();
	}

	/// Checks the index against a full recomputation of the queue.
	pub(crate) fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			StoreQueueIndex::recompute(&self.work) == self.index,
			"store queue index drifted from its queue"
		);
		Ok(())
	}
}

impl StoreQueueIndex {
	fn recompute(work: &VecDeque<StoreWork>) -> Self {
		let mut index = Self::default();
		for work in work {
			index.apply(work, true);
		}
		index
	}

	fn apply(&mut self, work: &StoreWork, add: bool) {
		let counts = &mut self.counts;
		match work {
			StoreWork::FilesystemQuery { request_id } => {
				count(&mut counts.opens, add);
				count_key(&mut self.queries, request_id, add);
			}
			StoreWork::FilesystemLoad { key } => {
				count(&mut counts.loads, add);
				count_key(&mut self.loads, key, add);
			}
			StoreWork::FilesystemApply { key } => {
				count(&mut counts.applies, add);
				count_key(&mut self.applies, key, add);
			}
			StoreWork::FilesystemFlush => {
				count(&mut counts.flushes, add);
				count(&mut self.flushes, add);
			}
			StoreWork::ReadUnloadRow { key } => {
				count(&mut counts.row_reads, add);
				count_key(&mut self.unload_reads, key, add);
			}
			StoreWork::ReadRow { volume_key } => {
				count(&mut counts.row_reads, add);
				count_key(&mut self.row_reads, volume_key, add);
			}
			StoreWork::ReadDeleteRow { .. } => count(&mut counts.row_reads, add),
			StoreWork::WriteToCold {
				after_observations, ..
			} => {
				count(&mut counts.passes, add);
				if after_observations.is_some() {
					count(&mut self.waiting_passes, add);
				}
			}
			StoreWork::SweepDeleted { key } => {
				count(&mut counts.other, add);
				count_key(&mut self.sweeps, key, add);
			}
			StoreWork::DeleteRetired { tablet_id } => {
				count(&mut counts.other, add);
				count_key(&mut self.retired, tablet_id, add);
			}
			StoreWork::PrepareSplit => {
				count(&mut counts.other, add);
				count(&mut self.prepare_split, add);
			}
			StoreWork::Materialize { .. }
			| StoreWork::ForkCleanup { .. }
			| StoreWork::Fork { .. }
			| StoreWork::PendingWork
			| StoreWork::SweepPending { .. } => count(&mut counts.other, add),
			StoreWork::ForkReference { request_id } => {
				count(&mut counts.other, add);
				count_key(&mut self.fork_references, request_id, add);
			}
			StoreWork::Checkpoint { request_id } => {
				count(&mut counts.other, add);
				count_key(&mut self.checkpoints, request_id, add);
			}
		}
	}
}

fn count(value: &mut u32, add: bool) {
	*value = if add {
		value.saturating_add(1)
	} else {
		value.saturating_sub(1)
	};
}

fn count_key<K: Ord + Clone>(map: &mut BTreeMap<K, u32>, key: &K, add: bool) {
	if add {
		*map.entry(key.clone()).or_default() += 1;
	} else if let Some(value) = map.get_mut(key) {
		*value = value.saturating_sub(1);
		if *value == 0 {
			map.remove(key);
		}
	}
}

/// Collapse pass triggers while preserving their place behind earlier row lookups.
pub(crate) fn queue_cold_pass(state: &mut MaintainerState) {
	queue_pass(state, None);
}

pub(super) fn queue_pass(state: &mut MaintainerState, wait: Option<u64>) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	state.store_queue.queue_pass(wait);
}

/// Queued store work counted by kind, kept by the store queue as items enter and leave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueSample {
	pub opens: u32,
	pub loads: u32,
	pub applies: u32,
	pub flushes: u32,
	pub row_reads: u32,
	pub passes: u32,
	pub other: u32,
}

/// The capacity and gauge class of an in-flight operation. Admission decides by
/// footprint (`footprint.rs`); these classes only count read and keyed slots and feed
/// the published in-flight gauges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StoreAccess {
	/// Replica, peer and coordinator I/O, or a held-image read bounded elsewhere.
	None,
	/// Forks, checkpoint changes, materialization, sweeps and other work that once held the
	/// whole store. They keep this gauge class but are admitted by their footprints, so
	/// most run beside work of other volumes.
	Exclusive,
	/// A filesystem apply or load. It changes or reads only its own volume, so ordered
	/// reads and keyed work of other volumes may overlap it. The store loop runs each
	/// admitted piece on its own task, so up to `max_keyed_in_flight` of them make
	/// progress at once; the store's volume stripes keep two of one volume apart. Keyed
	/// work never runs beside other exclusive work or a pass, beside keyed work of its
	/// volume, or beside an ordered read of its volume.
	Keyed,
	/// A cold write pass. It owns the store like exclusive work, except that ordered
	/// reads of volumes it does not carry may overlap it.
	Pass,
	OrderedRead,
	SessionRead,
	/// A flush writing its records into the store.
	Staging,
	/// A staged flush making its memtable durable. Opens, applies, loads and row
	/// lookups of volumes it does not carry may overlap it; another flush and every
	/// other mutation wait for it.
	Uploading,
}

pub(crate) fn operation_access(operation: &Operation) -> StoreAccess {
	use super::filesystem_reads::QueryOrder;
	match operation {
		Operation::PendingWork { .. }
		| Operation::SweepPending { .. }
		| Operation::Checkpoint { .. }
		| Operation::ForkReference { .. }
		| Operation::ForkStore { .. }
		| Operation::ForkCleanupStore { .. }
		| Operation::MaterializeStore { .. }
		| Operation::SweepDeleted { .. }
		| Operation::OpenCold
		| Operation::PrepareSplit { .. }
		| Operation::DeleteRetired { .. }
		| Operation::ReplayOp { .. } => StoreAccess::Exclusive,
		Operation::FilesystemFlush {
			stage: super::filesystem::FlushStage::Staging,
			..
		} => StoreAccess::Staging,
		Operation::WriteToCold { .. } => StoreAccess::Pass,
		Operation::FilesystemLoad { .. } | Operation::FilesystemApply { .. } => StoreAccess::Keyed,
		Operation::ReadOpenKeys { .. } => StoreAccess::SessionRead,
		Operation::Repair(operation) => match operation {
			super::operator_repair::RepairOperation::Diagnostics { .. }
			| super::operator_repair::RepairOperation::InspectRead { .. }
			| super::operator_repair::RepairOperation::Prepare { .. }
			| super::operator_repair::RepairOperation::AuditList { .. } => StoreAccess::SessionRead,
			super::operator_repair::RepairOperation::Call { .. }
			| super::operator_repair::RepairOperation::Claim { .. }
			| super::operator_repair::RepairOperation::Forget { .. } => StoreAccess::None,
		},
		Operation::ReadRow { .. }
		| Operation::ReadUnloadRow { .. }
		| Operation::ReadDeleteRow { .. } => StoreAccess::OrderedRead,
		Operation::FilesystemFlush {
			stage: super::filesystem::FlushStage::Uploading,
			..
		} => StoreAccess::Uploading,
		Operation::FilesystemQuery { order, .. } => match order {
			QueryOrder::Ordered => StoreAccess::OrderedRead,
			QueryOrder::Session | QueryOrder::Claim => StoreAccess::SessionRead,
			QueryOrder::HeldImage => StoreAccess::None,
		},
		Operation::Alarm(
			super::alarms::Attempt::Running(request)
			| super::alarms::Attempt::Cancelling(request)
			| super::alarms::Attempt::Obsolete(request),
		) if request.uses_store() => StoreAccess::Keyed,
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
		| Operation::Alarm(_) => StoreAccess::None,
	}
}

/// In-flight store operations by access class, derived from `operations`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct StoreUsage {
	pub exclusive: usize,
	pub staging: usize,
	pub keyed: usize,
	pub passes: usize,
	pub ordered_reads: usize,
	pub session_reads: usize,
	pub uploading: usize,
}

impl StoreUsage {
	pub(crate) fn of(state: &MaintainerState) -> Self {
		state.operations.index().store
	}

	pub(crate) fn reads(&self) -> usize {
		self.ordered_reads + self.session_reads
	}
}

/// Why the oldest queued store item cannot start. Derived from the queue head and the
/// in-flight operations after admission ran, so it names what holds the head back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdmissionBlock {
	/// A tablet-wide operation other than a flush is in flight, or the head is one and
	/// waits for the store to drain.
	TabletOp,
	/// A flush is staging, or the head is a flush waiting for the store to drain.
	FlushBarrier,
	/// A cold write pass is in flight and the head may not overlap it, or the head is a
	/// pass waiting for the store to drain.
	PassBarrier,
	/// In-flight work of the head's own volume conflicts with it.
	VolumeOrder,
	/// Every read slot is taken.
	ReadSlots,
	/// Every keyed slot is taken.
	KeyedSlots,
	/// The head is a flush and every flush staging or upload slot is taken.
	FlushSlots,
	/// The head is an operator pass waiting for fresh observations or captured reads.
	WaitingPass,
}

impl AdmissionBlock {
	pub const ALL: [AdmissionBlock; 8] = [
		AdmissionBlock::TabletOp,
		AdmissionBlock::FlushBarrier,
		AdmissionBlock::PassBarrier,
		AdmissionBlock::VolumeOrder,
		AdmissionBlock::ReadSlots,
		AdmissionBlock::KeyedSlots,
		AdmissionBlock::FlushSlots,
		AdmissionBlock::WaitingPass,
	];

	/// Bounded metric label.
	pub fn label(self) -> &'static str {
		match self {
			AdmissionBlock::TabletOp => "tablet_op",
			AdmissionBlock::FlushBarrier => "flush_barrier",
			AdmissionBlock::PassBarrier => "pass_barrier",
			AdmissionBlock::VolumeOrder => "volume_order",
			AdmissionBlock::ReadSlots => "read_slots",
			AdmissionBlock::KeyedSlots => "keyed_slots",
			AdmissionBlock::FlushSlots => "flush_slots",
			AdmissionBlock::WaitingPass => "waiting_pass",
		}
	}
}

/// The store part of the published snapshot.
pub(crate) fn snapshot(state: &MaintainerState) -> crate::tablet_maintainer_machine::StoreSnapshot {
	let usage = StoreUsage::of(state);
	crate::tablet_maintainer_machine::StoreSnapshot {
		queued_work: state.store_queue.len() as u32,
		queued_session_reads: state.session_reads.len() as u32,
		exclusive_in_flight: usage.exclusive as u32,
		keyed_in_flight: usage.keyed as u32,
		passes_in_flight: usage.passes as u32,
		flush_staging_in_flight: usage.staging as u32,
		flush_uploading_in_flight: usage.uploading as u32,
		ordered_reads_in_flight: usage.ordered_reads as u32,
		session_reads_in_flight: usage.session_reads as u32,
		read_capacity: state.config.max_cold_reads,
		per_volume_read_capacity: state.config.max_cold_reads_per_volume,
		held_image_reads_in_flight: state.operations.index().counts.held_image_query,
		replica_log_reads_in_flight: state.operations.index().counts.filesystem_read,
		fork_jobs: state.forks.jobs.len() as u32,
		fork_cleanup_jobs: state.fork_cleanup.job_count() as u32,
		background_read_capacity: state.config.max_background_cold_reads,
		keyed_capacity: state.config.max_keyed_in_flight,
		flush_staging_capacity: state.config.max_flush_staging,
		flush_upload_capacity: state.config.max_flush_uploads,
		admission_scan_limit: state.config.admission_scan_limit,
		scan_limited: state.store_queue.len() > state.config.admission_scan_limit as usize,
		queued_by_kind: state.store_queue.counts(),
		head_block: admission_block(state),
		initial_bytes_held: state.initial_bytes_held,
		initial_bytes_capacity: state.config.tuning().max_pending_initial_bytes,
		loaded_volumes: state.loaded_volumes.len() as u32,
		loaded_volume_capacity: state.config.max_loaded_volumes,
		initial_writes_withheld: state.initial_writes_withheld,
	}
}

/// Classifies why the queue head cannot start, from state alone.
pub(crate) fn admission_block(state: &MaintainerState) -> Option<AdmissionBlock> {
	use super::footprint::{TabletAccess, TabletInFlight, WorkSlot};
	if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) {
		return None;
	}
	let head = state.store_queue.front()?;
	if AdmissionInputs::of(state).waiting_front {
		return Some(AdmissionBlock::WaitingPass);
	}
	let tablet = TabletInFlight::of(state);
	let usage = StoreUsage::of(state);
	if tablet.exclusive {
		return Some(if usage.staging > 0 {
			AdmissionBlock::FlushBarrier
		} else {
			AdmissionBlock::TabletOp
		});
	}
	let footprint = super::footprint::work_footprint(head, state);
	match footprint.tablet {
		TabletAccess::None | TabletAccess::Independent | TabletAccess::PendingScan => {
			if super::footprint::pending_scan_conflict(state, head, &footprint) {
				return Some(AdmissionBlock::TabletOp);
			}
			if footprint
				.volumes
				.iter()
				.any(|(volume, access)| super::footprint::volume_conflict(state, volume, *access))
			{
				return Some(AdmissionBlock::VolumeOrder);
			}
			match super::footprint::work_slot(head) {
				WorkSlot::Read if usage.reads() >= state.config.max_cold_reads as usize => {
					Some(AdmissionBlock::ReadSlots)
				}
				WorkSlot::Keyed if usage.keyed >= state.config.max_keyed_in_flight as usize => {
					Some(AdmissionBlock::KeyedSlots)
				}
				WorkSlot::Flush if super::footprint::flush_slot_full(state) => {
					Some(AdmissionBlock::FlushSlots)
				}
				WorkSlot::Pass if state.operations.index().store.passes > 0 => {
					Some(AdmissionBlock::PassBarrier)
				}
				WorkSlot::Read
				| WorkSlot::Keyed
				| WorkSlot::Flush
				| WorkSlot::Pass
				| WorkSlot::None => None,
			}
		}
		TabletAccess::Exclusive => {
			if !super::footprint::store_idle(state) {
				return Some(barrier_reason(head));
			}
			None
		}
	}
}

/// The reason a queued barrier waiting for the store to drain reports.
fn barrier_reason(work: &StoreWork) -> AdmissionBlock {
	match work {
		StoreWork::FilesystemFlush => AdmissionBlock::FlushBarrier,
		StoreWork::WriteToCold { .. } => AdmissionBlock::PassBarrier,
		StoreWork::Materialize { .. }
		| StoreWork::ForkCleanup { .. }
		| StoreWork::Fork { .. }
		| StoreWork::ForkReference { .. }
		| StoreWork::PendingWork
		| StoreWork::SweepPending { .. }
		| StoreWork::Checkpoint { .. }
		| StoreWork::FilesystemQuery { .. }
		| StoreWork::FilesystemLoad { .. }
		| StoreWork::FilesystemApply { .. }
		| StoreWork::ReadDeleteRow { .. }
		| StoreWork::SweepDeleted { .. }
		| StoreWork::PrepareSplit
		| StoreWork::DeleteRetired { .. }
		| StoreWork::ReadUnloadRow { .. }
		| StoreWork::ReadRow { .. } => AdmissionBlock::TabletOp,
	}
}

/// Start the deadline only when this operation owns its store slot, never while it waits.
///
/// Queue admission reruns only when one of its inputs changed since it last settled.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	drive_admission(state, effects, false);
}

/// Store admission as `drive` runs it; `force` reruns queue admission even when its
/// inputs are unchanged, for the debug oracle's full pass.
pub(crate) fn drive_admission(state: &mut MaintainerState, effects: &mut Effects, force: bool) {
	if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) {
		return;
	}
	if force || state.store_admission.as_ref() != Some(&AdmissionInputs::of(state)) {
		prioritize_waiting_opens(state);
		drive_queue(state, effects, !force);
		state.store_admission = Some(AdmissionInputs::of(state));
	}
	drive_session_reads(state, effects);
}

/// Everything queue admission reads, by version where the input is large. When these
/// equal the record from the last settled admission, another pass would admit nothing.
///
/// `select` reads the queue, the in-flight store operations, waiting filesystem callers
/// and the status round. Callers enter `pending_fs_reads` only with a queue push or a
/// session read, which `select` does not read, and leave it through a reply that bumps
/// the queue version. Dispatch-time checks run only after `select` chose an item, and
/// removing that item bumps the queue version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AdmissionInputs {
	store_version: u64,
	session_reads_released: u64,
	queue_version: u64,
	observed_generation: u64,
	phase: MaintainerPhase,
	/// Whether an operator pass at the head still waits for its captured reads, which
	/// are not store operations and so do not change the store version as they finish.
	waiting_front: bool,
}

impl AdmissionInputs {
	fn of(state: &MaintainerState) -> Self {
		let waiting_front = match state.store_queue.front() {
			Some(StoreWork::WriteToCold {
				after_observations: Some(_),
				reads,
			}) => reads
				.as_ref()
				.is_none_or(|reads| reads.iter().any(|id| state.operations.contains_key(id))),
			_ => false,
		};
		Self {
			store_version: state.operations.index().store_version,
			session_reads_released: state.operations.index().session_reads_released,
			queue_version: state.store_queue.version,
			observed_generation: state.status_polls.observed_generation,
			phase: state.phase.clone(),
			waiting_front,
		}
	}
}

/// Moves the loads and opens of volumes that a queued open waits on ahead of other
/// loads, within the leading run of the queue that holds only loads and opens.
fn prioritize_waiting_opens(state: &mut MaintainerState) {
	// Only loads ever move, so a queue without loads keeps its order.
	if !state.store_queue.has_loads() {
		return;
	}
	// A waiting operator pass at the head is skipped by admission; keep it in place.
	let start = usize::from(matches!(
		state.store_queue.front(),
		Some(StoreWork::WriteToCold { .. })
	));
	let region = state
		.store_queue
		.iter()
		.skip(start)
		.take_while(|work| {
			matches!(
				work,
				StoreWork::FilesystemLoad { .. } | StoreWork::FilesystemQuery { .. }
			)
		})
		.count();
	if region < 2 {
		return;
	}

	let waiting: BTreeSet<depot_types::VolumeKey> = state
		.store_queue
		.iter()
		.skip(start)
		.take(region)
		.filter_map(|work| {
			let StoreWork::FilesystemQuery { request_id } = work else {
				return None;
			};
			state
				.pending_fs_reads
				.get(request_id)
				.map(|request| request.volume_key().clone())
		})
		.collect();
	// Opens themselves always move: each observes only its own volume.
	let required = |work: &StoreWork| {
		let StoreWork::FilesystemLoad { key } = work else {
			return true;
		};
		waiting.contains(&key.0.volume_key)
	};
	let leading = || state.store_queue.iter().skip(start).take(region);
	if leading().take_while(|work| required(work)).count()
		== leading().filter(|work| required(work)).count()
	{
		return;
	}
	let leading = leading().cloned().collect::<Vec<_>>();

	// A stable partition keeps each volume's load ahead of its opens.
	let (first, rest): (Vec<_>, Vec<_>) = leading.into_iter().partition(|work| required(work));
	state
		.store_queue
		.reorder(start, first.into_iter().chain(rest).collect());
}

/// Admits queued session reads into free read slots, in arrival order, except that a volume
/// already holding `max_cold_reads_per_volume` session reads lets later reads of other volumes
/// go first. Reads passed over keep their order at the front of the queue, and take any slots
/// left once no other volume's read in the scanned window is admissible, so the share only
/// applies under contention. A pass runs only while a slot is free, and it fills every free slot
/// it can, so its cost (one walk of the operations and of at most `admission_scan_limit` queued
/// reads) is paid about once per released slot.
fn drive_session_reads(state: &mut MaintainerState, effects: &mut Effects) {
	let max_reads = state.config.max_cold_reads as usize;
	let mut reads = StoreUsage::of(state).reads();
	let per_volume = state.config.max_cold_reads_per_volume as usize;
	if per_volume == 0 {
		while reads < max_reads {
			let Some(request_id) = state.session_reads.pop_front() else {
				break;
			};
			if super::filesystem_reads::dispatch(state, request_id, effects) {
				reads += 1;
			}
		}
		return;
	}
	if reads >= max_reads || state.session_reads.is_empty() {
		return;
	}

	// Session reads in flight by volume.
	let mut in_flight: BTreeMap<depot_types::VolumeKey, usize> = BTreeMap::new();
	for operation in state.operations.values() {
		if let Operation::FilesystemQuery {
			request_id,
			order:
				super::filesystem_reads::QueryOrder::Session
				| super::filesystem_reads::QueryOrder::Claim,
			..
		} = operation
			&& let Some(request) = state.pending_fs_reads.get(request_id)
		{
			*in_flight.entry(request.volume_key().clone()).or_default() += 1;
		}
	}

	// Admit in arrival order, passing over reads of volumes at their share. The scan is
	// bounded like store admission, so a deep queue of one volume costs a bounded pass.
	let scan_limit = state.config.admission_scan_limit.max(1) as usize;
	let mut passed_over = VecDeque::new();
	let mut scanned = 0;
	while reads < max_reads && scanned < scan_limit {
		let Some(request_id) = state.session_reads.pop_front() else {
			break;
		};
		scanned += 1;
		let Some(volume) = state
			.pending_fs_reads
			.get(&request_id)
			.map(|request| request.volume_key().clone())
		else {
			// Its caller left; dispatch drops it without taking a slot.
			super::filesystem_reads::dispatch(state, request_id, effects);
			continue;
		};
		let held = in_flight.entry(volume).or_default();
		if *held >= per_volume {
			passed_over.push_back(request_id);
			continue;
		}
		if super::filesystem_reads::dispatch(state, request_id, effects) {
			reads += 1;
			*held += 1;
		}
	}
	if reads < max_reads {
		// No other admissible volume's read is left in the scanned window: the capped volumes
		// take the free slots, oldest first, so the share never leaves a slot idle.
		while reads < max_reads {
			let Some(request_id) = passed_over.pop_front() else {
				break;
			};
			if super::filesystem_reads::dispatch(state, request_id, effects) {
				reads += 1;
			}
		}
	}
	if !passed_over.is_empty() {
		crate::metrics::TABLET_SESSION_READS_PASSED_OVER.inc_by(passed_over.len() as u64);
	}
	for request_id in passed_over.into_iter().rev() {
		state.session_reads.push_front(request_id);
	}
}



/// `select` continued from `progress`, which it advances as it judges items.
fn select_resuming(
	state: &MaintainerState,
	usage: &StoreUsage,
	start: usize,
	progress: &mut ScanProgress,
) -> Option<usize> {
	use super::footprint::{TabletAccess, TabletInFlight, WorkSlot};
	let tablet = TabletInFlight::of(state);
	if tablet.exclusive {
		return None;
	}
	if progress.at_barrier {
		return None;
	}
	let idle = super::footprint::store_idle(state);
	let max_reads = state.config.max_cold_reads as usize;
	let max_keyed = state.config.max_keyed_in_flight as usize;
	let end = state
		.store_queue
		.len()
		.min(start.saturating_add(state.config.admission_scan_limit as usize));
	for index in progress.next.max(start)..end {
		let work = &state.store_queue[index];
		let footprint = super::footprint::work_footprint(work, state);
		if footprint
			.volumes
			.keys()
			.any(|volume| progress.waiting_volumes.contains(volume))
		{
			// A multi-volume item carries the dependency to all volumes it touches.
			progress
				.waiting_volumes
				.extend(footprint.volumes.into_keys());
			continue;
		}
		// A flush whose every owed volume waits on work queued ahead of it, with nothing else
		// to flush, would only requeue itself, and the scan would pick it again at once. It
		// waits like blocked work until that work has started.
		if matches!(work, StoreWork::FilesystemFlush)
			&& super::filesystem::flush_may_wait(state)
			&& super::filesystem::flush_waits_on_ahead(
				state,
				&queued_predecessors(state, index, start),
			) {
			// The verdict reads flush requests and the maintainer log's wants, which the
			// admission cursor does not track, so no cursor is kept past a held flush.
			progress.held_flush = true;
			progress.skipped = true;
			progress
				.waiting_volumes
				.extend(footprint.volumes.into_keys());
			continue;
		}
		match footprint.tablet {
			// Retired-store deletion conflicts only with tablet-wide work, which is not
			// in flight here, and a pending-work scan only with writers of pending rows,
			// so both start like per-volume work.
			TabletAccess::None | TabletAccess::Independent | TabletAccess::PendingScan => {
				let slot_full = match super::footprint::work_slot(work) {
					WorkSlot::Read => {
						let full = usage.reads() >= max_reads;
						progress.read_slot_full |= full;
						full
					}
					WorkSlot::Keyed => usage.keyed >= max_keyed,
					WorkSlot::Flush => super::footprint::flush_slot_full(state),
					WorkSlot::Pass => usage.passes > 0,
					WorkSlot::None => false,
				};
				let blocked = slot_full
					|| super::footprint::pending_scan_conflict(state, work, &footprint)
					|| footprint.volumes.iter().any(|(volume, access)| {
						super::footprint::volume_conflict(state, volume, *access)
					});
				if blocked {
					progress.skipped = true;
					progress
						.waiting_volumes
						.extend(footprint.volumes.into_keys());
					continue;
				}
				return Some(index);
			}
			// A barrier keeps its place in the queue.
			TabletAccess::Exclusive => {
				if !progress.skipped && idle {
					return Some(index);
				}
				progress.next = index;
				progress.at_barrier = true;
				return None;
			}
		}
	}
	progress.next = end;
	None
}

/// Volumes of queued work whose order a flush or pass must preserve. The admission scan
/// and the held-flush verdict use the same set, so a flush is held only on work its
/// dispatch would also wait on.
fn queued_predecessors(
	state: &MaintainerState,
	index: usize,
	start: usize,
) -> BTreeSet<super::footprint::StoreVolume> {
	state
		.store_queue
		.iter()
		.take(index)
		// The waiting observation trigger is deliberately bypassed by admission too.
		.skip(start)
		.flat_map(|work| {
			super::footprint::work_footprint(work, state)
				.volumes
				.into_keys()
		})
		.collect()
}

/// How far a store admission scan got, and what it learned on the way, so a later scan
/// of the same items can continue from there.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ScanProgress {
	/// The next queue position to judge.
	next: usize,
	/// Volumes whose queued work had to wait, so their later work waits too.
	waiting_volumes: super::footprint::VolumeSet,
	/// Whether some judged item had to wait, which keeps a later barrier from starting.
	skipped: bool,
	/// The scan reached a barrier that may not start; nothing behind it is judged.
	at_barrier: bool,
	/// Some judged item waited only because every read slot was taken, so a session read
	/// that ends may let it start.
	read_slot_full: bool,
	/// A flush was held because only work queued ahead of it could give it anything to do.
	held_flush: bool,
}

/// Where the last settled admission scan stopped. When the store, the observations,
/// the phase and every queued item it judged are unchanged, and only new work was
/// appended, each judged item would be judged the same again, so the next scan starts
/// at the appended tail. Without this, every queued open of a burst rescans the whole
/// window, which made each event cost grow with the queue.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AdmissionCursor {
	store_version: u64,
	session_reads_released: u64,
	observed_generation: u64,
	phase: MaintainerPhase,
	queue_edits: u64,
	start: usize,
	progress: ScanProgress,
}

impl AdmissionCursor {
	fn of(state: &MaintainerState, start: usize, progress: ScanProgress) -> Self {
		Self {
			store_version: state.operations.index().store_version,
			session_reads_released: state.operations.index().session_reads_released,
			observed_generation: state.status_polls.observed_generation,
			phase: state.phase.clone(),
			queue_edits: state.store_queue.edits,
			start,
			progress,
		}
	}

	/// Whether nothing but appends changed since this scan settled. A session read that
	/// began or ended since changes no verdict unless some item waited for a read slot.
	fn current(&self, state: &MaintainerState, start: usize) -> bool {
		let index = state.operations.index();
		let reads_released = self.session_reads_released != index.session_reads_released;
		self.store_version == index.store_version
			&& !(reads_released && self.progress.read_slot_full)
			&& self.observed_generation == state.status_polls.observed_generation
			&& self.phase == state.phase
			&& self.queue_edits == state.store_queue.edits
			&& self.start == start
	}
}

/// Admits the store queue in order, as `select` decides, until nothing more may start.
/// With `resume`, a scan continues from the last settled one when only appends happened
/// since; the debug oracle's full pass passes `false` and rescans from the start, which
/// proves the resumed scan reaches the same verdicts.
fn drive_queue(state: &mut MaintainerState, effects: &mut Effects, resume: bool) {
	loop {
		let usage = StoreUsage::of(state);
		if super::footprint::TabletInFlight::of(state).exclusive {
			break;
		}
		// Capture once after the requested status round. Waiting for every
		// ongoing read to drain can starve forever under continuous appends.
		if usage.passes == 0
			&& matches!(
				state.store_queue.front(),
				Some(StoreWork::WriteToCold {
					after_observations: Some(generation),
					reads: None
				}) if super::poll::refreshed(state, *generation)
			) && !state.store_queue.has_loads()
		{
			// One page per already-admitted read is enough for this bounded pass;
			// waiting for a whole suffix defeats cold batching.
			let reads = state
				.operations
				.index()
				.filesystem_reads
				.values()
				.flatten()
				.copied()
				.collect();
			state.store_queue.capture_front_pass_reads(reads);
		}
		// Ready store work behind the barrier must still run, especially the
		// apply effects needed to satisfy its fixed target.
		let waiting = match state.store_queue.front() {
			Some(StoreWork::WriteToCold {
				after_observations: Some(_),
				reads,
			}) => reads
				.as_ref()
				.is_none_or(|reads| reads.iter().any(|id| state.operations.contains_key(id))),
			_ => false,
		};
		let start = usize::from(waiting);
		// The cursor is taken, not cloned: its waiting set holds every blocked volume of a
		// long queue, and copying it per event would cost what the resume saves.
		let resumed = match state.admission_cursor.take() {
			Some(cursor) if resume && cursor.current(state, start) => Some(cursor.progress),
			_ => None,
		};
		let mut progress = resumed.unwrap_or(ScanProgress {
			next: start,
			..Default::default()
		});
		let Some(index) = select_resuming(state, &usage, start, &mut progress) else {
			state.admission_cursor = match progress.held_flush {
				true => None,
				false => Some(AdmissionCursor::of(state, start, progress)),
			};
			break;
		};
		// A flush skips volumes that queued work ahead of it touches.
		let ahead = if matches!(
			state.store_queue[index],
			StoreWork::FilesystemFlush | StoreWork::WriteToCold { .. }
		) {
			queued_predecessors(state, index, start)
		} else {
			BTreeSet::new()
		};
		let Some(work) = state.store_queue.remove(index) else {
			break;
		};
		match work {
			StoreWork::Materialize { scope } => super::materialize::dispatch(state, scope, effects),
			StoreWork::ForkCleanup { work } => super::fork_cleanup::dispatch(state, work, effects),
			StoreWork::Fork { volume_key } => super::forks::dispatch(state, volume_key, effects),
			StoreWork::PendingWork => super::pending_work::dispatch(state, effects),
			StoreWork::SweepPending { scope } => {
				if state.phase != MaintainerPhase::Serving {
					continue;
				}
				// A sweep decides holds by the store's rows, which recovered maintainer log
				// ops of its volume may still change; it waits with the held sweeps until
				// that replay ends. A delete's drain still applying the incarnation's tail
				// writes rows a sweep in progress would miss, so the sweep waits for it too.
				if super::maintainer_log::replay::pending_for_scope(state, &scope)
					|| super::drain::applying_scope(state, &scope)
				{
					state.held_sweeps.insert(scope);
					continue;
				}
				let op_id = state.begin(
					Operation::SweepPending {
						scope: scope.clone(),
					},
					effects,
				);
				effects.execute(MaintainerEffect::SweepPendingRequest { op_id, scope });
			}

			StoreWork::ForkReference { request_id } => {
				super::fork_references::dispatch(state, request_id, effects)
			}
			StoreWork::Checkpoint { request_id } => {
				super::checkpoints::dispatch(state, request_id, effects)
			}
			StoreWork::FilesystemQuery { request_id } => {
				super::filesystem_reads::dispatch(state, request_id, effects);
			}
			StoreWork::FilesystemLoad { key } => {
				let op_id = state.begin(Operation::FilesystemLoad { key: key.clone() }, effects);
				effects.execute(MaintainerEffect::FilesystemLoadRequest { op_id, key });
			}
			StoreWork::FilesystemApply { key } => {
				super::filesystem::dispatch_apply(state, key, effects)
			}
			StoreWork::FilesystemFlush => super::filesystem::dispatch_flush(state, &ahead, effects),
			StoreWork::DeleteRetired { tablet_id } => {
				super::retired::dispatch(state, tablet_id, effects)
			}
			StoreWork::SweepDeleted { key } => super::deletion::dispatch_sweep(state, key, effects),
			StoreWork::ReadDeleteRow { key } => {
				if !state.deleting.get(&key).is_some_and(|delete| {
					matches!(delete.step, super::deletion::DeleteStep::ReadingRow { .. })
				}) {
					continue;
				}
				let op_id = state.begin(Operation::ReadDeleteRow { key: key.clone() }, effects);
				effects.execute(MaintainerEffect::ReadRowRequest {
					op_id,
					volume_key: key.0.volume_key,
					incarnation: Some(key.0.volume_incarnation),
				});
			}

			StoreWork::PrepareSplit => super::split::dispatch(state, effects),
			StoreWork::ReadUnloadRow { key } => {
				if !super::unload::ready(state, &key, effects) {
					continue;
				}
				if !state
					.unloading
					.get(&key)
					.is_some_and(|unload| unload.row_check == super::unload::RowCheck::Queued)
				{
					continue;
				}
				let op_id = state.begin(Operation::ReadUnloadRow { key: key.clone() }, effects);
				state
					.unloading
					.get_mut(&key)
					.expect("queued unload")
					.row_check = super::unload::RowCheck::Reading(op_id);
				effects.execute(MaintainerEffect::ReadRowRequest {
					op_id,
					volume_key: key.0.volume_key,
					incarnation: Some(key.0.volume_incarnation),
				});
			}
			StoreWork::ReadRow { volume_key } => {
				if !state.pending.contains_key(&volume_key) {
					continue;
				}
				let op_id = state.begin(
					Operation::ReadRow {
						volume_key: volume_key.clone(),
					},
					effects,
				);
				effects.execute(MaintainerEffect::ReadRowRequest {
					op_id,
					volume_key,
					incarnation: None,
				});
			}
			StoreWork::WriteToCold {
				after_observations, ..
			} => {
				super::cold_pass::dispatch(state, &ahead, effects);
				if after_observations.is_some() {
					// Preserve the operator intent until the refreshed committed
					// pages have entered the ordered store queue. Flush behind
					// them, rather than clearing that intent on an earlier floor.
					let keys = state
						.filesystem
						.volumes
						.keys()
						.filter(|key| !state.deleting.contains_key(*key))
						.cloned()
						.collect::<Vec<_>>();
					for key in keys {
						state.filesystem.request_flush(&key);
					}
					super::filesystem::queue_flush(state);
				}
			}
		}
	}
}


