//! Closing is background work: establish the cold row, then forget only replicas still at that position.

use super::{
	MaintainerPhase, MaintainerState, Operation, cold, store::StoreWork, volume_keys::KeyId,
};
use crate::tablet_maintainer_machine::types::CLOSE_NOTICE_WAIT_MS;
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::{VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_client_protocol_v1::{ClientResponse, VolumeRejectResponse},
	depot_log_shard_v1::VolumeLogReplicaState,
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest, StorageWorkerResponse,
	},
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One close tracks its row lookup and the members still awaiting a conditional forget.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Unloading {
	pub row_check: RowCheck,
	/// Why the volume unloads, which decides what is rechecked when it reaches the store.
	pub reason: UnloadReason,
	/// This unload may return a newly drawn, unacknowledged incarnation to creation cleanup.
	pub abandoned_create: bool,
	pub remaining: BTreeSet<WorkerId>,
	/// Voting replicas that confirmed a conditional forget at the covering row.
	/// Absence alone may mean disk loss and is not evidence for a safe cold reopen.
	pub confirmed: BTreeSet<WorkerId>,
	/// A quorum conditionally forgot this incarnation at a durable cold floor. The
	/// closure fact survives assignment changes; a reopen removes its whole unload.
	pub closed: Option<ColdClose>,
	/// Members whose last forget was refused or failed and waits for `UnloadRetry`.
	pub waiting: BTreeSet<WorkerId>,
	/// A completed recovery must wait before another attempt can claim a newer client epoch.
	pub recovery_backoff: bool,
	/// When this unload began, for the unload duration metric only.
	pub began_boot_ms: u64,
	/// A requested close waits out its `CloseNotices` timer before deciding what it may
	/// forget. The writer announces nothing at close, and the shards notice the maintainer of
	/// its last appends within their notice delay; those notices mark the unload. Only a
	/// suffix still unproven when the timer fires asks the members for a status probe.
	pub awaiting_notices: bool,
}

/// Positive quorum evidence that every acknowledged entry of this closed incarnation
/// is durable in the tablet store. The containing unload supplies the exact key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ColdClose {
	pub cold: VolumeLogPosition,
}

/// Why an unload began.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum UnloadReason {
	/// The holder closed the volume, or a finished repair left it idle. Nothing is rechecked.
	Requested,
	/// A joined process released its clients. Only its captured claims may be forgotten;
	/// this opportunistic cleanup never takes over an unpublished tail or another writer.
	Released {
		client: rivet_data::generated::shared_depot_types_v1::ClientIdentity,
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	},
	/// The volume was idle for `idle_unload_after_ms`. Activity before the row read cancels it.
	Idle,
	/// An operator asked to unload volumes quiet for `quiet_ms`. Activity before the row
	/// read cancels it, as for `Idle`.
	Quiet { quiet_ms: u64 },
	/// The process named by the volume's claim is gone, as its topology watch reported. The
	/// row read requires the volume to still name it and to have stayed quiet since
	/// `quiet_since`, when it parked; see `holders`.
	HolderGone {
		holder: rivet_data::generated::shared_depot_types_v1::VolumeHolder,
		quiet_since: u64,
	},
}

/// The row lookup belongs to this close attempt; a reopened and reclosed volume cannot adopt an older result.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RowCheck {
	Queued,
	Reading(OpId),
	Checked,
}

/// Reply independently of object storage latency, retaining one unload per loaded incarnation.
pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	key: VolumeLogKey,
	client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	fx: &mut Effects,
) {
	let hash = partition_key_hash(&key.0.volume_key);
	let rejected = if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) || !state.tablet.hash_range.contains(hash)
	{
		Some(depot_types::client::not_maintainer())
	} else if state.phase == MaintainerPhase::Frozen {
		Some(VolumeRejectResponse::TabletSplitting)
	} else if key.0.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize {
		Some(depot_types::client::overloaded())
	} else if !state.unloading.contains_key(&key)
		// An exact claim already superseded by an observed claim can only be acknowledged
		// below. Its close must not wait for work owned by the newer lifecycle.
		&& !client.as_ref().is_some_and(|client| {
			state
				.loaded_volumes
				.get(&key)
				.is_some_and(|volume| matches!(close_match(volume, Some(client)), CloseMatch::Mismatch))
		})
		&& (super::deletion::ambiguous(state, &key.0.volume_key)
				|| super::forks::holds_key(state, &key.0.volume_key)
				|| state.pending.contains_key(&key.0.volume_key)
				|| super::repair::holds_key(state, &key.0.volume_key)
				|| super::catch_up::holds_key(state, &key.0.volume_key))
	{
		Some(depot_types::client::overloaded())
	} else {
		None
	};
	if let Some(reason) = rejected {
		super::lifecycle::reject(id, reason, fx);
		return;
	}
	// A duplicate close acknowledges the existing unload, even while its
	// recovery client holds this key. It must not turn success into Overloaded.
	if state.deleting.contains_key(&key) && !super::deletion::is_replaced(state, &key) {
		fx.execute(MaintainerEffect::Reply {
			request_id: id,
			response: ClientResponse::VolumeCloseResponse,
		});
		return;
	}
	if let Some(volume) = state.loaded_volumes.get(&key) {
		let deleted = volume
			.row(&key)
			.is_some_and(|row| row.status == FsVolumeStatus::Deleted);
		match close_match(volume, client.as_ref()) {
			CloseMatch::Matches if !deleted && !state.unloading.contains_key(&key) => {
				state.pending_closes.remove(&key);
				begin(state, key.clone(), UnloadReason::Requested);
				// The client announces nothing at close, so the notices of its last appends
				// may still be on their way. The unload waits one notice window for them
				// before it decides what it may forget; see `notices_due`.
				if let Some(unloading) = state.unloading.get_mut(&key) {
					unloading.awaiting_notices = true;
					fx.arm_timer_after(MaintainerTimer::CloseNotices(key), CLOSE_NOTICE_WAIT_MS);
				}
			}
			CloseMatch::Unobserved(client) if !deleted && !state.unloading.contains_key(&key) => {
				// The client claimed an epoch newer than any claim observed yet. Probe the
				// members before deciding; never unload on an epoch nobody reported.
				state.pending_closes.insert(key.clone(), client);
				super::poll::request(state, key, fx);
			}
			CloseMatch::Matches | CloseMatch::Unobserved(_) | CloseMatch::Mismatch => {}
		}
	}
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response: ClientResponse::VolumeCloseResponse,
	});
}

/// The `CloseNotices` timer of a requested close fired: the shards have had their notice
/// window to report the writer's last appends. When a member still shows a tail above what
/// the members prove committed, a notice was lost or delayed, so one status probe observes
/// every member before the unload decides; its replies mark the volume. Otherwise the unload
/// proceeds on the notices alone and sends nothing.
pub(crate) fn notices_due(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	let Some(unloading) = state.unloading.get_mut(&key) else {
		return;
	};
	if !unloading.awaiting_notices {
		return;
	}
	unloading.awaiting_notices = false;

	let proven = super::filesystem::known_committed(state, &key);
	let unproven_suffix = state.loaded_volumes.get(&key).is_some_and(|volume| {
		volume
			.members
			.values()
			.any(|member| member.appended.volume_seq > proven)
	});
	if unproven_suffix {
		super::poll::request(state, key.clone(), fx);
	}
	mark(state, &key);
}

/// How a closing client relates to the claims observed on a volume's members.
enum CloseMatch {
	/// The client is the newest observed claim holder, or there is no claim and no client.
	Matches,
	/// The client's epoch is newer than every observed claim. A probe may reveal its claim.
	Unobserved(rivet_data::generated::shared_depot_types_v1::ClientIdentity),
	/// An observed claim is at least as new and differs, so this close is stale.
	Mismatch,
}

