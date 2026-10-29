//! In-flight maintainer operations and an index of the facts the drives ask about them.
//!
//! The operation map is the only authority. The index is derived state kept inside the
//! same type: every insert, removal and in-place update first withdraws the old
//! operation's contribution and then adds the new one, so the index is always a pure
//! function of the operations. Debug validation compares it with a full recomputation.

use super::{
	Operation,
	catch_up::CopyStep,
	filesystem::FlushStage,
	filesystem_reads::QueryOrder,
	footprint::{TabletAccess, VolumeAccess, VolumeMap},
	store::StoreAccess,
	store::StoreUsage,
};
use crate::tablet_maintainer_machine::types::{OpId, RequestId};
use depot_types::{VolumeKey, VolumeLogKey};
use rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

/// Operations holding each key. A set of identities rather than a count, so a removal
/// withdraws exactly the operation that contributed and empty entries disappear.
type ByKey<K> = BTreeMap<K, BTreeSet<OpId>>;

/// In-flight operations by kind. Filled by an exhaustive match, so a new operation
/// kind must decide which count it belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct OperationCounts {
	pub alarm_keyed: u32,
	pub poll: u32,
	pub promotion_list: u32,
	pub trim: u32,
	pub recover_volume: u32,
	pub catch_up: u32,
	pub report_catch_up: u32,
	/// Retired-tablet deletion and its coordinator report, which share one slot.
	pub retired: u32,
	pub report_parent_log: u32,
	pub report_version_marks: u32,
	/// Replica deletions and drain claims, which share the delete window.
	pub delete_replica: u32,
	pub forget_volume: u32,
	pub forget_orphan: u32,
	pub filesystem_read: u32,
	pub filesystem_flush: u32,
	pub write_to_cold: u32,
	pub held_image_query: u32,
	/// Split preparation and its coordinator report.
	pub split: u32,
	pub list: u32,
	pub load: u32,
	/// Opening row lookups (`ReadRow`).
	pub read_row: u32,
}

/// Facts about the in-flight operations that drives and admission read per event.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct OperationIndex {
	pub alarms: BTreeSet<OpId>,
	pub alarm_pages: BTreeSet<OpId>,
	pub alarm_queued: BTreeSet<OpId>,
	pub alarm_parked: BTreeSet<OpId>,
	pub alarms_by_volume: ByKey<VolumeKey>,
	pub counts: OperationCounts,
	pub store: StoreUsage,
	/// Bumped on every change to an operation that occupies the store before or after
	/// the change, except session reads. Store admission reruns only when this or its other
	/// inputs change. It has no recomputation and is excluded from the debug comparison.
	pub store_version: u64,
	/// Bumped when a session read ends. A session read touches no volume and no tablet-wide
	/// state, so beginning one can only fill a read slot, and ending one matters to admission
	/// only when queued work waited for a read slot. Excluded from the debug comparison.
	pub session_reads_released: u64,
	/// Catch-up copies and filesystem reads, each reserving one read budget of cold bytes.
	pub read_reservations: u32,
	/// Twice the page bytes of every in-flight apply, as the cold byte budget counts them.
	pub apply_bytes: u64,
	pub trims: ByKey<(VolumeLogKey, WorkerId)>,
	pub promotion_lists: ByKey<WorkerId>,
	pub filesystem_reads: ByKey<VolumeLogKey>,
	pub filesystem_applies: ByKey<VolumeLogKey>,
	pub filesystem_loads: ByKey<VolumeLogKey>,
	pub loads: ByKey<VolumeLogKey>,
	pub forget_orphans: ByKey<VolumeLogKey>,
	pub forgets: ByKey<(VolumeLogKey, WorkerId)>,
	pub delete_replicas: ByKey<VolumeLogKey>,
	/// Repair loads and recoveries, which reserve their volume key.
	pub repair_holds: ByKey<VolumeLogKey>,
	/// Catch-up copies that may still recreate a replica.
	pub catch_up_holds: ByKey<VolumeLogKey>,
	/// Catch-up copy steps by learner. Each learner has at most one.
	pub catch_up_learners: ByKey<WorkerId>,
	/// Rows carried by in-flight cold write passes.
	pub pass_rows: ByKey<VolumeLogKey>,
	/// Rows carried by in-flight cold write passes whose status is deleted.
	pub deleted_pass_rows: ByKey<VolumeLogKey>,
	/// Records carried by an in-flight flush, in either stage.
	pub flush_records: ByKey<VolumeLogKey>,
	/// Records carried by an uploading flush.
	pub uploading_records: ByKey<VolumeLogKey>,
	/// Lifecycle row lookups of opening, unloading and deletion, by volume key.
	pub row_reads: ByKey<VolumeKey>,
	/// Opening row lookups alone, by volume key. Admission checks a key against these
	/// per open, which a scan of every operation made linear in the operation count.
	pub read_rows: ByKey<VolumeKey>,
	/// In-flight ordered filesystem queries by volume key. A query blocks its volume
	/// only while its caller is still pending, which callers check per request.
	pub ordered_queries: BTreeMap<VolumeKey, BTreeMap<OpId, RequestId>>,
	/// Per-volume store footprints of in-flight operations. Keyed by operation, so a
	/// removal withdraws exactly its own contribution.
	pub store_volumes: VolumeMap<BTreeMap<OpId, VolumeAccess>>,
	/// Tablet-wide store footprints of in-flight operations, only those other than `None`.
	pub store_tablet: BTreeMap<OpId, TabletAccess>,
	/// In-flight operations that may write pending-work rows.
	pub pending_writers: BTreeSet<OpId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Operations {
	operations: BTreeMap<OpId, Operation>,
	index: OperationIndex,
}

