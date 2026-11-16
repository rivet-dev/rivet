use crate::Effects;
use rivet_data::families::VersionFamily;
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

	/// The version family the state and timers are encoded in. The runner encodes both at the
	/// family's write version, the highest every live storage worker reads, or above it behind
	/// a tablet version mark when that version cannot express them; see `Runner`.
	const FAMILY: VersionFamily;

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

	/// The incarnation the state was last reset for after a group fork, the stamp
	/// `VolumeRecordView::pending_fork_reset` compares. While it differs from the incarnation
	/// of a volume made by a group fork, the state is inherited, and the runner arms the Depot
	/// alarm again in the commit that resets it. A machine whose volume is never made by a
	/// group fork returns `None`.
	fn fork_reset(state: &Self::State) -> Option<[u8; 16]>;

	fn debug_validate(state: &Self::State) -> anyhow::Result<()>;

	fn normalize(state: &Self::State, _now: Clocks, _ranks: &RankMap<Self::Slot>) -> Self::State {
		state.clone()
	}

	/// Encodes the state at `version` of [`Self::FAMILY`]. A version that cannot express the
	/// state must refuse with [`rivet_data::versioned::Inexpressible`], which is the only error
	/// the runner skips while it looks for the lowest version at or above the selected one that
	/// expresses it. Any other error fails the transition. See `state_encoding`.
	fn serialize_state(state: &Self::State, version: u16) -> anyhow::Result<Vec<u8>>;

	fn deserialize_state(bytes: &[u8]) -> anyhow::Result<Self::State>;

	/// The timer's stable identity. Arming a timer replaces the armed timer with the same id and
	/// cancelling one removes it, whatever encoding its payload was stored in. The id names
	/// every field that distinguishes two timers to the machine, and never changes for a timer
	/// that may be stored.
	fn timer_id(timer: &Self::Timer) -> Vec<u8>;

	/// Encodes the timer at `version` of [`Self::FAMILY`]. A version that cannot express the
	/// timer must refuse with [`rivet_data::versioned::Inexpressible`] (anywhere in the error's
	/// chain), which is the only error the runner skips while it looks for the lowest version at
	/// or above the selected one that expresses it. Any other error fails the transition. See
	/// `timer_encoding`.
	fn serialize_timer(timer: &Self::Timer, version: u16) -> anyhow::Result<Vec<u8>>;

	/// Encodes a timer rearmed by a delivery of the same timer id. The fired payload
	/// can establish a version the volume's readers already had to support.
	fn serialize_rearmed_timer(
		timer: &Self::Timer,
		version: u16,
		_fired_payload: &[u8],
	) -> anyhow::Result<Vec<u8>> {
		Self::serialize_timer(timer, version)
	}

	fn deserialize_timer(bytes: &[u8]) -> anyhow::Result<Self::Timer>;
}
