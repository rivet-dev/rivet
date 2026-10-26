//! Reports a split child's import of its parent's maintainer log to the coordinator. The
//! retired parent's cleanup owner forgets the parent's log replicas once the manifest records
//! every child's import; a child that never reported would keep one small log on its
//! replicas for good.
//!
//! The import is durable before the report: a child's own log serves only after it logged the
//! parent's ops (`maintainer_log::parent_imported`), and the report names nothing but the
//! child and its parent. It is idempotent at the coordinator, retried under this grant until
//! accepted or until a manifest shows it recorded, and dropped with the grant.

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::{
	ParentLogImported, TabletCoordinatorResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Report {
	/// This tablet's own log holds its parent's ops, so the parent's log is not needed here.
	pub(crate) imported: bool,
	/// The manifest's retired record of the parent lists this tablet, or is gone.
	pub(crate) recorded: bool,
	pub(crate) step: Step,
	/// A refusal was logged for this report; later refusals are debug until it is accepted.
	pub(crate) warned: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Step {
	/// Nothing to report: not imported yet, already recorded, or not a split child.
	#[default]
	Idle,
	/// The import is not recorded; the next drive reports it.
	Due,
	/// A report is in flight; its operation owns the deadline.
	Running,
	/// The last report was not accepted; `ParentLogReportRetry` makes it due again.
	Waiting,
}

/// The log module imported the parent's ops into this tablet's own log.
pub fn imported(state: &mut MaintainerState) {
	let report = &mut state.parent_log;
	report.imported = true;
	if !report.recorded && report.step == Step::Idle && state.tablet.parent_tablet_id.is_some() {
		report.step = Step::Due;
	}
}

/// The supervisor's manifest view says whether the parent's retired record lists this tablet,
/// or no longer exists. A recorded import stays recorded: the record only gains imports or
/// goes away, so a view that lacks it is older than one that showed it.
pub(crate) fn recorded(state: &mut MaintainerState, recorded: bool, fx: &mut Effects) {
	let report = &mut state.parent_log;
	if recorded {
		report.recorded = true;
		if report.step == Step::Waiting {
			fx.cancel_timer(MaintainerTimer::ParentLogReportRetry);
		}
		if report.step != Step::Running {
			report.step = Step::Idle;
		}
		return;
	}
	if !report.recorded
		&& report.imported
		&& report.step == Step::Idle
		&& state.tablet.parent_tablet_id.is_some()
	{
		report.step = Step::Due;
	}
}

/// Sends the due report under the current grant. A frozen child reports too: the import it
/// proves is durable, and its own split will retire it before it serves again.
pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) || state.parent_log.step != Step::Due
	{
		return;
	}
	let (Some(parent), Some(maintainer)) = (state.tablet.parent_tablet_id, state.tablet.maintainer)
	else {
		state.parent_log.step = Step::Idle;
		return;
	};
	if state.operations.index().counts.report_parent_log > 0 {
		return;
	}
	state.parent_log.step = Step::Running;
	let op_id = state.begin(Operation::ReportParentLogImported, fx);
	fx.execute(MaintainerEffect::ReportParentLogImportedRequest {
		op_id,
		request: ParentLogImported {
			tablet_id: state.tablet.id,
			maintainer,
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			parent_tablet_id: parent,
		},
	});
}

/// The coordinator answered, or the transport failed. Acceptance records the import; any
/// other answer waits for the retry interval.
pub(crate) fn reported(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<TabletCoordinatorResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::ReportParentLogImported)
	) {
		return;
	}
	state.finish(op_id, fx);
	if state.parent_log.step != Step::Running {
		return;
	}
	// A manifest recorded the import while the report was in flight.
	if matches!(result, Ok(TabletCoordinatorResponse::Accepted(_))) || state.parent_log.recorded {
		state.parent_log.recorded = true;
		state.parent_log.step = Step::Idle;
		state.parent_log.warned = false;
		return;
	}
	if let Ok(response) = &result
		&& permanent(response)
	{
		if state.parent_log.warned {
			tracing::debug!(?response, "parent log import report refused again");
		} else {
			state.parent_log.warned = true;
			tracing::warn!(
				?response,
				"parent log import report refused for good under this grant; retrying until the grant moves"
			);
		}
	}
	state.parent_log.step = Step::Waiting;
	fx.arm_timer_after(
		MaintainerTimer::ParentLogReportRetry,
		state.config.tuning().retry_ms,
	);
}

/// Whether the coordinator's answer refuses this report for good under the current grant:
/// anything but an accepted report or an overloaded or non-coordinating answer.
fn permanent(response: &TabletCoordinatorResponse) -> bool {
	use rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorRejectResponse as Reject;
	match response {
		TabletCoordinatorResponse::Accepted(_)
		| TabletCoordinatorResponse::StopPermitted
		| TabletCoordinatorResponse::StopPending(_)
		| TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			Reject::Overloaded | Reject::NotCoordinator,
		) => false,
		TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			Reject::StaleTabletGrantEpoch(_)
			| Reject::StaleTabletReplicaSet(_)
			| Reject::UnknownTablet
			| Reject::ReconfigurationInProgress
			| Reject::SplitInProgress
			| Reject::InvalidCommand
			| Reject::VersionUnreadable(_),
		) => true,
	}
}

pub(crate) fn report_timeout(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	fx.execute(MaintainerEffect::CancelParentLogReportRequest { op_id });
	reported(state, op_id, Err("parent log report timed out".into()), fx);
}

/// The retry interval passed: an unaccepted report is due again.
pub(crate) fn retry(state: &mut MaintainerState) {
	if state.parent_log.step == Step::Waiting {
		state.parent_log.step = Step::Due;
	}
}