impl Serialize for Operations {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.operations.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Operations {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let operations = BTreeMap::<OpId, Operation>::deserialize(deserializer)?;
		let index = OperationIndex::recompute(&operations);
		Ok(Self { operations, index })
	}
}

impl Operations {
	pub(crate) fn index(&self) -> &OperationIndex {
		&self.index
	}

	pub(crate) fn get(&self, id: &OpId) -> Option<&Operation> {
		self.operations.get(id)
	}

	pub(crate) fn contains_key(&self, id: &OpId) -> bool {
		self.operations.contains_key(id)
	}

	pub(crate) fn iter(&self) -> std::collections::btree_map::Iter<'_, OpId, Operation> {
		self.operations.iter()
	}

	/// Seeks a diagnostic cursor without revisiting preceding operations.
	pub(crate) fn after(
		&self,
		after: Option<OpId>,
	) -> std::collections::btree_map::Range<'_, OpId, Operation> {
		use std::ops::Bound;
		self.operations.range((
			after.map_or(Bound::Unbounded, Bound::Excluded),
			Bound::Unbounded,
		))
	}

	pub(crate) fn values(&self) -> std::collections::btree_map::Values<'_, OpId, Operation> {
		self.operations.values()
	}

	

	

	pub(crate) fn is_empty(&self) -> bool {
		self.operations.is_empty()
	}

	/// Records a newly begun operation. Only `MaintainerState::begin` calls this.
	pub(crate) fn insert(&mut self, id: OpId, operation: Operation) {
		if let Some(old) = self.operations.remove(&id) {
			self.index.withdraw(id, &old);
		}
		self.index.contribute(id, &operation);
		self.index.store_changed(&operation, false);
		self.operations.insert(id, operation);
	}

	/// Removes a finished operation. Only `MaintainerState::finish` calls this.
	pub(crate) fn remove(&mut self, id: &OpId) -> Option<Operation> {
		let operation = self.operations.remove(id)?;
		self.index.withdraw(*id, &operation);
		self.index.store_changed(&operation, true);
		Some(operation)
	}

	/// Changes an operation in place, keeping the index exact across the change.
	pub(crate) fn update<R>(
		&mut self,
		id: &OpId,
		change: impl FnOnce(&mut Operation) -> R,
	) -> Option<R> {
		let operation = self.operations.get_mut(id)?;
		self.index.withdraw(*id, operation);
		self.index.store_changed(operation, true);
		let result = change(operation);
		self.index.contribute(*id, operation);
		self.index.store_changed(operation, false);
		Some(result)
	}

	/// Removes every operation when the lifetime ends.
	pub(crate) fn take_all(&mut self) -> BTreeMap<OpId, Operation> {
		let store_version = self.index.store_version.wrapping_add(1);
		let session_reads_released = self.index.session_reads_released.wrapping_add(1);
		self.index = OperationIndex {
			store_version,
			session_reads_released,
			..Default::default()
		};
		std::mem::take(&mut self.operations)
	}

	/// Checks the incrementally maintained index against a full recomputation.
	pub(crate) fn validate(&self) -> anyhow::Result<()> {
		let mut expected = OperationIndex::recompute(&self.operations);
		expected.store_version = self.index.store_version;
		expected.session_reads_released = self.index.session_reads_released;
		anyhow::ensure!(
			expected == self.index,
			"operation index drifted from its operations"
		);
		Ok(())
	}
}

