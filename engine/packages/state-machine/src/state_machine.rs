use crate::Effects;
use rivet_env::Clocks;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	hash::Hash,
};

/// Correlates an effect and its response within one state machine lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpId<S> {
	/// Identifies a correlation lane; InFlightOps can supersede earlier work in that lane.
	pub slot: S,
	/// Increases for every operation in this slot, not just retries.
	pub index: u32,
}

/// Mints response-correlation IDs when a state machine already owns its in-flight state,
/// for example in `ConditionalWrite`. Use [`InFlightOps`] when per-slot busy tracking is needed.
/// Keep one sequence for each slot throughout the state machine lifetime, including helper replacement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpSequence<S: Ord> {
	/// Last allocated index per slot; retained even after the caller completes its work.
	indices: BTreeMap<S, u32>,
}

impl<S: Ord> Default for OpSequence<S> {
	/// Starts every slot at zero without requiring a default slot value.
	fn default() -> Self {
		Self {
			indices: BTreeMap::new(),
		}
	}
}

impl<S: Copy + Ord> OpSequence<S> {
	/// Mints the next ID without recording pending work; the caller must correlate responses.
	/// Each slot starts at one. Exhaustion panics rather than wrapping and reusing a stale ID:
	/// a state machine lifetime must allocate fewer than 2^32 operations in any one slot.
	pub fn next(&mut self, slot: S) -> OpId<S> {
		let index = self.indices.entry(slot).or_default();
		*index = index.checked_add(1).expect("operation index exhausted");
		OpId {
			slot,
			index: *index,
		}
	}

	/// Returns zero for an unused slot, or the last ID's index regardless of completion status.
	pub fn last_index(&self, slot: S) -> u32 {
		self.indices.get(&slot).copied().unwrap_or(0)
	}

	/// Zeros indices in a comparison-only copy; never normalize a live allocator.
	/// Preserves the existing operation normalization convention, independent of ranks.
	/// Callers must normalize queued responses consistently, preserving stale/current distinctions.
	pub fn normalize(&mut self, _ranks: &RankMap<S>) {
		for index in self.indices.values_mut() {
			*index = 0;
		}
	}
}

/// Allocates IDs and tracks busy slots for state machines that do not already store pending work.
/// Carry [`Self::issue`]'s ID through the effect and response, then call [`Self::complete`]
/// before applying the result. Use [`OpSequence`] alone when another phase or helper tracks it.
///
/// Issuing a replacement in one slot rejects the previous operation's response, regardless of
/// arrival order. The current response is accepted only once. Other slots remain independent.
/// Supersession does not cancel tasks, timers, or external work.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InFlightOps<S: Ord> {
	/// Sole owner of indices, including slots whose operations have completed.
	sequence: OpSequence<S>,
	/// Busy status only; each pending ID is derived from the sequence's last index.
	running: BTreeSet<S>,
}

impl<S: Ord> Default for InFlightOps<S> {
	/// Creates an empty tracker without requiring a default slot value.
	fn default() -> Self {
		Self {
			sequence: OpSequence::default(),
			running: BTreeSet::new(),
		}
	}
}

impl<S: Copy + Ord> InFlightOps<S> {
	/// Allocates an ID and marks its slot busy, invalidating older completions in that slot.
	/// Cancelling timers or external work requires separate effects. Sequence exhaustion panics.
	pub fn issue(&mut self, slot: S) -> OpId<S> {
		let op = self.sequence.next(slot);
		self.running.insert(slot);
		op
	}

	/// Accepts the current response once, clearing busy status without discarding its index.
	/// Returns false without changes for stale, duplicate, or unknown completions.
	/// Accepting a completion does not cancel its timer or external work.
	pub fn complete(&mut self, op: OpId<S>) -> bool {
		if self.pending(op.slot) != Some(op) {
			return false;
		}
		self.running.remove(&op.slot);
		true
	}

