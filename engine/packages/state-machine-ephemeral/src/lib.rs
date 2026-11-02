//! Runs state machines whose execution state lives in memory.
//!
//! Events enter through an inbox or timers. Each transition publishes its snapshot
//! before dispatching effects. Effects may write durable data; recovery of the
//! execution state is the caller's responsibility.
//!
//! For transitions without this runtime, use `rivet-state-machine`. For work
//! recovered from persisted intent, use `rivet-durable-reconciler`.

pub mod state_machine;
pub use state_machine::{
	Effects, EphemeralStateMachine, EphemeralStateMachineEffect, EphemeralStateMachineEvent,
	NoTimer, RuntimeEffect, RuntimeEvent, StateMachineAdapter,
};

pub mod runner;
pub mod shared_timers;
pub use shared_timers::SharedTimers;

pub use runner::{
	BusyObserver, CompletionSender, EffectContext, Executor, PublishObserver, Runner, RunnerHandle,
	SnapshotObserver, effect_context,
};
