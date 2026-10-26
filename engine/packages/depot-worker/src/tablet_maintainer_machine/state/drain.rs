//! Applies an incarnation's final log tail to the store before a delete or restore stops
//! its apply work.
//!
//! A holder's checkpoint is answered once its entry commits, before any flush. If the maintainer
//! restarts, the entry may live only in the log, and the replicas may not even know it committed.
//! Delete and restore then drop queued apply work and read only the store, so without this step
//! such an entry is lost. The drain claims the incarnation's replicas at a new client epoch, which
//! fences every writer. The longest log a claim quorum holds is the final history, exactly what a
//! recovery would commit. The drain applies through that position from a replica holding it, and
//! finishes; nothing is written to cold for it. A delete's tail then stays in the tombstoned
//! replicas until the deleted record's ordinary pass covers it. A restore's tail stays in the
//! replaced incarnation's log, which is not trimmed until a pass covers the restore's publication.
//! Only then does the caller quiesce. When nothing above cold is unapplied, the drain finishes as
//! soon as the claim quorum answers.

use super::{MaintainerState, Operation, membership};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest as Request,
		StorageWorkerResponse as Response,
	},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One incarnation's drain. The owner (a delete or a restore) starts it and waits for
/// `finished` before it quiesces the incarnation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Drain {
	/// The fencing identity, above every epoch observed for the incarnation.
	pub client: ClientIdentity,
	pub members: BTreeMap<WorkerId, Member>,
	pub phase: Phase,
	/// The owner's last known cold position, used when the store row is not loaded.
	pub cold_floor: Option<VolumeLogPosition>,
	/// The tail the drain applied, once it is done; the owner's record covers it.
	pub applied_through: Option<VolumeLogPosition>,
	/// Whether the tail came from a recovered delete's tombstones rather than a claim quorum.
	/// Claiming a tombstone reports it absent, so such a drain never claims: it waits for a
	/// member holding the tail, re-driven by its retry deadline.
	pub from_tombstones: bool,
	pub started_boot_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Member {
	/// The next claim has not been sent.
	Ready,
	/// The correlated claim operation owns the attempt and its deadline.
	Running,
	/// The last claim failed; the retry alarm makes it ready again.
	Waiting,
	/// This replica is claimed for `client` and holds the log through `appended`.
	Claimed { appended: VolumeLogPosition },
	/// This member answered that it holds no live log for the incarnation, so it holds no
	/// unapplied entry. Only an actual not-loaded or deleted reply sets it; a member that has
	/// not answered never counts.
	Absent,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Phase {
	/// Members are claiming; the tail is not fixed yet.
	Claiming,
	/// A claim quorum fixed the final tail. Apply treats entries through it as committed, and
	/// the drain waits until apply reaches it.
	Applying { through: VolumeLogPosition },
	/// Everything of the incarnation is in the store.
	Done,
}

/// Starts draining `key` with the owner's fencing identity, unless a drain already runs. It
/// claims every current member that may hold the incarnation's log, including one this tablet
/// has not heard from since it opened: an unlisted replica may hold the only copy of a
/// committed entry. A member counts as absent only when its own complete listing omitted the
/// incarnation.
pub(crate) fn start(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	client: ClientIdentity,
	cold_floor: Option<VolumeLogPosition>,
) {
	if state.drains.contains_key(key) {
		return;
	}
	let observed = state.loaded_volumes.get(key);
	let members = state
		.members()
		.into_iter()
		.map(|worker| {
			let listed_without = state.listed.contains(&worker)
				&& observed.is_none_or(|volume| !volume.members.contains_key(&worker));
			let member = if listed_without {
				Member::Absent
			} else {
				Member::Ready
			};
			(worker, member)
		})
		.collect();
	state.drains.insert(
		key.clone(),
		Drain {
			client,
			members,
			phase: Phase::Claiming,
			cold_floor,
			applied_through: None,
			from_tombstones: false,
			started_boot_ms: state.now_boot_ms,
		},
	);
}

/// Starts applying the tail a recovered delete found on `key`'s tombstoned replicas. The
/// tombstones fence every writer already, so nothing is claimed: the tail is the longest log
/// they hold, which an earlier drain fixed and applied before the maintainer moved. Its rows
/// may exist only in that earlier lifetime's lost memtable, and the deleted record's pass must
/// cover them before the sweep, or a checkpoint or fork hold kept only in the tail is lost.
pub(crate) fn start_tail(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	client: ClientIdentity,
	through: VolumeLogPosition,
	fx: &mut Effects,
) {
	if state.drains.contains_key(key) {
		return;
	}
	let members = state
		.members()
		.into_iter()
		.map(|worker| (worker, Member::Absent))
		.collect();
	crate::metrics::DELETE_DRAINS_APPLIED.inc();
	state.drains.insert(
		key.clone(),
		Drain {
			client,
			members,
			phase: Phase::Applying { through },
			cold_floor: None,
			applied_through: None,
			from_tombstones: true,
			started_boot_ms: state.now_boot_ms,
		},
	);
	fx.arm_timer_after(
		MaintainerTimer::DeleteRetry(key.clone()),
		state.config.tuning().retry_ms,
	);
}

/// A fencing identity above every epoch observed for `key`, issued under this lifetime's
/// grant, or `None` while none can be issued.
pub(crate) fn fencing_client(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	floor: u64,
	incarnation: [u8; 16],
) -> Option<ClientIdentity> {
	let floor = state
		.loaded_volumes
		.get(key)
		.into_iter()
		.flat_map(|volume| volume.members.values())
		.fold(floor, |floor, replica| {
			floor
				.max(replica.appended.client_epoch)
				.max(replica.cold.client_epoch)
				.max(
					replica
						.claimed_client
						.as_ref()
						.map_or(0, |client| client.client_epoch),
				)
		});
	let epoch = super::claim_epochs::issue(state, floor.checked_add(1)?)?;
	Some(ClientIdentity {
		client_epoch: epoch,
		client_incarnation: incarnation,
	})
}

/// Whether the drain of `key` has applied the whole tail.
pub(crate) fn finished(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.drains
		.get(key)
		.is_some_and(|drain| drain.phase == Phase::Done)
}

/// Whether a drain still runs for `key`, so its owner must wait before cleanup.
pub(crate) fn active(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.drains
		.get(key)
		.is_some_and(|drain| drain.phase != Phase::Done)
}

/// Whether a drain of `key` is kept, running or finished. Its owner keeps a finished drain
/// until the restore's publication is logged or the delete adopts it, and until then the
/// incarnation's applied position may lie in a tail no quorum holds.
pub(crate) fn held(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state.drains.contains_key(key)
}

/// Whether a drain is applying a fixed tail of `key`, so cleanup gates must let apply and
/// flush work through. While claiming, the gates stay closed as for any retiring volume.
pub(crate) fn applying(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	through(state, key).is_some()
}

/// Whether a drain applies the tail of the incarnation `scope` names, by its hash, name and
/// incarnation. Drains are few, so the scan is bounded.
pub(crate) fn applying_scope(
	state: &MaintainerState,
	scope: &depot_fs_core::keys::slatedb::Scope,
) -> bool {
	state.drains.iter().any(|(key, drain)| {
		matches!(drain.phase, Phase::Applying { .. })
			&& depot_types::partition_key_hash(&key.0.volume_key) == scope.hash
			&& key.0.volume_key.name.as_deref().map(|name| name.to_vec()) == scope.name
			&& key.0.volume_incarnation == *scope.incarnation.as_bytes()
	})
}

/// Whether a recovered delete's drain applies the tail `key`'s tombstones keep.
pub(crate) fn applying_from_tombstones(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.drains
		.get(key)
		.is_some_and(|drain| drain.from_tombstones && matches!(drain.phase, Phase::Applying { .. }))
}

/// The final tail apply may treat as committed on a replica holding exactly it.
pub(crate) fn through<'a>(
	state: &'a MaintainerState,
	key: &VolumeLogKey,
) -> Option<&'a VolumeLogPosition> {
	match &state.drains.get(key)?.phase {
		Phase::Applying { through } => Some(through),
		Phase::Claiming | Phase::Done => None,
	}
}

