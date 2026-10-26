use rivet_env::{Clocks, Env, clock::ClockError};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

/// Executor-only gate over the latest published state and a fresh clock observation.
/// Application wiring supplies runner-owned liveness; deterministic cores never hold this guard.
#[derive(Clone)]
pub struct DispatchGuard<S> {
	/// Injected clock, never stored in transition state.
	env: Arc<dyn Env>,

	/// Channel closure revokes dispatch even though the last value remains readable.
	snapshot: watch::Receiver<S>,

	/// Runner/application lifetime latch shared with teardown.
	alive: Arc<AtomicBool>,
}

impl<S> DispatchGuard<S> {
	/// Constructs a gate without starting tasks or observing time.
	pub fn new(env: Arc<dyn Env>, snapshot: watch::Receiver<S>, alive: Arc<AtomicBool>) -> Self {
		Self {
			env,
			snapshot,
			alive,
		}
	}

	/// Denies on stopped runner, closed channel, clock failure, or failed domain predicate.
	pub fn permits(&self, pred: impl Fn(&S, Clocks) -> bool) -> bool {
		if !self.alive.load(Ordering::SeqCst) || self.snapshot.has_changed().is_err() {
			return false;
		}
		self.clocks()
			.is_ok_and(|now| pred(&self.snapshot.borrow(), now))
	}

	/// Reads the injected clock; faults are explicit and never manufacture authority.
	pub fn clocks(&self) -> Result<Clocks, ClockError> {
		self.env.clock().observe()
	}

	/// Borrows the current coherent reader view; use permits for dispatch authority.
	pub fn snapshot(&self) -> watch::Ref<'_, S> {
		self.snapshot.borrow()
	}
}