	/// Returns the ID awaited by this slot, including work whose response is still in flight.
	pub fn pending(&self, slot: S) -> Option<OpId<S>> {
		self.running.contains(&slot).then(|| OpId {
			slot,
			index: self.sequence.last_index(slot),
		})
	}

	/// Includes completed operations so callers can classify and normalize late responses.
	pub fn last_index(&self, slot: S) -> u32 {
		self.sequence.last_index(slot)
	}

	/// Returns a comparison-only copy with zero indices and unchanged busy status.
	/// Never replace the live tracker. Normalize queued completions consistently,
	/// preserving current/stale and idle/running distinctions.
	pub fn normalize(&self) -> Self {
		let mut normalized = self.clone();
		normalized.sequence.normalize(&RankMap::default());
		normalized
	}
}

/// Per-delivery randomness drawn by the runner and logged with the event for replay.
/// Consume once per transition; never store the seed or its generator in state.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entropy(
	/// Recorded seed supplied by the runner or a deterministic test driver.
	pub [u8; 32],
);

impl Entropy {
	/// Consume this delivery's seed to create its pure, local draw sequence.
	pub fn rng(self) -> rivet_env::rand::LocalRng {
		rivet_env::rand::LocalRng::from_seed(self.0)
	}
}

/// `now` is stamped at delivery, not when the work was issued or completed.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<E> {
	/// Clock observation at delivery.
	pub now: Clocks,

	/// Runner-drawn seed logged with the event; consume once and never store in state.
	pub entropy: Entropy,

	/// Domain input or runtime alarm being delivered.
	pub body: E,
}

/// Groups operations that supersede one another, such as refreshes of the same
/// counter. With [`InFlightOps`], issuing a replacement makes earlier completions in that
/// slot stale regardless of arrival order. Work in other slots stays independent.
/// Declaring a slot alone does not reject events or cancel running work.
pub trait Slot:
	Copy + Debug + Ord + Hash + Serialize + DeserializeOwned + Send + Sync + 'static
{
}

/// Use when a state machine does not track operations in slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoSlot {}

impl Slot for NoSlot {}

/// One I/O lane. Unlike NoSlot, this inhabited value can be carried in an OpId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SingleSlot;

impl Slot for SingleSlot {}

pub trait StateMachineState:
	Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send
{
}

pub trait EventBody: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send {
	/// Bounded metrics label for this event, written as an exhaustive match. Wrapper variants
	/// forward to the wrapped event's name.
	fn name(&self) -> &'static str;
}

/// Work required by a state machine transition, such as writing storage, scheduling
/// timers, or stopping the process. The transition emits effects; the runner or
/// executor performs them.
pub trait Effect: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send {
	/// Bounded metrics label for this effect, written as an exhaustive match. Wrapper variants
	/// forward to the wrapped effect's name.
	fn name(&self) -> &'static str;
}

/// Use when transitions emit no effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoEffect {}

impl Effect for NoEffect {
	fn name(&self) -> &'static str {
		match *self {}
	}
}

/// Excluded from simulation fingerprints and durable recovery data.
pub trait Snapshot: Clone + Debug + PartialEq + Send + Sync {}