fn close_match(
	volume: &super::LoadedVolume,
	client: Option<&rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
) -> CloseMatch {
	let claims = volume
		.members
		.values()
		.filter_map(|v| v.claimed_client.as_ref());
	match client {
		Some(client) => {
			let mut found = false;
			let compatible = claims.clone().all(|claim| {
				found |= claim == client;
				claim.client_epoch < client.client_epoch || claim == client
			});
			if found && compatible {
				CloseMatch::Matches
			} else if compatible {
				CloseMatch::Unobserved(client.clone())
			} else {
				CloseMatch::Mismatch
			}
		}
		None if claims.count() == 0 => CloseMatch::Matches,
		None => CloseMatch::Mismatch,
	}
}

/// A status probe of `key` answered. A close waiting on an unobserved claim begins its
/// unload once the claim is observed. It is dropped once observations rule it out or
/// the probe has nothing left in flight for the volume.
pub(crate) fn status_observed(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(client) = state.pending_closes.get(key).cloned() else {
		return;
	};
	let Some(volume) = state.loaded_volumes.get(key) else {
		state.pending_closes.remove(key);
		return;
	};
	let deleted = volume
		.row(key)
		.is_some_and(|row| row.status == FsVolumeStatus::Deleted);
	match close_match(volume, Some(&client)) {
		CloseMatch::Matches => {
			state.pending_closes.remove(key);
			if !deleted && !state.unloading.contains_key(key) {
				begin(state, key.clone(), UnloadReason::Requested);
			}
		}
		CloseMatch::Mismatch => {
			state.pending_closes.remove(key);
		}
		CloseMatch::Unobserved(_) => {
			if !super::poll::probing(state, key) {
				state.pending_closes.remove(key);
			}
		}
	}
}

/// Schedule one closed incarnation without synthesizing a client request or reply.
pub(crate) fn begin(state: &mut MaintainerState, key: VolumeLogKey, reason: UnloadReason) {
	// An incarnation whose stored record is lost is never unloaded, whatever the reason: the
	// unload's row would be fabricated from the replicas' creation evidence and the forgets
	// would drop the only copy of the data it covered. Its delete is the way out.
	// A repair claim's client holds the volume until the claim ends, which resumes it.
	if state.unloading.contains_key(&key)
		|| super::filesystem::is_record_missing(state, &key)
		|| super::operator_repair::claims::holds(state, &key)
	{
		return;
	}
	// A holder-gone unload always reads its row through the store, where its verdict is
	// rechecked, so it never takes the stored-row shortcut below.
	let recheck = matches!(reason, UnloadReason::HolderGone { .. });
	state.unloading.insert(
		key.clone(),
		Unloading {
			row_check: RowCheck::Queued,
			reason,
			abandoned_create: state
				.loaded_volumes
				.get(&key)
				.is_some_and(|v| v.creation.as_ref().is_some_and(|c| !c.acknowledged)),
			remaining: state.members().into_iter().collect(),
			confirmed: BTreeSet::new(),
			closed: None,
			waiting: BTreeSet::new(),
			recovery_backoff: false,
			began_boot_ms: state.now_boot_ms,
			awaiting_notices: false,
		},
	);
	state.unload_drive.dirty.insert(key.clone());
	super::catch_up::mark(state, &key);
	if !recheck && wrote_stored_row(state, &key) {
		state
			.unloading
			.get_mut(&key)
			.expect("current unload")
			.row_check = RowCheck::Checked;
		return;
	}
	state
		.store_queue
		.push_back(StoreWork::ReadUnloadRow { key });
}

/// Whether the volume's stored row is the last row this grant wrote, so the store holds it
/// and the unload's row read would only return it again.
fn wrote_stored_row(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let Some(written) = state.rows_written.get(key) else {
		return false;
	};
	state
		.loaded_volumes
		.get(key)
		.and_then(|volume| volume.stored_row.as_ref())
		.is_some_and(|row| (&row.cold, &row.status) == (&written.0, &written.1))
}

/// Waits a full idle period before `key`'s next idle check, after an idle unload was cancelled
/// for a reason the volume's activity does not change. The cancel queued the check from that
/// activity, which for an idle volume is already due, so the same row read would cancel it
/// again without time passing.
pub(crate) fn postpone_idle(state: &mut MaintainerState, key: &VolumeLogKey) {
	if state.loaded_volumes.contains_key(key) {
		let at = state
			.now_boot_ms
			.saturating_add(state.config.idle_unload_after_ms);
		let id = state.volume_keys.intern(key);
		state.idle_checks.queue.schedule(id, at);
	}
}

/// A reopened key cancels decisions from the previous close. A forget already sent may still
/// land, but it is conditional on the row's position, so it removes only a replica that holds
/// nothing above the row.
pub(crate) fn cancel_key(
	state: &mut MaintainerState,
	key: &depot_types::VolumeKey,
	fx: &mut Effects,
) {
	let keys = state
		.unloading
		.range(super::incarnations(key))
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	for key in keys {
		cancel(state, &key, fx);
	}
}

/// Retire one incarnation's close without cancelling another incarnation that shares its volume key.
///
/// The forgets this close has in flight keep running, detached from it: cancelling a sent
/// forget would not stop it from applying, only hide its answer. Their replies go to
/// [`late_forget`], which reports a member they removed as lost, so a reopen never counts a
/// replica that a forget removed after it answered.
pub(crate) fn cancel(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	state.unloading.remove(key);
	super::catch_up::mark(state, key);
	// The volume stays loaded, so its idle check resumes from its activity.
	track_idle(state, key);
	state
		.store_queue
		.remove_all(&[StoreWork::ReadUnloadRow { key: key.clone() }]);
	fx.cancel_timer(MaintainerTimer::UnloadRetry(key.clone()));
	fx.cancel_timer(MaintainerTimer::UnloadRecoveryRetry(key.clone()));
	let pending = state
		.operations
		.index()
		.forgets
		.range(super::operations::workers_of(key))
		.flat_map(|(_, ids)| ids.iter().copied())
		.collect::<Vec<_>>();
	for id in pending {
		state.operations.update(&id, |operation| {
			if let Operation::ForgetVolumes { detached, .. } = operation {
				detached.insert(key.clone());
			}
		});
	}
	// An active store read keeps its reservation until completion; its result is ignored.
}

/// A forget sent by a close that was cancelled before it answered. When it removed the
/// replica, the member is lost: its observation is dropped, an open loading the volume stops
/// counting it, and member repair reloads it once the volume serves again. A refusal, an
/// absent replica or an unknown outcome removed nothing this reply can prove; a forget that
/// applies later is found by the next observation of the member.
pub(crate) fn late_forget(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	expected: VolumeLogPosition,
	result: Result<StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	let response = match result {
		Ok(StorageWorkerResponse::ReplicaStateResponse(response)) => response,
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::AppendedMismatch(refusal))) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&[refusal_label(&refusal.state, &expected)])
				.inc();
			// Member repair waits for this forget to end; a member missing for another
			// reason may be reloaded now.
			super::member_repair::absent(state, &key, &worker, fx);
			return;
		}
		Ok(_) | Err(_) => {
			super::member_repair::absent(state, &key, &worker, fx);
			return;
		}
	};
	let removed = response.state.status != VolumeLogReplicaStatus::Deleted
		&& response.state.appended == expected
		&& response.state.cold == expected;
	if !removed {
		super::member_repair::absent(state, &key, &worker, fx);
		return;
	}
	crate::metrics::TABLET_UNLOAD_FORGETS
		.with_label_values(&["late"])
		.inc();
	let incarnation = response.state.replica_incarnation;
	if let Some(volume) = state.loaded_volumes.get_mut(&key)
		&& volume
			.members
			.get(&worker)
			.is_some_and(|member| member.replica_incarnation == incarnation)
	{
		volume.members.remove(&worker);
		super::catch_up::mark(state, &key);
	}
	super::lifecycle::member_lost(state, &key, &worker, incarnation, fx);
	super::member_repair::absent(state, &key, &worker, fx);
}

/// The metric label of a conditional forget the replica refused: `claim_changed` when the
/// replica still sat at the expected position, so only a newer claim kept it, otherwise
/// `mismatch` for a replica that moved past that position.
fn refusal_label(replica: &VolumeLogReplicaState, expected: &VolumeLogPosition) -> &'static str {
	if replica.appended == *expected && replica.cold == *expected {
		"claim_changed"
	} else {
		"mismatch"
	}
}

