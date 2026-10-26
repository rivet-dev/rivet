//! Bounded ephemeral activity pages. Cursors bind a logical request and tablet grant;
//! skipped operations consume the scan budget and advance the cursor even on empty pages.

use super::{RepairOperation, calls::CallStep, claims::ClaimPhase};
use crate::tablet_maintainer_machine::{
	state::{
		MaintainerState, OpenStep, Operation,
		alarms::Attempt,
		deletion::{ColdProgress, DeleteStep},
		filesystem::FlushStage,
	},
	types::OpId,
};
use depot_types::{VolumeKey, VolumeLogKey};
use rivet_data::generated::depot_client_protocol_v1::{
	RepairActivity, RepairDiagnosticRow, RepairDiagnosticValue, RepairDiagnosticsPage,
	RepairDiagnosticsRequest,
};

const MAX_SCAN: usize = 400;

pub(super) fn page(
	state: &MaintainerState,
	request: &RepairDiagnosticsRequest,
) -> Result<RepairDiagnosticsPage, ()> {
	let key = VolumeLogKey(request.volume_log_key.clone());
	let prefix = prefix(state, &key);
	let after = match request.after.as_deref() {
		None => None,
		Some(bytes) if bytes.len() == 49 && bytes[..40] == prefix && bytes[40] <= 1 => Some((
			bytes[40],
			u64::from_be_bytes(bytes[41..].try_into().map_err(|_| ())?),
		)),
		Some(_) => return Err(()),
	};
	let limit = request.limit.clamp(1, 100) as usize;
	let mut rows = Vec::new();
	// At most six direct lifecycle lookups and four claims/calls: no operation scan here.
	if after.is_none_or(|(group, _)| group == 0) {
		for (slot, activity) in summaries(state, &key) {
			if after.is_some_and(|(_, id)| slot <= id) {
				continue;
			}
			let cursor = cursor(&prefix, 0, slot);
			rows.push(row(cursor.clone(), activity));
			if rows.len() == limit {
				return Ok(RepairDiagnosticsPage {
					rows,
					after: Some(cursor.into()),
				});
			}
		}
	}
	let mut cursor_after = None;
	let start = after
		.filter(|(group, _)| *group == 1)
		.map(|(_, id)| OpId(id));
	let mut operations = state.operations.after(start);
	for _ in 0..MAX_SCAN {
		let Some((id, operation)) = operations.next() else {
			return Ok(RepairDiagnosticsPage { rows, after: None });
		};
		let cursor = cursor(&prefix, 1, id.0);
		cursor_after = Some(cursor.clone().into());
		if let Some((kind, phase, shared)) = describe(state, &key.0.volume_key, operation) {
			rows.push(row(
				cursor,
				activity(state, Some(id.0), None, kind, phase, shared),
			));
			if rows.len() == limit {
				break;
			}
		}
	}
	Ok(RepairDiagnosticsPage {
		rows,
		after: cursor_after,
	})
}

fn prefix(state: &MaintainerState, key: &VolumeLogKey) -> [u8; 40] {
	let mut prefix = [0; 40];
	prefix[..16].copy_from_slice(&key.digest());
	prefix[16..32].copy_from_slice(&state.tablet.id);
	prefix[32..].copy_from_slice(&state.tablet.tablet_grant_epoch.to_be_bytes());
	prefix
}
fn cursor(prefix: &[u8; 40], group: u8, id: u64) -> Vec<u8> {
	let mut bytes = prefix.to_vec();
	bytes.push(group);
	bytes.extend_from_slice(&id.to_be_bytes());
	bytes
}
fn row(key: Vec<u8>, activity: RepairActivity) -> RepairDiagnosticRow {
	RepairDiagnosticRow {
		key: key.into(),
		raw: Vec::new().into(),
		decoded: Some(RepairDiagnosticValue::RepairActivity(activity)),
		decode_error: None,
	}
}
fn activity(
	state: &MaintainerState,
	operation_id: Option<u64>,
	repair_operation_id: Option<[u8; 16]>,
	kind: &'static str,
	phase: &'static str,
	shared: bool,
) -> RepairActivity {
	RepairActivity {
		tablet_id: state.tablet.id,
		tablet_grant_epoch: state.tablet.tablet_grant_epoch,
		operation_id,
		repair_operation_id,
		kind: kind.into(),
		phase: phase.into(),
		shared,
	}
}

