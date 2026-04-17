use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::FutureExt;
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::tunnel::HibernatingWebSocketMetadata;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::ActorConfig;
use crate::actor::callbacks::{ActorInstanceCallbacks, Request, RunRequest};
use crate::actor::connection::{ConnHandle, ConnectionManager, HibernatableConnectionMetadata};
use crate::actor::event::EventBroadcaster;
use crate::actor::metrics::ActorMetrics;
use crate::actor::queue::Queue;
use crate::actor::schedule::Schedule;
use crate::actor::sleep::{CanSleep, SleepController};
use crate::actor::state::{ActorState, OnStateChangeCallback, PersistedActor};
use crate::actor::vars::ActorVars;
use crate::inspector::Inspector;
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ListOpts, SaveStateOpts};

/// Shared actor runtime context.
///
/// This public surface is the foreign-runtime contract for `rivetkit-core`.
/// Native Rust, NAPI-backed TypeScript, and future V8 runtimes should be able
/// to drive actor behavior through `ActorFactory` plus the methods exposed here
/// and on the returned runtime objects like `Kv`, `SqliteDb`, `Schedule`,
/// `Queue`, `ConnHandle`, and `WebSocket`.
#[derive(Clone, Debug)]
pub struct ActorContext(Arc<ActorContextInner>);

#[derive(Debug)]
pub(crate) struct ActorContextInner {
	state: ActorState,
	vars: ActorVars,
	kv: Kv,
	sql: SqliteDb,
	schedule: Schedule,
	queue: Queue,
	broadcaster: EventBroadcaster,
	connections: ConnectionManager,
	sleep: SleepController,
	runtime_handle: Option<Handle>,
	action_lock: tokio::sync::Mutex<()>,
	abort_signal: CancellationToken,
	prevent_sleep: AtomicBool,
	sleep_requested: AtomicBool,
	destroy_requested: AtomicBool,
	destroy_completed: AtomicBool,
	destroy_completion_notify: Notify,
	inspector: std::sync::RwLock<Option<Inspector>>,
	callbacks: std::sync::RwLock<Option<Arc<ActorInstanceCallbacks>>>,
	metrics: ActorMetrics,
	actor_id: String,
	name: String,
	key: ActorKey,
	region: String,
}

impl ActorContext {
	pub fn new(
		actor_id: impl Into<String>,
		name: impl Into<String>,
		key: ActorKey,
		region: impl Into<String>,
	) -> Self {
		Self::build(
			actor_id.into(),
			name.into(),
			key,
			region.into(),
			ActorConfig::default(),
			Kv::default(),
			SqliteDb::default(),
		)
	}

	pub fn new_with_kv(
		actor_id: impl Into<String>,
		name: impl Into<String>,
		key: ActorKey,
		region: impl Into<String>,
		kv: Kv,
	) -> Self {
		Self::build(
			actor_id.into(),
			name.into(),
			key,
			region.into(),
			ActorConfig::default(),
			kv,
			SqliteDb::default(),
		)
	}

	pub(crate) fn new_runtime(
		actor_id: impl Into<String>,
		name: impl Into<String>,
		key: ActorKey,
		region: impl Into<String>,
		config: ActorConfig,
		kv: Kv,
		sql: SqliteDb,
	) -> Self {
		Self::build(
			actor_id.into(),
			name.into(),
			key,
			region.into(),
			config,
			kv,
			sql,
		)
	}

