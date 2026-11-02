//! A second refresh supersedes the first; late and duplicate replies are ignored.
//!
//! Run with `cargo run -p rivet-state-machine-ephemeral --example counter_slot`.
//! The executor hands dispatched reads to the scenario, which supplies their replies.
//! A real executor would call its storage client and send the same completion event.

use rivet_env::{Clocks, Env, ProductionEnv};
use rivet_state_machine::Entropy;
use rivet_state_machine::{InFlightOps, OpId, RankMap};
use rivet_state_machine_ephemeral::{
	EffectContext, EphemeralStateMachine, EphemeralStateMachineEvent, Executor, NoTimer, Runner,
	RuntimeEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.without_time()
		.with_target(false)
		.init();

	let env: Arc<dyn Env> = Arc::new(ProductionEnv::new()?);
	demonstrate_superseded_reads(env).await
}

async fn demonstrate_superseded_reads(env: Arc<dyn Env>) -> anyhow::Result<()> {
	// Start the runner. The executor forwards reads to this scenario.
	let (read_dispatch, mut dispatched_reads) = mpsc::channel(2);
	let handle = Runner::<Counter>::spawn(
		env,
		CounterState::default(),
		Arc::new(CounterExecutor {
			dispatched_reads: read_dispatch,
		}),
	);

	// Request two refreshes, waiting until each read reaches the executor.
	handle.inbox.send(CounterEvent::Refresh).await?;
	let first_read = dispatched_reads
		.recv()
		.await
		.expect("first read dispatched");

	handle.inbox.send(CounterEvent::Refresh).await?;
	let replacement_read = dispatched_reads
		.recv()
		.await
		.expect("replacement read dispatched");

	// Reply to the newer read first, then send a duplicate and the older reply.
	replacement_read.send_reply(20).await?;
	replacement_read.send_reply(999).await?;
	first_read.send_reply(10).await?;

	// Close the inbox and wait until every queued reply has been processed.
	drop((first_read, replacement_read, handle.inbox));
	let _ = handle.stopped.await;

	// Neither the duplicate's 999 nor the older read's 10 may replace 20.
	let snapshot = handle.snapshot.borrow().clone();
	assert_eq!(snapshot.value, 20);
	assert!(!snapshot.loading);
	tracing::info!(value = snapshot.value, "late and duplicate replies ignored");

	Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
enum Slot {
	Refresh,
}

impl rivet_state_machine::Slot for Slot {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CounterState {
	value: u64,
	ops: InFlightOps<Slot>,
}

impl rivet_state_machine::StateMachineState for CounterState {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum CounterEvent {
	Refresh,
	Loaded { op: OpId<Slot>, value: u64 },
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
struct ReadCounter {
	op: OpId<Slot>,
}

impl rivet_state_machine::Effect for ReadCounter {
	fn name(&self) -> &'static str {
		"read_counter"
	}
}

#[derive(Clone, Debug, PartialEq)]
struct CounterSnapshot {
	value: u64,
	loading: bool,
}

impl rivet_state_machine::Snapshot for CounterSnapshot {}

struct Counter;

impl EphemeralStateMachine for Counter {
	type Slot = Slot;

	type State = CounterState;

	type Event = CounterEvent;

	type Effect = ReadCounter;

	type Timer = NoTimer;

	type Snapshot = CounterSnapshot;

	const DEBUG_NAME: &'static str = "counter_slot";

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
		effects: &mut rivet_state_machine::Effects<Self::Effect, Self::Timer>,
	) {
		match event.body {
			RuntimeEvent::Input(CounterEvent::Refresh) => {
				let op = state.ops.issue(Slot::Refresh);

				effects.execute(ReadCounter { op });
			}
			RuntimeEvent::Input(CounterEvent::Loaded { op, value }) => {
				// Complete before touching state: only the latest read is accepted, once.
				if !state.ops.complete(op) {
					return;
				}

				state.value = value;
			}
			RuntimeEvent::Timer(timer) => match timer {},
		}
	}

	fn snapshot(state: &CounterState) -> CounterSnapshot {
		CounterSnapshot {
			value: state.value,
			loading: state.ops.pending(Slot::Refresh).is_some(),
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

struct PendingRead {
	op: OpId<Slot>,
	inbox: mpsc::Sender<CounterEvent>,
}

impl PendingRead {
	/// Queues a reply; the runner processes it later.
	async fn send_reply(&self, value: u64) -> anyhow::Result<()> {
		self.inbox
			.send(CounterEvent::Loaded { op: self.op, value })
			.await?;

		Ok(())
	}
}

struct CounterExecutor {
	dispatched_reads: mpsc::Sender<PendingRead>,
}

impl Executor<Counter> for CounterExecutor {
	fn execute(&self, effect: ReadCounter, ctx: EffectContext<Counter>) {
		let inbox = ctx.inbox();
		let env = ctx.env();
		let dispatched_reads = self.dispatched_reads.clone();
		let read = PendingRead {
			op: effect.op,
			inbox: inbox.clone(),
		};

		env.spawn(Box::pin(async move {
			// Hand the read to the scenario in place of performing real I/O.
			let _ = dispatched_reads.send(read).await;
		}));
	}
}
