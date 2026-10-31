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
	/// applied, and returns the preparation the transition receives. The machine is locked, so the
	/// state cannot change underneath it, but no transaction is open, so it may do I/O such
	/// as a read of another volume. An error fails the event without applying it; the
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
	/// event without persisting anything, like a failed `prepare`.
	fn write<'a>(
		&'a self,
		_effect: M::WriteEffect,
		_tx: &'a mut Transaction,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
		Box::pin(async { Ok(()) })
	}

	/// Runs after the transition commits. Spawn async work through `ctx.spawn` so its duration
	/// and result are recorded.
	fn execute(&self, effect: M::Effect, ctx: EffectContext);
}