/// Whether a replica appended through `appended` holds the fixed tail `through`: the same
/// epoch, at least as long. Entries it holds past the tail at that epoch never committed.
pub(crate) fn holds_through(appended: &VolumeLogPosition, through: &VolumeLogPosition) -> bool {
	appended.client_epoch == through.client_epoch && appended.volume_seq >= through.volume_seq
}

/// Removes the drain once its owner no longer needs it. A sweep of the incarnation that
/// waited for the drain's tail may run now.
pub(crate) fn forget(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if state.drains.remove(key).is_some() {
		super::pending_work::release_held_of(state, &key.0.volume_key, fx);
	}
}

/// Sends ready claims, fixes the tail once a claim quorum answers, and finishes once the
/// store holds the tail.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.drains.is_empty() {
		return;
	}
	// A drain whose delete or restore ended is no longer needed. The deletion drive adopts a
	// finished drain of a delete.
	let unneeded = state
		.drains
		.keys()
		.filter(|key| {
			!state.deleting.contains_key(*key) && !super::forks::restores(&state.forks, key)
		})
		.cloned()
		.collect::<Vec<_>>();
	for key in unneeded {
		forget(state, &key, fx);
	}
	let keys = state.drains.keys().cloned().collect::<Vec<_>>();
	for key in keys {
		// A drain that lost its tail claims again in this same pass, and a tail fixed in this
		// pass starts applying in it too.
		apply(state, &key);
		send_ready(state, &key, fx);
		fix_tail(state, &key);
		apply(state, &key);
	}
}

