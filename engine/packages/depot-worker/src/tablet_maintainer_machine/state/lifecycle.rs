//! One pending create/open per key selects one incarnation and waits for member evidence.

use super::{
	JointColdOpen, LoadedVolume, LoadingVolume, MaintainerPhase, MaintainerState, OpenStep,
	Operation, PendingOpen, fence,
};
use crate::tablet_maintainer_machine::types::CREATION_RETRY_MS;
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::{VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_client_protocol_v1::{self as client, ClientResponse, VolumeRejectResponse},
	depot_storage_worker_protocol_v1::{self as wire, StorageWorkerRequest, StorageWorkerResponse},
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_state_machine::Entropy;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A stale empty replica is trimmed, conditionally forgotten, then loaded at the stored floor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ReplicaOpenStep {
	Load,
	Trim,
	Forget,
	/// Not an open step: the maintainer's reload of a member missing from a loaded
	/// volume, owned by `member_repair`. It shares the load operation so every check of
	/// in-flight loads covers it.
	MissingMember,
}

pub(crate) fn reject(request_id: RequestId, reason: VolumeRejectResponse, effects: &mut Effects) {
	effects.cancel_timer(MaintainerTimer::OpenRequest(request_id));
	effects.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::VolumeRejectResponse(reason),
	});
}

/// Rejects an open as overloaded and counts why, so a load run shows which admission
/// limit or key conflict turned callers away.
fn reject_overloaded(request_id: RequestId, reason: &'static str, effects: &mut Effects) {
	crate::metrics::TABLET_VOLUME_OPEN_REJECTED
		.with_label_values(&[reason])
		.inc();
	reject(request_id, depot_types::client::overloaded(), effects);
}

/// Rejects busy keys rather than run conflicting lifecycle operations concurrently.
pub(crate) fn admit(state: &mut MaintainerState, mut caller: OpenVolume, effects: &mut Effects) {
	// The transport names the requesting worker. A holder naming another worker is not
	// trusted, so the claim this open grants records no holder.
	if caller
		.holder
		.as_ref()
		.is_some_and(|holder| holder.worker_id != *caller.worker_id.as_bytes())
	{
		tracing::warn!(
			worker = ?caller.worker_id,
			"volume open named a holder on another worker; its claim records no holder"
		);
		caller.holder = None;
	}
	let invalid = if caller.deadline_boot_ms <= effects.clocks().boot_ms {
		Some("deadline")
	} else if !depot_fs_core::checkpoints::valid_policy(caller.retention.as_ref())
		|| (caller.mode == OpenMode::Existing && caller.retention.is_some())
	{
		Some("invalid_retention")
	} else {
		None
	};
	if let Some(reason) = invalid {
		reject_overloaded(caller.request_id, reason, effects);
		return;
	}
	// A maintainer log's reserved address is never a client volume.
	if super::maintainer_log::is_log_volume(&caller.volume_key) {
		reject(
			caller.request_id,
			VolumeRejectResponse::VolumeExists,
			effects,
		);
		return;
	}
	// A stale route must refresh its manifest even when this tablet is frozen.
	let hash = partition_key_hash(&caller.volume_key);
	if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) || !state.tablet.hash_range.contains(hash)
	{
		reject(
			caller.request_id,
			depot_types::client::not_maintainer(),
			effects,
		);
		return;
	}
	if state.phase == MaintainerPhase::Frozen {
		reject(
			caller.request_id,
			VolumeRejectResponse::TabletSplitting,
			effects,
		);
		return;
	}
	if caller.mode == OpenMode::Create && !super::stop::may_initialize(state, caller.request_id) {
		reject_overloaded(caller.request_id, "owner_handoff", effects);
		return;
	}
	let busy = if caller.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
	{
		Some("key_too_large")
	} else if super::maintainer_log::replay::pending_for(state, &caller.volume_key) {
		// Recovered maintainer log ops of this volume are not in the store yet.
		Some("log_replay")
	} else if state.request_count() >= state.config.max_requests as usize {
		Some("request_capacity")
	} else if {
		let counts = state.operations.index().counts;
		(counts.read_row as usize + counts.load as usize)
			> (state.config.max_requests as usize)
				.saturating_mul(state.members().len())
				.saturating_sub(state.members().len())
	} {
		Some("row_read_capacity")
	} else if super::forks::holds_key(state, &caller.volume_key) {
		Some("fork_hold")
	} else if state.pending.contains_key(&caller.volume_key) {
		Some("key_opening")
	} else if state
		.operations
		.index()
		.read_rows
		.contains_key(&caller.volume_key)
	{
		Some("key_row_read")
	} else {
		None
	};
	if let Some(reason) = busy {
		reject_overloaded(caller.request_id, reason, effects);
		return;
	}

	if creates(&caller) && deletion_blocks_create(state, &caller.volume_key) {
		if !super::stop::may_initialize(state, caller.request_id) {
			reject_overloaded(caller.request_id, "owner_handoff", effects);
			return;
		}
		await_deletion(state, caller, effects);
		return;
	}
	// A create may replace an incarnation whose delete was acknowledged; the old
	// incarnation's cleanup goes on in the background.
	let recreating = creates(&caller)
		&& matches!(
			super::deletion::recreate(state, &caller.volume_key),
			super::deletion::Recreate::Over
		);
	if recreating && !super::stop::may_initialize(state, caller.request_id) {
		reject_overloaded(caller.request_id, "owner_handoff", effects);
		return;
	}
	if !recreating && super::deletion::holds_key(state, &caller.volume_key) {
		let reason = if super::deletion::lookup_pending(state, &caller.volume_key) {
			depot_types::client::overloaded()
		} else {
			VolumeRejectResponse::VolumeDeleted
		};
		reject(caller.request_id, reason, effects);
		return;
	}
	// An operator repair claim is the name's only writer while it lives, so no other open
	// is granted a claim that would fence it.
	if let Some(reason) = super::operator_repair::claims::under_repair(state, &caller.volume_key) {
		reject(caller.request_id, reason, effects);
		return;
	}
	// The incarnation an open would select has a stored record the store lost, which no
	// maintainer can rebuild; nothing serves the name until that incarnation is deleted, and
	// the caller is told which one. A create goes past it once its delete is admitted.
	if let Some(incarnation) = super::filesystem::record_missing(state, &caller.volume_key) {
		reject(
			caller.request_id,
			depot_types::client::record_missing(incarnation),
			effects,
		);
		return;
	}

	// The volume's apply is far behind its commits and catching up; the opener retries once
	// it has, instead of replaying the whole gap within its own deadline.
	if caller.mode != OpenMode::Create
		&& super::filesystem::open_waits_for_apply(state, &caller.volume_key)
	{
		reject_overloaded(caller.request_id, "apply_behind", effects);
		return;
	}

	let now_boot_ms = state.now_boot_ms;
	for (_, volume) in state
		.loaded_volumes
		.range_mut(super::incarnations(&caller.volume_key))
	{
		volume.last_activity = now_boot_ms;
	}

	// Any deletion evidence wins before incarnation selection. Partial startup evidence
	// cannot choose an incarnation until the remaining member has answered. The newest
	// incarnation comes first.
	let mut candidates: Vec<_> = state
		.loaded_volumes
		.range(super::incarnations(&caller.volume_key))
		.filter(|(key, _)| !super::deletion::is_replaced(state, key))
		.collect();
	candidates
		.sort_by_key(|(_, volume)| std::cmp::Reverse(super::replica_creation::known_epoch(volume)));
	if !recreating
		&& candidates.iter().any(|(key, volume)| {
			volume
				.row(key)
				.is_some_and(|row| row.status == FsVolumeStatus::Deleted)
		}) {
		reject(
			caller.request_id,
			VolumeRejectResponse::VolumeDeleted,
			effects,
		);
		return;
	}
	if !candidates.is_empty()
		&& state.listed.len() < state.members().len()
		&& !candidates.iter().any(|(_, volume)| {
			super::membership::quorum(&state.tablet.replica_set, |w| {
				volume.members.contains_key(w)
			}) || volume.creation.is_some()
				|| volume.unacknowledged_cold_open.is_some()
				|| volume.stored_row.is_some()
		}) {
		reject(
			caller.request_id,
			depot_types::client::overloaded(),
			effects,
		);
		return;
	}
	let mut selected = candidates
		.iter()
		.filter(|_| !recreating)
		.find(|(_, volume)| {
			super::membership::quorum(&state.tablet.replica_set, |w| {
				volume.members.contains_key(w)
			}) || volume.creation.is_some()
				|| volume.unacknowledged_cold_open.is_some()
				|| volume.stored_row.is_some()
		})
		// Recovered observations alone cannot know how far an earlier owner wrote to cold.
		.filter(|(_, volume)| {
			volume.creation.is_some()
				|| volume.stored_row.is_some()
				|| volume.unacknowledged_cold_open.is_some()
		})
		.and_then(|(key, volume)| {
			let row = volume.row(key)?;
			let metadata = volume
				.stored_row
				.as_deref()
				.or_else(|| volume.creation.as_ref().map(|c| &c.row))
				.or(volume.unacknowledged_cold_open.as_deref())?;
			(metadata.cold == row.cold).then_some(row)
		});
	// A voting quorum confirmed no suffix above this stored row before its members
	// were forgotten. That quorum permits a fresh cold open without waiting for
	// offline destinations. Mere absence after a disk wipe is not this proof.
	//
	// Any other unload of the key is cancelled below, however far it got. Safety does not
	// depend on how much it forgot: each forget is conditional at the replica on
	// `appended == cold == row.cold` and fenced by the tablet grant, so a removed member held
	// nothing above the row, and the open reloads it as a fresh `Repairing` replica that cannot
	// vote. The open then needs fresh in-sync answers from a quorum, which meets every quorum
	// that acknowledged an entry above the row. A forget still in flight keeps running detached
	// from the cancelled unload; if its answer shows it removed a member the open counted, the
	// open stops counting that member (see `member_lost`), and member repair reloads it.
	let unloaded = state
		.unloading
		.range(super::incarnations(&caller.volume_key))
		.find_map(|(key, unload)| {
			(super::unload::closed_through(state, key).is_some()
				|| super::membership::quorum(&state.tablet.replica_set, |w| {
					unload.confirmed.contains(w)
				}))
			.then(|| {
				state
					.loaded_volumes
					.get(key)?
					.stored_row
					.as_ref()
					.map(|row| (key.clone(), row.to_record()))
			})
			.flatten()
		});
	if let Some((key, row)) = unloaded {
		state
			.loaded_volumes
			.get_mut(&key)
			.expect("confirmed unload")
			.unacknowledged_cold_open = Some(Box::new(row.clone()));
		selected = Some(row);
	}
	let volume_key = caller.volume_key.clone();
	super::unload::cancel_key(state, &volume_key, effects);
	super::repair::cancel_key(state, &volume_key, effects);
	let waiting = super::repair::holds_key(state, &volume_key);
	let answer_budget_ms = arm_deadline(state, &caller, effects);
	if waiting {
		// The wait ends before the open's answer deadline, so the open preempts the repair
		// and goes on instead of being refused while it waits.
		effects.arm_timer_after(
			MaintainerTimer::RepairWait(caller.request_id),
			state
				.config
				.tuning()
				.repair_open_wait_ms
				.min(answer_budget_ms / 2)
				.max(1),
		);
	}
	state.pending.insert(
		volume_key.clone(),
		PendingOpen {
			caller,
			step: if waiting {
				OpenStep::WaitingForRepair
			} else {
				OpenStep::ReadingRow
			},
		},
	);
	if waiting {
		return;
	}
	if let Some(row) = selected.filter(|_| state.fork_checked.contains(&volume_key)) {
		load(state, volume_key, row, None, effects);
	} else {
		state
			.store_queue
			.push_back(super::store::StoreWork::ReadRow {
				volume_key: volume_key.clone(),
			});
		if state.memtable_budget_shedding {
			state.opens_admitted_while_budget_full.push(volume_key);
		}
	}
}