	fn build(
		actor_id: String,
		name: String,
		key: ActorKey,
		region: String,
		config: ActorConfig,
		kv: Kv,
		sql: SqliteDb,
	) -> Self {
		let metrics = ActorMetrics::new(actor_id.clone(), name.clone());
		let state = ActorState::new(kv.clone(), config.clone());
		let schedule = Schedule::new(state.clone(), actor_id.clone(), config);
		let abort_signal = CancellationToken::new();
		let queue = Queue::new(
			kv.clone(),
			ActorConfig::default(),
			Some(abort_signal.clone()),
			metrics.clone(),
		);
		let connections = ConnectionManager::new(
			actor_id.clone(),
			kv.clone(),
			ActorConfig::default(),
			metrics.clone(),
		);
		let sleep = SleepController::default();
		let runtime_handle = Handle::try_current().ok();

		let ctx = Self(Arc::new(ActorContextInner {
			state,
			vars: ActorVars::default(),
			kv,
			sql,
			schedule,
			queue,
			broadcaster: EventBroadcaster::default(),
			connections,
			sleep,
			runtime_handle,
			action_lock: tokio::sync::Mutex::new(()),
			abort_signal,
			prevent_sleep: AtomicBool::new(false),
			sleep_requested: AtomicBool::new(false),
			destroy_requested: AtomicBool::new(false),
			destroy_completed: AtomicBool::new(false),
			destroy_completion_notify: Notify::new(),
			inspector: std::sync::RwLock::new(None),
			callbacks: std::sync::RwLock::new(None),
			metrics,
			actor_id,
			name,
			key,
			region,
		}));
		ctx.configure_sleep_hooks();
		ctx
	}

	pub fn state(&self) -> Vec<u8> {
		self.0.state.state()
	}

	pub fn set_state(&self, state: Vec<u8>) {
		self.0.state.set_state(state);
		self.record_state_updated();
		self.reset_sleep_timer();
	}

	pub async fn save_state(&self, opts: SaveStateOpts) -> Result<()> {
		self.0.state.save_state(opts).await?;
		self.record_state_updated();
		Ok(())
	}

	pub fn vars(&self) -> Vec<u8> {
		self.0.vars.vars()
	}

	pub fn set_vars(&self, vars: Vec<u8>) {
		self.0.vars.set_vars(vars);
		self.reset_sleep_timer();
	}

