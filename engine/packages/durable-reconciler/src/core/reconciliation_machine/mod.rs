pub mod effect;
pub mod event;
pub(super) mod executor;
pub mod snapshot;
pub mod state;
pub mod types;

use super::Reconciler;
use effect::Effect;
use event::Body;
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, RankMap};
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use snapshot::Snapshot;
use state::ReconcilerPhase;
pub use state::ReconcilerState;
use std::collections::VecDeque;
use std::marker::PhantomData;
use types::{Slot, Timer};

pub struct Reconciliation<R>(PhantomData<fn() -> R>);

impl<R: Reconciler> EphemeralStateMachine for Reconciliation<R> {
	type Slot = Slot;

	type State = ReconcilerState<R::Effect>;

	type Event = Body<R::State>;

	type Effect = Effect<R::Effect>;

	type Timer = Timer;

	type Snapshot = Snapshot;

	const DEBUG_NAME: &'static str = R::NAME;

	fn timer_name(timer: &Timer) -> &'static str {
		match timer {
			Timer::Poll => "poll",
			Timer::EffectTimeout(..) => "effect_timeout",
		}
	}

	fn start(
		s: &mut Self::State,
		_: Clocks,
		_: Entropy,
		effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		let op = s.allocate_operation();
		s.phase = ReconcilerPhase::Loading { op };
		// This load observes every write that happened before it.
		s.wake_pending = false;
		{
			effects.execute(Effect::Load(op));
		}
	}
	fn on_event(
		s: &mut Self::State,
		e: EphemeralStateMachineEvent<Self>,
		effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		match e.body {
			RuntimeEvent::Input(Body::Loaded { op, result }) if matches!(&s.phase, ReconcilerPhase::Loading { op: pending } if *pending == op) =>
			{
				effects.execute(Effect::LoadedMetric {
					success: result.is_ok(),
				});
				let remaining =
					result.map_or_else(|_| VecDeque::new(), |state| R::reconcile(&state).into());
				Self::next(s, remaining, effects);
			}
			RuntimeEvent::Input(Body::Completed { op, .. }) if matches!(&s.phase, ReconcilerPhase::Executing { op: pending, .. } if *pending == op) =>
			{
				let ReconcilerPhase::Executing { remaining, .. } =
					std::mem::replace(&mut s.phase, ReconcilerPhase::NotStarted)
				else {
					unreachable!()
				};
				effects.cancel_timer(Timer::EffectTimeout(op));
				Self::next(s, remaining, effects);
			}
			RuntimeEvent::Timer(Timer::EffectTimeout(op)) if matches!(&s.phase, ReconcilerPhase::Executing { op: pending, .. } if *pending == op) =>
			{
				let ReconcilerPhase::Executing { remaining, .. } =
					std::mem::replace(&mut s.phase, ReconcilerPhase::NotStarted)
				else {
					unreachable!()
				};
				effects.execute(Effect::CancelWork);
				Self::next(s, remaining, effects);
			}
			RuntimeEvent::Timer(Timer::Poll)
				if matches!(&s.phase, ReconcilerPhase::WaitingToPoll) =>
			{
				Self::start(s, e.now, e.entropy, effects)
			}
			RuntimeEvent::Input(Body::Wake) => match &s.phase {
				ReconcilerPhase::WaitingToPoll => {
					effects.cancel_timer(Timer::Poll);
					Self::start(s, e.now, e.entropy, effects)
				}
				ReconcilerPhase::Loading { .. } | ReconcilerPhase::Executing { .. } => {
					s.wake_pending = true;
				}
				ReconcilerPhase::NotStarted | ReconcilerPhase::Stopped => {}
			},
			_ => {}
		}
	}
	fn snapshot(s: &Self::State) -> Snapshot {
		Snapshot {
			done: matches!(s.phase, ReconcilerPhase::Stopped),
		}
	}
	fn debug_validate(s: &Self::State) -> anyhow::Result<()> {
		if let Some(op) = s.pending() {
			anyhow::ensure!(
				op.index == s.next_operation_index,
				"pending correlation differs from allocation history"
			);
		}
		Ok(())
	}
	fn normalize(s: &Self::State, _: Clocks, _: &RankMap<Slot>) -> Self::State {
		let mut s = s.clone();
		s.next_operation_index = 0;
		match &mut s.phase {
			ReconcilerPhase::Loading { op } | ReconcilerPhase::Executing { op, .. } => op.index = 0,
			_ => {}
		}
		s
	}
}

impl<R: Reconciler> Reconciliation<R> {
	fn stop(
		s: &mut ReconcilerState<R::Effect>,
		effects: &mut rivet_state_machine::Effects<Effect<R::Effect>, Timer>,
	) {
		effects.execute(Effect::CancelWork);
		effects.cancel_timer(Timer::Poll);
		if let ReconcilerPhase::Executing { op, .. } = s.phase {
			effects.cancel_timer(Timer::EffectTimeout(op));
		}
		s.phase = ReconcilerPhase::Stopped;
	}
	fn next(
		s: &mut ReconcilerState<R::Effect>,
		mut remaining: VecDeque<R::Effect>,
		effects: &mut rivet_state_machine::Effects<Effect<R::Effect>, Timer>,
	) {
		if let Some(effect) = remaining.pop_front() {
			let Some(deadline_boot_ms) = effects.now_boot_ms().checked_add(s.timeout_ms) else {
				return Self::stop(s, effects);
			};
			let op = s.allocate_operation();
			s.phase = ReconcilerPhase::Executing { op, remaining };
			{
				effects.arm_timer_at(Timer::EffectTimeout(op), deadline_boot_ms);
				effects.execute(Effect::Execute(op, effect));
			}
		} else if s.wake_pending {
			let op = s.allocate_operation();
			s.phase = ReconcilerPhase::Loading { op };
			s.wake_pending = false;
			effects.execute(Effect::Load(op));
		} else {
			let Some(poll_at_boot_ms) = effects.now_boot_ms().checked_add(s.poll_ms) else {
				return Self::stop(s, effects);
			};
			s.phase = ReconcilerPhase::WaitingToPoll;
			{
				effects.arm_timer_at(Timer::Poll, poll_at_boot_ms);
			}
		}
	}
}


