use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg(feature = "wasm-runtime")]
use futures::channel::oneshot as futures_oneshot;
#[cfg(feature = "wasm-runtime")]
use futures::future::AbortHandle;
use parking_lot::Mutex;
use rivet_envoy_client::async_counter::AsyncCounter;
use tokio::sync::Notify;
use tokio::task::JoinSet;

use crate::actor::task_types::UserTaskKind;

/// Classifies actor work so sleep can apply one policy model across different APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorWorkKind {
	/// User work that keeps the actor out of idle sleep while it runs.
	KeepAwake,
	/// Runtime-owned work that should behave like keep-awake without exposing a user API.
	InternalKeepAwake,
	/// User work that may continue into sleep grace but should not block idle sleep.
	WaitUntil,
	/// Detached runtime task that drains during shutdown.
	RegisteredTask,
	/// Async WebSocket callback work that must hold sleep while a callback is running.
	WebSocketCallback,
	/// Disconnect callback work that must finish before sleep or destroy finalizes.
	DisconnectCallback,
}

/// Defines how a work kind participates in idle sleep and shutdown grace.
#[derive(Debug, Clone, Copy)]
pub struct ActorWorkPolicy {
	/// True when active work should prevent the actor from entering idle sleep.
	pub blocks_idle_sleep: bool,
	/// True when active work should delay sleep-grace runtime cleanup.
	pub drains_shutdown_grace: bool,
	/// True when detached work should be cancelled after the shutdown deadline.
	pub aborts_at_shutdown_deadline: bool,
	/// User-facing task kind used for metrics and lifecycle diagnostics.
	pub user_task_kind: Option<UserTaskKind>,
}

impl ActorWorkKind {
	/// Returns the lifecycle policy owned by this work kind.
	pub fn policy(self) -> ActorWorkPolicy {
		match self {
			ActorWorkKind::KeepAwake => ActorWorkPolicy {
				blocks_idle_sleep: true,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: false,
				user_task_kind: None,
			},
			ActorWorkKind::InternalKeepAwake => ActorWorkPolicy {
				blocks_idle_sleep: true,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: false,
				user_task_kind: None,
			},
			ActorWorkKind::WaitUntil => ActorWorkPolicy {
				blocks_idle_sleep: false,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: true,
				user_task_kind: Some(UserTaskKind::WaitUntil),
			},
			ActorWorkKind::RegisteredTask => ActorWorkPolicy {
				blocks_idle_sleep: false,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: true,
				user_task_kind: Some(UserTaskKind::RegisteredTask),
			},
			ActorWorkKind::WebSocketCallback => ActorWorkPolicy {
				blocks_idle_sleep: true,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: true,
				user_task_kind: Some(UserTaskKind::WebSocketCallback),
			},
			ActorWorkKind::DisconnectCallback => ActorWorkPolicy {
				blocks_idle_sleep: true,
				drains_shutdown_grace: true,
				aborts_at_shutdown_deadline: true,
				user_task_kind: Some(UserTaskKind::DisconnectCallback),
			},
		}
	}

	/// Returns a stable label for logs and metric fields.
	pub(crate) fn label(self) -> &'static str {
		match self {
			ActorWorkKind::KeepAwake => "keep_awake",
			ActorWorkKind::InternalKeepAwake => "internal_keep_awake",
			ActorWorkKind::WaitUntil => "wait_until",
			ActorWorkKind::RegisteredTask => "registered_task",
			ActorWorkKind::WebSocketCallback => "websocket_callback",
			ActorWorkKind::DisconnectCallback => "disconnect_callback",
		}
	}
}

