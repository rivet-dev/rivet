//! Per-process config coordination, independent of Topology, Depot and singleton leadership.
use rivet_state_machine_util::IoTimer;
pub mod effect;
pub mod event;
pub mod executor;
pub mod snapshot;
pub mod state;
pub mod types;

use effect::Effect;
use event::Event;
use rivet_env::Clocks;
use rivet_object_store_util::conditional_write as cw;
use rivet_state_machine::Entropy;
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEffect as Fx, RuntimeEvent,
};
use state::{Phase, State};
use types::*;

/// Pure transition core; bootstrap and one-shot updates run the same protocol as the Always service.
pub struct ClusterConfigStateMachine;

impl rivet_state_machine::StateMachineState for State {}
impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::RefreshRequest => "refresh_request",
			Event::StopRequest => "stop_request",
			Event::ReadConfigResponse { .. } => "read_config_response",
			Event::PrepareConfigResponse { .. } => "prepare_config_response",
			Event::WriteConfigResponse { .. } => "write_config_response",
			Event::ApplyConfigResponse { .. } => "apply_config_response",
		}
	}
}
impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::ReadConfigRequest { .. } => "read_config_request",
			Effect::PrepareConfigRequest { .. } => "prepare_config_request",
			Effect::WriteConfigRequest { .. } => "write_config_request",
			Effect::ApplyConfigRequest { .. } => "apply_config_request",
			Effect::CancelOperation { .. } => "cancel_operation",
		}
	}
}
impl rivet_state_machine::Snapshot for snapshot::Snapshot {}
/// Ordered runtime effects, with timer changes preceding new executor dispatch.
pub type Effects = rivet_state_machine_util::Effects<Effect, Timer>;

