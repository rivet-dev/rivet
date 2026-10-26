//! Bounded discovery passes advance to a fixed domain-deadline horizon without rewinding.
//! Omitted hints request a later pass; future hints never prevent a pass from finishing.
use super::{MaintainerPhase, MaintainerState, Operation};
use crate::{
	alarms::{Request as AlarmRequest, Response as AlarmResponse},
	tablet_maintainer_machine::{
		Effects,
		effect::MaintainerEffect,
		types::{MaintainerTimer, OpId},
	},
};
use depot_types::{VolumeLogKey, alarm::revision};
use rivet_data::generated::{
	depot_alarm_v1::{AlarmCurrent, AlarmRetryRecord},
	depot_tablet_coordinator_protocol_v1::{
		TabletCoordinatorRejectResponse as CoordinatorReject,
		TabletCoordinatorResponse as CoordinatorResponse,
	},
	shared_depot_alarms_v1::{AlarmOwner, AlarmSummary},
	shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Alarms {
	pub enabled: bool,
	pub owner: Option<AlarmOwner>,
	pub owner_id: [u8; 16],
	candidates: rivet_state_machine::IndexedDeadlineQueue<VolumeLogKey>,
	/// At most one invalidation per occupied volume, bounded by active and parked work.
	deferred: BTreeMap<depot_types::VolumeKey, VolumeLogKey>,
	scan: Scan,
	armed_at: Option<u64>,
	/// Earliest retry pass for an omitted hint; independent of cached future deadlines.
	rescan_after: Option<u64>,
	/// Alternate ready retries and fresh candidates when both can use the next slot.
	prefer_queued: bool,
	pub initialization: Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmIndexSource>,
	pub reporting_ready: bool,
	pub split_manifest: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Attempt {
	/// The projection waits for its own conflicting store work; no I/O or timer is running.
	Queued(AlarmRequest),
	/// Executor work is in flight; expiry cancels before releasing capacity.
	Running(AlarmRequest),
	/// Keep capacity until cancellation completes, then retry the same immutable I/O body.
	Cancelling(AlarmRequest),
	/// No future or execution slot is held; the retry deadline owns the immutable body.
	Waiting(AlarmRequest),
	/// A replacement invalidated the work; retain capacity until its cancellation completes.
	Obsolete(AlarmRequest),
}
impl Attempt {
	pub(crate) fn request(&self) -> &AlarmRequest {
		match self {
			Self::Queued(r)
			| Self::Running(r)
			| Self::Cancelling(r)
			| Self::Waiting(r)
			| Self::Obsolete(r) => r,
		}
	}
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Scan {
	after: Option<bytes::Bytes>,
	/// Fixed at the first page; callback retries cannot move the end of a live pass.
	through_unix_ms: Option<i64>,
	/// EOF or a page beyond the fixed horizon, whose earliest future wake starts the next pass.
	exhausted: bool,
	epoch: u64,
}

/// Only the derived alarm index is visited, never the unrelated operation map.
fn pending(state: &MaintainerState) -> Vec<(OpId, Attempt)> {
	state
		.operations
		.index()
		.alarms
		.iter()
		.filter_map(|id| match state.operations.get(id) {
			Some(Operation::Alarm(attempt)) => Some((*id, attempt.clone())),
			_ => None,
		})
		.collect()
}
fn pending_volume(state: &MaintainerState, key: &depot_types::VolumeKey) -> Vec<(OpId, Attempt)> {
	state
		.operations
		.index()
		.alarms_by_volume
		.get(key)
		.into_iter()
		.flatten()
		.filter_map(|id| match state.operations.get(id) {
			Some(Operation::Alarm(attempt)) => Some((*id, attempt.clone())),
			_ => None,
		})
		.collect()
}
/// Projection work obeys the same per-volume and tablet barriers as filesystem replay.
fn blocked(state: &MaintainerState, request: &AlarmRequest) -> bool {
	if !request.uses_store() {
		return false;
	}
	let tablet = super::footprint::TabletInFlight::of(state);
	let volume = request
		.key()
		.map(|key| super::footprint::StoreVolume::from(&key.0.volume_key));
	tablet.exclusive
		|| tablet.pending_scan
		|| state.operations.index().store.keyed >= state.config.max_keyed_in_flight as usize
		|| volume.is_some_and(|volume| {
			super::footprint::volume_conflict(state, &volume, super::footprint::VolumeAccess::Keyed)
		})
}
fn start(state: &mut MaintainerState, request: AlarmRequest, fx: &mut Effects) {
	if blocked(state, &request) {
		park(state, Attempt::Queued(request), fx);
	} else {
		let id = state.begin(Operation::Alarm(Attempt::Running(request.clone())), fx);
		fx.execute(MaintainerEffect::AlarmRequest { op_id: id, request });
	}
}
fn wait(state: &mut MaintainerState, request: AlarmRequest, fx: &mut Effects) {
	park(state, Attempt::Waiting(request), fx);
}
fn rediscover_at(state: &mut MaintainerState, at: u64, now: u64) {
	let at = if at > now {
		at
	} else {
		now.saturating_add(state.config.alarms.retry_min_ms)
	};
	state.alarms.rescan_after = Some(state.alarms.rescan_after.map_or(at, |old| old.min(at)));
}
fn rediscover(state: &mut MaintainerState, fx: &Effects) {
	rediscover_at(state, fx.now_boot_ms(), fx.now_boot_ms());
}
/// Parked bodies have a separate bounded budget, so failures cannot reserve execution slots.
fn park(state: &mut MaintainerState, attempt: Attempt, fx: &mut Effects) {
	if attempt.request().key().is_some()
		&& state.operations.index().alarm_parked.len()
			>= state.config.alarms.max_candidates as usize
	{
		// Reads have no uncertain mutation to retain; their canonical index entry survives.
		if matches!(attempt.request(), AlarmRequest::Read { .. }) {
			if let Some(key) = attempt.request().key() {
				state.alarms.candidates.cancel(key);
				state.alarms.deferred.remove(&key.0.volume_key);
			}
			rediscover(state, fx);
			return;
		}
		let read = state.operations.index().alarm_parked.iter().copied().find(|id| {
			matches!(state.operations.get(id), Some(Operation::Alarm(a)) if matches!(a.request(), AlarmRequest::Read { .. }))
		});
		if let Some(id) = read {
			if let Some(Operation::Alarm(read)) = state.finish(id, fx) {
				if let Some(key) = read.request().key() {
					state.alarms.deferred.remove(&key.0.volume_key);
				}
			}
			rediscover(state, fx);
		} else {
			// Never discard or rebase an uncertain write to make room. End this owner
			// if its bounded retention is exhausted; recovery uses the canonical log.
			super::fence(state, fx);
			return;
		}
	}
	state.begin(Operation::Alarm(attempt), fx);
}
fn obsolete(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	match state.operations.get(&id).cloned() {
		Some(Operation::Alarm(Attempt::Running(request) | Attempt::Cancelling(request))) => {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Obsolete(request)));
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id });
		}
		Some(Operation::Alarm(Attempt::Queued(_) | Attempt::Waiting(_))) => {
			state.finish(id, fx);
		}
		_ => {}
	}
}

