use super::super::ReconcileConfig;
use super::types::Slot;
use rivet_state_machine::OpId;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Exclusive execution state of one generic reconciliation runner; durable intent stays in its store.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcilerState<E> {
	/// Owns pending work and remaining effects; the runner owns alarm deadlines.
	pub(super) phase: ReconcilerPhase<E>,
	/// Monotonic correlation allocation, retained even when no call is pending.
	pub(super) next_operation_index: u32,
	/// Delay after one load/execute pass before rereading authoritative intent.
	pub(super) poll_ms: u64,
	/// Bound on an individual effect; loading retains its existing separate cancellation semantics.
	pub(super) timeout_ms: u64,
	/// A wake arrived during a pass that may have loaded the record before the write,
	/// so the next pass starts as soon as this one ends instead of after `poll_ms`.
	pub(super) wake_pending: bool,
}
/// Mutually exclusive phases, with required payloads stored only where they are meaningful.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ReconcilerPhase<E> {
	/// Constructing a runner does not start storage work.
	NotStarted,
	/// Awaiting the exact load response; loading has no effect-execution deadline.
	Loading { op: OpId<Slot> },
	/// One effect is outstanding; later effects remain ordered behind it.
	Executing {
		op: OpId<Slot>,
		remaining: VecDeque<E>,
	},
	/// No work is outstanding until this local poll deadline.
	WaitingToPoll,
	/// No queue or call remains owned by the core; cancellation is not remote rollback.
	Stopped,
}
impl<E> ReconcilerState<E> {
	/// Validate intervals without starting tasks or reading storage.
	pub fn new(config: ReconcileConfig) -> anyhow::Result<Self> {
		let poll_ms = config.poll_interval.as_millis().try_into()?;
		let timeout_ms = config.effect_timeout.as_millis().try_into()?;
		anyhow::ensure!(
			poll_ms > 0 && timeout_ms > 0,
			"reconciliation intervals must be at least one millisecond"
		);
		Ok(Self {
			phase: ReconcilerPhase::NotStarted,
			next_operation_index: 0,
			poll_ms,
			timeout_ms,
			wake_pending: false,
		})
	}
	/// The phase is the only pending-operation authority; this is a read-only projection.
	pub(super) fn pending(&self) -> Option<OpId<Slot>> {
		match &self.phase {
			ReconcilerPhase::Loading { op } => Some(*op),
			ReconcilerPhase::Executing { op, .. } => Some(*op),
			_ => None,
		}
	}
	pub(super) fn allocate_operation(&mut self) -> OpId<Slot> {
		self.next_operation_index = self
			.next_operation_index
			.checked_add(1)
			.expect("operation index exhausted");
		OpId {
			slot: Slot::Work,
			index: self.next_operation_index,
		}
	}
}
impl<
	E: Clone + std::fmt::Debug + std::hash::Hash + Eq + Serialize + serde::de::DeserializeOwned + Send,
> rivet_state_machine::StateMachineState for ReconcilerState<E>
{
}
