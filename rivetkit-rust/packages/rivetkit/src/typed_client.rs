use std::io::Cursor;
use std::marker::PhantomData;

use anyhow::{Context, Result, bail};
use ciborium::Value as CborValue;
use rivetkit_client::{
	Client, GetOptions, GetOrCreateOptions,
	connection::{ActorConnection, Event as ClientEvent, SubscriptionHandle},
	handle::ActorHandle,
};
use serde_json::Value as JsonValue;

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

	values.into_iter().map(crate::action::cbor_to_json).collect()
}

fn decode_event<E: Event>(event: &ClientEvent) -> Result<E> {
	decode_event_args(&event.raw_args).with_context(|| format!("decode typed event '{}'", E::NAME))
}

fn decode_event_args<E: Event>(raw_args: &[u8]) -> Result<E> {
	let value: CborValue =
		ciborium::from_reader(Cursor::new(raw_args)).context("decode typed event args as cbor")?;
	match value {
		CborValue::Array(values) if values.is_empty() => {
			crate::event::deserialize_cbor_value(CborValue::Null)
				.map_err(|error| anyhow::anyhow!(error.to_string()))
				.context("decode typed event from empty args")
		}
		CborValue::Array(mut values) if values.len() == 1 => {
			let value = values.remove(0);
			crate::event::deserialize_cbor_value(value)
				.map_err(|error| anyhow::anyhow!(error.to_string()))
				.context("decode typed event from single arg")
		}
		CborValue::Array(values) => crate::event::deserialize_cbor_value(CborValue::Array(values))
			.map_err(|error| anyhow::anyhow!(error.to_string()))
			.context("decode typed event from positional args"),
		value => crate::event::deserialize_cbor_value(value)
			.map_err(|error| anyhow::anyhow!(error.to_string()))
			.context("decode typed event from legacy payload"),
	}
}


#[cfg(test)]
mod tests {
	use std::io::Cursor;

	use serde::{Deserialize, Serialize};
	use serde_json::Value as JsonValue;

	use super::encode_action_args;
	use crate::action::{self, Action};
	use crate::test_fixtures::HrSensitive;

	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct Ping {
		id: HrSensitive,
	}

	impl Action for Ping {
		type Output = ();

		const NAME: &'static str = "ping";
	}

	#[test]
	fn encode_action_args_emits_string_not_byte_array() {
		let args = encode_action_args(&Ping {
			id: HrSensitive([0xde, 0xad, 0xbe, 0xef]),
		})
		.expect("encode action args");

		assert_eq!(args.len(), 1);
		let object = args[0].as_object().expect("named struct arg is a json object");
		// The uuid-like field must be a json string, not a number array.
		assert_eq!(
			object.get("id").and_then(JsonValue::as_str),
			Some("deadbeef"),
			"uuid-like field must be a string, got {:?}",
			object.get("id"),
		);
	}

	#[test]
	fn encode_action_args_round_trips_through_actor_decode() {
		let action = Ping {
			id: HrSensitive([0x01, 0x02, 0x03, 0x04]),
		};
		let args = encode_action_args(&action).expect("encode action args");

		// Mimic the engine re-encoding the JSON args into the CBOR buffer the actor receives.
		let mut cbor = Vec::new();
		ciborium::into_writer(&JsonValue::Array(args), &mut cbor).expect("encode json args as cbor");

		let decoded = action::decode_positional::<Ping>(&cbor).expect("actor decodes args");
		assert_eq!(decoded, action);

		// The wire form carries the string form.
		let value: ciborium::Value =
			ciborium::from_reader(Cursor::new(&cbor)).expect("decode cbor value");
		let ciborium::Value::Array(values) = value else {
			panic!("positional args should be an array");
		};
		let ciborium::Value::Map(fields) = &values[0] else {
			panic!("named struct arg should remain a map");
		};
		assert!(matches!(&fields[0].1, ciborium::Value::Text(text) if text == "01020304"));
	}
}
