//! Repair claims: an operator's exclusive, fenced writer of one volume.
//!
//! A repair claim is an ordinary claim the maintainer grants to a volume client it runs
//! itself (the executor's repair clients), at an epoch it issues from its own band, so it
//! sorts above every claim the volume's members hold and below every claim a later grant
//! issues. Opening the client claims the members, which fences the current writer at the
//! shards, recovers the log's tail and commits a claim entry. While the claim exists in any
//! phase the maintainer refuses opens, creates, deletes and restores of the name with
//! `VolumeUnderRepair`, never unloads the incarnation, and refuses a second claim, so the
//! claim's client is the volume's only writer and no other writer is granted one: there is
//! no claim ping-pong.
//!
//! Every write is the client's own append under its claim, which each shard checks; the
//! maintainer never writes a replica or the store for the claim. An append lands just above
//! the claim's tail, which the call names, so nothing is written at or below cold, and the
//! normal apply carries it to the store as new row versions; older positions keep the old
//! versions, so a checkpoint or fork hold keeps what it pinned.
//!
//! The claim ends on release, after its time to live with no call naming it, when the
//! tablet's replica set changes, or when the maintainer loses its grant: the next lifetime's
//! epochs sort above it, and the executor stops every repair client before the grant is
//! released. A claim that fails to open or loses an append's outcome closes its client.
//!
//! One `RepairOperation::Claim` operation lives with each claim. Its timer is the deadline of
//! the claim's current phase: the open, the append in flight, the close, or the time to live.

use super::{
	RepairOperation,
	calls::{self, Completion, Run, refusal},
};
use crate::repair_types::{PrepareReads, RepairPrepared};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::{MaintainerEffect, RepairEffect},
	state::{MaintainerState, Operation},
	types::{MaintainerTimer, OpId, RepairAppendFailure, RepairClaimOpen, RepairClaimOpened},
};

use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		RepairAppend, RepairAppendPlan, RepairAppended, RepairClaimAcquire, RepairClaimGranted,
		RepairClaimPlan, RepairClaimRelease, RepairDetail, RepairRefusalCode, RepairRefused,
		RepairReleasePlan, RepairReloadPlan, RepairReplicaReload, RepairRowChange,
		VolumeRejectResponse, VolumeUnderRepair,
	},
	depot_fs_entry_v1::{FsEntry, FsTxn},
	depot_log_shard_v1::{PayloadCodec, PayloadWrite},
	shared_depot_fs_v1::{FsOp, FsVolumeStatus},
	shared_depot_types_v1::{
		ClientIdentity, TabletPhase, VolumeLogPosition, VolumeLogReplicaStatus,
	},
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use vbare::Versioned;

/// Repair claims one tablet holds at once.
pub(crate) const MAX_CLAIMS: usize = 4;

/// The longest time to live a claim may ask for.
pub(crate) const MAX_TTL_MS: u64 = 30 * 60 * 1000;

/// How long a claim's client may take to open, append one entry or close.
const CLIENT_TIMEOUT_MS: u64 = 30_000;

/// How long a refused open of a name under repair waits before trying again.
const UNDER_REPAIR_RETRY_MS: u64 = 1000;

/// Rows one append may expect.
const MAX_PREIMAGES: usize = 64;

