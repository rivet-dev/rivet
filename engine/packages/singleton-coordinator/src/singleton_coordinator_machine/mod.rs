//! One candidate per eligible worker. Ownership is reusable infrastructure, not placement policy.
use rivet_state_machine_util::{IoTimer, LeaseChange};
pub mod effect;
pub mod event;
pub mod executor;
pub mod snapshot;
pub mod state;
pub mod types;

mod publication;
use effect::Effect;
use event::Event;
use rivet_env::Clocks;
use rivet_object_store_util::conditional_write as cw;
use rivet_state_machine::Entropy;
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use state::{
	ControlState, IoCall, ManifestView, Mutation, OwnerHeartbeat, ServicesState, State, Yielding,
};
use types::*;

/// Starts the existing ClusterSingleton scope after a confirmed named lease claim.
/// Worker expiry permanently fences this candidate; coordinator loss stops only its singleton scope.
pub struct SingletonCoordinator;
type Effects = rivet_state_machine_util::Effects<Effect, Timer>;
impl rivet_state_machine::StateMachineState for State {}
impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::AuthorityChanged(..) => "authority_changed",
			Event::StopRequest => "stop_request",
			Event::CloseRequest => "close_request",
			Event::RelinquishTenure { .. } => "relinquish_tenure",
			Event::OwnerUnlisted { .. } => "owner_unlisted",
			Event::OwnerListed { .. } => "owner_listed",
			Event::EntropyResponse { .. } => "entropy_response",
			Event::ReadLeaseResponse { .. } => "read_lease_response",
			Event::InspectHeartbeatResponse { .. } => "inspect_heartbeat_response",
			Event::AcquireLeaseResponse { .. } => "acquire_lease_response",
			Event::ReleaseLeaseResponse { .. } => "release_lease_response",
			Event::StepDownResponse { .. } => "step_down_response",
			Event::WithdrawStepDownResponse { .. } => "withdraw_step_down_response",
			Event::StopScopeResponse { .. } => "stop_scope_response",
		}
	}
}
impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::EntropyRequest { .. } => "entropy_request",
			Effect::ReadLeaseRequest { .. } => "read_lease_request",
			Effect::InspectHeartbeatRequest { .. } => "inspect_heartbeat_request",
			Effect::AcquireLeaseRequest { .. } => "acquire_lease_request",
			Effect::ReleaseLeaseRequest { .. } => "release_lease_request",
			Effect::StepDownRequest { .. } => "step_down_request",
			Effect::WithdrawStepDownRequest { .. } => "withdraw_step_down_request",
			Effect::StartScope { .. } => "start_scope",
			Effect::StopScopeRequest { .. } => "stop_scope_request",
		}
	}
}
impl rivet_state_machine::Snapshot for snapshot::Snapshot {}

