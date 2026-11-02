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
	/// Admission is closed; accepted generations must finish local teardown.
	DrainingActors { deadline: Instant },
	/// Every accepted generation finished locally; retain transport until event ACKs arrive.
	AcknowledgingEvents { deadline: Instant },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Completion {
	Acknowledged,
	DeadlineReached,
}

impl State {
	/// Returns true only for the first request. Later requests cannot extend the budget.
	pub fn begin(&mut self, now: Instant, budget: Duration) -> bool {
		if self.is_draining() {
			return false;
		}
		*self = Self::DrainingActors {
			deadline: now + budget,
		};
		true
	}

	pub fn is_draining(&self) -> bool {
		!matches!(self, Self::Running)
	}

	pub fn deadline(&self) -> Option<Instant> {
		match self {
			Self::Running => None,
			Self::DrainingActors { deadline } | Self::AcknowledgingEvents { deadline } => {
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
