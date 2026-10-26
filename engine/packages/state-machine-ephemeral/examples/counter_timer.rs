//! Increment a counter on three timer ticks, then stop rearming the timer.
//!
//! Run with `cargo run -p rivet-state-machine-ephemeral --example counter_timer`.
//! Each next tick is scheduled from delivery time, so delayed ticks do not cause
//! a burst of catch-up increments.

use rivet_env::{Clocks, Env, ProductionEnv};
use rivet_state_machine::Entropy;
use rivet_state_machine::{NoEffect, NoSlot};
use rivet_state_machine_ephemeral::{
	EffectContext, EphemeralStateMachine, EphemeralStateMachineEvent, Executor, Runner,
	RuntimeEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const TICK_INTERVAL_MS: u64 = 100;
const TICK_COUNT: u64 = 3;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.without_time()
		.with_target(false)
		.init();

	let env: Arc<dyn Env> = Arc::new(ProductionEnv::new()?);
	demonstrate_timer_ticks(env).await
}

async fn demonstrate_timer_ticks(env: Arc<dyn Env>) -> anyhow::Result<()> {
	let mut handle =
		Runner::<Counter>::spawn(env, CounterState::default(), Arc::new(CounterExecutor));

	// Startup arms the first tick. Keep the inbox open while the timers run.
	loop {
		let value = handle.snapshot.borrow_and_update().value;
		if value == TICK_COUNT {
			break;
		}

		handle.snapshot.changed().await?;
	}

	// The final tick did not rearm. Close the inbox to stop the runner.
	drop(handle.inbox);
	let _ = handle.stopped.await;

	let snapshot = handle.snapshot.borrow().clone();
	assert_eq!(snapshot.value, TICK_COUNT);
	tracing::info!(value = snapshot.value, "counter finished ticking");

	Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CounterState {
	value: u64,
}

impl rivet_state_machine::StateMachineState for CounterState {}

impl rivet_state_machine::Snapshot for CounterState {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum CounterEvent {}

impl rivet_state_machine::EventBody for CounterEvent {
	fn name(&self) -> &'static str {
		"counter_event"
	}
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
enum Timer {
	Tick,
}

struct Counter;

impl EphemeralStateMachine for Counter {
	type Slot = NoSlot;

	type State = CounterState;

	type Event = CounterEvent;

	type Effect = NoEffect;

	type Timer = Timer;

	type Snapshot = CounterState;

	const DEBUG_NAME: &'static str = "counter_timer";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			Timer::Tick => "tick",
		}
	}

	fn start(
		_: &mut CounterState,
		now: Clocks,
		_: Entropy,
		effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		{
			effects.arm_timer_at(Timer::Tick, now.boot_ms + TICK_INTERVAL_MS);
		}
	}

	fn on_event(
		state: &mut CounterState,
		event: EphemeralStateMachineEvent<Self>,
		effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		match event.body {
			RuntimeEvent::Input(input) => match input {},
			RuntimeEvent::Timer(Timer::Tick) => state.value += 1,
		}

		if state.value == TICK_COUNT {
			return;
		}

		{
			effects.arm_timer_at(Timer::Tick, event.now.boot_ms + TICK_INTERVAL_MS);
		}
	}

	fn snapshot(state: &CounterState) -> CounterState {
		state.clone()
	}

	fn debug_validate(state: &CounterState) -> anyhow::Result<()> {
		anyhow::ensure!(state.value <= TICK_COUNT, "counter exceeded its tick limit");

		Ok(())
	}
}

struct CounterExecutor;

impl Executor<Counter> for CounterExecutor {
	fn execute(&self, effect: NoEffect, _ctx: EffectContext<Counter>) {
		match effect {}
	}
}
