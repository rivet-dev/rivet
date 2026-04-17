use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, OnceLock};

use ciborium::{de::from_reader, ser::into_writer};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::actor::Actor;
use rivetkit_core::{
	ActorContext, ActorKey, ConnHandle, Kv, Queue, Schedule, SqliteDb,
};

pub struct Ctx<A: Actor> {
	inner: ActorContext,
	state_cache: Arc<Mutex<Option<Arc<A::State>>>>,
	vars: Arc<OnceLock<Arc<A::Vars>>>,
}

impl<A: Actor> Ctx<A> {
	pub fn new(inner: ActorContext, vars: Arc<A::Vars>) -> Self {
		let vars_slot = OnceLock::new();
		let _ = vars_slot.set(vars);

		Self {
			inner,
			state_cache: Arc::new(Mutex::new(None)),
			vars: Arc::new(vars_slot),
		}
	}

	pub(crate) fn new_bootstrap(inner: ActorContext) -> Self {
		Self {
			inner,
			state_cache: Arc::new(Mutex::new(None)),
			vars: Arc::new(OnceLock::new()),
		}
	}

	pub fn inner(&self) -> &ActorContext {
		&self.inner
	}

	pub fn into_inner(self) -> ActorContext {
		self.inner
	}

	pub fn state(&self) -> Arc<A::State> {
		let mut state_cache = self
			.state_cache
			.lock()
			.expect("typed actor state cache lock poisoned");
		if let Some(state) = state_cache.as_ref() {
			return Arc::clone(state);
		}

		let state_bytes = self.inner.state();
		let state = Arc::new(
			deserialize_cbor(&state_bytes)
				.expect("failed to deserialize actor state from CBOR"),
		);
		*state_cache = Some(Arc::clone(&state));
		state
	}

	pub fn set_state(&self, state: &A::State) {
		let state_bytes = serialize_cbor(state)
			.expect("failed to serialize actor state to CBOR");
		self.inner.set_state(state_bytes);
		*self
			.state_cache
			.lock()
			.expect("typed actor state cache lock poisoned") = None;
	}

	pub fn vars(&self) -> &A::Vars {
		self
			.vars
			.get()
			.expect("typed actor vars accessed before initialization")
			.as_ref()
	}

	pub fn kv(&self) -> &Kv {
		self.inner.kv()
	}

	pub fn sql(&self) -> &SqliteDb {
		self.inner.sql()
	}

	pub fn schedule(&self) -> &Schedule {
		self.inner.schedule()
	}

	pub fn queue(&self) -> &Queue {
		self.inner.queue()
	}

	pub fn actor_id(&self) -> &str {
		self.inner.actor_id()
	}

	pub fn name(&self) -> &str {
		self.inner.name()
	}

	pub fn key(&self) -> &ActorKey {
		self.inner.key()
	}

	pub fn region(&self) -> &str {
		self.inner.region()
	}

	pub fn abort_signal(&self) -> &CancellationToken {
		self.inner.abort_signal()
	}

	pub fn aborted(&self) -> bool {
		self.inner.aborted()
	}

	pub fn sleep(&self) {
		self.inner.sleep();
	}

	pub fn destroy(&self) {
		self.inner.destroy();
	}

	pub fn set_prevent_sleep(&self, prevent: bool) {
		self.inner.set_prevent_sleep(prevent);
	}

	pub fn prevent_sleep(&self) -> bool {
		self.inner.prevent_sleep()
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		self.inner.wait_until(future);
	}

	pub fn broadcast<E: Serialize>(&self, name: &str, event: &E) {
		let event_bytes = serialize_cbor(event)
			.expect("failed to serialize broadcast event to CBOR");
		self.inner.broadcast(name, &event_bytes);
	}

	pub fn conns(&self) -> Vec<ConnCtx<A>> {
		self
			.inner
			.conns()
			.into_iter()
			.map(ConnCtx::new)
			.collect()
	}

	pub(crate) fn initialize_vars(&self, vars: Arc<A::Vars>) {
		let _ = self.vars.set(vars);
	}

	pub(crate) fn invalidate_state_cache(&self) {
		*self
			.state_cache
			.lock()
			.expect("typed actor state cache lock poisoned") = None;
	}
}

impl<A: Actor> fmt::Debug for Ctx<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let state_cached = self
			.state_cache
			.lock()
			.expect("typed actor state cache lock poisoned")
			.is_some();
		let vars_initialized = self.vars.get().is_some();
		f.debug_struct("Ctx")
			.field("inner", &self.inner)
			.field("state_cached", &state_cached)
			.field("vars_initialized", &vars_initialized)
			.finish()
	}
}

pub struct ConnCtx<A: Actor> {
	inner: ConnHandle,
	_phantom: PhantomData<fn() -> A>,
}

impl<A: Actor> ConnCtx<A> {
	pub fn new(inner: ConnHandle) -> Self {
		Self {
			inner,
			_phantom: PhantomData,
		}
	}

	pub fn inner(&self) -> &ConnHandle {
		&self.inner
	}

	pub fn into_inner(self) -> ConnHandle {
		self.inner
	}

	pub fn id(&self) -> &str {
		self.inner.id()
	}

	pub fn params(&self) -> A::ConnParams {
		let params = self.inner.params();
		deserialize_cbor(&params)
			.expect("failed to deserialize connection params from CBOR")
	}