/// Preserve admitted hints and their deadlines. Future hints do not reserve page
/// capacity, and due hints must keep their place while a new pass discovers more work.
fn restart_scan(state: &mut MaintainerState) {
	state.alarms.rescan_after = None;
	state.alarms.scan.after = None;
	state.alarms.scan.through_unix_ms = None;
	state.alarms.scan.exhausted = false;
	state.alarms.scan.epoch = state.alarms.scan.epoch.wrapping_add(1);
}
fn candidate(state: &mut MaintainerState, key: VolumeLogKey, at_boot_ms: u64, now: u64) {
	if state.alarms.owner.is_none() {
		return;
	}
	let key = key.detach();
	if state
		.operations
		.index()
		.alarms_by_volume
		.contains_key(&key.0.volume_key)
	{
		state
			.alarms
			.deferred
			.entry(key.0.volume_key.clone())
			.or_insert_with(|| key.clone());
		return;
	}
	let cap = state.config.alarms.max_candidates as usize;
	if state.alarms.candidates.len() >= cap && state.alarms.candidates.scheduled_at(&key).is_none()
	{
		// Keep the earliest hints without rewinding this pass. Rewinding on every
		// eviction lets repeated failures or new commits starve later index rows.
		let last = state.alarms.candidates.iter().next_back().cloned();
		if let Some((at, last)) = last {
			if at_boot_ms >= at {
				rediscover_at(state, at_boot_ms, now);
				return;
			}
			state.alarms.candidates.cancel(&last);
			rediscover_at(state, at, now);
		}
	}
	state.alarms.candidates.schedule_earliest(key, at_boot_ms);
}
fn due_at(
	state: &MaintainerState,
	key: &VolumeLogKey,
	current: &AlarmCurrent,
	fx: &Effects,
) -> Option<u64> {
	let pending = current.summary.pending.as_ref()?;
	let domain = fx.now_boot_ms().saturating_add(
		pending
			.deadline_unix_ms
			.saturating_sub(fx.clocks().unix_ms)
			.max(0) as u64,
	);
	let retry = current.retry.as_ref().filter(|retry| {
		Some(&retry.revision) == revision(key.0.volume_incarnation, &current.summary).as_ref()
	});
	let retry = retry.map_or(0, |retry| {
		if retry.owner_id == state.alarms.owner_id {
			retry.next_boot_ms
		} else {
			fx.now_boot_ms().saturating_add(
				retry
					.next_unix_ms
					.saturating_sub(fx.clocks().unix_ms)
					.max(0) as u64,
			)
		}
	});
	Some(domain.max(retry))
}
fn current_candidate(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	current: &AlarmCurrent,
	fx: &Effects,
) {
	if let Some(at) = due_at(state, &key, current, fx) {
		candidate(state, key, at, fx.now_boot_ms());
	}
}

