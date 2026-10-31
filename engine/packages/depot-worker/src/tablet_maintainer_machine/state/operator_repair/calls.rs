//! The pipeline every mutating repair call runs through, whatever its body.
//!
//! 1. Admission: a serving maintainer of the call's volume, within the caller budget, with
//!    no other call of the name in flight and at most `MAX_CALLS` calls on the tablet. A
//!    retry of a call this lifetime finished answers its stored outcome.
//! 2. Preparing: one store read of the call's audit row, with any facts its plan needs. A
//!    stored row answers a retry of an earlier lifetime's call, whatever grant it names;
//!    otherwise the call must name this maintainer's tablet and grant.
//! 3. Planning: the body computes its plan against the maintainer's state now. A dry run
//!    replies with the plan and its digest; a real call must carry that digest.
//! 4. Logging the intent: the call's audit op without an outcome is logged in the maintainer
//!    log and applied to the store before anything else happens.
//! 5. Running: the body checks its preconditions again and runs its effect. A body whose
//!    effect is executor work, such as a repair claim's client, waits for that work.
//! 6. The call replies, and its audit op is logged again with the outcome.
//!
//! A lost grant ends every call: the next lifetime finds the intent row without an outcome
//! and answers `RepairUnknown` to a retry, since the effect may have run.

use super::{RepairOperation, admissible, refuse};
use crate::repair_types::{PrepareReads, RepairPrepared};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::{MaintainerEffect, RepairEffect},
	state::{MaintainerState, Operation},
	types::{OpId, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		ClientResponse, RepairApplied, RepairAuditRow, RepairBody, RepairDetail, RepairOutcome,
		RepairPlan, RepairPlanned, RepairRefusalCode, RepairRefused, RepairRequest, RepairResponse,
		RepairResult, RepairUnknown,
	},
	depot_maintainer_log_v1::{MaintainerOp, RepairAuditOp},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use vbare::Encode;

/// Mutating calls one tablet runs at once.
pub(crate) const MAX_CALLS: usize = 4;

/// Finished calls whose outcomes this lifetime keeps for retries. Older ones answer from
/// their audit rows.
const MAX_FINISHED: usize = 256;

/// How long a caller refused for a busy name or tablet waits before trying again.
pub(crate) const BUSY_RETRY_MS: u64 = 250;

/// The mutating calls of this lifetime.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Calls {
	/// Calls in flight, by operation id.
	pub(crate) active: BTreeMap<[u8; 16], Call>,
	/// Outcomes of finished calls, by operation id, with the request each answered.
	finished: BTreeMap<[u8; 16], (RepairRequest, RepairOutcome)>,
	/// `finished` in finishing order, for evicting the oldest.
	finished_order: VecDeque<[u8; 16]>,
}

/// One mutating call in flight.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Call {
	pub(crate) request_id: RequestId,
	pub(crate) request: RepairRequest,
	pub(crate) requested_at_unix_ms: i64,
	pub(crate) step: CallStep,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum CallStep {
	/// Reading the call's audit row and its plan's store facts, under operation `op_id`.
	Preparing { op_id: OpId },
	/// The intent audit op is submitted. The call runs once the op's store apply finishes.
	/// From here on operation `op_id` bounds the call: its deadline answers the call as
	/// unknown, since its intent may be logged and its effect may have run.
	LoggingIntent {
		plan: RepairPlan,
		digest: [u8; 32],
		op_id: OpId,
	},
	/// The body's executor work is in flight; the body's own state machine completes the
	/// call with `complete`.
	Running {
		plan: RepairPlan,
		digest: [u8; 32],
		op_id: OpId,
	},
	/// The effect ran; the call waits until the store holds what it changed, so an applied
	/// outcome survives the maintainer's next lifetime.
	Persisting {
		plan: RepairPlan,
		digest: [u8; 32],
		op_id: OpId,
	},
}

