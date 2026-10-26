//! Pure state machine transitions, operation tracking, and trace fingerprints.
//!
//! Use `rivet-state-machine-ephemeral` to run transitions with an inbox, timers,
//! snapshots, and an executor. Use `rivet-durable-reconciler` to derive work from
//! persisted intent and resume it after a crash.
//!
//! The driver supplies one [`Effects`] builder to each transition. Its clock snapshot
//! anchors relative timers; the driver publishes the snapshot before draining work.
//!
//! See `examples/counter_core.rs` for driving transitions directly.

pub mod state_machine;
pub use state_machine::*;

pub mod trace;
pub use trace::TraceHasher;

pub mod deadline_queue;
pub use deadline_queue::{DeadlineQueue, IndexedDeadlineQueue};

pub mod effects;
pub use effects::{Effects, RuntimeEffect};

pub mod telemetry;
pub use telemetry::{EffectCompletion, EffectContext, EffectOutcome, RunnerMetrics, RunnerPhase};
