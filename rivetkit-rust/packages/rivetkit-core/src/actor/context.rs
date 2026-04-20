use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use rivet_envoy_client::tunnel::HibernatingWebSocketMetadata;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::sync::{Notify, broadcast, mpsc, oneshot};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::actor::callbacks::{ActorEvent, Reply, Request, StateDelta};
use crate::actor::connection::{
	ConnHandle, ConnHandles, ConnectionManager, HibernatableConnectionMetadata,
	PendingHibernationChanges,
};
use crate::actor::diagnostics::ActorDiagnostics;
use crate::actor::event::EventBroadcaster;
use crate::actor::metrics::ActorMetrics;
use crate::actor::queue::Queue;
use crate::actor::schedule::Schedule;
use crate::actor::sleep::{CanSleep, SleepController};
use crate::actor::state::{ActorState, PersistedActor};
use crate::actor::task::{
	LIFECYCLE_EVENT_INBOX_CHANNEL, LifecycleEvent, actor_channel_overloaded_error,
};
use crate::actor::task_types::UserTaskKind;
use crate::actor::vars::ActorVars;
use crate::actor::work_registry::RegionGuard;
use crate::ActorConfig;
use crate::inspector::{Inspector, InspectorSnapshot};
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, ConnId, ListOpts, SaveStateOpts};

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
	activity: ActivityState,
	prevent_sleep: AtomicBool,
	in_on_state_change: Arc<AtomicBool>,
	sleep_requested: AtomicBool,
	destroy_requested: AtomicBool,
	destroy_completed: AtomicBool,
	destroy_completion_notify: Notify,
	abort_signal: CancellationToken,
	inspector: std::sync::RwLock<Option<Inspector>>,
	inspector_attach_count: std::sync::RwLock<Option<Arc<AtomicU32>>>,
	inspector_overlay_tx:
		std::sync::RwLock<Option<broadcast::Sender<Arc<Vec<u8>>>>>,
	actor_events: std::sync::RwLock<Option<mpsc::Sender<ActorEvent>>>,
	lifecycle_events: std::sync::RwLock<Option<mpsc::Sender<LifecycleEvent>>>,
	hibernated_connection_liveness_override:
		std::sync::RwLock<Option<BTreeSet<(Vec<u8>, Vec<u8>)>>>,
	lifecycle_event_inbox_capacity: usize,
	metrics: ActorMetrics,
	diagnostics: ActorDiagnostics,
	actor_id: String,
	name: String,
	key: ActorKey,
	region: String,
}

#[derive(Debug, Default)]
pub(crate) struct ActivityState {
	dirty: AtomicBool,
	notification_pending: AtomicBool,
}

impl ActivityState {
	fn mark_dirty(&self) {
		self.dirty.store(true, Ordering::SeqCst);
	}

	fn take_dirty(&self) -> bool {
		self.dirty.swap(false, Ordering::SeqCst)
	}

	fn try_begin_notification(&self) -> bool {
		self
			.notification_pending
			.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
			.is_ok()
	}