/// Holds per-kind counters and task sets used by actor sleep and shutdown.
pub(crate) struct WorkRegistry {
	/// Counts user keep-awake regions that block idle sleep.
	pub(crate) keep_awake: Arc<AsyncCounter>,
	/// Counts runtime-owned keep-awake regions that block idle sleep.
	pub(crate) internal_keep_awake: Arc<AsyncCounter>,
	/// Counts async WebSocket callbacks currently running.
	pub(crate) websocket_callback: Arc<AsyncCounter>,
	/// Counts disconnect callbacks currently running.
	pub(crate) disconnect_callback: Arc<AsyncCounter>,
	/// Counts work that must drain before sleep-grace runtime cleanup.
	pub(crate) shutdown_counter: Arc<AsyncCounter>,
	/// Counts lifecycle hooks dispatched by core into the actor runtime.
	pub(crate) core_dispatched_hooks: Arc<AsyncCounter>,
	// Forced-sync: shutdown tasks are inserted from sync paths and moved out
	// before awaiting shutdown.
	/// Detached shutdown work that can be aborted during final teardown.
	pub(crate) shutdown_tasks: Mutex<JoinSet<()>>,
	/// Detached shutdown work that must be joined even after the grace deadline.
	pub(crate) unabortable_shutdown_tasks: Mutex<JoinSet<()>>,
	#[cfg(feature = "wasm-runtime")]
	/// Wasm-local shutdown tasks tracked by completion channel and abort handle.
	pub(crate) local_shutdown_tasks: Mutex<Vec<LocalShutdownTask>>,
	/// Wakes sleep waiters when core-owned idle blockers reach zero.
	pub(crate) idle_notify: Arc<Notify>,
	/// Woken on every transition of a sleep-affecting counter that is not
	/// otherwise guarded by `ActorWorkRegion`. In practice this covers
	/// externally-owned counters like the envoy HTTP request counter whose
	/// increments happen outside rivetkit-core.
	pub(crate) activity_notify: Arc<Notify>,
	/// Set once final teardown starts so new detached work is refused.
	pub(crate) teardown_started: AtomicBool,
	/// Set when the grace deadline has elapsed and abortable work should be cancelled.
	pub(crate) shutdown_deadline_reached: AtomicBool,
}

#[cfg(feature = "wasm-runtime")]
pub(crate) struct LocalShutdownTask {
	pub(crate) abort_handle: AbortHandle,
	pub(crate) complete_rx: futures_oneshot::Receiver<()>,
	pub(crate) aborts_at_shutdown_deadline: bool,
}

impl WorkRegistry {
	/// Creates an empty registry and wires idle notifications for idle-blocking counters.
	pub(crate) fn new() -> Self {
		let idle_notify = Arc::new(Notify::new());
		let keep_awake = Arc::new(AsyncCounter::new());
		keep_awake.register_zero_notify(&idle_notify);
		let internal_keep_awake = Arc::new(AsyncCounter::new());
		internal_keep_awake.register_zero_notify(&idle_notify);
		let websocket_callback = Arc::new(AsyncCounter::new());
		websocket_callback.register_zero_notify(&idle_notify);
		let disconnect_callback = Arc::new(AsyncCounter::new());
		disconnect_callback.register_zero_notify(&idle_notify);

		Self {
			keep_awake,
			internal_keep_awake,
			websocket_callback,
			disconnect_callback,
			shutdown_counter: Arc::new(AsyncCounter::new()),
			core_dispatched_hooks: Arc::new(AsyncCounter::new()),
			shutdown_tasks: Mutex::new(JoinSet::new()),
			unabortable_shutdown_tasks: Mutex::new(JoinSet::new()),
			#[cfg(feature = "wasm-runtime")]
			local_shutdown_tasks: Mutex::new(Vec::new()),
			idle_notify,
			activity_notify: Arc::new(Notify::new()),
			teardown_started: AtomicBool::new(false),
			shutdown_deadline_reached: AtomicBool::new(false),
		}
	}

	/// Starts a user keep-awake region.
	pub(crate) fn keep_awake_guard(&self) -> RegionGuard {
		RegionGuard::new(self.keep_awake.clone())
	}

	/// Starts a runtime-owned keep-awake region.
	pub(crate) fn internal_keep_awake_guard(&self) -> RegionGuard {
		RegionGuard::new(self.internal_keep_awake.clone())
	}

	/// Starts an async WebSocket callback region.
	pub(crate) fn websocket_callback_guard(&self) -> RegionGuard {
		RegionGuard::new(self.websocket_callback.clone())
	}

	/// Starts a disconnect callback region.
	pub(crate) fn disconnect_callback_guard(&self) -> RegionGuard {
		RegionGuard::new(self.disconnect_callback.clone())
	}
}

impl Default for WorkRegistry {
	fn default() -> Self {
		Self::new()
	}
}

/// RAII guard that decrements an actor work counter when dropped.
pub(crate) struct RegionGuard {
	counter: Arc<AsyncCounter>,
	log_kind: Option<&'static str>,
	log_actor_id: Option<String>,
}

impl RegionGuard {
	/// Increments a counter and returns a guard that will decrement it.
	fn new(counter: Arc<AsyncCounter>) -> Self {
		counter.increment();
		Self {
			counter,
			log_kind: None,
			log_actor_id: None,
		}
	}

	/// Wraps a counter that has already been incremented.
	pub(crate) fn from_incremented(counter: Arc<AsyncCounter>) -> Self {
		Self {
			counter,
			log_kind: None,
			log_actor_id: None,
		}
	}

	/// Enables paired debug logs for the lifetime of this guard.
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/work_registry.rs"]
mod tests;