/// The repair claims of this lifetime, by volume name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Claims {
	pub(crate) by_name: BTreeMap<depot_types::VolumeKey, Claim>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Claim {
	pub(crate) key: VolumeLogKey,
	/// The operation id of the call that took the claim; refusals of the name report it.
	pub(crate) operation_id: [u8; 16],
	pub(crate) ttl_ms: u64,
	/// The claim's operation, whose timer is the current phase's deadline.
	pub(crate) op_id: OpId,
	pub(crate) phase: ClaimPhase,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ClaimPhase {
	/// The client is claiming the members at `claim_epoch` for the acquire call.
	Opening { claim_epoch: u64 },
	/// The client holds `client`'s claim; `tail` is its last committed entry, the claim
	/// entry first. The timer is the time to live.
	Held {
		client: ClientIdentity,
		tail: VolumeLogPosition,
	},
	/// The client appends one entry above `tail` for the call `operation_id`.
	Appending {
		client: ClientIdentity,
		tail: VolumeLogPosition,
		operation_id: [u8; 16],
	},
	/// The client is closing; `operation_id` is the release call waiting for it, if any.
	Closing { operation_id: Option<[u8; 16]> },
}

/// The claim that holds `volume_key`, in any phase.
pub(crate) fn holding<'a>(
	state: &'a MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> Option<&'a Claim> {
	state.repairs.claims.by_name.get(volume_key)
}

/// Whether a repair claim holds `key`'s name, so nothing else may claim, unload or retire it.
pub(crate) fn holds(state: &MaintainerState, key: &VolumeLogKey) -> bool {
	holding(state, &key.0.volume_key).is_some()
}

/// The refusal of an open, create, delete or restore of a name a repair claim holds.
pub(crate) fn under_repair(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
) -> Option<VolumeRejectResponse> {
	holding(state, volume_key).map(|claim| {
		VolumeRejectResponse::VolumeUnderRepair(VolumeUnderRepair {
			retry_after_ms: UNDER_REPAIR_RETRY_MS,
			operation_id: claim.operation_id,
		})
	})
}

fn refused(code: RepairRefusalCode, retry_after_ms: Option<u64>, message: &str) -> RepairRefused {
	refusal(code, retry_after_ms, message.into(), None)
}

/// The highest appended position the maintainer's view of `key`'s members shows.
fn appended(state: &MaintainerState, key: &VolumeLogKey) -> Option<VolumeLogPosition> {
	state
		.loaded_volumes
		.get(key)?
		.members
		.values()
		.map(|replica| replica.appended.clone())
		.max_by_key(|position| (position.volume_seq, position.client_epoch))
}

/// Checks everything an acquire needs now and returns what it would fence.
pub(crate) fn acquire_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairClaimAcquire,
) -> Result<RepairClaimPlan, RepairRefused> {
	if body.ttl_ms == 0 || body.ttl_ms > MAX_TTL_MS {
		return Err(refused(
			RepairRefusalCode::InvalidEntry,
			None,
			"ttlMs must be positive and at most 30 minutes",
		));
	}
	if holding(state, &key.0.volume_key).is_some() {
		return Err(refused(
			RepairRefusalCode::ClaimHeld,
			Some(UNDER_REPAIR_RETRY_MS),
			"a repair claim already holds the name",
		));
	}
	if state.repairs.claims.by_name.len() >= MAX_CLAIMS {
		return Err(refused(
			RepairRefusalCode::RepairBusy,
			Some(calls::BUSY_RETRY_MS),
			"the tablet holds its most repair claims",
		));
	}
	// A membership change claims volumes to catch a learner up, which would fence the
	// repair client mid-repair.
	if !matches!(state.tablet.replica_set.phase, TabletPhase::Stable) {
		return Err(refused(
			RepairRefusalCode::NotStable,
			Some(1000),
			"the tablet's replica set is changing",
		));
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return Err(refused(
			RepairRefusalCode::NotLoaded,
			None,
			"the incarnation is not loaded; an open of the name loads it",
		));
	};
	let blocked = if super::super::filesystem::is_record_missing(state, key) {
		Some("the incarnation is quarantined for a lost record; rebuild its record first")
	} else if state.deleting.contains_key(key) {
		Some("the incarnation is being deleted")
	} else if state.unloading.contains_key(key) {
		Some("the incarnation is unloading")
	} else if state.pending.contains_key(&key.0.volume_key) {
		Some("an open, create or delete of the name is in progress")
	} else if super::super::repair::holds_key(state, &key.0.volume_key)
		|| super::super::catch_up::holds_key(state, &key.0.volume_key)
		|| super::super::member_repair::loading(state, key)
	{
		Some("a recovery or member load of the name is in progress")
	} else if super::super::forks::holds_key(state, &key.0.volume_key) {
		Some("a fork or restore of the name is in progress")
	} else if super::super::maintainer_log::replay::pending_for(state, &key.0.volume_key) {
		Some("recovered maintainer log ops of the name are still to apply")
	} else {
		None
	};
	if let Some(message) = blocked {
		return Err(refused(RepairRefusalCode::RepairBusy, Some(1000), message));
	}
	let record = volume.row(key);
	if record
		.as_ref()
		.is_none_or(|record| record.status != FsVolumeStatus::Live)
	{
		return Err(refused(
			RepairRefusalCode::IncarnationChanged,
			None,
			"the incarnation is not live",
		));
	}
	let in_sync = |worker: &WorkerId| {
		volume
			.members
			.get(worker)
			.is_some_and(|replica| replica.status == VolumeLogReplicaStatus::InSync)
	};
	if !super::super::membership::quorum(&state.tablet.replica_set, in_sync) {
		return Err(refused(
			RepairRefusalCode::NoQuorum,
			Some(1000),
			"no quorum of members is in sync; reload the missing members first",
		));
	}
	let holder = super::super::holders::holder(volume);
	if holder.is_some() && !body.fence_holder {
		return Err(refused(
			RepairRefusalCode::LiveHolder,
			None,
			"the volume's newest claim names a holder process; set fenceHolder to fence it",
		));
	}
	let appended = appended(state, key).unwrap_or(VolumeLogPosition {
		volume_seq: 0,
		client_epoch: 0,
	});
	if let Some(expected) = &body.expected_appended
		&& *expected != appended
	{
		return Err(refusal(
			RepairRefusalCode::PositionChanged,
			None,
			format!(
				"the members' highest appended position is {} at epoch {}",
				appended.volume_seq, appended.client_epoch
			),
			None,
		));
	}
	Ok(RepairClaimPlan { holder })
}

