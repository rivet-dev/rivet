//! Store operations whose durable record is a maintainer log op.
//!
//! A live operation first submits its op and keeps its store footprint while the op is in
//! flight, which holds its volume's apply at the op's position. Once the op commits, the
//! operation applies it to the store through the same guarded store change that replay runs
//! after a crash, and only then replies. Replay dispatches recovered ops as `ReplayOp`
//! operations with the same footprint.

use super::{MaintainerState, Operation, maintainer_log::LogToken};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::OpId};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_maintainer_log_v1::MaintainerOp;

/// The live store operation `op_id` has its op committed at `token`: apply it.
pub(crate) fn committed(
	state: &mut MaintainerState,
	op_id: OpId,
	token: LogToken,
	fx: &mut Effects,
) {
	match state.operations.get(&op_id) {
		Some(Operation::Checkpoint { .. }) => super::checkpoints::logged(state, op_id, token, fx),
		Some(Operation::ForkReference { .. }) => {
			super::fork_references::logged(state, op_id, token, fx)
		}
		// A fence cleared every operation.
		Some(_) | None => {}
	}
}

/// The volume key whose store rows an op changes, for its footprint.
pub(crate) fn op_volume(op: &MaintainerOp) -> Option<depot_types::VolumeKey> {
	match op {
		MaintainerOp::CheckpointOp(op) => Some(op.request.volume_key.clone()),
		MaintainerOp::RetentionCheckpointOp(op) => {
			Some(op.target.volume_log_key.volume_key.clone())
		}
		MaintainerOp::ForkReferenceOp(op) => Some(op.request.spec().parent_volume_key.clone()),
		MaintainerOp::ForkPublishOp(op) => Some(op.spec.destination_volume_key.clone()),
		MaintainerOp::ForkFailOp(op) => Some(op.spec.destination_volume_key.clone()),
		MaintainerOp::RepairAuditOp(op) => {
			Some(op.row.request.envelope.volume_log_key.volume_key.clone())
		}
	}
}

/// Applies one recovered op to the store. `key` is the volume whose apply reached the op's
/// position, when the op follows one.
pub(crate) fn replay(
	state: &mut MaintainerState,
	token: LogToken,
	op: MaintainerOp,
	key: Option<VolumeLogKey>,
	fx: &mut Effects,
) {
	let volume_key = op_volume(&op);
	let (mut record, applied) = key
		.as_ref()
		.and_then(|key| state.filesystem.volumes.get(key))
		.map(|volume| {
			(
				Some(volume.record.to_record()),
				Some(volume.applied.clone()),
			)
		})
		.unwrap_or((None, None));
	// An acquire replays at the parent as it was when the live op ran. A parent deleted
	// later is no longer registered for apply, and its first record may never have reached
	// cold, so the acquire takes the record the parent's replicas describe, live as it was.
	if record.is_none()
		&& let rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkReferenceOp(op) =
			&op && matches!(
		op.request,
		rivet_data::generated::depot_client_protocol_v1::FsForkReferenceRequest::FsForkAcquire(_)
	) && let Some(target) = &op.target
	{
		let target_key = VolumeLogKey(target.volume_log_key.clone());
		record = state
			.loaded_volumes
			.get(&target_key)
			.and_then(|volume| volume.row(&target_key))
			.map(|mut row| {
				row.status = rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus::Live;
				row
			});
	}
	let loaded_parent_incarnations = match &op {
		rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkReferenceOp(op) => {
			super::loaded_incarnations(state, &op.request.spec().parent_volume_key)
		}
		_ => Vec::new(),
	};
	let op_id = state.begin(Operation::ReplayOp { token, volume_key }, fx);
	fx.execute(MaintainerEffect::MaintainerOpRequest {
		op_id,
		op,
		record,
		applied,
		region: state.config.local_region,
		loaded_parent_incarnations,
	});
}

/// A replayed op's store apply answered. A fork publication whose pages may be lost leaves a
/// fork job to copy and publish again; the op is finished when that job publishes.
pub(crate) fn replayed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<crate::tablet_maintainer_machine::types::ReplayOutcome, String>,
	fx: &mut Effects,
) {
	use crate::tablet_maintainer_machine::types::ReplayOutcome;
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::ReplayOp { .. })
	) {
		return;
	}
	let Some(Operation::ReplayOp { token, .. }) = state.finish(op_id, fx) else {
		unreachable!("checked above")
	};
	match result {
		Ok(ReplayOutcome::Applied) => {
			restore_replayed(state, token, fx);
			let audit = match super::maintainer_log::op(state, token) {
				Some(MaintainerOp::RepairAuditOp(op)) => Some(op),
				Some(_) | None => None,
			};
			super::maintainer_log::replay::finished(state, token, fx);
			if let Some(op) = audit {
				super::operator_repair::calls::audit_applied(state, op, fx);
			}
		}
		Ok(ReplayOutcome::RetentionChecked {
			key,
			checkpoint_seq,
		}) => {
			super::filesystem::retention_checked(state, &key, checkpoint_seq);
			super::maintainer_log::replay::finished(state, token, fx)
		}
		Ok(ReplayOutcome::ResumeFork(spec)) => super::forks::resume(state, spec, token),
		Err(error) => {
			tracing::warn!(%error, "replaying a maintainer log op failed in the store; fencing");
			super::fence(state, fx);
		}
	}
}

/// A replayed restore publication is in the store and wrote the replaced incarnation's own
/// record Deleted: that incarnation runs an ordinary delete, as after a live restore.
fn restore_replayed(state: &mut MaintainerState, token: LogToken, fx: &mut Effects) {
	let Some(MaintainerOp::ForkPublishOp(op)) = super::maintainer_log::op(state, token) else {
		return;
	};
	let Some(replaced) = op.spec.restore else {
		return;
	};
	let replaced = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: op.spec.destination_volume_key,
		volume_incarnation: replaced,
	});
	super::deletion::retire(state, replaced, fx);
}

/// Every recovered op of `volume_key` is applied, while other volumes still replay. The
/// unloads, deletions, sweeps and fork cleanups of that volume that waited for its replay may
/// start.
pub(crate) fn volume_replayed(
	state: &mut MaintainerState,
	volume_key: &depot_types::VolumeKey,
	fx: &mut Effects,
) {
	// Marking the unload scope marks the volume's deletions too.
	super::unload::mark_scope(
		state,
		super::unload::UnloadScope::Keys(vec![volume_key.clone()]),
	);
	super::deletion::replay_ended(state, volume_key);
	super::pending_work::release_held_of(state, volume_key, fx);
	super::fork_cleanup::requeue_queued_of(state, volume_key);
	// An incarnation whose record a replayed op of its name wrote registers now.
	state
		.filesystem
		.awaiting_replay
		.retain(|key| key.0.volume_key != *volume_key);
	super::filesystem::load_incarnations(state, volume_key);
}

/// Every recovered op is applied, so the work that waited for replay may start.
pub(crate) fn replay_finished(state: &mut MaintainerState, fx: &mut Effects) {
	super::unload::mark_all(state);
	// Sweeps wait for replay to end, so every deletion may advance now.
	super::deletion::mark_all(state);
	super::deletion::replay_ended_all(state);
	super::pending_work::release_held(state, fx);
	super::fork_cleanup::requeue_queued(state);
	// Incarnations whose records replayed ops of their names wrote register now.
	state.filesystem.awaiting_replay.clear();
	let names = state
		.loaded_volumes
		.keys()
		.map(|key| key.0.volume_key.clone())
		.collect::<std::collections::BTreeSet<_>>();
	for name in names {
		super::filesystem::load_incarnations(state, &name);
	}
}