impl CallStep {
	/// The operation bounding the call from its intent on.
	fn deadline(&self) -> Option<OpId> {
		match self {
			CallStep::Preparing { .. } => None,
			CallStep::LoggingIntent { op_id, .. }
			| CallStep::Running { op_id, .. }
			| CallStep::Persisting { op_id, .. } => Some(*op_id),
		}
	}
}

/// The volume a call names.
pub(crate) fn call_key(request: &RepairRequest) -> VolumeLogKey {
	VolumeLogKey(request.envelope.volume_log_key.clone())
}

/// Admits a mutating call and starts its preparing read.
pub(crate) fn request(
	state: &mut MaintainerState,
	request_id: RequestId,
	request: RepairRequest,
	fx: &mut Effects,
) {
	if let Err(reason) = admissible(state) {
		refuse(request_id, reason, fx);
		return;
	}
	let key = call_key(&request);
	if !super::super::in_tablet(state, &key.0.volume_key) {
		refuse(request_id, depot_types::client::not_maintainer(), fx);
		return;
	}
	let operation_id = request.envelope.operation_id;
	// A retry of a call this lifetime finished answers what the call answered.
	if let Some((first, outcome)) = state.repairs.calls.finished.get(&operation_id) {
		let result = match same_call(first, &request) {
			true => outcome_result(outcome.clone()),
			false => conflict(),
		};
		reply(request_id, operation_id, true, result, fx);
		return;
	}
	if state.repairs.calls.active.contains_key(&operation_id) {
		let result = refused(
			RepairRefusalCode::OperationInProgress,
			Some(BUSY_RETRY_MS),
			"a call with this operation id is in progress".into(),
			None,
		);
		reply(request_id, operation_id, false, result, fx);
		return;
	}
	let busy = state.repairs.calls.active.len() >= MAX_CALLS
		|| state
			.repairs
			.calls
			.active
			.values()
			.any(|call| call_key(&call.request).0.volume_key == key.0.volume_key);
	if busy {
		let result = refused(
			RepairRefusalCode::RepairBusy,
			Some(BUSY_RETRY_MS),
			"another repair call of this name, or too many of this tablet, is in progress".into(),
			None,
		);
		reply(request_id, operation_id, false, result, fx);
		return;
	}
	// The preparing read takes a read slot, so a full read budget refuses before any work.
	if super::super::store::StoreUsage::of(state).reads() >= state.config.max_cold_reads as usize {
		refuse(request_id, depot_types::client::overloaded(), fx);
		return;
	}

	let reads = prepare_reads(state, &request);
	let op_id = state.begin(
		Operation::Repair(RepairOperation::Prepare { operation_id }),
		fx,
	);
	state.repairs.calls.active.insert(
		operation_id,
		Call {
			request_id,
			request,
			requested_at_unix_ms: fx.clocks().unix_ms,
			step: CallStep::Preparing { op_id },
		},
	);
	fx.execute(MaintainerEffect::Repair(RepairEffect::PrepareRequest {
		op_id,
		volume_log_key: key,
		operation_id,
		reads,
	}));
}

/// The refusal message of a call fenced by another tablet or grant than this maintainer's.
fn grant_changed(state: &MaintainerState, request: &RepairRequest) -> Option<String> {
	if request.envelope.tablet_id == state.tablet.id
		&& request.envelope.tablet_grant_epoch == state.tablet.tablet_grant_epoch
	{
		return None;
	}
	Some(format!(
		"the call names tablet {} at grant {} but the maintainer serves tablet {} at grant {}",
		uuid::Uuid::from_bytes(request.envelope.tablet_id),
		request.envelope.tablet_grant_epoch,
		uuid::Uuid::from_bytes(state.tablet.id),
		state.tablet.tablet_grant_epoch
	))
}

