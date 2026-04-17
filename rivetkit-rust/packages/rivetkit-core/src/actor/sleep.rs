use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::task::yield_now;
use tokio::time::{Instant, sleep, timeout};

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;

#[derive(Clone)]
pub struct SleepController(Arc<SleepControllerInner>);

struct SleepControllerInner {
	config: Mutex<ActorConfig>,
	envoy_handle: Mutex<Option<EnvoyHandle>>,
	generation: Mutex<Option<u32>>,
	ready: AtomicBool,
	started: AtomicBool,
	run_handler_active: AtomicBool,
	keep_awake_count: AtomicU32,
	internal_keep_awake_count: AtomicU32,
	websocket_callback_count: AtomicU32,
	pending_disconnect_count: AtomicU32,
	sleep_timer: Mutex<Option<JoinHandle<()>>>,
	run_handler: Mutex<Option<JoinHandle<()>>>,
	shutdown_tasks: Mutex<Vec<JoinHandle<()>>>,
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
	ActiveRun,
	ActiveConnections,
	PendingDisconnectCallbacks,
	ActiveWebSocketCallbacks,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum AsyncRegion {
	KeepAwake,
	InternalKeepAwake,
	WebSocketCallbacks,
	PendingDisconnectCallbacks,
}

impl SleepController {
	pub fn new(config: ActorConfig) -> Self {
		Self(Arc::new(SleepControllerInner {
			config: Mutex::new(config),
			envoy_handle: Mutex::new(None),
			generation: Mutex::new(None),
			ready: AtomicBool::new(false),
			started: AtomicBool::new(false),
			run_handler_active: AtomicBool::new(false),
			keep_awake_count: AtomicU32::new(0),
			internal_keep_awake_count: AtomicU32::new(0),
			websocket_callback_count: AtomicU32::new(0),
			pending_disconnect_count: AtomicU32::new(0),
			sleep_timer: Mutex::new(None),
			run_handler: Mutex::new(None),
			shutdown_tasks: Mutex::new(Vec::new()),
		}))
	}

