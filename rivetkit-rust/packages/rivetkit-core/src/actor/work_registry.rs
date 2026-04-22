use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use parking_lot::Mutex;
use rivet_util::async_counter::AsyncCounter;
use tokio::sync::Notify;
use tokio::task::JoinSet;

pub(crate) struct WorkRegistry {
	pub(crate) keep_awake: Arc<AsyncCounter>,
	pub(crate) internal_keep_awake: Arc<AsyncCounter>,
	pub(crate) websocket_callback: Arc<AsyncCounter>,
	pub(crate) shutdown_counter: Arc<AsyncCounter>,
	pub(crate) core_dispatched_hooks: Arc<AsyncCounter>,
	// Forced-sync: shutdown tasks are inserted from sync paths and moved out
	// before awaiting shutdown.
	pub(crate) shutdown_tasks: Mutex<JoinSet<()>>,
	pub(crate) idle_notify: Arc<Notify>,
	/// Woken on every transition of a sleep-affecting counter that is not
	/// otherwise guarded by `KeepAwakeGuard` / `WebSocketCallbackGuard` /
	/// `DisconnectCallbackGuard`. In practice this covers externally-owned
	/// counters like the envoy HTTP request counter whose increments happen
	/// outside rivetkit-core.
	pub(crate) activity_notify: Arc<Notify>,
	pub(crate) prevent_sleep_notify: Arc<Notify>,
	pub(crate) teardown_started: AtomicBool,
}

impl WorkRegistry {
	pub(crate) fn new() -> Self {
		let idle_notify = Arc::new(Notify::new());
		let keep_awake = Arc::new(AsyncCounter::new());
		keep_awake.register_zero_notify(&idle_notify);
		let internal_keep_awake = Arc::new(AsyncCounter::new());
		internal_keep_awake.register_zero_notify(&idle_notify);
		let websocket_callback = Arc::new(AsyncCounter::new());
		websocket_callback.register_zero_notify(&idle_notify);

		Self {
			keep_awake,
			internal_keep_awake,
			websocket_callback,
			shutdown_counter: Arc::new(AsyncCounter::new()),
			core_dispatched_hooks: Arc::new(AsyncCounter::new()),
			shutdown_tasks: Mutex::new(JoinSet::new()),
			idle_notify,
			activity_notify: Arc::new(Notify::new()),
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
	log_kind: Option<&'static str>,
	log_actor_id: Option<String>,
}

impl RegionGuard {
	fn new(counter: Arc<AsyncCounter>) -> Self {
		counter.increment();
		Self {
			counter,
			log_kind: None,
			log_actor_id: None,
		}
	}

	pub(crate) fn from_incremented(counter: Arc<AsyncCounter>) -> Self {
		Self {
			counter,
			log_kind: None,
			log_actor_id: None,
		}
	}

	pub(crate) fn with_log_fields(mut self, kind: &'static str, actor_id: Option<String>) -> Self {
		let count = self.counter.load();
		match actor_id.as_deref() {
			Some(actor_id) => tracing::debug!(actor_id, kind, count, "sleep keep-awake engaged"),
			None => tracing::debug!(kind, count, "sleep keep-awake engaged"),
		}
		self.log_kind = Some(kind);
		self.log_actor_id = actor_id;
		self
	}
}

impl Drop for RegionGuard {
	fn drop(&mut self) {
		self.counter.decrement();
		let Some(kind) = self.log_kind else {
			return;
		};
		let count = self.counter.load();
		match self.log_actor_id.as_deref() {
			Some(actor_id) => tracing::debug!(actor_id, kind, count, "sleep keep-awake disengaged"),
			None => tracing::debug!(kind, count, "sleep keep-awake disengaged"),
		}
	}
}

/// `CountGuard` is the same RAII shape as `RegionGuard`, but used for task-counting sites.
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

		assert!(
			result.is_err(),
			"panic should propagate through catch_unwind"
		);
		assert_eq!(work.keep_awake.load(), 0);
	}
}
