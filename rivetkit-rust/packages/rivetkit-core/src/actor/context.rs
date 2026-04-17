use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::actor::connection::ConnHandle;
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
struct ActorContextInner {
	state: ActorState,
	vars: ActorVars,
	kv: Kv,
	sql: SqliteDb,
	schedule: Schedule,
	queue: Queue,
	conns: RwLock<Vec<ConnHandle>>,
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
		let kv = Kv::default();
		let sql = SqliteDb::default();
		let schedule = Schedule::default();
		let queue = Queue::default();

		Self(Arc::new(ActorContextInner {
			state: ActorState::new(kv.clone(), ActorConfig::default()),
			vars: ActorVars::default(),
			kv,
			sql,
			schedule,
			queue,
			conns: RwLock::new(Vec::new()),
			abort_signal: CancellationToken::new(),
			prevent_sleep: AtomicBool::new(false),
			sleep_requested: AtomicBool::new(false),
			destroy_requested: AtomicBool::new(false),
			actor_id: actor_id.into(),
			name: name.into(),
			key,
			region: region.into(),
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

	pub fn broadcast(&self, _name: &str, _args: &[u8]) {}

	pub fn conns(&self) -> Vec<ConnHandle> {
		self.0
			.conns
			.read()
			.expect("actor connections lock poisoned")
			.clone()
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

}

impl Default for ActorContext {
	fn default() -> Self {
		Self::new("", "", Vec::new(), "")
	}
}