/// Refuses every pending open whose row lookup waits behind a store write while the memtable
/// budget sheds: that write cannot finish until an upload frees the budget, so the open could
/// only wait out its deadline. Runs when shedding starts.
pub(crate) fn shed_queued_row_reads(state: &mut MaintainerState, fx: &mut Effects) {
	let waiting: Vec<depot_types::VolumeKey> = state
		.pending
		.iter()
		.filter(|(key, pending)| {
			matches!(pending.step, OpenStep::ReadingRow)
				&& lookup_waits_behind_store_write(state, key)
		})
		.map(|(key, _)| key.clone())
		.collect();
	for key in waiting {
		shed_row_read(state, &key, fx);
	}
}

/// Refuses the opens admitted during this event while shedding whose row lookup store
/// admission left waiting behind a store write. Runs after the event's drives.
pub(crate) fn shed_unstarted_row_reads(state: &mut MaintainerState, fx: &mut Effects) {
	for key in std::mem::take(&mut state.opens_admitted_while_budget_full) {
		if lookup_waits_behind_store_write(state, &key) {
			shed_row_read(state, &key, fx);
		}
	}
}

/// Whether the queued row lookup of `key` waits behind store work that writes, which a full
/// memtable budget holds: an in-flight keyed or writing operation of the volume, such as its
/// apply or a flush staging its record, or an apply or load of the volume queued ahead of it.
/// A lookup already in flight, or one waiting only for a read slot, finishes on its own.
fn lookup_waits_behind_store_write(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	let index = state.operations.index();
	if index.read_rows.contains_key(key)
		|| !state
			.store_queue
			.contains(&super::store::StoreWork::ReadRow {
				volume_key: key.clone(),
			}) {
		return false;
	}
	let writer_in_flight = index
		.store_volumes
		.get(&super::footprint::StoreVolume::from(key))
		.is_some_and(|holders| {
			holders.values().any(|access| {
				matches!(
					access,
					super::footprint::VolumeAccess::Keyed | super::footprint::VolumeAccess::Write
				)
			})
		});
	writer_in_flight || state.store_queue.has_keyed_work(key)
}

/// Refuses the pending open of `key` as overloaded with a retry hint and drops its queued row
/// lookup. The lookup never ran, so the open loaded nothing and leaves no orphan; its
/// admission cancelled any unload of the key, so the key is marked for the unload drive as
/// after an expired open.
fn shed_row_read(state: &mut MaintainerState, key: &depot_types::VolumeKey, fx: &mut Effects) {
	let Some(pending) = state.pending.remove(key) else {
		return;
	};
	state
		.store_queue
		.remove_all(&[super::store::StoreWork::ReadRow {
			volume_key: key.clone(),
		}]);
	super::unload::mark_scope(state, super::unload::UnloadScope::Keys(vec![key.clone()]));
	reject_overloaded(pending.caller.request_id, "memtable_full", fx);
}

/// Resume an admitted reopen only after all recovery work for its key has joined.
pub(crate) fn resume_after_repair(
	state: &mut MaintainerState,
	key: &depot_types::VolumeKey,
	effects: &mut Effects,
) {
	if state.phase != MaintainerPhase::Serving || super::repair::holds_key(state, key) {
		return;
	}
	if state
		.pending
		.get(key)
		.is_some_and(|pending| matches!(pending.step, OpenStep::WaitingForRepair))
	{
		let pending = state.pending.remove(key).expect("waiting caller");
		effects.cancel_timer(MaintainerTimer::RepairWait(pending.caller.request_id));
		// Re-evaluate deletion, incarnation, and cold after the recovery completion.
		admit(state, pending.caller, effects);
	}
}

/// The parked reopen `id` has waited long enough for a required learner's recovery. Cancel
/// the key's recovery jobs; the open resumes when they join, and the serving client matches
/// the learner while it serves.
pub(crate) fn repair_wait_expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(key) = state.open_deadlines.keys.get(&id).cloned() else {
		return;
	};
	let still_waiting = state.pending.get(&key).is_some_and(|pending| {
		pending.caller.request_id == id && matches!(pending.step, OpenStep::WaitingForRepair)
	});
	if !still_waiting {
		return;
	}
	super::repair::preempt_key(state, &key, fx);
}

/// Whether `caller` may create the volume when it is absent.
fn creates(caller: &OpenVolume) -> bool {
	match caller.mode {
		OpenMode::Create | OpenMode::CreateIfAbsent => true,
		OpenMode::Existing => false,
	}
}

/// Whether a deletion of `key`'s current incarnation is still running or yet to start: a
/// deletion has not reached its tombstone quorum, or a loaded incarnation's row is a tombstone
/// the deletion drive has not taken up. Either way the delete was not acknowledged, so a
/// create cannot draw a newer incarnation yet.
fn deletion_blocks_create(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	matches!(
		super::deletion::recreate(state, key),
		super::deletion::Recreate::Blocked
	)
}