/// The incarnation's own live record raises the known cold floor; a Deleted one ends this close.
pub(crate) fn row(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	op_id: OpId,
	result: Result<Option<FsVolumeRecord>, String>,
	fx: &mut Effects,
) {
	if !state
		.unloading
		.get(&key)
		.is_some_and(|unload| unload.row_check == RowCheck::Reading(op_id))
	{
		return;
	}
	let Ok(row) = result else {
		super::fence(state, fx);
		return;
	};
	// The lookup reads this incarnation's own record.
	if row
		.as_ref()
		.is_some_and(|row| row.volume_incarnation != key.0.volume_incarnation)
	{
		super::fence(state, fx);
		return;
	}
	// No record for a volume whose replicas start above zero: the record is lost, and this
	// unload must not write one from memory. The mark cancels the unload.
	if row.is_none() && super::filesystem::record_lost(state, &key) {
		super::filesystem::mark_record_missing(state, &key, fx);
		return;
	}
	if let Some(row) = row {
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.stored_row = Some(row.clone().into());
		}
		// The stored row can make an unregistered volume eligible for filesystem apply.
		super::filesystem::offer(state, &key);
		if row.status == FsVolumeStatus::Deleted {
			super::deletion::observe_loaded(state, &key);
			cancel_key(state, &key.0.volume_key, fx);
			return;
		}
		super::filesystem::stored(state, &key, &row, fx);
	}
	state
		.unloading
		.get_mut(&key)
		.expect("current unload")
		.row_check = RowCheck::Checked;
	mark(state, &key);
}

/// Retry only after backoff; intervening events cannot busy-loop an unavailable member.
pub(crate) fn retry(state: &mut MaintainerState, key: VolumeLogKey, _fx: &mut Effects) {
	if let Some(unload) = state.unloading.get_mut(&key) {
		unload.waiting.clear();
	}
}

/// Back off after joined cleanup, including success, while status observations catch up with the new claim.
pub(crate) fn recovery_finished(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if let Some(unload) = state.unloading.get_mut(key) {
		unload.recovery_backoff = true;
		fx.arm_timer_after(
			MaintainerTimer::UnloadRecoveryRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
	}
}

/// Only the recovery retry alarm releases its backoff; ordinary forget retries cannot busy-loop recovery.
pub(crate) fn retry_recovery(state: &mut MaintainerState, key: VolumeLogKey) {
	if let Some(unload) = state.unloading.get_mut(&key) {
		unload.recovery_backoff = false;
	}
}

/// Idle checks visited per timer delivery. Due checks past it stay queued and the
/// timer fires again at once, so one delivery's work stays bounded.
const IDLE_CHECKS_PER_TIMER: usize = 512;

/// When each loaded volume is next checked for idle unload. Entries are check times, not
/// authority: a due check rederives the deadline from the volume's `last_activity`, which
/// only moves later, so writers of that field need no hook. Volumes are named by their
/// interned key id; an id whose volume unloaded resolves to nothing and is dropped.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct IdleChecks {
	pub(crate) queue: IndexedDeadlineQueue<KeyId>,
	/// The time `MaintainerTimer::IdleUnload` is armed for.
	pub(crate) armed_at: Option<u64>,
}

/// Schedules a newly loaded volume's idle check at its next due time.
pub(crate) fn track_idle(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(volume) = state.loaded_volumes.get(key) else {
		return;
	};
	// A quarantined incarnation never unloads, so it owns no idle check.
	if super::filesystem::is_record_missing(state, key) {
		return;
	}
	let at = next_check_at(state, volume.last_activity, state.now_boot_ms);
	let id = state.volume_keys.intern(key);
	if state.idle_checks.queue.scheduled_at(&id).is_some() {
		return;
	}
	state.idle_checks.queue.schedule(id, at);
}

/// When a volume last active at `last_activity` is next checked: at its holder check while
/// that is still ahead of `now` and earlier than its idle deadline, else at the idle deadline.
/// A volume whose claim names no holder skips the holder check when it falls due.
fn next_check_at(state: &MaintainerState, last_activity: u64, now: u64) -> u64 {
	let idle_at = last_activity.saturating_add(state.config.idle_unload_after_ms);
	let holder_at = last_activity.saturating_add(state.config.tuning().holder_check_after_ms);
	if holder_at < idle_at && holder_at > now {
		holder_at
	} else {
		idle_at
	}
}

/// Runs the due idle checks. Delivery time is the clock, so a late timer cannot extend
/// activity, and each volume's current `last_activity` decides whether it is idle.
pub(crate) fn idle_due(state: &mut MaintainerState, fx: &mut Effects) {
	state.idle_checks.armed_at = None;
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let now = state.now_boot_ms;
	let due = state.idle_checks.queue.pop_due(now, IDLE_CHECKS_PER_TIMER);
	crate::metrics::record_idle_visits("idle_unload", due.len());
	// An idle minority that orphan cleanup can prove is forgotten instead of unloaded.
	let orphans_provable = super::opening::orphans_provable(state);
	let mut orphans_due = Vec::new();
	for (_, id) in due {
		let Some(key) = state.volume_keys.key(id).cloned() else {
			continue;
		};
		let Some(volume) = state.loaded_volumes.get(&key) else {
			continue;
		};
		// A quarantined incarnation neither parks nor unloads, and no check is queued again.
		if super::filesystem::is_record_missing(state, &key) {
			continue;
		}
		// A volume parked under the live process its claim names stays loaded; its holder's
		// check, not the idle rule, decides when it unloads.
		if super::holders::parked(state, &key, volume) {
			continue;
		}
		let last_activity = volume.last_activity;
		let deadline = last_activity.saturating_add(state.config.idle_unload_after_ms);
		if deadline > now {
			// A quiet volume past its holder check parks under the process its claim names,
			// which keeps it loaded while that process is live. A volume whose claim names no
			// holder waits for its idle deadline. New activity moves both checks later.
			let holder_due =
				last_activity.saturating_add(state.config.tuning().holder_check_after_ms);
			let held_by = super::holders::holder(volume);
			if holder_due <= now
				&& holder_due < deadline
				&& !state.pending.contains_key(&key.0.volume_key)
				&& !state.unloading.contains_key(&key)
			{
				match held_by {
					Some(holder) => super::holders::park(state, id, &holder),
					None => state.idle_checks.queue.schedule(id, deadline),
				}
			} else {
				let at = next_check_at(state, last_activity, now);
				state.idle_checks.queue.schedule(id, at);
			}
			continue;
		}
		if orphans_provable && super::opening::is_orphan(state, &key, volume) {
			orphans_due.push(key);
		} else if !volume.is_deleted()
			&& !state.pending.contains_key(&key.0.volume_key)
			&& !state.unloading.contains_key(&key)
		{
			begin(state, key.clone(), UnloadReason::Idle);
		}
		// No recheck is queued. An unload removes the volume, and a cancelled unload or a
		// finished open schedules the next check from the volume's activity.
	}
	if !orphans_due.is_empty() {
		super::opening::collect_orphans(
			state,
			"idle",
			super::opening::OrphanScan::Volumes(orphans_due),
			fx,
		);
	}
}

/// Keeps `MaintainerTimer::IdleUnload` armed at the head of the idle check queue.
pub(crate) fn arm_idle(state: &mut MaintainerState, fx: &mut Effects) {
	let next_at = state.idle_checks.queue.next_at();
	super::arm_queue_head(
		&mut state.idle_checks.armed_at,
		next_at,
		&state.phase,
		state.now_boot_ms,
		MaintainerTimer::IdleUnload,
		fx,
	);
}

/// Every queued idle check belongs to a loaded volume.
pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	state.idle_checks.queue.debug_validate()?;
	for (_, id) in state.idle_checks.queue.iter() {
		anyhow::ensure!(
			state
				.volume_keys
				.key(*id)
				.is_some_and(|key| state.loaded_volumes.contains_key(key)),
			"idle check queued for a volume that is not loaded"
		);
	}
	Ok(())
}