impl EphemeralStateMachine for SingletonCoordinator {
	type Slot = Slot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = snapshot::Snapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "singleton_coordinator";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			Timer::Publication(..) => "publication",
			Timer::Check => "check",
			Timer::Io(..) => "io",
			Timer::WorkerLease => "worker_lease",
			Timer::StepDown => "step_down",
			Timer::Stop => "stop",
		}
	}

	fn start(state: &mut State, now: Clocks, _: Entropy, out: &mut Effects) {
		state.now_boot_ms = now.boot_ms;
		if can_work(state) {
			schedule(state, out);
		}
	}

	fn on_event(state: &mut State, event: EphemeralStateMachineEvent<Self>, out: &mut Effects) {
		state.now_boot_ms = event.now.boot_ms;
		// Earlier overdue alarms must not dispatch after the worker lease expires.
		if matches!(event.body, RuntimeEvent::Timer(_))
			&& state.lease.on_timer(out) == LeaseChange::Fenced
		{
			fence(state, out);
		}
		let body = match event.body {
			RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op))) => timeout_response(state, op)
				.map(RuntimeEvent::Input)
				.unwrap_or(RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op)))),
			body => body,
		};
		match body {
			RuntimeEvent::Input(Event::AuthorityChanged(view)) => {
				state.eligible = view.eligible;
				state.store_clock = view.store_clock;
				let deadline = if view.fenced {
					None
				} else {
					view.deadline_boot_ms
				};
				if state.lease.observe(deadline, out) == LeaseChange::Fenced {
					fence(state, out);
				}
				if state.lease.live() {
					reconcile_intent(state, out);
					if matches!(state.control, ControlState::Idle) && can_work(state) {
						schedule(state, out);
					}
					// A new store clock anchor or eligibility can move the next deadline.
					rearm_check(state, out);
				}
			}
			RuntimeEvent::Input(Event::StopRequest) => {
				state.shutdown = true;
				reconcile_intent(state, out);
				if state.services == ServicesState::Absent && state.control.mutation().is_none() {
					suspend(state, out);
				}
			}
			RuntimeEvent::Input(Event::CloseRequest) => {
				state.shutdown = true;
				// Revoking before an announced deadline is always safe, and storage has already
				// drained, so serving out a step-down would only outlast the teardown budget.
				stop_scope(state, "coordinator closed", out);
				if state.services == ServicesState::Absent && state.control.mutation().is_none() {
					suspend(state, out);
				}
			}
			RuntimeEvent::Input(Event::OwnerUnlisted { owner }) => {
				let watched = known_lease(state)
					.filter(|lease| !lease.released)
					.is_some_and(|lease| lease.tenure.owner == owner);
				if watched {
					// Drop live evidence so the next check inspects now, but keep a
					// missing-heartbeat wait: the hint cannot shorten it.
					if matches!(
						&state.owner_heartbeat,
						Some(OwnerHeartbeat::Live { identity, .. }) if *identity == owner
					) {
						state.owner_heartbeat = None;
					}
					state.owner_unlisted = Some(owner);
					rearm_check(state, out);
				}
			}
			RuntimeEvent::Input(Event::OwnerListed { owner }) => {
				if state.owner_unlisted == Some(owner) {
					state.owner_unlisted = None;
				}
			}
			RuntimeEvent::Input(Event::RelinquishTenure { tenure, reason: _ }) => {
				if serving_tenure(state) == Some(&tenure) {
					stop_scope(state, "tenure relinquished", out);
				}
			}
			RuntimeEvent::Input(Event::StopScopeResponse {
				tenure,
				effects_safe,
			}) => {
				if matches!(&state.services, ServicesState::Stopping { tenure: current, .. } if *current == tenure)
				{
					out.cancel_timer(Timer::Stop);
					if !effects_safe {
						// Cancellation does not fence already-sent remote writes. Never release on uncertainty.
						tracing::error!(
							"singleton services stopped with remote writes possibly in flight; exiting instead of releasing the tenure"
						);
						out.exit();
					} else if state.lease.fenced() {
						tracing::info!(
							coordinator_generation = tenure.coordinator_generation,
							"singleton services joined"
						);
						state.services = ServicesState::Absent;
					} else {
						tracing::info!(
							coordinator_generation = tenure.coordinator_generation,
							"singleton services joined"
						);
						state.services = ServicesState::SafeToRelease(tenure);
						release(state, out);
					}
				}
			}
			RuntimeEvent::Input(Event::EntropyResponse {
				op,
				attempt_id,
				retry_delay_ms,
			}) => {
				if let ControlState::SamplingJitter { call } = state.control
					&& call.op == op
				{
					cancel_timers(state, out);
					if attempt_id == [0; 16]
						|| !(state.config.retry_min_ms..=state.config.retry_max_ms)
							.contains(&retry_delay_ms)
					{
						suspend(state, out);
						tracing::error!(attempt_id = ?attempt_id, retry_delay_ms, "singleton coordinator received invalid retry entropy; exiting process");
						out.exit();
					} else if can_work(state) {
						state.attempt_id = attempt_id;
						let check_at_boot_ms =
							next_check_boot_ms(state).saturating_add(retry_delay_ms);
						state.control = ControlState::Waiting {
							jitter_ms: retry_delay_ms,
							check_at_boot_ms,
						};
						out.arm_timer_at(Timer::Check, check_at_boot_ms);
					} else {
						suspend(state, out);
					}
				}
			}
			RuntimeEvent::Input(Event::ReadLeaseResponse { op, result }) => {
				if state.control.reading() && pending_matches(state, op) {
					let result = observe_manifest(state, result);
					if matches!(state.control, ControlState::Publishing(_)) {
						publication::read_response(state, op, result, out);
					} else {
						cancel_timers(state, out);
						handle_read(state, result, out);
					}
				}
			}
			RuntimeEvent::Input(Event::InspectHeartbeatResponse { op, result }) => {
				if let ControlState::ReadingOwnerHeartbeat {
					call,
					lease: observed,
					read_started_store_lower_unix_ms,
				} = state.control.clone()
					&& call.op == op
				{
					suspend(state, out);
					let expired = observe_owner(
						state,
						&observed.tenure.owner,
						&result,
						read_started_store_lower_unix_ms,
					);
					if expired {
						claim(state, Some(observed), out);
					} else {
						schedule(state, out);
					}
				}
			}
			RuntimeEvent::Input(Event::AcquireLeaseResponse { op }) => {
				if matches!(state.control.mutation(), Some(Mutation::Acquire(_))) {
					publication::write_response(state, op, out);
				}
			}
			RuntimeEvent::Input(Event::ReleaseLeaseResponse { op }) => {
				if matches!(state.control.mutation(), Some(Mutation::Release { .. })) {
					publication::write_response(state, op, out);
				}
			}
			RuntimeEvent::Input(Event::StepDownResponse { op }) => {
				if matches!(state.control.mutation(), Some(Mutation::StepDown { .. })) {
					publication::write_response(state, op, out);
				}
			}
			RuntimeEvent::Input(Event::WithdrawStepDownResponse { op }) => {
				if matches!(
					state.control.mutation(),
					Some(Mutation::WithdrawStepDown { .. })
				) {
					publication::write_response(state, op, out);
				}
			}
			RuntimeEvent::Timer(Timer::Publication(timer)) => {
				publication::event(state, RuntimeEvent::Timer(timer), out);
			}
			RuntimeEvent::Timer(Timer::Check) => {
				if matches!(&state.control, ControlState::Waiting { .. }) {
					check(state, out);
				}
			}
			RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op))) => {
				if matches!(state.control, ControlState::SamplingJitter { call, .. } if call.op == op)
				{
					schedule(state, out);
				}
			}
			RuntimeEvent::Timer(Timer::StepDown) => {
				if matches!(state.services, ServicesState::SteppingDown { .. }) {
					stop_scope(state, "step-down deadline", out);
				}
			}
			RuntimeEvent::Timer(Timer::Stop) => {
				if matches!(state.services, ServicesState::Stopping { .. }) {
					tracing::error!(
						stop_timeout_ms = state.config.stop_timeout_ms,
						"singleton services did not stop before the stop deadline; exiting process"
					);
					out.exit();
				}
			}
			RuntimeEvent::Timer(Timer::WorkerLease) => {}
		}
	}

	fn snapshot(state: &State) -> snapshot::Snapshot {
		snapshot::Snapshot::from_state(state)
	}

	fn debug_validate(state: &State) -> anyhow::Result<()> {
		state.config.validate()?;
		if let Some(call) = state.control.pending() {
			anyhow::ensure!(
				call.op.index > 0 && call.op.index <= state.sequence.last_index(call.op.slot),
				"pending operation exceeds allocator"
			);
		}
		anyhow::ensure!(
			!state.lease.fenced() || serving_tenure(state).is_none(),
			"fenced worker runs singleton scope"
		);
		anyhow::ensure!(
			!matches!(
				state.control.mutation(),
				Some(Mutation::StepDown { .. } | Mutation::WithdrawStepDown { .. })
			) || matches!(state.services, ServicesState::SteppingDown { .. }),
			"step-down publication outside a step-down"
		);
		if let Some(tenure) = serving_tenure(state) {
			anyhow::ensure!(
				tenure.owner == state.identity
					&& tenure.coordinator_generation > 0
					&& state.lease.live(),
				"running scope lacks valid local tenure"
			);
		}
		Ok(())
	}
}