/// The store facts a body's plan needs read.
fn prepare_reads(state: &MaintainerState, request: &RepairRequest) -> PrepareReads {
	let key = call_key(request);
	match &request.body {
		RepairBody::RepairRebuildRecord(_) => match super::quarantine::probe_record(state, &key) {
			Some(record) => PrepareReads::Probe(record),
			None => PrepareReads::None,
		},
		RepairBody::RepairAppend(body) => super::claims::append_reads(state, &key, body),
		RepairBody::RepairClaimAcquire(_)
		| RepairBody::RepairClaimRelease(_)
		| RepairBody::RepairReplicaReload(_)
		| RepairBody::RepairAbortUnload
		| RepairBody::RepairRedrive
		| RepairBody::RepairReplicaForget(_) => PrepareReads::None,
	}
}

/// The preparing read answered.
pub(crate) fn prepared(
	state: &mut MaintainerState,
	op_id: OpId,
	operation_id: [u8; 16],
	result: Result<RepairPrepared, String>,
	fx: &mut Effects,
) {
	let Some(call) = state.repairs.calls.active.get(&operation_id) else {
		return;
	};
	if call.step != (CallStep::Preparing { op_id }) {
		return;
	}
	let call = state
		.repairs
		.calls
		.active
		.remove(&operation_id)
		.expect("call checked above");
	let prepared = match result {
		Ok(prepared) => prepared,
		Err(error) => {
			tracing::warn!(%error, ?operation_id, "a repair call's preparing read failed");
			refuse(call.request_id, depot_types::client::overloaded(), fx);
			return;
		}
	};

	// A call an earlier lifetime logged answers from its audit row.
	if !call.request.envelope.dry_run
		&& let Some(row) = prepared.audit
	{
		let result = match (same_call(&row.request, &call.request), row.outcome) {
			(false, _) => conflict(),
			(true, Some(outcome)) => outcome_result(outcome),
			(true, None) => RepairResult::RepairUnknown(RepairUnknown {
				message: "the call's intent was logged by an earlier maintainer and its outcome is unknown; inspect the volume".into(),
			}),
		};
		reply(call.request_id, operation_id, true, result, fx);
		return;
	}

	// The grant fence is checked after the audit row, so a retry that names the grant of an
	// earlier lifetime still learns that lifetime's outcome.
	if let Some(message) = grant_changed(state, &call.request) {
		let result = refused(RepairRefusalCode::GrantChanged, None, message, None);
		reply(call.request_id, operation_id, false, result, fx);
		return;
	}
	let plan = match plan(state, &call.request, &prepared) {
		Ok(plan) => plan,
		Err(refusal) => {
			reply(call.request_id, operation_id, false, refusal, fx);
			return;
		}
	};
	let digest = digest(&call.request, &plan);
	if call.request.envelope.dry_run {
		let result = RepairResult::RepairPlanned(RepairPlanned {
			plan,
			plan_digest: digest,
		});
		reply(call.request_id, operation_id, false, result, fx);
		return;
	}
	match call.request.envelope.plan_digest {
		None => {
			let result = refused(
				RepairRefusalCode::PlanRequired,
				None,
				"a real call must carry the digest of its dry run's plan".into(),
				Some(plan),
			);
			reply(call.request_id, operation_id, false, result, fx);
			return;
		}
		Some(expected) if expected != digest => {
			let result = refused(
				RepairRefusalCode::PlanChanged,
				None,
				"the plan changed since the dry run".into(),
				Some(plan),
			);
			reply(call.request_id, operation_id, false, result, fx);
			return;
		}
		Some(_) => {}
	}
	if !super::super::maintainer_log::admits(state) {
		let result = refused(
			RepairRefusalCode::RepairBusy,
			Some(1000),
			"the maintainer log is not serving or is full".into(),
			None,
		);
		reply(call.request_id, operation_id, false, result, fx);
		return;
	}

	// The intent is durable and in the store before the effect runs.
	log_audit(state, &call, digest, None);
	let op_id = state.begin(
		Operation::Repair(RepairOperation::Call { operation_id }),
		fx,
	);
	state.repairs.calls.active.insert(
		operation_id,
		Call {
			step: CallStep::LoggingIntent {
				plan,
				digest,
				op_id,
			},
			..call
		},
	);
}