/// Recheck an idle or holder-gone decision at dispatch, after any queued open or read.
pub(crate) fn ready(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) -> bool {
	let reason = state.unloading.get(key).map(|unload| unload.reason.clone());
	match reason {
		Some(UnloadReason::Idle) => {
			if state
				.loaded_volumes
				.get(key)
				.is_none_or(|v| !v.idle(state.now_boot_ms, state.config.idle_unload_after_ms))
			{
				cancel(state, key, fx);
				return false;
			}
			true
		}
		Some(UnloadReason::Quiet { quiet_ms }) => {
			if state
				.loaded_volumes
				.get(key)
				.is_none_or(|v| !v.idle(state.now_boot_ms, quiet_ms))
			{
				cancel(state, key, fx);
				return false;
			}
			true
		}
		Some(UnloadReason::HolderGone {
			holder,
			quiet_since,
		}) => super::holders::ready(state, key, &holder, quiet_since, fx),
		Some(UnloadReason::Requested | UnloadReason::Released { .. }) | None => true,
	}
}

/// Which unloading volumes the next `drive` evaluates. The drive runs after every event
/// but visits only these, so its cost per event does not grow with the number of
/// unloading volumes. Every change that can let an unload advance marks its volume: an
/// operation of that volume key finishing, an event naming the key, a new unload, and the
/// end of a shared limit for the volumes waiting on it. The debug oracle checks that no
/// change is missed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct UnloadDrive {
	pub dirty: BTreeSet<VolumeLogKey>,
	/// Volumes that stopped at the full forget window; a finished forget marks them.
	pub window_waiting: BTreeSet<VolumeLogKey>,
	/// Volumes whose recovery waited for a repair slot; a finished recovery marks them.
	pub repair_waiting: BTreeSet<VolumeLogKey>,
}

/// What an event or a finished operation may have changed for unloading volumes.
pub(crate) enum UnloadScope {
	None,
	/// Every incarnation of these volume keys.
	Keys(Vec<depot_types::VolumeKey>),
	/// Membership or discovery changed for every volume.
	All,
}

/// Marks an unloading volume for the next `drive`.
pub(crate) fn mark(state: &mut MaintainerState, key: &VolumeLogKey) {
	if state.unloading.contains_key(key) {
		state.unload_drive.dirty.insert(key.clone());
	}
}

/// Marks the unloading volumes `scope` covers.
pub(crate) fn mark_scope(state: &mut MaintainerState, scope: UnloadScope) {
	match scope {
		UnloadScope::None => {}
		UnloadScope::Keys(volume_keys) => {
			for volume_key in volume_keys {
				super::deletion::mark_volume_key(state, &volume_key);
				let keys = state
					.unloading
					.range(super::incarnations(&volume_key))
					.map(|(key, _)| key.clone())
					.collect::<Vec<_>>();
				state.unload_drive.dirty.extend(keys);
			}
		}
		UnloadScope::All => {
			mark_all(state);
			super::deletion::mark_all(state);
		}
	}
}

/// Marks every unloading volume, for rare changes that affect all of them and for the
/// debug oracle's full pass.
pub(crate) fn mark_all(state: &mut MaintainerState) {
	let keys = state.unloading.keys().cloned().collect::<Vec<_>>();
	state.unload_drive.dirty.extend(keys);
}

/// A finished operation may free the forget window, a repair slot, or anything its own
/// volume keys wait on.
pub(crate) fn operation_finished(state: &mut MaintainerState, operation: &Operation) {
	match operation {
		Operation::ForgetVolumes { .. } | Operation::ForgetOrphan { .. } => {
			let waiting = std::mem::take(&mut state.unload_drive.window_waiting);
			state.unload_drive.dirty.extend(waiting);
		}
		Operation::RecoverVolume { .. } => {
			let waiting = std::mem::take(&mut state.unload_drive.repair_waiting);
			state.unload_drive.dirty.extend(waiting);
		}
		// Drain claims share the replica delete window.
		Operation::DeleteReplica { .. } | Operation::DrainClaim { .. } => {
			super::deletion::replica_delete_finished(state)
		}
		// A removed record waits for the next ordinary pass, so a finished pass may let
		// every deletion advance.
		Operation::WriteToCold { .. }
		| Operation::FilesystemFlush { .. }
		| Operation::PendingWork { .. }
		| Operation::SweepPending { .. } => {
			super::deletion::mark_all(state);
		}
		Operation::MaterializeStore { .. }
		| Operation::MaterializePeer { .. }
		| Operation::ForkCleanupStore { .. }
		| Operation::ForkCleanupPeer { .. }
		| Operation::ForkStore { .. }
		| Operation::ForkPeer { .. }
		| Operation::ForkReference { .. }
		| Operation::Checkpoint { .. }
		| Operation::FilesystemQuery { .. }
		| Operation::FilesystemLoad { .. }
		| Operation::FilesystemRead { .. }
		| Operation::FilesystemApply { .. }
		| Operation::PromotionList { .. }
		| Operation::CatchUp { .. }
		| Operation::ReadDeleteRow { .. }
		| Operation::SweepDeleted { .. }
		| Operation::PrepareSplit { .. }
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::DeleteRetired { .. }
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::ReportVersionMarks { .. }
		| Operation::ReadUnloadRow { .. }
		| Operation::Trim { .. }
		| Operation::TrimBatch { .. }
		| Operation::Poll { .. }
		| Operation::OpenCold
		| Operation::List { .. }
		| Operation::ReadRow { .. }
		| Operation::ReadOpenKeys { .. }
		| Operation::Repair(_)
		| Operation::Load { .. }
		| Operation::MaintainerLog { .. }
		| Operation::ReplayOp { .. }
		| Operation::Alarm(_) => {}
	}
	let scope = operation_scope(operation);
	mark_scope(state, scope);
}