/// Defines transition logic only: no I/O, ambient clock reads, or randomness.
/// Custom drivers can call this contract directly. The runtime in
/// `rivet-state-machine-ephemeral` exposes its transitions through
/// `StateMachineAdapter`; the diagram shows that integration.
///
/// ```text
///            Runner starts
///                   |
///                   | calls
///                   v
///            StateMachine::snapshot(&state)
///                   |
///                   | returns initial reader view
///                   v
///            Runner initializes snapshot watch
///                   |
///                   | calls
///                   v
///            StateMachine::start(&mut state, now, entropy)
///                   |
///                   | fills the borrowed effects builder
///                   v
///       +--> Runner updates snapshot watch if changed
///       |           |
///       |           v
///       |    Runner handles effects in order
///       |           |
///       |           +-- Timer ----> arms/cancels timer
///       |           |
///       |           +-- Other ----> calls Executor::execute(...)
///       |           |              completion returns through inbox
///       |           |
///       |           +-- Exit -----> stops
///       |           |
///       |           v
///       |    Runner waits for inbox message or timer
///       |           |
///       |           | calls with Event { now, entropy, body }
///       |           v
///       |    StateMachine::on_event(&mut state, event)
///       |           |
///       |           | fills the borrowed effects builder
///       +-----------+
/// ```
///
/// Callers send event bodies through `handle.inbox.send(body).await`; the runner
/// supplies delivery time and entropy. `handle.snapshot` is a `tokio::sync::watch::Receiver`
/// retaining the latest published view. Publishing does not wait for external
/// work to finish; its completion arrives as another event.
///
/// ## Slots
///
/// A slot names a group of operations that supersede one another, usually an enum
/// variant. Issuing a new operation in a slot makes completion events from its
/// previous operation stale; other slots remain independent.
///
/// For example, a new fetch in a `Config` slot replaces the pending fetch, so a late
/// reply from the old fetch is ignored. Superseding work does not cancel its I/O.
pub trait StateMachine: Sized + Send + Sync + 'static {
	type Slot: Slot;
	type State: StateMachineState;
	type Event: EventBody;
	type Effect: Effect;
	type Timer: Clone + std::fmt::Debug + Send;
	type Snapshot: Snapshot;

	/// Label for logs and metrics.
	const DEBUG_NAME: &'static str;

	/// Called once per state machine lifetime, before events, to begin startup or
	/// recovery. Entropy is drawn and logged by the runner; consume it once and never store it.
	/// Recovery effects must be idempotent because earlier work may repeat.
	fn start(
		state: &mut Self::State,
		now: Clocks,
		entropy: Entropy,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	);

	/// Ignore stale or duplicate operation completions.
	///
	/// Each store effect needs an `OpTimeout`. Expiry is indeterminate, not proof of
	/// failure: retire the operation and resolve ambiguity through read-back or retry.
	/// Cancel or replace obsolete timers before reusing them. `InFlightOps` does not cancel
	/// timers or external work, and cancellation cannot undo a committed write.
	fn on_event(
		state: &mut Self::State,
		event: Event<Self::Event>,
		effects: &mut Effects<Self::Effect, Self::Timer>,
	);

	/// Provides the data readers need to know about this state machine. Readers see
	/// a consistent published snapshot independently of the state machine's main loop.
	/// The runner publishes after startup and every event, before dispatching effects,
	/// and wakes readers only when this view changes.
	fn snapshot(state: &Self::State) -> Self::Snapshot;

	/// Runs after each event in debug and simulation, but not production release
	/// builds. Expected faults are valid protocol inputs, not invariant failures.
	/// Cross-node properties belong in cluster oracles.
	fn debug_validate(state: &Self::State) -> anyhow::Result<()>;

	/// Removes history-dependent differences in timestamps, operation IDs, counters,
	/// versions, and ETags, returning a copy for state comparison. Preserves relative
	/// timing and other distinctions that affect future behavior.
	/// Defaults to an exact copy; override only for rank-sensitive state data.
	fn normalize(state: &Self::State, _now: Clocks, _ranks: &RankMap<Self::Slot>) -> Self::State {
		state.clone()
	}
}

/// Lets simulation tests skip equivalent states they have already explored.
///
/// For example, renewal versions `41, 42` and `899, 900` both become `1, 2`.
/// This preserves equality and ordering when the exact numbers do not matter.
///
/// `StateMachine::normalize()`. State machines, stored records, and queued
/// messages all use the same replacement numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankMap<S: Ord> {
	pub etag: BTreeMap<String, u64>,
	pub version: BTreeMap<u64, u64>,
	pub op: BTreeMap<(S, u32), u64>,
}

impl<S: Ord> Default for RankMap<S> {
	fn default() -> Self {
		Self {
			etag: BTreeMap::new(),
			version: BTreeMap::new(),
			op: BTreeMap::new(),
		}
	}
}