/// Whether an unload of the key has started forgetting its replicas. A checkpoint or a fork
/// reference must then wait for the unload to finish: its cold row or the remaining replicas
/// decide what it sees. An open never waits: it cancels the unload and reloads the forgotten
/// members itself.
pub(crate) fn unload_blocks_open(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> bool {
	state
		.unloading
		.range(super::incarnations(volume_key))
		.any(|(key, unload)| {
			unload.remaining.len() < state.members().len()
				|| !unload.waiting.is_empty()
				|| state
					.operations
					.index()
					.forgets
					.range(super::operations::workers_of(key))
					.next()
					.is_some()
		})
}

/// Parks a creating open until the deletion of its key's current incarnation releases the key,
/// under the open's own deadline. A second waiting create of the key is turned away.
fn await_deletion(state: &mut MaintainerState, caller: OpenVolume, effects: &mut Effects) {
	if state.awaiting_deletion.contains_key(&caller.volume_key) {
		reject_overloaded(caller.request_id, "key_awaiting_deletion", effects);
		return;
	}
	arm_deadline(state, &caller, effects);
	state
		.awaiting_deletion
		.insert(caller.volume_key.clone(), caller);
}

/// Admits again every waiting create whose key no deletion blocks any more. It then reads the
/// key's record, finds it released and draws a new incarnation. Runs after every deletion
/// transition.
pub(crate) fn resume_after_deletion(state: &mut MaintainerState, effects: &mut Effects) {
	if state.awaiting_deletion.is_empty() || state.phase != MaintainerPhase::Serving {
		return;
	}
	let released = state
		.awaiting_deletion
		.keys()
		.filter(|key| !deletion_blocks_create(state, key))
		.cloned()
		.collect::<Vec<_>>();
	for key in released {
		let caller = state
			.awaiting_deletion
			.remove(&key)
			.expect("released waiter");
		admit(state, caller, effects);
	}
}

/// Draws a random incarnation only for an absent row under the key's sole pending create.
pub(crate) fn row(
	state: &mut MaintainerState,
	key: depot_types::VolumeKey,
	result: Result<depot_fs_core::forks::LifecycleLookup, String>,
	entropy: Entropy,
	effects: &mut Effects,
) {
	if !state.pending.contains_key(&key) {
		return;
	}
	let Some(result) = super::fork_admission::lookup(state, &key, result) else {
		let pending = state.pending.remove(&key).expect("current caller");
		reject(
			pending.caller.request_id,
			depot_types::client::overloaded(),
			effects,
		);
		return;
	};
	let pending = state.pending.get(&key).expect("current caller");
	// A create over an acknowledged delete goes past the deleted incarnation, and a stored
	// row naming an incarnation a newer one replaced is obsolete: neither is the row.
	let recreating = creates(&pending.caller)
		&& matches!(
			super::deletion::recreate(state, &key),
			super::deletion::Recreate::Over
		);
	let obsolete = |incarnation: [u8; 16]| {
		let candidate = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: key.clone(),
			volume_incarnation: incarnation,
		});
		(recreating && state.deleting.contains_key(&candidate))
			|| super::deletion::is_replaced(state, &candidate)
	};
	// A new incarnation is created above every incarnation of the volume key this maintainer
	// knows, the stored one included. The lookup leaves out a stored incarnation of another
	// volume key sharing the name; the tablet's claim epoch counter, which never issues an
	// epoch twice or below an earlier one, orders the new incarnation above it.
	let epoch_floor = result
		.as_ref()
		.ok()
		.and_then(|row| row.as_ref())
		.map_or(0, |row| row.incarnation_epoch)
		.max(super::replica_creation::highest_epoch(state, &key));
	let result = match result {
		Ok(Some(row)) if obsolete(row.volume_incarnation) => Ok(None),
		other => other,
	};
	let observed = state
		.loaded_volumes
		.range(super::incarnations(&key))
		.filter(|(candidate, _)| !obsolete(candidate.0.volume_incarnation))
		.filter_map(|(candidate, volume)| volume.row(candidate))
		.collect::<Vec<_>>();
	let fallback = (observed.len() == 1).then(|| observed[0].clone());
	if matches!(result, Ok(None)) && observed.len() > 1 {
		let pending = state.pending.remove(&key).expect("current caller");
		reject(
			pending.caller.request_id,
			depot_types::client::overloaded(),
			effects,
		);
		return;
	}
	// The fallback incarnation is known only from its replicas. With a cold position above
	// zero its record was written, and the pointer lookup that found none cannot say whether
	// its own row survived: opening it on the observed row would load replicas at a position
	// the store may not hold and queue that row as its record. Its own-record lookup decides
	// first: it registers the incarnation, whose stored row the next open then uses, or
	// quarantines it, which the next open answers as `VolumeRecordMissing`.
	if matches!(result, Ok(None))
		&& let Some(row) = &fallback
	{
		let fallback_key =
			VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: row.volume_incarnation,
			});
		if super::filesystem::is_record_missing(state, &fallback_key) {
			let pending = state.pending.remove(&key).expect("current caller");
			reject(
				pending.caller.request_id,
				depot_types::client::record_missing(row.volume_incarnation),
				effects,
			);
			return;
		}
		if super::filesystem::record_lost(state, &fallback_key) {
			super::filesystem::offer(state, &fallback_key);
			let pending = state.pending.remove(&key).expect("current caller");
			reject_overloaded(pending.caller.request_id, "record_lookup", effects);
			return;
		}
	}
	let created = matches!(result, Ok(None)) && fallback.is_none();
	let stored = result.as_ref().ok().and_then(|row| row.clone());
	let row = match result {
		Ok(None) if fallback.is_some() => fallback.expect("observed volume"),
		Ok(Some(row)) => row,
		Ok(None) => {
			if pending.caller.mode == OpenMode::Existing {
				let pending = state.pending.remove(&key).expect("current caller");
				reject(
					pending.caller.request_id,
					VolumeRejectResponse::VolumeNotFound,
					effects,
				);
				return;
			};
			// A create-if-absent may read an existing row after quiescence, but
			// cannot draw a new incarnation after the old owner promised Ready.
			if !super::stop::may_initialize(state, pending.caller.request_id) {
				let pending = state.pending.remove(&key).expect("current caller");
				reject_overloaded(pending.caller.request_id, "owner_handoff", effects);
				return;
			}
			// The ordering epoch is drawn with the creation's first claim; see `load`.
			FsVolumeRecord {
				incarnation_epoch: 0,
				recent_writes: Vec::new(),
				base_chain: Vec::new(),
				retention: pending.caller.retention.clone(),
				partition_key: key.partition_key.clone(),
				name: key.name.clone(),
				volume_incarnation: *entropy.rng().uuid_v4().as_bytes(),
				status: FsVolumeStatus::Live,
				cold: VolumeLogPosition {
					client_epoch: 0,
					volume_seq: 0,
				},
			}
		}
		Err(_) => {
			fence(state, effects);
			return;
		}
	};
	if row.status == FsVolumeStatus::Deleted {
		super::deletion::observe_row(state, key.clone(), row);
		let pending = state.pending.remove(&key).expect("current caller");
		if creates(&pending.caller) {
			await_deletion(state, pending.caller, effects);
			return;
		}
		reject(
			pending.caller.request_id,
			VolumeRejectResponse::VolumeDeleted,
			effects,
		);
		return;
	}
	let log_key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: key.clone(),
		volume_incarnation: row.volume_incarnation,
	});
	let mut row = row;
	if let Some(volume) = state.loaded_volumes.get_mut(&log_key) {
		if let Some(stored) = stored {
			volume.stored_row = Some(stored.into());
		}
		if let Some(observed) = volume.row(&log_key)
			&& observed.cold.volume_seq > row.cold.volume_seq
		{
			row.cold = observed.cold;
		}
	}
	super::deletion::observe_loaded(state, &log_key);
	// A stored row makes an already loaded volume eligible for filesystem apply without an
	// in-sync quorum, so register it now rather than at its next member observation.
	super::filesystem::offer(state, &log_key);
	load(state, key, row, created.then_some(epoch_floor), effects);
}

/// The entries of an opener's initial writes, at the sequences right after `cold` and under the
/// open claim's epoch.
fn initial_entries(
	cold: &VolumeLogPosition,
	client_epoch: u64,
	initial: &[rivet_data::generated::depot_log_shard_v1::PayloadWrite],
	now_unix_ms: i64,
) -> Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry> {
	initial
		.iter()
		.zip(cold.volume_seq + 1..)
		.map(
			|(write, volume_seq)| rivet_data::generated::depot_log_shard_v1::VolumeLogEntry {
				written_at_unix_ms: now_unix_ms,
				volume_seq,
				client_epoch,
				body: rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(
					write.clone(),
				),
			},
		)
		.collect()
}

