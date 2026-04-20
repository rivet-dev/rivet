use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_util::async_counter::AsyncCounter;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, sleep_until, timeout_at};

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::work_registry::{CountGuard, RegionGuard, WorkRegistry};

#[derive(Clone)]
pub struct SleepController(Arc<SleepControllerInner>);

struct SleepControllerInner {
	config: Mutex<ActorConfig>,
	envoy_handle: Mutex<Option<EnvoyHandle>>,
	generation: Mutex<Option<u32>>,
	http_request_counter: Mutex<Option<Arc<AsyncCounter>>>,
	#[cfg(test)]
	sleep_request_count: AtomicUsize,
	#[cfg(test)]
	destroy_request_count: AtomicUsize,
	ready: AtomicBool,
	started: AtomicBool,
	sleep_timer: Mutex<Option<JoinHandle<()>>>,
	work: WorkRegistry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanSleep {
	Yes,
	NotReady,
	PreventSleep,
	NoSleep,
	ActiveHttpRequests,
	ActiveKeepAwake,
	ActiveInternalKeepAwake,
	ActiveConnections,
	ActiveWebSocketCallbacks,
}

impl SleepController {
	pub fn new(config: ActorConfig) -> Self {
		Self(Arc::new(SleepControllerInner {
			config: Mutex::new(config),
			envoy_handle: Mutex::new(None),
			generation: Mutex::new(None),
			http_request_counter: Mutex::new(None),
			#[cfg(test)]
			sleep_request_count: AtomicUsize::new(0),
			#[cfg(test)]
			destroy_request_count: AtomicUsize::new(0),
			ready: AtomicBool::new(false),
			started: AtomicBool::new(false),
			sleep_timer: Mutex::new(None),
			work: WorkRegistry::new(),
		}))
	}