/// Runs an acquire: checks the plan again, issues the claim's epoch and opens its client.
pub(crate) fn acquire(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	operation_id: [u8; 16],
	body: &RepairClaimAcquire,
	plan: &RepairClaimPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	// The checks run again; only the holder the plan fences must be the same.
	let now = acquire_plan(state, key, body)?;
	if now.holder != plan.holder {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			None,
			"another process holds the volume since the plan",
		));
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return Err(refused(RepairRefusalCode::NotLoaded, None, "not loaded"));
	};
	let Some(row) = volume.row(key) else {
		return Err(refused(RepairRefusalCode::NotLoaded, None, "no record"));
	};
	// The claim sorts above every epoch the members hold, as a recovery's does.
	let floor = volume
		.members
		.values()
		.map(super::super::maintainer_log::epoch_floor)
		.fold(row.cold.client_epoch, u64::max);
	let Some(claim_epoch) = floor
		.checked_add(1)
		.and_then(|min| super::super::claim_epochs::issue(state, min))
	else {
		return Err(refused(
			RepairRefusalCode::RepairBusy,
			Some(1000),
			"no claim epoch can be issued under this grant now",
		));
	};
	let op_id = state.begin(
		Operation::Repair(RepairOperation::Claim {
			volume_key: key.0.volume_key.clone(),
		}),
		fx,
	);
	fx.arm_timer_after(MaintainerTimer::Operation(op_id), CLIENT_TIMEOUT_MS);
	state.repairs.claims.by_name.insert(
		key.0.volume_key.clone(),
		Claim {
			key: key.clone(),
			operation_id,
			ttl_ms: body.ttl_ms,
			op_id,
			phase: ClaimPhase::Opening { claim_epoch },
		},
	);
	// The previous holder's later appends are refused by the shards once the client claims
	// them, and its reopen is refused here while the claim lives.
	tracing::warn!(
		?key,
		claim_epoch,
		operation_id = %uuid::Uuid::from_bytes(operation_id),
		"an operator repair claim is fencing the volume's writer"
	);
	fx.execute(MaintainerEffect::Repair(RepairEffect::ClaimOpenRequest {
		op_id,
		request: RepairClaimOpen {
			volume_log_key: key.clone(),
			row,
			replica_set: state.tablet.replica_set.clone(),
			claim_epoch,
			timeout_ms: CLIENT_TIMEOUT_MS,
		},
	}));
	Ok(Run::Pending)
}