/// Picks the open claim and initial entries for an open of an unacknowledged creation, and
/// records the grant. `epoch` is the claim the observed members allow and `highest` the
/// highest observed claim epoch.
///
/// The newest grant is repeated, with its exact entries, only for the same client sending the
/// same initial writes while no member was claimed above it. A first open gets its initial
/// writes as entries when they fit the budgets. Any other open is claimed above every epoch this
/// creation granted and gets no entries, so its writes follow its own claim entry.
fn grant_creating_open(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	volume_key: &depot_types::VolumeKey,
	incarnation: [u8; 16],
	epoch: u64,
	highest: u64,
	effects: &mut Effects,
) -> (
	Option<ClientIdentity>,
	Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
) {
	let caller_initial = &state.pending[volume_key].caller.initial;
	let granted = state.loaded_volumes[key]
		.creation
		.as_ref()
		.and_then(|creation| creation.granted.clone());
	let repeats = |granted: &super::GrantedOpen| {
		granted.client.client_incarnation == incarnation
			&& granted.client.client_epoch >= highest
			&& granted.entries_sent == caller_initial.len() as u64
			&& granted.entries.len() == caller_initial.len()
			&& granted
				.entries
				.iter()
				.zip(caller_initial)
				.all(|(entry, write)| {
					matches!(&entry.body, rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(sent) if sent == write)
				})
	};
	match granted {
		Some(granted) if repeats(&granted) => (Some(granted.client), granted.entries),
		// The caller issued `epoch` above every epoch this creation granted, so it is
		// already higher than the previous grant.
		Some(_) => {
			let client = ClientIdentity {
				client_epoch: epoch,
				client_incarnation: incarnation,
			};
			if !caller_initial.is_empty() {
				state.initial_writes_withheld += 1;
			}
			record_grant(state, key, client.clone(), Vec::new());
			(Some(client), Vec::new())
		}
		None => {
			let client = ClientIdentity {
				client_epoch: epoch,
				client_incarnation: incarnation,
			};
			let cold = state.loaded_volumes[key]
				.creation
				.as_ref()
				.expect("unacknowledged creation")
				.row
				.cold
				.clone();
			let entries = initial_entries(&cold, epoch, caller_initial, effects.clocks().unix_ms);
			let bytes = super::initial_entry_bytes(&entries);
			let entries = if bytes <= state.config.tuning().max_initial_bytes
				&& state.initial_bytes_held.saturating_add(bytes)
					<= state.config.tuning().max_pending_initial_bytes
			{
				entries
			} else {
				state.initial_writes_withheld += 1;
				Vec::new()
			};
			record_grant(state, key, client.clone(), entries.clone());
			(Some(client), entries)
		}
	}
}

/// Records `client` as the creation's newest grant with the entries sent under it, moving
/// the held byte count from the previous grant's entries to these.
fn record_grant(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	client: ClientIdentity,
	entries: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
) {
	state.release_initial_entries(key);
	state.initial_bytes_held = state
		.initial_bytes_held
		.saturating_add(super::initial_entry_bytes(&entries));
	if let Some(creation) = state
		.loaded_volumes
		.get_mut(key)
		.and_then(|volume| volume.creation.as_mut())
	{
		let entries_sent = entries.len() as u64;
		creation.granted = Some(super::GrantedOpen {
			client,
			entries,
			entries_sent,
		});
	}
}

/// Keeps one immutable row and incarnation across all members of the load attempt.
///
/// `creation_floor` is set for a create that drew a new incarnation: the highest ordering epoch
/// known for the name. The new incarnation's ordering epoch is its first claim's epoch, drawn
/// above that floor from this lifetime's grant band, so it is higher than every epoch issued
/// before, including every older incarnation's.
fn load(
	state: &mut MaintainerState,
	volume_key: depot_types::VolumeKey,
	mut row: FsVolumeRecord,
	creation_floor: Option<u64>,
	effects: &mut Effects,
) {
	let created = creation_floor.is_some();
	let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: volume_key.clone(),
		volume_incarnation: row.volume_incarnation,
	});
	if !state.loaded_volumes.contains_key(&key)
		&& state.reserves_at_least(state.config.max_loaded_volumes as usize)
	{
		let pending = state.pending.remove(&volume_key).expect("current caller");
		reject(
			pending.caller.request_id,
			depot_types::client::overloaded(),
			effects,
		);
		return;
	}
	let inserted = !state.loaded_volumes.contains_key(&key);
	state
		.loaded_volumes
		.entry(key.clone())
		.or_insert_with(|| LoadedVolume {
			members: Default::default(),
			last_activity: state.now_boot_ms,
			creation: created.then(|| {
				Box::new(super::Creation {
					row: row.clone(),
					acknowledged: false,
					granted: None,
				})
			}),
			// Serving requires a quorum of intact disks. With no observed replica,
			// their completed listings establish an unloaded volume at the stored floor.
			unacknowledged_cold_open: (!created).then(|| Box::new(row.clone())),
			stored_row: None,
			status: Default::default(),
		});
	super::track_opened(state, &key);

	// A loaded volume may have a newer acknowledged suffix than this maintainer
	// observed. An absent replica cannot vote just because an old listing saw it.
	// Initial create/cold-open retries are different: no client has been admitted yet.
	let volume = &state.loaded_volumes[&key];
	let status = if volume.unacknowledged_cold_open.is_some()
		|| volume
			.creation
			.as_ref()
			.is_some_and(|creation| !creation.acknowledged)
	{
		VolumeLogReplicaStatus::InSync
	} else {
		VolumeLogReplicaStatus::Repairing
	};
	let unacknowledged_create = volume
		.creation
		.as_ref()
		.is_some_and(|creation| !creation.acknowledged);
	// Every open claims its volume at an epoch this maintainer issues, including an open of a
	// volume another writer still holds: the scheduler that decides who should write is
	// assumed to possibly have a bug, so exclusivity is enforced here and at the replicas,
	// never trusted to it. The loads apply the claim when the replica set is stable; otherwise
	// the opener claims the members itself at the same granted epoch.
	//
	// A create claims only the volume it creates. A create of a volume that already exists
	// is not an open, so it must not fence the volume's current writer.
	let claims = match state.pending[&volume_key].caller.mode {
		OpenMode::Create => unacknowledged_create,
		OpenMode::Existing | OpenMode::CreateIfAbsent => true,
	};
	// Each claim gets a fresh epoch, above every epoch the members hold and every epoch this
	// creation already granted, a retry by the same client included. A reused epoch could be
	// one a newer grant retired, with entries under it on some members only.
	let claim = match (claims, state.pending[&volume_key].caller.client_incarnation) {
		(true, Some(incarnation)) => {
			let granted = volume
				.creation
				.as_ref()
				.and_then(|creation| creation.granted.as_ref())
				.map_or(0, |granted| granted.client.client_epoch);
			let highest = volume
				.members
				.values()
				.map(super::maintainer_log::epoch_floor)
				.fold(row.cold.client_epoch.max(granted), u64::max);
			Some((incarnation, highest))
		}
		(false, _) | (true, None) => None,
	};
	// A new claim takes its epoch from this lifetime's grant, so no lifetime of this tablet
	// hands one epoch to two clients. When none can be issued, the open is turned away for a
	// retry. A new incarnation's ordering epoch is its first claim's, or a dedicated one when
	// the create claims nothing; both are drawn above every epoch known for the name.
	let ordering_floor = creation_floor.filter(|_| inserted);
	let claim = match claim {
		Some((incarnation, highest)) => {
			let Some(epoch) = highest
				.max(ordering_floor.unwrap_or(0))
				.checked_add(1)
				.and_then(|min| super::claim_epochs::issue(state, min))
			else {
				if inserted {
					state.remove_loaded_volume(&key);
				}
				let pending = state.pending.remove(&volume_key).expect("current caller");
				reject_overloaded(pending.caller.request_id, "claim_epoch", effects);
				return;
			};
			Some((incarnation, epoch, highest))
		}
		None => None,
	};
	if let Some(floor) = ordering_floor {
		let incarnation_epoch = match &claim {
			Some((_, epoch, _)) => Some(*epoch),
			None => floor
				.checked_add(1)
				.and_then(|min| super::claim_epochs::issue(state, min)),
		};
		let Some(incarnation_epoch) = incarnation_epoch else {
			state.remove_loaded_volume(&key);
			let pending = state.pending.remove(&volume_key).expect("current caller");
			reject_overloaded(pending.caller.request_id, "claim_epoch", effects);
			return;
		};
		row.incarnation_epoch = incarnation_epoch;
		if let Some(creation) = state
			.loaded_volumes
			.get_mut(&key)
			.and_then(|volume| volume.creation.as_mut())
		{
			creation.row.incarnation_epoch = incarnation_epoch;
		}
	}
	// A creation grants each claim to one client and one set of initial entries; see
	// `GrantedOpen`. Only an exact retry of the newest grant reuses its claim and entries.
	let (open_claim, initial_entries) = match (claim, unacknowledged_create) {
		(Some((incarnation, epoch, highest)), true) => grant_creating_open(
			state,
			&key,
			&volume_key,
			incarnation,
			epoch,
			highest,
			effects,
		),
		(Some((incarnation, epoch, _)), false) => (
			Some(ClientIdentity {
				client_epoch: epoch,
				client_incarnation: incarnation,
			}),
			Vec::new(),
		),
		(None, _) => (None, Vec::new()),
	};
	let holder = state.pending[&volume_key].caller.holder.clone();
	let open_claim =
		open_claim.map(
			|client| rivet_data::generated::shared_depot_types_v1::OpenClaim {
				client,
				tablet_replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
				holder,
			},
		);
	let initial_through = initial_entries.last().map(|entry| VolumeLogPosition {
		volume_seq: entry.volume_seq,
		client_epoch: entry.client_epoch,
	});
	let joint_cold_open = joint_cold_open(state).then(|| {
		if super::promotion::observed(state) {
			JointColdOpen::Observed(state.tablet.clone())
		} else {
			JointColdOpen::AwaitingBarrier
		}
	});
	let joint_claim = joint_cold_open.is_some();
	state
		.pending
		.get_mut(&volume_key)
		.expect("current caller")
		.step = OpenStep::Loading(LoadingVolume {
		joint_cold_open,
		initial_through,
		open_claim: open_claim.clone(),
		states: BTreeMap::new(),
		unacknowledged_create,
		key: key.clone(),
		row: row.clone(),
		succeeded: BTreeSet::new(),
		operations: BTreeSet::new(),
		read_wait: false,
		promoted: None,
		lost: BTreeMap::new(),
	});
	if created {
		effects.arm_timer_after(
			MaintainerTimer::CreationExpired(key.clone()),
			CREATION_RETRY_MS,
		);
	}
	// The loads below carry the creation and its initial entries, which a quorum may apply
	// after an early refusal, so a creating open is answered only at the caller's own
	// deadline: a create the caller hears nothing about fails with an unknown outcome, never
	// as refused.
	if unacknowledged_create {
		let caller = &state.pending[&volume_key].caller;
		effects.arm_timer_at(
			MaintainerTimer::OpenRequest(caller.request_id),
			caller.deadline_boot_ms,
		);
	}
	let creation =
		state.loaded_volumes[&key].creation_evidence(state.config.write_versions.maintainer_log);
	for worker in state.members() {
		let mut load = wire::LoadVolumeRequest {
			creation: creation.clone(),
			open_claim: open_claim.clone(),
			learner_replica_set_version: None,
			disk_incarnation: state
				.disk_incarnation(&worker)
				.expect("current load member"),
			claimed_client: None,
			claimed_holder: None,
			header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0.clone(),
					tablet_grant: state.grant(),
				},
			),
			cold: row.cold.clone(),
			status: status.clone(),
			initial_entries: initial_entries.clone(),
		};
		prepare_joint_cold_load(joint_claim, &mut load, None);
		prepare_learner_load(state, &key, &worker, &mut load, None);
		let request = StorageWorkerRequest::LoadVolumeRequest(load);
		let op_id = state.begin(
			Operation::Load {
				step: ReplicaOpenStep::Load,
				worker: worker.clone(),
				key: key.clone(),
			},
			effects,
		);
		let OpenStep::Loading(loading) = &mut state
			.pending
			.get_mut(&volume_key)
			.expect("current caller")
			.step
		else {
			unreachable!()
		};
		loading.operations.insert(op_id);
		effects.execute(MaintainerEffect::ReplicaRequest {
			op_id,
			worker_id: worker,
			request,
		});
	}
}