	pub(crate) fn configure(&self, config: ActorConfig) {
		*self.0.config.lock().expect("sleep config lock poisoned") = config;
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(
		&self,
		actor_id: &str,
		envoy_handle: EnvoyHandle,
		generation: Option<u32>,
	) {
		*self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned") = Some(envoy_handle);
		*self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned") = generation;
		*self
			.0
			.http_request_counter
			.lock()
			.expect("sleep http request counter lock poisoned") =
			self.lookup_http_request_counter(actor_id);
	}

	#[allow(dead_code)]
	pub(crate) fn clear_envoy(&self) {
		*self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned") = None;
		*self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned") = None;
		*self
			.0
			.http_request_counter
			.lock()
			.expect("sleep http request counter lock poisoned") = None;
	}

	pub(crate) fn envoy_handle(&self) -> Option<EnvoyHandle> {
		self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned")
			.clone()
	}

	pub(crate) fn generation(&self) -> Option<u32> {
		*self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned")
	}

	pub(crate) fn request_sleep(&self, actor_id: &str) {
		#[cfg(test)]
		self.0.sleep_request_count.fetch_add(1, Ordering::SeqCst);
		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned")
			.clone();
		let generation = *self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned");
		if let Some(envoy_handle) = envoy_handle {
			envoy_handle.sleep_actor(actor_id.to_owned(), generation);
		}
	}

	pub(crate) fn request_destroy(&self, actor_id: &str) {
		#[cfg(test)]
		self.0.destroy_request_count.fetch_add(1, Ordering::SeqCst);
		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned")
			.clone();
		let generation = *self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned");
		if let Some(envoy_handle) = envoy_handle {
			envoy_handle.destroy_actor(actor_id.to_owned(), generation);
		}
	}

	#[allow(dead_code)]
	pub(crate) fn set_ready(&self, ready: bool) {
		self.0.ready.store(ready, Ordering::SeqCst);
	}

	#[allow(dead_code)]
	pub(crate) fn ready(&self) -> bool {
		self.0.ready.load(Ordering::SeqCst)
	}

	#[allow(dead_code)]
	pub(crate) fn set_started(&self, started: bool) {
		self.0.started.store(started, Ordering::SeqCst);
	}

	#[allow(dead_code)]
	pub(crate) fn started(&self) -> bool {
		self.0.started.load(Ordering::SeqCst)
	}

	#[cfg(test)]
	pub(crate) fn sleep_request_count(&self) -> usize {
		self.0.sleep_request_count.load(Ordering::SeqCst)
	}

	pub(crate) async fn can_sleep(&self, ctx: &ActorContext) -> CanSleep {
		let config = self.config();
		if !self.0.ready.load(Ordering::SeqCst) || !self.0.started.load(Ordering::SeqCst) {
			return CanSleep::NotReady;
		}
		if ctx.prevent_sleep() {
			return CanSleep::PreventSleep;
		}
		if config.no_sleep {
			return CanSleep::NoSleep;
		}
		if self.active_http_request_count(ctx) > 0 {
			return CanSleep::ActiveHttpRequests;
		}
		if self.keep_awake_count() > 0 {
			return CanSleep::ActiveKeepAwake;
		}
		if self.internal_keep_awake_count() > 0 {
			return CanSleep::ActiveInternalKeepAwake;
		}
		if !ctx.conns().is_empty() {
			return CanSleep::ActiveConnections;
		}
		if self.websocket_callback_count() > 0 {
			return CanSleep::ActiveWebSocketCallbacks;
		}

		CanSleep::Yes
	}

	pub(crate) fn reset_sleep_timer(&self, ctx: ActorContext) {
		self.cancel_sleep_timer();

		let Ok(runtime) = Handle::try_current() else {
			return;
		};

		let controller = self.clone();
		// Intentionally detached compatibility timer for contexts that are not
		// wired to ActorTask. ActorTask-owned actors use lifecycle events and a
		// task-local sleep deadline instead.
		let task = runtime.spawn(async move {
			if controller.can_sleep(&ctx).await != CanSleep::Yes {
				return;
			}

			let timeout = controller.config().sleep_timeout;
			sleep(timeout).await;

			if controller.can_sleep(&ctx).await == CanSleep::Yes {
				ctx.sleep();
			}
		});

		*self
			.0
			.sleep_timer
			.lock()
			.expect("sleep timer lock poisoned") = Some(task);
	}

	pub(crate) fn cancel_sleep_timer(&self) {
		let timer = self
			.0
			.sleep_timer
			.lock()
			.expect("sleep timer lock poisoned")
			.take();
		if let Some(timer) = timer {
			timer.abort();
		}
	}

	pub(crate) async fn wait_for_sleep_idle_window(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		loop {
			let idle = self.0.work.idle_notify.notified();
			tokio::pin!(idle);
			idle.as_mut().enable();

			if self.sleep_shutdown_idle_ready(ctx) {
				return true;
			}

			if timeout_at(deadline, idle).await.is_err() {
				return false;
			}
		}
	}

	pub(crate) async fn wait_for_shutdown_tasks(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		loop {
			let prevent_sleep = self.0.work.prevent_sleep_notify.notified();
			tokio::pin!(prevent_sleep);
			prevent_sleep.as_mut().enable();

			let shutdown_count = self.shutdown_task_count();
			let websocket_count = self.websocket_callback_count();
			if shutdown_count == 0 && websocket_count == 0 && !ctx.prevent_sleep() {
				return true;
			}

			tokio::select! {
				drained = self.0.work.shutdown_counter.wait_zero(deadline), if shutdown_count > 0 => {
					if !drained {
						return false;
					}
				}
				drained = self.0.work.websocket_callback.wait_zero(deadline), if websocket_count > 0 => {
					if !drained {
						return false;
					}
				}
				_ = &mut prevent_sleep => {}
				_ = sleep_until(deadline) => return false,
			}
		}
	}

	pub(crate) async fn wait_for_internal_keep_awake_idle(
		&self,
		deadline: Instant,
	) -> bool {
		self.0.work.internal_keep_awake.wait_zero(deadline).await
	}

	pub(crate) async fn wait_for_http_requests_drained(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		let Some(counter) = self.http_request_counter(ctx) else {
			return true;
		};
		counter.wait_zero(deadline).await
	}

	pub(crate) fn keep_awake(&self) -> RegionGuard {
		self.0.work.keep_awake_guard()
	}

	pub(crate) fn keep_awake_count(&self) -> usize {
		self.0.work.keep_awake.load()
	}

	pub(crate) fn internal_keep_awake(&self) -> RegionGuard {
		self.0.work.internal_keep_awake_guard()
	}

	pub(crate) fn internal_keep_awake_count(&self) -> usize {
		self.0.work.internal_keep_awake.load()
	}

	pub(crate) fn websocket_callback(&self) -> RegionGuard {
		self.0.work.websocket_callback_guard()
	}

	fn websocket_callback_count(&self) -> usize {
		self.0.work.websocket_callback.load()
	}

	pub(crate) fn track_shutdown_task<F>(&self, fut: F)
	where
		F: Future<Output = ()> + Send + 'static,
	{
		let mut shutdown_tasks = self
			.0
			.work
			.shutdown_tasks
			.lock()
			.expect("shutdown tasks lock poisoned");
		if self.0.work.teardown_started.load(Ordering::Acquire) {
			tracing::warn!("shutdown task spawned after teardown; aborting immediately");
			return;
		}
		let counter = self.0.work.shutdown_counter.clone();
		counter.increment();
		let guard = CountGuard::from_incremented(counter);
		shutdown_tasks.spawn(async move {
			let _guard = guard;
			fut.await;
		});
	}

	#[allow(dead_code)]
	pub(crate) fn shutdown_task_count(&self) -> usize {
		self.0.work.shutdown_counter.load()
	}

	pub(crate) async fn teardown(&self) {
		self
			.0
			.work
			.teardown_started
			.store(true, Ordering::Release);
		let mut shutdown_tasks = {
			let mut guard = self
				.0
				.work
				.shutdown_tasks
				.lock()
				.expect("shutdown tasks lock poisoned");
			std::mem::take(&mut *guard)
		};
		shutdown_tasks.shutdown().await;
		*self
			.0
			.work
			.shutdown_tasks
			.lock()
			.expect("shutdown tasks lock poisoned") = shutdown_tasks;
	}

	fn sleep_shutdown_idle_ready(&self, ctx: &ActorContext) -> bool {
		self.active_http_request_count(ctx) == 0
			&& self.keep_awake_count() == 0
			&& self.internal_keep_awake_count() == 0
	}
	pub(crate) fn config(&self) -> ActorConfig {
		self.0
			.config
			.lock()
			.expect("sleep config lock poisoned")
			.clone()
	}

	fn active_http_request_count(&self, ctx: &ActorContext) -> usize {
		self
			.http_request_counter(ctx)
			.map(|counter| counter.load())
			.unwrap_or(0)
	}

	pub(crate) fn notify_prevent_sleep_changed(&self) {
		self.0.work.prevent_sleep_notify.notify_waiters();
	}

	fn http_request_counter(&self, ctx: &ActorContext) -> Option<Arc<AsyncCounter>> {
		if let Some(counter) = self
			.0
			.http_request_counter
			.lock()
			.expect("sleep http request counter lock poisoned")
			.clone()
		{
			return Some(counter);
		}

		let counter = self.lookup_http_request_counter(ctx.actor_id())?;
		*self
			.0
			.http_request_counter
			.lock()
			.expect("sleep http request counter lock poisoned") = Some(counter.clone());
		Some(counter)
	}

	fn lookup_http_request_counter(&self, actor_id: &str) -> Option<Arc<AsyncCounter>> {
		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned")
			.clone();
		let generation = *self
			.0
			.generation
			.lock()
			.expect("sleep generation lock poisoned");
		let envoy_handle = envoy_handle?;
		let counter = envoy_handle.http_request_counter(actor_id, generation)?;
		counter.register_zero_notify(&self.0.work.idle_notify);
		Some(counter)
	}
}

impl Default for SleepController {
	fn default() -> Self {
		Self::new(ActorConfig::default())
	}
}

impl std::fmt::Debug for SleepController {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SleepController")
			.field("ready", &self.0.ready.load(Ordering::SeqCst))
			.field("started", &self.0.started.load(Ordering::SeqCst))
			.field(
				"keep_awake_count",
				&self.keep_awake_count(),
			)
			.field(
				"internal_keep_awake_count",
				&self.internal_keep_awake_count(),
			)
			.field(
				"websocket_callback_count",
				&self.websocket_callback_count(),
			)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::Mutex as StdMutex;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use crate::actor::context::ActorContext;
	use crate::types::ActorKey;
	use rivet_util::async_counter::AsyncCounter;
	use tokio::sync::oneshot;
	use tokio::task::yield_now;
	use tokio::time::{Duration, Instant, advance};
	use tracing::{Event, Subscriber};
	use tracing::field::{Field, Visit};
	use tracing_subscriber::layer::{Context as LayerContext, Layer};
	use tracing_subscriber::prelude::*;
	use tracing_subscriber::registry::Registry;

