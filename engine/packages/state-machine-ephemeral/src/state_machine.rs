use rivet_env::Clocks;
use rivet_state_machine::{
	Effect, Entropy, Event, EventBody, RankMap, Slot, Snapshot, StateMachine, StateMachineState,
};
pub use rivet_state_machine::{Effects, RuntimeEffect};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fmt::Debug, hash::Hash};

/// Use when a state machine does not schedule timers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoTimer {}

/// Inputs delivered by the standard runner. Callers and executors send only the
/// `Input` payload; the runner supplies timer events.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEvent<I, T> {
	Input(I),
	/// Due armed alarms run before inbox inputs at the same delivery clock. Never
	/// delivered early; cancellation/replacement takes effect before the next alarm.
	/// Keep domain phase/operation checks, but do not duplicate I/O expiry checks.
	Timer(T),
}

impl<I: EventBody, T> EventBody for RuntimeEvent<I, T>
where
	T: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send,
{
	/// Timer events are named `timer` here because the timer type has no name of its own; the
	/// runner labels them with [`EphemeralStateMachine::timer_name`] instead.
	fn name(&self) -> &'static str {
		match self {
			RuntimeEvent::Input(input) => input.name(),
			RuntimeEvent::Timer(_) => "timer",
		}
	}
}

/// Defines pure transitions for the standard runner. `Event` and `Effect` are
/// application types; the runner owns timers and process exit. Use
/// [`StateMachineAdapter`] to expose these transitions through the pure
/// [`StateMachine`] contract for simulation or a custom driver.
///
/// ```text
///            Caller constructs executor
///                   |
///                   | passes executor
///                   v
///            Runner::spawn(env, state, executor)
///                   |
///                   v
///            Core startup produces effects
///                   |
///       +--> Publish snapshot if changed
///       |           |
///       |           v
///       |    Handle each RuntimeEffect in order
///       |           |
///       |           +-- Execute -----> Executor::execute(...)
///       |           |                  completion enters inbox
///       |           +-- ArmTimer ----> arm or replace timer
///       |           +-- CancelTimer -> cancel timer
///       |           +-- Exit --------> terminate; no later effects
///       |           |
///       |           v
///       |    Wait for inbox message or timer
///       |           |
///       |           +-- timer ready -> wake to inspect current alarm queue
///       |           +-- inbox ready -> retain input until due alarms finish
///       |           |
///       |           v
///       |    Read clock; deliver earliest due alarm before any retained input
///       |           |
///       |           +-- clock failed -> log and exit process
///       |           v
///       |    StateMachine::on_event(...)
///       |           |
///       |           | fills the borrowed effects builder
///       +-----------+
/// ```
///
/// ## Executor
///
/// The caller constructs the executor and passes it to `Runner::spawn`. Keep its
/// runtime dependencies and configuration outside serializable state. Dispatch
/// order is preserved; completion order is not. `execute` must return promptly and
/// send async completions through the inbox. Snapshots publish before dispatch,
/// so readers can observe pending work.
///
/// ## Timers and clock faults
///
/// There is one armed alarm per timer identity. Before each input delivery, the
/// runner reads the clock and drains alarms due at that time in (deadline, timer)
/// order. Each alarm's transition publishes its snapshot, applies runtime effects,
/// and dispatches domain effects before choosing the next event. Remote work need
/// not have finished. Cancellation and replacement therefore affect even
/// other already-due alarms. The clock is read again after each transition.
///
/// An alarm is never delivered early, but process pauses can delay it. Events carry
/// actual delivery time, not the scheduled deadline. Handle I/O expiry in the alarm
/// handler; do not duplicate its deadline check before every input. Keep operation
/// correlation and phase checks. Include operation IDs in timer variants when
/// useful, and schedule recurring alarms in the future to avoid starving inputs.
///
/// This orders alarms before inputs, not lease expiry before earlier overdue
/// alarms. Keep authority checks needed while handling other alarms and at external
/// admission/dispatch/reply boundaries. An I/O alarm cannot validate a historical
/// claim's original lease deadline, and timeout does not recall remote mutations.
///
/// Custom drivers and simulations must enforce this same ordering and replacement
/// contract. Direct calls to `on_event` or `StateMachineAdapter` only execute one
/// transition; they do not own an alarm queue. See [`StateMachine`] for slots.
///
/// Failed clock reads or timer sleeps are logged and terminate the process with
/// exit status 1. No event is delivered; an initial clock failure prevents startup.
/// The process supervisor is responsible for restarting the process.
///
/// ## Termination
///
/// `RuntimeEffect::Exit` exits the process in production, rather than stopping just
/// this runner. Dropping all inbox senders stops the runner after the queue drains.
/// Separately spawned effect tasks may outlive it.
pub trait EphemeralStateMachine: Sized + Send + Sync + 'static {
	type Slot: Slot;
	type State: StateMachineState;
	type Event: EventBody;
	type Effect: Effect;
	type Snapshot: Snapshot;
	type Timer: Clone + Debug + Ord + Hash + Serialize + DeserializeOwned + Send;

	/// Label for logs and metrics.
	const DEBUG_NAME: &'static str;

	/// Bounded metrics label for a timer event, written as an exhaustive match.
	fn timer_name(timer: &Self::Timer) -> &'static str;

	/// Bounds queued inputs before the machine's own admission budgets apply.
	fn inbox_capacity(_state: &Self::State) -> usize {
		256
	}

	/// Entropy is drawn and logged by the runner; consume once and never store in state.
	/// Recovery effects must be idempotent because earlier work may repeat.
	fn start(
		state: &mut Self::State,
		now: Clocks,
		entropy: Entropy,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	);

	/// Handle only alarms delivered by a conforming driver. Reject stale completions
	/// by operation/phase; the runner delivers due I/O alarms before input responses.
	/// Retain independent authority and historical-claim deadline checks.
	fn on_event(
		state: &mut Self::State,
		event: EphemeralStateMachineEvent<Self>,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	);

	fn snapshot(state: &Self::State) -> Self::Snapshot;

	/// Processing time, in microseconds, that a runner on simulated time spends on its clock
	/// after this event, so queues build behind it as they do behind a busy core. Real time
	/// passes on its own in production, where configurations leave this at zero.
	fn simulated_cost_us(_state: &Self::State, _event: &EphemeralStateMachineEvent<Self>) -> u64 {
		0
	}

	/// Expected faults are valid protocol inputs, not invariant failures.
	fn debug_validate(state: &Self::State) -> anyhow::Result<()>;

	/// Return a comparison copy, preserving distinctions that affect future behavior.
	/// Defaults to an exact copy; override only for rank-sensitive state data.
	fn normalize(state: &Self::State, _now: Clocks, _ranks: &RankMap<Self::Slot>) -> Self::State {
		state.clone()
	}
}

