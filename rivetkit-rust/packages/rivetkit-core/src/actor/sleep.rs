use parking_lot::Mutex;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_util::async_counter::AsyncCounter;
use std::future::Future;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicUsize as TestAtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
#[cfg(test)]
use tokio::time::sleep_until;
use tokio::time::{Instant, sleep};
use tracing::Instrument;

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::work_registry::{CountGuard, RegionGuard, WorkRegistry};
#[cfg(test)]
use crate::types::ActorKey;

/// Per-actor sleep state.
///
/// `ActorContext::reset_sleep_timer()` is invoked on every mutation that changes
/// a sleep predicate input. Production actors wake the owning `ActorTask` via a
/// single `Notify`; contexts not wired to an `ActorTask` use the detached
/// compatibility timer below.
pub(crate) struct SleepState {
	// Forced-sync: sleep controller config/runtime handles are synchronous
	// wiring slots cloned before actor I/O.
	pub(super) config: Mutex<ActorConfig>,
	pub(super) envoy_handle: Mutex<Option<EnvoyHandle>>,
	pub(super) generation: Mutex<Option<u32>>,
	pub(super) http_request_counter: Mutex<Option<Arc<AsyncCounter>>>,
	#[cfg(test)]
	sleep_request_count: TestAtomicUsize,
	#[cfg(test)]
	destroy_request_count: TestAtomicUsize,
	pub(super) lifecycle_started: AtomicBool,
	pub(super) run_handler_active_count: AtomicUsize,
	// Forced-sync: the compatibility sleep timer is aborted from sync paths.
	pub(super) sleep_timer: Mutex<Option<JoinHandle<()>>>,
	pub(super) work: WorkRegistry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanSleep {
	Yes,
	NotReady,
	NoSleep,
	ActiveHttpRequests,
	ActiveKeepAwake,
	ActiveInternalKeepAwake,
	ActiveRunHandler,
	ActiveDisconnectCallbacks,
	ActiveConnections,
	ActiveWebSocketCallbacks,
}

impl SleepState {
	pub fn new(config: ActorConfig) -> Self {
		Self {
			config: Mutex::new(config),
			envoy_handle: Mutex::new(None),
			generation: Mutex::new(None),
			http_request_counter: Mutex::new(None),
			#[cfg(test)]
			sleep_request_count: TestAtomicUsize::new(0),
			#[cfg(test)]
			destroy_request_count: TestAtomicUsize::new(0),
			lifecycle_started: AtomicBool::new(false),
			run_handler_active_count: AtomicUsize::new(0),
			sleep_timer: Mutex::new(None),
			work: WorkRegistry::new(),
		}
	}
}

impl Default for SleepState {
	fn default() -> Self {
		Self::new(ActorConfig::default())
	}
}

impl std::fmt::Debug for SleepState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SleepState")
			.field(
				"lifecycle_started",
				&self.lifecycle_started.load(Ordering::SeqCst),
			)
			.field(
				"run_handler_active_count",
				&self.run_handler_active_count.load(Ordering::SeqCst),
			)
			.field("keep_awake_count", &self.work.keep_awake.load())
			.field(
				"internal_keep_awake_count",
				&self.work.internal_keep_awake.load(),
			)
			.field(
				"websocket_callback_count",
				&self.work.websocket_callback.load(),
			)
			.finish()
	}
}

impl ActorContext {
	#[cfg(test)]
	pub(crate) fn new_for_sleep_tests(actor_id: impl Into<String>) -> Self {
		Self::new(actor_id, "sleep-test", ActorKey::default(), "local")
	}

	pub(crate) fn configure_sleep_state(&self, config: ActorConfig) {
		*self.0.sleep.config.lock() = config;
	}

	pub(crate) fn configure_sleep_envoy(&self, envoy_handle: EnvoyHandle, generation: Option<u32>) {
		*self.0.sleep.envoy_handle.lock() = Some(envoy_handle);
		*self.0.sleep.generation.lock() = generation;
		*self.0.sleep.http_request_counter.lock() =
			self.lookup_http_request_counter(self.actor_id());
	}

	pub(crate) fn sleep_envoy_handle(&self) -> Option<EnvoyHandle> {
		self.0.sleep.envoy_handle.lock().clone()
	}

	pub(crate) fn sleep_generation(&self) -> Option<u32> {
		*self.0.sleep.generation.lock()
	}