/// The claim's client opened, or failed to.
pub(crate) fn opened(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<RepairClaimOpened, String>,
	fx: &mut Effects,
) {
	let Some(volume_key) = claim_of(state, op_id) else {
		return;
	};
	let claim = state
		.repairs
		.claims
		.by_name
		.get(&volume_key)
		.expect("claim");
	let ClaimPhase::Opening { claim_epoch } = claim.phase else {
		return;
	};
	let operation_id = claim.operation_id;
	match result {
		Ok(opened) if opened.client.client_epoch == claim_epoch => {
			let ttl_ms = claim.ttl_ms;
			let claim = state
				.repairs
				.claims
				.by_name
				.get_mut(&volume_key)
				.expect("claim");
			claim.phase = ClaimPhase::Held {
				client: opened.client.clone(),
				tail: opened.claim_entry.clone(),
			};
			fx.arm_timer_after(MaintainerTimer::Operation(op_id), ttl_ms);
			calls::complete(
				state,
				operation_id,
				Ok(Some(RepairDetail::RepairClaimGranted(RepairClaimGranted {
					claim: opened.client,
					claim_entry: opened.claim_entry,
					in_sync: opened.in_sync.iter().map(|w| *w.as_bytes()).collect(),
					incomplete: opened.incomplete.iter().map(|w| *w.as_bytes()).collect(),
				}))),
				fx,
			);
		}
		Ok(_) | Err(_) => {
			let message = match &result {
				Ok(_) => "the repair client claimed at another epoch".to_owned(),
				Err(error) => error.clone(),
			};
			tracing::warn!(%message, ?volume_key, "an operator repair claim failed to open; closing it");
			// Claims went out before the open failed, so the writer may be fenced and a claim
			// entry committed; the outcome is unknown, never a refusal that ran nothing.
			calls::complete(
				state,
				operation_id,
				Err(Completion::Unknown(format!(
					"the repair claim did not open ({message}) and is being closed; it may have fenced the writer"
				))),
				fx,
			);
			close(state, &volume_key, None, fx);
		}
	}
}

/// Checks a release: it must name the held claim.
pub(crate) fn release_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairClaimRelease,
) -> Result<RepairReleasePlan, RepairRefused> {
	held(state, key, &body.claim)?;
	Ok(RepairReleasePlan {
		claim: body.claim.clone(),
	})
}

/// The held claim of `key` naming `client`, with its tail.
fn held(
	state: &MaintainerState,
	key: &VolumeLogKey,
	client: &ClientIdentity,
) -> Result<VolumeLogPosition, RepairRefused> {
	match holding(state, &key.0.volume_key) {
		Some(Claim {
			key: claimed,
			phase: ClaimPhase::Held {
				client: holder,
				tail,
			},
			..
		}) if claimed == key && holder == client => Ok(tail.clone()),
		Some(Claim {
			phase: ClaimPhase::Appending { .. } | ClaimPhase::Opening { .. },
			..
		}) => Err(refused(
			RepairRefusalCode::OperationInProgress,
			Some(calls::BUSY_RETRY_MS),
			"the repair claim is busy",
		)),
		Some(_) | None => Err(refused(
			RepairRefusalCode::ClaimLost,
			None,
			"no repair claim with this identity holds the volume",
		)),
	}
}

