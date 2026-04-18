use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;

use crate::actor::Actor;
use crate::validation::{decode_cbor, encode_cbor, panic_with_error};
use rivetkit_client::{Client, ClientConfig, EncodingKind, TransportKind};
use rivetkit_core::{
	ActorContext, ActorKey, ConnHandle, EnqueueAndWaitOpts, Kv, Queue,
	Schedule, SqliteDb,
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
		match self.try_state() {
			Ok(state) => state,
			Err(error) => panic_with_error(error),
		}
	}

	pub(crate) fn try_state(&self) -> anyhow::Result<Arc<A::State>> {
		let mut state_cache = self
			.state_cache
			.lock()
			.expect("typed actor state cache lock poisoned");
		if let Some(state) = state_cache.as_ref() {
			return Ok(Arc::clone(state));
		}

		let state_bytes = self.inner.state();
		let state = Arc::new(decode_cbor(&state_bytes, "actor state")?);
		*state_cache = Some(Arc::clone(&state));
		Ok(state)
	}

	pub fn set_state(&self, state: &A::State) {
		if let Err(error) = self.try_set_state(state) {
			panic_with_error(error);
		}
	}

	pub(crate) fn try_set_state(&self, state: &A::State) -> anyhow::Result<()> {
		let state_bytes = encode_cbor(state, "actor state")?;
		self.inner.set_state(state_bytes);
		self.invalidate_state_cache();
		Ok(())
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

	pub fn client(&self) -> anyhow::Result<Client> {
		Ok(Client::from_config(
			ClientConfig::new(self.inner.client_endpoint()?)
				.token_opt(self.inner.client_token()?)
				.namespace(self.inner.client_namespace()?)
				.pool_name(self.inner.client_pool_name()?)
				.encoding(EncodingKind::Bare)
				.transport(TransportKind::WebSocket)
				.disable_metadata_lookup(true),
		))
	}

	pub async fn enqueue_and_wait<Req, Res>(
		&self,
		name: &str,
		body: &Req,
		opts: EnqueueAndWaitOpts,
	) -> anyhow::Result<Option<Res>>
	where
		Req: Serialize,
		Res: DeserializeOwned,
	{
		let request_bytes = encode_cbor(body, "queue message body")?;
		let response_bytes = self
			.inner
			.queue()
			.enqueue_and_wait(name, &request_bytes, opts)
			.await?;

		response_bytes
			.map(|response_bytes| {
				decode_cbor(&response_bytes, "queue completion response")
			})
			.transpose()
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
		match self.try_params() {
			Ok(params) => params,
			Err(error) => panic_with_error(error),
		}
	}

	pub fn state(&self) -> A::ConnState {
		match self.try_state() {
			Ok(state) => state,
			Err(error) => panic_with_error(error),
		}
	}

	pub fn set_state(&self, state: &A::ConnState) {
		if let Err(error) = self.try_set_state(state) {
			panic_with_error(error);
		}
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

	pub(crate) fn try_params(&self) -> anyhow::Result<A::ConnParams> {
		let params = self.inner.params();
		decode_cbor(&params, "connection params")
	}

	pub(crate) fn try_state(&self) -> anyhow::Result<A::ConnState> {
		let state = self.inner.state();
		decode_cbor(&state, "connection state")
	}

	pub(crate) fn try_set_state(&self, state: &A::ConnState) -> anyhow::Result<()> {
		let state_bytes = encode_cbor(state, "connection state")?;
		self.inner.set_state(state_bytes);
		Ok(())
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
	encode_cbor(value, "CBOR value")
}

#[cfg(test)]
#[path = "../tests/modules/context.rs"]
mod tests;