/// Reads time out as unknown observations; entropy retains its separate retry policy.
fn timeout_response(state: &State, op: OpId) -> Option<Event> {
	match &state.control {
		ControlState::ReadingManifest { call } if call.op == op => Some(Event::ReadLeaseResponse {
			op,
			result: ManifestRead::Read(LeaseRead::Unknown),
		}),
		ControlState::ReadingOwnerHeartbeat { call, .. } if call.op == op => {
			Some(Event::InspectHeartbeatResponse {
				op,
				result: rivet_topology::heartbeat::Read::Unknown,
			})
		}
		ControlState::Idle
		| ControlState::Waiting { .. }
		| ControlState::Publishing(_)
		| ControlState::SamplingJitter { .. }
		| ControlState::ReadingManifest { .. }
		| ControlState::ReadingOwnerHeartbeat { .. } => None,
	}
}

fn can_work(state: &State) -> bool {
	!state.lease.fenced()
		&& state.lease.live()
		&& !matches!(state.services, ServicesState::Stopping { .. })
		&& ((!state.shutdown && state.eligible)
			|| state.control.mutation().is_some()
			|| matches!(
				state.services,
				ServicesState::Running(_)
					| ServicesState::SteppingDown { .. }
					| ServicesState::SafeToRelease(_)
			))
}

/// The tenure whose services currently act, running or stepping down.
fn serving_tenure(state: &State) -> Option<&CoordinatorTenure> {
	match &state.services {
		ServicesState::Running(tenure) | ServicesState::SteppingDown { tenure, .. } => Some(tenure),
		ServicesState::Absent
		| ServicesState::Stopping { .. }
		| ServicesState::SafeToRelease(_) => None,
	}
}

/// Whether this owner should be handing its tenure over: it is shutting down, no longer
/// eligible, or an operator asked for it on the manifest.
fn wants_step_down(state: &State) -> bool {
	state.shutdown || !state.eligible || release_requested(state)
}

/// Whether the latest manifest read carries an operator's release request for the tenure this
/// candidate serves. The request stays on the manifest through the step-down announcement and
/// is cleared by the release, so the owner sees it on every read until it has handed over.
fn release_requested(state: &State) -> bool {
	let Some(tenure) = serving_tenure(state) else {
		return false;
	};
	known_lease(state).is_some_and(|lease| {
		lease.tenure == *tenure
			&& !lease.released
			&& lease.release_request == ReleaseRequest::Pending
	})
}

