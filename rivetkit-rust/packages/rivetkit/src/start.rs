use std::io::Cursor;
use std::marker::PhantomData;

use anyhow::{Context, Result};
use rivetkit_core::error::ActorRuntime;
use rivetkit_core::{ActorEvent, ActorEvents, ActorStart};
use serde::de::DeserializeOwned;

use crate::{
	actor::Actor,
	context::{ConnCtx, Ctx},
	event::Event,
};

#[derive(Debug)]
pub struct Start<A: Actor> {
	pub ctx: Ctx<A>,
	pub input: Input<A>,
	pub snapshot: Snapshot,
	pub hibernated: Vec<Hibernated<A>>,
	pub events: Events<A>,
}

#[derive(Debug)]
pub struct Input<A: Actor> {
	bytes: Option<Vec<u8>>,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Input<A> {
	pub fn is_present(&self) -> bool {
		self.bytes.is_some()
	}

	pub fn decode(&self) -> Result<A::Input> {
		let bytes = self
			.bytes
			.as_deref()
			.ok_or_else(|| ActorRuntime::MissingInput.build())?;
		decode_cbor(bytes, "actor input")
	}

	pub fn decode_or<F>(&self, f: F) -> Result<A::Input>
	where
		F: FnOnce() -> A::Input,
	{
		match self.bytes.as_deref() {
			Some(bytes) => decode_cbor(bytes, "actor input"),
			None => Ok(f()),
		}
	}

	pub fn decode_or_default(&self) -> Result<A::Input>
	where
		A::Input: Default,
	{
		self.decode_or(A::Input::default)
	}

	pub fn raw(&self) -> Option<&[u8]> {
		self.bytes.as_deref()
	}
}

#[derive(Debug)]
pub struct Snapshot {
	bytes: Option<Vec<u8>>,
}

impl Snapshot {
	pub fn is_new(&self) -> bool {
		self.bytes.is_none()
	}

	pub fn decode<S>(&self) -> Result<Option<S>>
	where
		S: DeserializeOwned,
	{
		self.bytes
			.as_deref()
			.map(|bytes| decode_cbor(bytes, "actor snapshot"))
			.transpose()
	}

	pub fn decode_or_default<S>(&self) -> Result<S>
	where
		S: DeserializeOwned + Default,
	{
		Ok(self.decode()?.unwrap_or_default())
	}

	pub fn raw(&self) -> Option<&[u8]> {
		self.bytes.as_deref()
	}
}

#[derive(Debug)]
pub struct Hibernated<A: Actor> {
	pub conn: ConnCtx<A>,
}

#[derive(Debug)]
pub struct Events<A: Actor> {
	ctx: Ctx<A>,
	rx: ActorEvents,
	_p: PhantomData<fn() -> A>,
}

impl<A: Actor> Events<A> {
	pub async fn recv(&mut self) -> Option<Event<A>> {
		loop {
			let event = self.rx.recv().await?;
			if let Some(event) = self.handle_runtime_event(event).await {
				return Some(wrap_event(event));
			}
		}
	}

	pub fn try_recv(&mut self) -> Option<Event<A>> {
		while let Some(event) = self.rx.try_recv() {
			if let Some(event) = self.handle_runtime_event_sync(event) {
				return Some(wrap_event(event));
			}
		}
		None
	}

	async fn handle_runtime_event(&self, event: ActorEvent) -> Option<ActorEvent> {
		match event {
			ActorEvent::DisconnectConn { conn_id, reply } => {
				reply.send(self.ctx.disconnect_conn(&conn_id).await);
				None
			}
			event => Some(event),
		}
	}

	fn handle_runtime_event_sync(&self, event: ActorEvent) -> Option<ActorEvent> {
		match event {
			ActorEvent::DisconnectConn { conn_id, reply } => {
				let ctx = self.ctx.clone();
				tokio::spawn(async move {
					reply.send(ctx.disconnect_conn(&conn_id).await);
				});
				None
			}
			event => Some(event),
		}
	}
}

#[doc(hidden)]
pub fn wrap_start<A: Actor>(core_start: ActorStart) -> Result<Start<A>> {
	let ActorStart {
		ctx,
		input,
		snapshot,
		hibernated,
		events,
		startup_ready: _,
	} = core_start;

	let hibernated = hibernated
		.into_iter()
		.map(|(conn, bytes)| Hibernated {
			conn: ConnCtx::from({
				conn.set_state(bytes);
				conn
			}),
		})
		.collect();

	let ctx = Ctx::new(ctx);

	Ok(Start {
		ctx: ctx.clone(),
		input: Input {
			bytes: input,
			_p: PhantomData,
		},
		snapshot: Snapshot { bytes: snapshot },
		hibernated,
		events: Events {
			ctx,
			rx: events,
			_p: PhantomData,
		},
	})
}

fn wrap_event<A: Actor>(event: ActorEvent) -> Event<A> {
	Event::from_core(event)
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
	ciborium::from_reader(Cursor::new(bytes)).with_context(|| format!("decode {label} from cbor"))
}

#[cfg(test)]
mod tests {
	use serde::Serialize;
	use tokio::sync::mpsc::unbounded_channel;