/// Runs a release: closes the claim's client and answers once it closed.
pub(crate) fn release(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	operation_id: [u8; 16],
	body: &RepairClaimRelease,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	held(state, key, &body.claim)?;
	close(state, &key.0.volume_key, Some(operation_id), fx);
	Ok(Run::Pending)
}

/// Begins closing `volume_key`'s claim client, whatever its phase. `operation_id` is a
/// release call that waits for the close.
fn close(
	state: &mut MaintainerState,
	volume_key: &depot_types::VolumeKey,
	operation_id: Option<[u8; 16]>,
	fx: &mut Effects,
) {
	let Some(claim) = state.repairs.claims.by_name.get_mut(volume_key) else {
		return;
	};
	// A call waiting on the client's open or append loses its answer with the client, so it
	// is answered now: the work may have run.
	let waiting = match &claim.phase {
		ClaimPhase::Closing { .. } => return,
		ClaimPhase::Opening { .. } => Some(claim.operation_id),
		ClaimPhase::Appending { operation_id, .. } => Some(*operation_id),
		ClaimPhase::Held { .. } => None,
	};
	claim.phase = ClaimPhase::Closing { operation_id };
	let op_id = claim.op_id;
	let key = claim.key.clone();
	fx.arm_timer_after(MaintainerTimer::Operation(op_id), CLIENT_TIMEOUT_MS);
	fx.execute(MaintainerEffect::Repair(RepairEffect::ClaimCloseRequest {
		op_id,
		volume_log_key: key,
	}));
	if let Some(waiting) = waiting {
		calls::complete(
			state,
			waiting,
			Err(Completion::Unknown(
				"the repair claim was closed while this call's work was in flight; inspect the volume".into(),
			)),
			fx,
		);
	}
}

/// The claim's client closed: the claim ends and opens of the name are admitted again.
pub(crate) fn closed(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	let Some(volume_key) = claim_of(state, op_id) else {
		return;
	};
	end(state, &volume_key, fx);
}

/// Removes `volume_key`'s claim and its operation, answering a release that waits for it.
fn end(state: &mut MaintainerState, volume_key: &depot_types::VolumeKey, fx: &mut Effects) {
	let Some(claim) = state.repairs.claims.by_name.remove(volume_key) else {
		return;
	};
	state.finish(claim.op_id, fx);
	tracing::info!(key = ?claim.key, "an operator repair claim ended");
	if let ClaimPhase::Closing {
		operation_id: Some(operation_id),
	} = claim.phase
	{
		calls::complete(state, operation_id, Ok(None), fx);
	}
	// The volume resumes as any other: an idle one unloads on its own deadline.
	super::super::unload::track_idle(state, &claim.key);
	super::super::lifecycle::resume_after_repair(state, volume_key, fx);
}

/// The volume key of the claim whose operation is `op_id`, while that claim lives.
fn claim_of(state: &MaintainerState, op_id: OpId) -> Option<depot_types::VolumeKey> {
	match state.operations.get(&op_id) {
		Some(Operation::Repair(RepairOperation::Claim { volume_key }))
			if state
				.repairs
				.claims
				.by_name
				.get(volume_key)
				.is_some_and(|claim| claim.op_id == op_id) =>
		{
			Some(volume_key.clone())
		}
		Some(_) | None => None,
	}
}

/// The rows an append's preimages name, read at the volume's applied position once that is
/// the claim's tail.
pub(crate) fn append_reads(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairAppend,
) -> PrepareReads {
	let Some(volume) = state.filesystem.volumes.get(key) else {
		return PrepareReads::None;
	};
	if body.preimages.is_empty() || body.preimages.len() > MAX_PREIMAGES {
		return PrepareReads::None;
	}
	PrepareReads::Rows {
		record: volume.record.to_record(),
		applied: volume.applied.clone(),
		keys: body
			.preimages
			.iter()
			.map(|preimage| preimage.key.clone())
			.collect(),
	}
}