/// The volume keys an event names directly. Operation responses are covered when the
/// operation finishes, and a new unload marks itself.
pub(crate) fn event_scope(
	body: &rivet_state_machine_ephemeral::RuntimeEvent<
		crate::tablet_maintainer_machine::event::MaintainerEvent,
		MaintainerTimer,
	>,
) -> UnloadScope {
	use crate::tablet_maintainer_machine::event::MaintainerEvent as Input;
	use rivet_state_machine_ephemeral::RuntimeEvent;
	match body {
		RuntimeEvent::Input(Input::DeleteVolumeRequest { volume_log_key, .. }) => {
			log_key(volume_log_key)
		}
		RuntimeEvent::Input(Input::ReplicaStatusNotices { notices, .. }) => UnloadScope::Keys(
			notices
				.iter()
				.map(|notice| notice.volume_log_key.0.volume_key.clone())
				.collect(),
		),
		RuntimeEvent::Input(Input::FilesystemLoadRequest { key })
		| RuntimeEvent::Input(Input::FilesystemCommittedRequest { key, .. }) => log_key(key),
		RuntimeEvent::Input(Input::OpenVolumeRequest(_)) => UnloadScope::None,
		RuntimeEvent::Input(Input::AssignmentUpdated { .. }) => UnloadScope::All,
		RuntimeEvent::Input(Input::StopReadinessRequest { .. })
		| RuntimeEvent::Input(Input::ManifestObserved { .. })
		| RuntimeEvent::Input(Input::HandoffReadinessRequest { .. })
		| RuntimeEvent::Input(Input::MaterializeResponse { .. })
		| RuntimeEvent::Input(Input::ForkCleanupResponse { .. })
		| RuntimeEvent::Input(Input::ForkRequest { .. })
		| RuntimeEvent::Input(Input::ForkStoreResponse { .. })
		| RuntimeEvent::Input(Input::ForkPeerResponse { .. })
		| RuntimeEvent::Input(Input::ForkReferenceRequest { .. })
		| RuntimeEvent::Input(Input::ForkReferenceResponse { .. })
		| RuntimeEvent::Input(Input::PendingWorkResponse { .. })
		| RuntimeEvent::Input(Input::SweepPendingResponse { .. })
		| RuntimeEvent::Input(Input::CheckpointRequest { .. })
		| RuntimeEvent::Input(Input::CheckpointResponse { .. })
		| RuntimeEvent::Input(Input::FilesystemQueryRequest { .. })
		| RuntimeEvent::Input(Input::FilesystemQueryResponse { .. })
		| RuntimeEvent::Input(Input::FilesystemLoadResponse { .. })
		| RuntimeEvent::Input(Input::FilesystemApplyResponse { .. })
		| RuntimeEvent::Input(Input::FilesystemFlushStaged { .. })
		| RuntimeEvent::Input(Input::FilesystemFlushResponse { .. })
		| RuntimeEvent::Input(Input::MemtableBudget { .. })
		| RuntimeEvent::Input(Input::KvCommandRequest { .. })
		| RuntimeEvent::Input(Input::ReportCatchUpResponse { .. })
		| RuntimeEvent::Input(Input::RetiredTabletsUpdated { .. })
		| RuntimeEvent::Input(Input::DeleteRetiredResponse { .. })
		| RuntimeEvent::Input(Input::ReportRetiredResponse { .. })
		| RuntimeEvent::Input(Input::ReportParentLogImportedResponse { .. })
		| RuntimeEvent::Timer(MaintainerTimer::ParentLogReportRetry)
		| RuntimeEvent::Input(Input::ReportVersionMarksResponse { .. })
		| RuntimeEvent::Input(Input::OperatorTierRequest { .. })
		| RuntimeEvent::Input(Input::OperatorUnloadRequest { .. })
		| RuntimeEvent::Input(Input::SweepDeletedResponse { .. })
		| RuntimeEvent::Input(Input::PrepareSplitResponse { .. })
		| RuntimeEvent::Input(Input::ReportSplitResponse { .. })
		| RuntimeEvent::Input(Input::HolderGone { .. })
		| RuntimeEvent::Input(Input::HolderWatchRefused { .. })
		| RuntimeEvent::Input(Input::RecoverVolumeResponse { .. })
		| RuntimeEvent::Input(Input::CloseVolumeRequest { .. })
		| RuntimeEvent::Input(Input::ColdClosed { .. })
		| RuntimeEvent::Input(Input::WriteToColdResponse { .. })
		| RuntimeEvent::Input(Input::AlarmResponse { .. })
		| RuntimeEvent::Input(Input::AlarmIndexReadyResponse { .. })
		| RuntimeEvent::Input(Input::AlarmBootstrapProgress { .. })
		| RuntimeEvent::Input(Input::FilesystemAlarmApplyResponse { .. })
		| RuntimeEvent::Input(Input::OpenColdResponse { .. })
		| RuntimeEvent::Input(Input::ReplicaResponse { .. })
		| RuntimeEvent::Input(Input::ReadRowResponse { .. })
		| RuntimeEvent::Input(Input::ReadOpenKeysResponse { .. })
		| RuntimeEvent::Input(Input::Repair(_))
		| RuntimeEvent::Input(Input::Drain)
		| RuntimeEvent::Input(Input::Fence)
		| RuntimeEvent::Input(Input::MaintainerOpResponse { .. }) => UnloadScope::None,
		RuntimeEvent::Timer(MaintainerTimer::CreationExpired(key))
		| RuntimeEvent::Timer(MaintainerTimer::UnloadRetry(key))
		| RuntimeEvent::Timer(MaintainerTimer::CloseNotices(key))
		| RuntimeEvent::Timer(MaintainerTimer::UnloadRecoveryRetry(key))
		| RuntimeEvent::Timer(MaintainerTimer::TrimRetry(key, _))
		| RuntimeEvent::Timer(MaintainerTimer::FilesystemDue(key))
		| RuntimeEvent::Timer(MaintainerTimer::FilesystemRetry(key))
		| RuntimeEvent::Timer(MaintainerTimer::DeleteRequest(key))
		| RuntimeEvent::Timer(MaintainerTimer::DeleteRetry(key)) => log_key(key),
		RuntimeEvent::Timer(MaintainerTimer::ForkRetry(volume_key)) => {
			UnloadScope::Keys(vec![volume_key.clone()])
		}
		RuntimeEvent::Timer(MaintainerTimer::Alarms)
		| RuntimeEvent::Timer(MaintainerTimer::IdleUnload)
		| RuntimeEvent::Timer(MaintainerTimer::HolderWatchRetry)
		| RuntimeEvent::Timer(MaintainerTimer::OpenRead(_))
		| RuntimeEvent::Timer(MaintainerTimer::PendingWorkTick)
		| RuntimeEvent::Timer(MaintainerTimer::HeldSweepRetry)
		| RuntimeEvent::Timer(MaintainerTimer::ForkCleanupDue)
		| RuntimeEvent::Timer(MaintainerTimer::MaterializeRetry(_))
		| RuntimeEvent::Timer(MaintainerTimer::RetentionDue)
		| RuntimeEvent::Timer(MaintainerTimer::FilesystemFlushBatch)
		| RuntimeEvent::Timer(MaintainerTimer::CatchUp)
		| RuntimeEvent::Timer(MaintainerTimer::SplitRetry)
		| RuntimeEvent::Timer(MaintainerTimer::OrphanSweep)
		| RuntimeEvent::Timer(MaintainerTimer::WriteToCold)
		| RuntimeEvent::Timer(MaintainerTimer::ColdBatch)
		| RuntimeEvent::Timer(MaintainerTimer::Poll)
		| RuntimeEvent::Timer(MaintainerTimer::ForkRequest(_))
		| RuntimeEvent::Timer(MaintainerTimer::CheckpointRequest(_))
		| RuntimeEvent::Timer(MaintainerTimer::ForkReferenceRequest(_))
		| RuntimeEvent::Timer(MaintainerTimer::OpenRequest(_))
		| RuntimeEvent::Timer(MaintainerTimer::RepairWait(_))
		| RuntimeEvent::Timer(MaintainerTimer::RetiredRetry(_))
		| RuntimeEvent::Timer(MaintainerTimer::VersionMarksRetry)
		| RuntimeEvent::Timer(MaintainerTimer::ReadRequest(_))
		| RuntimeEvent::Timer(MaintainerTimer::Operation(_))
		| RuntimeEvent::Timer(MaintainerTimer::ListRetry(_))
		| RuntimeEvent::Timer(MaintainerTimer::MemberRepairRetry(..))
		| RuntimeEvent::Timer(MaintainerTimer::PromotionRetry(_))
		| RuntimeEvent::Timer(MaintainerTimer::MaintainerLogDue)
		| RuntimeEvent::Timer(MaintainerTimer::MemtableBudgetShed)
		| RuntimeEvent::Timer(MaintainerTimer::MaintainerLogRetry(_)) => UnloadScope::None,
	}
}

fn log_key(key: &VolumeLogKey) -> UnloadScope {
	UnloadScope::Keys(vec![key.0.volume_key.clone()])
}