/// The instant until which this candidate leaves its own released tenure to other candidates,
/// after an operator stepped it down, or None when it may claim. Bounded, so a cluster with no
/// other candidate gets its coordinator back. A newer tenure ends the yield at once.
fn yield_until_boot_ms(state: &State, observed: &ObservedLease) -> Option<u64> {
	let yielding = state.yielding.as_ref()?;
	if yielding.tenure != observed.tenure || yielding.until_boot_ms <= state.now_boot_ms {
		return None;
	}
	Some(yielding.until_boot_ms)
}

/// Starts or cancels a step-down to match the owner's intent. A pending publication is left to
/// finish; its readback re-evaluates the intent against the manifest it reads.
fn reconcile_intent(state: &mut State, out: &mut Effects) {
	if state.control.mutation().is_some() {
		return;
	}
	match state.services.clone() {
		ServicesState::Running(tenure) if wants_step_down(state) => {
			match known_lease(state).filter(|lease| lease.tenure == tenure && !lease.released) {
				Some(base) if base.step_down_ms == 0 => {
					announce_step_down(state, tenure, base, out)
				}
				// Without a manifest to announce on, hand over the slow way rather than keep serving.
				Some(_) | None => stop_scope(state, "step-down without a manifest base", out),
			}
		}
		ServicesState::SteppingDown { tenure, .. } if !wants_step_down(state) => {
			withdraw_step_down(state, &tenure, out);
		}
		ServicesState::Running(_)
		| ServicesState::SteppingDown { .. }
		| ServicesState::Absent
		| ServicesState::Stopping { .. }
		| ServicesState::SafeToRelease(_) => {}
	}
}

/// A commit ID no earlier write by this candidate used: the attempt prefix and a counter.
fn next_commit_id(state: &mut State) -> [u8; 16] {
	state.commits = state.commits.saturating_add(1);
	let mut commit_id = state.attempt_id;
	commit_id[8..].copy_from_slice(&state.commits.to_be_bytes());
	commit_id
}

/// Arms the revocation deadline, then sends the announcement on `base`. The deadline counts
/// from now, before the first send, which the store's stamp cannot precede beyond its
/// truncation. A repeated announcement keeps the earlier deadline, since the earlier write may
/// still have landed.
fn announce_step_down(
	state: &mut State,
	tenure: CoordinatorTenure,
	base: ObservedLease,
	out: &mut Effects,
) {
	let announced_revoke_at_boot_ms = state
		.now_boot_ms
		.saturating_add(state.config.step_down_ms)
		.saturating_sub(state.config.step_down_margin_ms);
	let revoke_at_boot_ms = match &state.services {
		ServicesState::SteppingDown {
			revoke_at_boot_ms, ..
		} => announced_revoke_at_boot_ms.min(*revoke_at_boot_ms),
		ServicesState::Running(_)
		| ServicesState::Absent
		| ServicesState::Stopping { .. }
		| ServicesState::SafeToRelease(_) => announced_revoke_at_boot_ms,
	};
	if revoke_at_boot_ms <= state.now_boot_ms {
		stop_scope(state, "step-down deadline already reached", out);
		return;
	}
	if matches!(state.services, ServicesState::Running(_)) {
		tracing::info!(
			coordinator_generation = tenure.coordinator_generation,
			revoke_at_boot_ms,
			"singleton tenure stepping down"
		);
	}
	state.services = ServicesState::SteppingDown {
		tenure,
		revoke_at_boot_ms,
	};
	out.arm_timer_at(Timer::StepDown, revoke_at_boot_ms);
	let commit_id = next_commit_id(state);
	publish(
		state,
		Mutation::StepDown {
			observed: base,
			commit_id,
			step_down_ms: state.config.step_down_ms,
		},
		out,
	);
}

/// Conditions a withdrawal on the latest known version of this tenure's manifest. Without one,
/// the next check reads the manifest and retries from there.
fn withdraw_step_down(state: &mut State, tenure: &CoordinatorTenure, out: &mut Effects) {
	let Some(base) = known_lease(state).filter(|lease| lease.tenure == *tenure && !lease.released)
	else {
		return;
	};
	let commit_id = next_commit_id(state);
	publish(
		state,
		Mutation::WithdrawStepDown {
			observed: base,
			commit_id,
		},
		out,
	);
}

/// The withdrawal is on the manifest: no announcement conditioned on an older version can land
/// any more, and the tenure serves on without a deadline.
pub(super) fn leave_step_down(state: &mut State, tenure: CoordinatorTenure, out: &mut Effects) {
	tracing::info!(
		coordinator_generation = tenure.coordinator_generation,
		"singleton tenure step-down withdrawn"
	);
	out.cancel_timer(Timer::StepDown);
	state.services = ServicesState::Running(tenure);
}

/// The scope has joined: read the manifest at once, without jitter, and release the version
/// it finds, so a candidate can claim before any announced deadline. A suspended publication
/// resumes first.
fn release(state: &mut State, out: &mut Effects) {
	if matches!(state.control, ControlState::Publishing(_)) {
		schedule(state, out);
	} else {
		read(state, out);
	}
}

