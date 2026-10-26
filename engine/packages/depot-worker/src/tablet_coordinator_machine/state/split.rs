//! A split publishes its frozen parent and both children through manifest CAS writes.

use super::Reject;
use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::{
		ParentLogImported, RetiredTabletDeleted, SplitPrepared, SplitTablet, StaleTabletGrantEpoch,
	},
	depot_tablet_manifest_v1::{RetiredTablet, Tablet, TabletManifest, TabletSplit},
	shared_depot_types_v1::TabletPhase,
};
use std::collections::BTreeSet;

/// Reserve every live, retired and not-yet-published identity before accepting a manifest.
pub(super) fn validate(manifest: &TabletManifest) -> anyhow::Result<()> {
	let mut ids = BTreeSet::new();
	for tablet in &manifest.tablets {
		anyhow::ensure!(
			tablet.id != [0; 16] && ids.insert(tablet.id),
			"invalid tablet identity"
		);
		anyhow::ensure!(
			tablet.tablet_grant_epoch < u64::MAX,
			"retired epoch cannot grant a live tablet"
		);
		anyhow::ensure!(
			tablet.parent_tablet_id != Some(tablet.id),
			"tablet cannot be its own parent"
		);
	}
	for retired in &manifest.retired_tablets {
		anyhow::ensure!(
			manifest
				.tablets
				.iter()
				.filter(|tablet| tablet.hash_range.contains(retired.partition_key_hash_start))
				.count() == 1,
			"retired tablet must have exactly one cleanup owner"
		);
		anyhow::ensure!(
			retired.tablet_id != [0; 16]
				&& retired.checkpoint_id != [0; 16]
				&& ids.insert(retired.tablet_id),
			"invalid retired tablet identity"
		);
		anyhow::ensure!(
			retired.children.len() == 2
				&& retired
					.log_imported_by
					.iter()
					.all(|child| retired.children.contains(child)),
			"retired tablet must name its two children and only their imports"
		);
	}
	for tablet in &manifest.tablets {
		if let Some(split) = &tablet.split {
			anyhow::ensure!(
				tablet.replica_set.phase == TabletPhase::Stable && tablet.maintainer.is_some(),
				"split needs a stable granted tablet"
			);
			anyhow::ensure!(
				tablet.hash_range.start < split.split_key_hash
					&& tablet.hash_range.contains(split.split_key_hash),
				"split is outside tablet range"
			);
			anyhow::ensure!(
				split.checkpoint_id != [0; 16]
					&& split.left_tablet_id != [0; 16]
					&& split.right_tablet_id != [0; 16]
					&& ids.insert(split.left_tablet_id)
					&& ids.insert(split.right_tablet_id),
				"invalid split identities"
			);
			anyhow::ensure!(
				tablet.tablet_grant_epoch < u64::MAX - 1,
				"split has no child grant epoch left"
			);
		}
	}
	Ok(())
}

/// Freeze with one higher grant and retain the same children/checkpoint across retries.
pub(super) fn start(
	manifest: &mut TabletManifest,
	request: &SplitTablet,
	proposal: &TabletSplit,
	now: i64,
) -> Result<bool, Reject> {
	let index = manifest
		.tablets
		.iter()
		.position(|tablet| tablet.id == request.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	let parent = &manifest.tablets[index];
	if let Some(split) = &parent.split {
		return if split.split_key_hash == request.split_key_hash {
			Ok(false)
		} else {
			Err(Reject::SplitInProgress)
		};
	}
	if parent.replica_set.phase != TabletPhase::Stable {
		return Err(Reject::ReconfigurationInProgress);
	}
	// A new freeze grants a higher epoch, so it cannot allocate work to a draining owner.
	// Existing freezes return above and may still complete through SplitPrepared.
	if parent.maintainer.is_none_or(|owner| {
		manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == owner)
	}) || request.split_key_hash <= parent.hash_range.start
		|| !parent.hash_range.contains(request.split_key_hash)
		|| !rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange::is_split_aligned(
			request.split_key_hash,
		) || proposal.split_key_hash != request.split_key_hash
	{
		return Err(Reject::InvalidCommand);
	}
	// Leave another epoch for the child stores, which inherit the frozen parent's epoch.
	let epoch = parent
		.tablet_grant_epoch
		.checked_add(1)
		.filter(|epoch| *epoch < u64::MAX - 1)
		.ok_or(Reject::InvalidCommand)?;
	let mut candidate = manifest.clone();
	let parent = &mut candidate.tablets[index];
	parent.split = Some(proposal.clone());
	parent.tablet_grant_epoch = epoch;
	parent.granted_at = now;
	validate(&candidate).map_err(|_| Reject::InvalidCommand)?;
	*manifest = candidate;
	Ok(true)
}