	use super::SleepController;

	#[derive(Default)]
	struct MessageVisitor {
		message: Option<String>,
	}

	impl Visit for MessageVisitor {
		fn record_str(&mut self, field: &Field, value: &str) {
			if field.name() == "message" {
				self.message = Some(value.to_owned());
			}
		}

		fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
			if field.name() == "message" {
				self.message = Some(format!("{value:?}").trim_matches('"').to_owned());
			}
		}
	}

	#[derive(Clone)]
	struct ShutdownTaskRefusedLayer {
		count: Arc<AtomicUsize>,
	}

	impl<S> Layer<S> for ShutdownTaskRefusedLayer
	where
		S: Subscriber,
	{
		fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
			if *event.metadata().level() != tracing::Level::WARN {
				return;
			}

			let mut visitor = MessageVisitor::default();
			event.record(&mut visitor);
			if visitor.message.as_deref()
				== Some("shutdown task spawned after teardown; aborting immediately")
			{
				self.count.fetch_add(1, Ordering::SeqCst);
			}
		}
	}

	struct NotifyOnDrop(StdMutex<Option<oneshot::Sender<()>>>);

	impl NotifyOnDrop {
		fn new(sender: oneshot::Sender<()>) -> Self {
			Self(StdMutex::new(Some(sender)))
		}
	}