fn summaries(state: &MaintainerState, key: &VolumeLogKey) -> Vec<(u64, RepairActivity)> {
	let name = &key.0.volume_key;
	let mut rows = Vec::new();
	let mut add = |slot, kind, phase, operation| {
		rows.push((slot, activity(state, None, operation, kind, phase, false)))
	};
	if let Some(open) = state.pending.get(name) {
		add(
			0,
			"open",
			match open.step {
				OpenStep::WaitingForRepair => "WAITING_REPAIR",
				OpenStep::ReadingRow => "READING_ROW",
				OpenStep::Loading(_) => "LOADING",
			},
			None,
		);
	} else if state.awaiting_deletion.contains_key(name) {
		add(0, "open", "WAITING_DELETE", None);
	}
	if let Some(unload) = state.unloading.get(key) {
		use crate::tablet_maintainer_machine::state::{operations, unload::RowCheck};
		let phase = if unload.awaiting_notices {
			"WAITING_NOTICES"
		} else if unload.recovery_backoff {
			"RECOVERY_BACKOFF"
		} else if state
			.operations
			.index()
			.forgets
			.range(operations::workers_of(key))
			.next()
			.is_some()
		{
			"FORGETTING"
		} else if !unload.waiting.is_empty() {
			"FORGET_BACKOFF"
		} else {
			match unload.row_check {
				RowCheck::Queued => "ROW_QUEUED",
				RowCheck::Reading(_) => "READING_ROW",
				RowCheck::Checked => "ROW_CHECKED",
			}
		};
		add(1, "unload", phase, None);
	}
	if let Some(delete) = state.deleting.get(key) {
		let phase = match &delete.step {
			DeleteStep::ReadingRow { .. } => "READING_ROW",
			DeleteStep::WaitingForReplay { .. } => "WAITING_REPLAY",
			DeleteStep::WaitingForCover { .. } => "WAITING_COVER",
			DeleteStep::Active(progress) => match progress.cold {
				ColdProgress::WaitingForReplica => "WAITING_REPLICA",
				ColdProgress::WaitingForRow => "WAITING_ROW",
				ColdProgress::Staged { .. } => "STAGED",
				ColdProgress::Ready => "READY",
				ColdProgress::Held => "HELD",
				ColdProgress::Sweeping => "SWEEPING",
				ColdProgress::Removed { .. } => "REMOVED",
				ColdProgress::Gone => "GONE",
			},
		};
		add(2, "delete", phase, None);
	}
	if let Some(job) = state.forks.jobs.get(name) {
		add(3, "fork", job.activity_phase(), Some(job.spec.operation_id));
	}
	if let Some(claim) = state.repairs.claims.by_name.get(name) {
		let phase = match claim.phase {
			ClaimPhase::Opening { .. } => "OPENING",
			ClaimPhase::Held { .. } => "HELD",
			ClaimPhase::Appending { .. } => "APPENDING",
			ClaimPhase::Closing { .. } => "CLOSING",
		};
		add(4, "repair_claim", phase, Some(claim.operation_id));
	}
	if let Some((id, call)) = state
		.repairs
		.calls
		.active
		.iter()
		.find(|(_, call)| &call.request.envelope.volume_log_key.volume_key == name)
	{
		let phase = match call.step {
			CallStep::Preparing { .. } => "PREPARING",
			CallStep::LoggingIntent { .. } => "LOGGING_INTENT",
			CallStep::Running { .. } => "RUNNING",
			CallStep::Persisting { .. } => "PERSISTING",
		};
		add(5, "repair_call", phase, Some(*id));
	}
	rows
}

