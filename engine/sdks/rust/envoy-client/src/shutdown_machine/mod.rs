//! Shutdown state machine owned and driven by the Envoy loop.
//!
//! Admission closes at `begin`. The loop registers accepted generations before processing
//! their task messages and removes their unfinished marker only on local task completion.
//! The unfinished set survives routing retirement and includes failed startups.
//! Pending events remain replayable until acknowledged. Reconnects and repeated shutdown
//! requests preserve the original deadline; only the loop closes the transport.

use std::time::Duration;

use crate::time::Instant;

#[derive(Debug, Default)]
pub enum State {
	/// Actor starts may be admitted. No shutdown work is outstanding.
	#[default]
	Running,
	/// Admission is closed; accepted generations must finish local teardown. The engine stops
	/// them over its drain period. At `sweep_at`, if still set, the envoy asks every actor the
	/// engine has not stopped to sleep, so an engine that stopped driving the drain still leaves
	/// each actor time to stop before the deadline.
	DrainingActors {
		deadline: Instant,
		sweep_at: Option<Instant>,
	},
	/// Every accepted generation finished locally; retain transport until event ACKs arrive.
	AcknowledgingEvents { deadline: Instant },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Completion {
	Acknowledged,
	DeadlineReached,
}

impl State {
	/// Returns true only for the first request. Later requests cannot extend the budget or
	/// move the sweep. The sweep runs `sweep_after` from now, and the drain ends `budget` from
	/// now.
	pub fn begin(&mut self, now: Instant, sweep_after: Duration, budget: Duration) -> bool {
		if self.is_draining() {
			return false;
		}
		*self = Self::DrainingActors {
			deadline: now + budget,
			sweep_at: Some(now + sweep_after),
		};
		true
	}

	/// Whether the sweep is due at `now`. It is due once: a true result clears it.
	pub fn take_due_sweep(&mut self, now: Instant) -> bool {
		match self {
			Self::DrainingActors { sweep_at, .. } if sweep_at.is_some_and(|at| now >= at) => {
				*sweep_at = None;
				true
			}
			Self::Running | Self::DrainingActors { .. } | Self::AcknowledgingEvents { .. } => false,
		}
	}

	/// The next time the loop must wake for shutdown work: the sweep, or else the deadline.
	pub fn next_wake(&self) -> Option<Instant> {
		match self {
			Self::Running => None,
			Self::DrainingActors { deadline, sweep_at } => {
				Some(sweep_at.map_or(*deadline, |sweep_at| sweep_at.min(*deadline)))
			}
			Self::AcknowledgingEvents { deadline } => Some(*deadline),
		}
	}

	pub fn is_draining(&self) -> bool {
		!matches!(self, Self::Running)
	}

	pub fn deadline(&self) -> Option<Instant> {
		match self {
			Self::Running => None,
			Self::DrainingActors { deadline, .. } | Self::AcknowledgingEvents { deadline } => {
				Some(*deadline)
			}
		}
	}

	/// Reconcile after each inbox event and before admission of the next one. Counts come
	/// from the loop's unfinished-generation and pending-event sets. Channel closure
	/// and routing retirement are not teardown barriers.
	pub fn reconcile(
		&mut self,
		now: Instant,
		actors_pending: bool,
		events_pending: bool,
	) -> Option<Completion> {
		let deadline = self.deadline()?;
		if now >= deadline {
			return Some(Completion::DeadlineReached);
		}
		if actors_pending {
			return None;
		}
		*self = Self::AcknowledgingEvents { deadline };
		(!events_pending).then_some(Completion::Acknowledged)
	}
}
