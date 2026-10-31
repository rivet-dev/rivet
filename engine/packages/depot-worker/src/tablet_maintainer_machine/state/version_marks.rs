//! Tablet version marks: the highest version of each family written into the tablet's
//! SlateDB data. The record row holds them; opening refuses a record whose marks this binary
//! cannot read. The maintainer reports marks that rose above the manifest's to the
//! coordinator, which never grants the tablet to a worker that cannot read them.

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use rivet_data::{
	families::{VersionFamily, mark_of, merge_marks, raise_mark},
	generated::{
		depot_tablet_coordinator_protocol_v1::{
			TabletCoordinatorRejectResponse, TabletCoordinatorResponse, VersionMarksReport,
		},
		shared_depot_types_v1::VersionMark,
	},
};
use serde::{Deserialize, Serialize};

/// Report progress for the current grant. Reset with the rest of the serving state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct VersionReport {
	/// Marks the coordinator accepted under this grant. The next assignment carries them in
	/// the manifest, so until then they are not reported again.
	pub accepted: Vec<VersionMark>,
	/// A failed report waits for `MaintainerTimer::VersionMarksRetry`.
	pub waiting_retry: bool,
	/// The coordinator refused a report under this grant epoch for good, such as for a stale
	/// grant. No report is sent again under it; a newer grant reports afresh.
	pub abandoned_grant: Option<u64>,
	/// Opening refused the store's marks and sent them in a last report. Whatever its outcome,
	/// including the operation timeout, the maintainer then fences.
	pub refusing: bool,
}

/// Raises `marks` to the versions this grant writes into tablet data, `versions`: the record
/// itself at `depot.maintainer_log`, and SlateDB row values at `depot.fs_rows` once this mark
/// is durable. Returns whether any rose.
pub(crate) fn raise_written(
	marks: &mut Vec<VersionMark>,
	versions: &crate::tablet_maintainer_machine::types::TabletWriteVersions,
) -> bool {
	let mut raised = raise_mark(marks, VersionFamily::DEPOT_FS_ROWS, versions.fs_rows);
	raised |= raise_mark(
		marks,
		VersionFamily::DEPOT_MAINTAINER_LOG,
		versions.maintainer_log,
	);
	raised
}

/// The marks the tablet's data holds as this maintainer knows them, including ones a
/// dispatched pass is writing.
fn current(state: &MaintainerState) -> &[VersionMark] {
	state
		.maintainer_log
		.record
		.as_ref()
		.map_or(&[], |record| record.version_marks.as_slice())
}

/// Whether the manifest, together with marks already accepted under this grant, covers
/// every current mark.
/// Families whose version 1 every binary that records version marks reads. A mark at
/// version 1 of one of them can never refuse a grant, so it is not reported, which keeps the
/// first pass of every tablet from writing the manifest. A family added to `raise_written`
/// later must not be listed here: binaries that predate it cannot read even its version 1.
const BASELINE: [VersionFamily; 2] = [
	VersionFamily::DEPOT_FS_ROWS,
	VersionFamily::DEPOT_MAINTAINER_LOG,
];

/// Whether the coordinator needs `mark` to refuse a worker that cannot read it.
fn reportable(mark: &VersionMark) -> bool {
	mark.version > 1 || !BASELINE.contains(&VersionFamily(mark.family))
}

/// Runs after every event, so it allocates nothing.
fn covered(state: &MaintainerState) -> bool {
	current(state)
		.iter()
		.filter(|mark| reportable(mark))
		.all(|mark| {
			let family = VersionFamily(mark.family);
			let known = mark_of(&state.tablet.version_marks, family)
				.max(mark_of(&state.version_report.accepted, family));
			known.is_some_and(|known| known >= mark.version)
		})
}

/// Reports marks the manifest lacks. One report at a time; a failed one waits for its retry.
pub(crate) fn drive(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Serving
		|| state.version_report.waiting_retry
		|| state.version_report.abandoned_grant == Some(state.tablet.tablet_grant_epoch)
		|| state.operations.index().counts.report_version_marks > 0
		|| covered(state)
	{
		return;
	}
	let Some(maintainer) = state.tablet.maintainer else {
		return;
	};
	let version_marks: Vec<VersionMark> = current(state)
		.iter()
		.filter(|mark| reportable(mark))
		.cloned()
		.collect();
	let op_id = state.begin(
		Operation::ReportVersionMarks {
			version_marks: version_marks.clone(),
		},
		effects,
	);
	effects.execute(MaintainerEffect::ReportVersionMarksRequest {
		op_id,
		request: VersionMarksReport {
			tablet_id: state.tablet.id,
			maintainer,
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			version_marks,
		},
	});
}