fn cancel_timers(state: &State, out: &mut Effects) {
	if let Some(call) = state.control.pending() {
		call.accept(call.op, Timer::Io, out);
	}
	out.cancel_timer(Timer::Check);
}

fn pending_matches(state: &State, op: OpId) -> bool {
	state.control.pending().is_some_and(|call| call.op == op)
}

fn suspend(state: &mut State, out: &mut Effects) {
	cancel_timers(state, out);
	if let ControlState::Publishing(progress) = &mut state.control {
		let mut child = rivet_state_machine::Effects::new(out.clocks());
		progress.suspend(&mut child);
		publication::effects(child, None, out);
	} else {
		state.control = ControlState::Idle;
	}
}

fn begin(state: &mut State, out: &mut Effects) -> IoCall<OpId> {
	cancel_timers(state, out);
	IoCall::issue(
		&mut state.sequence,
		Slot,
		state.config.io_timeout_ms,
		Timer::Io,
		out,
	)
}

/// Version of the retained manifest projection, the If-None-Match base for the next read.
fn known_version(state: &State) -> Option<String> {
	state
		.manifest
		.as_ref()
		.and_then(|manifest| manifest.lease.as_ref())
		.map(|lease| lease.version.clone())
}

/// Resolves a conditional read against the retained projection and records a definite read.
fn observe_manifest(state: &mut State, result: ManifestRead) -> LeaseRead {
	let result = match result {
		ManifestRead::Unchanged => match known_lease(state) {
			Some(lease) => LeaseRead::Found(lease),
			None => {
				tracing::warn!(
					"lease store answered unchanged to an unconditional read; treating it as unknown"
				);
				LeaseRead::Unknown
			}
		},
		ManifestRead::Read(read) => read,
	};
	let lease = match &result {
		LeaseRead::Found(lease) => Some(lease.clone()),
		LeaseRead::Missing => None,
		LeaseRead::Unknown => return result,
	};
	state.manifest = Some(ManifestView {
		lease,
		read_boot_ms: state.now_boot_ms,
	});
	result
}

fn known_lease(state: &State) -> Option<ObservedLease> {
	state
		.manifest
		.as_ref()
		.and_then(|manifest| manifest.lease.clone())
}

/// The owner this candidate is waiting on: a live manifest tenure while eligible to succeed it.
fn watched_owner(state: &State) -> Option<ObservedLease> {
	if state.services != ServicesState::Absent || state.shutdown || !state.eligible {
		return None;
	}
	known_lease(state).filter(|lease| !lease.released)
}

/// Updates heartbeat evidence for the manifest owner and reports whether it has expired.
fn observe_owner(
	state: &mut State,
	owner: &rivet_topology::worker::WorkerIdentity,
	result: &rivet_topology::heartbeat::Read,
	read_started_store_lower_unix_ms: Option<i64>,
) -> bool {
	let previous = state.owner_heartbeat.take();
	let mut missing = match &previous {
		Some(OwnerHeartbeat::Missing(missing)) => Some(missing.clone()),
		Some(OwnerHeartbeat::Live { .. } | OwnerHeartbeat::Unknown { .. }) | None => None,
	};
	let expired = rivet_topology::ownership::observe_expiration(
		owner,
		rivet_topology::worker::WorkerRole::Storage,
		result,
		state.now_boot_ms,
		state.config.takeover_wait_ms,
		read_started_store_lower_unix_ms,
		&mut missing,
	);
	state.owner_heartbeat = match result {
		rivet_topology::heartbeat::Read::Found(entry) => Some(OwnerHeartbeat::Live {
			identity: owner.clone(),
			modified_unix_ms: entry.modified_unix_ms,
			draining: entry.heartbeat.state == rivet_topology::heartbeat::HeartbeatState::Draining,
		}),
		rivet_topology::heartbeat::Read::Missing => missing.map(OwnerHeartbeat::Missing),
		// An unknown read adds nothing; earlier evidence stands and the retry follows its deadline,
		// which is now or past unless a newer store clock anchor moved it slightly ahead.
		rivet_topology::heartbeat::Read::Unknown => previous.or(Some(OwnerHeartbeat::Unknown {
			identity: owner.clone(),
		})),
	};
	expired
}