/// Whether `entry` holds only what an append may write: KV puts, deletes and range deletes,
/// or the abort of a transaction left pending.
fn valid_entry(entry: &FsEntry) -> Result<(), &'static str> {
	if depot_fs_core::txn::validate(entry).is_err() {
		return Err("the entry is not a valid filesystem entry");
	}
	let kv_ops = entry.ops.iter().all(|op| {
		matches!(
			op,
			FsOp::FsKvPut(_) | FsOp::FsKvDelete(_) | FsOp::FsKvDeleteRange(_)
		)
	});
	match &entry.txn {
		None if !entry.ops.is_empty() && kv_ops => Ok(()),
		Some(FsTxn::FsTxnAbort(_)) if entry.ops.is_empty() => Ok(()),
		None | Some(FsTxn::FsTxnAbort(_) | FsTxn::FsTxnPending(_) | FsTxn::FsTxnCommit(_)) => {
			Err("an append holds KV puts, deletes and range deletes, or only a transaction abort")
		}
	}
}

/// Checks an append against the held claim, the applied position and its preimages.
pub(crate) fn append_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairAppend,
	prepared: &RepairPrepared,
) -> Result<RepairAppendPlan, RepairRefused> {
	let tail = held(state, key, &body.claim)?;
	if tail != body.expected_appended {
		return Err(refusal(
			RepairRefusalCode::PositionChanged,
			None,
			format!(
				"the claim's tail is {} at epoch {}",
				tail.volume_seq, tail.client_epoch
			),
			None,
		));
	}
	valid_entry(&body.entry)
		.map_err(|message| refused(RepairRefusalCode::InvalidEntry, None, message))?;
	if body.preimages.len() > MAX_PREIMAGES {
		return Err(refused(
			RepairRefusalCode::InvalidEntry,
			None,
			"an append expects at most 64 rows",
		));
	}
	// The preimages are read where the maintainer applied the claim's whole history, which
	// nothing but this claim can extend.
	let applied = state
		.filesystem
		.volumes
		.get(key)
		.map(|volume| volume.applied.clone());
	if applied.as_ref() != Some(&tail) {
		return Err(refused(
			RepairRefusalCode::Lagging,
			Some(100),
			"the maintainer has not applied the claim's tail yet",
		));
	}
	let rows = match (&prepared.rows, body.preimages.is_empty()) {
		(_, true) => Vec::new(),
		// The rows must have been read at the tail itself: rows read below it may predate an
		// entry of the recovered history the operator never saw.
		(Some((read_at, rows)), false)
			if *read_at == tail && rows.len() == body.preimages.len() =>
		{
			rows.clone()
		}
		(Some(_) | None, false) => {
			return Err(refused(
				RepairRefusalCode::Lagging,
				Some(100),
				"the preimage rows were not read at the claim's tail",
			));
		}
	};
	let mut changes = Vec::with_capacity(rows.len());
	for (preimage, stored) in body.preimages.iter().zip(rows) {
		let digest = stored
			.as_ref()
			.map(|bytes| <[u8; 32]>::from(Sha256::digest(bytes)));
		if digest != preimage.value_digest {
			return Err(refused(
				RepairRefusalCode::PreimageMismatch,
				None,
				"a row does not hold the expected value at the claim's tail",
			));
		}
		changes.push(RepairRowChange {
			key: preimage.key.clone(),
			before: stored.map(Into::into),
		});
	}
	Ok(RepairAppendPlan {
		lands: VolumeLogPosition {
			volume_seq: tail.volume_seq + 1,
			client_epoch: body.claim.client_epoch,
		},
		rows: changes,
	})
}

