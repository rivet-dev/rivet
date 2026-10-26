//! Repair forget of a replica outside the tablet's replica set, such as a removed member's
//! leftover that no cleanup reached. The maintainer checks the set and the name's own work
//! when it plans and again when it runs; the shard checks the observed replica, its fully cold
//! tail and its claim against its own view of a stable set in the same admission as the
//! forget, so a late forget never drops a replica that something counts again.

use super::{
	RepairOperation,
	calls::{self, Completion, Run, refusal},
};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	state::{MaintainerState, Operation},
	types::OpId,
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		RepairForgetPlan, RepairRefusalCode, RepairRefused, RepairReplicaForget,
	},
	depot_storage_worker_protocol_v1::{self as wire, RejectResponse, StorageWorkerResponse},
	shared_depot_types_v1::TabletPhase,
};
use rivet_topology::worker::WorkerId;

fn refused(code: RepairRefusalCode, retry_after_ms: Option<u64>, message: &str) -> RepairRefused {
	refusal(code, retry_after_ms, message.into(), None)
}

/// Checks that `body`'s worker may have its replica forgotten: the set is stable without it
/// and no deletion, fork or restore of the name owns its replicas.
pub(crate) fn plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	body: &RepairReplicaForget,
) -> Result<RepairForgetPlan, RepairRefused> {
	let replica_set = &state.tablet.replica_set;
	if !matches!(replica_set.phase, TabletPhase::Stable) {
		return Err(refused(
			RepairRefusalCode::NotStable,
			Some(1000),
			"the tablet's replica set is changing",
		));
	}
	if state.members().contains(&WorkerId::from_bytes(body.worker)) {
		return Err(refused(
			RepairRefusalCode::AssignedMember,
			None,
			"the worker is a member of the tablet's replica set; its replica is not forgotten by repair",
		));
	}
	if state.deleting.contains_key(key) {
		return Err(refused(
			RepairRefusalCode::Deleting,
			None,
			"the incarnation is being deleted, which forgets its replicas itself",
		));
	}
	if super::super::forks::holds_key(state, &key.0.volume_key) {
		return Err(refused(
			RepairRefusalCode::RepairBusy,
			Some(1000),
			"a fork or restore of the name is in progress",
		));
	}
	Ok(RepairForgetPlan {
		replica_set_version: replica_set.tablet_replica_set_version,
	})
}

/// Runs a forget: sends the conditional forget to the worker under the tablet grant.
pub(crate) fn forget(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	operation_id: [u8; 16],
	body: &RepairReplicaForget,
	planned: &RepairForgetPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	if plan(state, key, body)? != *planned {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			None,
			"the tablet's replica set changed since the plan",
		));
	}
	let op_id = state.begin(
		Operation::Repair(RepairOperation::Forget { operation_id }),
		fx,
	);
	fx.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: WorkerId::from_bytes(body.worker),
		request: wire::StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
			header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
				wire::MaintainerRequestHeader {
					volume_log_key: key.0.clone(),
					tablet_grant: state.grant(),
				},
			),
			expected_appended: Some(body.expected_appended.clone()),
			expected_claim: body.expected_claim.clone(),
			observation: Some(wire::ForgetObservation {
				replica_incarnation: body.replica_incarnation,
				tablet_replica_set_version: planned.replica_set_version,
			}),
		}),
	});
	Ok(Run::Pending)
}

/// The worker answered the forget sent for call `operation_id`.
pub(crate) fn completed(
	state: &mut MaintainerState,
	operation_id: [u8; 16],
	result: Result<StorageWorkerResponse, String>,
	fx: &mut Effects,
) {
	let outcome = match result {
		// The shard answers a forget with the state it removed.
		Ok(StorageWorkerResponse::ReplicaStateResponse(_)) => Ok(None),
		// The worker holds no replica of the incarnation, or another forget of it is already
		// syncing; either way nothing is left for this call to forget.
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::VolumeNotLoaded)) => Ok(None),
		Ok(StorageWorkerResponse::RejectResponse(
			RejectResponse::AppendedMismatch(_) | RejectResponse::VolumeLogReplicaDeleted(_),
		)) => Err(Completion::Refused(refused(
			RepairRefusalCode::ReplicaChanged,
			None,
			"the replica is not fully cold at the expected position, holds another claim, lost entries or is deleted; inspect it again",
		))),
		Ok(StorageWorkerResponse::RejectResponse(RejectResponse::NotServing)) => {
			Err(Completion::Refused(refused(
				RepairRefusalCode::ReplicaChanged,
				Some(1000),
				"the worker refused the forget: it is overloaded, does not see the planned stable replica set without itself, or holds another replica incarnation; inspect it again",
			)))
		}
		Ok(StorageWorkerResponse::RejectResponse(reason)) => Err(Completion::Refused(refused(
			RepairRefusalCode::ReplicaChanged,
			Some(1000),
			&format!("the worker refused the forget: {reason:?}"),
		))),
		Ok(response) => Err(Completion::Unknown(format!(
			"the worker answered the forget unexpectedly: {response:?}; inspect the replica"
		))),
		Err(error) => Err(Completion::Unknown(format!(
			"the forget did not answer: {error}; inspect the replica before forgetting again"
		))),
	};
	calls::complete(state, operation_id, outcome, fx);
}

/// The forget outlived its deadline; the worker may still remove the replica.
pub(crate) fn timed_out(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	let Some(Operation::Repair(RepairOperation::Forget { operation_id })) = state.finish(op_id, fx)
	else {
		return;
	};
	fx.execute(MaintainerEffect::CancelReplicaRequest { op_id });
	completed(state, operation_id, Err("timed out".into()), fx);
}


