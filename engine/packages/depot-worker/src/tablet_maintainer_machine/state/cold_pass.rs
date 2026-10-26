//! A bounded background pass carries creation rows to the cold store after the disk quorum
//! reply, and stages deletions' rows for the tablet's ordinary passes to make cold.

use super::{MaintainerPhase, MaintainerState, Operation, fence};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord;
use rivet_data::generated::shared_depot_types_v1::VolumeLogKey as WireVolumeLogKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Rows waiting for a cold pass, in queue order, with membership and removal by key in
/// O(log n). A key is queued at most once. Equality, hashing and encoding see only the keys
/// in order, never the positions that order them.
#[derive(Clone, Debug, Default)]
pub(crate) struct PendingRows {
	/// Queue position to key. Positions only grow at the back and shrink at the front.
	order: BTreeMap<i64, VolumeLogKey>,
	positions: BTreeMap<VolumeLogKey, i64>,
}

impl PendingRows {
	pub(crate) fn len(&self) -> usize {
		self.order.len()
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.order.is_empty()
	}

	pub(crate) fn contains(&self, key: &VolumeLogKey) -> bool {
		self.positions.contains_key(key)
	}

	

	

	/// Queues `key` at the back and returns whether it was not already queued.
	pub(crate) fn push_back(&mut self, key: VolumeLogKey) -> bool {
		let position = self.order.last_key_value().map_or(0, |(last, _)| last + 1);
		self.insert(position, key)
	}

	/// Queues `key` at the front and returns whether it was not already queued.
	pub(crate) fn push_front(&mut self, key: VolumeLogKey) -> bool {
		let position = self
			.order
			.first_key_value()
			.map_or(0, |(first, _)| first - 1);
		self.insert(position, key)
	}

	pub(crate) fn pop_front(&mut self) -> Option<VolumeLogKey> {
		let (_, key) = self.order.pop_first()?;
		self.positions.remove(&key);
		Some(key)
	}

	/// Removes `key` wherever it is queued and returns whether it was queued.
	pub(crate) fn remove(&mut self, key: &VolumeLogKey) -> bool {
		let Some(position) = self.positions.remove(key) else {
			return false;
		};
		self.order.remove(&position);
		true
	}

	fn insert(&mut self, position: i64, key: VolumeLogKey) -> bool {
		if self.positions.contains_key(&key) {
			return false;
		}
		self.positions.insert(key.clone(), position);
		self.order.insert(position, key);
		true
	}
}

impl PartialEq for PendingRows {
	fn eq(&self, other: &Self) -> bool {
		self.order.values().eq(other.order.values())
	}
}

impl Eq for PendingRows {}

impl std::hash::Hash for PendingRows {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		for key in self.order.values() {
			key.hash(state);
		}
	}
}

impl Serialize for PendingRows {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_seq(self.order.values())
	}
}

impl<'de> Deserialize<'de> for PendingRows {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let mut rows = Self::default();
		for key in Vec::<VolumeLogKey>::deserialize(deserializer)? {
			rows.push_back(key);
		}
		Ok(rows)
	}
}

/// Arms the age fallback when the first creation row enters an empty queue, so a row
/// waits at most `write_to_cold_after_ms` for a pass. Every push onto `pending_rows`
/// calls this; nothing is armed while the queue is empty.
pub(crate) fn row_queued(state: &MaintainerState, effects: &mut Effects) {
	arm_for_first_row(
		&state.phase,
		&state.pending_rows,
		state.config.write_to_cold_after_ms,
		effects,
	);
}

/// `row_queued` for callers that hold another part of the state mutably.
pub(crate) fn arm_for_first_row(
	phase: &MaintainerPhase,
	pending_rows: &PendingRows,
	write_to_cold_after_ms: u64,
	effects: &mut Effects,
) {
	if *phase == MaintainerPhase::Serving && pending_rows.len() == 1 {
		effects.arm_timer_after(MaintainerTimer::WriteToCold, write_to_cold_after_ms);
	}
}

/// What a pass does with one queued row.
enum Selected {
	/// Work of its volume is in flight or queued ahead of the pass; it keeps its place.
	Deferred,
	/// Nothing for this pass to write; it leaves the queue.
	Dropped,
	Row(depot_types::VolumeKey, FsVolumeRecord),
}

/// Idle passes issue no storage operation; one outstanding pass owns its immutable row set.
///
/// A pass that carries only deletions' Deleted rows uploads nothing: it stages them in the
/// memtable, and the tablet's next ordinary tiering or pressure pass makes them cold.
/// Lifecycle work never writes to cold, and the tombstone quorum already made each delete
/// durable, so only its cleanup waits for that pass. A pass carrying any other row uploads
/// as before, and the deleted rows with it ride the same upload.
///
/// A row whose volume has conflicting work in flight, or work
/// queued ahead of the pass (`ahead`), stays at the front of `pending_rows` in order.
/// Those rows become `pass_deferred`, and the pass is queued again behind them.
pub(crate) fn dispatch(
	state: &mut MaintainerState,
	ahead: &std::collections::BTreeSet<super::footprint::StoreVolume>,
	effects: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if state.pending_rows.is_empty() || state.operations.index().counts.write_to_cold > 0 {
		return;
	}

	// Bound each pass independently of how many volumes were created during a stall.
	let mut rows = Vec::new();
	let mut deferred = Vec::new();
	let mut upload = false;
	while rows.len() < state.config.tuning().max_rows_per_pass as usize {
		let Some(key) = state.pending_rows.pop_front() else {
			break;
		};
		match select(state, &key, ahead, effects) {
			Selected::Deferred => deferred.push(key),
			Selected::Dropped => {}
			Selected::Row(volume_key, row) => {
				if !awaits_deleted_row(state, &key) {
					upload = true;
				}
				rows.push((volume_key, row));
			}
		}
	}
	// Deferred rows keep their place at the front, and the next pass waits for them.
	state.pass_deferred = deferred.iter().cloned().collect();
	for key in deferred.into_iter().rev() {
		state.pending_rows.push_front(key);
	}
	if !state.pass_deferred.is_empty() {
		super::store::queue_cold_pass(state);
	}
	// The age fallback only covers rows still waiting; the pass completion queues
	// the next pass for any that remain.
	if state.pending_rows.is_empty() {
		effects.cancel_timer(MaintainerTimer::WriteToCold);
	}
	if rows.is_empty() {
		return;
	}
	let keys = rows
		.iter()
		.map(|(key, row)| {
			(
				VolumeLogKey(WireVolumeLogKey {
					volume_key: key.clone(),
					volume_incarnation: row.volume_incarnation,
				}),
				row.clone(),
			)
		})
		.collect();
	// Only an uploading pass counts as a pass: a store write staged by a pass that uploads
	// nothing is covered by the next one that does.
	let dispatch = if upload {
		Some(super::maintainer_log::store_pass_dispatched(state))
	} else {
		None
	};
	let op_id = state.begin(
		Operation::WriteToCold {
			rows: keys,
			dispatch,
		},
		effects,
	);
	effects.execute(MaintainerEffect::WriteToColdRequest {
		op_id,
		rows,
		upload,
	});
}

