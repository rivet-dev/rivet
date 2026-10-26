//! Refresh a counter while ignoring superseded and duplicate completions.
//!
//! Run with `cargo run -p rivet-state-machine --example counter_core`.
//! This example drives the core directly and supplies completion events in a
//! chosen order. An async executor would carry each Load effect's operation ID
//! through its read and return it in the Loaded event. Slots do not cancel reads.

use rivet_env::Clocks;
use rivet_state_machine::Entropy;
use rivet_state_machine::{Event, InFlightOps, OpId, RankMap, StateMachine};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
enum Slot {
	Load,
}

impl rivet_state_machine::Slot for Slot {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CounterState {
	value: i64,
	error: Option<String>,
	ops: InFlightOps<Slot>,
}

impl rivet_state_machine::StateMachineState for CounterState {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum CounterEvent {
	Refresh,
	Loaded {
		op: OpId<Slot>,
		result: Result<i64, String>,
	},
}

impl rivet_state_machine::EventBody for CounterEvent {
	fn name(&self) -> &'static str {
		match self {
			CounterEvent::Refresh => "refresh",
			CounterEvent::Loaded { .. } => "loaded",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum CounterEffect {
	Load { op: OpId<Slot> },
}

impl rivet_state_machine::Effect for CounterEffect {
	fn name(&self) -> &'static str {
		match self {
			CounterEffect::Load { .. } => "load",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CounterSnapshot {
	value: i64,
	error: Option<String>,
	loading: bool,
}

impl rivet_state_machine::Snapshot for CounterSnapshot {}

struct Counter;

impl StateMachine for Counter {
	type Slot = Slot;

	type State = CounterState;

	type Event = CounterEvent;

	type Timer = ();

	type Effect = CounterEffect;

	type Snapshot = CounterSnapshot;

	const DEBUG_NAME: &'static str = "counter_core";

	fn start(
		_: &mut CounterState,
		_: Clocks,
		_: Entropy,
		_effects: &mut rivet_state_machine::Effects<CounterEffect, ()>,
	) {
	}

	fn on_event(
		state: &mut CounterState,
		event: Event<CounterEvent>,
		effects: &mut rivet_state_machine::Effects<CounterEffect, ()>,
	) {
		match event.body {
			CounterEvent::Refresh => {
				let op = state.ops.issue(Slot::Load);
				state.error = None;

				effects.execute(CounterEffect::Load { op });
			}
			CounterEvent::Loaded { op, result } => {
				// Check before applying either success or failure: both can be stale.
				if !state.ops.complete(op) {
					return;
				}

				match result {
					Ok(value) => state.value = value,
					Err(error) => state.error = Some(error),
				}
			}
		}
	}

	fn snapshot(state: &CounterState) -> CounterSnapshot {
		CounterSnapshot {
			value: state.value,
			error: state.error.clone(),
			loading: state.ops.pending(Slot::Load).is_some(),
		}
	}

	fn debug_validate(_: &CounterState) -> anyhow::Result<()> {
		Ok(())
	}

	fn normalize(state: &CounterState, _: Clocks, _: &RankMap<Slot>) -> CounterState {
		CounterState {
			ops: state.ops.normalize(),
			..state.clone()
		}
	}
}

fn main() {
	tracing_subscriber::fmt()
		.without_time()
		.with_target(false)
		.init();

	let now = Clocks {
		boot_ms: 0,
		unix_ms: 0,
	};
	let mut state = CounterState::default();
	assert!(
		{
			let mut output = rivet_state_machine::Effects::new(now);
			Counter::start(&mut state, now, Entropy([0; 32]), &mut output);
			output
		}
		.is_empty()
	);

	// Both reads can still be running. B supersedes A as soon as B is issued.
	let a = refresh(&mut state, now);
	let b = refresh(&mut state, now);

	loaded(&mut state, now, b, Ok(20));
	assert_eq!(Counter::snapshot(&state).value, 20);
	tracing::info!("b completes: counter = 20");

	loaded(&mut state, now, a, Ok(10));
	loaded(&mut state, now, b, Ok(20));
	assert_eq!(Counter::snapshot(&state).value, 20);
	assert!(!Counter::snapshot(&state).loading);
	tracing::info!("late a and duplicate b: counter stays 20");

	// Supersession, not arrival order, determines whether a completion is stale.
	let c = refresh(&mut state, now);
	let d = refresh(&mut state, now);
	loaded(&mut state, now, c, Err("old read failed".into()));
	assert_eq!(state.ops.pending(Slot::Load), Some(d));
	assert_eq!(Counter::snapshot(&state).error, None);
	tracing::info!("c fails before d completes: ignored; d remains pending");

	loaded(&mut state, now, d, Ok(30));
	assert_eq!(Counter::snapshot(&state).value, 30);
	assert!(!Counter::snapshot(&state).loading);
	tracing::info!("d completes: counter = 30");
}

fn refresh(state: &mut CounterState, now: Clocks) -> OpId<Slot> {
	let effects = {
		let mut output = rivet_state_machine::Effects::new(now);
		Counter::on_event(
			state,
			Event {
				entropy: Entropy([0; 32]),
				now,
				body: CounterEvent::Refresh,
			},
			&mut output,
		);
		output
	};
	let [rivet_state_machine::RuntimeEffect::Execute(CounterEffect::Load { op })] =
		effects.as_slice()
	else {
		panic!("refresh must request one load");
	};
	op.to_owned()
}

fn loaded(state: &mut CounterState, now: Clocks, op: OpId<Slot>, result: Result<i64, String>) {
	let effects = {
		let mut output = rivet_state_machine::Effects::new(now);
		Counter::on_event(
			state,
			Event {
				entropy: Entropy([0; 32]),
				now,
				body: CounterEvent::Loaded { op, result },
			},
			&mut output,
		);
		output
	};
	assert!(effects.is_empty());
	Counter::debug_validate(state).unwrap();
}