/// Runs an append: checks the claim's tail again and sends the entry through its client.
pub(crate) fn append(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	operation_id: [u8; 16],
	body: &RepairAppend,
	plan: &RepairAppendPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	let tail = held(state, key, &body.claim)?;
	if tail != body.expected_appended
		|| plan.lands.volume_seq != tail.volume_seq + 1
		|| plan.lands.client_epoch != body.claim.client_epoch
	{
		return Err(refused(
			RepairRefusalCode::PositionChanged,
			None,
			"the claim's tail moved since the plan",
		));
	}
	let version = rivet_data::families::VersionFamily::DEPOT_FS_ENTRY
		.info()
		.map_or(1, |info| info.min_read);
	let payload = rivet_data::versioned::DepotFsEntryData::serialize_with_embedded_version(
		&body.entry,
		version,
	)
	.map_err(|_| {
		refused(
			RepairRefusalCode::InvalidEntry,
			None,
			"the entry does not encode",
		)
	})?;
	let claim = state
		.repairs
		.claims
		.by_name
		.get_mut(&key.0.volume_key)
		.expect("held claim");
	claim.phase = ClaimPhase::Appending {
		client: body.claim.clone(),
		tail,
		operation_id,
	};
	let op_id = claim.op_id;
	fx.arm_timer_after(MaintainerTimer::Operation(op_id), CLIENT_TIMEOUT_MS);
	fx.execute(MaintainerEffect::Repair(RepairEffect::ClaimAppendRequest {
		op_id,
		volume_log_key: key.clone(),
		body: PayloadWrite {
			payload_family: rivet_data::families::VersionFamily::DEPOT_FS_ENTRY.0,
			payload_version: version,
			client_seq: 0,
			request_id: [0; 16],
			payload_digest: [0; 16],
			codec: PayloadCodec::None,
			payload: payload.into(),
		},
	}));
	Ok(Run::Pending)
}

/// The claim's append committed, or failed.
pub(crate) fn appended_entry(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<VolumeLogPosition, RepairAppendFailure>,
	fx: &mut Effects,
) {
	let Some(volume_key) = claim_of(state, op_id) else {
		return;
	};
	let claim = state
		.repairs
		.claims
		.by_name
		.get(&volume_key)
		.expect("claim");
	let ClaimPhase::Appending {
		client,
		tail,
		operation_id,
	} = claim.phase.clone()
	else {
		return;
	};
	let ttl_ms = claim.ttl_ms;
	match result {
		Ok(position)
			if position.volume_seq == tail.volume_seq + 1
				&& position.client_epoch == client.client_epoch =>
		{
			let claim = state
				.repairs
				.claims
				.by_name
				.get_mut(&volume_key)
				.expect("claim");
			claim.phase = ClaimPhase::Held {
				client,
				tail: position.clone(),
			};
			fx.arm_timer_after(MaintainerTimer::Operation(op_id), ttl_ms);
			calls::complete(
				state,
				operation_id,
				Ok(Some(RepairDetail::RepairAppended(RepairAppended {
					position,
				}))),
				fx,
			);
		}
		Err(RepairAppendFailure::NotExecuted(message)) => {
			let claim = state
				.repairs
				.claims
				.by_name
				.get_mut(&volume_key)
				.expect("claim");
			claim.phase = ClaimPhase::Held { client, tail };
			fx.arm_timer_after(MaintainerTimer::Operation(op_id), ttl_ms);
			calls::complete(
				state,
				operation_id,
				Err(Completion::Refused(refusal(
					RepairRefusalCode::RepairBusy,
					Some(1000),
					format!("the append was not written: {message}"),
					None,
				))),
				fx,
			);
		}
		Ok(_) | Err(RepairAppendFailure::Unknown(_)) => {
			// The tail is no longer known, so the claim cannot take another append.
			let message = match &result {
				Err(RepairAppendFailure::Unknown(message)) => message.clone(),
				Ok(position) => format!("the entry landed at an unexpected position {position:?}"),
				Err(RepairAppendFailure::NotExecuted(message)) => message.clone(),
			};
			calls::complete(
				state,
				operation_id,
				Err(Completion::Unknown(format!(
					"the append's outcome is unknown ({message}); the claim was closed, so inspect the volume's log"
				))),
				fx,
			);
			close(state, &volume_key, None, fx);
		}
	}
}