/// First instant at which inspecting `owner` could find it expired, from retained evidence.
/// None when there is no evidence for this owner or time alone cannot expire it (no store clock).
fn heartbeat_due_boot_ms(
	state: &State,
	owner: &rivet_topology::worker::WorkerIdentity,
) -> Option<u64> {
	let wait_ms = state.config.takeover_wait_ms;
	match &state.owner_heartbeat {
		Some(OwnerHeartbeat::Missing(missing)) if missing.identity == *owner => {
			Some(missing.first_missing_boot_ms.saturating_add(wait_ms))
		}
		Some(OwnerHeartbeat::Unknown { identity }) if identity == owner => Some(state.now_boot_ms),
		Some(OwnerHeartbeat::Live {
			identity,
			modified_unix_ms,
			..
		}) if identity == owner => {
			// Mirrors observe_expiration: expired once the store-time lower bound at read start
			// is at least `modified + wait`. The lower bound advances one for one with boot time.
			let clock = state.store_clock.as_ref()?;
			let until_expiry_ms = i128::from(*modified_unix_ms) + i128::from(wait_ms)
				- i128::from(clock.lower_unix_ms);
			let until_expiry_ms = until_expiry_ms.clamp(0, i128::from(u64::MAX)) as u64;
			Some(clock.at_boot_ms.saturating_add(until_expiry_ms))
		}
		Some(
			OwnerHeartbeat::Missing(_)
			| OwnerHeartbeat::Live { .. }
			| OwnerHeartbeat::Unknown { .. },
		)
		| None => None,
	}
}

/// Whether the watched owner is handing over. With an announced step-down, that is from
/// shortly before the owner revokes its services, when its release can appear, until the
/// deadline; before then the deadline is known and nothing is worth polling for. Without
/// one, it is draining or absent from the storage listing. Its release is the change a
/// candidate waits for, and nothing else announces it.
fn handover_pending(state: &State) -> bool {
	watched_owner(state).is_some_and(|owner| {
		// Without a store clock the window is unknown, so the hints below still apply.
		if owner.step_down_at_unix_ms().is_some()
			&& let Some(due) = step_down_due_boot_ms(state)
		{
			return due <= state.now_boot_ms;
		}
		let unlisted = state.owner_unlisted.as_ref() == Some(&owner.tenure.owner);
		let draining = matches!(
			&state.owner_heartbeat,
			Some(OwnerHeartbeat::Live { identity, draining: true, .. })
				if *identity == owner.tenure.owner
		);
		unlisted || draining
	})
}

/// The boot instant at which this candidate's store-time lower bound reaches the start of the
/// watched owner's release window: one margin and one jitter bound before its announced
/// deadline, since the owner revokes about one margin before it and releases once its scope
/// joins. None without an announcement or a store clock.
fn step_down_due_boot_ms(state: &State) -> Option<u64> {
	let deadline_unix_ms = watched_owner(state)?.step_down_at_unix_ms()?;
	let lead_ms = state
		.config
		.step_down_margin_ms
		.saturating_add(state.config.retry_max_ms);
	let window_unix_ms = i128::from(deadline_unix_ms) - i128::from(lead_ms);
	let clock = state.store_clock.as_ref()?;
	let until_window_ms = window_unix_ms - i128::from(clock.lower_unix_ms);
	let until_window_ms = until_window_ms.clamp(0, i128::from(u64::MAX)) as u64;
	Some(clock.at_boot_ms.saturating_add(until_window_ms))
}

/// When the retained manifest must be read again: one interval after the last definite read,
/// at the start of the watched owner's release window, or immediately while a handover is
/// pending.
fn manifest_due_boot_ms(state: &State) -> u64 {
	let now = state.now_boot_ms;
	let Some(manifest) = &state.manifest else {
		return now;
	};
	if handover_pending(state) {
		return now;
	}
	let interval_due = manifest
		.read_boot_ms
		.saturating_add(state.config.manifest_check_interval_ms);
	match step_down_due_boot_ms(state) {
		Some(step_down_due) => interval_due.min(step_down_due),
		None => interval_due,
	}
}

/// Earliest instant the next check could change a decision. The manifest is read on its
/// documented interval because step-down requests and releases by a live worker send no
/// notification; everything else waits for a known deadline or runs now.
fn next_check_boot_ms(state: &State) -> u64 {
	let now = state.now_boot_ms;
	let manifest_due = manifest_due_boot_ms(state);
	let due = match &state.services {
		ServicesState::Running(_) | ServicesState::SteppingDown { .. } => manifest_due,
		ServicesState::SafeToRelease(_) | ServicesState::Stopping { .. } => now,
		ServicesState::Absent => match watched_owner(state) {
			Some(owner) => match heartbeat_due_boot_ms(state, &owner.tenure.owner) {
				Some(heartbeat_due) => manifest_due.min(heartbeat_due),
				None => manifest_due,
			},
			// A released tenure this candidate yields is claimed once the yield ends.
			None => known_lease(state)
				.and_then(|lease| yield_until_boot_ms(state, &lease))
				.unwrap_or(now),
		},
	};
	due.max(now)
}

/// Samples jitter, then arms the check alarm at the next known deadline.
fn schedule(state: &mut State, out: &mut Effects) {
	if matches!(state.control, ControlState::Publishing(_)) {
		read(state, out);
		return;
	}
	if !can_work(state) {
		suspend(state, out);
		return;
	}
	let call = begin(state, out);
	state.control = ControlState::SamplingJitter { call };
	out.execute(Effect::EntropyRequest {
		op: call.op,
		min_delay_ms: state.config.retry_min_ms,
		max_delay_ms: state.config.retry_max_ms,
	});
}

