//! Quarantine repair: rebuild the record of an incarnation whose stored record is lost.
//!
//! A quarantined incarnation (see `Filesystem::record_missing`) has replicas above cold and no
//! record, and the maintainer never writes one on its own: only the store knows whether the
//! rows the lost record covered survived. An operator who checked them decides. The rebuild
//! takes the record the replicas' creation evidence and cold positions describe, which is what
//! a first record or an unload would write, registers the incarnation as a loaded volume with
//! a first record, and queues that record for the tablet's ordinary pass. Nothing is written
//! at or below cold except the record, no replica is changed, and no row is touched: the
//! filesystem apply resumes from the record's cold position as for any other volume.
//!
//! The rebuilt record has no write receipts. A client that retries a write at or below the
//! record's cold position after the rebuild is not deduplicated against it.

use super::calls::{refusal, refused};
use crate::repair_types::RepairPrepared;
use crate::tablet_maintainer_machine::{Effects, state::MaintainerState};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		RepairPlan, RepairRebuildPlan, RepairRebuildRecord, RepairRefusalCode, RepairRefused,
		RepairResult,
	},
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
};

/// The record a rebuild of `key` would write, while `key` is quarantined, not being deleted,
/// and its replicas hold creation evidence.
pub(crate) fn probe_record(state: &MaintainerState, key: &VolumeLogKey) -> Option<FsVolumeRecord> {
	if !super::super::filesystem::is_record_missing(state, key) || state.deleting.contains_key(key)
	{
		return None;
	}
	let volume = state.loaded_volumes.get(key)?;
	volume.replica_creation()?;
	volume
		.row(key)
		.filter(|record| record.status == FsVolumeStatus::Live)
}

/// The rebuild's plan against the maintainer's state now and the store's probe.
pub(crate) fn plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairRebuildRecord,
	prepared: &RepairPrepared,
) -> Result<RepairRebuildPlan, RepairResult> {
	let record = check(state, key).map_err(RepairResult::RepairRefused)?;
	let Some(probe) = prepared
		.probe
		.as_ref()
		.filter(|probe| probe.record == record)
	else {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			Some(super::calls::BUSY_RETRY_MS),
			"the quarantine changed while the call was prepared".into(),
			None,
		));
	};
	let plan = RepairRebuildPlan {
		record,
		format_row_present: probe.format_row_present,
		root_inode_present: probe.root_inode_present,
	};
	if (!plan.format_row_present || !plan.root_inode_present) && !body.allow_missing_evidence {
		return Err(refused(
			RepairRefusalCode::NoEvidence,
			None,
			"the store lacks the volume format row or root inode; an explicit missing-evidence override is required".into(),
			Some(RepairPlan::RepairRebuildPlan(plan)),
		));
	}
	if plan.record.cold != body.expected_cold {
		return Err(refused(
			RepairRefusalCode::PositionChanged,
			None,
			"the replicas' cold position is not the expected one".into(),
			Some(RepairPlan::RepairRebuildPlan(plan)),
		));
	}
	Ok(plan)
}

/// Checks that `key` may be rebuilt now and returns the record it would get.
fn check(state: &MaintainerState, key: &VolumeLogKey) -> Result<FsVolumeRecord, RepairRefused> {
	if !super::super::filesystem::is_record_missing(state, key) {
		return Err(refusal(
			RepairRefusalCode::NotQuarantined,
			None,
			"the incarnation is not quarantined for a lost record".into(),
			None,
		));
	}
	if state.deleting.contains_key(key) {
		return Err(refusal(
			RepairRefusalCode::Deleting,
			None,
			"the incarnation is being deleted".into(),
			None,
		));
	}
	// A newer acknowledged incarnation holds the name; reviving this one would hide it.
	if super::super::deletion::is_replaced(state, key) {
		return Err(refusal(
			RepairRefusalCode::IncarnationChanged,
			None,
			"a newer incarnation replaced this one".into(),
			None,
		));
	}
	let Some(volume) = state.loaded_volumes.get(key) else {
		return Err(refusal(
			RepairRefusalCode::IncarnationChanged,
			None,
			"the incarnation is not loaded".into(),
			None,
		));
	};
	if volume.replica_creation().is_none() {
		return Err(refusal(
			RepairRefusalCode::NoEvidence,
			None,
			"no replica holds the incarnation's creation evidence, so its base chain and retention are unknown".into(),
			None,
		));
	}
	match volume.row(key) {
		Some(record) if record.status == FsVolumeStatus::Live => Ok(record),
		Some(_) | None => Err(refusal(
			RepairRefusalCode::NoQuorum,
			Some(1000),
			"no in-sync member reports the incarnation's cold position".into(),
			None,
		)),
	}
}

/// Runs a planned rebuild. Checks the quarantine and the record again, since the intent was
/// logged after the plan, then registers the incarnation with the planned record and queues
/// that record for the ordinary pass.
pub(crate) fn rebuild(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	plan: &RepairRebuildPlan,
	fx: &mut Effects,
) -> Result<(), RepairRefused> {
	let record = check(state, key)?;
	if record != plan.record {
		return Err(refusal(
			RepairRefusalCode::PlanChanged,
			None,
			"the record the replicas describe changed since the plan".into(),
			None,
		));
	}
	tracing::warn!(
		?key,
		cold = ?record.cold,
		format_row_present = plan.format_row_present,
		root_inode_present = plan.root_inode_present,
		"an operator rebuilt the lost record of a quarantined volume from its replicas' creation evidence"
	);
	super::super::filesystem::rebuild_record(state, key.clone(), record, fx);
	Ok(())
}