fn send_ready(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(drain) = state.drains.get(key) else {
		return;
	};
	if drain.phase != Phase::Claiming {
		return;
	}
	let client = drain.client.clone();
	// Claims share the delete request window with replica deletions.
	let available = (state.config.tuning().max_delete_requests as usize)
		.saturating_sub(state.operations.index().counts.delete_replica as usize);
	let ready = drain
		.members
		.iter()
		.filter(|(_, member)| **member == Member::Ready)
		.map(|(worker, _)| worker.clone())
		.take(available)
		.collect::<Vec<_>>();
	for worker in ready {
		let request = Request::ClaimRequest(wire::ClaimRequest {
			header: wire::ReplicaRequestHeader::ClientRequestHeader(wire::ClientRequestHeader {
				volume_log_key: key.0.clone(),
				client: client.clone(),
				tablet_replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
			}),
			holder: None,
		});
		if let Some(drain) = state.drains.get_mut(key) {
			drain.members.insert(worker.clone(), Member::Running);
		}
		let op_id = state.begin(
			Operation::DrainClaim {
				key: key.clone(),
				worker: worker.clone(),
			},
			fx,
		);
		fx.execute(MaintainerEffect::ReplicaRequest {
			op_id,
			worker_id: worker,
			request,
		});
	}
}

/// The cold position the store holds for `key`, from the apply state or the stored row.
fn stored_cold(state: &MaintainerState, key: &VolumeLogKey) -> Option<VolumeLogPosition> {
	if let Some(volume) = state.filesystem.volumes.get(key) {
		return volume.record_persisted.then(|| volume.record.cold.clone());
	}
	state
		.loaded_volumes
		.get(key)?
		.stored_row
		.as_ref()
		.map(|row| row.cold.clone())
}