/// Whether `key`'s queued row is the Deleted row of a deletion still waiting for it.
fn awaits_deleted_row(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	matches!(
		state.deleting.get(key),
		Some(super::deletion::Deletion {
			step: super::deletion::DeleteStep::Active(super::deletion::DeleteProgress {
				cold: super::deletion::ColdProgress::WaitingForRow,
				..
			}),
			..
		})
	)
}

/// Decides what a pass does with the queued row of `key`.
fn select(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	ahead: &std::collections::BTreeSet<super::footprint::StoreVolume>,
	effects: &mut Effects,
) -> Selected {
	let volume = super::footprint::StoreVolume::from(&key.0.volume_key);
	if ahead.contains(&volume)
		|| super::footprint::volume_conflict(state, &volume, super::footprint::VolumeAccess::Write)
	{
		return Selected::Deferred;
	}
	// An incarnation whose record the store lost gets no live record from memory; only
	// its delete's tombstone is written. See `filesystem::record_missing`.
	if !state.deleting.contains_key(key) && super::filesystem::is_record_missing(state, key) {
		return Selected::Dropped;
	}
	if !state.deleting.contains_key(key) && super::filesystem::owns(state, key) {
		// The filesystem writes a registered volume's record with its data: the record
		// rides the tablet's next tiering pass.
		if state
			.filesystem
			.volumes
			.get(key)
			.is_some_and(|volume| !volume.record_persisted)
		{
			super::filesystem::request_due_flush(state, key, effects);
		}
		return Selected::Dropped;
	}
	// A deletion's row waits until its drain has claimed and applied the final tail: the
	// row then names that tail, and a live record flushed with the tail cannot land after
	// it. The deletion queues its row again once the drain ends.
	if state.deleting.contains_key(key) && super::drain::active(state, key) {
		return Selected::Dropped;
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return Selected::Dropped;
	};
	let row = if let Some(super::deletion::Deletion {
		step: super::deletion::DeleteStep::Active(progress),
		..
	}) = state.deleting.get(key)
	{
		Some(progress.row.clone())
	} else {
		volume.row(key)
	};
	let Some(mut row) = row else {
		return Selected::Dropped;
	};
	// A deletion admitted during an older live pass keeps that pass's completed floor.
	if let Some(stored) = &volume.stored_row
		&& stored.cold.volume_seq > row.cold.volume_seq
	{
		row.cold = stored.cold.clone();
	}
	Selected::Row(key.0.volume_key.clone(), row)
}

/// A completed uploading pass retires creation metadata only for the incarnations in it. A
/// pass that only staged deleted rows leaves them waiting for an ordinary pass to cover them.
pub(crate) fn completed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<(), String>,
	effects: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::WriteToCold { .. })
	) {
		return;
	}
	let Some(Operation::WriteToCold { rows, dispatch }) = state.finish(op_id, effects) else {
		return;
	};
	let Ok(()) = result else {
		fence(state, effects);
		return;
	};
	match dispatch {
		Some(dispatch) => {
			super::maintainer_log::pass_completed(state, dispatch, effects);
			super::materialize::passes_completed(state);
			for (key, row) in rows {
				row_stored(state, key, row);
			}
		}
		None => {
			// The rows are in the memtable: a pass dispatched from now on makes them cold.
			let through_dispatch = super::maintainer_log::wrote_uncovered(state, effects);
			for (key, row) in rows {
				super::deletion::row_staged(state, &key, row, through_dispatch);
			}
		}
	}
	super::operator_repair::calls::check_persisted(state, effects);
	if !state.pending_rows.is_empty() {
		super::store::queue_cold_pass(state);
	}
}

/// A row this grant wrote is cold: its incarnation's creation metadata retires and the
/// stored row is the one written.
pub(crate) fn row_stored(state: &mut MaintainerState, key: VolumeLogKey, row: FsVolumeRecord) {
	super::deletion::row_written(state, &key, &row);
	state.release_initial_entries(&key);
	if let Some(volume) = state.loaded_volumes.get_mut(&key) {
		volume.creation = None;
		state
			.rows_written
			.insert(key.clone(), (row.cold.clone(), row.status));
		volume.stored_row = Some(row.into());
	}
	super::deletion::observe_loaded(state, &key);
	// A stored row makes a volume whose creation was never acknowledged eligible.
	super::filesystem::offer(state, &key);
}