/// Ordinary opens must not recreate a Learning-only destination at the previous
/// Stable generation. It remains nonvoting until the opener matches it; old voters
/// retain their ordinary claim and initial-write optimization.
fn prepare_learner_load(
	state: &MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	load: &mut wire::LoadVolumeRequest,
	forgotten: Option<&rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState>,
) {
	if let rivet_data::generated::shared_depot_types_v1::TabletPhase::Learning(next) =
		&state.tablet.replica_set.phase
		&& next.next.contains(worker.as_bytes())
		&& !state.tablet.replica_set.members.contains(worker.as_bytes())
	{
		load.learner_replica_set_version =
			Some(state.tablet.replica_set.tablet_replica_set_version);
		load.open_claim = None;
		// A forget reply may carry a fence newer than the cached member it removed.
		let copied = state
			.loaded_volumes
			.get(key)
			.and_then(super::LoadedVolume::trim_claim);
		let newest = copied
			.iter()
			.chain(load.claimed_client.iter())
			.chain(forgotten.and_then(|r| r.claimed_client.as_ref()))
			.max_by_key(|c| c.client_epoch)
			.cloned();
		load.claimed_client = newest;
		load.claimed_holder = None;
		load.status = VolumeLogReplicaStatus::Repairing;
		load.initial_entries.clear();
	}
}

/// Only current attempt responses count toward the client quorum; late member
/// responses still improve observations without issuing a second client reply.
pub(crate) fn loaded(
	state: &mut MaintainerState,
	op_id: OpId,
	worker: WorkerId,
	key: VolumeLogKey,
	step: ReplicaOpenStep,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	if state.disk_incarnation(&worker).is_none() {
		return;
	}
	// A replica that a cancelled close's late forget removed no longer exists, so its answer
	// to a load or trim of this attempt is no evidence. A forget step's answer describes the
	// replica it removed and keeps its meaning.
	let result = match result {
		Ok(StorageWorkerResponse::ReplicaStateResponse(response))
			if step != ReplicaOpenStep::Forget
				&& answers_lost_replica(state, &key, &worker, &response.state) =>
		{
			Err("the answering replica was forgotten by a cancelled close".to_string())
		}
		other => other,
	};
	// Complete only the current attempt's reconciliation. A canceled open cannot
	// turn a late trim or forget response into new lifecycle work.
	let loading = state
		.pending
		.get(&key.0.volume_key)
		.and_then(|pending| match &pending.step {
			OpenStep::Loading(loading)
				if loading.key == key && loading.operations.contains(&op_id) =>
			{
				Some(loading)
			}
			_ => None,
		});
	if let Some(loading) = loading {
		let row = loading.row.clone();
		let open_claim = loading.open_claim.clone();
		let joint_claim = loading.joint_cold_open.is_some();
		let returned = match &result {
			Ok(StorageWorkerResponse::ReplicaStateResponse(r)) => Some(&r.state),
			_ => None,
		};
		// A member reloaded in sync by `promote_caught_up` keeps the newest fence seen: the
		// one its answers showed, or a newer claim the forgotten replica gained since.
		let promoted_claim = loading
			.promoted
			.as_ref()
			.filter(|promoted| {
				step == ReplicaOpenStep::Forget && promoted.workers.contains(&worker)
			})
			.map(|promoted| {
				let epoch = |client: &Option<ClientIdentity>| {
					client.as_ref().map(|client| client.client_epoch)
				};
				let newer = returned.filter(|forgotten| {
					epoch(&forgotten.claimed_client) >= epoch(&promoted.claimed_client)
				});
				match newer {
					Some(forgotten) => (
						forgotten.claimed_client.clone(),
						forgotten.claimed_holder.clone(),
					),
					None => (
						promoted.claimed_client.clone(),
						promoted.claimed_holder.clone(),
					),
				}
			});
		let next = match (&step, returned) {
			(ReplicaOpenStep::Load, Some(replica))
				if replica.status != VolumeLogReplicaStatus::Deleted
					&& replica.appended.volume_seq < row.cold.volume_seq =>
			{
				Some(ReplicaOpenStep::Trim)
			}
			// A replica emptied by corruption recovery keeps its record, and with it the
			// mark that it may lack acknowledged entries, until a client repairs it.
			(ReplicaOpenStep::Trim, Some(replica))
				if replica.appended == row.cold
					&& replica.cold == row.cold
					&& !replica.lost_entries =>
			{
				Some(ReplicaOpenStep::Forget)
			}
			(ReplicaOpenStep::Forget, Some(_)) => Some(ReplicaOpenStep::Load),
			(ReplicaOpenStep::Forget, None)
				if matches!(
					&result,
					Ok(StorageWorkerResponse::RejectResponse(
						wire::RejectResponse::VolumeNotLoaded
					))
				) =>
			{
				Some(ReplicaOpenStep::Load)
			}
			_ => None,
		};
		if let Some(next) = next {
			if let Some(volume) = state.loaded_volumes.get_mut(&key) {
				if step == ReplicaOpenStep::Forget {
					volume.members.remove(&worker);
				} else if let Some(replica) = returned {
					volume.members.insert(worker.clone(), replica.clone());
				}
			}
			super::deletion::observe_loaded(state, &key);
			super::filesystem::member_observed(state, &key);
			let header = wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0.clone(),
					tablet_grant: state.grant(),
				},
			);
			let request = match next {
				ReplicaOpenStep::Trim => StorageWorkerRequest::TrimRequest(wire::TrimRequest {
					claimed_client: state
						.loaded_volumes
						.get(&key)
						.and_then(super::LoadedVolume::trim_claim),
					header,
					cold: row.cold.clone(),
				}),
				// The forget expects the claim the trim just answered with, so a claim
				// installed after that answer keeps the replica.
				ReplicaOpenStep::Forget => {
					StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
						observation: None,
						header,
						expected_appended: Some(row.cold.clone()),
						expected_claim: returned.and_then(|replica| replica.claimed_client.clone()),
					})
				}
				ReplicaOpenStep::Load => {
					let mut load = wire::LoadVolumeRequest {
						creation: state.loaded_volumes.get(&key).and_then(|volume| {
							volume.creation_evidence(state.config.write_versions.maintainer_log)
						}),
						// A promoted member installs its selected fence atomically with
						// InSync. That fence already includes a Joint opener's issued claim;
						// a load cannot combine a copied fence with an open claim.
						open_claim: match &promoted_claim {
							Some((Some(_), _)) => None,
							Some((None, _)) | None => open_claim,
						},
						learner_replica_set_version: None,
						disk_incarnation: state
							.disk_incarnation(&worker)
							.expect("current load member"),
						header,
						cold: row.cold.clone(),
						claimed_client: promoted_claim
							.as_ref()
							.and_then(|(client, _)| client.clone()),
						claimed_holder: promoted_claim.and_then(|(_, holder)| holder),
						status: if state.loaded_volumes[&key]
							.members
							.values()
							.any(|r| r.appended.volume_seq > row.cold.volume_seq)
						{
							VolumeLogReplicaStatus::Repairing
						} else {
							VolumeLogReplicaStatus::InSync
						},
						initial_entries: Vec::new(),
					};
					prepare_joint_cold_load(joint_claim, &mut load, returned);
					prepare_learner_load(state, &key, &worker, &mut load, returned);
					StorageWorkerRequest::LoadVolumeRequest(load)
				}
				// `next` above is only ever an open's own step.
				ReplicaOpenStep::MissingMember => {
					unreachable!("an open never advances to a member repair load")
				}
			};
			let next_id = state.begin(
				Operation::Load {
					worker: worker.clone(),
					key: key.clone(),
					step: next,
				},
				effects,
			);
			let OpenStep::Loading(loading) = &mut state
				.pending
				.get_mut(&key.0.volume_key)
				.expect("current caller")
				.step
			else {
				unreachable!()
			};
			loading.operations.remove(&op_id);
			loading.operations.insert(next_id);
			effects.execute(MaintainerEffect::ReplicaRequest {
				op_id: next_id,
				worker_id: worker,
				request,
			});
			return;
		}
	}
	let floor = loading.map(|loading| loading.row.cold.volume_seq);
	// A completed forget remains absence even after the opener stopped waiting.
	if step == ReplicaOpenStep::Forget {
		if matches!(
			&result,
			Ok(StorageWorkerResponse::ReplicaStateResponse(_)
				| StorageWorkerResponse::RejectResponse(wire::RejectResponse::VolumeNotLoaded))
		) && let Some(volume) = state.loaded_volumes.get_mut(&key)
		{
			volume.members.remove(&worker);
		}
		super::catch_up::mark(state, &key);
		if floor.is_none() {
			return;
		}
	}

	// A replica retaining the floor and its newer suffix can serve immediately;
	// background trims may lag without making the open wait for their delivery.
	let mut succeeded = false;
	let mut returned_state = None;
	let mut deleted = false;
	if let Ok(StorageWorkerResponse::ReplicaStateResponse(response)) = result {
		deleted = response.state.status == VolumeLogReplicaStatus::Deleted;
		succeeded = response.state.status == VolumeLogReplicaStatus::InSync
			&& floor.is_none_or(|floor| response.state.appended.volume_seq >= floor);
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			returned_state = Some(response.state.clone());
			// A reply that arrives after the open stopped waiting may be older than a
			// notice already observed from the same replica; it never rolls that back.
			let newer_observed = floor.is_none()
				&& volume.members.get(&worker).is_some_and(|old| {
					old.replica_incarnation == response.state.replica_incarnation
						&& old.appended.volume_seq > response.state.appended.volume_seq
				});
			if !newer_observed {
				volume.members.insert(worker.clone(), response.state);
			}
		}
		super::deletion::observe_loaded(state, &key);
		// The reply can complete the in-sync quorum that makes the volume eligible for
		// filesystem apply, even when the open that started it no longer waits.
		super::filesystem::member_observed(state, &key);
	}
	let Some(pending) = state.pending.get_mut(&key.0.volume_key) else {
		return;
	};
	let OpenStep::Loading(loading) = &mut pending.step else {
		return;
	};
	if loading.key != key || !loading.operations.remove(&op_id) {
		return;
	}
	if let Some(returned) = returned_state {
		loading.states.insert(worker.clone(), returned);
	}
	if succeeded {
		loading.succeeded.insert(worker);
	}
	decide(state, key, deleted, effects);
}