/// Moves the armed check earlier when current state has an earlier deadline. Jitter follows
/// the deadline so candidates do not all hit the store at once. A later deadline keeps the
/// armed check: it fires, finds nothing due, and schedules the later one, which costs one read
/// but cannot be starved by a stream of inputs.
fn rearm_check(state: &mut State, out: &mut Effects) {
	let ControlState::Waiting {
		jitter_ms,
		check_at_boot_ms,
	} = state.control
	else {
		return;
	};
	let earlier = next_check_boot_ms(state).saturating_add(jitter_ms);
	if earlier < check_at_boot_ms {
		state.control = ControlState::Waiting {
			jitter_ms,
			check_at_boot_ms: earlier,
		};
		out.arm_timer_at(Timer::Check, earlier);
	}
}

/// The check alarm fired: inspect the watched owner directly while the manifest is fresh,
/// otherwise read the manifest.
fn check(state: &mut State, out: &mut Effects) {
	if !can_work(state) {
		suspend(state, out);
		return;
	}
	let manifest_fresh = state.now_boot_ms < manifest_due_boot_ms(state);
	match watched_owner(state) {
		Some(owner) if manifest_fresh => inspect(state, owner, out),
		Some(owner) => {
			if handover_pending(state) {
				// Bounded by the owner's release: the manifest is re-read at jitter cadence until then.
				tracing::debug!(
					owner = ?owner.tenure.owner.worker_id,
					"re-reading the singleton manifest while its owner hands over"
				);
			}
			read(state, out);
		}
		None => read(state, out),
	}
}

fn read(state: &mut State, out: &mut Effects) {
	if !can_work(state) {
		suspend(state, out);
		return;
	}
	if let ControlState::Publishing(progress) = &state.control {
		if progress.suspended() {
			let op = state.sequence.next(Slot);
			let known_version = known_version(state);
			if let ControlState::Publishing(progress) = &mut state.control {
				let mut child = rivet_state_machine::Effects::new(out.clocks());
				progress.resume(state.now_boot_ms, op, &mut child);
				publication::effects(child, known_version, out);
			}
		}
		return;
	}
	let known_version = known_version(state);
	let call = begin(state, out);
	state.control = ControlState::ReadingManifest { call };
	out.execute(Effect::ReadLeaseRequest {
		op: call.op,
		known_version,
	});
}

/// Whether the owner's announced step-down deadline has passed in store time, judged by this
/// candidate's lower bound, which never runs ahead of the store.
fn step_down_reached(state: &State, observed: &ObservedLease) -> bool {
	let Some(deadline_unix_ms) = observed.step_down_at_unix_ms() else {
		return false;
	};
	state
		.store_clock
		.as_ref()
		.is_some_and(|clock| clock.lower_bound(state.now_boot_ms) >= deadline_unix_ms)
}

/// Prove the manifest owner's heartbeat expired before preparing a claim against `observed`.
fn inspect(state: &mut State, observed: ObservedLease, out: &mut Effects) {
	let owner = observed.tenure.owner.clone();
	let read_started_store_lower_unix_ms = state
		.store_clock
		.as_ref()
		.map(|clock| clock.lower_bound(state.now_boot_ms));
	let call = begin(state, out);
	state.control = ControlState::ReadingOwnerHeartbeat {
		call,
		lease: observed,
		read_started_store_lower_unix_ms,
	};
	out.execute(Effect::InspectHeartbeatRequest { op: call.op, owner });
}

fn publish(state: &mut State, mutation: Mutation, out: &mut Effects) {
	cancel_timers(state, out);
	let permitted = can_work(state);
	let op = state.sequence.next(Slot);
	let mut child = rivet_state_machine::Effects::new(out.clocks());
	let mut progress = cw::ConditionalWrite::start(
		mutation,
		cw::Config {
			io_timeout_ms: state.config.io_timeout_ms,
			retry_delay_ms: state.config.retry_min_ms,
		},
		state.now_boot_ms,
		op,
		&mut child,
	);
	if permitted {
		publication::effects(child, known_version(state), out);
	} else {
		progress.suspend(&mut child);
	}
	state.control = ControlState::Publishing(progress);
}

fn fence(state: &mut State, out: &mut Effects) {
	state.lease.observe(None, out);
	suspend(state, out);
	stop_scope(state, "worker lease fenced", out);
	if matches!(state.services, ServicesState::SafeToRelease(_)) {
		state.services = ServicesState::Absent;
	}
}

