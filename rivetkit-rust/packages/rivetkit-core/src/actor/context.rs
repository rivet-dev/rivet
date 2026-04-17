use std::future::Future;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use crate::actor::callbacks::ActorInstanceCallbacks;
use crate::actor::connection::{
	ConnHandle, ConnectionManager, HibernatableConnectionMetadata,
};
use crate::actor::event::EventBroadcaster;
use crate::actor::queue::Queue;
use crate::actor::schedule::Schedule;
use crate::actor::state::{ActorState, OnStateChangeCallback, PersistedActor};
use crate::actor::vars::ActorVars;
use crate::ActorConfig;
use crate::kv::Kv;
use crate::sqlite::SqliteDb;
use crate::types::{ActorKey, SaveStateOpts};

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
		let actor_id = actor_id.into();
		let name = name.into();
		let region = region.into();
		let config = ActorConfig::default();
		let kv = Kv::default();
		let sql = SqliteDb::default();
		let queue = Queue::default();
		let state = ActorState::new(kv.clone(), config.clone());
		let schedule = Schedule::new(state.clone(), actor_id.clone(), config);
		let connections =
			ConnectionManager::new(actor_id.clone(), kv.clone(), ActorConfig::default());

		Self(Arc::new(ActorContextInner {
			state,
			vars: ActorVars::default(),
			kv,
			sql,
			schedule,
			queue,
			broadcaster: EventBroadcaster::default(),
			connections,
			abort_signal: CancellationToken::new(),
			prevent_sleep: AtomicBool::new(false),
			sleep_requested: AtomicBool::new(false),
			destroy_requested: AtomicBool::new(false),
			actor_id,
			name,
			key,
			region,
		}))
	}

	pub fn state(&self) -> Vec<u8> {
		self.0.state.state()
	}

	pub fn set_state(&self, state: Vec<u8>) {
		self.0.state.set_state(state);
	}

	pub async fn save_state(&self, opts: SaveStateOpts) -> Result<()> {
		self.0.state.save_state(opts).await
	}

	pub fn vars(&self) -> Vec<u8> {
		self.0.vars.vars()
	}

	pub fn set_vars(&self, vars: Vec<u8>) {
		self.0.vars.set_vars(vars);
	}

	pub fn kv(&self) -> &Kv {
		&self.0.kv
	}

	pub fn sql(&self) -> &SqliteDb {
		&self.0.sql
	}

	pub fn schedule(&self) -> &Schedule {
		&self.0.schedule
	}

	pub fn queue(&self) -> &Queue {
		&self.0.queue
	}

	pub fn sleep(&self) {
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
		self.0.state.flush_on_shutdown();
		self.0.destroy_requested.store(true, Ordering::SeqCst);
		self.0.abort_signal.cancel();
	}

	pub fn set_prevent_sleep(&self, prevent: bool) {
		self.0.prevent_sleep.store(prevent, Ordering::SeqCst);
	}

	pub fn prevent_sleep(&self) -> bool {
		self.0.prevent_sleep.load(Ordering::SeqCst)
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		tokio::spawn(future);
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

	#[allow(dead_code)]
	pub(crate) fn load_persisted_actor(&self, persisted: PersistedActor) {
		self.0.state.load_persisted(persisted);
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
	}

	#[allow(dead_code)]
	pub(crate) fn add_conn(&self, conn: ConnHandle) {
		self.0.connections.insert_existing(conn);
	}

	#[allow(dead_code)]
	pub(crate) fn remove_conn(&self, conn_id: &str) -> Option<ConnHandle> {
		self.0.connections.remove_existing(conn_id)
	}

	#[allow(dead_code)]
	pub(crate) fn configure_connection_runtime(
		&self,
		config: ActorConfig,
		callbacks: Arc<ActorInstanceCallbacks>,
	) {
		self.0.connections.configure_runtime(config, callbacks);
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
}

impl Default for ActorContext {
	fn default() -> Self {
		Self::new("", "", Vec::new(), "")
	}
}