/// The volume keys whose unload an operation's completion may affect.
fn operation_scope(operation: &Operation) -> UnloadScope {
	match operation {
		Operation::Trim { key, .. }
		| Operation::RecoverVolume { key, .. }
		| Operation::CatchUp { key, .. }
		| Operation::DeleteReplica { key, .. }
		| Operation::DrainClaim { key, .. }
		| Operation::SweepDeleted { key, .. }
		| Operation::ReadUnloadRow { key }
		| Operation::ReadDeleteRow { key }
		| Operation::FilesystemRead { key, .. }
		| Operation::FilesystemApply { key, .. }
		| Operation::FilesystemLoad { key, .. }
		| Operation::Load { key, .. }
		| Operation::ForgetOrphan { key, .. } => log_key(key),
		Operation::ReadRow { volume_key } | Operation::ReadOpenKeys { volume_key, .. } => {
			UnloadScope::Keys(vec![volume_key.clone()])
		}
		Operation::Repair(_) => UnloadScope::None,
		Operation::FilesystemQuery { volume_key, .. } => match volume_key {
			Some(volume_key) => UnloadScope::Keys(vec![volume_key.clone()]),
			None => UnloadScope::None,
		},
		Operation::Poll { keys, .. } => {
			UnloadScope::Keys(keys.iter().map(|key| key.0.volume_key.clone()).collect())
		}
		Operation::TrimBatch { trims, .. } => UnloadScope::Keys(
			trims
				.iter()
				.map(|(key, _)| key.0.volume_key.clone())
				.collect(),
		),
		Operation::ForgetVolumes { volumes, .. } => UnloadScope::Keys(
			volumes
				.iter()
				.map(|(key, _)| key.0.volume_key.clone())
				.collect(),
		),
		Operation::WriteToCold { rows, .. } => {
			UnloadScope::Keys(rows.keys().map(|key| key.0.volume_key.clone()).collect())
		}
		Operation::FilesystemFlush { records, .. } => {
			UnloadScope::Keys(records.iter().map(|record| record.volume_key()).collect())
		}
		Operation::List { .. }
		| Operation::PromotionList { .. }
		| Operation::OpenCold
		| Operation::PrepareSplit { .. } => UnloadScope::All,
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
		| Operation::ReportCatchUp { .. }
		| Operation::ReportSplit
		| Operation::DeleteRetired { .. }
		| Operation::ReportRetired { .. }
		| Operation::ForgetRetiredLog { .. }
		| Operation::ReportParentLogImported
		| Operation::Alarm(_)
		| Operation::ReportVersionMarks { .. }
		| Operation::MaintainerLog { .. } => UnloadScope::None,
		Operation::ReplayOp { volume_key, .. } => match volume_key {
			Some(volume_key) => UnloadScope::Keys(vec![volume_key.clone()]),
			None => UnloadScope::None,
		},
	}
}

/// Drive row durability before forget, sharing the cleanup RPC budget with orphan cleanup.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if state.unloading.is_empty() {
		return;
	}
	// This runs after every maintainer event over the marked volumes, so each check of
	// in-flight work is an index lookup rather than a scan of the operations.

	// Forgets ready this pass, grouped by member into batches that are sent as one
	// request each. `max_forgets_in_flight` bounds requests, so a member has at most one
	// batch building here.
	let mut batches: BTreeMap<WorkerId, Vec<(VolumeLogKey, VolumeLogPosition)>> = BTreeMap::new();
	let batch_limit = state.config.max_forget_batch_volumes as usize;

	let keys = std::mem::take(&mut state.unload_drive.dirty)
		.into_iter()
		.collect::<Vec<_>>();
	let mut window_full_at = None;
	'keys: for (position, key) in keys.iter().cloned().enumerate() {
		// A mark can outlive its unload.
		if !state.unloading.contains_key(&key) {
			continue;
		}
		// Forgetting replicas waits for the volume's recovered maintainer log ops; the end of
		// its replay marks it again.
		if super::maintainer_log::replay::pending_for(state, &key.0.volume_key) {
			continue;
		}
		if super::deletion::ambiguous(state, &key.0.volume_key) {
			continue;
		}
		let Some(volume) = state.loaded_volumes.get(&key) else {
			cancel(state, &key, fx);
			continue;
		};
		if let UnloadReason::Released { client, holder } = &state.unloading[&key].reason
			&& volume.members.values().any(|replica| {
				replica.claimed_client.as_ref().is_some_and(|claim| {
					claim.client_epoch > client.client_epoch
						|| (claim != client && replica.claimed_holder.as_ref() != Some(holder))
				})
			}) {
			cancel(state, &key, fx);
			continue;
		}
		// Only this lifetime's never-acknowledged create can return to orphan cleanup.
		// A loaded minority from an earlier lifetime is not equivalent evidence.
		if state.unloading[&key].abandoned_create
			&& state.unloading[&key].row_check == RowCheck::Checked
			&& volume.stored_row.is_none()
			&& !super::membership::quorum(&state.tablet.replica_set, |w| {
				volume.members.contains_key(w)
			}) && !state.pending.contains_key(&key.0.volume_key)
			&& !state.operations.index().loads.contains_key(&key)
		{
			cancel(state, &key, fx);
			state.release_initial_entries(&key);
			state.loaded_volumes.get_mut(&key).unwrap().creation = None;
			super::opening::collect_orphans(
				state,
				"abandoned_create",
				super::opening::OrphanScan::Volumes(vec![key.clone()]),
				fx,
			);
			state.orphan_backlog.insert(key.clone());
			fx.arm_timer_after(MaintainerTimer::OrphanSweep, state.config.tuning().retry_ms);
			continue;
		}
		let Some(row) = volume.row(&key) else {
			continue;
		};
		if row.status == FsVolumeStatus::Deleted {
			super::deletion::observe_loaded(state, &key);
			cancel_key(state, &key.0.volume_key, fx);
			continue;
		}
		// Finish the current lifecycle and cold-store work before deciding what may leave memory.
		if state.unloading[&key].row_check != RowCheck::Checked
			|| state.pending.contains_key(&key.0.volume_key)
			|| super::repair::holds_key(state, &key.0.volume_key)
			|| super::catch_up::holds_key(state, &key.0.volume_key)
			|| super::member_repair::loading(state, &key)
			|| store_busy(state, &key)
		{
			continue;
		}
		// An expired, unacknowledged create is not permission to publish a new row.
		// TODO: Resolve retention or cleanup of uncertain minority creates before releasing this unload.
		if volume.stored_row.is_none()
			&& !super::membership::quorum(&state.tablet.replica_set, |w| {
				volume.members.contains_key(w)
			}) {
			continue;
		}
		// A closing client announces nothing, so the members' notices of its last appends,
		// or the backstop probe after them, decide what the unload may forget. Both mark
		// this volume.
		if matches!(state.unloading[&key].reason, UnloadReason::Requested)
			&& (state.unloading[&key].awaiting_notices || super::poll::probing(state, &key))
		{
			continue;
		}
		if super::filesystem::known_committed(state, &key) > row.cold.volume_seq {
			// The volume is put away only once ordinary tiering uploaded its data; it never
			// asks for a pass of its own. The pass's completion, and the trims and status
			// observations that then raise the members' cold positions, mark this volume.
			continue;
		}

		// Recover an unpublished suffix before any member leaves the voting set. A member
		// emptied by corruption recovery may have lost one that no observed member shows, so
		// it needs the same recovery, which repairs it from an in-sync quorum.
		if volume
			.members
			.values()
			.any(|r| r.appended.volume_seq > row.cold.volume_seq || r.lost_entries)
		{
			if matches!(state.unloading[&key].reason, UnloadReason::Released { .. }) {
				// A new claim could fence a client whose claim is still in flight. Leave
				// unpublished history to the existing membership/idle recovery policy.
				cancel(state, &key, fx);
				continue;
			}
			if !volume
				.creation
				.as_ref()
				.is_some_and(|creation| !creation.acknowledged)
				&& !state.unloading[&key].recovery_backoff
				&& super::repair::active_jobs(state) < state.config.tuning().max_repairs as usize
				&& super::membership::quorum(&state.tablet.replica_set, |w| {
					volume
						.members
						.get(w)
						.is_some_and(|r| r.status == VolumeLogReplicaStatus::InSync)
				}) && !forgetting(state, &key)
			{
				super::repair::recover(state, key, row, super::repair::RecoveryPurpose::Unload, fx);
			} else if super::repair::active_jobs(state)
				>= state.config.tuning().max_repairs as usize
			{
				state.unload_drive.repair_waiting.insert(key);
			}
			continue;
		}

		// The row written by this exact pass must cover every replica that may be forgotten.
		// A filesystem volume's row rides its next tiering pass, since the cold pass skips
		// rows the filesystem owns; that pass's completion marks this volume.
		if volume.stored_row.as_deref() != Some(&row) && super::filesystem::owns(state, &key) {
			continue;
		}
		if volume.stored_row.as_deref() != Some(&row) {
			// A queued row stays queued until a pass writes it: a pass that leaves it out
			// queues the next one, and the pass that writes it marks this volume. This pass
			// uploads, unlike a deletion's: the replicas may be forgotten only once the row
			// is cold, and no filesystem tiering carries the record of a volume the
			// filesystem does not own. Such a volume is rare (a load that never registered),
			// so the upload is too.
			if state.pending_rows.push_back(key.clone()) {
				super::cold_pass::row_queued(state, fx);
			}
			super::store::queue_cold_pass(state);
			continue;
		}
		// A maintainer log op of the volume that no pass has trimmed keeps its replicas.
		if super::maintainer_log::names_volume(state, &key) {
			continue;
		}
		// Each member's step ends in an operation that names this volume: its trim, or the
		// forget batch carrying it. A full forget window parks it in `window_waiting`, and
		// `UnloadRetry` names it after a refused forget, so it is not evaluated again until
		// one of those finishes.
		let remaining = state.unloading[&key]
			.remaining
			.iter()
			.cloned()
			.collect::<Vec<_>>();
		for worker in remaining {
			let released = match &state.unloading[&key].reason {
				UnloadReason::Released { client, .. } => Some(
					state.loaded_volumes[&key]
						.members
						.get(&worker)
						.and_then(|replica| replica.claimed_client.clone())
						.unwrap_or_else(|| client.clone()),
				),
				_ => None,
			};
			let counts = state.operations.index().counts;
			let requests = (counts.forget_volume + counts.forget_orphan) as usize + batches.len();
			if (released.is_some() || !batches.contains_key(&worker))
				&& requests >= state.config.tuning().max_forgets_in_flight as usize
			{
				// Other members' open batches may still fill; without any, nothing can.
				state.unload_drive.window_waiting.insert(key.clone());
				if batches.is_empty() {
					window_full_at = Some(position);
					break 'keys;
				}
				continue;
			}
			if state.unloading[&key].waiting.contains(&worker)
				|| state
					.operations
					.index()
					.forgets
					.contains_key(&(key.clone(), worker.clone()))
			{
				continue;
			}

			if state.loaded_volumes[&key]
				.members
				.get(&worker)
				.is_some_and(|r| r.cold.volume_seq < row.cold.volume_seq)
			{
				// An in-flight trim of this member already carries the request, and its
				// completion marks this volume again.
				let trimming = state
					.operations
					.index()
					.trims
					.contains_key(&(key.clone(), worker.clone()));
				if !trimming {
					cold::trim(state, key.clone(), worker, fx);
				}
				continue;
			}
			if let Some(client) = released {
				let op_id = state.begin(
					Operation::ForgetVolumes {
						worker: worker.clone(),
						volumes: vec![(key.clone(), row.cold.clone())],
						detached: BTreeSet::new(),
					},
					fx,
				);
				fx.execute(MaintainerEffect::ReplicaRequest {
					op_id,
					worker_id: worker,
					request: StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
						observation: None,
						header: wire::ReplicaRequestHeader::GrantClientRequestHeader(
							wire::GrantClientRequestHeader {
								volume_log_key: key.0.clone(),
								tablet_grant: state.grant(),
								client,
							},
						),
						expected_appended: Some(row.cold.clone()),
						// The granted client header is the claim this forget checks.
						expected_claim: None,
					}),
				});
				continue;
			}
			let batch = batches.entry(worker.clone()).or_default();
			batch.push((key.clone(), row.cold.clone()));
			if batch.len() >= batch_limit {
				let volumes = batches.remove(&worker).expect("full batch");
				send_forgets(state, worker, volumes, fx);
			}
		}
	}
	// Volumes after a full forget window were not evaluated; a finished forget marks them.
	if let Some(position) = window_full_at {
		state
			.unload_drive
			.window_waiting
			.extend(keys[position + 1..].iter().cloned());
	}
	for (worker, volumes) in batches {
		send_forgets(state, worker, volumes, fx);
	}
}