	pub async fn kv_batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		self.0.kv.batch_get(keys).await
	}

	pub async fn kv_batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		self.0.kv.batch_put(entries).await
	}

	pub async fn kv_batch_delete(&self, keys: &[&[u8]]) -> Result<()> {
		self.0.kv.batch_delete(keys).await
	}

	pub async fn kv_delete_range(&self, start: &[u8], end: &[u8]) -> Result<()> {
		self.0.kv.delete_range(start, end).await
	}

	pub async fn kv_list_prefix(
		&self,
		prefix: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		self.0.kv.list_prefix(prefix, opts).await
	}

	pub async fn kv_list_range(
		&self,
		start: &[u8],
		end: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		self.0.kv.list_range(start, end, opts).await
	}

	pub fn kv(&self) -> &Kv {
		&self.0.kv
	}

	pub fn sql(&self) -> &SqliteDb {
		&self.0.sql
	}

	pub async fn db_exec(&self, sql: &str) -> Result<Vec<u8>> {
		self.0.sql.exec_rows_cbor(sql).await
	}

	pub async fn db_query(&self, sql: &str, params: Option<&[u8]>) -> Result<Vec<u8>> {
		self.0.sql.query_rows_cbor(sql, params).await
	}

	pub async fn db_run(&self, sql: &str, params: Option<&[u8]>) -> Result<()> {
		self.0.sql.run_cbor(sql, params).await?;
		Ok(())
	}

	pub fn schedule(&self) -> &Schedule {
		&self.0.schedule
	}

	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		self.0.schedule.set_alarm(timestamp_ms)
	}

	pub fn queue(&self) -> &Queue {
		&self.0.queue
	}

	pub fn sleep(&self) {
		self.0.sleep.cancel_sleep_timer();
		self.0.sleep_requested.store(true, Ordering::SeqCst);
		if let Ok(runtime) = Handle::try_current() {
			let ctx = self.clone();
			runtime.spawn(async move {
				tokio::time::sleep(Duration::from_millis(1)).await;
				if let Err(error) = ctx.persist_hibernatable_connections().await {
					tracing::error!(
						?error,
						"failed to persist hibernatable connections on sleep"
					);
				}
				ctx.0.sleep.request_sleep(ctx.actor_id());
			});
			return;
		}

		self.0.sleep.request_sleep(self.actor_id());
	}

	pub fn destroy(&self) {
		self.mark_destroy_requested();

		let actor_id = self.actor_id().to_owned();
		let sleep = self.0.sleep.clone();
		if let Ok(runtime) = Handle::try_current() {
			runtime.spawn(async move {
				sleep.request_destroy(&actor_id);
			});
			return;
		}

		sleep.request_destroy(&actor_id);
	}

	pub fn mark_destroy_requested(&self) {
		self.0.sleep.cancel_sleep_timer();
		self.0.state.flush_on_shutdown();
		self.0.destroy_requested.store(true, Ordering::SeqCst);
		self.0.destroy_completed.store(false, Ordering::SeqCst);
		self.0.abort_signal.cancel();
	}

	pub fn set_prevent_sleep(&self, prevent: bool) {
		self.0.prevent_sleep.store(prevent, Ordering::SeqCst);
		self.reset_sleep_timer();
	}

	pub fn prevent_sleep(&self) -> bool {
		self.0.prevent_sleep.load(Ordering::SeqCst)
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		let Ok(runtime) = Handle::try_current() else {
			tracing::warn!("skipping wait_until without a tokio runtime");
			return;
		};

		let handle = runtime.spawn(future);
		self.0.sleep.track_shutdown_task(handle);
	}

	pub fn actor_id(&self) -> &str {
		&self.0.actor_id
	}

	pub fn name(&self) -> &str {
		&self.0.name
	}

	pub fn key(&self) -> &ActorKey {
		&self.0.key
	}

	pub fn region(&self) -> &str {
		&self.0.region
	}

	pub fn abort_signal(&self) -> &CancellationToken {
		&self.0.abort_signal
	}

	pub fn aborted(&self) -> bool {
		self.0.abort_signal.is_cancelled()
	}

	#[doc(hidden)]
	pub fn record_startup_create_state(&self, duration: Duration) {
		self.0.metrics.observe_create_state(duration);
	}

	#[doc(hidden)]
	pub fn record_startup_create_vars(&self, duration: Duration) {
		self.0.metrics.observe_create_vars(duration);
	}

	pub fn broadcast(&self, name: &str, args: &[u8]) {
		self.0.broadcaster.broadcast(&self.conns(), name, args);
	}

	pub fn conns(&self) -> Vec<ConnHandle> {
		self.0.connections.list()
	}

	pub async fn client_call(&self, _request: &[u8]) -> Result<Vec<u8>> {
		Err(anyhow!("actor client bridge is not configured"))
	}

	pub fn client_endpoint(&self) -> Result<String> {
		self.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.endpoint().to_owned())
			.ok_or_else(|| anyhow!("actor client endpoint is not configured"))
	}

	pub fn client_token(&self) -> Result<Option<String>> {
		self.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.token().map(ToOwned::to_owned))
			.ok_or_else(|| anyhow!("actor client token is not configured"))
	}

	pub fn client_namespace(&self) -> Result<String> {
		self.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.namespace().to_owned())
			.ok_or_else(|| anyhow!("actor client namespace is not configured"))
	}

	pub fn client_pool_name(&self) -> Result<String> {
		self.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.pool_name().to_owned())
			.ok_or_else(|| anyhow!("actor client pool name is not configured"))
	}

	pub fn ack_hibernatable_websocket_message(
		&self,
		gateway_id: &[u8],
		request_id: &[u8],
		server_message_index: u16,
	) -> Result<()> {
		let envoy_handle = self
			.0
			.sleep
			.envoy_handle()
			.ok_or_else(|| anyhow!("hibernatable websocket ack is not configured"))?;
		let gateway_id: [u8; 4] = gateway_id
			.try_into()
			.map_err(|_| anyhow!("invalid hibernatable websocket gateway id"))?;
		let request_id: [u8; 4] = request_id
			.try_into()
			.map_err(|_| anyhow!("invalid hibernatable websocket request id"))?;
		envoy_handle.send_hibernatable_ws_message_ack(gateway_id, request_id, server_message_index);
		Ok(())
	}

	#[allow(dead_code)]
	pub(crate) fn load_persisted_actor(&self, persisted: PersistedActor) {
		self.0.state.load_persisted(persisted);
	}

	#[allow(dead_code)]
	pub(crate) fn persisted_actor(&self) -> PersistedActor {
		self.0.state.persisted()
	}

	pub(crate) fn set_has_initialized(&self, has_initialized: bool) {
		self.0.state.set_has_initialized(has_initialized);
	}

	#[allow(dead_code)]
	pub(crate) fn set_on_state_change_callback(&self, callback: Option<OnStateChangeCallback>) {
		self.0.state.set_on_state_change_callback(callback);
	}

	pub fn set_in_on_state_change_callback(&self, in_callback: bool) {
		self.0.state.set_in_on_state_change_callback(in_callback);
	}

	pub(crate) async fn wait_for_on_state_change_idle(&self) {
		self.0.state.wait_for_on_state_change_idle().await;
	}

	pub(crate) fn trigger_throttled_state_save(&self) {
		self.0.state.trigger_throttled_save();
		self.reset_sleep_timer();
	}

	pub(crate) fn record_startup_on_migrate(&self, duration: Duration) {
		self.0.metrics.observe_on_migrate(duration);
	}

	pub(crate) fn record_startup_on_wake(&self, duration: Duration) {
		self.0.metrics.observe_on_wake(duration);
	}

	pub(crate) fn record_total_startup(&self, duration: Duration) {
		self.0.metrics.observe_total_startup(duration);
	}

	pub(crate) fn record_action_call(&self, action_name: &str) {
		self.0.metrics.observe_action_call(action_name);
	}

	pub(crate) fn record_action_error(&self, action_name: &str) {
		self.0.metrics.observe_action_error(action_name);
	}

	pub(crate) fn record_action_duration(&self, action_name: &str, duration: Duration) {
		self.0
			.metrics
			.observe_action_duration(action_name, duration);
	}

	#[doc(hidden)]
	pub fn render_metrics(&self) -> Result<String> {
		self.0.metrics.render()
	}

	pub(crate) fn metrics_content_type(&self) -> String {
		self.0.metrics.metrics_content_type()
	}

	#[allow(dead_code)]
	pub(crate) fn add_conn(&self, conn: ConnHandle) {
		self.0.connections.insert_existing(conn);
		self.record_connections_updated();
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn remove_conn(&self, conn_id: &str) -> Option<ConnHandle> {
		let removed = self.0.connections.remove_existing(conn_id);
		if removed.is_some() {
			self.record_connections_updated();
			self.reset_sleep_timer();
		}
		removed
	}

	#[allow(dead_code)]
	pub(crate) fn configure_connection_runtime(
		&self,
		config: ActorConfig,
		callbacks: Arc<ActorInstanceCallbacks>,
	) {
		self.0.sleep.configure(config.clone());
		self.0
			.connections
			.configure_runtime(config, callbacks.clone());
		*self
			.0
			.callbacks
			.write()
			.expect("actor callbacks lock poisoned") = Some(callbacks);
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(&self, envoy_handle: EnvoyHandle, generation: Option<u32>) {
		self.0
			.sleep
			.configure_envoy(envoy_handle.clone(), generation);
		self.0.schedule.configure_envoy(envoy_handle, generation);
	}

	#[allow(dead_code)]
	pub(crate) fn clear_envoy(&self) {
		self.0.sleep.clear_envoy();
		self.0.schedule.clear_envoy();
	}

	#[allow(dead_code)]
	pub(crate) async fn connect_conn<F>(
		&self,
		params: Vec<u8>,
		is_hibernatable: bool,
		hibernation: Option<HibernatableConnectionMetadata>,
		request: Option<Request>,
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: Future<Output = Result<Vec<u8>>> + Send,
	{
		let conn = self
			.0
			.connections
			.connect_with_state(
				self,
				params,
				is_hibernatable,
				hibernation,
				request,
				create_state,
			)
			.await?;
		self.record_connections_updated();
		Ok(conn)
	}

	#[allow(dead_code)]
	pub async fn connect_conn_with_request<F>(
		&self,
		params: Vec<u8>,
		request: Option<Request>,
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: Future<Output = Result<Vec<u8>>> + Send,
	{
		self.connect_conn(params, false, None, request, create_state)
			.await
	}

	pub(crate) fn reconnect_hibernatable_conn(
		&self,
		gateway_id: &[u8],
		request_id: &[u8],
	) -> Result<ConnHandle> {
		self.0
			.connections
			.reconnect_hibernatable(self, gateway_id, request_id)
	}

	#[allow(dead_code)]
	pub(crate) async fn persist_hibernatable_connections(&self) -> Result<()> {
		self.0.connections.persist_hibernatable().await
	}

	#[allow(dead_code)]
	pub(crate) async fn restore_hibernatable_connections(&self) -> Result<Vec<ConnHandle>> {
		let restored = self.0.connections.restore_persisted(self).await?;
		if !restored.is_empty() {
			if let Some(envoy_handle) = self.0.sleep.envoy_handle() {
				let meta_entries = restored
					.iter()
					.filter_map(|conn| {
						let hibernation = conn.hibernation()?;
						Some(HibernatingWebSocketMetadata {
							gateway_id: hibernation.gateway_id.clone().try_into().ok()?,
							request_id: hibernation.request_id.clone().try_into().ok()?,
							envoy_message_index: hibernation.client_message_index,
							rivet_message_index: hibernation.server_message_index,
							path: hibernation.request_path,
							headers: hibernation.request_headers.into_iter().collect(),
						})
					})
					.collect();
				envoy_handle.restore_hibernating_requests(self.actor_id().to_owned(), meta_entries);
			}
			self.record_connections_updated();
		}
		Ok(restored)
	}

	#[allow(dead_code)]
	pub(crate) fn configure_inspector(&self, inspector: Option<Inspector>) {
		*self
			.0
			.inspector
			.write()
			.expect("actor inspector lock poisoned") = inspector;
	}

	pub(crate) fn inspector(&self) -> Option<Inspector> {
		self.0
			.inspector
			.read()
			.expect("actor inspector lock poisoned")
			.clone()
	}

	pub(crate) fn downgrade(&self) -> Weak<ActorContextInner> {
		Arc::downgrade(&self.0)
	}

	pub(crate) fn from_weak(weak: &Weak<ActorContextInner>) -> Option<Self> {
		weak.upgrade().map(Self)
	}

	#[allow(dead_code)]
	pub(crate) fn set_ready(&self, ready: bool) {
		self.0.sleep.set_ready(ready);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn ready(&self) -> bool {
		self.0.sleep.ready()
	}

	#[allow(dead_code)]
	pub(crate) fn set_started(&self, started: bool) {
		self.0.sleep.set_started(started);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn started(&self) -> bool {
		self.0.sleep.started()
	}

	#[allow(dead_code)]
	pub(crate) fn set_run_handler_active(&self, active: bool) {
		self.0.sleep.set_run_handler_active(active);
		self.reset_sleep_timer();
	}

	pub fn run_handler_active(&self) -> bool {
		self.0.sleep.run_handler_active()
	}

	pub fn restart_run_handler(&self) -> Result<()> {
		if self.run_handler_active() {
			return Ok(());
		}

		let callbacks = self
			.0
			.callbacks
			.read()
			.expect("actor callbacks lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("actor run handler callbacks are not configured"))?;
		if callbacks.run.is_none() {
			return Err(anyhow!("actor run handler is not configured"));
		}

		let runtime = self
			.0
			.runtime_handle
			.clone()
			.ok_or_else(|| anyhow!("actor run handler restart requires a tokio runtime"))?;
		self.set_run_handler_active(true);
		let task_ctx = self.clone();
		let handle = runtime.spawn(async move {
			let run = callbacks
				.run
				.as_ref()
				.expect("run handler presence checked before restart");
			let result = AssertUnwindSafe(run(RunRequest {
				ctx: task_ctx.clone(),
			}))
			.catch_unwind()
			.await;
			task_ctx.set_run_handler_active(false);

			match result {
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					tracing::error!(?error, "actor run handler failed");
				}
				Err(panic) => {
					tracing::error!(
						panic = %panic_payload_message(panic.as_ref()),
						"actor run handler panicked"
					);
				}
			}
		});
		self.track_run_handler(handle);
		Ok(())
	}

	pub(crate) async fn lock_action_execution(&self) -> tokio::sync::MutexGuard<'_, ()> {
		self.0.action_lock.lock().await
	}

	pub(crate) fn destroy_requested(&self) -> bool {
		self.0.destroy_requested.load(Ordering::SeqCst)
	}

	pub fn is_destroy_requested(&self) -> bool {
		self.destroy_requested()
	}

	pub(crate) async fn wait_for_destroy_completion(&self) {
		if self.0.destroy_completed.load(Ordering::SeqCst) {
			return;
		}

		loop {
			let notified = self.0.destroy_completion_notify.notified();
			if self.0.destroy_completed.load(Ordering::SeqCst) {
				return;
			}
			notified.await;
			if self.0.destroy_completed.load(Ordering::SeqCst) {
				return;
			}
		}
	}

	pub async fn wait_for_destroy_completion_public(&self) {
		self.wait_for_destroy_completion().await;
	}

	pub(crate) fn mark_destroy_completed(&self) {
		self.0.destroy_completed.store(true, Ordering::SeqCst);
		self.0.destroy_completion_notify.notify_waiters();
	}

	pub(crate) fn track_run_handler(&self, handle: JoinHandle<()>) {
		self.0.sleep.track_run_handler(handle);
	}

	#[allow(dead_code)]
	pub(crate) async fn can_sleep(&self) -> CanSleep {
		self.0.sleep.can_sleep(self).await
	}

	pub(crate) async fn wait_for_run_handler(&self, timeout_duration: Duration) -> bool {
		self.0.sleep.wait_for_run_handler(timeout_duration).await
	}

	pub(crate) async fn wait_for_sleep_idle_window(&self, deadline: Instant) -> bool {
		self.0
			.sleep
			.wait_for_sleep_idle_window(self, deadline)
			.await
	}

	pub(crate) async fn wait_for_shutdown_tasks(&self, deadline: Instant) -> bool {
		self.0.sleep.wait_for_shutdown_tasks(self, deadline).await
	}

	pub(crate) fn reset_sleep_timer(&self) {
		self.0.sleep.reset_sleep_timer(self.clone());
	}

	pub(crate) fn cancel_sleep_timer(&self) {
		self.0.sleep.cancel_sleep_timer();
	}

	pub(crate) fn cancel_local_alarm_timeouts(&self) {
		self.0.schedule.cancel_local_alarm_timeouts();
	}

	#[allow(dead_code)]
	pub(crate) fn configure_sleep(&self, config: ActorConfig) {
		self.0.sleep.configure(config.clone());
		self.0.queue.configure_sleep(config);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn sleep_requested(&self) -> bool {
		self.0.sleep_requested.load(Ordering::SeqCst)
	}

	pub(crate) fn request_sleep_if_pending(&self) {
		if self.sleep_requested() {
			self.0.sleep.request_sleep(self.actor_id());
		}
	}

	#[allow(dead_code)]
	pub(crate) fn begin_keep_awake(&self) {
		self.0.sleep.begin_keep_awake();
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn end_keep_awake(&self) {
		self.0.sleep.end_keep_awake();
		self.reset_sleep_timer();
	}

	pub(crate) fn begin_internal_keep_awake(&self) {
		self.0.sleep.begin_internal_keep_awake();
		self.reset_sleep_timer();
	}

	pub(crate) fn end_internal_keep_awake(&self) {
		self.0.sleep.end_internal_keep_awake();
		self.reset_sleep_timer();
	}

	pub(crate) async fn internal_keep_awake_task(
		&self,
		future: BoxFuture<'static, Result<()>>,
	) -> Result<()> {
		self.begin_internal_keep_awake();
		let result = future.await;
		self.end_internal_keep_awake();
		result
	}

	pub(crate) async fn wait_for_internal_keep_awake_idle(&self, deadline: Instant) -> bool {
		self.0
			.sleep
			.wait_for_internal_keep_awake_idle(deadline)
			.await
	}

	pub(crate) async fn wait_for_http_requests_drained(&self, deadline: Instant) -> bool {
		self.0
			.sleep
			.wait_for_http_requests_drained(self, deadline)
			.await
	}

	pub(crate) async fn with_websocket_callback<F, Fut, T>(&self, run: F) -> T
	where
		F: FnOnce() -> Fut,
		Fut: Future<Output = T>,
	{
		self.0.sleep.begin_websocket_callback();
		self.reset_sleep_timer();
		let result = run().await;
		self.0.sleep.end_websocket_callback();
		self.reset_sleep_timer();
		result
	}

	#[allow(dead_code)]
	pub fn begin_websocket_callback(&self) {
		self.0.sleep.begin_websocket_callback();
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub fn end_websocket_callback(&self) {
		self.0.sleep.end_websocket_callback();
		self.reset_sleep_timer();
	}

	pub(crate) fn begin_pending_disconnect(&self) {
		self.0.sleep.begin_pending_disconnect();
		self.reset_sleep_timer();
	}

	pub(crate) fn end_pending_disconnect(&self) {
		self.0.sleep.end_pending_disconnect();
		self.reset_sleep_timer();
	}

	fn configure_sleep_hooks(&self) {
		let internal_keep_awake_ctx = self.clone();
		self.0
			.schedule
			.set_internal_keep_awake(Some(Arc::new(move |future| {
				let ctx = internal_keep_awake_ctx.clone();
				Box::pin(async move { ctx.internal_keep_awake_task(future).await })
			})));

		let queue_ctx = self.clone();
		self.0
			.queue
			.set_wait_activity_callback(Some(Arc::new(move || {
				queue_ctx.reset_sleep_timer();
			})));

		let queue_ctx = self.clone();
		self.0
			.queue
			.set_inspector_update_callback(Some(Arc::new(move |queue_size| {
				queue_ctx.record_queue_updated(queue_size);
			})));
	}

	fn record_state_updated(&self) {
		if let Some(inspector) = self.inspector() {
			inspector.record_state_updated();
		}
	}

	pub(crate) fn record_connections_updated(&self) {
		let Some(inspector) = self.inspector() else {
			return;
		};
		let active_connections = self.0.connections.active_count();
		inspector.record_connections_updated(active_connections);
	}

	fn record_queue_updated(&self, queue_size: u32) {
		if let Some(inspector) = self.inspector() {
			inspector.record_queue_updated(queue_size);
		}
	}
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
	if let Some(message) = payload.downcast_ref::<&'static str>() {
		(*message).to_owned()
	} else if let Some(message) = payload.downcast_ref::<String>() {
		message.clone()
	} else {
		"unknown panic payload".to_owned()
	}
}

impl Default for ActorContext {
	fn default() -> Self {
		Self::new("", "", Vec::new(), "")
	}
}

#[cfg(test)]
#[path = "../../tests/modules/context.rs"]
pub(crate) mod tests;