	impl Drop for NotifyOnDrop {
		fn drop(&mut self) {
			if let Some(sender) = self.0.lock().expect("drop notify lock poisoned").take() {
				let _ = sender.send(());
			}
		}
	}

	#[tokio::test(start_paused = true)]
	async fn shutdown_task_counter_reaches_zero_after_completion() {
		let controller = SleepController::default();
		let (done_tx, done_rx) = oneshot::channel();

		controller.track_shutdown_task(async move {
			let _ = done_tx.send(());
		});

		done_rx.await.expect("shutdown task should complete");
		yield_now().await;

		assert_eq!(controller.shutdown_task_count(), 0);
		assert!(
			controller
				.0
				.work
				.shutdown_counter
				.wait_zero(Instant::now() + Duration::from_millis(1))
				.await
		);
	}

	#[tokio::test(start_paused = true)]
	async fn shutdown_task_counter_reaches_zero_after_panic() {
		let controller = SleepController::default();

		controller.track_shutdown_task(async move {
			panic!("boom");
		});

		yield_now().await;
		yield_now().await;

		assert_eq!(controller.shutdown_task_count(), 0);
		assert!(
			controller
				.0
				.work
				.shutdown_counter
				.wait_zero(Instant::now() + Duration::from_millis(1))
				.await
		);
	}

	#[tokio::test(start_paused = true)]
	async fn teardown_aborts_tracked_shutdown_tasks() {
		let controller = SleepController::default();
		let (drop_tx, drop_rx) = oneshot::channel();
		let (_never_tx, never_rx) = oneshot::channel::<()>();
		let notify = NotifyOnDrop::new(drop_tx);

		controller.track_shutdown_task(async move {
			let _notify = notify;
			let _ = never_rx.await;
		});

		assert_eq!(controller.shutdown_task_count(), 1);

		controller.teardown().await;
		advance(Duration::from_millis(1)).await;

		drop_rx.await.expect("teardown should abort the tracked task");
		assert_eq!(controller.shutdown_task_count(), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn track_shutdown_task_refuses_spawns_after_teardown() {
		let controller = SleepController::default();
		let warning_count = Arc::new(AtomicUsize::new(0));
		let subscriber = Registry::default().with(ShutdownTaskRefusedLayer {
			count: warning_count.clone(),
		});
		let _guard = tracing::subscriber::set_default(subscriber);

		controller.teardown().await;
		controller.track_shutdown_task(async move {
			panic!("post-teardown shutdown task should never spawn");
		});
		yield_now().await;

		assert_eq!(controller.shutdown_task_count(), 0);
		assert_eq!(warning_count.load(Ordering::SeqCst), 1);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_idle_window_without_work_returns_next_tick() {
		let controller = SleepController::default();
		let ctx = ActorContext::new(
			"actor-sleep-idle",
			"sleep-idle",
			ActorKey::default(),
			"local",
		);

		let waiter = tokio::spawn({
			let controller = controller.clone();
			let ctx = ctx.clone();
			async move {
				controller
					.wait_for_sleep_idle_window(&ctx, Instant::now() + Duration::from_secs(1))
					.await
			}
		});

		yield_now().await;

		assert!(waiter.is_finished(), "idle wait should not poll in 10ms slices");
		assert!(waiter.await.expect("idle waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_idle_window_waits_for_http_counter_zero_transition() {
		let controller = SleepController::default();
		let ctx = ActorContext::new(
			"actor-http-idle",
			"http-idle",
			ActorKey::default(),
			"local",
		);
		let counter = Arc::new(AsyncCounter::new());
		counter.register_zero_notify(&controller.0.work.idle_notify);
		*controller
			.0
			.http_request_counter
			.lock()
			.expect("sleep http request counter lock poisoned") = Some(counter.clone());

		counter.increment();
		let waiter = tokio::spawn({
			let controller = controller.clone();
			let ctx = ctx.clone();
			async move {
				controller
					.wait_for_sleep_idle_window(&ctx, Instant::now() + Duration::from_secs(1))
					.await
			}
		});

		yield_now().await;
		assert!(
			!waiter.is_finished(),
			"http request drain should stay blocked while the counter is non-zero"
		);

		counter.decrement();
		yield_now().await;

		assert!(waiter.is_finished(), "idle wait should resume on the next scheduler tick");
		assert!(waiter.await.expect("http idle waiter should join"));
	}
}