impl EphemeralStateMachine for ClusterConfigStateMachine {
	type Slot = Slot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = snapshot::Snapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "cluster_config";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			Timer::Deadline => "deadline",
			Timer::Refresh => "refresh",
			Timer::Io(..) => "io",
			Timer::Publication(..) => "publication",
		}
	}

	fn start(state: &mut State, now: Clocks, _: Entropy, out: &mut Effects) {
		if state.purpose != Purpose::Follow {
			let deadline = now
				.boot_ms
				.saturating_add(state.settings.operation_timeout_ms);
			state.operation_deadline_boot_ms = Some(deadline);
			out.arm_timer_at(Timer::Deadline, deadline);
		}
		read(state, now.boot_ms, out);
	}
	fn on_event(state: &mut State, event: EphemeralStateMachineEvent<Self>, out: &mut Effects) {
		if matches!(state.phase, Phase::Finished { .. }) {
			return;
		}
		let now = event.now.boot_ms;
		// Overall operation lifetime is separate from I/O alarm ordering. Earlier overdue retry
		// alarms must not dispatch a PUT after the bounded bootstrap/CLI operation has expired.
		if state
			.operation_deadline_boot_ms
			.is_some_and(|deadline| now >= deadline)
		{
			finish(state, Err("config operation timed out; a sent write may still commit; inspect configuration before resubmitting".into()), out);
			return;
		}
		match event.body {
			RuntimeEvent::Input(Event::StopRequest) => finish(
				state,
				Err("config state machine stopped; a sent write may still commit".into()),
				out,
			),
			RuntimeEvent::Input(Event::RefreshRequest) if state.purpose == Purpose::Follow => {
				match state.phase {
					Phase::Idle => read(state, now, out),
					Phase::Waiting => {} // Repeated UPS hints must not defeat failure backoff.
					_ => state.refresh_requested = true,
				}
			}
			RuntimeEvent::Timer(Timer::Refresh)
				if matches!(state.phase, Phase::Idle | Phase::Waiting) =>
			{
				read(state, now, out)
			}
			RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op)))
				if state.phase.pending().is_some_and(|call| call.op == op)
					&& !matches!(state.phase, Phase::Publishing(_)) =>
			{
				// Publication is already confirmed before Applying. Restarting an Update here
				// would merge the patch again and could overwrite a newer operator decision.
				if matches!(state.phase, Phase::Applying { .. })
					&& matches!(state.purpose, Purpose::Update { .. })
				{
					finish(
						state,
						Err("config was stored, but local application was not confirmed; inspect current settings before resubmitting".into()),
						out,
					);
					return;
				}
				cancel_call(state, out);
				fail(
					state,
					"config executor exchange timed out".into(),
					true,
					out,
				);
			}
			RuntimeEvent::Timer(Timer::Publication(timer)) => {
				publication(state, RuntimeEvent::Timer(timer), now, out)
			}
			RuntimeEvent::Input(Event::ReadConfigResponse { op, result }) => {
				if let Phase::Publishing(progress) = &state.phase {
					let request = progress.request();
					let result = match result {
						ReadResult::Unknown(_) => cw::Readback::Unknown,
						ReadResult::Found(ref found) if found.bytes == request.bytes => {
							cw::Readback::Confirmed(result)
						}
						ReadResult::Found(ref found)
							if Some(&found.etag) == request.expected_etag.as_ref() =>
						{
							cw::Readback::Unchanged
						}
						ReadResult::Missing if request.expected_etag.is_none() => {
							cw::Readback::Unchanged
						}
						_ => cw::Readback::Reconcile(result),
					};
					publication(
						state,
						RuntimeEvent::Input(cw::Event::ReadResponse { op, result }),
						now,
						out,
					);
				} else if matches!(state.phase, Phase::Reading {call} if call.op == op) {
					cancel_call(state, out);
					read_result(state, result, now, out);
				}
			}
			RuntimeEvent::Input(Event::PrepareConfigResponse { op, result }) if matches!(state.phase, Phase::Preparing {call} if call.op == op) =>
			{
				cancel_call(state, out);
				match result {
					Ok(request) => {
						let op = state.sequence.next(Slot);
						let mut child = rivet_state_machine::Effects::new(out.clocks());
						let progress = cw::ConditionalWrite::start(
							request,
							cw::Config {
								io_timeout_ms: state.settings.io_timeout_ms,
								retry_delay_ms: state.settings.retry_delay_ms,
							},
							now,
							op,
							&mut child,
						);
						state.phase = Phase::Publishing(progress);
						publication_effects(child, out);
					}
					Err(error) => fail(state, error, false, out),
				}
			}
			RuntimeEvent::Input(Event::WriteConfigResponse { op, result }) => publication(
				state,
				RuntimeEvent::Input(cw::Event::WriteResponse { op, result }),
				now,
				out,
			),
			RuntimeEvent::Input(Event::ApplyConfigResponse { op, result }) if matches!(&state.phase, Phase::Applying {call, ..} if call.op == op) =>
			{
				cancel_call(state, out);
				match result {
					Ok(()) => {
						let Phase::Applying { etag, .. } = &state.phase else {
							unreachable!()
						};
						state.applied_etag = Some(etag.clone());
						complete_cycle(state, now, out);
					}
					Err(error) => fail(state, error, false, out),
				}
			}
			_ => {}
		}
	}
	fn snapshot(state: &State) -> snapshot::Snapshot {
		state.into()
	}
	fn debug_validate(state: &State) -> anyhow::Result<()> {
		state.settings.validate()?;
		anyhow::ensure!(
			state.purpose == Purpose::Follow || !state.refresh_requested,
			"only followers coalesce refresh hints"
		);
		if let Phase::Publishing(progress) = &state.phase {
			anyhow::ensure!(
				matches!(state.purpose, Purpose::Bootstrap | Purpose::Update { .. }),
				"reader cannot publish config"
			);
			anyhow::ensure!(
				progress
					.request()
					.expected_etag
					.as_ref()
					.is_none_or(|etag| !etag.is_empty()),
				"empty config ETag"
			);
		}
		Ok(())
	}
}
fn call(state: &mut State, now: u64, out: &mut Effects) -> IoCall<OpId> {
	let timeout = state.settings.io_timeout_ms.min(
		state
			.operation_deadline_boot_ms
			.unwrap_or(u64::MAX)
			.saturating_sub(now),
	);
	IoCall::issue(&mut state.sequence, Slot, timeout, Timer::Io, out)
}
fn cancel_call(state: &State, out: &mut Effects) {
	if let Some(call) = state.phase.pending() {
		call.accept(call.op, Timer::Io, out);
		out.execute(Effect::CancelOperation { op: call.op });
	}
}
fn read(state: &mut State, now: u64, out: &mut Effects) {
	out.cancel_timer(Timer::Refresh);
	state.refresh_requested = false;
	let call = call(state, now, out);
	state.phase = Phase::Reading { call };
	out.execute(Effect::ReadConfigRequest { op: call.op });
}
fn prepare(state: &mut State, preparation: Preparation, now: u64, out: &mut Effects) {
	let call = call(state, now, out);
	state.phase = Phase::Preparing { call };
	out.execute(Effect::PrepareConfigRequest {
		op: call.op,
		preparation,
	});
}
fn apply(state: &mut State, snapshot: StoredConfig, now: u64, out: &mut Effects) {
	if state.applied_etag.as_ref() == Some(&snapshot.etag) {
		complete_cycle(state, now, out);
		return;
	}
	let call = call(state, now, out);
	state.phase = Phase::Applying {
		call,
		etag: snapshot.etag.clone(),
	};
	out.execute(Effect::ApplyConfigRequest {
		op: call.op,
		snapshot,
	});
}
fn read_result(state: &mut State, result: ReadResult, now: u64, out: &mut Effects) {
	match result {
		ReadResult::Found(snapshot) => {
			if let Purpose::Update { patch } = &state.purpose {
				prepare(
					state,
					Preparation::Update {
						base: snapshot,
						patch: patch.clone(),
					},
					now,
					out,
				)
			} else {
				apply(state, snapshot, now, out)
			}
		}
		ReadResult::Missing if state.purpose == Purpose::Bootstrap => {
			prepare(state, Preparation::Bootstrap, now, out)
		}
		ReadResult::Missing => fail(state, "cluster config is missing".into(), false, out),
		ReadResult::Unknown(error) => fail(state, error, true, out),
		ReadResult::Invalid(error) => fail(state, error, false, out),
	}
}
fn fail(state: &mut State, error: String, transient: bool, out: &mut Effects) {
	state.diagnostic = Some(error.clone());
	if state.purpose == Purpose::Follow || transient {
		state.phase = Phase::Waiting;
		// A malformed snapshot will not heal through rapid retries; retain last-good settings.
		let delay = if transient {
			state.settings.retry_delay_ms
		} else {
			state.settings.refresh_interval_ms
		};
		out.arm_timer_after(Timer::Refresh, delay);
	} else {
		finish(state, Err(error), out)
	}
}
fn complete_cycle(state: &mut State, now: u64, out: &mut Effects) {
	state.diagnostic = None;
	if state.purpose != Purpose::Follow {
		finish(state, Ok(()), out)
	} else if state.refresh_requested {
		read(state, now, out)
	} else {
		state.phase = Phase::Idle;
		out.arm_timer_after(Timer::Refresh, state.settings.refresh_interval_ms);
	}
}
fn finish(state: &mut State, result: Result<(), String>, out: &mut Effects) {
	let unresolved = if let Phase::Publishing(progress) = &mut state.phase {
		let mut child = rivet_state_machine::Effects::new(out.clocks());
		progress.suspend(&mut child);
		publication_effects(child, out);
		Some(progress.request().clone())
	} else {
		cancel_call(state, out);
		None
	};
	out.cancel_timer(Timer::Refresh);
	out.cancel_timer(Timer::Deadline);
	state.phase = Phase::Finished { result, unresolved };
}
fn publication_effects(
	effects: rivet_state_machine::Effects<cw::Effect<PreparedConfig, OpId>, cw::Timer<OpId>>,
	out: &mut Effects,
) {
	// Cancelling publication I/O also cancels its executor task, before the alarm.
	for effect in effects.as_slice().iter().cloned() {
		match effect {
			Fx::Execute(cw::Effect::ReadRequest { op }) => {
				out.execute(Effect::ReadConfigRequest { op })
			}
			Fx::Execute(cw::Effect::WriteRequest { op, request }) => {
				out.execute(Effect::WriteConfigRequest { op, request })
			}
			Fx::ArmTimer { timer, at_boot_ms } => {
				out.arm_timer_at(Timer::Publication(timer), at_boot_ms)
			}
			Fx::CancelTimer(timer) => {
				if let cw::Timer::Io(IoTimer::Io(op)) = timer {
					out.execute(Effect::CancelOperation { op });
				}
				out.cancel_timer(Timer::Publication(timer));
			}
			Fx::Exit => out.exit(),
		}
	}
}