/// Whether `replica` is a replica of `key` that a cancelled close's late forget removed while
/// the current open of the key was loading it.
fn answers_lost_replica(
	state: &MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	replica: &rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
) -> bool {
	matches!(
		state.pending.get(&key.0.volume_key),
		Some(PendingOpen {
			step: OpenStep::Loading(loading),
			..
		}) if loading.key == *key
			&& loading.lost.get(worker) == Some(&replica.replica_incarnation)
	)
}

/// A forget sent by a close that an open cancelled removed replica `incarnation` of `worker`.
/// When the open loading `key` counted that replica, it stops counting it and checks its
/// quorum again; an answer that arrives later from the same replica is not counted either.
/// The open then waits for its other loads, promotes caught-up members, or is refused for a
/// retry, as it would had the member never answered.
pub(crate) fn member_lost(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: &WorkerId,
	incarnation: [u8; 16],
	effects: &mut Effects,
) {
	let Some(PendingOpen {
		step: OpenStep::Loading(loading),
		..
	}) = state.pending.get_mut(&key.0.volume_key)
	else {
		return;
	};
	if loading.key != *key {
		return;
	}
	loading.lost.insert(worker.clone(), incarnation);
	let counted = loading
		.states
		.get(worker)
		.is_some_and(|replica| replica.replica_incarnation == incarnation);
	if !counted {
		return;
	}
	loading.states.remove(worker);
	let was_counted = loading.succeeded.remove(worker);
	let quorum =
		super::membership::quorum(&state.tablet.replica_set, |w| loading.succeeded.contains(w));
	if !was_counted || quorum {
		return;
	}
	tracing::debug!(
		worker_id = ?worker,
		"a cancelled close forgot a member this open counted; rechecking its quorum"
	);
	decide(state, key.clone(), false, effects);
}

/// Decides what the current open of `key` does next from its load attempt: wait for more
/// answers, promote caught-up members, wait for a read-back, or answer its caller.
fn decide(state: &mut MaintainerState, key: VolumeLogKey, deleted: bool, effects: &mut Effects) {
	let Some(PendingOpen {
		step: OpenStep::Loading(loading),
		..
	}) = state.pending.get(&key.0.volume_key)
	else {
		return;
	};
	let quorum =
		super::membership::quorum(&state.tablet.replica_set, |w| loading.succeeded.contains(w));
	let loads_pending = !loading.operations.is_empty();
	let has_open_claim = loading.open_claim.is_some();
	// Before other loads finish, the proof below also requires the issued claim.
	if !deleted
		&& !quorum
		&& (!loads_pending || has_open_claim)
		&& promote_caught_up(state, &key, effects)
	{
		return;
	}
	if !deleted && !quorum && loads_pending {
		return;
	}
	// A read-back needs a fresh answer from every member, so an open that asked for one
	// waits for the rest of its loads, bounded by its read timer.
	if !deleted
		&& quorum
		&& loads_pending
		&& super::open_reads::waits_for_members(state, &key.0.volume_key, effects)
	{
		return;
	}
	complete_load(state, key, deleted, effects);
}

/// Joint cold promotion requires both current majorities, not an old-set-only
/// proof. The post-barrier attempt below pins this exact assignment and proves
/// every acknowledged entry is cold before conditionally forgetting a replica.
fn joint_cold_open(state: &MaintainerState) -> bool {
	matches!(
		&state.tablet.replica_set.phase,
		rivet_data::generated::shared_depot_types_v1::TabletPhase::Joint(_)
	)
}

/// A new Repairing replica must not let a retired client claim it again before promotion.
/// The claim is already issued; install it with the replica without changing its status.
/// Retain a later fence returned by a conditional forget, including matching holder data.
fn prepare_joint_cold_load(
	joint_claim: bool,
	load: &mut wire::LoadVolumeRequest,
	forgotten: Option<&rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState>,
) {
	if !joint_claim {
		return;
	}
	let Some(open) = load.open_claim.take() else {
		return;
	};
	let mut fence = (open.client, open.holder);
	if let Some(client) = &load.claimed_client
		&& client.client_epoch >= fence.0.client_epoch
	{
		fence = (client.clone(), load.claimed_holder.clone());
	}
	if let Some(replica) = forgotten
		&& let Some(client) = &replica.claimed_client
		&& client.client_epoch >= fence.0.client_epoch
	{
		fence = (client.clone(), replica.claimed_holder.clone());
	}
	load.claimed_client = Some(fence.0);
	load.claimed_holder = fence.1;
}