/// An audit op's store apply finished. The intent of a call waiting for it lets the call run.
pub(crate) fn audit_applied(state: &mut MaintainerState, op: RepairAuditOp, fx: &mut Effects) {
	if op.row.outcome.is_some() {
		return;
	}
	let operation_id = op.row.request.envelope.operation_id;
	let Some(call) = state.repairs.calls.active.get(&operation_id) else {
		return;
	};
	let CallStep::LoggingIntent {
		plan,
		digest,
		op_id,
	} = call.step.clone()
	else {
		return;
	};
	if call.request != op.row.request {
		return;
	}
	let call = state
		.repairs
		.calls
		.active
		.remove(&operation_id)
		.expect("call checked above");
	match run(state, &call.request, &plan, fx) {
		Ok(Run::Pending) => {
			state.repairs.calls.active.insert(
				operation_id,
				Call {
					step: CallStep::Running {
						plan,
						digest,
						op_id,
					},
					..call
				},
			);
		}
		Ok(Run::Done(_)) if persists(&plan) => {
			// The changed rows ride a pass that starts now rather than at the queue's age.
			super::super::store::queue_cold_pass(state);
			state.repairs.calls.active.insert(
				operation_id,
				Call {
					step: CallStep::Persisting {
						plan,
						digest,
						op_id,
					},
					..call
				},
			);
		}
		Ok(Run::Done(detail)) => {
			let outcome = RepairOutcome::RepairApplied(RepairApplied {
				plan,
				plan_digest: digest,
				detail,
			});
			finish(state, call, digest, outcome, fx);
		}
		Err(refusal) => finish(
			state,
			call,
			digest,
			RepairOutcome::RepairRefused(refusal),
			fx,
		),
	}
}

/// How a body's effect went when the call ran it.
pub(crate) enum Run {
	/// The effect is done, with what the caller learns of it.
	Done(Option<RepairDetail>),
	/// Executor work is in flight; the body completes the call later.
	Pending,
}

/// Why a running call did not apply.
pub(crate) enum Completion {
	Refused(RepairRefused),
	/// The effect may have run.
	Unknown(String),
}

/// Completes a running call with its body's outcome.
pub(crate) fn complete(
	state: &mut MaintainerState,
	operation_id: [u8; 16],
	result: Result<Option<RepairDetail>, Completion>,
	fx: &mut Effects,
) {
	if !state
		.repairs
		.calls
		.active
		.get(&operation_id)
		.is_some_and(|call| matches!(call.step, CallStep::Running { .. }))
	{
		return;
	}
	let Some(call) = state.repairs.calls.active.remove(&operation_id) else {
		return;
	};
	let CallStep::Running { plan, digest, .. } = call.step.clone() else {
		return;
	};
	let outcome = match result {
		Ok(detail) => RepairOutcome::RepairApplied(RepairApplied {
			plan,
			plan_digest: digest,
			detail,
		}),
		Err(Completion::Refused(refusal)) => RepairOutcome::RepairRefused(refusal),
		Err(Completion::Unknown(message)) => RepairOutcome::RepairUnknown(RepairUnknown {
			message: message.into(),
		}),
	};
	finish(state, call, digest, outcome, fx);
}

/// Whether a plan's effect is durable only once a pass writes it, so its call waits for that.
fn persists(plan: &RepairPlan) -> bool {
	match plan {
		RepairPlan::RepairRebuildPlan(_) => true,
		RepairPlan::RepairClaimPlan(_)
		| RepairPlan::RepairReleasePlan(_)
		| RepairPlan::RepairAppendPlan(_)
		| RepairPlan::RepairReloadPlan(_)
		| RepairPlan::RepairAbortUnloadPlan(_)
		| RepairPlan::RepairRedrivePlan(_)
		| RepairPlan::RepairForgetPlan(_) => false,
	}
}

