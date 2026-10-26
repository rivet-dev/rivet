use rivet_env::Clocks;
use std::ops::{Deref, DerefMut};

/// Effects emitted by a durable transition. Derefs to the standard [`rivet_state_machine::Effects`]
/// and additionally collects nondurable executions and write effects in separate lists.
pub struct Effects<E, T, W = ()> {
	inner: rivet_state_machine::Effects<E, T>,
	nondurable: Vec<E>,
	writes: Vec<W>,
}

impl<E, T, W> Effects<E, T, W> {
	/// Uses the event's clock snapshot without reading a wall clock.
	pub fn new(clocks: Clocks) -> Self {
		Self {
			inner: rivet_state_machine::Effects::new(clocks),
			nondurable: Vec::new(),
			writes: Vec::new(),
		}
	}

	/// Queues a write the executor applies inside the transition's own transaction, so it is
	/// durable exactly when the transition is: with the state, or not at all. Writes are
	/// applied in insertion order before the transaction commits.
	pub fn write(&mut self, effect: W) {
		self.writes.push(effect);
	}

	pub fn writes(&self) -> &[W] {
		&self.writes
	}

	/// Called by the runtime when it applies the transition's transaction.
	pub fn drain_writes(&mut self) -> Vec<W> {
		std::mem::take(&mut self.writes)
	}

	/// Queues executor work like [`rivet_state_machine::Effects::execute`], but in the
	/// nondurable list, preserving insertion order.
	pub fn nondurable_execute(&mut self, effect: E) {
		self.nondurable.push(effect);
	}

	pub fn nondurable(&self) -> &[E] {
		&self.nondurable
	}

	/// Called by the runtime after a transition has finished publishing its state.
	pub fn drain_nondurable(&mut self) -> Vec<E> {
		std::mem::take(&mut self.nondurable)
	}
}

impl<E, T, W> Deref for Effects<E, T, W> {
	type Target = rivet_state_machine::Effects<E, T>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<E, T, W> DerefMut for Effects<E, T, W> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