/// Retire Joint cold opens when their assignment changes, including attempts that
/// have not yet promoted. Their fallback loads must not lose the issued claim. Sent
/// mutations keep their normal uncertainty barriers and may only settle replies.
pub(super) fn assignment_updated(state: &mut MaintainerState, effects: &mut Effects) {
	let stale: Vec<_> = state
		.pending
		.values()
		.filter_map(|pending| {
			matches!(&pending.step, OpenStep::Loading(loading)
			if loading.joint_cold_open.is_some())
			.then_some(pending.caller.request_id)
		})
		.collect();
	for id in stale {
		expired(state, id, effects);
	}
}

/// Reloads in sync the members an open found `Repairing` with nothing above the row's cold
/// position, when the open's own answers prove that no acknowledged entry lies above it.
/// Returns whether it started any reload, in which case the open waits for them.
///
/// An acknowledged entry above cold was held by an in-sync quorum when it was acknowledged.
/// Each of those replicas keeps it: a forget needs the replica's appended position at a
/// published cold position, a recovering client truncates only entries that were never
/// acknowledged, and a lost disk changes its incarnation, which fences this grant. Corruption
/// recovery empties a replica on the same disk, but marks it `lost_entries` until a client
/// repairs it, so a marked answer proves nothing and is never counted or promoted. A member
/// that was absent, and is now a fresh replica at cold, was therefore not in that quorum. So
/// when a quorum of this attempt's answers ends at or below the row's cold position, and no
/// observed member holds a suffix above it, every acknowledged entry is in the cold store. A
/// member whose appended and cold positions both equal the row's cold position then holds all
/// of them, and only its `Repairing` status keeps it from voting. Nothing promotes such a
/// member otherwise: client repair needs a client, and a client needs an in-sync quorum to
/// open.
///
/// Each such member is forgotten at exactly that position, so a member that gained an entry
/// meanwhile refuses, and loaded again in sync. The reload carries the newest claim seen on
/// the answers or on the forgotten replica. A Joint open also installs the pending opener's
/// issued claim with InSync: resurrecting an older claim would let its client append before the
/// opener receives its reply and claims. Stable opens install their claim without changing an
/// existing Repairing status; creations load in sync. One open promotes at most once, so a
/// refused forget or a member that comes back `Repairing` ends the open.
/// Learning retains the old voting quorum: the same proof can reopen its current voters,
/// while Learning-only destinations stay nonvoting. The opener still claims its granted
/// client before serving; issuing a claim alone does not fence the previous writer.
fn promote_caught_up(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	effects: &mut Effects,
) -> bool {
	let Some(PendingOpen {
		step: OpenStep::Loading(loading),
		..
	}) = state.pending.get(&key.0.volume_key)
	else {
		return false;
	};
	let stable = matches!(
		state.tablet.replica_set.phase,
		rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
	);
	let learning = matches!(
		state.tablet.replica_set.phase,
		rivet_data::generated::shared_depot_types_v1::TabletPhase::Learning(_)
	);
	let joint_cold_open = loading.open_claim.is_some()
		&& super::promotion::observed(state)
		&& matches!(&loading.joint_cold_open,
			Some(JointColdOpen::Observed(tablet)) if tablet == &state.tablet);
	if (!stable && !learning && !joint_cold_open)
		|| loading.promoted.is_some()
		|| loading.unacknowledged_create
		|| loading.initial_through.is_some()
		|| loading.key != *key
	{
		return false;
	}
	if joint_cold_open {
		let index = state.operations.index();
		if index.loads.contains_key(key)
			|| index.repair_holds.contains_key(key)
			|| index.catch_up_holds.contains_key(key)
			|| index.forget_orphans.contains_key(key)
			|| state
				.members()
				.iter()
				.any(|worker| index.forgets.contains_key(&(key.clone(), worker.clone())))
		{
			return false;
		}
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return false;
	};
	let cold = loading.row.cold.clone();

	// The proof: a quorum of this attempt's answers holds nothing above cold, and neither
	// does any member observed in another way. A replica emptied by corruption recovery
	// may have lost acknowledged entries, so its answer is no evidence either way.
	// While other loads remain, this same quorum must have installed the exact issued
	// claim: a Load can return unchanged without accepting its requested claim.
	let no_suffix = |replica: &rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState| {
		replica.status != VolumeLogReplicaStatus::Deleted
			&& replica.appended.volume_seq <= cold.volume_seq
	};
	let answered_without_suffix = super::membership::quorum(&state.tablet.replica_set, |w| {
		loading.states.get(w).is_some_and(|replica| {
			!replica.lost_entries
				&& no_suffix(replica)
				&& (loading.operations.is_empty()
					|| loading
						.open_claim
						.as_ref()
						.is_some_and(|open| replica.claimed_client.as_ref() == Some(&open.client)))
		})
	});
	let observed_suffix = volume
		.members
		.values()
		.chain(loading.states.values())
		.any(|replica| !no_suffix(replica));
	if !answered_without_suffix || observed_suffix {
		return false;
	}

	let members = state.members();
	let workers: BTreeSet<WorkerId> = loading
		.states
		.iter()
		.filter(|(worker, replica)| {
			members.contains(worker)
				&& (!learning || state.tablet.replica_set.members.contains(worker.as_bytes()))
				&& state.disk_incarnation(worker).is_some()
				&& replica.status == VolumeLogReplicaStatus::Repairing
				&& !replica.lost_entries
				&& replica.appended == cold
				&& replica.cold == cold
		})
		.map(|(worker, _)| worker.clone())
		.collect();
	if workers.is_empty() {
		return false;
	}
	// The issued claim is newer than prior grants, but is not a disk fence until
	// installed. Include it in the reload itself, before the opener can run.
	let mut fence = joint_cold_open
		.then_some(loading.open_claim.as_ref())
		.flatten()
		.map(|claim| (&claim.client, claim.holder.as_ref()));
	for replica in volume.members.values().chain(loading.states.values()) {
		let Some(client) = replica.claimed_client.as_ref() else {
			continue;
		};
		if let Some((previous, _)) = fence {
			let previous: &ClientIdentity = previous;
			if previous.client_epoch == client.client_epoch && previous != client {
				return false;
			}
			if previous.client_epoch > client.client_epoch {
				continue;
			}
		}
		fence = Some((client, replica.claimed_holder.as_ref()));
	}
	// Each forget expects the claim this attempt's answer showed for its member.
	let expected_claims = workers
		.iter()
		.map(|worker| {
			let claim = loading
				.states
				.get(worker)
				.and_then(|replica| replica.claimed_client.clone());
			(worker.clone(), claim)
		})
		.collect::<Vec<_>>();
	let promoted = super::Promoted {
		workers: workers.clone(),
		claimed_client: fence.map(|(client, _)| client.clone()),
		claimed_holder: fence.and_then(|(_, holder)| holder.cloned()),
	};
	tracing::debug!(
		phase = ?state.tablet.replica_set.phase,
		promoted = workers.len(),
		cold = cold.volume_seq,
		"reloading caught-up repairing members in sync"
	);

	let mut op_ids = Vec::new();
	for (worker, expected_claim) in expected_claims {
		let request = StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
			observation: None,
			header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0.clone(),
					tablet_grant: state.grant(),
				},
			),
			expected_appended: Some(cold.clone()),
			expected_claim,
		});
		let op_id = state.begin(
			Operation::Load {
				step: ReplicaOpenStep::Forget,
				worker: worker.clone(),
				key: key.clone(),
			},
			effects,
		);
		op_ids.push(op_id);
		effects.execute(MaintainerEffect::ReplicaRequest {
			op_id,
			worker_id: worker,
			request,
		});
	}
	// Starting operations never changes the pending open checked above.
	if let Some(PendingOpen {
		step: OpenStep::Loading(loading),
		..
	}) = state.pending.get_mut(&key.0.volume_key)
	{
		loading.operations.extend(op_ids);
		loading.promoted = Some(promoted);
	}
	true
}