/// Whether the store now holds what `plan` changed on `key`.
fn persisted(state: &MaintainerState, key: &VolumeLogKey, plan: &RepairPlan) -> bool {
	match plan {
		RepairPlan::RepairRebuildPlan(plan) => state
			.loaded_volumes
			.get(key)
			.and_then(|volume| volume.stored_row.as_ref())
			.is_some_and(|row| row.cold.volume_seq >= plan.record.cold.volume_seq),
		RepairPlan::RepairClaimPlan(_)
		| RepairPlan::RepairReleasePlan(_)
		| RepairPlan::RepairAppendPlan(_)
		| RepairPlan::RepairReloadPlan(_)
		| RepairPlan::RepairAbortUnloadPlan(_)
		| RepairPlan::RepairRedrivePlan(_)
		| RepairPlan::RepairForgetPlan(_) => true,
	}
}

/// Finishes every call whose change the store now holds. Called after each pass and flush.
pub(crate) fn check_persisted(state: &mut MaintainerState, fx: &mut Effects) {
	let done = state
		.repairs
		.calls
		.active
		.iter()
		.filter_map(|(operation_id, call)| match &call.step {
			CallStep::Persisting { plan, op_id, .. }
				if persisted(state, &call_key(&call.request), plan) =>
			{
				Some((*operation_id, *op_id))
			}
			CallStep::Preparing { .. }
			| CallStep::LoggingIntent { .. }
			| CallStep::Running { .. }
			| CallStep::Persisting { .. } => None,
		})
		.collect::<Vec<_>>();
	for (operation_id, _) in done {
		let Some(call) = state.repairs.calls.active.remove(&operation_id) else {
			continue;
		};
		let CallStep::Persisting { plan, digest, .. } = call.step.clone() else {
			continue;
		};
		let outcome = RepairOutcome::RepairApplied(RepairApplied {
			plan,
			plan_digest: digest,
			detail: None,
		});
		finish(state, call, digest, outcome, fx);
	}
}

/// A call outlived its deadline after its intent was submitted. The intent may be logged and
/// the effect may have run or still run, so the outcome is unknown until an inspection or a
/// retry with the same operation id shows it.
pub(crate) fn call_timed_out(
	state: &mut MaintainerState,
	op_id: OpId,
	operation_id: [u8; 16],
	fx: &mut Effects,
) {
	if !state
		.repairs
		.calls
		.active
		.get(&operation_id)
		.is_some_and(|call| call.step.deadline() == Some(op_id))
	{
		return;
	}
	let Some(call) = state.repairs.calls.active.remove(&operation_id) else {
		return;
	};
	let (digest, message) = match &call.step {
		CallStep::LoggingIntent { digest, .. } => (
			*digest,
			"the call's intent was not in the store within its deadline; it may still be logged",
		),
		CallStep::Running { digest, .. } => (
			*digest,
			"the call's effect did not answer within its deadline; inspect the volume",
		),
		CallStep::Persisting { digest, .. } => (
			*digest,
			"the change ran but the store did not hold it within the call's deadline; inspect the volume",
		),
		CallStep::Preparing { .. } => return,
	};
	let outcome = RepairOutcome::RepairUnknown(RepairUnknown {
		message: message.into(),
	});
	finish(state, call, digest, outcome, fx);
}

/// The preparing read outlived its deadline. It holds no store footprint, so the call ends
/// here, before anything was logged, and its late response is dropped.
pub(crate) fn prepare_timed_out(
	state: &mut MaintainerState,
	op_id: OpId,
	operation_id: [u8; 16],
	fx: &mut Effects,
) {
	if state
		.repairs
		.calls
		.active
		.get(&operation_id)
		.is_some_and(|call| call.step == (CallStep::Preparing { op_id }))
		&& let Some(call) = state.repairs.calls.active.remove(&operation_id)
	{
		refuse(call.request_id, depot_types::client::overloaded(), fx);
	}
}