/// Sends one batch of conditional forgets to a member; `forgotten_batch` handles its reply.
///
/// Each forget expects the claim this maintainer last observed on the member. A claim
/// installed after that makes the shard keep the replica and answer with its state, so a
/// forget that arrives late cannot drop a member whose load installed a claim, even when its
/// answer never reaches this maintainer. A load installs its open's claim only while the
/// shard sees a stable replica set. Otherwise a late forget can still drop the member before
/// the opener claims it, and member repair reloads it once enough members are in sync.
fn send_forgets(
	state: &mut MaintainerState,
	worker: WorkerId,
	volumes: Vec<(VolumeLogKey, VolumeLogPosition)>,
	fx: &mut Effects,
) {
	let request = wire::ForgetVolumesRequest {
		tablet_grant: state.grant(),
		volumes: volumes
			.iter()
			.map(|(key, cold)| {
				let expected_claim = state
					.loaded_volumes
					.get(key)
					.and_then(|volume| volume.members.get(&worker))
					.and_then(|replica| replica.claimed_client.clone());
				wire::ForgetVolume {
					volume_log_key: key.0.clone(),
					expected_appended: cold.clone(),
					expected_claim,
				}
			})
			.collect(),
	};
	let op_id = state.begin(
		Operation::ForgetVolumes {
			worker: worker.clone(),
			volumes,
			detached: BTreeSet::new(),
		},
		fx,
	);
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: worker,
		request: StorageWorkerRequest::ForgetVolumesRequest(request),
	});
}

/// Splits a batched forget reply into one result per volume, in request order. A failed
/// request, a whole-batch rejection or a reply that does not answer every volume gives each
/// volume that same failure, which takes the ordinary retry path. A volume whose close was
/// cancelled while the batch was in flight takes [`late_forget`] instead.
pub(crate) fn forgotten_batch(
	state: &mut MaintainerState,
	worker: WorkerId,
	volumes: Vec<(VolumeLogKey, VolumeLogPosition)>,
	detached: BTreeSet<VolumeLogKey>,
	result: Result<StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	let results: Vec<Result<StorageWorkerResponse, String>> = match result {
		Ok(response @ StorageWorkerResponse::ReplicaStateResponse(_)) if volumes.len() == 1 => {
			vec![Ok(response)]
		}
		Ok(StorageWorkerResponse::ForgetVolumesResponse(reply))
			if reply.results.len() == volumes.len() =>
		{
			reply
				.results
				.into_iter()
				.map(|result| match result {
					wire::ForgetVolumeResult::ReplicaStateResponse(response) => {
						Ok(StorageWorkerResponse::ReplicaStateResponse(response))
					}
					wire::ForgetVolumeResult::RejectResponse(reason) => {
						Ok(StorageWorkerResponse::RejectResponse(reason))
					}
				})
				.collect()
		}
		Ok(StorageWorkerResponse::RejectResponse(reason)) => volumes
			.iter()
			.map(|_| Ok(StorageWorkerResponse::RejectResponse(reason.clone())))
			.collect(),
		Ok(_) => volumes
			.iter()
			.map(|_| Err("unexpected batched forget reply".to_string()))
			.collect(),
		Err(error) => volumes.iter().map(|_| Err(error.clone())).collect(),
	};
	for ((key, cold), result) in volumes.into_iter().zip(results) {
		if detached.contains(&key) {
			late_forget(state, key, worker.clone(), cold, result, fx);
		} else {
			forgotten(state, key, worker.clone(), cold, result, fx);
		}
	}
}

/// Whether in-flight lifecycle or store work of this incarnation must finish before it may
/// leave memory: a replica load, a filesystem load or apply, or a flush or cold write pass
/// carrying its record.
fn store_busy(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	let index = state.operations.index();
	index.loads.contains_key(key)
		|| index.filesystem_loads.contains_key(key)
		|| index.filesystem_applies.contains_key(key)
		|| index.flush_records.contains_key(key)
		|| index.pass_rows.contains_key(key)
}

/// Whether a forget of any member of this incarnation is in flight.
fn forgetting(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.operations
		.index()
		.forgets
		.range(super::operations::workers_of(key))
		.next()
		.is_some()
}