/// Answers the current caller of `key` from its load attempt: a rejection, a creation or an
/// open, the open possibly after a read-back.
pub(crate) fn complete_load(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	deleted: bool,
	effects: &mut Effects,
) {
	let pending = state
		.pending
		.remove(&key.0.volume_key)
		.expect("current caller");
	// Only a load that left the volume on fewer than every member can leave an orphaned
	// minority behind, so only such a load queues the volume for the orphan retry. A load
	// that reached every member arms nothing, so a stream of opens never sweeps the tablet.
	let partial = deleted
		|| state.loaded_volumes.get(&key).is_none_or(|volume| {
			state
				.members()
				.iter()
				.any(|worker| !volume.members.contains_key(worker))
		});
	if partial {
		state.orphan_backlog.insert(key.clone());
		effects.arm_timer_after(MaintainerTimer::OrphanSweep, state.config.tuning().retry_ms);
	}
	let OpenStep::Loading(loading) = pending.step else {
		unreachable!("current load")
	};
	if deleted {
		reject(
			pending.caller.request_id,
			VolumeRejectResponse::VolumeDeleted,
			effects,
		);
		return;
	}
	if !super::membership::quorum(&state.tablet.replica_set, |w| loading.succeeded.contains(w)) {
		if state
			.loaded_volumes
			.get(&key)
			.is_some_and(|volume| volume.members.is_empty() && volume.creation.is_none())
		{
			super::filesystem::forget(state, &key, effects);
			state.remove_loaded_volume(&key);
		}
		reject(
			pending.caller.request_id,
			depot_types::client::overloaded(),
			effects,
		);
		return;
	}

	// The disk quorum is the creation acknowledgement. Its background row waits for
	// the cold pass; loading an already present replica never draws another incarnation.
	let volume = state.loaded_volumes.get_mut(&key).expect("loaded volume");
	volume.last_activity = state.now_boot_ms;
	effects.cancel_timer(MaintainerTimer::OpenRequest(pending.caller.request_id));
	if let Some(row) = volume.unacknowledged_cold_open.take() {
		volume.stored_row = Some((*row).into());
	}
	if let Some(creation) = volume.creation.as_mut() {
		creation.acknowledged = true;
	}
	// An acknowledged creation sends no initial entries again.
	super::release_volume_initial_entries(volume, &mut state.initial_bytes_held);
	effects.cancel_timer(MaintainerTimer::CreationExpired(key.clone()));
	let row = volume.row(&key).expect("in-sync quorum");
	if volume.stored_row.is_none()
		&& !state.pending_rows.contains(&key)
		&& !state.operations.index().pass_rows.contains_key(&key)
	{
		state.pending_rows.push_back(key.clone());
		super::cold_pass::row_queued(state, effects);
	}
	// The acknowledged volume is now eligible for filesystem apply.
	super::filesystem::load(state, key.clone());
	let states = loading
		.states
		.into_iter()
		.map(|(worker, state)| client::MemberReplicaState {
			worker_id: *worker.as_bytes(),
			state,
		})
		.collect();
	let response = if pending.caller.mode == OpenMode::Create {
		Some(ClientResponse::VolumeCreateResponse(
			client::VolumeCreateResponse {
				record: row,
				open_claim: loading.open_claim,
				states,
			},
		))
	} else {
		// The initial writes are committed when a quorum of the tablet's replica set holds them
		// in sync, claimed for this open, above the row's cold position: the same evidence the
		// client checks before it appends after them.
		let created = match (&loading.initial_through, &loading.open_claim) {
			(Some(through), Some(claim)) => {
				let holds = |worker: &WorkerId| {
					states.iter().any(|member: &client::MemberReplicaState| {
						member.worker_id == *worker.as_bytes()
							&& member.state.appended == *through
							&& member.state.claimed_client.as_ref() == Some(&claim.client)
							&& member.state.status == VolumeLogReplicaStatus::InSync
							&& member.state.cold == loading.row.cold
					})
				};
				super::membership::quorum(&state.tablet.replica_set, holds).then(|| through.clone())
			}
			(Some(_), None) | (None, _) => None,
		};
		// Entries sent under the claim but not confirmed at a quorum may sit on some replicas
		// and not others, so the opener must not append under that claim. It is granted a
		// higher claim epoch, which it applies to the members itself, so those entries stay
		// at a lower epoch. Without an epoch to issue the open is refused for a retry.
		let open_claim = match (&loading.initial_through, &created, loading.open_claim) {
			(Some(_), None, Some(granted)) => {
				let above = states
					.iter()
					.map(|member: &client::MemberReplicaState| {
						super::maintainer_log::epoch_floor(&member.state)
					})
					.fold(granted.client.client_epoch, u64::max)
					.checked_add(1);
				match above.and_then(|min| super::claim_epochs::issue(state, min)) {
					Some(epoch) => {
						let client = ClientIdentity {
							client_epoch: epoch,
							client_incarnation: granted.client.client_incarnation,
						};
						record_grant(state, &key, client.clone(), Vec::new());
						Some(rivet_data::generated::shared_depot_types_v1::OpenClaim {
							client,
							..granted
						})
					}
					None => {
						reject_overloaded(pending.caller.request_id, "claim_epoch", effects);
						return;
					}
				}
			}
			(Some(_), None, None) => None,
			(Some(_), Some(_), open_claim) | (None, _, open_claim) => open_claim,
		};
		// The opener's recovery need not read the entries the maintainer already applied.
		let applied = super::filesystem::applied_history(state, &key, &row.cold);
		// An open of an existing volume may answer named keys in this reply; it then waits
		// for their read instead of replying now.
		super::open_reads::begin(
			state,
			pending.caller.request_id,
			&key,
			pending.caller.read.clone(),
			pending.caller.deadline_boot_ms,
			client::VolumeOpenResponse {
				record: row,
				open_claim,
				states,
				created,
				read: None,
				applied,
			},
			effects,
		)
	};
	if let Some(response) = response {
		effects.execute(MaintainerEffect::Reply {
			request_id: pending.caller.request_id,
			response,
		});
	}
	super::cold::queue_lagging_trims(state, &key);
	// The open answered on a quorum; reload any member its loads did not reach.
	super::member_repair::open_completed(state, &key, effects);
}

/// Retires one expired caller without rolling back loads already dispatched to disk.
pub(crate) fn expired(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let Some(key) = state.open_deadlines.keys.remove(&id) else {
		return;
	};
	fx.cancel_timer(MaintainerTimer::RepairWait(id));
	if state
		.awaiting_deletion
		.get(&key)
		.is_some_and(|caller| caller.request_id == id)
	{
		state.awaiting_deletion.remove(&key);
		count_expired("awaiting_deletion");
		reject(id, depot_types::client::overloaded(), fx);
		return;
	}
	if state
		.pending
		.get(&key)
		.is_some_and(|pending| pending.caller.request_id == id)
	{
		if let Some(pending) = state.pending.remove(&key) {
			count_expired(match pending.step {
				OpenStep::WaitingForRepair => "waiting_for_repair",
				OpenStep::ReadingRow => "reading_row",
				OpenStep::Loading(_) => "loading",
			});
		}
		super::unload::mark_scope(state, super::unload::UnloadScope::Keys(vec![key.clone()]));
		reject(id, depot_types::client::overloaded(), fx);
		// An expired load may have left minority replicas of any incarnation of this key.
		let loaded = state
			.loaded_volumes
			.range(super::incarnations(&key))
			.map(|(loaded, _)| loaded.clone())
			.collect::<Vec<_>>();
		state.orphan_backlog.extend(loaded);
		fx.arm_timer_after(MaintainerTimer::OrphanSweep, state.config.tuning().retry_ms);
	}
}

/// Counts an open that reached its deadline, by the step it was waiting in, so a load run
/// shows which step holds opens past their callers' deadlines.
fn count_expired(step: &'static str) {
	crate::metrics::TABLET_VOLUME_OPEN_EXPIRED
		.with_label_values(&[step])
		.inc();
}

/// The volume key of each open whose deadline timer is armed. A caller keeps its key for its
/// whole life, so an entry stays correct however the caller moves between `pending` and the
/// parked opens. An entry outlives a caller answered before its deadline, so entries of
/// answered callers are dropped once they outnumber the live ones.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct OpenDeadlines {
	keys: BTreeMap<RequestId, depot_types::VolumeKey>,
}

impl OpenDeadlines {
	
}

/// Arms an open's answer deadline, records its key for the expiry, and returns how long the
/// open may be held.
///
/// A held open is answered at half of the caller's remaining time. A refusal sent at the
/// caller's own deadline reaches it after it gave up, so the caller reports the maintainer
/// unreachable instead of the refusal; a caller without a deadline waits as long as its open
/// needs, as for a required learner's promotion. An open admitted again after a repair or a
/// deletion is armed again at half of what then remains, which stays before the caller's
/// deadline. Once a creating open's loads are out, `load` moves the deadline to the caller's
/// own, since the loads may still create the volume and a refusal would then be false.
fn arm_deadline(state: &mut MaintainerState, caller: &OpenVolume, effects: &mut Effects) -> u64 {
	let now_boot_ms = effects.clocks().boot_ms;
	let budget_ms = (caller.deadline_boot_ms.saturating_sub(now_boot_ms) / 2).max(1);
	effects.arm_timer_at(
		MaintainerTimer::OpenRequest(caller.request_id),
		now_boot_ms.saturating_add(budget_ms),
	);
	state
		.open_deadlines
		.keys
		.insert(caller.request_id, caller.volume_key.clone());

	// Answered callers leave their entries behind. Dropping them once they outnumber the live
	// callers keeps the index proportional to the parked opens at amortized constant cost.
	let live = state.pending.len() + state.awaiting_deletion.len();
	if state.open_deadlines.keys.len() > 2 * live + 64 {
		let (pending, deletion) = (&state.pending, &state.awaiting_deletion);
		state.open_deadlines.keys.retain(|id, key| {
			*id == caller.request_id
				|| pending.get(key).is_some_and(|p| p.caller.request_id == *id)
				|| deletion.get(key).is_some_and(|c| c.request_id == *id)
		});
	}
	budget_ms
}