fn fix_tail(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(drain) = state.drains.get(key) else {
		return;
	};
	if drain.phase != Phase::Claiming {
		return;
	}
	let answered = |worker: &WorkerId| {
		matches!(
			drain.members.get(worker),
			Some(Member::Claimed { .. } | Member::Absent)
		)
	};
	if !membership::quorum(&state.tablet.replica_set, answered) {
		return;
	}
	let tail = drain
		.members
		.values()
		.filter_map(|member| match member {
			Member::Claimed { appended } => Some(appended.clone()),
			Member::Ready | Member::Running | Member::Waiting | Member::Absent => None,
		})
		.max_by_key(|appended| (appended.client_epoch, appended.volume_seq));
	let cold = stored_cold(state, key).or_else(|| drain.cold_floor.clone());
	// An incarnation whose stored record is lost has nothing to apply its tail into; the tail
	// stays in the replicas until the delete forgets them. See `filesystem::record_missing`.
	let phase = if super::filesystem::is_record_missing(state, key) {
		Phase::Done
	} else {
		match (tail, cold) {
			(None, _) => Phase::Done,
			(Some(tail), Some(cold)) if tail.volume_seq <= cold.volume_seq => Phase::Done,
			(Some(tail), _) => {
				crate::metrics::DELETE_DRAINS_APPLIED.inc();
				Phase::Applying { through: tail }
			}
		}
	};
	let drain = state.drains.get_mut(key).expect("checked drain");
	drain.phase = phase;
	if drain.phase == Phase::Done {
		observe_finished(drain, state.now_boot_ms);
	}
}

fn apply(state: &mut MaintainerState, key: &VolumeLogKey) {
	let Some(through) = through(state, key).cloned() else {
		return;
	};
	// The incarnation was quarantined after this drain fixed its tail (its record lookup
	// answered after the delete activated): the tail cannot be applied, so the drain ends.
	if super::filesystem::is_record_missing(state, key) {
		let now = state.now_boot_ms;
		let drain = state.drains.get_mut(key).expect("applying drain");
		drain.phase = Phase::Done;
		observe_finished(drain, now);
		return;
	}
	// Apply reads the tail from a replica that holds it at its epoch: one holding exactly it,
	// or a longer one whose entries past the tail, at the same epoch, were never committed
	// and are not applied. If none does any longer, for example after a repair truncated it,
	// claim again at a higher epoch and fix a new tail. A tombstoned replica keeps its tail,
	// so it is a source for a delete's drain.
	let holds_tail = state.loaded_volumes.get(key).is_some_and(|volume| {
		volume.members.values().any(|replica| {
			use rivet_data::generated::shared_depot_types_v1::VolumeLogReplicaStatus as Status;
			holds_through(&replica.appended, &through)
				&& matches!(replica.status, Status::InSync | Status::Deleted)
		})
	});
	if !holds_tail {
		if state.drains[key].from_tombstones {
			// No observed member holds the tail now. The retry deadline loads it again
			// once one does; claiming would report the tombstones absent and drop the tail.
			return;
		}
		// The new claim is issued like any other, so no other client is handed its epoch.
		// Without one, the next drive asks again.
		let current = state.drains[key].client.clone();
		let Some(client) =
			fencing_client(state, key, current.client_epoch, current.client_incarnation)
		else {
			return;
		};
		let drain = state.drains.get_mut(key).expect("applying drain");
		drain.client = client;
		for member in drain.members.values_mut() {
			*member = Member::Ready;
		}
		drain.phase = Phase::Claiming;
		return;
	}
	super::filesystem::load(state, key.clone());
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return;
	};
	if volume.applied.volume_seq >= through.volume_seq {
		let now = state.now_boot_ms;
		let drain = state.drains.get_mut(key).expect("applying drain");
		drain.phase = Phase::Done;
		drain.applied_through = Some(through);
		observe_finished(drain, now);
		return;
	}
	super::filesystem::mark_candidate(state, key);
}