/// Replies with the call's outcome, keeps it for retries and logs it in the call's audit.
fn finish(
	state: &mut MaintainerState,
	call: Call,
	digest: [u8; 32],
	outcome: RepairOutcome,
	fx: &mut Effects,
) {
	let operation_id = call.request.envelope.operation_id;
	if let Some(op_id) = call.step.deadline() {
		state.finish(op_id, fx);
	}
	reply(
		call.request_id,
		operation_id,
		false,
		outcome_result(outcome.clone()),
		fx,
	);
	// The outcome op rides the log like any other. If this lifetime ends before it commits,
	// a retry finds the intent alone and answers that the outcome is unknown.
	if state.maintainer_log.serving() {
		log_audit(state, &call, digest, Some(outcome.clone()));
	}
	let calls = &mut state.repairs.calls;
	calls.finished.insert(operation_id, (call.request, outcome));
	calls.finished_order.push_back(operation_id);
	while calls.finished_order.len() > MAX_FINISHED {
		if let Some(oldest) = calls.finished_order.pop_front() {
			calls.finished.remove(&oldest);
		}
	}
}

/// Logs the call's audit op, applied to the store through the replay queue once it commits.
fn log_audit(
	state: &mut MaintainerState,
	call: &Call,
	digest: [u8; 32],
	outcome: Option<RepairOutcome>,
) {
	let op = MaintainerOp::RepairAuditOp(RepairAuditOp {
		row: RepairAuditRow {
			request: call.request.clone(),
			requested_at_unix_ms: call.requested_at_unix_ms,
			plan_digest: digest,
			outcome,
		},
	});
	super::super::maintainer_log::replay::submit_unordered(state, op);
}