pub type EphemeralStateMachineEvent<M> =
	Event<RuntimeEvent<<M as EphemeralStateMachine>::Event, <M as EphemeralStateMachine>::Timer>>;

pub type EphemeralStateMachineEffect<M> =
	RuntimeEffect<<M as EphemeralStateMachine>::Effect, <M as EphemeralStateMachine>::Timer>;

/// Exposes an ephemeral state machine to drivers using the pure transition contract.
pub struct StateMachineAdapter<M>(std::marker::PhantomData<fn() -> M>);

impl<M: EphemeralStateMachine> StateMachine for StateMachineAdapter<M> {
	type Slot = <M as EphemeralStateMachine>::Slot;

	type State = <M as EphemeralStateMachine>::State;

	type Event =
		RuntimeEvent<<M as EphemeralStateMachine>::Event, <M as EphemeralStateMachine>::Timer>;

	type Effect = M::Effect;

	type Timer = M::Timer;

	type Snapshot = <M as EphemeralStateMachine>::Snapshot;

	const DEBUG_NAME: &'static str = <M as EphemeralStateMachine>::DEBUG_NAME;

	fn start(
		state: &mut Self::State,
		now: Clocks,
		entropy: Entropy,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	) {
		<M as EphemeralStateMachine>::start(state, now, entropy, effects)
	}

	fn on_event(
		state: &mut Self::State,
		event: Event<Self::Event>,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	) {
		<M as EphemeralStateMachine>::on_event(state, event, effects)
	}

	fn snapshot(state: &Self::State) -> Self::Snapshot {
		<M as EphemeralStateMachine>::snapshot(state)
	}

	fn debug_validate(state: &Self::State) -> anyhow::Result<()> {
		<M as EphemeralStateMachine>::debug_validate(state)
	}

	fn normalize(state: &Self::State, now: Clocks, ranks: &RankMap<Self::Slot>) -> Self::State {
		<M as EphemeralStateMachine>::normalize(state, now, ranks)
	}
}