/// Never materialize or serialize an operation's payload. Batch work is explicitly shared;
/// its potentially large key list is not walked to guess whether it touched this name.
fn describe(
	state: &MaintainerState,
	name: &VolumeKey,
	operation: &Operation,
) -> Option<(&'static str, &'static str, bool)> {
	let own = |key: &VolumeLogKey| &key.0.volume_key == name;
	let scoped = |scope: &depot_fs_core::keys::slatedb::Scope| {
		scope.hash == depot_types::partition_key_hash(name)
			&& scope.name.as_deref() == name.name.as_deref()
	};
	let (kind, phase, relevant, shared) = match operation {
		Operation::Alarm(attempt) => (
			"alarm",
			match attempt {
				Attempt::Queued(_) => "QUEUED",
				Attempt::Running(_) => "RUNNING",
				Attempt::Cancelling(_) => "CANCELLING",
				Attempt::Waiting(_) => "WAITING",
				Attempt::Obsolete(_) => "OBSOLETE",
			},
			attempt.request().key().is_none_or(own),
			attempt.request().key().is_none(),
		),
		Operation::MaterializeStore { scope } => {
			("materialize_store", "IN_FLIGHT", scoped(scope), false)
		}
		Operation::MaterializePeer { scope } => {
			("materialize_peer", "IN_FLIGHT", scoped(scope), false)
		}
		Operation::ForkCleanupStore { work } => (
			"fork_cleanup_store",
			"IN_FLIGHT",
			scoped(&work.scope),
			false,
		),
		Operation::ForkCleanupPeer { work } => {
			("fork_cleanup_peer", "IN_FLIGHT", scoped(&work.scope), false)
		}
		Operation::ForkStore {
			volume_key,
			parent_volume_key,
		} => (
			"fork_store",
			"IN_FLIGHT",
			volume_key == name || parent_volume_key.as_ref() == Some(name),
			false,
		),
		Operation::ForkPeer { volume_key } => ("fork_peer", "IN_FLIGHT", volume_key == name, false),
		Operation::ForkReference { volumes, .. } => (
			"fork_reference",
			"IN_FLIGHT",
			volumes.len() > 2 || volumes.iter().take(2).any(|v| v == name),
			volumes.len() > 2,
		),
		Operation::PendingWork { volume } => (
			"pending_work",
			"IN_FLIGHT",
			volume
				.as_ref()
				.is_none_or(|v| v == &depot_fs_core::work::PendingVolume::of(name)),
			volume.is_none(),
		),
		Operation::SweepPending { scope } => ("sweep_pending", "IN_FLIGHT", scoped(scope), false),
		Operation::Checkpoint { volume_key, .. } => {
			("checkpoint", "IN_FLIGHT", volume_key == name, false)
		}
		Operation::FilesystemQuery {
			request_id,
			volume_key,
			..
		} => (
			"filesystem_query",
			"IN_FLIGHT",
			volume_key.as_ref() == Some(name)
				|| state
					.pending_fs_reads
					.get(request_id)
					.is_some_and(|request| request.volume_key() == name),
			false,
		),
		Operation::FilesystemLoad { key } => ("filesystem_load", "IN_FLIGHT", own(key), false),
		Operation::FilesystemRead { key, .. } => ("filesystem_read", "IN_FLIGHT", own(key), false),
		Operation::FilesystemApply { key, .. } => {
			("filesystem_apply", "IN_FLIGHT", own(key), false)
		}
		Operation::FilesystemFlush { stage, .. } => (
			"filesystem_flush",
			match stage {
				FlushStage::Staging => "STAGING",
				FlushStage::Uploading => "UPLOADING",
			},
			true,
			true,
		),
		Operation::PromotionList { .. } => ("promotion_list", "IN_FLIGHT", true, true),
		Operation::CatchUp { key, .. } => ("catch_up", "IN_FLIGHT", own(key), false),
		Operation::ReadDeleteRow { key } => ("read_delete_row", "IN_FLIGHT", own(key), false),
		Operation::DeleteReplica { key, .. } => ("delete_replica", "IN_FLIGHT", own(key), false),
		Operation::DrainClaim { key, .. } => ("drain_claim", "IN_FLIGHT", own(key), false),
		Operation::SweepDeleted { key } => ("sweep_deleted", "IN_FLIGHT", own(key), false),
		Operation::PrepareSplit { .. } => ("prepare_split", "IN_FLIGHT", true, true),
		Operation::ReportCatchUp { .. } => ("report_catch_up", "IN_FLIGHT", true, true),
		Operation::ReportSplit => ("report_split", "IN_FLIGHT", true, true),
		Operation::DeleteRetired { .. } => ("delete_retired", "IN_FLIGHT", true, true),
		Operation::ForgetRetiredLog { .. } => ("forget_retired_log", "IN_FLIGHT", true, true),
		Operation::ReportRetired { .. } => ("report_retired", "IN_FLIGHT", true, true),
		Operation::ReportParentLogImported => ("report_parent_log", "IN_FLIGHT", true, true),
		Operation::ReportVersionMarks { .. } => ("report_version_marks", "IN_FLIGHT", true, true),
		Operation::RecoverVolume { key, cancelled, .. } => (
			"recover_volume",
			if *cancelled { "CANCELLING" } else { "RUNNING" },
			own(key),
			false,
		),
		Operation::ReadUnloadRow { key } => ("read_unload_row", "IN_FLIGHT", own(key), false),
		Operation::ForgetVolumes { .. } => ("forget_batch", "IN_FLIGHT", true, true),
		Operation::WriteToCold { .. } => ("write_to_cold", "IN_FLIGHT", true, true),
		Operation::Trim { key, .. } => ("trim", "IN_FLIGHT", own(key), false),
		Operation::TrimBatch { .. } => ("trim_batch", "IN_FLIGHT", true, true),
		Operation::Poll { .. } => ("poll", "IN_FLIGHT", true, true),
		Operation::OpenCold => ("open_cold", "IN_FLIGHT", true, true),
		Operation::List { .. } => ("list", "IN_FLIGHT", true, true),
		Operation::ReadRow { volume_key } => ("read_row", "IN_FLIGHT", volume_key == name, false),
		Operation::ReadOpenKeys { volume_key, .. } => {
			("read_open_keys", "IN_FLIGHT", volume_key == name, false)
		}
		Operation::Load { key, .. } => ("load", "IN_FLIGHT", own(key), false),
		Operation::ForgetOrphan { key, .. } => ("forget_orphan", "IN_FLIGHT", own(key), false),
		Operation::MaintainerLog { .. } => ("maintainer_log", "IN_FLIGHT", true, true),
		Operation::ReplayOp { volume_key, .. } => (
			"replay",
			"IN_FLIGHT",
			volume_key.as_ref().is_none_or(|key| key == name),
			volume_key.is_none(),
		),
		Operation::Repair(RepairOperation::InspectRead { volume_key, .. }) => {
			("repair_inspect", "IN_FLIGHT", volume_key == name, false)
		}
		Operation::Repair(
			RepairOperation::Prepare { operation_id }
			| RepairOperation::Call { operation_id }
			| RepairOperation::Forget { operation_id },
		) => (
			"repair",
			"IN_FLIGHT",
			state
				.repairs
				.calls
				.active
				.get(operation_id)
				.is_some_and(|call| &call.request.envelope.volume_log_key.volume_key == name),
			false,
		),
		Operation::Repair(RepairOperation::Claim { volume_key }) => {
			("repair_claim", "IN_FLIGHT", volume_key == name, false)
		}
		// These read-only requests retain no volume identity in the operation. Their own
		// returned rows provide diagnostics; never expose unrelated request identities.
		Operation::Repair(
			RepairOperation::Diagnostics { .. } | RepairOperation::AuditList { .. },
		) => return None,
	};
	if relevant {
		Some((kind, phase, shared))
	} else {
		None
	}
}


