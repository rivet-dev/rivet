use std::future::Future;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::actor::callbacks::ActorInstanceCallbacks;
use crate::actor::connection::{
	ConnHandle, ConnectionManager, HibernatableConnectionMetadata,
};
use crate::actor::event::EventBroadcaster;
use crate::actor::queue::Queue;
use crate::actor::schedule::Schedule;
use crate::actor::sleep::{CanSleep, SleepController};
use crate::actor::state::{ActorState, OnStateChangeCallback, PersistedActor};
use crate::actor::vars::ActorVars;
use crate::ActorConfig;
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
	abort_signal: CancellationToken,
	prevent_sleep: AtomicBool,
	sleep_requested: AtomicBool,
	destroy_requested: AtomicBool,
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

	#[cfg(test)]
	pub(crate) fn new_with_kv(
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

	fn build(
		actor_id: String,
		name: String,
		key: ActorKey,
		region: String,
		config: ActorConfig,
		kv: Kv,
		sql: SqliteDb,
	) -> Self {
		let state = ActorState::new(kv.clone(), config.clone());
		let schedule = Schedule::new(state.clone(), actor_id.clone(), config);
		let abort_signal = CancellationToken::new();
		let queue = Queue::new(kv.clone(), ActorConfig::default(), Some(abort_signal.clone()));
		let connections =
			ConnectionManager::new(actor_id.clone(), kv.clone(), ActorConfig::default());
		let sleep = SleepController::default();

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
			abort_signal,
			prevent_sleep: AtomicBool::new(false),
			sleep_requested: AtomicBool::new(false),
			destroy_requested: AtomicBool::new(false),
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
		self.reset_sleep_timer();
	}

	pub async fn save_state(&self, opts: SaveStateOpts) -> Result<()> {
		self.0.state.save_state(opts).await
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

	pub async fn db_exec(&self, _sql: &str) -> Result<Vec<u8>> {
		Err(anyhow!("actor database exec is not configured"))
	}

	pub async fn db_query(
		&self,
		_sql: &str,
		_params: Option<&[u8]>,
	) -> Result<Vec<u8>> {
		Err(anyhow!("actor database query is not configured"))
	}

	pub async fn db_run(&self, _sql: &str, _params: Option<&[u8]>) -> Result<()> {
		Err(anyhow!("actor database run is not configured"))
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
		if let Ok(runtime) = Handle::try_current() {
			let ctx = self.clone();
			runtime.spawn(async move {
				if let Err(error) = ctx.persist_hibernatable_connections().await {
					tracing::error!(
						?error,
						"failed to persist hibernatable connections on sleep"
					);
				}
			});
		}

		self.0.sleep_requested.store(true, Ordering::SeqCst);
	}

	pub fn destroy(&self) {
		self.0.sleep.cancel_sleep_timer();
		self.0.state.flush_on_shutdown();
		self.0.destroy_requested.store(true, Ordering::SeqCst);
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

	pub fn broadcast(&self, name: &str, args: &[u8]) {
		self.0.broadcaster.broadcast(&self.conns(), name, args);
	}

	pub fn conns(&self) -> Vec<ConnHandle> {
		self.0.connections.list()
	}

	pub async fn client_call(&self, _request: &[u8]) -> Result<Vec<u8>> {
		Err(anyhow!("actor client bridge is not configured"))
	}

	pub fn ack_hibernatable_websocket_message(
		&self,
		_gateway_id: &[u8],
		_request_id: &[u8],
		_server_message_index: u16,
	) -> Result<()> {
		Err(anyhow!("hibernatable websocket ack is not configured"))
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
	pub(crate) fn set_on_state_change_callback(
		&self,
		callback: Option<OnStateChangeCallback>,
	) {
		self.0.state.set_on_state_change_callback(callback);
	}

	pub(crate) fn trigger_throttled_state_save(&self) {
		self.0.state.trigger_throttled_save();
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn add_conn(&self, conn: ConnHandle) {
		self.0.connections.insert_existing(conn);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn remove_conn(&self, conn_id: &str) -> Option<ConnHandle> {
		let removed = self.0.connections.remove_existing(conn_id);
		if removed.is_some() {
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
		self.0.connections.configure_runtime(config, callbacks);
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(
		&self,
		envoy_handle: EnvoyHandle,
		generation: Option<u32>,
	) {
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
		create_state: F,
	) -> Result<ConnHandle>
	where
		F: Future<Output = Result<Vec<u8>>> + Send,
	{
		self.0
			.connections
			.connect_with_state(
				self,
				params,
				is_hibernatable,
				hibernation,
				create_state,
			)
			.await
	}

	#[allow(dead_code)]
	pub(crate) async fn persist_hibernatable_connections(&self) -> Result<()> {
		self.0.connections.persist_hibernatable().await
	}

	#[allow(dead_code)]
	pub(crate) async fn restore_hibernatable_connections(
		&self,
	) -> Result<Vec<ConnHandle>> {
		self.0.connections.restore_persisted(self).await
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
	pub(crate) fn set_started(&self, started: bool) {
		self.0.sleep.set_started(started);
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn set_run_handler_active(&self, active: bool) {
		self.0.sleep.set_run_handler_active(active);
		self.reset_sleep_timer();
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
		self.0.sleep.wait_for_sleep_idle_window(self, deadline).await
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
	pub(crate) fn begin_websocket_callback(&self) {
		self.0.sleep.begin_websocket_callback();
		self.reset_sleep_timer();
	}

	#[allow(dead_code)]
	pub(crate) fn end_websocket_callback(&self) {
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
		self.0.schedule.set_internal_keep_awake(Some(Arc::new(move |future| {
			let ctx = internal_keep_awake_ctx.clone();
			Box::pin(async move { ctx.internal_keep_awake_task(future).await })
		})));

		let queue_ctx = self.clone();
		self.0.queue.set_wait_activity_callback(Some(Arc::new(move || {
			queue_ctx.reset_sleep_timer();
		})));
	}
}

impl Default for ActorContext {
	fn default() -> Self {
		Self::new("", "", Vec::new(), "")
	}
}

#[cfg(test)]
mod tests {
	use super::ActorContext;
	use crate::kv::Kv;
	use crate::types::ListOpts;

	#[tokio::test]
	async fn kv_helpers_delegate_to_kv_wrapper() {
		let ctx = ActorContext::new_with_kv("actor-1", "actor", Vec::new(), "local", Kv::new_in_memory());

		ctx.kv_batch_put(&[(b"alpha".as_slice(), b"1".as_slice())])
			.await
			.expect("kv batch put should succeed");

		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get should succeed");
		assert_eq!(values, vec![Some(b"1".to_vec())]);

		let listed = ctx
			.kv_list_prefix(b"alp", ListOpts::default())
			.await
			.expect("kv list prefix should succeed");
		assert_eq!(listed, vec![(b"alpha".to_vec(), b"1".to_vec())]);

		ctx.kv_batch_delete(&[b"alpha".as_slice()])
			.await
			.expect("kv batch delete should succeed");
		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get after delete should succeed");
		assert_eq!(values, vec![None]);
	}

	#[tokio::test]
	async fn foreign_runtime_only_helpers_fail_explicitly_when_unconfigured() {
		let ctx = ActorContext::default();

		assert!(ctx.db_exec("select 1").await.is_err());
		assert!(ctx.db_query("select 1", None).await.is_err());
		assert!(ctx.db_run("select 1", None).await.is_err());
		assert!(ctx.client_call(b"call").await.is_err());
		assert!(ctx.set_alarm(Some(1)).is_err());
		assert!(ctx
			.ack_hibernatable_websocket_message(b"gateway", b"request", 1)
			.is_err());
	}
}