/// A closed incarnation needs no voting replicas while its remaining cleanup runs.
/// Only positive quorum forgets establish this fact; absence and lost disks do not.
/// Reopening removes the unload and therefore invalidates this certificate.
pub(crate) fn closed_through<'a>(
	state: &'a MaintainerState,
	key: &VolumeLogKey,
) -> Option<&'a VolumeLogPosition> {
	let proof = state.unloading.get(key)?.closed.as_ref()?;
	let volume = state.loaded_volumes.get(key)?;
	let row = volume.stored_row.as_ref()?;
	(row.status == FsVolumeStatus::Live
		&& row.volume_incarnation == key.0.volume_incarnation
		&& (row.cold == proof.cold || row.cold.volume_seq > proof.cold.volume_seq)
		&& super::filesystem::known_committed(state, key) <= row.cold.volume_seq
		// A member that learned of a commit past cold from a reopened writer disproves the
		// close even before a quorum of claimed tails proves that commit here.
		&& volume
			.members
			.values()
			.all(|replica| replica.known_committed_volume_seq <= row.cold.volume_seq))
	.then_some(&row.cold)
}

/// Conditional failure preserves newer entries; only confirmed forgotten members leave the loaded map.
pub(crate) fn forgotten(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	expected: VolumeLogPosition,
	result: Result<StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	if !state.unloading.contains_key(&key) {
		return;
	}
	let removed = match result {
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::NotClaimed(response)))
			if matches!(state.unloading[&key].reason, UnloadReason::Released { .. }) =>
		{
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["refused"])
				.inc();
			let UnloadReason::Released { client, holder } = &state.unloading[&key].reason else {
				unreachable!()
			};
			if response
				.state
				.claimed_client
				.as_ref()
				.is_some_and(|observed| observed.client_epoch <= client.client_epoch)
				&& response.state.claimed_holder.as_ref() == Some(holder)
			{
				// An unobserved older claim of the same released process is still in
				// scope. Retain existing close confirmations and retry its exact claim.
				cold::observe(state, &key, worker.clone(), response.state);
				false
			} else {
				cancel(state, &key, fx);
				return;
			}
		}
		Ok(StorageWorkerResponse::RejectResponse(
			RejectResponse::StaleClientEpoch(_) | RejectResponse::ClientConflict(_),
		)) if matches!(state.unloading[&key].reason, UnloadReason::Released { .. }) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["refused"])
				.inc();
			// Never adopt a replacement writer's claim and retry the old close against it.
			cancel(state, &key, fx);
			return;
		}
		Ok(StorageWorkerResponse::ReplicaStateResponse(response)) => {
			let removed = response.state.status != VolumeLogReplicaStatus::Deleted
				&& response.state.appended == expected
				&& response.state.cold == expected;
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&[match removed {
					true => "forgotten",
					false => "mismatch",
				}])
				.inc();
			if removed
				&& response.state.status == VolumeLogReplicaStatus::InSync
				&& !response.state.lost_entries
			{
				state
					.unloading
					.get_mut(&key)
					.expect("current unload")
					.confirmed
					.insert(worker.clone());
			}
			removed
		}
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::VolumeNotLoaded)) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["not_loaded"])
				.inc();
			true
		}
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::AppendedMismatch(response))) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&[refusal_label(&response.state, &expected)])
				.inc();
			cold::observe(state, &key, worker.clone(), response.state);
			false
		}
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::VolumeLogReplicaDeleted(
			response,
		))) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["deleted"])
				.inc();
			cold::observe(state, &key, worker.clone(), response.state);
			cancel_key(state, &key.0.volume_key, fx);
			return;
		}
		// The member answered and refused, so the forget did not apply.
		Ok(StorageWorkerResponse::RejectResponse(_)) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["refused"])
				.inc();
			false
		}
		// No answer, or one that does not say what happened: the forget may still apply. The
		// retry sends the same conditional forget, and an open that cancels this close meanwhile
		// counts the member only from its own fresh answer.
		Ok(_) | Err(_) => {
			crate::metrics::TABLET_UNLOAD_FORGETS
				.with_label_values(&["failed"])
				.inc();
			false
		}
	};
	if !removed {
		state
			.unloading
			.get_mut(&key)
			.expect("current unload")
			.waiting
			.insert(worker);
		fx.arm_timer_after(
			MaintainerTimer::UnloadRetry(key),
			state.config.tuning().retry_ms,
		);
		return;
	}
	// Capture the quorum while it still names the assignment that established
	// the close. Later Learning/Joint transitions do not undo a closed history.
	let unload = &state.unloading[&key];
	if unload.closed.is_none()
		&& super::membership::quorum(&state.tablet.replica_set, |w| unload.confirmed.contains(w))
		&& let Some(row) = state
			.loaded_volumes
			.get(&key)
			.and_then(|v| v.stored_row.as_ref())
		&& row.status == FsVolumeStatus::Live
		&& row.volume_incarnation == key.0.volume_incarnation
		&& (row.cold == expected || row.cold.volume_seq > expected.volume_seq)
	{
		state.unloading.get_mut(&key).unwrap().closed = Some(ColdClose {
			cold: row.cold.clone(),
		});
	}
	if let Some(volume) = state.loaded_volumes.get_mut(&key) {
		volume.members.remove(&worker);
	}
	super::catch_up::mark(state, &key);
	let unload = state.unloading.get_mut(&key).expect("current unload");
	unload.remaining.remove(&worker);
	if !unload.remaining.is_empty() {
		return;
	}
	finish(state, key, fx);
}

/// Keeps every unload waiting for exactly the current members after a membership change.
/// A joining member may receive a copy of the volume, so it must forget the volume too. A
/// retired member's replica holds nothing the new set needs, since promotion proved the new
/// set covers every acknowledged write, and a worker that left for good never answers its
/// forget, so waiting for it would keep the volume loaded and its forget retrying forever.
/// An unload left with no member to wait for had every current member forget the volume at
/// its covering row, so it finishes as `forgotten` would.
pub(crate) fn members_changed(
	state: &mut MaintainerState,
	previous: &[WorkerId],
	fx: &mut Effects,
) {
	let members = state.members();
	let joined: Vec<WorkerId> = members
		.iter()
		.filter(|worker| !previous.contains(worker))
		.cloned()
		.collect();
	let changed: Vec<VolumeLogKey> = state
		.unloading
		.iter()
		.filter(|(_, unload)| {
			!joined.is_empty() || unload.remaining.iter().any(|w| !members.contains(w))
		})
		.map(|(key, _)| key.clone())
		.collect();
	for key in changed {
		let unload = state.unloading.get_mut(&key).expect("current unload");
		unload.remaining.extend(joined.iter().cloned());
		unload.remaining.retain(|worker| members.contains(worker));
		unload.waiting.retain(|worker| members.contains(worker));
		let finished = unload.remaining.is_empty();
		if let Some(volume) = state.loaded_volumes.get_mut(&key) {
			volume.members.retain(|worker, _| members.contains(worker));
		}
		super::catch_up::mark(state, &key);
		state.unload_drive.dirty.insert(key.clone());
		if finished {
			finish(state, key, fx);
		}
	}
}

/// Ends the unload of `key` once no member it waits for still holds the volume.
fn finish(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	// The durable row is now the only source for a later open; stale observations cannot revive this map entry.
	if let Some(unload) = state.unloading.remove(&key) {
		fx.execute(MaintainerEffect::RecordUnloadDuration {
			duration_ms: state.now_boot_ms.saturating_sub(unload.began_boot_ms),
		});
	}
	state.remove_loaded_volume(&key);
	super::filesystem::forget(state, &key, fx);
	state.pending_rows.remove(&key);
	state.trim_ready.retain(|(k, _)| k != &key);
	state.trim_backoff.retain(|(k, _)| k != &key);
	state.pending_trims.remove(&key);
	fx.cancel_timer(MaintainerTimer::UnloadRetry(key.clone()));
	fx.cancel_timer(MaintainerTimer::UnloadRecoveryRetry(key));
}








