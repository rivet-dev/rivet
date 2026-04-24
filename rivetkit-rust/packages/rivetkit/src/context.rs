use std::future::Future;
use std::io::Cursor;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result};
use rivetkit_client::{Client, ClientConfig, EncodingKind, TransportKind};
use rivetkit_core::{
	ActorContext, ActorKey, ConnHandle, ConnId, Kv, RequestSaveOpts, SqliteDb, StateDelta,
	actor::connection::ConnHandles, error::ActorRuntime,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::actor::Actor;

#[derive(Debug)]
pub struct Ctx<A: Actor> {
	inner: ActorContext,
	client: Arc<OnceLock<Client>>,
	_p: PhantomData<fn() -> A>,
}

pub struct Schedule<'a> {
	inner: &'a ActorContext,
}

impl<A: Actor> Clone for Ctx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			_p: PhantomData,
		}
	}
}

impl<A: Actor> Ctx<A> {
	pub fn new(inner: ActorContext) -> Self {
		Self {
			inner,
			client: Arc::new(OnceLock::new()),
			_p: PhantomData,
		}
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

	pub fn kv(&self) -> &Kv {
		self.inner.kv()
	}

	pub fn sql(&self) -> &SqliteDb {
		self.inner.sql()
	}

	pub fn queue(&self) -> &ActorContext {
		self.inner.queue()
	}

	pub fn schedule(&self) -> Schedule<'_> {
		Schedule { inner: &self.inner }
	}

	/// Requests a save without surfacing delivery failures to the caller.
	///
	/// If save-request delivery must be observed, use the error-aware
	/// `request_save_and_wait` path on the underlying core context instead.
	pub fn request_save(&self, opts: RequestSaveOpts) {
		self.inner.request_save(opts);
	}

	pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()> {
		self.inner.save_state(deltas).await
	}

	pub fn sleep(&self) {
		self.inner.sleep();
	}

	pub fn destroy(&self) {
		self.inner.destroy();
	}

	#[deprecated(note = "no-op: use `keep_awake` or `wait_until` instead")]
	pub fn set_prevent_sleep(&self, enabled: bool) {
		#[allow(deprecated)]
		self.inner.set_prevent_sleep(enabled);
	}

	#[deprecated(note = "no-op: always returns false")]
	pub fn prevent_sleep(&self) -> bool {
		#[allow(deprecated)]
		self.inner.prevent_sleep()
	}

	pub fn wait_until(&self, future: impl Future<Output = ()> + Send + 'static) {
		self.inner.wait_until(future);
	}

	pub fn broadcast<E: Serialize>(&self, name: &str, event: &E) -> Result<()> {
		let event_bytes = encode_cbor(event, "broadcast event")?;
		self.inner.broadcast(name, &event_bytes);
		Ok(())
	}

	pub fn conns(&self) -> ConnIter<'_, A> {
		ConnIter {
			inner: self.inner.conns(),
			_p: PhantomData,
		}
	}

	pub fn conns_vec(&self) -> Vec<ConnCtx<A>> {
		self.conns().collect()
	}

	pub async fn disconnect_conn(&self, id: &ConnId) -> Result<()> {
		self.inner.disconnect_conn(id.clone()).await
	}

	pub async fn disconnect_conns<F>(&self, pred: F) -> Result<()>
	where
		F: Fn(&ConnCtx<A>) -> bool,
	{
		self.inner
			.disconnect_conns(|conn| pred(&ConnCtx::new(conn.clone())))
			.await
	}

	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		self.inner.set_alarm(timestamp_ms)
	}

	pub fn client(&self) -> Result<Client> {
		if let Some(client) = self.client.get() {
			return Ok(client.clone());
		}

		let endpoint = self.inner.client_endpoint().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client endpoint".to_owned(),
			}
			.build()
		})?;
		let namespace = self.inner.client_namespace().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client namespace".to_owned(),
			}
			.build()
		})?;
		let pool_name = self.inner.client_pool_name().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "actor client pool name".to_owned(),
			}
			.build()
		})?;
		let client = Client::new(
			ClientConfig::new(endpoint)
				.token_opt(self.inner.client_token().map(ToOwned::to_owned))
				.namespace(namespace)
				.pool_name(pool_name)
				.encoding(EncodingKind::Bare)
				.transport(TransportKind::WebSocket)
				.disable_metadata_lookup(true),
		);

		match self.client.set(client) {
			Ok(()) => self.client.get().cloned().ok_or_else(|| {
				ActorRuntime::NotConfigured {
					component: "actor client cache".to_owned(),
				}
				.build()
			}),
			Err(client) => Ok(self.client.get().cloned().unwrap_or(client)),
		}
	}

	pub fn inner(&self) -> &ActorContext {
		&self.inner
	}

	pub fn into_inner(self) -> ActorContext {
		self.inner
	}
}

impl Schedule<'_> {
	pub fn after(&self, duration: std::time::Duration, action_name: &str, args: &[u8]) {
		self.inner.after(duration, action_name, args);
	}

	pub fn at(&self, timestamp_ms: i64, action_name: &str, args: &[u8]) {
		self.inner.at(timestamp_ms, action_name, args);
	}
}

pub struct ConnIter<'a, A: Actor> {
	inner: ConnHandles<'a>,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> ConnIter<'_, A> {
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

impl<A: Actor> Iterator for ConnIter<'_, A> {
	type Item = ConnCtx<A>;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(ConnCtx::new)
	}
}

#[derive(Debug)]
pub struct ConnCtx<A: Actor> {
	inner: ConnHandle,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Clone for ConnCtx<A> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_p: PhantomData,
		}
	}
}

impl<A: Actor> ConnCtx<A> {
	pub(crate) fn new(inner: ConnHandle) -> Self {
		Self {
			inner,
			_p: PhantomData,
		}
	}

	pub fn id(&self) -> &str {
		self.inner.id()
	}

	pub fn is_hibernatable(&self) -> bool {
		self.inner.is_hibernatable()
	}

	pub fn params(&self) -> Result<A::ConnParams> {
		decode_cbor(&self.inner.params(), "connection params")
	}

	pub fn state(&self) -> Result<A::ConnState> {
		decode_cbor(&self.inner.state(), "connection state")
	}

	pub fn set_state(&self, state: &A::ConnState) -> Result<()> {
		self.inner
			.set_state(encode_cbor(state, "connection state")?);
		Ok(())
	}

	pub fn send<E: Serialize>(&self, name: &str, event: &E) -> Result<()> {
		let event_bytes = encode_cbor(event, "connection event")?;
		self.inner.send(name, &event_bytes);
		Ok(())
	}

	pub async fn disconnect(&self, reason: Option<&str>) -> Result<()> {
		self.inner.disconnect(reason).await
	}

	pub fn inner(&self) -> &ConnHandle {
		&self.inner
	}

	pub fn into_inner(self) -> ConnHandle {
		self.inner
	}
}

impl<A: Actor> From<ConnHandle> for ConnCtx<A> {
	fn from(value: ConnHandle) -> Self {
		Self::new(value)
	}
}

fn encode_cbor<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded)
		.with_context(|| format!("encode {label} as cbor"))?;
	Ok(encoded)
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
	ciborium::from_reader(Cursor::new(bytes)).with_context(|| format!("decode {label} from cbor"))
}

#[cfg(test)]
#[path = "../tests/modules/context.rs"]
mod tests;