fn publication(
	state: &mut State,
	input: RuntimeEvent<cw::Event<OpId, ReadResult>, cw::Timer<OpId>>,
	now: u64,
	out: &mut Effects,
) {
	let Phase::Publishing(progress) = &mut state.phase else {
		return;
	};
	let mut child = rivet_state_machine::Effects::new(out.clocks());
	let update = progress.on_event(now, input, || state.sequence.next(Slot), &mut child);
	publication_effects(child, out);
	match update {
        Some(cw::Outcome::Confirmed(cw::Confirmation::Write {etag})) => {
            let snapshot = StoredConfig {bytes: progress.request().bytes.clone(), etag};
            apply(state, snapshot, now, out);
        },
        Some(cw::Outcome::Confirmed(cw::Confirmation::Read(ReadResult::Found(snapshot)))) => apply(state, snapshot, now, out),
        Some(cw::Outcome::Reconcile(ReadResult::Found(snapshot))) if state.purpose == Purpose::Bootstrap => apply(state, snapshot, now, out),
        Some(cw::Outcome::Reconcile(_)) => finish(state, Err("cluster config changed or publication could not be confirmed; inspect current settings before resubmitting; the original update was not rebased".into()), out),
        Some(_) => unreachable!("only matching stored bytes confirm a config mutation"),
        None => {},
    }
}
