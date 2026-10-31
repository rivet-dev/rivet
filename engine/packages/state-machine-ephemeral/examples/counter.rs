//! Increment a counter through the runner and read its final snapshot.
//!
//! Run with `cargo run -p rivet-state-machine-ephemeral --example counter`.
//! See `counter_slot.rs` for async reads and superseded replies.

use rivet_env::{Clocks, Env, ProductionEnv};
use rivet_state_machine::Entropy;
use rivet_state_machine::{NoEffect, NoSlot};
use rivet_state_machine_ephemeral::{
	EffectContext, EphemeralStateMachine, EphemeralStateMachineEvent, Executor, NoTimer, Runner,
	RuntimeEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.without_time()
		.with_target(false)
		.init();

	let env: Arc<dyn Env> = Arc::new(ProductionEnv::new()?);
	demonstrate_increments(env).await
}

async fn demonstrate_increments(env: Arc<dyn Env>) -> anyhow::Result<()> {
	let handle = Runner::<Counter>::spawn(env, CounterState::default(), Arc::new(CounterExecutor));

	handle.inbox.send(Increment).await?;
	handle.inbox.send(Increment).await?;

	// Sending queues each input. Closing the inbox lets us wait for both to finish.
	drop(handle.inbox);
	let _ = handle.stopped.await;

	let snapshot = handle.snapshot.borrow().clone();
	assert_eq!(snapshot.value, 2);
	tracing::info!(value = snapshot.value, "counter incremented twice");

	Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CounterState {
	value: u64,
}

impl rivet_state_machine::StateMachineState for CounterState {}

impl rivet_state_machine::Snapshot for CounterState {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Increment;

impl rivet_state_machine::EventBody for Increment {
	fn name(&self) -> &'static str {
		"increment"
	}
}

struct Counter;

impl EphemeralStateMachine for Counter {
	type Slot = NoSlot;

	type State = CounterState;

	type Event = Increment;

	type Effect = NoEffect;

	type Timer = NoTimer;

	type Snapshot = CounterState;

	const DEBUG_NAME: &'static str = "counter";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match *timer {}
	}

	fn start(
		_: &mut CounterState,
		_: Clocks,
		_: Entropy,
		_effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
	}

	fn on_event(
		state: &mut CounterState,
		event: EphemeralStateMachineEvent<Self>,
		_effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		match event.body {
			RuntimeEvent::Input(Increment) => state.value += 1,
			RuntimeEvent::Timer(timer) => match timer {},
		}
	}

	fn snapshot(state: &CounterState) -> CounterState {
		state.clone()
	}

	fn debug_validate(_: &CounterState) -> anyhow::Result<()> {
		Ok(())
	}
}

struct CounterExecutor;

impl Executor<Counter> for CounterExecutor {
	fn execute(&self, effect: NoEffect, _ctx: EffectContext<Counter>) {
		match effect {}
	}
}