	fn clear_notification_pending(&self) {
		self.notification_pending.store(false, Ordering::SeqCst);
	}
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
		let diagnostics = ActorDiagnostics::new(actor_id.clone());
		let lifecycle_event_inbox_capacity = config.lifecycle_event_inbox_capacity;
		let state = ActorState::new_with_metrics(kv.clone(), config.clone(), metrics.clone());
		let in_on_state_change = state.in_on_state_change_flag();
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
		let ctx = Self(Arc::new(ActorContextInner {
			state,
			vars: ActorVars::default(),
			kv,
			sql,
			schedule,
			queue,
			broadcaster: EventBroadcaster,
			connections,
			sleep,
			activity: ActivityState::default(),
			prevent_sleep: AtomicBool::new(false),
			in_on_state_change,
			sleep_requested: AtomicBool::new(false),
			destroy_requested: AtomicBool::new(false),
			destroy_completed: AtomicBool::new(false),
			destroy_completion_notify: Notify::new(),
			abort_signal,
			inspector: std::sync::RwLock::new(None),
			inspector_attach_count: std::sync::RwLock::new(None),
			inspector_overlay_tx: std::sync::RwLock::new(None),
			actor_events: std::sync::RwLock::new(None),
			lifecycle_events: std::sync::RwLock::new(None),
			hibernated_connection_liveness_override: std::sync::RwLock::new(None),
			lifecycle_event_inbox_capacity,
			metrics,
			diagnostics,
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

	pub fn set_state(&self, state: Vec<u8>) -> Result<()> {
		let routed_to_actor_task = self.0.state.lifecycle_events_configured();
		self.0.state.set_state(state)?;
		if !routed_to_actor_task {
			self.record_state_updated();
			self.reset_sleep_timer();
		}
		Ok(())
	}

	pub fn request_save(&self, immediate: bool) {
		self.0.state.request_save(immediate);
	}

	pub fn request_save_within(&self, ms: u32) {
		self.0.state.request_save_within(ms);
	}

	pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()> {
		let save_request_revision = self.0.state.save_request_revision();
		self
			.save_state_with_revision(deltas, save_request_revision)
			.await
	}

	pub(crate) async fn persist_state(&self, opts: SaveStateOpts) -> Result<()> {
		self.0.state.persist_state(opts).await?;
		self.record_state_updated();
		Ok(())
	}

	pub(crate) async fn wait_for_pending_state_writes(&self) {
		self.0.state.wait_for_pending_writes().await;
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

	pub async fn db_query(
		&self,
		sql: &str,
		params: Option<&[u8]>,
	) -> Result<Vec<u8>> {
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

	/// Resync persisted alarms with the runtime's alarm transport.
	///
	/// Foreign-runtime adapters should call this during startup after loading
	/// any persisted schedule state and before accepting user callbacks that rely
	/// on future alarms being armed.
	pub fn init_alarms(&self) {
		self.0.schedule.sync_future_alarm_logged();
	}

	pub fn queue(&self) -> &Queue {
		&self.0.queue
	}

	pub fn sleep(&self) {
		self.0.sleep.cancel_sleep_timer();
		self.0.sleep_requested.store(true, Ordering::SeqCst);
		if let Ok(runtime) = Handle::try_current() {
			let ctx = self.clone();
			// Intentionally detached: `sleep()` is a user-facing bridge that only
			// asks envoy to stop this actor; ActorTask owns the actual shutdown
			// drain and hibernation persistence.
			runtime.spawn(async move {
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
			// Intentionally detached without an extra defer: the spawned task is
			// already enough to decouple the user-facing destroy signal from the
			// caller, and ActorTask owns the actual shutdown once stop arrives.
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

	/// Prevents the actor from entering sleep while enabled.
	///
	/// Shutdown drain loops continue polling until this is cleared or the
	/// configured grace deadline is reached.
	pub fn set_prevent_sleep(&self, enabled: bool) {
		let previous = self.0.prevent_sleep.swap(enabled, Ordering::SeqCst);
		if previous != enabled {
			self.0.sleep.notify_prevent_sleep_changed();
		}
		self.reset_sleep_timer();
	}

	pub fn prevent_sleep(&self) -> bool {
		self.0.prevent_sleep.load(Ordering::SeqCst)
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		if Handle::try_current().is_err() {
			tracing::warn!("skipping wait_until without a tokio runtime");
			return;
		}

		let ctx = self.clone();
		// Intentionally detached but tracked by SleepController: waitUntil work
		// is a public side task that shutdown drains/aborts through
		// `shutdown_tasks`, not an ActorTask dispatch child.
		self.0.sleep.track_shutdown_task(async move {
			ctx.record_user_task_started(UserTaskKind::WaitUntil);
			let started_at = Instant::now();
			future.await;
			ctx.record_user_task_finished(UserTaskKind::WaitUntil, started_at.elapsed());
		});
	}

	pub async fn keep_awake<F>(&self, future: F) -> F::Output
	where
		F: Future,
	{
		let _guard = self.keep_awake_guard();
		future.await
	}

	pub async fn internal_keep_awake<F>(&self, future: F) -> F::Output
	where
		F: Future,
	{
		let _guard = self.internal_keep_awake_guard();
		future.await
	}

	pub fn keep_awake_count(&self) -> usize {
		self.0.sleep.keep_awake_count()
	}

	pub fn internal_keep_awake_count(&self) -> usize {
		self.0.sleep.internal_keep_awake_count()
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

	#[doc(hidden)]
	pub fn record_startup_create_state(&self, duration: Duration) {
		self.0.metrics.observe_create_state(duration);
	}

	#[doc(hidden)]
	pub fn record_startup_create_vars(&self, duration: Duration) {
		self.0.metrics.observe_create_vars(duration);
	}

	pub fn broadcast(&self, name: &str, args: &[u8]) {
		self.0.broadcaster.broadcast(self.conns(), name, args);
	}

	/// Returns a lock-backed iterator over live connections.
	///
	/// Do not hold the returned iterator across `.await`. It keeps a read lock
	/// on the connection map until dropped, which blocks connection writers.
	#[must_use]
	pub fn conns(&self) -> ConnHandles<'_> {
		self.0.connections.iter()
	}

	pub async fn client_call(&self, _request: &[u8]) -> Result<Vec<u8>> {
		Err(anyhow!("actor client bridge is not configured"))
	}

	pub fn client_endpoint(&self) -> Result<String> {
		self
			.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.endpoint().to_owned())
			.ok_or_else(|| anyhow!("actor client endpoint is not configured"))
	}

	pub fn client_token(&self) -> Result<Option<String>> {
		self
			.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.token().map(ToOwned::to_owned))
			.ok_or_else(|| anyhow!("actor client token is not configured"))
	}

	pub fn client_namespace(&self) -> Result<String> {
		self
			.0
			.sleep
			.envoy_handle()
			.map(|handle| handle.namespace().to_owned())
			.ok_or_else(|| anyhow!("actor client namespace is not configured"))
	}

	pub fn client_pool_name(&self) -> Result<String> {
		self
			.0
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
		envoy_handle.send_hibernatable_ws_message_ack(
			gateway_id,
			request_id,
			server_message_index,
		);
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

	/// Marks whether this actor has completed its first-create initialization.
	///
	/// Foreign-runtime adapters should set this before the pre-ready persistence
	/// flush that commits first-create state to KV.
	pub fn set_has_initialized(&self, has_initialized: bool) {
		self.0.state.set_has_initialized(has_initialized);
	}

	pub fn set_in_on_state_change_callback(&self, in_callback: bool) {
		self.0.state.set_in_on_state_change_callback(in_callback);
	}

	pub fn in_on_state_change_callback(&self) -> bool {
		self.0.in_on_state_change.load(Ordering::SeqCst)
	}

	pub fn on_request_save(&self, hook: Box<dyn Fn(bool) + Send + Sync>) {
		self.0.state.on_request_save(hook);
	}

	/// Dispatches any scheduled actions whose deadline has already passed.
	///
	/// Foreign-runtime adapters should call this after startup callbacks complete
	/// so overdue scheduled work enters the normal actor event loop.
	pub async fn drain_overdue_scheduled_events(&self) -> Result<()> {
		for event in self.0.schedule.due_events(now_timestamp_ms()) {
			self
				.dispatch_scheduled_action(&event.event_id, event.action, event.args)
				.await;
		}

		self.0.schedule.sync_alarm_logged();
		Ok(())
	}

	pub(crate) fn metrics(&self) -> &ActorMetrics {
		&self.0.metrics
	}

	pub(crate) fn record_user_task_started(&self, kind: UserTaskKind) {
		self.0.metrics.begin_user_task(kind);
	}

	pub(crate) fn record_user_task_finished(
		&self,
		kind: UserTaskKind,
		duration: Duration,
	) {
		self.0.metrics.end_user_task(kind, duration);
	}

	pub(crate) fn record_shutdown_wait(
		&self,
		reason: crate::actor::task_types::StopReason,
		duration: Duration,
	) {
		self.0.metrics.observe_shutdown_wait(reason, duration);
	}

	pub(crate) fn record_shutdown_timeout(
		&self,
		reason: crate::actor::task_types::StopReason,
	) {
		self.0.metrics.inc_shutdown_timeout(reason);
	}

	pub(crate) fn record_direct_subsystem_shutdown_warning(
		&self,
		subsystem: &str,
		operation: &str,
	) {
		self
			.0
			.metrics
			.inc_direct_subsystem_shutdown_warning(subsystem, operation);
	}

	pub(crate) fn warn_work_sent_to_stopping_instance(&self, operation: &'static str) {
		if let Some(suppression) = self
			.0
			.diagnostics
			.record("work_sent_to_stopping_instance")
		{
			tracing::warn!(
				actor_id = %suppression.actor_id,
				operation,
				per_actor_suppressed = suppression.per_actor_suppressed,
				global_suppressed = suppression.global_suppressed,
				"work sent to stopping actor instance"
			);
		}
	}

	pub(crate) fn warn_self_call_risk(&self, operation: &'static str) {
		if let Some(suppression) = self.0.diagnostics.record("self_call_risk") {
			tracing::warn!(
				actor_id = %suppression.actor_id,
				operation,
				per_actor_suppressed = suppression.per_actor_suppressed,
				global_suppressed = suppression.global_suppressed,
				"actor dispatch may be parked behind the current instance"
			);
		}
	}

	pub(crate) fn warn_long_shutdown_drain(
		&self,
		reason: &'static str,
		phase: &'static str,
		elapsed: Duration,
	) {
		if let Some(suppression) = self.0.diagnostics.record("long_shutdown_drain") {
			tracing::warn!(
				actor_id = %suppression.actor_id,
				reason,
				phase,
				elapsed_ms = elapsed.as_millis() as u64,
				per_actor_suppressed = suppression.per_actor_suppressed,
				global_suppressed = suppression.global_suppressed,
				"actor shutdown drain is taking longer than expected"
			);
		}
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
	) {
		self.0.sleep.configure(config.clone());
		self.0.connections.configure_runtime(config);
	}

	pub(crate) fn configure_actor_events(
		&self,
		sender: Option<mpsc::Sender<ActorEvent>>,
	) {
		*self
			.0
			.actor_events
			.write()
			.expect("actor events lock poisoned") = sender;
	}

	pub(crate) fn try_send_actor_event(
		&self,
		event: ActorEvent,
		operation: &'static str,
	) -> Result<()> {
		let sender = self
			.0
			.actor_events
			.read()
			.expect("actor events lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("actor event inbox is not configured"))?;
		let permit = sender.try_reserve().map_err(|_| {
			actor_channel_overloaded_error(
				"actor_event_inbox",
				self.0.lifecycle_event_inbox_capacity,
				operation,
				Some(&self.0.metrics),
			)
		})?;
		permit.send(event);
		Ok(())
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(
		&self,
		envoy_handle: EnvoyHandle,
		generation: Option<u32>,
	) {
		self.0
			.sleep
			.configure_envoy(self.actor_id(), envoy_handle.clone(), generation);
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
		self.notify_activity_dirty_or_reset_sleep_timer();
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
		self
			.connect_conn(params, false, None, request, create_state)
			.await
	}

	pub(crate) fn reconnect_hibernatable_conn(
		&self,
		gateway_id: &[u8],
		request_id: &[u8],
	) -> Result<ConnHandle> {
		self
			.0
			.connections
			.reconnect_hibernatable(self, gateway_id, request_id)
	}

	pub async fn disconnect_conn(&self, id: ConnId) -> Result<()> {
		self
			.0
			.connections
			.disconnect_transport_only(self, |conn| conn.id() == id)
			.await
	}

	pub async fn disconnect_conns<F>(&self, predicate: F) -> Result<()>
	where
		F: FnMut(&ConnHandle) -> bool,
	{
		self
			.0
			.connections
			.disconnect_transport_only(self, predicate)
			.await
	}

	pub(crate) fn request_hibernation_transport_save(&self, conn_id: &str) {
		self.0
			.connections
			.queue_hibernation_update(conn_id.to_owned());
		self.request_save(false);
	}

	pub(crate) fn request_hibernation_transport_removal(
		&self,
		conn_id: impl Into<String>,
	) {
		self.0.connections.queue_hibernation_removal(conn_id.into());
		self.request_save(false);
	}

	pub fn queue_hibernation_removal(&self, conn_id: impl Into<String>) {
		self.request_hibernation_transport_removal(conn_id);
	}

	pub fn has_pending_hibernation_changes(&self) -> bool {
		self.0.connections.has_pending_hibernation_changes()
	}

	pub fn take_pending_hibernation_changes(&self) -> Vec<ConnId> {
		self.0.connections.pending_hibernation_removals()
	}

	pub(crate) fn hibernated_connection_is_live(
		&self,
		gateway_id: &[u8],
		request_id: &[u8],
	) -> Result<bool> {
		if let Some(override_pairs) = self
			.0
			.hibernated_connection_liveness_override
			.read()
			.expect("hibernated connection liveness override lock poisoned")
			.as_ref()
		{
			return Ok(
				override_pairs.contains(&(gateway_id.to_vec(), request_id.to_vec()))
			);
		}

		let Some(envoy_handle) = self.0.sleep.envoy_handle() else {
			return Ok(false);
		};
		let gateway_id: [u8; 4] = gateway_id
			.try_into()
			.map_err(|_| anyhow!("invalid hibernatable websocket gateway id"))?;
		let request_id: [u8; 4] = request_id
			.try_into()
			.map_err(|_| anyhow!("invalid hibernatable websocket request id"))?;
		let is_live = envoy_handle.hibernatable_connection_is_live(
			self.actor_id(),
			self.0.sleep.generation(),
			gateway_id,
			request_id,
		);
		Ok(is_live)
	}

	#[cfg(test)]
	pub(crate) fn set_hibernated_connection_liveness_override<I>(
		&self,
		pairs: I,
	) where
		I: IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
	{
		*self
			.0
			.hibernated_connection_liveness_override
			.write()
			.expect("hibernated connection liveness override lock poisoned") =
			Some(pairs.into_iter().collect());
	}

	fn prepare_state_deltas(
		&self,
		deltas: Vec<StateDelta>,
	) -> Result<(Vec<StateDelta>, PendingHibernationChanges)> {
		fn finish_with_error(
			manager: &ConnectionManager,
			pending: PendingHibernationChanges,
			error: anyhow::Error,
		) -> Result<(Vec<StateDelta>, PendingHibernationChanges)> {
			manager.restore_pending_hibernation_changes(pending);
			Err(error)
		}

		let mut next_deltas = Vec::new();
		let mut explicit_updates = std::collections::BTreeMap::new();
		let mut explicit_removals = std::collections::BTreeSet::new();

		for delta in deltas {
			match delta {
				StateDelta::ConnHibernation { conn, bytes } => {
					if let Some(handle) = self.0.connections.connection(&conn) {
						handle.set_state(bytes.clone());
					}
					explicit_updates.insert(conn, bytes);
				}
				StateDelta::ConnHibernationRemoved(conn) => {
					explicit_removals.insert(conn);
				}
				other => next_deltas.push(other),
			}
		}

		let pending = self.0.connections.take_pending_hibernation_changes();
		let mut removal_ids = pending.removed.clone();
		removal_ids.extend(explicit_removals.iter().cloned());

		for (conn, bytes) in explicit_updates {
			if removal_ids.contains(&conn) {
				continue;
			}
			let encoded = match self
				.0
				.connections
				.encode_hibernation_delta(&conn, bytes)
			{
				Ok(encoded) => encoded,
				Err(error) => {
					return finish_with_error(&self.0.connections, pending, error);
				}
			};
			next_deltas.push(StateDelta::ConnHibernation {
				conn,
				bytes: encoded,
			});
		}

		for conn in &pending.updated {
			if removal_ids.contains(conn) || explicit_removals.contains(conn) {
				continue;
			}
			let Some(handle) = self.0.connections.connection(conn) else {
				continue;
			};
			if !handle.is_hibernatable() || handle.hibernation().is_none() {
				continue;
			}
			let encoded = match self
				.0
				.connections
				.encode_hibernation_delta(conn, handle.state())
			{
				Ok(encoded) => encoded,
				Err(error) => {
					return finish_with_error(&self.0.connections, pending, error);
				}
			};
			next_deltas.push(StateDelta::ConnHibernation {
				conn: conn.clone(),
				bytes: encoded,
			});
		}

		for conn in removal_ids {
			next_deltas.push(StateDelta::ConnHibernationRemoved(conn));
		}

		Ok((next_deltas, pending))
	}

	#[allow(dead_code)]
	pub(crate) async fn restore_hibernatable_connections(
		&self,
	) -> Result<Vec<ConnHandle>> {
		let restored = self.0.connections.restore_persisted(self).await?;
		if !restored.is_empty() {
			if let Some(envoy_handle) = self.0.sleep.envoy_handle() {
				let meta_entries: Vec<_> = restored
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
				envoy_handle
					.restore_hibernating_requests(self.actor_id().to_owned(), meta_entries);
			}
			self.record_connections_updated();
			self.notify_activity_dirty_or_reset_sleep_timer();
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

	pub fn inspector_snapshot(&self) -> InspectorSnapshot {
		self.inspector()
			.map(|inspector| inspector.snapshot())
			.unwrap_or_default()
	}

	pub(crate) fn configure_inspector_runtime(
		&self,
		attach_count: Arc<AtomicU32>,
		overlay_tx: broadcast::Sender<Arc<Vec<u8>>>,
	) {
		*self
			.0
			.inspector_attach_count
			.write()
			.expect("actor inspector attach count lock poisoned") =
			Some(attach_count);
		*self
			.0
			.inspector_overlay_tx
			.write()
			.expect("actor inspector overlay sender lock poisoned") =
			Some(overlay_tx);
	}

	pub(crate) fn inspector_attach(&self) {
		let Some(attach_count) = self.inspector_attach_count_arc() else {
			return;
		};
		if attach_count.fetch_add(1, Ordering::SeqCst) == 0 {
			self.notify_inspector_attachments_changed();
		}
	}

	pub(crate) fn inspector_detach(&self) {
		let Some(attach_count) = self.inspector_attach_count_arc() else {
			return;
		};
		let Ok(previous) = attach_count.fetch_update(
			Ordering::SeqCst,
			Ordering::SeqCst,
			|current| current.checked_sub(1),
		) else {
			return;
		};
		if previous == 1 {
			self.notify_inspector_attachments_changed();
		}
	}

	#[cfg(test)]
	pub(crate) fn inspector_attach_count(&self) -> u32 {
		self
			.inspector_attach_count_arc()
			.map(|attach_count| attach_count.load(Ordering::SeqCst))
			.unwrap_or(0)
	}

	pub(crate) fn subscribe_inspector(&self) -> broadcast::Receiver<Arc<Vec<u8>>> {
		self
			.0
			.inspector_overlay_tx
			.read()
			.expect("actor inspector overlay sender lock poisoned")
			.clone()
			.expect("actor inspector runtime must be configured before subscribing")
			.subscribe()
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

	#[allow(dead_code)]
	pub(crate) async fn can_sleep(&self) -> CanSleep {
		self.0.sleep.can_sleep(self).await
	}

	pub(crate) async fn wait_for_sleep_idle_window(&self, deadline: Instant) -> bool {
		self.0.sleep.wait_for_sleep_idle_window(self, deadline).await
	}

	pub(crate) async fn wait_for_shutdown_tasks(&self, deadline: Instant) -> bool {
		self.0.sleep.wait_for_shutdown_tasks(self, deadline).await
	}

	pub(crate) async fn teardown_sleep_controller(&self) {
		self.0.sleep.teardown().await;
	}

	pub(crate) fn reset_sleep_timer(&self) {
		self.notify_activity_dirty_or_reset_sleep_timer();
	}

	pub(crate) fn cancel_sleep_timer(&self) {
		self.0.sleep.cancel_sleep_timer();
	}

	pub(crate) fn cancel_local_alarm_timeouts(&self) {
		self.0.schedule.cancel_local_alarm_timeouts();
	}

	pub(crate) fn configure_lifecycle_events(
		&self,
		sender: Option<mpsc::Sender<LifecycleEvent>>,
	) {
		self.0.state.configure_lifecycle_events(sender.clone());
		*self
			.0
			.lifecycle_events
			.write()
			.expect("lifecycle events lock poisoned") = sender;
	}

	pub(crate) fn notify_inspector_serialize_requested(&self) {
		self.try_send_lifecycle_event(
			LifecycleEvent::InspectorSerializeRequested,
			"inspector_serialize_requested",
		);
	}

	pub(crate) fn save_requested(&self) -> bool {
		self.0.state.save_requested()
	}

	pub(crate) fn save_requested_immediate(&self) -> bool {
		self.0.state.save_requested_immediate()
	}

	pub(crate) fn save_deadline(&self, immediate: bool) -> Instant {
		self.0.state.compute_save_deadline(immediate).into()
	}

	pub(crate) fn save_request_revision(&self) -> u64 {
		self.0.state.save_request_revision()
	}

	pub(crate) fn notify_activity_dirty(&self) -> bool {
		self.0.activity.mark_dirty();
		let sender = self
			.0
			.lifecycle_events
			.read()
			.expect("lifecycle events lock poisoned")
			.clone();
		let Some(sender) = sender else {
			return false;
		};

		if !self.0.activity.try_begin_notification() {
			return true;
		}

		match sender.try_reserve() {
			Ok(permit) => {
				permit.send(LifecycleEvent::ActivityDirty);
			}
			Err(_) => {
				self.0.activity.clear_notification_pending();
				let _ = actor_channel_overloaded_error(
					LIFECYCLE_EVENT_INBOX_CHANNEL,
					self.0.lifecycle_event_inbox_capacity,
					"activity_dirty",
					Some(&self.0.metrics),
				);
			}
		}

		true
	}

	pub(crate) fn acknowledge_activity_dirty(&self) -> bool {
		self.0.activity.clear_notification_pending();
		self.0.activity.take_dirty()
	}

	pub(crate) fn notify_activity_dirty_or_reset_sleep_timer(&self) {
		if self.notify_activity_dirty() {
			return;
		}

		self.0.sleep.reset_sleep_timer(self.clone());
	}

	fn notify_inspector_attachments_changed(&self) {
		self.try_send_lifecycle_event(
			LifecycleEvent::InspectorAttachmentsChanged,
			"inspector_attachments_changed",
		);
	}

	#[allow(dead_code)]
	pub(crate) fn configure_sleep(&self, config: ActorConfig) {
		self.0.sleep.configure(config.clone());
		self.0.queue.configure_sleep(config);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn sleep_config(&self) -> ActorConfig {
		self.0.sleep.config()
	}

	#[allow(dead_code)]
	pub(crate) fn sleep_requested(&self) -> bool {
		self.0.sleep_requested.load(Ordering::SeqCst)
	}

	fn keep_awake_guard(&self) -> KeepAwakeGuard {
		let guard = KeepAwakeGuard::new(self.clone(), self.0.sleep.keep_awake());
		self.notify_activity_dirty_or_reset_sleep_timer();
		guard
	}

	fn internal_keep_awake_guard(&self) -> KeepAwakeGuard {
		let guard = KeepAwakeGuard::new(self.clone(), self.0.sleep.internal_keep_awake());
		self.notify_activity_dirty_or_reset_sleep_timer();
		guard
	}

	pub(crate) async fn internal_keep_awake_task(
		&self,
		future: BoxFuture<'static, Result<()>>,
	) -> Result<()> {
		self.internal_keep_awake(future).await
	}

	pub(crate) async fn wait_for_internal_keep_awake_idle(
		&self,
		deadline: Instant,
	) -> bool {
		self.0
			.sleep
			.wait_for_internal_keep_awake_idle(deadline)
			.await
	}

	pub(crate) async fn wait_for_http_requests_drained(
		&self,
		deadline: Instant,
	) -> bool {
		self.0
			.sleep
			.wait_for_http_requests_drained(self, deadline)
			.await
	}

	pub fn websocket_callback_region(&self) -> WebSocketCallbackRegion {
		WebSocketCallbackRegion {
			guard: Some(
				self.websocket_callback_guard(UserTaskKind::WebSocketCallback),
			),
		}
	}

	pub(crate) async fn with_websocket_callback<F, Fut, T>(&self, run: F) -> T
	where
		F: FnOnce() -> Fut,
		Fut: Future<Output = T>,
	{
		let _guard = self.websocket_callback_region();
		run().await
	}

	fn websocket_callback_guard(
		&self,
		kind: UserTaskKind,
	) -> WebSocketCallbackGuard {
		let region = self.0.sleep.websocket_callback();
		self.record_user_task_started(kind);
		self.reset_sleep_timer();
		WebSocketCallbackGuard::new(self.clone(), kind, region)
	}

	fn configure_sleep_hooks(&self) {
		let internal_keep_awake_ctx = self.clone();
		self.0.schedule.set_internal_keep_awake(Some(Arc::new(move |future| {
			let ctx = internal_keep_awake_ctx.clone();
			Box::pin(async move { ctx.internal_keep_awake_task(future).await })
		})));

		let queue_ctx = self.clone();
		self.0.queue.set_wait_activity_callback(Some(Arc::new(move || {
			queue_ctx.notify_activity_dirty_or_reset_sleep_timer();
		})));

		let queue_ctx = self.clone();
		self.0.queue.set_inspector_update_callback(Some(Arc::new(
			move |queue_size| {
				queue_ctx.record_queue_updated(queue_size);
			},
		)));
	}

	pub(crate) fn record_state_updated(&self) {
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

	pub(crate) async fn save_state_with_revision(
		&self,
		deltas: Vec<StateDelta>,
		save_request_revision: u64,
	) -> Result<()> {
		let (deltas, pending_hibernation_changes) =
			match self.prepare_state_deltas(deltas) {
				Ok(prepared) => prepared,
				Err(error) => return Err(error),
			};
		if let Err(error) = self
			.0
			.state
			.apply_state_deltas(deltas, save_request_revision)
			.await
		{
			self
				.0
				.connections
				.restore_pending_hibernation_changes(pending_hibernation_changes);
			return Err(error);
		}
		self.record_state_updated();
		Ok(())
	}

	async fn dispatch_scheduled_action(
		&self,
		event_id: &str,
		action: String,
		args: Vec<u8>,
	) {
		self.record_user_task_started(UserTaskKind::ScheduledAction);
		let started_at = Instant::now();

		self
			.internal_keep_awake(async {
				let (reply_tx, reply_rx) = oneshot::channel();

				match self.try_send_actor_event(
					ActorEvent::Action {
						name: action.clone(),
						args,
						conn: None,
						reply: Reply::from(reply_tx),
					},
					"scheduled_action",
				) {
					Ok(()) => match reply_rx.await {
						Ok(Ok(_)) => {}
						Ok(Err(error)) => {
							tracing::error!(
								?error,
								event_id,
								action_name = action,
								"scheduled event execution failed"
							);
						}
						Err(error) => {
							tracing::error!(
								?error,
								event_id,
								action_name = action,
								"scheduled event reply dropped"
							);
						}
					},
					Err(error) => {
						tracing::error!(
							?error,
							event_id,
							action_name = action,
							"failed to enqueue scheduled event"
						);
					}
				}
			})
			.await;

		self.record_user_task_finished(
			UserTaskKind::ScheduledAction,
			started_at.elapsed(),
		);
		self.0.schedule.cancel(event_id);
	}

	fn inspector_attach_count_arc(&self) -> Option<Arc<AtomicU32>> {
		self
			.0
			.inspector_attach_count
			.read()
			.expect("actor inspector attach count lock poisoned")
			.clone()
	}

	fn try_send_lifecycle_event(
		&self,
		event: LifecycleEvent,
		operation: &'static str,
	) {
		let Some(sender) = self
			.0
			.lifecycle_events
			.read()
			.expect("lifecycle events lock poisoned")
			.clone()
		else {
			return;
		};

		match sender.try_reserve() {
			Ok(permit) => {
				permit.send(event);
			}
			Err(_) => {
				let _ = actor_channel_overloaded_error(
					LIFECYCLE_EVENT_INBOX_CHANNEL,
					self.0.lifecycle_event_inbox_capacity,
					operation,
					Some(&self.0.metrics),
				);
			}
		}
	}
}

fn now_timestamp_ms() -> i64 {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default();
	i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

struct KeepAwakeGuard {
	ctx: ActorContext,
	region: Option<RegionGuard>,
}

impl KeepAwakeGuard {
	fn new(ctx: ActorContext, region: RegionGuard) -> Self {
		Self {
			ctx,
			region: Some(region),
		}
	}
}

impl Drop for KeepAwakeGuard {
	fn drop(&mut self) {
		self.region.take();
		self.ctx.notify_activity_dirty_or_reset_sleep_timer();
	}
}

struct WebSocketCallbackGuard {
	ctx: ActorContext,
	kind: UserTaskKind,
	started_at: Instant,
	region: Option<RegionGuard>,
}

pub struct WebSocketCallbackRegion {
	guard: Option<WebSocketCallbackGuard>,
}

impl WebSocketCallbackGuard {
	fn new(ctx: ActorContext, kind: UserTaskKind, region: RegionGuard) -> Self {
		Self {
			ctx,
			kind,
			started_at: Instant::now(),
			region: Some(region),
		}
	}
}

impl Drop for WebSocketCallbackGuard {
	fn drop(&mut self) {
		self.ctx
			.record_user_task_finished(self.kind, self.started_at.elapsed());
		self.region.take();
		self.ctx.reset_sleep_timer();
	}
}

impl Drop for WebSocketCallbackRegion {
	fn drop(&mut self) {
		self.guard.take();
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
