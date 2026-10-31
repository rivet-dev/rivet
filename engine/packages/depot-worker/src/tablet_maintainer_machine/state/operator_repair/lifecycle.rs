//! Repair calls that nudge a volume's own lifecycle state machines: abort an unload that has
//! not forgotten any replica, and fire the volume's pending retry deadlines now. Neither adds a
//! transition: an abort takes the cancel a reopen takes, and a redrive delivers the same timers
//! the state machines armed themselves, which they handle idempotently.

use super::calls::{Run, refusal};
use crate::tablet_maintainer_machine::{Effects, state::MaintainerState, types::MaintainerTimer};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_client_protocol_v1::{
	RepairAbortUnloadPlan, RepairRedrivePlan, RepairRefusalCode, RepairRefused,
};

fn refused(code: RepairRefusalCode, retry_after_ms: Option<u64>, message: &str) -> RepairRefused {
	refusal(code, retry_after_ms, message.into(), None)
}

/// Checks that `key`'s unload may be aborted: it is unloading, no forget of it was sent or
/// confirmed, and no recovery of it is in flight.
pub(crate) fn abort_unload_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
) -> Result<RepairAbortUnloadPlan, RepairRefused> {
	let Some(unload) = state.unloading.get(key) else {
		return Err(refused(
			RepairRefusalCode::NotUnloading,
			None,
			"the incarnation is not unloading",
		));
	};
	let forgetting = state
		.operations
		.index()
		.forgets
		.range(super::super::operations::workers_of(key))
		.next()
		.is_some();
	if unload.closed.is_some() || !unload.confirmed.is_empty() || forgetting {
		return Err(refused(
			RepairRefusalCode::ForgetStarted,
			None,
			"the unload already forgot or is forgetting replicas",
		));
	}
	if super::super::repair::holds_key(state, &key.0.volume_key) {
		return Err(refused(
			RepairRefusalCode::RepairBusy,
			Some(1000),
			"the unload's recovery is in flight",
		));
	}
	Ok(RepairAbortUnloadPlan {
		reason: format!("{:?}", unload.reason).into(),
	})
}

/// Runs an abort: the cancel a reopen takes, then a full idle period before the next check.
pub(crate) fn abort_unload(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	plan: &RepairAbortUnloadPlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	if abort_unload_plan(state, key)? != *plan {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			None,
			"another unload of the incarnation began since the plan",
		));
	}
	tracing::warn!(?key, "an operator aborted a volume's unload");
	super::super::unload::cancel(state, key, fx);
	super::super::unload::postpone_idle(state, key);
	Ok(Run::Done(None))
}

/// The retry deadlines of `key` that are pending now.
fn pending_retries(state: &MaintainerState, key: &VolumeLogKey) -> Vec<(String, MaintainerTimer)> {
	let mut retries = Vec::new();
	if let Some(unload) = state.unloading.get(key) {
		if !unload.waiting.is_empty() {
			retries.push((
				"unload_forget".to_owned(),
				MaintainerTimer::UnloadRetry(key.clone()),
			));
		}
		if unload.recovery_backoff {
			retries.push((
				"unload_recovery".to_owned(),
				MaintainerTimer::UnloadRecoveryRetry(key.clone()),
			));
		}
	}
	if state
		.filesystem
		.volumes
		.get(key)
		.is_some_and(|volume| volume.retry_wait)
	{
		retries.push((
			"filesystem_apply".to_owned(),
			MaintainerTimer::FilesystemRetry(key.clone()),
		));
	}
	for worker in super::super::member_repair::waiting_members(state, key) {
		retries.push((
			format!(
				"member_reload:{}",
				uuid::Uuid::from_bytes(*worker.as_bytes())
			),
			MaintainerTimer::MemberRepairRetry(key.clone(), worker),
		));
	}
	retries
}

/// Names the retries a redrive would fire, or refuses when none is pending.
pub(crate) fn redrive_plan(
	state: &MaintainerState,
	key: &VolumeLogKey,
) -> Result<RepairRedrivePlan, RepairRefused> {
	let retries = pending_retries(state, key);
	if retries.is_empty() {
		return Err(refused(
			RepairRefusalCode::NothingToRedrive,
			None,
			"the incarnation has no pending retry deadline",
		));
	}
	Ok(RepairRedrivePlan {
		retries: retries.into_iter().map(|(name, _)| name.into()).collect(),
	})
}

/// Runs a redrive: rearms each pending retry deadline to fire now, which replaces the armed
/// one, so every retry runs once.
pub(crate) fn redrive(
	state: &mut MaintainerState,
	key: &VolumeLogKey,
	plan: &RepairRedrivePlan,
	fx: &mut Effects,
) -> Result<Run, RepairRefused> {
	let retries = pending_retries(state, key);
	let names = retries
		.iter()
		.map(|(name, _)| name.clone().into())
		.collect::<Vec<vbare::Str>>();
	if names != plan.retries {
		return Err(refused(
			RepairRefusalCode::PlanChanged,
			None,
			"the incarnation's pending retries changed since the plan",
		));
	}
	tracing::info!(?key, retries = ?names, "an operator redrove a volume's pending retries");
	for (_, timer) in retries {
		fx.arm_timer_after(timer, 0);
	}
	Ok(Run::Done(None))
}