	use super::*;
	use crate::action;

	struct EmptyActor;

	impl Actor for EmptyActor {
		type Input = ();
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	struct UnitActor;

	impl Actor for UnitActor {
		type Input = UnitInput;
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	#[derive(Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
	struct UnitInput;

	#[derive(Debug, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
	struct ExampleState {
		count: u32,
		label: String,
	}

	#[test]
	fn input_decode_round_trips_unit() {
		let bytes = cbor(&());
		let input = Input::<EmptyActor> {
			bytes: Some(bytes.clone()),
			_p: PhantomData,
		};

		assert!(input.is_present());
		assert_eq!(input.raw(), Some(bytes.as_slice()));
		assert_eq!(input.decode().expect("decode unit input"), ());
	}

	#[test]
	fn input_decode_round_trips_unit_struct() {
		let input = Input::<UnitActor> {
			bytes: Some(cbor(&UnitInput)),
			_p: PhantomData,
		};

		assert_eq!(input.decode().expect("decode unit struct input"), UnitInput);
	}

	#[test]
	fn input_decode_or_default_uses_default_when_missing() {
		let input = Input::<DefaultActor> {
			bytes: None,
			_p: PhantomData,
		};

		assert_eq!(
			input.decode_or_default().expect("default input"),
			DefaultInput { count: 7 }
		);
	}

	#[test]
	fn snapshot_decode_round_trips_map_struct() {
		let snapshot = Snapshot {
			bytes: Some(cbor(&ExampleState {
				count: 9,
				label: "hi".into(),
			})),
		};

		assert!(!snapshot.is_new());
		assert_eq!(
			snapshot.decode::<ExampleState>().expect("decode snapshot"),
			Some(ExampleState {
				count: 9,
				label: "hi".into(),
			})
		);
	}

	#[test]
	fn snapshot_decode_or_default_uses_default_when_missing() {
		let snapshot = Snapshot { bytes: None };

		assert!(snapshot.is_new());
		assert_eq!(
			snapshot
				.decode_or_default::<ExampleState>()
				.expect("default snapshot"),
			ExampleState::default()
		);
	}

	#[test]
	fn wrap_start_rehydrates_hibernated_connection_state() {
		let (tx, rx) = unbounded_channel();
		drop(tx);
		let start = wrap_start::<ConnActor>(ActorStart {
			ctx: rivetkit_core::ActorContext::new("actor-id", "test", Vec::new(), "local"),
			input: None,
			snapshot: None,
			hibernated: vec![(
				rivetkit_core::ConnHandle::new(
					"conn-id",
					cbor(&()),
					cbor(&ConnState { value: 1 }),
					true,
				),
				cbor(&ConnState { value: 5 }),
			)],
			events: rx.into(),
			startup_ready: None,
		})
		.expect("wrap start");

		assert_eq!(
			start.hibernated[0]
				.conn
				.state()
				.expect("decode hibernated conn state"),
			ConnState { value: 5 }
		);
	}

	#[test]
	fn events_try_recv_wraps_core_events() {
		let (tx, rx) = unbounded_channel();
		tx.send(ActorEvent::ConnectionClosed {
			conn: rivetkit_core::ConnHandle::new("conn-id", cbor(&()), cbor(&()), true),
		})
		.expect("queue event");

		let mut events = Events::<EmptyActor> {
			ctx: Ctx::new(rivetkit_core::ActorContext::new(
				"actor-id",
				"test",
				Vec::new(),
				"local",
			)),
			rx: rx.into(),
			_p: PhantomData,
		};

		let Some(Event::ConnClosed(closed)) = events.try_recv() else {
			panic!("expected typed connection-closed event");
		};

		assert_eq!(closed.conn.id(), "conn-id");
	}

	struct DefaultActor;

	impl Actor for DefaultActor {
		type Input = DefaultInput;
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	#[derive(Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
	struct DefaultInput {
		count: u32,
	}

	impl Default for DefaultInput {
		fn default() -> Self {
			Self { count: 7 }
		}
	}

	struct ConnActor;

	impl Actor for ConnActor {
		type Input = ();
		type ConnParams = ();
		type ConnState = ConnState;
		type Action = action::Raw;
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
	struct ConnState {
		value: u32,
	}

	fn cbor<T: Serialize>(value: &T) -> Vec<u8> {
		let mut encoded = Vec::new();
		ciborium::into_writer(value, &mut encoded).expect("encode test value as cbor");
		encoded
	}
}
