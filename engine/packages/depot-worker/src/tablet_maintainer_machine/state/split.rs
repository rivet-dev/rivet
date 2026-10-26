//! Frozen parents prepare immutable child stores before reporting the split ready.

use super::{MaintainerPhase, MaintainerState, Operation, fence};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::{
	SplitPrepared, TabletCoordinatorRejectResponse as Reject, TabletCoordinatorResponse as Response,
};
use serde::{Deserialize, Serialize};

/// A completed checkpoint supplies the exact manifest used by both clone attempts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplitStep {
	/// Retain the split record's named snapshot before projecting any child.
	Checkpoint,
	/// Prepare or verify the left destination from the retained manifest.
	CloneLeft { manifest_id: u64 },
	/// Prepare or verify the right destination after the left is initialized.
	CloneRight { manifest_id: u64 },
	/// Both destinations exist; retry the unchanged coordinator report until accepted.
	Report,
}

/// Active operation records own in-flight inputs; waiting retains identical retry inputs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum SplitPreparation {
	/// No I/O is active; the next dispatch queues this step.
	Ready(SplitStep),
	/// Exactly one split operation owns its immutable inputs and deadline.
	Running,
	/// A failed attempt completed; the retry alarm retains the same inputs.
	Waiting(SplitStep),
	/// The coordinator accepted the report; this parent only serves frozen reads.
	Complete,
}

/// Only the frozen phase can prepare children. Store work joins the same FIFO as reads.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Frozen {
		return;
	}
	let Some(SplitPreparation::Ready(step)) = state.split_preparation.clone() else {
		return;
	};
	if step == SplitStep::Report {
		let split = state.tablet.split.as_ref().expect("frozen split");
		let request = SplitPrepared {
			tablet_id: state.tablet.id,
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			checkpoint_id: split.checkpoint_id,
		};
		state.split_preparation = Some(SplitPreparation::Running);
		let op_id = state.begin(Operation::ReportSplit, effects);
		if state.alarms.enabled {
			effects.execute(MaintainerEffect::ReportAlarmSplitRequest {
				op_id,
				request,
				manifest_id: state
					.alarms
					.split_manifest
					.expect("completed split checkpoint"),
				maintainer: state.tablet.maintainer.expect("assigned split"),
			});
		} else {
			effects.execute(MaintainerEffect::ReportSplitRequest { op_id, request });
		}
	} else if !state
		.store_queue
		.contains(&super::store::StoreWork::PrepareSplit)
	{
		state
			.store_queue
			.push_back(super::store::StoreWork::PrepareSplit);
	}
}

/// Dispatch keeps the immutable step in the operation until its real completion arrives.
pub(crate) fn dispatch(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Frozen {
		return;
	}
	let Some(SplitPreparation::Ready(step)) = state.split_preparation.clone() else {
		return;
	};
	state.split_preparation = Some(SplitPreparation::Running);
	let op_id = state.begin(Operation::PrepareSplit { step: step.clone() }, effects);
	effects.execute(MaintainerEffect::PrepareSplitRequest {
		op_id,
		tablet: state.tablet.clone(),
		step,
	});
}

/// Failures retry the exact checkpoint or clone. A timeout neither cancels the storage I/O nor
/// fences; the operation waits for this, its own result.
pub(crate) fn prepared(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<u64, String>,
	effects: &mut Effects,
) {
	if state.phase != MaintainerPhase::Frozen
		|| !matches!(
			state.operations.get(&op_id),
			Some(Operation::PrepareSplit { .. })
		) {
		return;
	}
	let Some(Operation::PrepareSplit { step }) = state.finish(op_id, effects) else {
		unreachable!()
	};
	let Ok(manifest_id) = result else {
		wait(state, step, effects);
		return;
	};
	let next = match step {
		SplitStep::Checkpoint => SplitStep::CloneLeft { manifest_id },
		SplitStep::CloneLeft { manifest_id } => SplitStep::CloneRight { manifest_id },
		SplitStep::CloneRight { manifest_id } => {
			state.alarms.split_manifest = Some(manifest_id);
			SplitStep::Report
		}
		SplitStep::Report => unreachable!("reports use their own transport"),
	};
	state.split_preparation = Some(SplitPreparation::Ready(next));
}

/// Only coordinator acceptance completes preparation; moved authority ends this lifetime.
pub(crate) fn reported(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<Response, String>,
	effects: &mut Effects,
) {
	if state.phase != MaintainerPhase::Frozen
		|| !matches!(state.operations.get(&op_id), Some(Operation::ReportSplit))
	{
		return;
	}
	state.finish(op_id, effects);
	match result {
		Ok(Response::Accepted(_)) => state.split_preparation = Some(SplitPreparation::Complete),
		Ok(Response::StopPermitted | Response::StopPending(_))
		| Err(_)
		| Ok(Response::TabletCoordinatorRejectResponse(
			Reject::NotCoordinator | Reject::Overloaded,
		)) => wait(state, SplitStep::Report, effects),
		Ok(Response::TabletCoordinatorRejectResponse(
			Reject::StaleTabletGrantEpoch(_)
			| Reject::StaleTabletReplicaSet(_)
			| Reject::UnknownTablet
			| Reject::ReconfigurationInProgress
			| Reject::SplitInProgress
			| Reject::InvalidCommand
			| Reject::VersionUnreadable(_),
		)) => fence(state, effects),
	}
}

/// Cancel only the transport attempt; remote acceptance is reconciled by repeating the report.
pub(crate) fn report_timeout(state: &mut MaintainerState, op_id: OpId, effects: &mut Effects) {
	effects.execute(MaintainerEffect::CancelSplitReportRequest { op_id });
	reported(state, op_id, Err("split report timed out".into()), effects);
}

/// Preserve retry inputs after a terminal attempt result; no storage call is still active.
fn wait(state: &mut MaintainerState, step: SplitStep, effects: &mut Effects) {
	state.split_preparation = Some(SplitPreparation::Waiting(step));
	effects.arm_timer_after(MaintainerTimer::SplitRetry, state.config.tuning().retry_ms);
}

/// A retry cannot allocate new child identities or change a checkpoint after an uncertain result.
pub(crate) fn retry(state: &mut MaintainerState) {
	if state.phase != MaintainerPhase::Frozen {
		return;
	}
	if let Some(SplitPreparation::Waiting(step)) = state.split_preparation.clone() {
		state.split_preparation = Some(SplitPreparation::Ready(step));
	}
}