/// Opening refused the store's `marks`. Reports them, so the coordinator records them and
/// stops granting the tablet to workers that cannot read them, then fences once the report
/// answers or its operation timer fires. Returns false when there is nothing to report or no
/// grant to report under; the caller then fences at once.
pub(crate) fn report_refusal(
	state: &mut MaintainerState,
	marks: &[VersionMark],
	effects: &mut Effects,
) -> bool {
	let Some(maintainer) = state.tablet.maintainer else {
		return false;
	};
	let mut version_marks = marks.to_vec();
	rivet_data::families::normalize_marks(&mut version_marks);
	version_marks.retain(reportable);
	if version_marks.is_empty() || state.operations.index().counts.report_version_marks > 0 {
		return false;
	}
	state.version_report.refusing = true;
	let op_id = state.begin(
		Operation::ReportVersionMarks {
			version_marks: version_marks.clone(),
		},
		effects,
	);
	effects.execute(MaintainerEffect::ReportVersionMarksRequest {
		op_id,
		request: VersionMarksReport {
			tablet_id: state.tablet.id,
			maintainer,
			tablet_grant_epoch: state.tablet.tablet_grant_epoch,
			version_marks,
		},
	});
	true
}

/// Whether a refused report can never be accepted under the grant it named.
fn permanent(response: &TabletCoordinatorResponse) -> bool {
	match response {
		TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			TabletCoordinatorRejectResponse::NotCoordinator
			| TabletCoordinatorRejectResponse::Overloaded,
		) => false,
		TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			TabletCoordinatorRejectResponse::StaleTabletGrantEpoch(_)
			| TabletCoordinatorRejectResponse::StaleTabletReplicaSet(_)
			| TabletCoordinatorRejectResponse::UnknownTablet
			| TabletCoordinatorRejectResponse::ReconfigurationInProgress
			| TabletCoordinatorRejectResponse::SplitInProgress
			| TabletCoordinatorRejectResponse::InvalidCommand
			| TabletCoordinatorRejectResponse::VersionUnreadable(_),
		)
		| TabletCoordinatorResponse::StopPermitted
		| TabletCoordinatorResponse::StopPending(_)
		| TabletCoordinatorResponse::Accepted(_) => true,
	}
}

/// Acceptance records the reported marks. A transport failure, a coordinator that is not
/// current or one that is overloaded retries after `retry_ms`; any other refusal is permanent
/// for this grant, so reporting stops until a newer grant. A refusal report fences on any
/// outcome.
pub(crate) fn reported(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<TabletCoordinatorResponse, String>,
	effects: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::ReportVersionMarks { .. })
	) {
		return;
	}
	let Some(Operation::ReportVersionMarks { version_marks }) = state.finish(op_id, effects) else {
		return;
	};
	if state.version_report.refusing {
		state.version_report.refusing = false;
		super::fence(state, effects);
		return;
	}
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	match &result {
		Ok(TabletCoordinatorResponse::Accepted(_)) => {
			merge_marks(&mut state.version_report.accepted, &version_marks);
		}
		Ok(response) if permanent(response) => {
			tracing::warn!(
				tablet_id = %uuid::Uuid::from_bytes(state.tablet.id),
				tablet_grant_epoch = state.tablet.tablet_grant_epoch,
				?response,
				"coordinator refused the tablet's version marks for this grant; not reporting them again under it"
			);
			state.version_report.abandoned_grant = Some(state.tablet.tablet_grant_epoch);
		}
		Ok(_) | Err(_) => {
			state.version_report.waiting_retry = true;
			effects.arm_timer_after(
				MaintainerTimer::VersionMarksRetry,
				state.config.tuning().retry_ms,
			);
		}
	}
}

pub(crate) fn report_timeout(state: &mut MaintainerState, op_id: OpId, effects: &mut Effects) {
	effects.execute(MaintainerEffect::CancelVersionMarksReportRequest { op_id });
	reported(
		state,
		op_id,
		Err("version marks report timed out".into()),
		effects,
	);
}

pub(crate) fn retry(state: &mut MaintainerState) {
	state.version_report.waiting_retry = false;
}

/// Serving ended: drop report progress. A later grant starts from its own manifest entry.
pub(crate) fn cancel(state: &mut MaintainerState, effects: &mut Effects) {
	state.version_report = VersionReport::default();
	effects.cancel_timer(MaintainerTimer::VersionMarksRetry);
}