	pub(crate) fn request_sleep_from_envoy(&self) {
		#[cfg(test)]
		self.0
			.sleep
			.sleep_request_count
			.fetch_add(1, Ordering::SeqCst);
		let envoy_handle = self.0.sleep.envoy_handle.lock().clone();
		let generation = *self.0.sleep.generation.lock();
		if let Some(envoy_handle) = envoy_handle {
			envoy_handle.sleep_actor(self.actor_id().to_owned(), generation);
		}
	}

	pub(crate) fn request_destroy_from_envoy(&self) {
		#[cfg(test)]
		self.0
			.sleep
			.destroy_request_count
			.fetch_add(1, Ordering::SeqCst);
		let envoy_handle = self.0.sleep.envoy_handle.lock().clone();
		let generation = *self.0.sleep.generation.lock();
		if let Some(envoy_handle) = envoy_handle {
			envoy_handle.destroy_actor(self.actor_id().to_owned(), generation);
		}
	}

	pub(crate) fn set_lifecycle_started(&self, started: bool) {
		let previous = self
			.0
			.sleep
			.lifecycle_started
			.swap(started, Ordering::SeqCst);
		if previous != started {
			self.reset_sleep_timer();
		}
	}

	pub(crate) fn lifecycle_started(&self) -> bool {
		self.0.sleep.lifecycle_started.load(Ordering::SeqCst)
	}

	#[doc(hidden)]
	pub fn begin_run_handler(&self) {
		let previous = self
			.0
			.sleep
			.run_handler_active_count
			.fetch_add(1, Ordering::SeqCst);
		if previous == 0 {
			self.reset_sleep_timer();
		}
	}

	#[doc(hidden)]
	pub fn end_run_handler(&self) {
		match self.0.sleep.run_handler_active_count.fetch_update(
			Ordering::SeqCst,
			Ordering::SeqCst,
			|count| count.checked_sub(1),
		) {
			Ok(1) => self.reset_sleep_timer(),
			Ok(_) => {}
			Err(_) => {
				tracing::warn!(
					actor_id = %self.actor_id(),
					"run handler active counter underflow"
				);
			}
		}
	}

	pub(crate) fn run_handler_active(&self) -> bool {
		self.0.sleep.run_handler_active_count.load(Ordering::SeqCst) > 0
	}

	#[cfg(test)]
	pub(crate) fn sleep_request_count(&self) -> usize {
		self.0.sleep.sleep_request_count.load(Ordering::SeqCst)
	}

	pub(crate) async fn can_arm_sleep_timer(&self) -> CanSleep {
		let config = self.sleep_state_config();
		if !self.0.sleep.lifecycle_started.load(Ordering::SeqCst) {
			return CanSleep::NotReady;
		}
		if config.no_sleep {
			return CanSleep::NoSleep;
		}
		if self.active_http_request_count() > 0 {
			return CanSleep::ActiveHttpRequests;
		}
		if self.sleep_keep_awake_count() > 0 {
			return CanSleep::ActiveKeepAwake;
		}
		if self.sleep_internal_keep_awake_count() > 0 {
			return CanSleep::ActiveInternalKeepAwake;
		}
		// Queue receives are sleep-compatible: sleep aborts the wait via the
		// actor abort token, then the next generation restarts the run loop.
		if self.run_handler_active() && self.active_queue_wait_count() == 0 {
			return CanSleep::ActiveRunHandler;
		}
		if self.pending_disconnect_count() > 0 {
			return CanSleep::ActiveDisconnectCallbacks;
		}
		if !self.conns().is_empty() {
			return CanSleep::ActiveConnections;
		}
		if self.websocket_callback_count() > 0 {
			return CanSleep::ActiveWebSocketCallbacks;
		}

		CanSleep::Yes
	}

	pub(crate) fn can_finalize_sleep(&self) -> bool {
		self.0.sleep.work.core_dispatched_hooks.load() == 0
			&& self.shutdown_task_count() == 0
			&& self.sleep_keep_awake_count() == 0
			&& self.sleep_internal_keep_awake_count() == 0
			&& self.active_http_request_count() == 0
			&& self.websocket_callback_count() == 0
			&& self.pending_disconnect_count() == 0
	}