impl OperationIndex {
	/// Builds the index from the operations alone.
	pub(crate) fn recompute(operations: &BTreeMap<OpId, Operation>) -> Self {
		let mut index = Self::default();
		for (id, operation) in operations {
			index.contribute(*id, operation);
		}
		index
	}

	/// Records a store change for admission. `released` is whether `operation` stopped
	/// occupying the store rather than began to.
	fn store_changed(&mut self, operation: &Operation, released: bool) {
		match super::store::operation_access(operation) {
			StoreAccess::None => {}
			StoreAccess::SessionRead => {
				if released {
					self.session_reads_released = self.session_reads_released.wrapping_add(1);
				}
			}
			StoreAccess::Exclusive
			| StoreAccess::Keyed
			| StoreAccess::Pass
			| StoreAccess::OrderedRead
			| StoreAccess::Staging
			| StoreAccess::Uploading => self.store_version = self.store_version.wrapping_add(1),
		}
	}

	fn contribute(&mut self, id: OpId, operation: &Operation) {
		self.apply(id, operation, true);
	}

	fn withdraw(&mut self, id: OpId, operation: &Operation) {
		self.apply(id, operation, false);
	}

	/// Adds or withdraws one operation's contribution to every field.
	fn apply(&mut self, id: OpId, operation: &Operation, add: bool) {
		let footprint = super::footprint::operation_footprint(operation);
		if footprint.tablet != TabletAccess::None {
			if add {
				self.store_tablet.insert(id, footprint.tablet);
			} else {
				self.store_tablet.remove(&id);
			}
		}
		for (volume, access) in footprint.volumes {
			if add {
				self.store_volumes
					.entry(volume)
					.or_default()
					.insert(id, access);
			} else if let Some(holders) = self.store_volumes.get_mut(&volume) {
				holders.remove(&id);
				if holders.is_empty() {
					self.store_volumes.remove(&volume);
				}
			}
		}

		if super::footprint::writes_pending_rows(operation) {
			if add {
				self.pending_writers.insert(id);
			} else {
				self.pending_writers.remove(&id);
			}
		}

		let store = &mut self.store;
		match super::store::operation_access(operation) {
			StoreAccess::None => {}
			StoreAccess::Exclusive => step(&mut store.exclusive, add),
			StoreAccess::Keyed => step(&mut store.keyed, add),
			StoreAccess::Pass => step(&mut store.passes, add),
			StoreAccess::OrderedRead => step(&mut store.ordered_reads, add),
			StoreAccess::SessionRead => step(&mut store.session_reads, add),
			StoreAccess::Staging => step(&mut store.staging, add),
			StoreAccess::Uploading => step(&mut store.uploading, add),
		}

		let counts = &mut self.counts;
		match operation {
			Operation::Poll { .. } => count(&mut counts.poll, add),
			Operation::PromotionList { worker, .. } => {
				count(&mut counts.promotion_list, add);
				mark(&mut self.promotion_lists, worker.clone(), id, add);
			}
			Operation::Trim { key, worker, .. } => {
				count(&mut counts.trim, add);
				mark(&mut self.trims, (key.clone(), worker.clone()), id, add);
			}
			Operation::TrimBatch { worker, trims } => {
				for (key, _) in trims {
					count(&mut counts.trim, add);
					mark(&mut self.trims, (key.clone(), worker.clone()), id, add);
				}
			}
			Operation::RecoverVolume { key, .. } => {
				count(&mut counts.recover_volume, add);
				mark(&mut self.repair_holds, key.clone(), id, add);
			}
			Operation::CatchUp { key, learner, step } => {
				count(&mut counts.catch_up, add);
				mark(&mut self.catch_up_learners, learner.clone(), id, add);
				// A load holds the reservation its successful reply hands to the next read,
				// which begins without its own budget check.
				self.read_reservations = adjust(self.read_reservations, add);
				if matches!(step, CopyStep::Load { .. } | CopyStep::WaitingToLoad { .. }) {
					mark(&mut self.catch_up_holds, key.clone(), id, add);
				}
			}
			Operation::ReportCatchUp { .. } => count(&mut counts.report_catch_up, add),
			Operation::DeleteRetired { .. }
			| Operation::ForgetRetiredLog { .. }
			| Operation::ReportRetired { .. } => count(&mut counts.retired, add),
			Operation::ReportParentLogImported => count(&mut counts.report_parent_log, add),
			Operation::ReportVersionMarks { .. } => count(&mut counts.report_version_marks, add),
			Operation::DeleteReplica { key, .. } => {
				count(&mut counts.delete_replica, add);
				mark(&mut self.delete_replicas, key.clone(), id, add);
			}
			Operation::DrainClaim { .. } => count(&mut counts.delete_replica, add),
			Operation::ForgetVolumes {
				worker, volumes, ..
			} => {
				count(&mut counts.forget_volume, add);
				for (key, _) in volumes {
					mark(&mut self.forgets, (key.clone(), worker.clone()), id, add);
				}
			}
			Operation::ForgetOrphan { key, .. } => {
				count(&mut counts.forget_orphan, add);
				mark(&mut self.forget_orphans, key.clone(), id, add);
			}
			Operation::FilesystemRead { key, .. } => {
				count(&mut counts.filesystem_read, add);
				self.read_reservations = adjust(self.read_reservations, add);
				mark(&mut self.filesystem_reads, key.clone(), id, add);
			}
			Operation::FilesystemApply { key, bytes, .. } => {
				let doubled = bytes.saturating_mul(2);
				self.apply_bytes = if add {
					self.apply_bytes.saturating_add(doubled)
				} else {
					self.apply_bytes.saturating_sub(doubled)
				};
				mark(&mut self.filesystem_applies, key.clone(), id, add);
			}
			Operation::FilesystemLoad { key, .. } => {
				mark(&mut self.filesystem_loads, key.clone(), id, add);
			}
			Operation::FilesystemFlush { records, stage, .. } => {
				count(&mut counts.filesystem_flush, add);
				for record in records {
					let key = VolumeLogKey::from(
						rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
							volume_key: record.volume_key(),
							volume_incarnation: record.volume_incarnation,
						},
					);
					if *stage == FlushStage::Uploading {
						mark(&mut self.uploading_records, key.clone(), id, add);
					}
					mark(&mut self.flush_records, key, id, add);
				}
			}
			Operation::WriteToCold { rows, .. } => {
				count(&mut counts.write_to_cold, add);
				for (key, row) in rows {
					if row.status == FsVolumeStatus::Deleted {
						mark(&mut self.deleted_pass_rows, key.clone(), id, add);
					}
					mark(&mut self.pass_rows, key.clone(), id, add);
				}
			}
			Operation::FilesystemQuery {
				request_id,
				order,
				volume_key,
			} => match order {
				QueryOrder::Ordered => {
					if let Some(volume_key) = volume_key {
						let queries = self.ordered_queries.entry(volume_key.clone()).or_default();
						if add {
							queries.insert(id, *request_id);
						} else {
							queries.remove(&id);
							if queries.is_empty() {
								self.ordered_queries.remove(volume_key);
							}
						}
					}
				}
				QueryOrder::HeldImage => count(&mut counts.held_image_query, add),
				QueryOrder::Session | QueryOrder::Claim => {}
			},
			Operation::ReadOpenKeys { .. } | Operation::Repair(_) => {}
			Operation::ReadRow { volume_key } => {
				count(&mut counts.read_row, add);
				mark(&mut self.row_reads, volume_key.clone(), id, add);
				mark(&mut self.read_rows, volume_key.clone(), id, add);
			}
			Operation::ReadUnloadRow { key } | Operation::ReadDeleteRow { key } => {
				mark(&mut self.row_reads, key.0.volume_key.clone(), id, add);
			}
			Operation::PrepareSplit { .. } | Operation::ReportSplit => {
				count(&mut counts.split, add)
			}
			Operation::List { .. } => count(&mut counts.list, add),
			Operation::Load { key, .. } => {
				count(&mut counts.load, add);
				mark(&mut self.loads, key.clone(), id, add);
			}
			Operation::Alarm(attempt) => {
				if add {
					self.alarms.insert(id);
				} else {
					self.alarms.remove(&id);
				}
				if matches!(attempt, super::alarms::Attempt::Queued(_)) {
					if add {
						self.alarm_queued.insert(id);
					} else {
						self.alarm_queued.remove(&id);
					}
				}
				if let Some(key) = attempt.request().key() {
					if matches!(
						attempt,
						super::alarms::Attempt::Queued(_) | super::alarms::Attempt::Waiting(_)
					) {
						if add {
							self.alarm_parked.insert(id);
						} else {
							self.alarm_parked.remove(&id);
						}
					} else {
						count(&mut counts.alarm_keyed, add);
					}
					mark(
						&mut self.alarms_by_volume,
						key.0.volume_key.clone(),
						id,
						add,
					);
				} else {
					if add {
						self.alarm_pages.insert(id);
					} else {
						self.alarm_pages.remove(&id);
					}
				}
			}
			Operation::MaterializeStore { .. }
			| Operation::MaterializePeer { .. }
			| Operation::ForkCleanupStore { .. }
			| Operation::ForkCleanupPeer { .. }
			| Operation::ForkStore { .. }
			| Operation::ForkPeer { .. }
			| Operation::ForkReference { .. }
			| Operation::PendingWork { .. }
			| Operation::SweepPending { .. }
			| Operation::Checkpoint { .. }
			| Operation::SweepDeleted { .. }
			| Operation::OpenCold
			| Operation::MaintainerLog { .. }
			| Operation::ReplayOp { .. } => {}
		}
	}
}

/// The entries of a map keyed by `(VolumeLogKey, WorkerId)` that belong to `key`.
pub(crate) fn workers_of(key: &VolumeLogKey) -> std::ops::RangeInclusive<(VolumeLogKey, WorkerId)> {
	(key.clone(), WorkerId::from_bytes([0; 16]))..=(key.clone(), WorkerId::from_bytes([0xff; 16]))
}

fn mark<K: Ord>(map: &mut ByKey<K>, key: K, id: OpId, add: bool) {
	if add {
		map.entry(key).or_default().insert(id);
	} else if let Some(ids) = map.get_mut(&key) {
		ids.remove(&id);
		if ids.is_empty() {
			map.remove(&key);
		}
	}
}

fn count(value: &mut u32, add: bool) {
	*value = adjust(*value, add);
}

fn adjust(value: u32, add: bool) -> u32 {
	if add {
		value.saturating_add(1)
	} else {
		value.saturating_sub(1)
	}
}

fn step(value: &mut usize, add: bool) {
	*value = if add {
		value.saturating_add(1)
	} else {
		value.saturating_sub(1)
	};
}