/// The committed postimage supplies the new deadline and invalidates the previous attempt.
pub(crate) fn changed(
	state: &mut MaintainerState,
	key: VolumeLogKey,
	summary: Option<&AlarmSummary>,
	fx: &mut Effects,
) {
	state.alarms.deferred.remove(&key.0.volume_key);
	if let Some(summary) = summary {
		for (id, attempt) in pending_volume(state, &key.0.volume_key) {
			let stale = match attempt.request() {
				AlarmRequest::Call {
					key: old,
					summary: delivered,
					..
				}
				| AlarmRequest::Retry {
					key: old,
					summary: delivered,
					..
				} => {
					old == &key
						&& revision(key.0.volume_incarnation, delivered)
							!= revision(key.0.volume_incarnation, summary)
				}
				AlarmRequest::Read { key: old } => old == &key,
				AlarmRequest::Page { .. } => false,
			};
			if stale {
				obsolete(state, id, fx);
			}
		}
		state.alarms.candidates.cancel(&key);
		current_candidate(
			state,
			key,
			&AlarmCurrent {
				summary: summary.clone(),
				retry: None,
			},
			fx,
		);
	} else {
		candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
	}
}
pub(crate) fn published(state: &mut MaintainerState, record: &FsVolumeRecord, fx: &mut Effects) {
	let key = VolumeLogKey::from(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
		volume_key: record.volume_key(),
		volume_incarnation: record.volume_incarnation,
	});
	if record.status != FsVolumeStatus::Deleted
		|| state
			.alarms
			.deferred
			.get(&key.0.volume_key)
			.is_some_and(|old| old.0.volume_incarnation == key.0.volume_incarnation)
	{
		state.alarms.deferred.remove(&key.0.volume_key);
	}
	for (id, attempt) in pending_volume(state, &key.0.volume_key) {
		if attempt.request().key().is_some_and(|old| {
			if record.status == FsVolumeStatus::Deleted {
				old.0.volume_incarnation == key.0.volume_incarnation
			} else {
				old.0.volume_incarnation != key.0.volume_incarnation
			}
		}) {
			obsolete(state, id, fx);
		}
	}
	state.alarms.candidates.cancel(&key);
	if record.status != FsVolumeStatus::Deleted {
		candidate(state, key, fx.now_boot_ms(), fx.now_boot_ms());
	}
}
pub(crate) fn tick(state: &mut MaintainerState, _fx: &mut Effects) {
	state.alarms.armed_at = None;
}

pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.alarms.owner.is_none() {
		return;
	}
	if state.phase != MaintainerPhase::Serving {
		if state.alarms.armed_at.take().is_some() {
			fx.cancel_timer(MaintainerTimer::Alarms);
		}
		for (id, attempt) in pending(state) {
			match attempt {
				Attempt::Running(_) | Attempt::Cancelling(_) | Attempt::Obsolete(_) => {
					fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id })
				}
				Attempt::Queued(_) | Attempt::Waiting(_) => {
					state.finish(id, fx);
				}
			}
		}
		return;
	}
	if state.alarms.scan.exhausted
		&& state
			.alarms
			.rescan_after
			.is_some_and(|at| at <= fx.now_boot_ms())
	{
		restart_scan(state);
	}
	while state.phase == MaintainerPhase::Serving
		&& state.operations.index().counts.alarm_keyed < state.config.alarms.max_operations
	{
		let queued = state.operations.index().alarm_queued.iter().find_map(|id| {
			match state.operations.get(id) {
				Some(Operation::Alarm(Attempt::Queued(request))) if !blocked(state, request) => {
					Some((*id, request.clone()))
				}
				_ => None,
			}
		});
		let due = state
			.alarms
			.candidates
			.next_at()
			.is_some_and(|at| at <= fx.now_boot_ms());
		if let Some((id, request)) = queued.filter(|_| state.alarms.prefer_queued || !due) {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Running(request.clone())));
			let timeout = state.config.alarms.io_timeout_ms;
			fx.arm_timer_after(MaintainerTimer::Operation(id), timeout);
			fx.execute(MaintainerEffect::AlarmRequest { op_id: id, request });
			state.alarms.prefer_queued = false;
			continue;
		}
		let Some((_, key)) = state.alarms.candidates.pop_due(fx.now_boot_ms(), 1).pop() else {
			break;
		};
		// Completion of the occupying operation revalidates the key, so no busy-wait timer is needed.
		if state
			.operations
			.index()
			.alarms_by_volume
			.contains_key(&key.0.volume_key)
		{
			state
				.alarms
				.deferred
				.entry(key.0.volume_key.clone())
				.or_insert(key);
			continue;
		}
		start(state, AlarmRequest::Read { key }, fx);
		state.alarms.prefer_queued = true;
	}
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	if !state.alarms.scan.exhausted
		&& state.operations.index().alarm_pages.is_empty()
		&& (state.alarms.candidates.len() < state.config.alarms.max_candidates as usize
			|| state
				.alarms
				.candidates
				.next_at()
				.is_some_and(|at| at > fx.now_boot_ms()))
	{
		state
			.alarms
			.scan
			.through_unix_ms
			.get_or_insert(fx.clocks().unix_ms);
		start(
			state,
			AlarmRequest::Page {
				after: state.alarms.scan.after.clone(),
				epoch: state.alarms.scan.epoch,
				limit: (state.config.alarms.page_size as usize).min(
					(state.config.alarms.max_candidates as usize - state.alarms.candidates.len())
						.max(1),
				),
			},
			fx,
		);
	}
	let mut next =
		if state.operations.index().counts.alarm_keyed < state.config.alarms.max_operations {
			state.alarms.candidates.next_at()
		} else {
			None
		};
	if state.alarms.scan.exhausted
		&& let Some(at) = state.alarms.rescan_after
	{
		next = Some(next.map_or(at, |next| next.min(at)));
	}
	if next != state.alarms.armed_at {
		state.alarms.armed_at = next;
		match next {
			Some(at) => fx.arm_timer_at(MaintainerTimer::Alarms, at),
			None => fx.cancel_timer(MaintainerTimer::Alarms),
		}
	}
}
pub(crate) fn expired(state: &mut MaintainerState, id: OpId, fx: &mut Effects) {
	match state.operations.get(&id).cloned() {
		Some(Operation::Alarm(Attempt::Running(request))) => {
			state
				.operations
				.insert(id, Operation::Alarm(Attempt::Cancelling(request)));
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id });
		}
		Some(Operation::Alarm(Attempt::Cancelling(_) | Attempt::Obsolete(_))) => {
			fx.execute(MaintainerEffect::CancelAlarmRequest { op_id: id })
		}
		Some(Operation::Alarm(Attempt::Waiting(request))) => {
			if state.phase == MaintainerPhase::Serving {
				state
					.operations
					.insert(id, Operation::Alarm(Attempt::Queued(request)));
			} else {
				state.finish(id, fx);
			}
		}
		_ => {}
	}
}
pub(crate) fn completed(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<AlarmResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&id),
		Some(Operation::Alarm(
			Attempt::Running(_) | Attempt::Cancelling(_) | Attempt::Obsolete(_)
		))
	) {
		return;
	}
	let Some(Operation::Alarm(attempt)) = state.finish(id, fx) else {
		return;
	};
	let recheck = attempt
		.request()
		.key()
		.and_then(|key| state.alarms.deferred.remove(&key.0.volume_key));
	let rechecked = recheck.is_some();
	if let Some(latest) = recheck {
		// A live attempt already validates its own key. Reoffering the same hint
		// before parking its retry can evict the next scan row and rewind discovery.
		if matches!(attempt, Attempt::Obsolete(_)) || attempt.request().key() != Some(&latest) {
			candidate(state, latest, fx.now_boot_ms(), fx.now_boot_ms());
		}
	}
	let (request, result) = match attempt {
		Attempt::Running(request) => (request, result),
		Attempt::Cancelling(request) if matches!(request, AlarmRequest::Call { .. }) => {
			(request, Err("alarm callback timed out".to_owned()))
		}
		Attempt::Cancelling(request) => {
			if state.phase == MaintainerPhase::Serving {
				wait(state, request, fx);
			}
			return;
		}
		Attempt::Obsolete(request) => {
			if !rechecked {
				if let Some(key) = request.key() {
					candidate(state, key.clone(), fx.now_boot_ms(), fx.now_boot_ms());
				}
			}
			return;
		}
		Attempt::Waiting(_) | Attempt::Queued(_) => return,
	};
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let result = match result {
		Err(_) if matches!(request, AlarmRequest::Call { .. }) => Ok(AlarmResponse::Called {
			busy: false,
			randomness: 0,
		}),
		result => result,
	};
	match (request, result) {
		(request, Err(_)) => wait(state, request, fx),
		(AlarmRequest::Page { epoch, .. }, Ok(AlarmResponse::Page { rows, done })) => {
			if epoch != state.alarms.scan.epoch {
				return;
			}
			let mut reached_horizon = false;
			for (physical, row) in rows {
				state.alarms.scan.after = Some(physical);
				let key = VolumeLogKey::from(row.volume_log_key);
				if super::in_tablet(state, &key.0.volume_key) {
					let current = AlarmCurrent {
						summary: row.summary,
						retry: row.retry,
					};
					if let Some(eligible) = depot_types::alarm::index_deadline(&current.summary)
						&& state
							.alarms
							.scan
							.through_unix_ms
							.is_some_and(|through| eligible > through)
					{
						reached_horizon = true;
						if let Some(at) = due_at(state, &key, &current, fx) {
							rediscover_at(state, at, fx.now_boot_ms());
						}
					}
					current_candidate(state, key, &current, fx);
				}
			}
			state.alarms.scan.exhausted = done || reached_horizon;
		}
		(AlarmRequest::Read { key }, Ok(AlarmResponse::Read { current })) => {
			let Some(current) = current else {
				return;
			};
			let Some(at) = due_at(state, &key, &current, fx) else {
				return;
			};
			if at > fx.now_boot_ms() {
				candidate(state, key, at, fx.now_boot_ms());
				return;
			}
			start(
				state,
				AlarmRequest::Call {
					key,
					summary: current.summary,
					attempt: current.retry.map_or(0, |retry| retry.attempt),
				},
				fx,
			);
		}
		(
			AlarmRequest::Call {
				key,
				summary,
				attempt,
			},
			Ok(AlarmResponse::Called { randomness, .. }),
		) => {
			let ceiling = state
				.config
				.alarms
				.retry_min_ms
				.saturating_mul(1u64 << attempt.min(63))
				.min(state.config.alarms.retry_max_ms);
			let delay = ceiling
				.saturating_add(u64::from(randomness) % (ceiling / 4 + 1))
				.min(state.config.alarms.retry_max_ms);
			let Some(revision) = revision(key.0.volume_incarnation, &summary) else {
				return;
			};
			let record = AlarmRetryRecord {
				revision,
				next_unix_ms: fx.clocks().unix_ms.saturating_add(delay as i64),
				next_boot_ms: fx.now_boot_ms().saturating_add(delay),
				attempt: attempt.saturating_add(1),
				owner_id: state.alarms.owner_id,
			};
			start(
				state,
				AlarmRequest::Retry {
					key,
					summary,
					record,
				},
				fx,
			);
		}
		(AlarmRequest::Retry { key, .. }, Ok(AlarmResponse::Updated { current })) => {
			if let Some(current) = current {
				current_candidate(state, key, &current, fx);
			}
		}
		(
			AlarmRequest::Page { .. }
			| AlarmRequest::Read { .. }
			| AlarmRequest::Call { .. }
			| AlarmRequest::Retry { .. },
			Ok(
				AlarmResponse::Page { .. }
				| AlarmResponse::Read { .. }
				| AlarmResponse::Called { .. }
				| AlarmResponse::Updated { .. },
			),
		) => super::fence(state, fx),
	}
}