	/// Spawn the fallback sleep timer used by `ActorContext`s that are not
	/// bound to an `ActorTask`.
	///
	/// This path only engages when `configure_lifecycle_events` has not been
	/// wired, which in practice means test contexts. Production actors built
	/// through the registry always have an `ActorTask` and never spawn this
	/// detached timer.
	pub(crate) fn reset_sleep_timer_state(&self) {
		self.cancel_sleep_timer();

		let Ok(runtime) = Handle::try_current() else {
			tracing::debug!(
				actor_id = %self.actor_id(),
				"sleep activity reset skipped without tokio runtime"
			);
			return;
		};

		tracing::debug!(
			actor_id = %self.actor_id(),
			sleep_timeout_ms = self.0.sleep.config.lock().sleep_timeout.as_millis() as u64,
			"sleep activity reset"
		);

		let ctx = self.clone();
		let task = runtime.spawn(async move {
			let can_sleep = ctx.can_sleep().await;
			if can_sleep != CanSleep::Yes {
				tracing::debug!(
					actor_id = %ctx.actor_id(),
					reason = ?can_sleep,
					"sleep idle timer skipped"
				);
				return;
			}

			let timeout = ctx.sleep_config().sleep_timeout;
			sleep(timeout).await;

			let can_sleep = ctx.can_sleep().await;
			if can_sleep == CanSleep::Yes {
				tracing::debug!(
					actor_id = %ctx.actor_id(),
					sleep_timeout_ms = timeout.as_millis() as u64,
					"sleep idle timer elapsed"
				);
				if let Err(err) = ctx.sleep() {
					tracing::debug!(
						actor_id = %ctx.actor_id(),
						?err,
						"sleep idle timer request suppressed"
					);
				}
			} else {
				tracing::warn!(
					actor_id = %ctx.actor_id(),
					reason = ?can_sleep,
					"sleep idle timer elapsed but actor stayed awake"
				);
			}
		});

		*self.0.sleep.sleep_timer.lock() = Some(task);
	}

	pub(crate) fn cancel_sleep_timer(&self) {
		let timer = self.0.sleep.sleep_timer.lock().take();
		if let Some(timer) = timer {
			timer.abort();
		}
	}

	pub(crate) async fn wait_for_internal_keep_awake_idle(&self, deadline: Instant) -> bool {
		self.0
			.sleep
			.work
			.internal_keep_awake
			.wait_zero(deadline)
			.await
	}

	#[cfg(test)]
	pub(crate) async fn wait_for_sleep_idle_window(&self, deadline: Instant) -> bool {
		loop {
			let activity = self.sleep_activity_notify();
			let activity_notified = activity.notified();
			tokio::pin!(activity_notified);
			activity_notified.as_mut().enable();
			let idle = self.0.sleep.work.idle_notify.notified();
			tokio::pin!(idle);
			idle.as_mut().enable();

			if self.can_finalize_sleep() {
				return true;
			}

			tokio::select! {
				_ = &mut activity_notified => {}
				_ = &mut idle => {}
				_ = sleep_until(deadline) => return false,
			}
		}
	}

	#[cfg(test)]
	pub(crate) async fn wait_for_shutdown_tasks(&self, deadline: Instant) -> bool {
		loop {
			let activity = self.sleep_activity_notify();
			let notified = activity.notified();
			tokio::pin!(notified);
			notified.as_mut().enable();

			let shutdown_count = self.shutdown_task_count();
			let websocket_count = self.websocket_callback_count();
			if shutdown_count == 0 && websocket_count == 0 {
				return true;
			}

			tokio::select! {
				drained = self.0.sleep.work.shutdown_counter.wait_zero(deadline), if shutdown_count > 0 => {
					if !drained {
						return false;
					}
				}
				drained = self.0.sleep.work.websocket_callback.wait_zero(deadline), if websocket_count > 0 => {
					if !drained {
						return false;
					}
				}
				_ = &mut notified => {}
				_ = sleep_until(deadline) => return false,
			}
		}
	}

	pub(crate) async fn wait_for_http_requests_drained(&self, deadline: Instant) -> bool {
		let Some(counter) = self.http_request_counter() else {
			return true;
		};
		counter.wait_zero(deadline).await
	}

	pub(crate) async fn wait_for_http_requests_idle(&self) {
		loop {
			let idle = self.0.sleep.work.idle_notify.notified();
			tokio::pin!(idle);
			idle.as_mut().enable();

			if self.active_http_request_count() == 0 {
				return;
			}

			idle.await;
		}
	}

	pub(crate) fn keep_awake_region(&self) -> RegionGuard {
		self.0.sleep.work.keep_awake_guard()
	}