/// The deadline of a claim's current phase passed.
pub(crate) fn timed_out(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	let Some(volume_key) = claim_of(state, op_id) else {
		return;
	};
	let claim = state
		.repairs
		.claims
		.by_name
		.get(&volume_key)
		.expect("claim");
	match claim.phase.clone() {
		ClaimPhase::Opening { .. } => {
			let operation_id = claim.operation_id;
			calls::complete(
				state,
				operation_id,
				Err(Completion::Unknown(
					"the repair claim did not open in time and is being closed; it may have fenced the writer".into(),
				)),
				fx,
			);
			close(state, &volume_key, None, fx);
		}
		ClaimPhase::Appending { operation_id, .. } => {
			calls::complete(
				state,
				operation_id,
				Err(Completion::Unknown(
					"the append did not answer in time; the claim was closed, so inspect the volume's log".into(),
				)),
				fx,
			);
			close(state, &volume_key, None, fx);
		}
		ClaimPhase::Held { .. } => {
			tracing::warn!(key = ?claim.key, "an operator repair claim expired; closing it");
			close(state, &volume_key, None, fx);
		}
		// The executor always answers a close; its own deadline passing ends the claim here
		// so the name is not held forever.
		ClaimPhase::Closing { .. } => {
			tracing::warn!(key = ?claim.key, "an operator repair claim's close did not answer in time; ending it");
			end(state, &volume_key, fx);
		}
	}
}

/// The tablet's replica set changed: every claim's client may lose its quorum or be fenced by
/// a learner's catch-up claim, so each claim ends.
pub(crate) fn replica_set_changed(state: &mut MaintainerState, fx: &mut Effects) {
	let names = state
		.repairs
		.claims
		.by_name
		.keys()
		.cloned()
		.collect::<Vec<_>>();
	for name in names {
		tracing::warn!(
			?name,
			"the tablet's replica set changed; closing an operator repair claim"
		);
		close(state, &name, None, fx);
	}
}

/// Checks a reload: the worker is a voter whose replica of the loaded volume is missing, and
/// the maintainer's member reload may load it now.
pub(crate) fn reload_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairReplicaReload,
) -> Result<RepairReloadPlan, RepairRefused> {
	let worker = WorkerId::from_bytes(body.worker);
	if !state.members().contains(&worker) {
		return Err(refused(
			RepairRefusalCode::NotAMember,
			None,
			"the worker is not a member of the tablet's replica set",
		));
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return Err(refused(
			RepairRefusalCode::NotLoaded,
			None,
			"the incarnation is not loaded",
		));
	};
	if volume.members.contains_key(&worker) {
		return Err(refused(
			RepairRefusalCode::NotMissing,
			None,
			"the worker holds a replica of the volume",
		));
	}
	let Some((source, cold)) = super::super::member_repair::reload_source(state, key, &worker)
	else {
		return Err(refused(
			RepairRefusalCode::RepairBusy,
			Some(1000),
			"the member cannot be reloaded now: no quorum is in sync, or other work owns the volume's replicas",
		));
	};
	Ok(RepairReloadPlan {
		worker: body.worker,
		source: *source.as_bytes(),
		cold,
	})
}

/// Runs a reload: starts the maintainer's own member reload of the worker.
pub(crate) fn reload(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	body: &RepairReplicaReload,
	plan: &RepairReloadPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	if reload_plan(state, key, body)? != *plan {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			None,
			"the volume's members changed since the plan",
		));
	}
	super::super::member_repair::absent(state, key, &WorkerId::from_bytes(body.worker), fx);
	Ok(Run::Done(None))
}