/// A frozen owner's exact checkpoint publishes both clones atomically. The replica
/// set stays unchanged, so existing clients need neither a new version nor a new claim.
pub(in crate::tablet_coordinator_machine) fn prepared(
	manifest: &mut TabletManifest,
	report: &SplitPrepared,
	now: i64,
) -> Result<bool, Reject> {
	let Some(index) = manifest
		.tablets
		.iter()
		.position(|tablet| tablet.id == report.tablet_id)
	else {
		if let Some(retired) = manifest
			.retired_tablets
			.iter()
			.find(|tablet| tablet.tablet_id == report.tablet_id)
		{
			return if retired.checkpoint_id == report.checkpoint_id {
				Ok(false)
			} else {
				Err(Reject::InvalidCommand)
			};
		}
		// Cleanup may already have removed the retired record before a duplicate report.
		return if manifest
			.tablets
			.iter()
			.any(|tablet| tablet.parent_tablet_id == Some(report.tablet_id))
		{
			Ok(false)
		} else {
			Err(Reject::UnknownTablet)
		};
	};
	let parent = &manifest.tablets[index];
	if report.tablet_grant_epoch != parent.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: parent.tablet_grant_epoch,
		}));
	}
	let split = parent
		.split
		.as_ref()
		.filter(|split| split.checkpoint_id == report.checkpoint_id)
		.ok_or(Reject::InvalidCommand)?;
	let epoch = parent
		.tablet_grant_epoch
		.checked_add(1)
		.filter(|epoch| *epoch < u64::MAX)
		.ok_or(Reject::InvalidCommand)?;
	let child = |id, start, end| Tablet {
		id,
		hash_range: rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange {
			start: start,
			end: end,
		},
		maintainer: parent.maintainer,
		tablet_grant_epoch: epoch,
		granted_at: now,
		replica_set: parent.replica_set.clone(),
		phase_started_at: parent.phase_started_at,
		split: None,
		parent_tablet_id: Some(parent.id),
		parent_range_start: Some(parent.hash_range.start),
		// Each child's store is a clone of the parent's data.
		version_marks: parent.version_marks.clone(),
	};
	let left = child(
		split.left_tablet_id,
		parent.hash_range.start,
		Some(split.split_key_hash),
	);
	let right = child(
		split.right_tablet_id,
		split.split_key_hash,
		parent.hash_range.end,
	);
	let retired = RetiredTablet {
		partition_key_hash_start: parent.hash_range.start,
		tablet_id: parent.id,
		checkpoint_id: split.checkpoint_id,
		children: vec![split.left_tablet_id, split.right_tablet_id],
		log_imported_by: Vec::new(),
	};
	let parent_id = parent.id;
	manifest.tablets.remove(index);
	manifest.tablets.extend([left, right]);
	// A pause holds the balancer off one tablet; its children are new tablets.
	manifest.paused.retain(|pause| pause.tablet_id != parent_id);
	// A child that is itself retired never reports its import: its own children read only
	// its log, so its parent's log is not needed for it any more.
	for older in manifest.retired_tablets.iter_mut() {
		if older.children.contains(&parent_id) && !older.log_imported_by.contains(&parent_id) {
			older.log_imported_by.push(parent_id);
		}
	}
	manifest.retired_tablets.push(retired);
	Ok(true)
}

/// A child's current maintainer logged the parent's ops into its own log: the parent's log
/// replicas are no longer needed by that child. Recorded once per child; a report for a
/// retired record already cleaned up, or by a maintainer other than the child's current one
/// at its grant, changes nothing or is refused.
pub(super) fn imported(
	manifest: &mut TabletManifest,
	report: &ParentLogImported,
) -> Result<bool, Reject> {
	let child = manifest
		.tablets
		.iter()
		.find(|tablet| tablet.id == report.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	if child.tablet_grant_epoch != report.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: child.tablet_grant_epoch,
		}));
	}
	if child.maintainer != Some(report.maintainer) {
		return Err(Reject::InvalidCommand);
	}
	let Some(retired) = manifest
		.retired_tablets
		.iter_mut()
		.find(|retired| retired.tablet_id == report.parent_tablet_id)
	else {
		return Ok(false);
	};
	if !retired.children.contains(&report.tablet_id) {
		return Err(Reject::InvalidCommand);
	}
	if retired.log_imported_by.contains(&report.tablet_id) {
		return Ok(false);
	}
	retired.log_imported_by.push(report.tablet_id);
	Ok(true)
}

/// The cleanup executor confirms deletion; repeated notifications are no-ops.
pub(super) fn deleted(
	manifest: &mut TabletManifest,
	report: &RetiredTabletDeleted,
) -> Result<bool, Reject> {
	let id = report.tablet_id;
	if manifest.tablets.iter().any(|tablet| tablet.id == id) {
		return Err(Reject::InvalidCommand);
	}
	let Some(retired) = manifest
		.retired_tablets
		.iter()
		.find(|retired| retired.tablet_id == id)
	else {
		return Ok(false);
	};
	let owner = manifest
		.tablets
		.iter()
		.find(|tablet| tablet.hash_range.contains(retired.partition_key_hash_start))
		.ok_or(Reject::InvalidCommand)?;
	if owner.tablet_grant_epoch != report.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: owner.tablet_grant_epoch,
		}));
	}
	if owner.maintainer != Some(report.maintainer) {
		return Err(Reject::InvalidCommand);
	}
	// The parent's log replicas are forgotten before the report; every child must have
	// imported the log first, or the report is premature.
	if !retired
		.children
		.iter()
		.all(|child| retired.log_imported_by.contains(child))
	{
		return Err(Reject::InvalidCommand);
	}
	let before = manifest.retired_tablets.len();
	manifest
		.retired_tablets
		.retain(|tablet| tablet.tablet_id != id);
	Ok(manifest.retired_tablets.len() != before)
}