fn observe_finished(drain: &Drain, now_boot_ms: u64) {
	let elapsed_ms = now_boot_ms.saturating_sub(drain.started_boot_ms);
	crate::metrics::DELETE_DRAIN_DURATION.observe(elapsed_ms as f64 / 1000.0);
}

/// Records one claim reply. A higher epoch elsewhere raises the fencing epoch for the retry.
pub(crate) fn claimed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	let Some(drain) = state.drains.get_mut(&key) else {
		// A delete that adopted this drain owns its outstanding claims.
		if state.deleting.contains_key(&key) {
			super::deletion::replica(
				state,
				key,
				worker,
				super::deletion::DeleteAction::Claim,
				result,
				fx,
			);
		}
		return;
	};
	if drain.members.get(&worker) != Some(&Member::Running) {
		return;
	}
	let next = match &result {
		Ok(Response::ReplicaStateResponse(response))
			if response.state.claimed_client.as_ref() == Some(&drain.client) =>
		{
			Member::Claimed {
				appended: response.state.appended.clone(),
			}
		}
		Ok(Response::RejectResponse(
			RejectResponse::VolumeNotLoaded | RejectResponse::VolumeLogReplicaDeleted(_),
		)) => Member::Absent,
		Ok(Response::RejectResponse(
			RejectResponse::StaleClientEpoch(wire::StaleClientEpoch { state: observed })
			| RejectResponse::ClientConflict(wire::ClientConflict { state: observed }),
		)) => {
			let floor = observed
				.cold
				.client_epoch
				.max(observed.appended.client_epoch)
				.max(
					observed
						.claimed_client
						.as_ref()
						.map_or(0, |client| client.client_epoch),
				);
			match floor.checked_add(1) {
				Some(epoch) => {
					// A raised epoch invalidates earlier claims; every member claims again.
					if epoch > drain.client.client_epoch {
						drain.client.client_epoch = epoch;
						for member in drain.members.values_mut() {
							if matches!(member, Member::Claimed { .. }) {
								*member = Member::Ready;
							}
						}
					}
					Member::Waiting
				}
				None => {
					super::fence(state, fx);
					return;
				}
			}
		}
		Ok(_) | Err(_) => Member::Waiting,
	};
	if next == Member::Waiting {
		fx.arm_timer_after(
			MaintainerTimer::DeleteRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
	}
	drain.members.insert(worker.clone(), next);
	if let Ok(Response::ReplicaStateResponse(response)) = result
		&& let Some(volume) = state.loaded_volumes.get_mut(&key)
	{
		volume.members.insert(worker, response.state);
		// A claim reply is a member observation, so it can make committed entries readable.
		super::filesystem::member_observed(state, &key);
	}
}

/// How long a tail from tombstones may wait before each re-drive logs a warning.
const TAIL_WARN_MS: u64 = 10_000;

/// The retry alarm releases failed claims. For a tail from tombstones, it is the deadline
/// that re-drives the apply while the tail is not in the store, and arms itself again.
pub(crate) fn retry(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(drain) = state.drains.get_mut(key) else {
		return;
	};
	for member in drain.members.values_mut() {
		if *member == Member::Waiting {
			*member = Member::Ready;
		}
	}
	if !drain.from_tombstones || !matches!(drain.phase, Phase::Applying { .. }) {
		return;
	}
	let waited_ms = state.now_boot_ms.saturating_sub(drain.started_boot_ms);
	if waited_ms >= TAIL_WARN_MS {
		tracing::warn!(
			?key,
			waited_ms,
			"recovered delete tail is not applied yet; a member holding it may be unreachable"
		);
	}
	super::filesystem::load(state, key.clone());
	super::filesystem::mark_candidate(state, key);
	let backoff_ms = state
		.config
		.tuning()
		.retry_ms
		.saturating_mul(8)
		.max(waited_ms.min(60_000));
	fx.arm_timer_after(MaintainerTimer::DeleteRetry(key.clone()), backoff_ms);
}