pub(crate) fn snapshot(state: &MaintainerState) -> crate::tablet_maintainer_machine::AlarmSnapshot {
	if state.alarms.owner.is_none() {
		return Default::default();
	}
	crate::tablet_maintainer_machine::AlarmSnapshot {
		candidates: state.alarms.candidates.len() as u32,
		operations: state.operations.index().counts.alarm_keyed,
		parked: state.operations.index().alarm_parked.len() as u32,
		parked_capacity: state.config.alarms.max_candidates,
		candidate_capacity: state.config.alarms.max_candidates,
		operation_capacity: state.config.alarms.max_operations,
	}
}

pub(crate) fn validate(state: &MaintainerState) -> anyhow::Result<()> {
	state.alarms.candidates.debug_validate()?;
	anyhow::ensure!(
		state.operations.index().counts.alarm_keyed <= state.config.alarms.max_operations,
		"alarm execution capacity exceeded"
	);
	anyhow::ensure!(
		state.operations.index().alarm_parked.len() <= state.config.alarms.max_candidates as usize,
		"parked alarm capacity exceeded"
	);
	anyhow::ensure!(
		state.alarms.candidates.len() <= state.config.alarms.max_candidates as usize,
		"alarm frontier capacity exceeded"
	);
	anyhow::ensure!(
		state.operations.index().alarm_pages.len() <= 1,
		"concurrent alarm pages"
	);
	Ok(())
}

pub(crate) fn ready_response(
	state: &mut MaintainerState,
	result: Result<
		rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
		String,
	>,
	fx: &mut Effects,
) {
	if state.phase != super::MaintainerPhase::Opening || !state.alarms.reporting_ready {
		return;
	}
	state.alarms.reporting_ready = false;
	match result {
		Ok(CoordinatorResponse::Accepted(_)) => {
			state.alarms.initialization = None;
			super::opening::finish(state, fx);
		}
		Err(_)
		| Ok(CoordinatorResponse::TabletCoordinatorRejectResponse(
			CoordinatorReject::Overloaded | CoordinatorReject::NotCoordinator,
		)) => fx.arm_timer_after(MaintainerTimer::SplitRetry, state.config.tuning().retry_ms),
		_ => super::fence(state, fx),
	}
}


