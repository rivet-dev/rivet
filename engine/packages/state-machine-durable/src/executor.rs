//! The executor a durable runner hands its effects to: `prepare` before a transition, `write`
//! inside its transaction, and `execute` after it commits.
use crate::{DurableStateMachine, RuntimeEvent};
use anyhow::Result;
use depot_worker::command::Transaction;
use std::{future::Future, pin::Pin};

/// The `Env` and telemetry record for one effect. A durable runner has no inbox: completions
/// re-enter the state machine as new commands.
pub type EffectContext = rivet_state_machine::EffectContext<()>;

pub trait Executor<M: DurableStateMachine>: Send + Sync + 'static {
	/// Runs before each transition with the machine's current state and the event about to be
	/// applied, and returns the preparation the transition receives. The machine is locked
	/// against other handlers, but an earlier uncertain commit may still land. The runner then
	/// discards the plan and calls `prepare` again against fresh state, so preparation must be
	/// safe to repeat. No transaction is open; it may do I/O such as a read of another volume.
	/// An error fails the event without applying it; the
	/// caller's retry owner, or the timer that caused it, tries again. Bound any I/O with a
	/// deadline: every event of the machine waits behind it.
	fn prepare<'a>(
		&'a self,
		_state: &'a M::State,
		_event: &'a RuntimeEvent<M::Event, M::Timer>,
	) -> Pin<Box<dyn Future<Output = Result<M::Preparation>> + Send + 'a>> {
		Box::pin(async { Ok(M::Preparation::default()) })
	}

	/// Applies a write effect inside the transition's transaction, before it commits, so the
	/// write is durable exactly when the state is. Only local operations on `tx` belong here:
	/// no network, no other volume, nothing the transaction cannot carry. An error fails the
	/// event without persisting anything, like a failed `prepare`. `version` is the version
	/// of `M::FAMILY` the runner encodes this batch's state at; a value of that family
	/// written here is encoded at it too. The runner raises a tablet version mark only for the
	/// state and armed timers, not for writes: a write that `version` cannot express fails the
	/// batch with the converter's untyped error instead of raising a mark. A machine whose state
	/// an older version expresses but whose writes it does not must raise its own mark before
	/// relying on this.
	fn write<'a>(
		&'a self,
		_effect: M::WriteEffect,
		_tx: &'a mut Transaction,
		_version: u16,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
		Box::pin(async { Ok(()) })
	}

	/// Runs after the transition commits. Spawn async work through `ctx.spawn` so its duration
	/// and result are recorded.
	fn execute(&self, effect: M::Effect, ctx: EffectContext);
}