	pub(crate) fn sleep_keep_awake_count(&self) -> usize {
		self.0.sleep.work.keep_awake.load()
	}

	pub(crate) fn internal_keep_awake_region(&self) -> RegionGuard {
		self.0.sleep.work.internal_keep_awake_guard()
	}

	pub(crate) fn sleep_internal_keep_awake_count(&self) -> usize {
		self.0.sleep.work.internal_keep_awake.load()
	}

	fn active_queue_wait_count(&self) -> usize {
		self.0.active_queue_wait_count.load(Ordering::SeqCst) as usize
	}

	pub(crate) fn websocket_callback_region_state(&self) -> RegionGuard {
		self.0.sleep.work.websocket_callback_guard()
	}

	pub(crate) fn websocket_callback_count(&self) -> usize {
		self.0.sleep.work.websocket_callback.load()
	}

	pub(crate) fn track_shutdown_task<F>(&self, fut: F) -> bool
	where
		F: Future<Output = ()> + Send + 'static,
	{
		if Handle::try_current().is_err() {
			tracing::warn!("shutdown task spawned without tokio runtime; running fallback");
			return false;
		}

		let mut shutdown_tasks = self.0.sleep.work.shutdown_tasks.lock();
		if self.0.sleep.work.teardown_started.load(Ordering::Acquire) {
			tracing::warn!("shutdown task spawned after teardown; aborting immediately");
			return false;
		}
		let counter = self.0.sleep.work.shutdown_counter.clone();
		counter.increment();
		let guard = CountGuard::from_incremented(counter);
		shutdown_tasks.spawn(
			async move {
				let _guard = guard;
				fut.await;
			}
			.in_current_span(),
		);
		true
	}

	pub(crate) fn shutdown_task_count(&self) -> usize {
		self.0.sleep.work.shutdown_counter.load()
	}

	pub(crate) fn begin_core_dispatched_hook(&self) {
		self.0.sleep.work.core_dispatched_hooks.increment();
		self.reset_sleep_timer();
	}

	pub fn mark_core_dispatched_hook_completed(&self) {
		self.0.sleep.work.core_dispatched_hooks.decrement();
		self.reset_sleep_timer();
	}

	pub(crate) fn core_dispatched_hook_count(&self) -> usize {
		self.0.sleep.work.core_dispatched_hooks.load()
	}

	pub(crate) async fn teardown_sleep_state(&self) {
		self.0
			.sleep
			.work
			.teardown_started
			.store(true, Ordering::Release);
		let mut shutdown_tasks = {
			let mut guard = self.0.sleep.work.shutdown_tasks.lock();
			std::mem::take(&mut *guard)
		};
		shutdown_tasks.shutdown().await;
		*self.0.sleep.work.shutdown_tasks.lock() = shutdown_tasks;
	}

	pub(crate) fn sleep_state_config(&self) -> ActorConfig {
		self.0.sleep.config.lock().clone()
	}

	pub(crate) fn active_http_request_count(&self) -> usize {
		self.http_request_counter()
			.map(|counter| counter.load())
			.unwrap_or(0)
	}

	pub(crate) fn sleep_activity_notify(&self) -> Arc<Notify> {
		self.0.sleep.work.activity_notify.clone()
	}

	fn http_request_counter(&self) -> Option<Arc<AsyncCounter>> {
		if let Some(counter) = self.0.sleep.http_request_counter.lock().clone() {
			return Some(counter);
		}

		let counter = self.lookup_http_request_counter(self.actor_id())?;
		*self.0.sleep.http_request_counter.lock() = Some(counter.clone());
		Some(counter)
	}

	fn lookup_http_request_counter(&self, actor_id: &str) -> Option<Arc<AsyncCounter>> {
		let envoy_handle = self.0.sleep.envoy_handle.lock().clone();
		let generation = *self.0.sleep.generation.lock();
		let envoy_handle = envoy_handle?;
		let counter = envoy_handle.http_request_counter(actor_id, generation)?;
		counter.register_zero_notify(&self.0.sleep.work.idle_notify);
		// The HTTP counter is owned by envoy-client, so neither increment nor
		// decrement goes through a rivetkit-core guard. Hook every transition
		// into the sleep activity notify so the sleep deadline gets
		// re-evaluated when a request starts or completes.
		let ctx = self.clone();
		counter.register_change_callback(Arc::new(move || {
			ctx.reset_sleep_timer();
		}));
		Some(counter)
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/sleep.rs"]
mod tests;