	pub(crate) fn configure(&self, config: ActorConfig) {
		*self.0.config.lock().expect("sleep config lock poisoned") = config;
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(&self, envoy_handle: EnvoyHandle, generation: Option<u32>) {
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
	}

	pub(crate) fn envoy_handle(&self) -> Option<EnvoyHandle> {
		self.0
			.envoy_handle
			.lock()
			.expect("sleep envoy handle lock poisoned")
			.clone()
	}

	pub(crate) fn request_sleep(&self, actor_id: &str) {
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

	#[allow(dead_code)]
	pub(crate) fn set_run_handler_active(&self, active: bool) {
		self.0.run_handler_active.store(active, Ordering::SeqCst);
	}

	pub(crate) fn run_handler_active(&self) -> bool {
		self.0.run_handler_active.load(Ordering::SeqCst)
	}

	pub(crate) fn track_run_handler(&self, handle: JoinHandle<()>) {
		let existing = self
			.0
			.run_handler
			.lock()
			.expect("run handler lock poisoned")
			.replace(handle);
		if let Some(existing) = existing {
			existing.abort();
		}
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
		if self.active_http_request_count(ctx).await > 0 {
			return CanSleep::ActiveHttpRequests;
		}
		if self.0.keep_awake_count.load(Ordering::SeqCst) > 0 {
			return CanSleep::ActiveKeepAwake;
		}
		if self.0.internal_keep_awake_count.load(Ordering::SeqCst) > 0 {
			return CanSleep::ActiveInternalKeepAwake;
		}
		if self.0.run_handler_active.load(Ordering::SeqCst)
			&& ctx.queue().active_queue_wait_count() == 0
		{
			return CanSleep::ActiveRun;
		}
		if !ctx.conns().is_empty() {
			return CanSleep::ActiveConnections;
		}
		if self.0.pending_disconnect_count.load(Ordering::SeqCst) > 0 {
			return CanSleep::PendingDisconnectCallbacks;
		}
		if self.0.websocket_callback_count.load(Ordering::SeqCst) > 0 {
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

	pub(crate) async fn wait_for_run_handler(&self, timeout_duration: Duration) -> bool {
		let Some(mut handle) = self
			.0
			.run_handler
			.lock()
			.expect("run handler lock poisoned")
			.take()
		else {
			self.0.run_handler_active.store(false, Ordering::SeqCst);
			return true;
		};

		let finished = match timeout(timeout_duration, &mut handle).await {
			Ok(Ok(())) => true,
			Ok(Err(error)) => {
				tracing::warn!(?error, "actor run handler join failed during shutdown");
				true
			}
			Err(_) => {
				tracing::warn!(
					timeout_ms = timeout_duration.as_millis() as u64,
					"actor run handler timed out during shutdown"
				);
				handle.abort();
				let _ = handle.await;
				false
			}
		};

		self.0.run_handler_active.store(false, Ordering::SeqCst);
		finished
	}

	pub(crate) async fn wait_for_sleep_idle_window(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		loop {
			if self.sleep_shutdown_idle_ready(ctx).await {
				return true;
			}

			let now = Instant::now();
			if now >= deadline {
				return false;
			}

			let sleep_for = (deadline - now).min(Duration::from_millis(10));
			sleep(sleep_for).await;
		}
	}

	pub(crate) async fn wait_for_shutdown_tasks(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		loop {
			if self.shutdown_tasks_drained(ctx) {
				yield_now().await;
				if self.shutdown_tasks_drained(ctx) {
					return true;
				}
			}

			let now = Instant::now();
			if now >= deadline {
				return false;
			}

			let sleep_for = (deadline - now).min(Duration::from_millis(10));
			sleep(sleep_for).await;
		}
	}

	pub(crate) async fn wait_for_internal_keep_awake_idle(&self, deadline: Instant) -> bool {
		loop {
			if self.0.internal_keep_awake_count.load(Ordering::SeqCst) == 0 {
				yield_now().await;
				if self.0.internal_keep_awake_count.load(Ordering::SeqCst) == 0 {
					return true;
				}
			}

			let now = Instant::now();
			if now >= deadline {
				return false;
			}

			let sleep_for = (deadline - now).min(Duration::from_millis(10));
			sleep(sleep_for).await;
		}
	}

	pub(crate) async fn wait_for_http_requests_drained(
		&self,
		ctx: &ActorContext,
		deadline: Instant,
	) -> bool {
		loop {
			if self.active_http_request_count(ctx).await == 0 {
				yield_now().await;
				if self.active_http_request_count(ctx).await == 0 {
					return true;
				}
			}

			let now = Instant::now();
			if now >= deadline {
				return false;
			}

			let sleep_for = (deadline - now).min(Duration::from_millis(10));
			sleep(sleep_for).await;
		}
	}

	#[allow(dead_code)]
	pub(crate) fn begin_keep_awake(&self) {
		self.begin_async_region(AsyncRegion::KeepAwake);
	}

	#[allow(dead_code)]
	pub(crate) fn end_keep_awake(&self) {
		self.end_async_region(AsyncRegion::KeepAwake);
	}

	pub(crate) fn begin_internal_keep_awake(&self) {
		self.begin_async_region(AsyncRegion::InternalKeepAwake);
	}

	pub(crate) fn end_internal_keep_awake(&self) {
		self.end_async_region(AsyncRegion::InternalKeepAwake);
	}

	pub(crate) fn begin_websocket_callback(&self) {
		self.begin_async_region(AsyncRegion::WebSocketCallbacks);
	}

	pub(crate) fn end_websocket_callback(&self) {
		self.end_async_region(AsyncRegion::WebSocketCallbacks);
	}

	pub(crate) fn begin_pending_disconnect(&self) {
		self.begin_async_region(AsyncRegion::PendingDisconnectCallbacks);
	}

	pub(crate) fn end_pending_disconnect(&self) {
		self.end_async_region(AsyncRegion::PendingDisconnectCallbacks);
	}

	pub(crate) fn track_shutdown_task(&self, handle: JoinHandle<()>) {
		let mut shutdown_tasks = self
			.0
			.shutdown_tasks
			.lock()
			.expect("shutdown tasks lock poisoned");
		shutdown_tasks.retain(|task| !task.is_finished());
		shutdown_tasks.push(handle);
	}

	#[allow(dead_code)]
	pub(crate) fn shutdown_task_count(&self) -> usize {
		let mut shutdown_tasks = self
			.0
			.shutdown_tasks
			.lock()
			.expect("shutdown tasks lock poisoned");
		shutdown_tasks.retain(|task| !task.is_finished());
		shutdown_tasks.len()
	}

	async fn sleep_shutdown_idle_ready(&self, ctx: &ActorContext) -> bool {
		self.active_http_request_count(ctx).await == 0
			&& self.0.keep_awake_count.load(Ordering::SeqCst) == 0
			&& self.0.internal_keep_awake_count.load(Ordering::SeqCst) == 0
			&& self.0.pending_disconnect_count.load(Ordering::SeqCst) == 0
	}

	fn shutdown_tasks_drained(&self, ctx: &ActorContext) -> bool {
		self.shutdown_task_count() == 0
			&& self.0.pending_disconnect_count.load(Ordering::SeqCst) == 0
			&& self.0.websocket_callback_count.load(Ordering::SeqCst) == 0
			&& !ctx.prevent_sleep()
	}

	fn begin_async_region(&self, region: AsyncRegion) {
		counter_for(&self.0, region).fetch_add(1, Ordering::SeqCst);
	}

	fn end_async_region(&self, region: AsyncRegion) {
		let counter = counter_for(&self.0, region);
		let previous = counter.fetch_sub(1, Ordering::SeqCst);
		if previous == 0 {
			counter.store(0, Ordering::SeqCst);
			tracing::warn!(
				region = region_name(region),
				"sleep async region count went below 0"
			);
		}
	}

	fn config(&self) -> ActorConfig {
		self.0
			.config
			.lock()
			.expect("sleep config lock poisoned")
			.clone()
	}

	async fn active_http_request_count(&self, ctx: &ActorContext) -> usize {
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
		let Some(envoy_handle) = envoy_handle else {
			return 0;
		};

		envoy_handle
			.get_active_http_request_count(ctx.actor_id(), generation)
			.await
			.unwrap_or(0)
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
				"run_handler_active",
				&self.0.run_handler_active.load(Ordering::SeqCst),
			)
			.field(
				"keep_awake_count",
				&self.0.keep_awake_count.load(Ordering::SeqCst),
			)
			.field(
				"internal_keep_awake_count",
				&self.0.internal_keep_awake_count.load(Ordering::SeqCst),
			)
			.field(
				"websocket_callback_count",
				&self.0.websocket_callback_count.load(Ordering::SeqCst),
			)
			.field(
				"pending_disconnect_count",
				&self.0.pending_disconnect_count.load(Ordering::SeqCst),
			)
			.finish()
	}
}

fn counter_for(inner: &SleepControllerInner, region: AsyncRegion) -> &AtomicU32 {
	match region {
		AsyncRegion::KeepAwake => &inner.keep_awake_count,
		AsyncRegion::InternalKeepAwake => &inner.internal_keep_awake_count,
		AsyncRegion::WebSocketCallbacks => &inner.websocket_callback_count,
		AsyncRegion::PendingDisconnectCallbacks => &inner.pending_disconnect_count,
	}
}

fn region_name(region: AsyncRegion) -> &'static str {
	match region {
		AsyncRegion::KeepAwake => "keep_awake",
		AsyncRegion::InternalKeepAwake => "internal_keep_awake",
		AsyncRegion::WebSocketCallbacks => "websocket_callbacks",
		AsyncRegion::PendingDisconnectCallbacks => "pending_disconnect_callbacks",
	}
}

#[cfg(test)]
#[path = "../../tests/modules/sleep.rs"]
mod tests;
