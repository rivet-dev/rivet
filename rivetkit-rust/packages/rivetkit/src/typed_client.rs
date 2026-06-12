use std::io::Cursor;
use std::marker::PhantomData;

use anyhow::{Context, Result, bail};
use ciborium::Value as CborValue;
use rivetkit_client::{
	Client, GetOptions, GetOrCreateOptions,
	connection::{ActorConnection, Event as ClientEvent, SubscriptionHandle},
	handle::ActorHandle,
};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};

use crate::action::{Action, Handles, encode_positional};
use crate::actor::Actor;
use crate::event::Event;

pub trait TypedClientExt {
	fn get_typed<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
		opts: GetOptions,
	) -> Result<TypedActorHandle<A>>;

	fn get_typed_default<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
	) -> Result<TypedActorHandle<A>> {
		self.get_typed(name, key, GetOptions::default())
	}

	fn get_or_create_typed<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
		opts: GetOrCreateOptions,
	) -> Result<TypedActorHandle<A>>;

	fn get_or_create_typed_default<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
	) -> Result<TypedActorHandle<A>> {
		self.get_or_create_typed(name, key, GetOrCreateOptions::default())
	}
}

impl TypedClientExt for Client {
	fn get_typed<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
		opts: GetOptions,
	) -> Result<TypedActorHandle<A>> {
		Ok(TypedActorHandle::new(self.get(
			name,
			key.into_actor_key(),
			opts,
		)?))
	}

	fn get_or_create_typed<A: Actor>(
		&self,
		name: &str,
		key: impl IntoActorKey,
		opts: GetOrCreateOptions,
	) -> Result<TypedActorHandle<A>> {
		Ok(TypedActorHandle::new(self.get_or_create(
			name,
			key.into_actor_key(),
			opts,
		)?))
	}
}

pub trait IntoActorKey {
	fn into_actor_key(self) -> Vec<String>;
}

impl IntoActorKey for Vec<String> {
	fn into_actor_key(self) -> Vec<String> {
		self
	}
}

impl IntoActorKey for Vec<&str> {
	fn into_actor_key(self) -> Vec<String> {
		self.into_iter().map(ToOwned::to_owned).collect()
	}
}

impl<const N: usize> IntoActorKey for [&str; N] {
	fn into_actor_key(self) -> Vec<String> {
		self.into_iter().map(ToOwned::to_owned).collect()
	}
}

impl<const N: usize> IntoActorKey for [String; N] {
	fn into_actor_key(self) -> Vec<String> {
		self.into_iter().collect()
	}
}

impl IntoActorKey for &[&str] {
	fn into_actor_key(self) -> Vec<String> {
		self.iter().map(|value| (*value).to_owned()).collect()
	}
}

impl IntoActorKey for &[String] {
	fn into_actor_key(self) -> Vec<String> {
		self.to_vec()
	}
}

pub struct TypedActorHandle<A: Actor> {
	inner: ActorHandle,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> TypedActorHandle<A> {
	pub fn new(inner: ActorHandle) -> Self {
		Self {
			inner,
			_p: PhantomData,
		}
	}

	pub fn inner(&self) -> &ActorHandle {
		&self.inner
	}

	pub fn into_inner(self) -> ActorHandle {
		self.inner
	}

	pub fn connect(&self) -> TypedActorConnection<A> {
		TypedActorConnection::new(self.inner.connect())
	}

	pub async fn send<M>(&self, action: M) -> Result<M::Output>
	where
		A: Handles<M>,
		M: Action,
	{
		self.call(action).await
	}

	pub async fn call<M: Action>(&self, action: M) -> Result<M::Output> {
		let output = self
			.inner
			.action(M::NAME, encode_action_args(&action)?)
			.await?;
		serde_json::from_value(output).context("decode typed action output")
	}
}

pub struct TypedActorConnection<A: Actor> {
	inner: ActorConnection,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> TypedActorConnection<A> {
	pub fn new(inner: ActorConnection) -> Self {
		Self {
			inner,
			_p: PhantomData,
		}
	}

	pub fn inner(&self) -> &ActorConnection {
		&self.inner
	}

	pub fn into_inner(self) -> ActorConnection {
		self.inner
	}

	pub async fn on<E>(&self, callback: impl Fn(E) + Send + Sync + 'static) -> SubscriptionHandle
	where
		E: Event,
	{
		self.inner
			.on_event_raw(E::NAME, move |event| match decode_event::<E>(&event) {
				Ok(event) => callback(event),
				Err(error) => {
					tracing::debug!(?error, event_name = E::NAME, "failed to decode typed event")
				}
			})
			.await
	}

	pub async fn send<M>(&self, action: M) -> Result<M::Output>
	where
		A: Handles<M>,
		M: Action,
	{
		self.call(action).await
	}

	pub async fn call<M: Action>(&self, action: M) -> Result<M::Output> {
		let output = self
			.inner
			.action(M::NAME, encode_action_args(&action)?)
			.await?;
		serde_json::from_value(output).context("decode typed connection action output")
	}

	pub async fn disconnect(&self) {
		self.inner.disconnect().await;
	}
}

pub(crate) fn encode_action_args<M: Action>(action: &M) -> Result<Vec<JsonValue>> {
	let encoded = encode_positional(action)?;
	let value: CborValue =
		ciborium::from_reader(Cursor::new(encoded)).context("decode positional action args")?;
	let CborValue::Array(values) = value else {
		bail!("positional action args must encode as a cbor array");
	};

	values.into_iter().map(cbor_to_json).collect()
}

fn decode_event<E: Event>(event: &ClientEvent) -> Result<E> {
	ciborium::from_reader(Cursor::new(&event.raw_args))
		.with_context(|| format!("decode typed event '{}'", E::NAME))
}

fn cbor_to_json(value: CborValue) -> Result<JsonValue> {
	Ok(match value {
		CborValue::Null => JsonValue::Null,
		CborValue::Bool(value) => JsonValue::Bool(value),
		CborValue::Integer(value) => integer_to_json(i128::from(value))?,
		CborValue::Float(value) => JsonValue::Number(
			JsonNumber::from_f64(value).context("cbor float cannot be represented as json")?,
		),
		CborValue::Bytes(value) => {
			JsonValue::Array(value.into_iter().map(JsonValue::from).collect())
		}
		CborValue::Text(value) => JsonValue::String(value),
		CborValue::Array(values) => JsonValue::Array(
			values
				.into_iter()
				.map(cbor_to_json)
				.collect::<Result<Vec<_>>>()?,
		),
		CborValue::Map(entries) => {
			let mut object = JsonMap::new();
			for (key, value) in entries {
				let CborValue::Text(key) = key else {
					bail!("cbor map key cannot be represented as a json object key");
				};
				object.insert(key, cbor_to_json(value)?);
			}
			JsonValue::Object(object)
		}
		CborValue::Tag(_, value) => cbor_to_json(*value)?,
		_ => bail!("cbor value cannot be represented as json"),
	})
}

fn integer_to_json(value: i128) -> Result<JsonValue> {
	if let Ok(value) = i64::try_from(value) {
		return Ok(JsonValue::Number(JsonNumber::from(value)));
	}
	if let Ok(value) = u64::try_from(value) {
		return Ok(JsonValue::Number(JsonNumber::from(value)));
	}
	bail!("cbor integer cannot be represented as json number")
}
