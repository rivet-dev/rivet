//! Ordered transition output owned by the runtime and anchored to one clock snapshot.

use crate::Effect;
use rivet_env::Clocks;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fmt::Debug, hash::Hash};

/// One transition's ordered work. Only the runtime creates and drains the builder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Effects<E, T> {
	#[serde(skip)]
	clocks: Clocks,
	items: Vec<RuntimeEffect<E, T>>,
}

impl<E, T> Effects<E, T> {
	/// Uses the event's clock snapshot without reading a wall clock.
	pub fn new(clocks: Clocks) -> Self {
		Self {
			clocks,
			items: Vec::new(),
		}
	}

	/// Supplies the same clock snapshot when embedding a child state machine.
	pub fn clocks(&self) -> Clocks {
		self.clocks
	}

	pub fn now_boot_ms(&self) -> u64 {
		self.clocks.boot_ms
	}

	/// Arms or replaces a named timer at an absolute boot-clock deadline.
	pub fn arm_timer_at(&mut self, timer: T, at_boot_ms: u64) {
		self.items
			.push(RuntimeEffect::ArmTimer { timer, at_boot_ms });
	}

	/// Arms a timer relative to the event's boot clock, saturating on overflow.
	pub fn arm_timer_after(&mut self, timer: T, delay_ms: u64) {
		self.arm_timer_at(timer, self.now_boot_ms().saturating_add(delay_ms));
	}

	/// Cancels the named timer without recalling external work.
	pub fn cancel_timer(&mut self, timer: T) {
		self.items.push(RuntimeEffect::CancelTimer(timer));
	}

	/// Queues executor work after snapshot publication, preserving insertion order.
	pub fn execute(&mut self, effect: E) {
		self.items.push(RuntimeEffect::Execute(effect));
	}

	/// Requests fail-stop after all preceding effects.
	pub fn exit(&mut self) {
		self.items.push(RuntimeEffect::Exit);
	}

	/// Maps a child's effects without changing their order or timer deadlines.
	pub fn extend_mapped<E2, T2>(
		&mut self,
		child: Effects<E2, T2>,
		timer: impl Fn(T2) -> T,
		mut effect: impl FnMut(E2) -> E,
	) {
		self.items
			.extend(child.items.into_iter().map(|item| match item {
				RuntimeEffect::Execute(value) => RuntimeEffect::Execute(effect(value)),
				RuntimeEffect::ArmTimer {
					timer: value,
					at_boot_ms,
				} => RuntimeEffect::ArmTimer {
					timer: timer(value),
					at_boot_ms,
				},
				RuntimeEffect::CancelTimer(value) => RuntimeEffect::CancelTimer(timer(value)),
				RuntimeEffect::Exit => RuntimeEffect::Exit,
			}));
	}

	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	pub fn len(&self) -> usize {
		self.items.len()
	}

	pub fn as_slice(&self) -> &[RuntimeEffect<E, T>] {
		&self.items
	}

	/// Called by the runtime after a transition has finished publishing its state.
	pub fn drain(&mut self) -> Vec<RuntimeEffect<E, T>> {
		std::mem::take(&mut self.items)
	}
}

/// Effects understood by the standard runner. Only `Execute` reaches the executor.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEffect<E, T> {
	Execute(E),
	/// Replaces any timer with the same identity. The deadline is absolute `boot_ms`.
	/// Before delivering an input, the runner drains due alarms in (deadline, timer)
	/// order, applying each transition's effects before choosing the next alarm.
	ArmTimer {
		timer: T,
		at_boot_ms: u64,
	},
	/// Does not invalidate an operation or cancel external work.
	CancelTimer(T),
	/// Terminates the process/node and skips later effects, without flushing or cleanup.
	/// The runner uses exit status 1. Finish graceful cleanup before emitting this effect.
	Exit,
}

impl<E: Effect, T> Effect for RuntimeEffect<E, T>
where
	T: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send,
{
	fn name(&self) -> &'static str {
		match self {
			RuntimeEffect::Execute(effect) => effect.name(),
			RuntimeEffect::ArmTimer { .. } => "arm_timer",
			RuntimeEffect::CancelTimer(_) => "cancel_timer",
			RuntimeEffect::Exit => "exit",
		}
	}
}
