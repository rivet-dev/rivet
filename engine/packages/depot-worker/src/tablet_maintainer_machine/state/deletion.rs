//! A delete acknowledges replica tombstones before cleaning filesystem rows and forgetting replicas.
//!
//! Every incarnation has its own record row, so each delete stores its own Deleted record and
//! sweeps its own rows, whatever other incarnations of the name do. Incarnations of one name
//! are ordered by their creation epoch (`replica_creation::newer`), which decides which one an
//! open of the name targets.

use super::{LoadedVolume, MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::{VolumeLogKey, partition_key_hash};
use rivet_data::generated::{
	depot_client_protocol_v2::{ClientResponse, VolumeRejectResponse},
	depot_storage_worker_protocol_v1::{
		self as wire, RejectResponse, StorageWorkerRequest as Request,
		StorageWorkerResponse as Response,
	},
	shared_depot_fs_v2::{FsVolumeRecord, FsVolumeStatus},
	shared_depot_types_v1::{ClientIdentity, VolumeLogReplicaStatus as Status},
};
use rivet_state_machine::Entropy;
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Deletion {
	pub caller: Option<RequestId>,
	pub step: DeleteStep,
	/// The store shows a newer incarnation of the name: a restore published over this one,
	/// or the name's pointer names a newer incarnation. This delete then reserves only its
	/// own incarnation, even when the newer one is not loaded.
	pub superseded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DeleteStep {
	/// The key is reserved while the ordered executor reads the incarnation's own record.
	/// `observed` is the tombstone a recovery found, if any.
	ReadingRow {
		observed: Option<FsVolumeRecord>,
	},
	/// Recovered maintainer log ops of the name are still to replay. A restore's publication
	/// among them writes this incarnation's record Deleted, so the delete neither reads the
	/// record nor acts until the name's replay ends, which moves it to `ReadingRow`. No store
	/// work of this delete is in flight meanwhile.
	WaitingForReplay {
		observed: Option<FsVolumeRecord>,
	},
	/// A restore published over this incarnation and wrote its record Deleted, but only a
	/// maintainer log op and the memtable hold that publication. A replay after a crash drains
	/// this incarnation's live log again before it publishes, so nothing marks its replicas
	/// until a pass dispatched after `through_dispatch` makes the publication durable; then
	/// the delete reads its record.
	WaitingForCover {
		through_dispatch: u64,
	},
	/// A live grouped incarnation whose group was forked after the last fork it stands pinned
	/// through logs its fork pin before the delete claims or drains it, so each child of those
	/// forks keeps reading it at the fork once its data would otherwise be swept. `row` is the
	/// record the delete read; nothing is claimed, stamped or written yet, and the incarnation
	/// is not retiring meanwhile, so it registers and applies through its pin floor under the
	/// ordinary pin path (`groups::pin_need`). A claim log of the volume waits here while its
	/// volume's delete does, so it can register for the pin's cut. Left for `ReadingRow`, which
	/// reads the record again and claims and drains from the answer, once the volume stands
	/// pinned through the group's latest fork, at the first drive after its pin applies
	/// (`pin_applied`) or at the retry deadline armed while waiting. A replay finds the logged
	/// pin in the volume's record and pins nothing again.
	Pinning {
		row: FsVolumeRecord,
	},
	Active(DeleteProgress),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct DeleteProgress {
	/// The incarnation's record as the delete writes it: Deleted once the delete reaches its
	/// first tombstone, at the cold position its drain applied.
	pub row: FsVolumeRecord,
	/// The fencing identity the delete claims live replicas with.
	pub client: ClientIdentity,
	pub members: BTreeMap<WorkerId, MemberProgress>,
	pub cold: ColdProgress,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ColdProgress {
	/// No replica tombstone has been observed yet; a client acknowledgement needs two.
	WaitingForReplica,
	/// The background row must reach cold storage before the first sweep.
	WaitingForRow,
	/// A pass staged the row in the memtable without uploading it. The tablet's next
	/// ordinary pass dispatched after `through_dispatch` makes it cold, and `row` is then
	/// the stored row. The tombstones are the delete's proof until then.
	Staged {
		through_dispatch: u64,
		row: Box<FsVolumeRecord>,
	},
	/// The row is in cold storage and the sweep step waits for its turn in the store queue.
	Ready,
	/// A read session or a fork or checkpoint hold kept the data; the sweep step retries
	/// after `retry_ms`.
	Held,
	Sweeping,
	/// The sweep queued the data's sweep in the store and released the name's pointer, which
	/// the next ordinary pass dispatched after `through_dispatch` makes durable. The tombstones
	/// are the delete's proof until then, so no replica is forgotten.
	Removed {
		through_dispatch: u64,
	},
	/// Cold metadata is gone; only unconditional replica forgets remain.
	Gone,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum MemberProgress {
	Ready(DeleteAction),
	Running,
	Waiting(DeleteAction),
	Deleted,
	Forgotten,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DeleteAction {
	Claim,
	Mark,
	Load,
	Forget,
}

/// A delete reserves the opaque key while its incarnation is the name's newest: opens and
/// other lifecycle work of the name wait for it. A delete of an incarnation a newer one
/// replaced reserves only itself.
pub(crate) fn holds_key(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	state
		.deleting
		.range(super::incarnations(key))
		.any(|(incarnation, _)| !is_replaced(state, incarnation))
}

/// Whether a newer incarnation of the name, created and acknowledged, replaced this one, so
/// its tombstones must not mask the newer incarnation's opens and reads.
pub(crate) fn is_replaced(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	state
		.deleting
		.get(key)
		.is_some_and(|delete| delete.superseded)
		|| super::replica_creation::newer(state, key).is_some()
}

/// Whether a create of `key` must wait or may draw a new incarnation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recreate {
	/// No deletion is under way and no deleted incarnation is current.
	Clear,
	/// A deletion has not reached its tombstone quorum, or the deleted incarnations are
	/// ambiguous.
	Blocked,
	/// The current incarnation is deleted at a tombstone quorum, so its delete was
	/// acknowledged: a create draws a newer incarnation at once and the old one's cleanup goes
	/// on in the background.
	Over,
}

/// Decides how a create of `key` treats the key's deleted incarnations. Only an acknowledged
/// delete, one whose tombstones reached a quorum, lets a create go past it, so a recovery that
/// finds the old incarnation's tombstones knows it was deleted before the new one existed.
pub(crate) fn recreate(state: &MaintainerState, key: &depot_types::VolumeKey) -> Recreate {
	let mut deleted = false;
	for (candidate, delete) in state.deleting.range(super::incarnations(key)) {
		if is_replaced(state, candidate) {
			continue;
		}
		let acknowledged = match &delete.step {
			DeleteStep::Active(progress) => {
				super::membership::quorum(&state.tablet.replica_set, |w| {
					matches!(progress.members.get(w), Some(MemberProgress::Deleted))
				})
			}
			DeleteStep::ReadingRow { .. }
			| DeleteStep::WaitingForReplay { .. }
			| DeleteStep::WaitingForCover { .. }
			| DeleteStep::Pinning { .. } => false,
		};
		if !acknowledged || deleted {
			return Recreate::Blocked;
		}
		deleted = true;
	}
	for (candidate, volume) in state.loaded_volumes.range(super::incarnations(key)) {
		if is_replaced(state, candidate) || state.deleting.contains_key(candidate) {
			continue;
		}
		let Some(row) = volume.row(candidate) else {
			continue;
		};
		if row.status == FsVolumeStatus::Deleted {
			// A tombstone the deletion drive has not taken up yet.
			return Recreate::Blocked;
		}
		if deleted {
			// A live incarnation beside a deleted one: let recovery sort them out first.
			return Recreate::Blocked;
		}
	}
	if deleted {
		Recreate::Over
	} else {
		Recreate::Clear
	}
}

/// A restore published a newer incarnation over `key`'s and wrote `key`'s own record Deleted.
/// The replaced incarnation runs an ordinary delete from that record: it marks its replicas,
/// stores its Deleted record, sweeps and forgets.
pub(crate) fn retire(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if state.deleting.contains_key(&key) || !state.loaded_volumes.contains_key(&key) {
		return;
	}
	let step = DeleteStep::WaitingForCover {
		through_dispatch: super::maintainer_log::wrote_uncovered(state, fx),
	};
	let reads = matches!(step, DeleteStep::ReadingRow { .. });
	state.deleting.insert(
		key.clone(),
		Deletion {
			caller: None,
			step,
			superseded: true,
		},
	);
	state.deletion_drive.dirty.insert(key.clone());
	quiesce(state, &key, fx);
	if reads {
		state
			.store_queue
			.push_back(StoreWork::ReadDeleteRow { key });
	}
}

/// A recovered tombstone needs its own stored record before other incarnations can be served.
pub(crate) fn ambiguous(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	state
		.deleting
		.range(super::incarnations(key))
		.any(|(_, delete)| {
			matches!(
				delete.step,
				DeleteStep::WaitingForReplay { .. } | DeleteStep::ReadingRow { observed: Some(_) }
			)
		})
}

/// A pending lookup has not established deletion of the current incarnation yet.
pub(crate) fn lookup_pending(state: &MaintainerState, key: &depot_types::VolumeKey) -> bool {
	state
		.deleting
		.range(super::incarnations(key))
		.any(|(_, delete)| {
			matches!(
				delete.step,
				DeleteStep::WaitingForReplay { .. } | DeleteStep::ReadingRow { .. }
			)
		})
}

pub(crate) fn pending_requests(state: &MaintainerState) -> usize {
	state
		.deleting
		.values()
		.filter(|delete| delete.caller.is_some())
		.count()
}

pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	key: VolumeLogKey,
	entropy: Entropy,
	fx: &mut Effects,
) {
	let hash = partition_key_hash(&key.0.volume_key);
	// Routing and authority precede key reservations and replica mutations.
	let obsolete_route = !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) || !state.tablet.hash_range.contains(hash);
	let lifecycle_in_flight = state.operations.index().loads.contains_key(&key)
		|| state.operations.index().forget_orphans.contains_key(&key);
	let busy = key.0.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
		|| state.request_count() >= state.config.max_requests as usize
		|| super::forks::holds_key(state, &key.0.volume_key)
		|| state.pending.contains_key(&key.0.volume_key)
		|| super::repair::holds_key(state, &key.0.volume_key)
		|| super::catch_up::holds_key(state, &key.0.volume_key)
		|| lifecycle_in_flight
		|| super::maintainer_log::replay::pending_for(state, &key.0.volume_key);
	let reason = if obsolete_route {
		Some(depot_types::client::not_maintainer())
	} else if state.phase == MaintainerPhase::Frozen {
		Some(VolumeRejectResponse::TabletSplitting)
	} else if let Some(under_repair) =
		super::operator_repair::claims::under_repair(state, &key.0.volume_key)
	{
		// The repair claim is the volume's only writer until it is released.
		Some(under_repair)
	} else if busy {
		Some(depot_types::client::overloaded())
	} else {
		None
	};
	if let Some(reason) = reason {
		super::lifecycle::reject(id, reason, fx);
		return;
	}
	if let Some(delete) = state.deleting.get_mut(&key) {
		if let DeleteStep::Active(progress) = &delete.step
			&& (matches!(progress.cold, ColdProgress::Gone)
				|| super::membership::quorum(&state.tablet.replica_set, |w| {
					matches!(progress.members.get(w), Some(MemberProgress::Deleted))
				})) {
			reply(id, fx);
		} else if delete.caller.is_none() {
			delete.caller = Some(id);
			fx.arm_timer_after(
				MaintainerTimer::DeleteRequest(key),
				state.config.tuning().operation_timeout_ms,
			);
		} else {
			super::lifecycle::reject(id, depot_types::client::overloaded(), fx);
		}
		return;
	}
	if holds_key(state, &key.0.volume_key) {
		super::lifecycle::reject(id, depot_types::client::overloaded(), fx);
		return;
	}
	if !state.loaded_volumes.contains_key(&key)
		&& state.reserves_at_least(state.config.max_loaded_volumes as usize)
	{
		super::lifecycle::reject(id, depot_types::client::overloaded(), fx);
		return;
	}

	super::unload::cancel_key(state, &key.0.volume_key, fx);
	let row = loaded_row(state, &key);
	state.deleting.insert(
		key.clone(),
		Deletion {
			caller: Some(id),
			step: DeleteStep::ReadingRow { observed: None },
			superseded: false,
		},
	);
	state.deletion_drive.dirty.insert(key.clone());
	fx.arm_timer_after(
		MaintainerTimer::DeleteRequest(key.clone()),
		state.config.tuning().operation_timeout_ms,
	);
	let known_floor = state
		.loaded_volumes
		.get(&key)
		.is_some_and(|volume| volume.creation.is_some() || volume.stored_row.is_some())
		|| state.operations.index().pass_rows.contains_key(&key);
	// A grouped incarnation already deleted without a stamp needs its pin mark, which only its
	// row lookup reads.
	let ready = |row: &FsVolumeRecord| {
		known_floor && state.fork_checked.contains(&key.0.volume_key) && !needs_pin_mark(&key, row)
	};
	if let Some(row) = row.filter(ready) {
		activate(state, key, row, None, entropy, fx);
	} else {
		state
			.store_queue
			.push_back(StoreWork::ReadDeleteRow { key });
	}
}

/// Deletion can claim a repairing replica even before it can serve client reads.
fn loaded_row(state: &MaintainerState, key: &VolumeLogKey) -> Option<FsVolumeRecord> {
	state.loaded_volumes.get(key).and_then(|volume| {
		volume.row(key).or_else(|| {
			volume
				.members
				.values()
				.next()
				.map(|replica| FsVolumeRecord {
					incarnation_epoch: super::replica_creation::known_epoch(volume).unwrap_or(0),
					recent_writes: Vec::new(),
					base_chain: Vec::new(),
					retention: None,
					partition_key: key.0.volume_key.partition_key.clone(),
					name: key.0.volume_key.name.clone(),
					volume_incarnation: key.0.volume_incarnation,
					status: FsVolumeStatus::Live,
					cold: replica.cold.clone(),
					fork_pinned_through: None,
					created_at_fork: None,
					deleted_at_fork: None,
					lazy_fork: None,
					metadata_local: false,
				})
		})
	})
}

/// Applies the lookup of the incarnation's own record. Resolving it can end the ambiguity
/// that kept other incarnations of the volume key from filesystem apply, so they are offered
/// again.
pub(crate) fn row(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	result: Result<depot_fs_core::forks::LifecycleLookup, String>,
	entropy: Entropy,
	fx: &mut Effects,
) {
	let volume_key = key.0.volume_key.clone();
	resolve_row(state, key, result, entropy, fx);
	super::filesystem::load_incarnations(state, &volume_key);
}

/// The lookup returns this incarnation's own record or none. A stored record starts the
/// delete from it; a recovered tombstone or a loaded volume without one starts it from what
/// the replicas show; with neither, the incarnation is already gone and the delete is done.
fn resolve_row(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	result: Result<depot_fs_core::forks::LifecycleLookup, String>,
	entropy: Entropy,
	fx: &mut Effects,
) {
	let Some(Deletion {
		step: DeleteStep::ReadingRow { observed },
		..
	}) = state.deleting.get(&key)
	else {
		return;
	};
	let observed = observed.clone();
	let pin_mark = result
		.as_ref()
		.ok()
		.and_then(|lookup| lookup.pinned_through);
	if result.as_ref().is_ok_and(|lookup| lookup.superseded) {
		state
			.deleting
			.get_mut(&key)
			.expect("reserved lookup")
			.superseded = true;
	}
	let Some(result) = super::fork_admission::lookup(state, &key.0.volume_key, result) else {
		let deleted = state.deleting.remove(&key).expect("reserved lookup");
		fx.cancel_timer(MaintainerTimer::DeleteRequest(key));
		if let Some(id) = deleted.caller {
			super::lifecycle::reject(id, depot_types::client::overloaded(), fx);
		}
		return;
	};

	// A recovered restore's publication of this name may write this incarnation's record
	// Deleted, so the record just read is not final until that replay ends; read it again then.
	if result.is_ok() && super::maintainer_log::replay::pending_for(state, &key.0.volume_key) {
		state
			.deleting
			.get_mut(&key)
			.expect("reserved recovery")
			.step = DeleteStep::WaitingForReplay { observed };
		return;
	}

	match result {
		Ok(Some(row)) if row.volume_incarnation != key.0.volume_incarnation => {
			tracing::error!(
				?key,
				"a lookup of one incarnation returned another's record; fencing"
			);
			super::fence(state, fx);
		}
		Ok(Some(mut row)) => {
			// A previous owner may have written farther than any surviving trim witness.
			if let Some(volume) = state.loaded_volumes.get_mut(&key) {
				volume.stored_row = Some(row.clone().into());
				if let Some(current) = volume.row(&key) {
					row.cold = current.cold;
				}
			}
			if observed.is_some() {
				row.status = FsVolumeStatus::Deleted;
			}
			activate(state, key, row, pin_mark, entropy, fx);
		}
		Ok(None) if observed.is_some() => {
			activate(
				state,
				key,
				observed.expect("recovered tombstone"),
				pin_mark,
				entropy,
				fx,
			);
		}
		Ok(None) if loaded_row(state, &key).is_some() => {
			// A create can have reached disk quorum before its background cold row exists.
			let mut row = loaded_row(state, &key).expect("loaded row checked");
			// A restore retired this incarnation and wrote its record Deleted; a pending sweep
			// that already removed that record leaves nothing to claim or drain.
			if state
				.deleting
				.get(&key)
				.is_some_and(|delete| delete.superseded)
			{
				row.status = FsVolumeStatus::Deleted;
			}
			activate(state, key, row, pin_mark, entropy, fx);
		}
		Ok(None) => {
			if let Some(id) = state.deleting.remove(&key).and_then(|delete| delete.caller) {
				reply(id, fx);
			}
			fx.cancel_timer(MaintainerTimer::DeleteRequest(key));
		}
		Err(_) => super::fence(state, fx),
	}
}

/// Stop selecting writes before publishing deletion; already dispatched store work retains its queue position.
/// `pin_mark` is the incarnation's pin mark its delete's row lookup read, if any.
fn activate(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	mut row: FsVolumeRecord,
	pin_mark: Option<u64>,
	entropy: Entropy,
	fx: &mut Effects,
) {
	// Choose one identity above every observed fence, retaining it across member retries.
	let known_deleted = row.status == FsVolumeStatus::Deleted;
	// A grouped volume records the fork it was deleted at, so a child of a later fork finds it
	// absent and a child of an earlier fork still reads it through the pin. A missing stamp
	// would read as fork 0, so the delete waits for the group's record, read meanwhile, and
	// activates after the retry delay.
	let deleted_at_fork = match row.deleted_at_fork {
		Some(stamp) => Some(stamp),
		// A delete already past its tombstones was stamped when it began, and its stamp is
		// missing only when its Deleted record never landed, or is fork 0 under a grant that
		// cannot record one. It logged its fork pin through that fork before any tombstone, so
		// the pin mark is the fork it was deleted at; the group may have forked since, and a
		// later stamp would claim the volume existed at a fork nothing pins it at.
		None if known_deleted => recovered_stamp(state, &key, &row, pin_mark),
		None => match super::groups::fork_stamp(state, &key.0.volume_key, fx) {
			Ok(stamp) => super::groups::recorded_stamp(state, &key.0.volume_key, stamp),
			Err(_) => {
				if let Some(delete) = state.deleting.get_mut(&key) {
					delete.step = DeleteStep::ReadingRow {
						observed: known_deleted.then(|| row.clone()),
					};
				}
				fx.arm_timer_after(
					MaintainerTimer::DeleteRetry(key),
					state.config.tuning().retry_ms,
				);
				return;
			}
		},
	};
	// A live grouped incarnation logs its fork pin before anything claims or drains it.
	if !known_deleted && needs_pin(state, &key, &row, deleted_at_fork) {
		start_pinning(state, key, row, fx);
		return;
	}
	let floor = state
		.loaded_volumes
		.get(&key)
		.into_iter()
		.flat_map(|volume| volume.members.values())
		.fold(row.cold.client_epoch, |floor, replica| {
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
	let Some(min) = floor.checked_add(1) else {
		super::fence(state, fx);
		return;
	};
	// The tail to apply is fixed before the delete takes its fencing epoch, which it does not
	// depend on, so a tablet that cannot tell it yet retries without consuming an epoch.
	let tail = match state.loaded_volumes.get(&key) {
		Some(volume) => {
			// A stored Deleted record already covers the tail its deletion's drain fixed, so
			// a longer log on a replica outside that drain's claim quorum was never committed
			// and is not applied. The filesystem also never registers a persisted Deleted
			// record, so a tail drain over one could not apply.
			let stored_deleted = volume
				.stored_row
				.as_ref()
				.is_some_and(|stored| stored.status == FsVolumeStatus::Deleted);
			// Without a stored record, a tombstone whose cold position moved was recorded and
			// then swept: its delete reached its last step, so no tail is left to apply.
			let swept = volume.stored_row.is_none() && row.cold.volume_seq > 0;
			if known_deleted && !stored_deleted && !swept {
				match tombstone_tail(state, &key, &row.cold) {
					TombstoneTail::Cold => None,
					TombstoneTail::Above(through) => Some(through),
					TombstoneTail::Unknown => {
						// Read the row and the members again after the retry delay.
						if let Some(delete) = state.deleting.get_mut(&key) {
							delete.step = DeleteStep::ReadingRow {
								observed: Some(row.clone()),
							};
						}
						fx.arm_timer_after(
							MaintainerTimer::DeleteRetry(key),
							state.config.tuning().retry_ms,
						);
						return;
					}
				}
			} else {
				None
			}
		}
		None => None,
	};
	// The delete's fencing epoch comes from this lifetime's grant. Without one, read the row
	// again after the retry delay and activate then.
	let Some(epoch) = super::claim_epochs::issue(state, min) else {
		if let Some(delete) = state.deleting.get_mut(&key) {
			delete.step = DeleteStep::ReadingRow {
				observed: known_deleted.then(|| row.clone()),
			};
		}
		fx.arm_timer_after(
			MaintainerTimer::DeleteRetry(key),
			state.config.tuning().retry_ms,
		);
		return;
	};
	let client = ClientIdentity {
		client_epoch: epoch,
		client_incarnation: *entropy.rng().uuid_v4().as_bytes(),
	};
	state
		.loaded_volumes
		.entry(key.clone())
		.or_insert_with(|| LoadedVolume {
			members: Default::default(),
			last_activity: state.now_boot_ms,
			creation: None,
			unacknowledged_cold_open: None,
			stored_row: Some(row.clone().into()),
			status: Default::default(),
		});
	super::track_loaded(state, &key);
	row.status = FsVolumeStatus::Deleted;
	row.deleted_at_fork = deleted_at_fork;
	let volume = &state.loaded_volumes[&key];
	// Missing members load a tombstone directly; present live replicas must be claimed first.
	let observations = volume.members.clone();
	let members = state
		.members()
		.into_iter()
		.map(|worker| {
			let progress = match observations.get(&worker) {
				Some(replica) if replica.status == Status::Deleted => MemberProgress::Deleted,
				Some(_) => MemberProgress::Ready(if known_deleted {
					DeleteAction::Mark
				} else {
					DeleteAction::Claim
				}),
				None => MemberProgress::Ready(DeleteAction::Load),
			};
			(worker, progress)
		})
		.collect();
	state.deleting.get_mut(&key).expect("reserved delete").step =
		DeleteStep::Active(DeleteProgress {
			row,
			client,
			members,
			cold: if known_deleted {
				ColdProgress::WaitingForRow
			} else {
				ColdProgress::WaitingForReplica
			},
		});

	// A live incarnation applies its final tail before its apply work stops. The members
	// wait for the drain, then claim again with the same identity. Tombstones a recovery found
	// may hold a tail above the stored cold position, which is applied the same way.
	if let Some(through) = tail {
		let DeleteStep::Active(DeleteProgress { client, .. }) = &state.deleting[&key].step else {
			unreachable!("activated above")
		};
		let client = client.clone();
		super::drain::start_tail(state, &key, client, through, fx);
		super::filesystem::load(state, key.clone());
	} else if known_deleted {
		quiesce(state, &key, fx);
	} else {
		let DeleteStep::Active(DeleteProgress { row, client, .. }) = &state.deleting[&key].step
		else {
			unreachable!("activated above")
		};
		let (client, cold) = (client.clone(), row.cold.clone());
		super::drain::start(state, &key, client, Some(cold));
	}
}

/// Whether the delete of `key`, whose record is `row`, must log its volume's fork pin before it
/// claims and drains: the volume is in a group (`latest_fork`, the group's latest fork, is set)
/// and stands pinned through an earlier fork, by its registered record when it is registered,
/// since a pin it logged in this grant is there first. A volume created at or after the latest
/// fork, and any volume outside a group, needs none and pays nothing. A claim log of a grouped
/// volume waits while its volume's delete has not claimed yet: the volume's pin may still need
/// it registered, and its own row read can finish before the volume's decides whether to pin.
/// Activating the volume's delete, its entering the step or its pin applying wakes it.
fn needs_pin(
	state: &MaintainerState,
	key: &VolumeLogKey,
	row: &FsVolumeRecord,
	latest_fork: Option<u64>,
) -> bool {
	if depot_types::is_claim_log(&key.0.volume_key).is_some() {
		let (base, _) = super::claims::base_of(key);
		// A registered claim log whose volume stands pinned through the latest fork has
		// nothing left to wait for, even while the volume's delete reads its record again.
		let pinned = state.filesystem.volumes.contains_key(key)
			&& super::groups::pin_need(state, key) == super::groups::PinNeed::None;
		if super::groups::group_of(&base).is_none() || pinned {
			return false;
		}
		return match state.deleting.get(&base).map(|delete| &delete.step) {
			Some(
				DeleteStep::ReadingRow { .. }
				| DeleteStep::WaitingForReplay { .. }
				| DeleteStep::WaitingForCover { .. }
				| DeleteStep::Pinning { .. },
			) => true,
			Some(DeleteStep::Active(_)) | None => false,
		};
	}
	let Some(latest_fork) = latest_fork else {
		return false;
	};
	let pinned_through = match state.filesystem.volumes.get(key) {
		Some(volume) => super::groups::pinned_through(&volume.record),
		None => row.fork_pinned_through.or(row.created_at_fork).unwrap_or(0),
	};
	latest_fork > pinned_through
}

/// Whether the delete of `key` starting from `row` must read the incarnation's pin mark with its
/// row before it activates: a grouped incarnation already deleted whose row carries no stamp,
/// which [`recovered_stamp`] then takes from the pin mark.
fn needs_pin_mark(key: &VolumeLogKey, row: &FsVolumeRecord) -> bool {
	row.status == FsVolumeStatus::Deleted
		&& row.deleted_at_fork.is_none()
		&& depot_types::keys::partition::forkable_group(&key.0.volume_key.partition_key).is_some()
}

/// The fork a delete recovered from its tombstones was deleted at, for a grouped volume: the
/// last fork its pin rows, stored pin mark or creation stamp cover, or fork 0 when none does.
/// An ungrouped volume has none.
fn recovered_stamp(
	state: &MaintainerState,
	key: &VolumeLogKey,
	row: &FsVolumeRecord,
	pin_mark: Option<u64>,
) -> Option<u64> {
	if depot_types::keys::partition::forkable_group(&key.0.volume_key.partition_key).is_none() {
		return None;
	}
	let stamp = pin_mark
		.max(row.fork_pinned_through)
		.max(row.created_at_fork)
		.unwrap_or(0);
	super::groups::recorded_stamp(state, &key.0.volume_key, Some(stamp))
}

/// Moves the delete of `key` to `DeleteStep::Pinning` with the record it read. An incarnation
/// the tablet does not hold loaded is loaded from that record, so it registers and its pin is
/// logged at its stored cut, which holds everything it applied.
fn start_pinning(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	row: FsVolumeRecord,
	fx: &mut Effects,
) {
	fx.arm_timer_after(
		MaintainerTimer::DeleteRetry(key.clone()),
		state.config.tuning().retry_ms,
	);
	state
		.loaded_volumes
		.entry(key.clone())
		.or_insert_with(|| LoadedVolume {
			members: Default::default(),
			last_activity: state.now_boot_ms,
			creation: None,
			unacknowledged_cold_open: None,
			stored_row: Some(row.clone().into()),
			status: Default::default(),
		});
	super::track_loaded(state, &key);
	if let Some(delete) = state.deleting.get_mut(&key) {
		delete.step = DeleteStep::Pinning { row };
	}
	state.deletion_drive.dirty.insert(key);
}

/// Drives a delete in `DeleteStep::Pinning`: registers the volume, logs its fork pin through
/// the ordinary pin path, and activates the delete once the volume stands pinned through the
/// group's latest fork. While it waits, the retry deadline armed when it entered the step, and
/// again at each firing (`pin_retry`), re-drives it, since a pin that could not be logged yet
/// (an apply or another op of the volume in flight, a group read) sends no notification of its
/// own. Repeating it with nothing changed emits nothing new.
fn drive_pin(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(Deletion {
		step: DeleteStep::Pinning { row },
		..
	}) = state.deleting.get(key)
	else {
		return;
	};
	let row = row.clone();
	let Ok(latest_fork) = super::groups::fork_stamp(state, &key.0.volume_key, fx) else {
		return;
	};
	let pinned = !needs_pin(state, key, &row, latest_fork)
		|| (state.filesystem.volumes.contains_key(key)
			&& super::groups::pin_need(state, key) == super::groups::PinNeed::None);
	// A recovery proving the volume's tail for the pin is left to join before the delete
	// claims; the finished operation marks the delete again.
	if pinned && super::repair::holds_key(state, &key.0.volume_key) {
		return;
	}
	// The delete reads its record again and activates from the answer, as any delete does,
	// so its drain starts before this pass's drain drive and is adopted when it ends.
	if pinned {
		if depot_types::is_claim_log(&key.0.volume_key).is_none() {
			crate::metrics::DELETE_FORK_PINS.inc();
		}
		fx.cancel_timer(MaintainerTimer::DeleteRetry(key.clone()));
		if let Some(delete) = state.deleting.get_mut(key) {
			delete.step = DeleteStep::ReadingRow { observed: None };
		}
		state
			.store_queue
			.push_back(StoreWork::ReadDeleteRow { key: key.clone() });
		return;
	}
	// A claim log only registers, so its volume's pin can span it, and only once its volume's
	// delete is pinning; before that it waits for the volume's decision.
	if depot_types::is_claim_log(&key.0.volume_key).is_some() {
		let (base, _) = super::claims::base_of(key);
		if pinning(state, &base) {
			super::filesystem::load(state, key.clone());
		}
		return;
	}
	if !state.filesystem.volumes.contains_key(key) {
		super::filesystem::load(state, key.clone());
		return;
	}
	match super::groups::pin_need(state, key) {
		super::groups::PinNeed::None => {}
		super::groups::PinNeed::Read => super::groups::hold_for_read(state, key, fx),
		super::groups::PinNeed::Pin | super::groups::PinNeed::Apply => {
			super::groups::pin(state, key, fx)
		}
		super::groups::PinNeed::Unsupported => super::groups::unsupported(state, key),
	}
}

/// Starts a recovery of each log of `key`'s volume whose pin floor waits for an unproven tail
/// (`RecoveryPurpose::DeletePin`). The delete's own claim, which would prove the tail, comes
/// only after the pin, and a deleted actor's holder writes nothing more, so without it the pin
/// would wait until the holder's process exits. A log that a recovery already holds, or for
/// which no `max_repairs` slot is free, waits for the next firing of the retry deadline of
/// `DeleteStep::Pinning`, which starts these recoveries rather than the drive, so no event
/// that frees a slot or ends another recovery has to wake the delete.
fn prove_unproven_tails(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	for log in super::groups::unproven_logs(state, key) {
		if super::repair::active_jobs(state) >= state.config.tuning().max_repairs as usize {
			tracing::debug!(?log, "a delete's tail proof waits for a repair slot");
			return;
		}
		if super::repair::holds_key(state, &log.0.volume_key) {
			continue;
		}
		let Some(row) = state
			.loaded_volumes
			.get(&log)
			.and_then(|volume| volume.row(&log))
		else {
			continue;
		};
		crate::metrics::DELETE_PIN_TAIL_PROOFS.inc();
		super::repair::recover(
			state,
			log,
			row,
			super::repair::RecoveryPurpose::DeletePin,
			fx,
		);
	}
}

/// The retry deadline of a delete waiting for its fork pin fired: it is armed again while the
/// delete still waits, a pin that waits for an unproven tail has the tail proven
/// (`prove_unproven_tails`), and the drive looks at the pin (`retry` marks it).
pub(crate) fn pin_retry(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	if !pinning(state, key) {
		return;
	}
	fx.arm_timer_after(
		MaintainerTimer::DeleteRetry(key.clone()),
		state.config.tuning().retry_ms,
	);
	if depot_types::is_claim_log(&key.0.volume_key).is_none() {
		prove_unproven_tails(state, key, fx);
	}
}

/// A fork pin of `volume_key`'s volume applied, or the volume registered: its deletes waiting
/// to pin look again at the next drive. A claim log's registration also wakes its volume's
/// delete.
pub(crate) fn pin_applied(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey) {
	let mut keys = pinning_incarnations(state, volume_key);
	if let Some((base, _)) = depot_types::is_claim_log(volume_key) {
		keys.extend(pinning_incarnations(state, &base));
	}
	state.deletion_drive.dirty.extend(keys);
}

/// The incarnations of `volume_key` whose deletes wait in `DeleteStep::Pinning`.
fn pinning_incarnations(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> Vec<VolumeLogKey> {
	state
		.deleting
		.range(super::incarnations(volume_key))
		.filter(|(_, delete)| matches!(delete.step, DeleteStep::Pinning { .. }))
		.map(|(key, _)| key.clone())
		.collect()
}

/// Whether the delete of `key` waits for its volume's fork pin, so the incarnation still
/// registers and applies like a live one.
pub(crate) fn pinning(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	matches!(
		state.deleting.get(key),
		Some(Deletion {
			step: DeleteStep::Pinning { .. },
			..
		})
	)
}

/// The tail a deleted incarnation's replicas hold above `cold`, if any: the shortest log
/// that a quorum of members holds at most. Every quorum includes a member holding every
/// committed entry, so the tail covers the committed history. The deletion's drain claimed a
/// quorum and took its longest log, so with that quorum observed the tail is at most what
/// the drain applied, and a longer log on a member outside it, which never committed, is not
/// applied. A member whose complete listing omitted the incarnation holds nothing. Without a
/// quorum of answers the tail is unknown, and `None` is not an answer either.
fn tombstone_tail(
	state: &MaintainerState,
	key: &VolumeLogKey,
	cold: &rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
) -> TombstoneTail {
	let observed = state.loaded_volumes.get(key).map(|volume| &volume.members);
	let held = |worker: &WorkerId| -> Option<Option<(u64, u64)>> {
		match observed.and_then(|members| members.get(worker)) {
			Some(replica) if matches!(replica.status, Status::InSync | Status::Deleted) => {
				let appended = &replica.appended;
				Some(
					(appended.volume_seq > cold.volume_seq)
						.then_some((appended.client_epoch, appended.volume_seq)),
				)
			}
			Some(_) => None,
			None if state.listed.contains(worker) => Some(None),
			None => None,
		}
	};
	let supports = |bound: Option<(u64, u64)>| {
		super::membership::quorum(&state.tablet.replica_set, |worker| match held(worker) {
			Some(None) => true,
			Some(Some(position)) => bound.is_some_and(|bound| position <= bound),
			None => false,
		})
	};
	if supports(None) {
		return TombstoneTail::Cold;
	}
	let mut candidates = observed
		.into_iter()
		.flat_map(|members| members.values())
		.filter(|replica| matches!(replica.status, Status::InSync | Status::Deleted))
		.map(|replica| replica.appended.clone())
		.filter(|appended| appended.volume_seq > cold.volume_seq)
		.collect::<Vec<_>>();
	candidates.sort_by_key(|appended| (appended.client_epoch, appended.volume_seq));
	candidates
		.into_iter()
		.find(|appended| supports(Some((appended.client_epoch, appended.volume_seq))))
		.map_or(TombstoneTail::Unknown, TombstoneTail::Above)
}

/// What a recovered delete's tombstones say about the tail above the stored cold position.
enum TombstoneTail {
	/// A quorum holds nothing above cold.
	Cold,
	/// The tail to apply before the delete continues.
	Above(rivet_data::generated::shared_depot_types_v1::VolumeLogPosition),
	/// Too few members answered to tell.
	Unknown,
}

/// Hands a finished drain's claims to the delete: members it claimed go straight to marking,
/// and a raised fencing epoch becomes the delete's own.
fn adopt_drain(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let Some(drain) = state.drains.remove(key) else {
		return;
	};
	// A discovered sweep of this incarnation waited for the tail; it may run now.
	super::pending_work::release_held_of(state, &key.0.volume_key, fx);
	if let Some(Deletion {
		step: DeleteStep::Active(progress),
		..
	}) = state.deleting.get_mut(key)
	{
		progress.client = drain.client.clone();
		// The deleted record's pass covers the tail the drain applied: the store then holds
		// every row through it, and replay of the tombstones starts above it.
		if let Some(tail) = &drain.applied_through
			&& tail.volume_seq > progress.row.cold.volume_seq
		{
			progress.row.cold = tail.clone();
		}
		for (worker, claimed) in &drain.members {
			// A member the delete would still claim or load takes the drain's claim instead.
			if !matches!(
				progress.members.get(worker),
				Some(MemberProgress::Ready(
					DeleteAction::Claim | DeleteAction::Load
				))
			) {
				continue;
			}
			// A claim still in flight answers the delete instead; see `drain::claimed`.
			let next = match claimed {
				super::drain::Member::Claimed { .. } => MemberProgress::Ready(DeleteAction::Mark),
				super::drain::Member::Running => MemberProgress::Running,
				super::drain::Member::Waiting => MemberProgress::Waiting(DeleteAction::Claim),
				super::drain::Member::Ready | super::drain::Member::Absent => continue,
			};
			progress.members.insert(worker.clone(), next);
		}
	}
	quiesce(state, key, fx);
}

/// Retire unsent work for this incarnation while leaving the active store operation in order.
pub(crate) fn quiesce(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	// Discard unsent cold work and stale status reads. The active store pass remains ordered.
	super::filesystem::quiesce(state, key, fx);
	// The incarnation's applied bytes no longer ask for a flush; the deleted record's pass
	// covers them.
	state.filesystem.set_unflushed_bytes(key, 0);
	state.pending_trims.remove(key);
	state.trim_ready.retain(|(queued, _)| queued != key);
	state.trim_backoff.retain(|(queued, _)| queued != key);
	state.pending_rows.remove(key);
	let index = state.operations.index();
	let cancelled = index
		.filesystem_reads
		.get(key)
		.into_iter()
		.flatten()
		.chain(
			index
				.trims
				.range(super::operations::workers_of(key))
				.flat_map(|(_, ids)| ids),
		)
		.copied()
		.collect::<BTreeSet<_>>();
	for id in cancelled {
		// A batched trim also carried other volumes' trims; they go back to the ready set.
		if let Some(super::Operation::TrimBatch { worker, trims }) = state.finish(id, fx) {
			for (other, _) in trims {
				if &other != key {
					state.trim_ready.insert((other, worker.clone()));
				}
			}
		}
		fx.execute(MaintainerEffect::CancelReplicaRequest { op_id: id });
	}
}

/// A client lookup can discover a Deleted row even when no shard currently holds its replica.
/// Retain only bounded cleanup work; the regular deletion driver will draw its identity and load tombstones.
pub(crate) fn observe_row(
	state: &mut MaintainerState,
	volume_key: depot_types::VolumeKey,
	row: FsVolumeRecord,
) {
	if state.phase != MaintainerPhase::Serving || row.status != FsVolumeStatus::Deleted {
		return;
	}
	let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key,
		volume_incarnation: row.volume_incarnation,
	});
	if state.loaded_volumes.contains_key(&key)
		|| state.reserves_at_least(state.config.max_loaded_volumes as usize)
	{
		return;
	}
	state.loaded_volumes.insert(
		key.clone(),
		LoadedVolume {
			members: Default::default(),
			last_activity: state.now_boot_ms,
			creation: None,
			unacknowledged_cold_open: None,
			stored_row: Some(row.into()),
			status: Default::default(),
		},
	);
	super::track_loaded(state, &key);
	observe_loaded(state, &key);
}

/// Expired callers get a retryable answer; tombstones and background cleanup retain ownership of the key.
pub(crate) fn expired(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	if let Some(id) = state
		.deleting
		.get_mut(&key)
		.and_then(|delete| delete.caller.take())
	{
		super::lifecycle::reject(id, depot_types::client::overloaded(), fx);
	}
}

/// The recovered maintainer log ops of `volume_key` are applied: its deletes that waited for
/// them read the store's row now.
pub(crate) fn replay_ended(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey) {
	let keys = state
		.deleting
		.range(super::incarnations(volume_key))
		.filter(|(_, delete)| matches!(delete.step, DeleteStep::WaitingForReplay { .. }))
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	read_rows_after_replay(state, keys);
}

/// Every recovered maintainer log op is applied: every delete that waited reads its row.
pub(crate) fn replay_ended_all(state: &mut MaintainerState) {
	let keys = state
		.deleting
		.iter()
		.filter(|(_, delete)| matches!(delete.step, DeleteStep::WaitingForReplay { .. }))
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	read_rows_after_replay(state, keys);
}

/// Moves each delete of `keys` that waits for replay to `ReadingRow` with the tombstone it
/// observed and queues one row read for it; a delete in any other step is left alone.
fn read_rows_after_replay(state: &mut MaintainerState, keys: Vec<VolumeLogKey>) {
	for key in keys {
		let Some(Deletion {
			step: DeleteStep::WaitingForReplay { observed },
			..
		}) = state.deleting.get_mut(&key)
		else {
			continue;
		};
		let observed = observed.take();
		state.deleting.get_mut(&key).expect("checked above").step =
			DeleteStep::ReadingRow { observed };
		state.deletion_drive.dirty.insert(key.clone());
		state
			.store_queue
			.push_back(StoreWork::ReadDeleteRow { key });
	}
}

/// Only the retry alarm releases failed member work; unrelated events cannot form a busy loop.
pub(crate) fn retry(state: &mut MaintainerState, key: VolumeLogKey) {
	// A delete that waited for a claim epoch reads its row again.
	if let Some(Deletion {
		step: DeleteStep::ReadingRow { .. },
		..
	}) = state.deleting.get(&key)
	{
		let work = StoreWork::ReadDeleteRow { key: key.clone() };
		let reading = state
			.operations
			.values()
			.any(|operation| matches!(operation, Operation::ReadDeleteRow { key: k } if *k == key));
		if !reading && !state.store_queue.contains(&work) {
			state.store_queue.push_back(work);
		}
		return;
	}
	match state.deleting.get_mut(&key).map(|delete| &mut delete.step) {
		Some(DeleteStep::Active(progress)) => {
			if progress.cold == ColdProgress::Held {
				progress.cold = ColdProgress::Ready;
			}
			for member in progress.members.values_mut() {
				if let MemberProgress::Waiting(action) = member {
					*member = MemberProgress::Ready(action.clone());
				}
			}
		}
		// Only the end of the name's replay moves a delete waiting for it.
		Some(DeleteStep::WaitingForReplay { .. }) => {}
		// The drive looks at the pin again.
		Some(DeleteStep::Pinning { .. }) => {
			state.deletion_drive.dirty.insert(key);
		}
		Some(DeleteStep::ReadingRow { .. }) | Some(DeleteStep::WaitingForCover { .. }) | None => {}
	}
}

/// Keep only observed tombstones in the recovery workset. Blocked candidates remain
/// until a later transition releases their pending open, repair or copy owner.
pub(crate) fn observe_loaded(state: &mut MaintainerState, key: &VolumeLogKey) {
	super::catch_up::mark(state, key);
	if state
		.loaded_volumes
		.get(key)
		.is_some_and(|volume| volume.is_deleted())
	{
		state.deletion_candidates.insert(key.clone());
	}
}

/// Restore deletion from any single tombstone, then share one bounded RPC window across all deletes.
/// Which deletions the next `drive` advances. The drive runs after every event but visits
/// only these, so its cost per event does not grow with the deletions in progress. Every
/// change that can let a deletion advance marks it: an event or a finished operation naming
/// its volume key, a new deletion, a finished replica delete for the deletions waiting on the
/// request window, and a finished cold pass for every deletion, since a removed record waits
/// for the next pass. The debug oracle checks that no change is missed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct DeletionDrive {
	pub dirty: BTreeSet<VolumeLogKey>,
	/// Deletions that stopped at the full replica-delete window; a finished delete marks them.
	pub window_waiting: BTreeSet<VolumeLogKey>,
}

/// Marks the deletions of every incarnation of `volume_key`.
pub(crate) fn mark_volume_key(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey) {
	let keys = state
		.deleting
		.range(super::incarnations(volume_key))
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	state.deletion_drive.dirty.extend(keys);
}

/// Marks every deletion, for rare changes that affect all of them and for the debug
/// oracle's full pass.
pub(crate) fn mark_all(state: &mut MaintainerState) {
	let keys = state.deleting.keys().cloned().collect::<Vec<_>>();
	state.deletion_drive.dirty.extend(keys);
}

/// A finished replica delete frees the request window for the deletions waiting on it.
pub(crate) fn replica_delete_finished(state: &mut MaintainerState) {
	let waiting = std::mem::take(&mut state.deletion_drive.window_waiting);
	state.deletion_drive.dirty.extend(waiting);
}

pub(crate) fn drive(state: &mut MaintainerState, entropy: Entropy, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	// Any one durable tombstone wins over missing members and stale Live observations.
	let mut taken_up = Vec::new();
	state.deletion_candidates.retain(|key| {
		let pending = state.loaded_volumes.contains_key(key) && !state.deleting.contains_key(key);
		if !pending {
			taken_up.push(key.0.volume_key.clone());
		}
		pending
	});
	let recovered = state
		.deletion_candidates
		.iter()
		.filter_map(|key| {
			let volume = state.loaded_volumes.get(key)?;
			(volume.is_deleted()
				&& !state.deleting.contains_key(key)
				&& !super::forks::holds_key(state, &key.0.volume_key)
				&& !state.pending.contains_key(&key.0.volume_key)
				&& !super::repair::holds_key(state, &key.0.volume_key)
				&& !super::catch_up::holds_key(state, &key.0.volume_key))
			.then(|| volume.row(key))
			.flatten()
			.filter(|row| row.status == FsVolumeStatus::Deleted)
			.map(|row| {
				(
					key.clone(),
					row,
					volume
						.stored_row
						.as_ref()
						.is_some_and(|stored| stored.status == FsVolumeStatus::Deleted),
				)
			})
		})
		.collect::<Vec<_>>();
	for (key, row, confirmed_row) in recovered {
		state.deletion_candidates.remove(&key);
		state.deletion_drive.dirty.insert(key.clone());
		super::unload::cancel_key(state, &key.0.volume_key, fx);
		quiesce(state, &key, fx);
		state.deleting.insert(
			key.clone(),
			Deletion {
				caller: None,
				step: DeleteStep::ReadingRow {
					observed: Some(row.clone()),
				},
				superseded: false,
			},
		);
		taken_up.push(key.0.volume_key.clone());
		if super::maintainer_log::replay::pending_for(state, &key.0.volume_key) {
			// The name's recovered ops decide which incarnation the store's row names, so
			// this delete reads it once they are applied.
			state.deleting.get_mut(&key).expect("inserted above").step =
				DeleteStep::WaitingForReplay {
					observed: Some(row),
				};
		} else if confirmed_row && !needs_pin_mark(&key, &row) {
			activate(state, key, row, None, entropy.clone(), fx);
		} else {
			state
				.store_queue
				.push_back(StoreWork::ReadDeleteRow { key });
		}
	}
	// Incarnations held back while a sibling's tombstone waited for recovery can load now.
	taken_up.sort();
	taken_up.dedup();
	for volume_key in taken_up {
		super::filesystem::load_incarnations(state, &volume_key);
	}

	// Everything below advances active deletions. This runs after every maintainer event, so
	// skip collecting queued rows and sweeps when nothing is being deleted.
	if state.deleting.is_empty() {
		super::lifecycle::resume_after_deletion(state, fx);
		return;
	}

	// Keep replica RPCs bounded independently of how much deletion work restart discovered.
	// Replay of a recovered maintainer log changes what every deletion may do, so all of
	// them are advanced while it runs.
	if super::maintainer_log::replay::replaying(state) {
		mark_all(state);
	}
	// A deletion that finishes marks its sibling incarnations, which may run again in the
	// same drive. Each further round needs a deletion that finished in the round before.
	let mut active = state.operations.index().counts.delete_replica as usize;
	loop {
		let keys = std::mem::take(&mut state.deletion_drive.dirty)
			.into_iter()
			.filter(|key| state.deleting.contains_key(key))
			.collect::<Vec<_>>();
		if keys.is_empty() {
			break;
		}
		for key in keys {
			if pinning(state, &key) {
				drive_pin(state, &key, fx);
				continue;
			}
			if super::drain::active(state, &key) {
				continue;
			}
			if super::drain::finished(state, &key) {
				adopt_drain(state, &key, fx);
			}
			advance(state, &key, fx);
			let limit = (state.config.tuning().max_delete_requests as usize).saturating_sub(active);
			let Some(Deletion {
				step: DeleteStep::Active(progress),
				..
			}) = state.deleting.get(&key)
			else {
				continue;
			};
			let ready = progress
				.members
				.iter()
				.filter_map(|(worker, member)| match member {
					MemberProgress::Ready(action) => Some((worker.clone(), action.clone())),
					_ => None,
				})
				.take(limit)
				.collect::<Vec<_>>();
			let blocked = progress
				.members
				.values()
				.filter(|member| matches!(member, MemberProgress::Ready(_)))
				.count() > ready.len();
			if blocked {
				state.deletion_drive.window_waiting.insert(key.clone());
			}
			active += ready.len();
			for (worker, action) in ready {
				send(state, &key, worker, action, fx);
			}
		}
	}
	// A deletion that finished or became obsolete above releases its key to a waiting create.
	super::lifecycle::resume_after_deletion(state, fx);
}

/// One replica proves deletion intent; a current voting quorum acknowledges the caller without awaiting cold storage.
fn advance(state: &mut MaintainerState, key: &VolumeLogKey, fx: &mut Effects) {
	let replaying_key = super::maintainer_log::replay::pending_for(state, &key.0.volume_key);
	let passes_completed = state.maintainer_log.passes_completed;
	if let Some(Deletion {
		step: step @ DeleteStep::WaitingForCover { .. },
		..
	}) = state.deleting.get_mut(key)
	{
		let DeleteStep::WaitingForCover { through_dispatch } = *step else {
			unreachable!("matched above")
		};
		if passes_completed > through_dispatch {
			*step = DeleteStep::ReadingRow { observed: None };
			state
				.store_queue
				.push_back(StoreWork::ReadDeleteRow { key: key.clone() });
		}
		return;
	}
	// A staged row that an ordinary pass has since covered is cold now.
	let covered = match state.deleting.get(key) {
		Some(Deletion {
			step:
				DeleteStep::Active(DeleteProgress {
					cold: ColdProgress::Staged {
						through_dispatch,
						row,
					},
					..
				}),
			..
		}) if passes_completed > *through_dispatch => Some((**row).clone()),
		Some(_) | None => None,
	};
	if let Some(row) = covered {
		if let Some(Deletion {
			step: DeleteStep::Active(progress),
			..
		}) = state.deleting.get_mut(key)
		{
			progress.cold = ColdProgress::WaitingForRow;
		}
		super::cold_pass::row_stored(state, key.clone(), row);
	}
	let Some(delete) = state.deleting.get_mut(key) else {
		return;
	};
	let DeleteStep::Active(progress) = &mut delete.step else {
		return;
	};
	let deleted = progress
		.members
		.values()
		.filter(|member| matches!(member, MemberProgress::Deleted))
		.count();
	let deleted_quorum = super::membership::quorum(&state.tablet.replica_set, |w| {
		matches!(progress.members.get(w), Some(MemberProgress::Deleted))
	});
	// The client observes only the replica quorum, never object-store progress.
	if deleted_quorum && let Some(id) = delete.caller.take() {
		fx.cancel_timer(MaintainerTimer::DeleteRequest(key.clone()));
		reply(id, fx);
	}

	// Preserve intent in the background row before removing any cold data.
	if matches!(progress.cold, ColdProgress::WaitingForReplica) && deleted > 0 {
		progress.cold = ColdProgress::WaitingForRow;
	}
	// A recovered op of this volume, such as a fork's acquire of it as a parent, applies
	// against its stored record first, so the Deleted record waits for replay.
	if matches!(progress.cold, ColdProgress::WaitingForRow)
		&& !replaying_key
		&& !state.operations.index().deleted_pass_rows.contains_key(key)
		&& state.pending_rows.push_back(key.clone())
	{
		super::cold_pass::arm_for_first_row(
			&state.phase,
			&state.pending_rows,
			state.config.write_to_cold_after_ms,
			fx,
		);
	}

	// A bounded sweep releases metadata last; every replica must then acknowledge forget.
	// The sweep decides by the store's rows, which recovered maintainer log ops of this
	// volume may still change, so it waits for that replay to end; the end marks it again.
	if progress.cold == ColdProgress::Ready && deleted_quorum && !replaying_key {
		let sweep = StoreWork::SweepDeleted { key: key.clone() };
		if !state.store_queue.contains(&sweep) {
			state.store_queue.push_back(sweep);
		}
	}
	if let ColdProgress::Removed { through_dispatch } = progress.cold
		&& state.maintainer_log.passes_completed > through_dispatch
	{
		gone(progress);
	}
	if matches!(progress.cold, ColdProgress::Gone)
		&& progress
			.members
			.values()
			.all(|member| matches!(member, MemberProgress::Forgotten))
	{
		let delete = state.deleting.remove(key).expect("complete delete");
		if let Some(id) = delete.caller {
			reply(id, fx);
		}
		super::filesystem::forget(state, key, fx);
		state.remove_loaded_volume(key);
		fx.cancel_timer(MaintainerTimer::DeleteRetry(key.clone()));
		fx.cancel_timer(MaintainerTimer::DeleteRequest(key.clone()));
		// The finished cleanup no longer reserves the volume key for other incarnations,
		// whose deletions may wait on it.
		super::filesystem::load_incarnations(state, &key.0.volume_key);
		mark_volume_key(state, &key.0.volume_key);
	}
}

/// Select authority from the operation: only claims use a client identity; lifecycle mutations use the fixed tablet grant.
fn send(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	worker: WorkerId,
	action: DeleteAction,
	fx: &mut Effects,
) {
	let DeleteStep::Active(progress) = &state.deleting[key].step else {
		return;
	};
	let header =
		wire::ReplicaRequestHeader::MaintainerRequestHeader(wire::MaintainerRequestHeader {
			volume_log_key: key.0.clone(),
			tablet_grant: state.grant(),
		});
	let request = match action {
		DeleteAction::Claim => Request::ClaimRequest(wire::ClaimRequest {
			header: wire::ReplicaRequestHeader::ClientRequestHeader(wire::ClientRequestHeader {
				volume_log_key: key.0.clone(),
				client: progress.client.clone(),
				tablet_replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
			}),
			holder: None,
		}),
		DeleteAction::Mark => {
			Request::MarkVolumeDeletedRequest(wire::MarkVolumeDeletedRequest { header })
		}
		DeleteAction::Load => Request::LoadVolumeRequest(wire::LoadVolumeRequest {
			// A tombstone keeps the incarnation's ordering epoch, so a recovery that finds
			// only tombstones still orders the name's incarnations.
			creation: state
				.loaded_volumes
				.get(key)
				.and_then(|volume| {
					volume.creation_evidence(state.config.write_versions.maintainer_log)
				})
				.or_else(|| {
					super::replica_creation::encode(
						&progress.row,
						state.config.write_versions.maintainer_log,
					)
				}),
			open_claim: None,
			learner_replica_set_version: None,
			disk_incarnation: state
				.disk_incarnation(&worker)
				.expect("current deletion member"),
			claimed_client: None,
			claimed_holder: None,
			header,
			cold: progress.row.cold.clone(),
			status: Status::Deleted,
			initial_entries: Vec::new(),
		}),
		DeleteAction::Forget => Request::ForgetVolumeRequest(wire::ForgetVolumeRequest {
			observation: None,
			header,
			expected_appended: None,
			expected_claim: None,
		}),
	};
	let DeleteStep::Active(progress) =
		&mut state.deleting.get_mut(key).expect("active delete").step
	else {
		unreachable!()
	};
	progress
		.members
		.insert(worker.clone(), MemberProgress::Running);
	let op_id = state.begin(
		Operation::DeleteReplica {
			key: key.clone(),
			worker: worker.clone(),
			action,
		},
		fx,
	);
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: worker,
		request,
	});
}

/// Consume only correlated replies; a higher client epoch changes the next claim, never deletion identity.
pub(crate) fn replica(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	worker: WorkerId,
	action: DeleteAction,
	result: Result<Response, String>,
	fx: &mut Effects,
) {
	let Some(Deletion {
		step: DeleteStep::Active(progress),
		..
	}) = state.deleting.get_mut(&key)
	else {
		return;
	};
	let observed = match &result {
		Ok(Response::ReplicaStateResponse(response)) => Some(response.state.clone()),
		Ok(Response::RejectResponse(RejectResponse::VolumeLogReplicaDeleted(response))) => {
			Some(response.state.clone())
		}
		Ok(Response::RejectResponse(RejectResponse::StaleClientEpoch(response))) => {
			Some(response.state.clone())
		}
		Ok(Response::RejectResponse(RejectResponse::ClientConflict(response))) => {
			Some(response.state.clone())
		}
		_ => None,
	};

	// Classify the outcome using the retained operation, not the response variant alone.
	let next = if action == DeleteAction::Forget
		&& matches!(
			&result,
			Ok(Response::ReplicaStateResponse(_)
				| Response::RejectResponse(RejectResponse::VolumeNotLoaded))
		) {
		MemberProgress::Forgotten
	} else if action == DeleteAction::Forget {
		MemberProgress::Waiting(DeleteAction::Forget)
	} else if observed
		.as_ref()
		.is_some_and(|replica| replica.status == Status::Deleted)
	{
		MemberProgress::Deleted
	} else if matches!(
		&result,
		Ok(Response::RejectResponse(RejectResponse::VolumeNotLoaded))
	) {
		MemberProgress::Ready(DeleteAction::Load)
	} else if let Some(observed) = &observed {
		match action {
			DeleteAction::Claim
				if matches!(result, Ok(Response::ReplicaStateResponse(_)))
					&& observed.claimed_client.as_ref() == Some(&progress.client) =>
			{
				MemberProgress::Ready(DeleteAction::Mark)
			}
			DeleteAction::Claim => {
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
				let Some(epoch) = floor.checked_add(1) else {
					super::fence(state, fx);
					return;
				};
				progress.client.client_epoch = progress.client.client_epoch.max(epoch);
				MemberProgress::Waiting(DeleteAction::Claim)
			}
			DeleteAction::Load => MemberProgress::Ready(DeleteAction::Claim),
			DeleteAction::Mark | DeleteAction::Forget => MemberProgress::Waiting(action),
		}
	} else {
		MemberProgress::Waiting(action)
	};
	if matches!(next, MemberProgress::Waiting(_)) {
		fx.arm_timer_after(
			MaintainerTimer::DeleteRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
	}
	progress.members.insert(worker.clone(), next);
	if let Some(observed) = observed
		&& let Some(volume) = state.loaded_volumes.get_mut(&key)
	{
		volume.members.insert(worker, observed);
		// A claim refusal carries the newer claim, which may name another holder process.
		super::holders::claim_observed(state, &key);
	}
	observe_loaded(state, &key);
}

/// The row's completed flush is the first cleanup barrier; an old Live pass cannot satisfy it.
pub(crate) fn row_written(state: &mut MaintainerState, key: &VolumeLogKey, row: &FsVolumeRecord) {
	if row.status == FsVolumeStatus::Deleted
		&& let Some(Deletion {
			step: DeleteStep::Active(progress),
			..
		}) = state.deleting.get_mut(key)
		&& matches!(progress.cold, ColdProgress::WaitingForRow)
	{
		progress.cold = ColdProgress::Ready;
	}
}

/// A pass staged a deletion's row in the memtable: it waits for an ordinary pass dispatched
/// after `through_dispatch` to make it cold.
pub(crate) fn row_staged(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	row: FsVolumeRecord,
	through_dispatch: u64,
) {
	if let Some(Deletion {
		step: DeleteStep::Active(progress),
		..
	}) = state.deleting.get_mut(key)
		&& matches!(progress.cold, ColdProgress::WaitingForRow)
	{
		progress.cold = ColdProgress::Staged {
			through_dispatch,
			row: Box::new(row),
		};
		state.deletion_drive.dirty.insert(key.clone());
	}
}

/// A sweep starts only after earlier writes finish.
pub(crate) fn dispatch_sweep(state: &mut MaintainerState, key: VolumeLogKey, fx: &mut Effects) {
	let Some(Deletion {
		step: DeleteStep::Active(progress),
		..
	}) = state.deleting.get_mut(&key)
	else {
		return;
	};
	if progress.cold != ColdProgress::Ready {
		return;
	}
	progress.cold = ColdProgress::Sweeping;
	let op_id = state.begin(Operation::SweepDeleted { key: key.clone() }, fx);
	fx.execute(MaintainerEffect::SweepDeletedRequest {
		op_id,
		volume_log_key: key,
		max_rows: state.config.tuning().max_delete_rows,
	});
}

/// Metadata tombstones must reach cold storage before unconditional forgets can run. A
/// complete sweep step has released the record and handed the data to a pending sweep.
pub(crate) fn swept(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<DeleteSweepResult, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::SweepDeleted { .. })
	) {
		return;
	}
	let Some(Operation::SweepDeleted { key }) = state.finish(op_id, fx) else {
		unreachable!()
	};
	let Ok(result) = result else {
		super::fence(state, fx);
		return;
	};
	let Some(Deletion {
		step: DeleteStep::Active(progress),
		..
	}) = state.deleting.get_mut(&key)
	else {
		return;
	};
	if result == DeleteSweepResult::Held {
		progress.cold = ColdProgress::Held;
		fx.arm_timer_after(
			MaintainerTimer::DeleteRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
	} else {
		if !matches!(
			state.deleting.get(&key),
			Some(Deletion {
				step: DeleteStep::Active(_),
				..
			})
		) {
			return;
		}
		let through_dispatch = super::maintainer_log::wrote_uncovered(state, fx);
		let Some(Deletion {
			step: DeleteStep::Active(progress),
			..
		}) = state.deleting.get_mut(&key)
		else {
			return;
		};
		progress.cold = ColdProgress::Removed { through_dispatch };
		// A claim log's delete releasing its record may let its volume's sweep go on.
		let (base, claim_log) = super::claims::base_of(&key);
		if claim_log != rivet_data::ROOT_CLAIM_LOG {
			state.deletion_drive.dirty.insert(base);
		}
		// The sweep released the metadata and left the data to the incarnation's pending
		// sweep, which runs in the background so the key can be created again at once.
		let sweep = StoreWork::SweepPending {
			scope: depot_fs_core::keys::slatedb::Scope::new(
				&key.0.volume_key,
				uuid::Uuid::from_bytes(key.0.volume_incarnation),
			),
		};
		if !state.store_queue.contains(&sweep) {
			state.store_queue.push_back(sweep);
		}
		// Sent deletion requests may still finish remotely. A later listing checks
		// the cold row before cleaning an obsolete tombstone, never overwriting its replacement.
		let old = state
			.operations
			.index()
			.delete_replicas
			.get(&key)
			.map_or_else(Vec::new, |ids| ids.iter().copied().collect());
		for id in old {
			state.finish(id, fx);
			fx.execute(MaintainerEffect::CancelReplicaRequest { op_id: id });
		}
	}
}

/// The removal is durable: every replica may be forgotten.
fn gone(progress: &mut DeleteProgress) {
	progress.cold = ColdProgress::Gone;
	for member in progress.members.values_mut() {
		*member = MemberProgress::Ready(DeleteAction::Forget);
	}
}

fn reply(id: RequestId, fx: &mut Effects) {
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response: ClientResponse::VolumeDeleteResponse,
	});
}