/// Revokes the serving tenure now; `reason` is a bounded label for the log. The stop alarm is a
/// backstop counted from now; the executor bounds a stepping-down scope from its scheduled
/// revocation on its own clock, however late this transition runs. A tenure revoked on an
/// operator's request is left to other candidates for one step-down period, so the holder the
/// operator removed does not win the election it just vacated.
fn stop_scope(state: &mut State, reason: &'static str, out: &mut Effects) {
	let Some(tenure) = serving_tenure(state).cloned() else {
		return;
	};
	if release_requested(state) {
		state.yielding = Some(Yielding {
			tenure: tenure.clone(),
			until_boot_ms: state.now_boot_ms.saturating_add(state.config.step_down_ms),
		});
	}
	tracing::info!(
		coordinator_generation = tenure.coordinator_generation,
		reason,
		"singleton tenure revoked"
	);
	suspend(state, out);
	// A step-down or its withdrawal no longer matters once the scope stops: release, or the
	// announced deadline, hands the tenure over. Abandon it rather than resume it later.
	if matches!(
		state.control.mutation(),
		Some(Mutation::StepDown { .. } | Mutation::WithdrawStepDown { .. })
	) {
		state.control = ControlState::Idle;
	}
	out.cancel_timer(Timer::StepDown);
	let deadline_boot_ms = state
		.now_boot_ms
		.saturating_add(state.config.stop_timeout_ms);
	state.services = ServicesState::Stopping {
		tenure: tenure.clone(),
	};
	out.execute(Effect::StopScopeRequest { tenure });
	out.arm_timer_at(Timer::Stop, deadline_boot_ms);
}

fn claim(state: &mut State, expected: Option<ObservedLease>, out: &mut Effects) {
	if !state.eligible || state.shutdown || !can_work(state) {
		return;
	}
	let previous = expected
		.as_ref()
		.map_or(0, |lease| lease.tenure.coordinator_generation);
	let Some(coordinator_generation) = previous.checked_add(1) else {
		fence(state, out);
		return;
	};
	let mutation = Mutation::Acquire(ClaimAttempt {
		expected,
		tenure: CoordinatorTenure {
			owner: state.identity.clone(),
			coordinator_generation,
			claim_id: state.attempt_id,
		},
	});
	publish(state, mutation, out);
}

fn handle_read(state: &mut State, result: LeaseRead, out: &mut Effects) {
	if !can_work(state) {
		suspend(state, out);
		return;
	}
	if result == LeaseRead::Unknown {
		schedule(state, out);
		return;
	}
	let observed = match result {
		LeaseRead::Found(observed) => Some(observed),
		LeaseRead::Missing => None,
		LeaseRead::Unknown => unreachable!(),
	};
	match state.services.clone() {
		ServicesState::Running(tenure) | ServicesState::SteppingDown { tenure, .. } => {
			let Some(found) = observed.filter(|found| found.tenure == tenure && !found.released)
			else {
				stop_scope(state, "manifest no longer names this tenure", out);
				return;
			};
			let announced = found.step_down_ms > 0;
			let stepping_down = matches!(state.services, ServicesState::SteppingDown { .. });
			match (stepping_down, wants_step_down(state)) {
				// An announcement without a local deadline cannot be trusted to revoke first.
				(false, _) if announced => {
					stop_scope(state, "step-down announced without a local deadline", out)
				}
				(false, true) => announce_step_down(state, tenure, found, out),
				(false, false) => schedule(state, out),
				(true, true) if announced => schedule(state, out),
				// The announcement has not landed, or a later write replaced it; announce again
				// on this version, keeping the earlier deadline.
				(true, true) => announce_step_down(state, tenure, found, out),
				(true, false) => withdraw_step_down(state, &tenure, out),
			}
		}
		ServicesState::SafeToRelease(tenure) => {
			if let Some(observed) =
				observed.filter(|found| found.tenure == tenure && !found.released)
			{
				let commit_id = next_commit_id(state);
				publish(
					state,
					Mutation::Release {
						observed,
						commit_id,
					},
					out,
				);
			} else {
				state.services = ServicesState::Absent;
				schedule(state, out);
			}
		}
		ServicesState::Absent => {
			if state.shutdown || !state.eligible {
				return;
			}
			if let (Some(yielding), Some(observed)) = (&state.yielding, &observed)
				&& yielding.tenure != observed.tenure
			{
				state.yielding = None;
			}
			match observed {
				None => claim(state, None, out),
				Some(observed) if observed.released => {
					if yield_until_boot_ms(state, &observed).is_some() {
						schedule(state, out);
					} else {
						claim(state, Some(observed), out);
					}
				}
				Some(observed) if step_down_reached(state, &observed) => {
					claim(state, Some(observed), out)
				}
				Some(observed) => {
					// Evidence that the owner cannot have expired yet makes an inspection now a
					// wasted read; wait for its deadline instead.
					let pending_expiry = heartbeat_due_boot_ms(state, &observed.tenure.owner)
						.is_some_and(|due| due > state.now_boot_ms);
					if pending_expiry {
						schedule(state, out);
					} else {
						inspect(state, observed, out);
					}
				}
			}
		}
		ServicesState::Stopping { .. } => {}
	}
}