fn plan(
	state: &MaintainerState,
	request: &RepairRequest,
	prepared: &RepairPrepared,
) -> Result<RepairPlan, RepairResult> {
	let key = call_key(request);
	match &request.body {
		RepairBody::RepairRebuildRecord(body) => {
			super::quarantine::plan(state, &key, body, prepared).map(RepairPlan::RepairRebuildPlan)
		}
		RepairBody::RepairClaimAcquire(body) => super::claims::acquire_plan(state, &key, body)
			.map(RepairPlan::RepairClaimPlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairClaimRelease(body) => super::claims::release_plan(state, &key, body)
			.map(RepairPlan::RepairReleasePlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairAppend(body) => super::claims::append_plan(state, &key, body, prepared)
			.map(RepairPlan::RepairAppendPlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairReplicaReload(body) => super::claims::reload_plan(state, &key, body)
			.map(RepairPlan::RepairReloadPlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairAbortUnload => super::lifecycle::abort_unload_plan(state, &key)
			.map(RepairPlan::RepairAbortUnloadPlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairRedrive => super::lifecycle::redrive_plan(state, &key)
			.map(RepairPlan::RepairRedrivePlan)
			.map_err(RepairResult::RepairRefused),
		RepairBody::RepairReplicaForget(body) => super::forget::plan(state, &key, body)
			.map(RepairPlan::RepairForgetPlan)
			.map_err(RepairResult::RepairRefused),
	}
}

fn run(
	state: &mut MaintainerState,
	request: &RepairRequest,
	plan: &RepairPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	let key = call_key(request);
	let operation_id = request.envelope.operation_id;
	match (&request.body, plan) {
		(RepairBody::RepairRebuildRecord(_), RepairPlan::RepairRebuildPlan(plan)) => {
			super::quarantine::rebuild(state, &key, plan, fx).map(|()| Run::Done(None))
		}
		(RepairBody::RepairClaimAcquire(body), RepairPlan::RepairClaimPlan(plan)) => {
			super::claims::acquire(state, &key, operation_id, body, plan, fx)
		}
		(RepairBody::RepairClaimRelease(body), RepairPlan::RepairReleasePlan(_)) => {
			super::claims::release(state, &key, operation_id, body, fx)
		}
		(RepairBody::RepairAppend(body), RepairPlan::RepairAppendPlan(plan)) => {
			super::claims::append(state, &key, operation_id, body, plan, fx)
		}
		(RepairBody::RepairReplicaReload(body), RepairPlan::RepairReloadPlan(plan)) => {
			super::claims::reload(state, &key, body, plan, fx)
		}
		(RepairBody::RepairAbortUnload, RepairPlan::RepairAbortUnloadPlan(plan)) => {
			super::lifecycle::abort_unload(state, &key, plan, fx)
		}
		(RepairBody::RepairReplicaForget(body), RepairPlan::RepairForgetPlan(plan)) => {
			super::forget::forget(state, &key, operation_id, body, plan, fx)
		}
		(RepairBody::RepairRedrive, RepairPlan::RepairRedrivePlan(plan)) => {
			super::lifecycle::redrive(state, &key, plan, fx)
		}
		(
			RepairBody::RepairRebuildRecord(_)
			| RepairBody::RepairClaimAcquire(_)
			| RepairBody::RepairClaimRelease(_)
			| RepairBody::RepairAppend(_)
			| RepairBody::RepairReplicaReload(_)
			| RepairBody::RepairAbortUnload
			| RepairBody::RepairRedrive
			| RepairBody::RepairReplicaForget(_),
			_,
		) => Err(refusal(
			RepairRefusalCode::PlanChanged,
			None,
			"the plan does not fit the call's body".into(),
			None,
		)),
	}
}

/// The digest a real call must carry: of the request's body and volume beside the plan, so a
/// changed fact or a changed body gives another digest. The operation id, reason and dry
/// run flag are not part of it, so the dry run's digest fits the real call.
pub(crate) fn digest(request: &RepairRequest, plan: &RepairPlan) -> [u8; 32] {
	let mut hasher = Sha256::new();
	hasher.update(request.envelope.volume_log_key.encode_to_bytes());
	hasher.update(request.envelope.tablet_id);
	hasher.update(request.envelope.tablet_grant_epoch.to_be_bytes());
	hasher.update(request.body.encode_to_bytes());
	hasher.update(plan.encode_to_bytes());
	hasher.finalize().into()
}

/// Whether `retry` is the same call as `first`: everything but the dry run flag and the plan
/// digest, which a dry run and its real call differ in.
fn same_call(first: &RepairRequest, retry: &RepairRequest) -> bool {
	let mut first = first.clone();
	let mut retry = retry.clone();
	first.envelope.dry_run = false;
	retry.envelope.dry_run = false;
	first.envelope.plan_digest = None;
	retry.envelope.plan_digest = None;
	first == retry
}

fn outcome_result(outcome: RepairOutcome) -> RepairResult {
	match outcome {
		RepairOutcome::RepairApplied(applied) => RepairResult::RepairApplied(applied),
		RepairOutcome::RepairRefused(refused) => RepairResult::RepairRefused(refused),
		RepairOutcome::RepairUnknown(unknown) => RepairResult::RepairUnknown(unknown),
	}
}

fn conflict() -> RepairResult {
	refused(
		RepairRefusalCode::OperationConflict,
		None,
		"this operation id was used by a call with another body".into(),
		None,
	)
}

pub(crate) fn refused(
	code: RepairRefusalCode,
	retry_after_ms: Option<u64>,
	message: String,
	current: Option<RepairPlan>,
) -> RepairResult {
	RepairResult::RepairRefused(refusal(code, retry_after_ms, message, current))
}

pub(crate) fn refusal(
	code: RepairRefusalCode,
	retry_after_ms: Option<u64>,
	message: String,
	current: Option<RepairPlan>,
) -> RepairRefused {
	RepairRefused {
		code,
		retry_after_ms,
		message: message.into(),
		current,
	}
}

fn reply(
	request_id: RequestId,
	operation_id: [u8; 16],
	already_applied: bool,
	result: RepairResult,
	fx: &mut Effects,
) {
	fx.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::RepairResponse(RepairResponse {
			operation_id,
			already_applied,
			result,
		}),
	});
}