	pub fn state(&self) -> A::ConnState {
		let state = self.inner.state();
		deserialize_cbor(&state)
			.expect("failed to deserialize connection state from CBOR")
	}

	pub fn set_state(&self, state: &A::ConnState) {
		let state_bytes = serialize_cbor(state)
			.expect("failed to serialize connection state to CBOR");
		self.inner.set_state(state_bytes);
	}

	pub fn is_hibernatable(&self) -> bool {
		self.inner.is_hibernatable()
	}

	pub fn send<E: Serialize>(&self, name: &str, event: &E) {
		let event_bytes = serialize_cbor(event)
			.expect("failed to serialize connection event to CBOR");
		self.inner.send(name, &event_bytes);
	}

	pub async fn disconnect(&self, reason: Option<&str>) -> anyhow::Result<()> {
		self.inner.disconnect(reason).await
	}
}

impl<A: Actor> From<ConnHandle> for ConnCtx<A> {
	fn from(value: ConnHandle) -> Self {
		Self::new(value)
	}
}

impl<A: Actor> fmt::Debug for ConnCtx<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ConnCtx").field("inner", &self.inner).finish()
	}
}

impl<A: Actor> Clone for Ctx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			state_cache: Arc::clone(&self.state_cache),
			vars: Arc::clone(&self.vars),
		}
	}
}

impl<A: Actor> Clone for ConnCtx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_phantom: PhantomData,
		}
	}
}

fn serialize_cbor<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
	let mut bytes = Vec::new();
	into_writer(value, &mut bytes)?;
	Ok(bytes)
}

fn deserialize_cbor<T: serde::de::DeserializeOwned>(
	bytes: &[u8],
) -> anyhow::Result<T> {
	Ok(from_reader(bytes)?)
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use anyhow::Result;
	use async_trait::async_trait;
	use serde::{Deserialize, Serialize};

	use super::{ConnCtx, Ctx};
	use crate::actor::Actor;
	use rivetkit_core::{ActorConfig, ActorContext};

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestState {
		value: i64,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestConnState {
		value: i64,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestConnParams {
		label: String,
	}

	#[derive(Debug, PartialEq, Eq)]
	struct TestVars {
		label: &'static str,
	}

	struct TestActor;

	#[async_trait]
	impl Actor for TestActor {
		type State = TestState;
		type ConnParams = TestConnParams;
		type ConnState = TestConnState;
		type Input = ();
		type Vars = TestVars;

		async fn create_state(
			_ctx: &Ctx<Self>,
			_input: &Self::Input,
		) -> Result<Self::State> {
			Ok(TestState { value: 0 })
		}

		async fn create_vars(_ctx: &Ctx<Self>) -> Result<Self::Vars> {
			Ok(TestVars { label: "vars" })
		}

		async fn create_conn_state(
			self: &Arc<Self>,
			_ctx: &Ctx<Self>,
			_params: &Self::ConnParams,
		) -> Result<Self::ConnState> {
			let _ = self;
			Ok(TestConnState { value: 0 })
		}

		async fn on_create(_ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self> {
			Ok(Self)
		}

		fn config() -> ActorConfig {
			ActorConfig::default()
		}
	}

	#[test]
	fn state_is_cached_until_set_state_invalidates_it() {
		let inner = ActorContext::new("actor-id", "test", Vec::new(), "local");
		inner.set_state(
			super::serialize_cbor(&TestState { value: 7 })
				.expect("serialize test state"),
		);

		let ctx = Ctx::<TestActor>::new(
			inner.clone(),
			Arc::new(TestVars { label: "vars" }),
		);
		let first = ctx.state();
		let second = ctx.state();

		assert!(Arc::ptr_eq(&first, &second));

		inner.set_state(
			super::serialize_cbor(&TestState { value: 99 })
				.expect("serialize replacement state"),
		);
		let still_cached = ctx.state();
		assert_eq!(still_cached.value, 7);

		ctx.set_state(&TestState { value: 11 });
		let refreshed = ctx.state();
		assert_eq!(refreshed.value, 11);
		assert!(!Arc::ptr_eq(&first, &refreshed));
	}

	#[test]
	fn vars_are_exposed_by_reference() {
		let ctx = Ctx::<TestActor>::new(
			ActorContext::new("actor-id", "test", Vec::new(), "local"),
			Arc::new(TestVars { label: "vars" }),
		);

		assert_eq!(ctx.vars().label, "vars");
	}

	#[test]
	fn connection_context_serializes_and_deserializes_cbor() {
		let conn = rivetkit_core::ConnHandle::new(
			"conn-id",
			super::serialize_cbor(&TestConnParams {
				label: "hello".into(),
			})
			.expect("serialize params"),
			super::serialize_cbor(&TestConnState { value: 5 })
				.expect("serialize state"),
			true,
		);
		let conn_ctx = ConnCtx::<TestActor>::new(conn);

		assert_eq!(conn_ctx.id(), "conn-id");
		assert_eq!(
			conn_ctx.params(),
			TestConnParams {
				label: "hello".into(),
			}
		);
		assert_eq!(conn_ctx.state(), TestConnState { value: 5 });
		assert!(conn_ctx.is_hibernatable());

		conn_ctx.set_state(&TestConnState { value: 8 });
		assert_eq!(conn_ctx.state(), TestConnState { value: 8 });
	}
}
