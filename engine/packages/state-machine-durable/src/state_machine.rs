use crate::Effects;
use rivet_env::Clocks;
pub use rivet_state_machine::RuntimeEffect;
use rivet_state_machine::{
	Effect, Entropy, Event, EventBody, RankMap, Slot, Snapshot, StateMachineState,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fmt::Debug, hash::Hash};

/// Inputs delivered by the durable runner. Callers send only the `Input` payload; timer
/// events are reserved for timers armed through [`Effects`].
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEvent<I, T> {
	Input(I),
	Timer(T),
}

impl<I: EventBody, T> EventBody for RuntimeEvent<I, T>
where
	T: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send,
{
	/// Timer events are named `timer` because the timer type has no name of its own.
	fn name(&self) -> &'static str {
		match self {
			RuntimeEvent::Input(input) => input.name(),
			RuntimeEvent::Timer(_) => "timer",
		}
	}
}

pub trait DurableStateMachine: Sized + Send + Sync + 'static {
	type Slot: Slot;
	type State: StateMachineState + Sync;
	type Event: EventBody + Sync;
	type Effect: Effect;
	type Snapshot: Snapshot;
	type Timer: Clone + Debug + Eq + Hash + Serialize + DeserializeOwned + Send + Sync;
	/// What the executor's `prepare` supplies to each transition: data read outside the
	/// deterministic core, such as a selection made from the current state. `()` for machines
	/// whose executor prepares nothing.
	type Preparation: Default + Send + 'static;
	/// A write the executor applies inside the transition's transaction, through
	/// `Executor::write`, so it lands atomically with the state. Only local writes to the
	/// machine's own volume belong here. `()` for machines that write nothing this way.
	type WriteEffect: Debug + Send + 'static;

	const DEBUG_NAME: &'static str;

	fn start(
		state: &mut Self::State,
		now: Clocks,
		entropy: Entropy,
		effects: &mut Effects<Self::Effect, Self::Timer, Self::WriteEffect>,
	);

	/// Applies one input or timer. `preparation` is what the executor prepared for this event
	/// from the state as it was just before; the transition is otherwise a pure function of
	/// its arguments.
	fn on_event(
		state: &mut Self::State,
		event: Event<RuntimeEvent<Self::Event, Self::Timer>>,
		preparation: Self::Preparation,
		effects: &mut Effects<Self::Effect, Self::Timer, Self::WriteEffect>,
	);

	fn snapshot(state: &Self::State) -> Self::Snapshot;

	fn debug_validate(state: &Self::State) -> anyhow::Result<()>;

	fn normalize(state: &Self::State, _now: Clocks, _ranks: &RankMap<Self::Slot>) -> Self::State {
		state.clone()
	}

	fn serialize_state(state: &Self::State) -> anyhow::Result<Vec<u8>>;

	fn deserialize_state(bytes: &[u8]) -> anyhow::Result<Self::State>;

	fn serialize_timer(timer: &Self::Timer) -> anyhow::Result<Vec<u8>>;

	fn deserialize_timer(bytes: &[u8]) -> anyhow::Result<Self::Timer>;
}
