use rivet_state_machine::Effects;
use serde::{Deserialize, Serialize};

/// Local worker authority, latched off after expiry or an absent lease view.
/// Runners must deliver due alarms before inputs; snapshots still check a fresh clock.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeaseWatch<T> {
	/// Latest observed deadline; never grants permission after the fence latch is set.
	deadline_boot_ms: Option<u64>,
	/// Irreversible for this state machine lifetime, even after a delayed renewal.
	fenced: bool,
	/// Parent's worker lease alarm, distinct from registration and release alarms.
	timer: T,
}

/// Tells the parent whether to perform its domain-specific fencing reaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseChange {
	/// A future deadline was installed or replaced.
	Armed,

	/// Authority was revoked now; returned only once during this watch's lifetime.
	Fenced,

	/// No new authority or fencing action is required.
	Unchanged,
}

impl<T: Clone> LeaseWatch<T> {
	/// Begins unobserved and unable to serve, without fencing before the first view.
	pub fn new(timer: T) -> Self {
		Self {
			deadline_boot_ms: None,
			fenced: false,
			timer,
		}
	}

	/// Applies a fresh view, arming its deadline or permanently fencing absent/expired authority.
	pub fn observe<E>(&mut self, deadline: Option<u64>, out: &mut Effects<E, T>) -> LeaseChange {
		if self.fenced {
			return LeaseChange::Unchanged;
		}
		self.deadline_boot_ms = deadline;
		if let Some(deadline) = deadline.filter(|deadline| *deadline > out.now_boot_ms()) {
			out.arm_timer_at(self.timer.clone(), deadline);
			LeaseChange::Armed
		} else {
			self.fenced = true;
			out.cancel_timer(self.timer.clone());
			LeaseChange::Fenced
		}
	}

	/// Checks time while handling an alarm; obsolete or early alarms do not revoke authority.
	pub fn on_timer<E>(&mut self, out: &mut Effects<E, T>) -> LeaseChange {
		if !self.fenced
			&& self
				.deadline_boot_ms
				.is_some_and(|deadline| out.now_boot_ms() >= deadline)
		{
			self.fenced = true;
			out.cancel_timer(self.timer.clone());
			LeaseChange::Fenced
		} else {
			LeaseChange::Unchanged
		}
	}

	/// Transition-only authority flag; external readers must also check the current clock.
	pub fn live(&self) -> bool {
		!self.fenced && self.deadline_boot_ms.is_some()
	}

	/// Exposes the original deadline for fresh snapshot/executor admission checks.
	pub fn deadline_boot_ms(&self) -> Option<u64> {
		self.deadline_boot_ms
	}

	/// Reports the irreversible local revocation latch.
	pub fn fenced(&self) -> bool {
		self.fenced
	}
}
