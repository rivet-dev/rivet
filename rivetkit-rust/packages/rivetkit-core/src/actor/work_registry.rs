use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use rivet_util::async_counter::AsyncCounter;
use tokio::sync::Notify;
use tokio::task::JoinSet;

#[allow(dead_code)]
pub(crate) struct WorkRegistry {
	pub(crate) keep_awake: Arc<AsyncCounter>,
	pub(crate) internal_keep_awake: Arc<AsyncCounter>,
	pub(crate) websocket_callback: Arc<AsyncCounter>,
	pub(crate) shutdown_counter: Arc<AsyncCounter>,
	pub(crate) shutdown_tasks: Mutex<JoinSet<()>>,
	pub(crate) idle_notify: Arc<Notify>,
	pub(crate) prevent_sleep_notify: Arc<Notify>,
	pub(crate) teardown_started: AtomicBool,
}

#[allow(dead_code)]
impl WorkRegistry {
	pub(crate) fn new() -> Self {
		let idle_notify = Arc::new(Notify::new());
		let keep_awake = Arc::new(AsyncCounter::new());
		keep_awake.register_zero_notify(&idle_notify);
		let internal_keep_awake = Arc::new(AsyncCounter::new());
		internal_keep_awake.register_zero_notify(&idle_notify);

		Self {
			keep_awake,
			internal_keep_awake,
			websocket_callback: Arc::new(AsyncCounter::new()),
			shutdown_counter: Arc::new(AsyncCounter::new()),
			shutdown_tasks: Mutex::new(JoinSet::new()),
			idle_notify,
			prevent_sleep_notify: Arc::new(Notify::new()),
			teardown_started: AtomicBool::new(false),
		}
	}

	pub(crate) fn keep_awake_guard(&self) -> RegionGuard {
		RegionGuard::new(self.keep_awake.clone())
	}

	pub(crate) fn internal_keep_awake_guard(&self) -> RegionGuard {
		RegionGuard::new(self.internal_keep_awake.clone())
	}

	pub(crate) fn websocket_callback_guard(&self) -> RegionGuard {
		RegionGuard::new(self.websocket_callback.clone())
	}
}

impl Default for WorkRegistry {
	fn default() -> Self {
		Self::new()
	}
}

pub(crate) struct RegionGuard {
	counter: Arc<AsyncCounter>,
}

impl RegionGuard {
	fn new(counter: Arc<AsyncCounter>) -> Self {
		counter.increment();
		Self { counter }
	}

	pub(crate) fn from_incremented(counter: Arc<AsyncCounter>) -> Self {
		Self { counter }
	}
}

impl Drop for RegionGuard {
	fn drop(&mut self) {
		self.counter.decrement();
	}
}

/// `CountGuard` is the same RAII shape as `RegionGuard`, but used for task-counting sites.
#[allow(dead_code)]
pub(crate) type CountGuard = RegionGuard;

#[cfg(test)]
mod tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};

	use super::WorkRegistry;

	#[test]
	fn region_guard_drop_decrements_counter() {
		let work = WorkRegistry::new();
		assert_eq!(work.keep_awake.load(), 0);

		{
			let _guard = work.keep_awake_guard();
			assert_eq!(work.keep_awake.load(), 1);
		}

		assert_eq!(work.keep_awake.load(), 0);
	}

	#[test]
	fn region_guard_drop_during_panic_unwind_decrements_counter() {
		let work = WorkRegistry::new();

		let result = catch_unwind(AssertUnwindSafe(|| {
			let _guard = work.keep_awake_guard();
			assert_eq!(work.keep_awake.load(), 1);
			panic!("boom");
		}));

		assert!(result.is_err(), "panic should propagate through catch_unwind");
		assert_eq!(work.keep_awake.load(), 0);
	}
}
