//! Replica-set phase writes use the same manifest CAS as placement and grants.

use super::Reject;
use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::{
		CatchUpReport, ReplaceReplica, StaleTabletGrantEpoch, StaleTabletReplicaSet,
	},
	depot_tablet_manifest_v1::{Tablet, TabletManifest},
	shared_depot_types_v1::{Joint, Learning, TabletPhase, TabletReplicaSet},
};
use std::collections::BTreeSet;

/// Shared membership rules keep all protocol participants on the same voting lists.
pub(super) fn validate(set: &TabletReplicaSet) -> anyhow::Result<()> {
	depot_types::membership::validate(set)
}

/// Keep a granted maintainer in both configurations so promotion never moves a grant implicitly.
pub(super) fn may_maintain(tablet: &Tablet, worker: &[u8; 16]) -> bool {
	tablet.replica_set.members.contains(worker)
		&& match &tablet.replica_set.phase {
			TabletPhase::Stable => true,
			TabletPhase::Learning(phase) => phase.next.contains(worker),
			TabletPhase::Joint(phase) => phase.next.contains(worker),
		}
}

pub(super) fn replace(
	manifest: &mut TabletManifest,
	request: &ReplaceReplica,
	now: i64,
) -> Result<bool, Reject> {
	let current = manifest
		.tablets
		.iter()
		.find(|t| t.id == request.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	if super::stop::blocks_reconfiguration(manifest, current, Some(&request.leaving))
		|| manifest.stopping.iter().any(|entry| {
			entry.worker_id == request.joining.worker_id && entry.permitted_at.is_some()
		}) {
		return Err(Reject::ReconfigurationInProgress);
	}
	let tablet = tablet(manifest, &request.tablet_id)?;
	if tablet.split.is_some() {
		return Err(Reject::SplitInProgress);
	}
	if tablet.replica_set.phase != TabletPhase::Stable {
		return Err(Reject::ReconfigurationInProgress);
	}
	if !tablet.replica_set.members.contains(&request.leaving)
		|| tablet
			.replica_set
			.members
			.contains(&request.joining.worker_id)
		|| request.joining.disk_incarnation == [0; 16]
		|| tablet.maintainer == Some(request.leaving)
	{
		return Err(Reject::InvalidCommand);
	}

	let mut next = tablet.replica_set.members.clone();
	*next
		.iter_mut()
		.find(|id| **id == request.leaving)
		.expect("validated leaving member") = request.joining.worker_id;
	write_phase(tablet, TabletPhase::Learning(Learning { next }), now)?;
	tablet.replica_set.disks.push(request.joining.clone());
	Ok(true)
}

/// Reuse Learning/Joint for a one-member expansion or graceful reduction.
pub(super) fn resize(
	manifest: &mut TabletManifest,
	id: &[u8; 16],
	next: Vec<[u8; 16]>,
	joining: Option<rivet_data::generated::shared_depot_types_v1::StorageWorkerDisk>,
	now: i64,
) -> Result<bool, Reject> {
	let current = manifest
		.tablets
		.iter()
		.find(|t| &t.id == id)
		.ok_or(Reject::UnknownTablet)?;
	let leaving = current
		.replica_set
		.members
		.iter()
		.find(|id| !next.contains(id));
	if super::stop::blocks_reconfiguration(manifest, current, leaving) {
		return Err(Reject::ReconfigurationInProgress);
	}
	let tablet = tablet(manifest, id)?;
	if tablet.split.is_some() {
		return Err(Reject::SplitInProgress);
	}
	if tablet.replica_set.phase != TabletPhase::Stable {
		return Err(Reject::ReconfigurationInProgress);
	}
	if tablet.maintainer.is_none_or(|id| !next.contains(&id)) {
		return Err(Reject::InvalidCommand);
	}
	let mut set = tablet.replica_set.clone();
	set.phase = TabletPhase::Learning(Learning { next });
	if let Some(disk) = joining {
		set.disks.push(disk);
	}
	validate(&set).map_err(|_| Reject::InvalidCommand)?;
	write_phase(tablet, set.phase, now)?;
	tablet.replica_set.disks = set.disks;
	Ok(true)
}

/// Default bound on a Learning phase. It must outlast the membership holder wait (15
/// minutes by default) plus the copy of a large log suffix, so a slow but progressing
/// learner finishes; a learner that cannot be reached or cannot catch up is aborted.
pub const DEFAULT_LEARNING_DEADLINE_MS: u64 = 60 * 60 * 1000;

pub(super) fn abort(
	manifest: &mut TabletManifest,
	id: &[u8; 16],
	now: i64,
) -> Result<bool, Reject> {
	let tablet = tablet(manifest, id)?;
	if tablet.split.is_some() {
		return Err(Reject::SplitInProgress);
	}
	match &tablet.replica_set.phase {
		TabletPhase::Stable => Ok(false),
		TabletPhase::Learning(_) => {
			write_phase(tablet, TabletPhase::Stable, now)?;
			tablet
				.replica_set
				.disks
				.retain(|disk| tablet.replica_set.members.contains(&disk.worker_id));
			Ok(true)
		}
		TabletPhase::Joint(_) => Err(Reject::ReconfigurationInProgress),
	}
}

/// Aborts every Learning phase that started at least `deadline_ms` before `now`, and every
/// one whose learner is in `missing`: assigned workers that storage discovery has not seen on
/// their assigned disk for the replacement grace. A learner never votes in Learning, so the
/// abort only returns the tablet to its old members, which still hold every committed entry;
/// the balancer or an operator can then start a new change. A split requires a Stable parent,
/// so no Learning tablet is splitting. Returns the aborted tablets and the next deadline, in
/// ms from `now`, of the Learning phases left running.
pub(super) fn expire_learning(
	manifest: &mut TabletManifest,
	deadline_ms: u64,
	missing: &BTreeSet<[u8; 16]>,
	now: i64,
) -> (Vec<[u8; 16]>, Option<u64>) {
	let deadline_ms = i64::try_from(deadline_ms).unwrap_or(i64::MAX);
	let mut expired = Vec::new();
	let mut next = None::<u64>;
	for tablet in &mut manifest.tablets {
		let TabletPhase::Learning(phase) = &tablet.replica_set.phase else {
			continue;
		};
		let phase_age_ms = now.saturating_sub(tablet.phase_started_at);
		let learner = phase
			.next
			.iter()
			.find(|worker| !tablet.replica_set.members.contains(worker))
			.copied();
		let reason = if phase_age_ms >= deadline_ms {
			"the learner did not catch up within the Learning deadline; check the joining worker's reachability and disk"
		} else if learner.is_some_and(|learner| missing.contains(&learner)) {
			"the learner left storage discovery or its disk for the replacement grace; the balancer can choose another worker"
		} else {
			let remaining = u64::try_from(deadline_ms - phase_age_ms).unwrap_or(u64::MAX);
			next = Some(next.map_or(remaining, |next| next.min(remaining)));
			continue;
		};
		if write_phase(tablet, TabletPhase::Stable, now).is_err() {
			continue;
		}
		tablet
			.replica_set
			.disks
			.retain(|disk| tablet.replica_set.members.contains(&disk.worker_id));
		tracing::warn!(
			tablet_id = %uuid::Uuid::from_bytes(tablet.id),
			phase_age_ms,
			deadline_ms,
			learner = ?learner.map(uuid::Uuid::from_bytes),
			reason,
			"aborting a tablet replica-set change in Learning; the tablet stays on its old members"
		);
		expired.push(tablet.id);
	}
	(expired, next)
}

/// A report advances only the exact grant and phase observed by its maintainer.
/// Repeated old reports see the newer version instead of advancing the next phase twice.
pub(super) fn report(
	manifest: &mut TabletManifest,
	report: &CatchUpReport,
	now: i64,
) -> Result<bool, Reject> {
	let current = manifest
		.tablets
		.iter()
		.find(|t| t.id == report.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	if let TabletPhase::Learning(phase) = &current.replica_set.phase {
		let leaving = current
			.replica_set
			.members
			.iter()
			.find(|id| !phase.next.contains(id));
		if super::stop::blocks_reconfiguration(manifest, current, leaving) {
			return Err(Reject::ReconfigurationInProgress);
		}
	}
	let tablet = tablet(manifest, &report.tablet_id)?;
	if tablet.tablet_grant_epoch != report.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: tablet.tablet_grant_epoch,
		}));
	}
	if tablet.replica_set.tablet_replica_set_version != report.tablet_replica_set_version {
		return Err(Reject::StaleTabletReplicaSet(StaleTabletReplicaSet {
			tablet_replica_set_version: tablet.replica_set.tablet_replica_set_version,
		}));
	}
	if tablet.maintainer.is_none() {
		return Err(Reject::InvalidCommand);
	}
	if tablet.split.is_some() {
		return Err(Reject::SplitInProgress);
	}
	match &tablet.replica_set.phase {
		TabletPhase::Stable => Err(Reject::InvalidCommand),
		TabletPhase::Learning(phase) => {
			let next = phase.next.clone();
			write_phase(tablet, TabletPhase::Joint(Joint { next }), now)
		}
		TabletPhase::Joint(phase) => {
			let members = phase.next.clone();
			write_phase(tablet, TabletPhase::Stable, now)?;
			tablet.replica_set.members = members;
			tablet
				.replica_set
				.disks
				.retain(|disk| tablet.replica_set.members.contains(&disk.worker_id));
			Ok(true)
		}
	}
}

fn tablet<'a>(manifest: &'a mut TabletManifest, id: &[u8; 16]) -> Result<&'a mut Tablet, Reject> {
	manifest
		.tablets
		.iter_mut()
		.find(|tablet| &tablet.id == id)
		.ok_or(Reject::UnknownTablet)
}

/// Check overflow before mutating either the phase or its diagnostic timestamp.
fn write_phase(tablet: &mut Tablet, phase: TabletPhase, now: i64) -> Result<bool, Reject> {
	let version = tablet
		.replica_set
		.tablet_replica_set_version
		.checked_add(1)
		.ok_or(Reject::InvalidCommand)?;
	tablet.replica_set.phase = phase;
	tablet.replica_set.tablet_replica_set_version = version;
	tablet.phase_started_at = now;
	Ok(true)
}
