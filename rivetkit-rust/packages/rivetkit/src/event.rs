use std::{fmt, io::Cursor, marker::PhantomData};

use anyhow::{Context, Result as AnyhowResult};
use ciborium::Value;
use rivetkit_core::actor::ShutdownKind;
use rivetkit_core::actor::schedule::ScheduledFireInfo;
use rivetkit_core::error::ActorRuntime;
use rivetkit_core::{
	ActorEvent, ActorHttpResponse, QueueSendResult, QueueSendStatus, Reply, Request, Response,
	SerializeStateReason, StateDelta, WebSocket,
};
use serde::{
	Serialize,
	de::{
		self, DeserializeOwned, DeserializeSeed, EnumAccess, VariantAccess, Visitor,
		value::BorrowedStrDeserializer,
	},
};

use crate::{action, actor::Actor, context::ConnCtx, persist};

/// An event an actor can broadcast to connected clients. Payloads must be JSON-compatible,
/// like [`crate::action::Action`] arguments.
pub trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {
	const NAME: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEntry {
	pub name: &'static str,
}

pub trait EventSet: Send + Sync + 'static {
	fn entries() -> Vec<EventEntry>;
}

impl EventSet for () {
	fn entries() -> Vec<EventEntry> {
		Vec::new()
	}
}

macro_rules! impl_event_set {
	($($event:ident),+) => {
		impl<$($event),+> EventSet for ($($event,)+)
		where
			$($event: Event,)+
		{
			fn entries() -> Vec<EventEntry> {
				vec![$(EventEntry { name: <$event as Event>::NAME }),+]
			}
		}
	};
}

impl_event_set!(E0);
impl_event_set!(E0, E1);
impl_event_set!(E0, E1, E2);
impl_event_set!(E0, E1, E2, E3);
impl_event_set!(E0, E1, E2, E3, E4);
impl_event_set!(E0, E1, E2, E3, E4, E5);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8, E9);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12);
impl_event_set!(E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13);
impl_event_set!(
	E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14
);
impl_event_set!(
	E0, E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15
);

#[derive(Debug)]
#[must_use = "dropping a RuntimeEvent<A> without replying sends actor/dropped_reply"]
pub enum RuntimeEvent<A: Actor> {
	Action(ActionCall<A>),
	Http(HttpCall),
	QueueSend(QueueSend<A>),
	WebSocketOpen(WsOpen<A>),
	ConnOpen(ConnOpen<A>),
	ConnClosed(ConnClosed<A>),
	Subscribe(Subscribe<A>),
	SerializeState(SerializeState<A>),
	Sleep(Sleep<A>),
	Destroy(Destroy<A>),
}

impl<A: Actor> RuntimeEvent<A> {
	pub(crate) fn from_core(event: ActorEvent) -> Self {
		match event {
			ActorEvent::Action {
				name,
				args,
				conn,
				scheduled_fire,
				reply,
			} => Self::Action(ActionCall {
				name,
				args,
				conn: conn.map(ConnCtx::from),
				scheduled_fire,
				reply: Some(reply),
			}),
			ActorEvent::HttpRequest { request, reply } => Self::Http(HttpCall {
				request: Some(request),
				reply: Some(reply),
			}),
			ActorEvent::QueueSend {
				name,
				body,
				conn,
				request,
				wait,
				timeout_ms,
				reply,
			} => Self::QueueSend(QueueSend {
				name,
				body,
				conn: ConnCtx::from(conn),
				request,
				wait,
				timeout_ms,
				reply: Some(reply),
			}),
			ActorEvent::WebSocketOpen {
				conn: _conn,
				ws,
				request,
				reply,
			} => Self::WebSocketOpen(WsOpen {
				ws,
				request,
				reply: Some(reply),
				_p: PhantomData,
			}),
			ActorEvent::ConnectionPreflight {
				conn,
				params,
				request,
				reply,
			} => Self::ConnOpen(ConnOpen {
				conn: ConnCtx::from(conn),
				params,
				request,
				reply: Some(reply),
			}),
			ActorEvent::ConnectionOpen { .. } => {
				unreachable!("ConnectionOpen is handled by Events")
			}
			ActorEvent::ConnectionClosed { conn } => Self::ConnClosed(ConnClosed {
				conn: ConnCtx::from(conn),
			}),
			ActorEvent::SubscribeRequest {
				conn,
				event_name,
				reply,
			} => Self::Subscribe(Subscribe {
				conn: ConnCtx::from(conn),
				event_name,
				reply: Some(reply),
			}),
			ActorEvent::SerializeState { reason, reply } => Self::SerializeState(SerializeState {
				reason,
				reply: Some(reply),
				_p: PhantomData,
			}),
			ActorEvent::RunGracefulCleanup { reason, reply } => match reason {
				ShutdownKind::Sleep => Self::Sleep(Sleep {
					reply: Some(reply),
					_p: PhantomData,
				}),
				ShutdownKind::Destroy => Self::Destroy(Destroy {
					reply: Some(reply),
					_p: PhantomData,
				}),
			},
			ActorEvent::DisconnectConn { .. } => {
				unreachable!("DisconnectConn is handled by foreign-runtime adapters")
			}
			ActorEvent::WorkflowHistoryRequested { .. }
			| ActorEvent::WorkflowReplayRequested { .. } => {
				unreachable!(
					"workflow events are handled by the TypeScript runtime; Rust actors never host workflows"
				)
			}
			ActorEvent::RunWake { .. } => {
				unreachable!("RunWake is acknowledged by the Rust event adapter")
			}
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the action or dropping it sends actor/dropped_reply"]
#[allow(dead_code)]
pub struct ActionCall<A: Actor> {
	pub(crate) name: String,
	pub(crate) args: Vec<u8>,
	pub(crate) conn: Option<ConnCtx<A>>,
	pub(crate) scheduled_fire: Option<ScheduledFireInfo>,
	pub(crate) reply: Option<Reply<Vec<u8>>>,
}

impl<A: Actor> Drop for ActionCall<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("Action", self.name.as_str());
		}
	}
}

impl<A: Actor> ActionCall<A> {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn conn(&self) -> Option<&ConnCtx<A>> {
		self.conn.as_ref()
	}

	pub fn scheduled_fire(&self) -> Option<&ScheduledFireInfo> {
		self.scheduled_fire.as_ref()
	}

	pub fn raw_args(&self) -> &[u8] {
		&self.args
	}

	/// Decode this call into the actor's action enum, keyed by action name. Part of the legacy
	/// event-loop API; new code should prefer the trait API. Decodes current-format payloads
	/// and falls back to the legacy binary form, matching [`Self::decode_as`].
	pub fn decode(&self) -> AnyhowResult<A::Action> {
		let name = self.name.as_str();
		let args = self.raw_args();

		<A::Action as serde::Deserialize>::deserialize(ActionDeserializer::new(name, args))
			.or_else(|current_error| {
				decode_legacy_action_enum::<A::Action>(name, args).map_err(|_| current_error)
			})
			.map_err(|error| {
				ActorRuntime::InvalidOperation {
					operation: format!("decode action '{}'", self.name),
					reason: error.to_string(),
				}
				.build()
			})
	}

	pub fn decode_as<T: DeserializeOwned>(&self) -> AnyhowResult<T> {
		action::decode_positional(self.raw_args()).with_context(|| {
			format!(
				"decode action '{}' args as {}",
				self.name,
				std::any::type_name::<T>()
			)
		})
	}

	pub fn ok<T: Serialize>(mut self, value: &T) {
		let result = encode_cbor(value, "encode action response as cbor");
		if let Some(reply) = self.reply.take() {
			reply.send(result);
		}
	}

	pub fn err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

struct ActionDeserializer<'a> {
	name: &'a str,
	args: &'a [u8],
}

impl<'a> ActionDeserializer<'a> {
	fn new(name: &'a str, args: &'a [u8]) -> Self {
		Self { name, args }
	}
}

impl<'de> de::Deserializer<'de> for ActionDeserializer<'de> {
	type Error = de::value::Error;

	fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		Err(de::Error::custom(
			"action payload must deserialize via an enum",
		))
	}

	fn deserialize_enum<V>(
		self,
		_name: &'static str,
		_variants: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_enum(ActionEnumAccess {
			name: self.name,
			args: self.args,
		})
	}

	serde::forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
		bytes byte_buf option unit unit_struct newtype_struct seq tuple
		tuple_struct map struct identifier ignored_any
	}
}

struct ActionEnumAccess<'a> {
	name: &'a str,
	args: &'a [u8],
}

impl<'de> EnumAccess<'de> for ActionEnumAccess<'de> {
	type Error = de::value::Error;
	type Variant = ActionVariantAccess<'de>;

	fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: DeserializeSeed<'de>,
	{
		let name = self.name;
		let variant = seed
			.deserialize(BorrowedStrDeserializer::<de::value::Error>::new(name))
			.map_err(|_| de::Error::custom(format!("unknown action variant: {name}")))?;
		Ok((variant, ActionVariantAccess { args: self.args }))
	}
}

struct ActionVariantAccess<'a> {
	args: &'a [u8],
}

impl<'de> VariantAccess<'de> for ActionVariantAccess<'de> {
	type Error = de::value::Error;

	fn unit_variant(self) -> Result<(), Self::Error> {
		match self.args {
			// Empty bytes, cbor null, or an empty cbor array (`[]`) all map to a
			// no-argument action. Clients encode the empty positional argument
			// list as an empty cbor array.
			[] | [0xf6] | [0x80] => Ok(()),
			_ => Err(de::Error::custom(
				"unit action variant expects empty args, cbor null, or an empty cbor array",
			)),
		}
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: DeserializeSeed<'de>,
	{
		seed.deserialize(action_payload_value(self.args)?)
			.map_err(map_json_error)
	}

	fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		de::Deserializer::deserialize_tuple(action_payload_value(self.args)?, len, visitor)
			.map_err(map_json_error)
	}

	fn struct_variant<V>(
		self,
		fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		de::Deserializer::deserialize_struct(action_payload_value(self.args)?, "action", fields, visitor)
			.map_err(map_json_error)
	}
}

/// Decode an action variant's payload into a JSON-shaped value, symmetric with the encoder.
fn action_payload_value(args: &[u8]) -> Result<serde_json::Value, de::value::Error> {
	let value = decode_action_value(args)?;
	action::cbor_to_json(value).map_err(|error| de::Error::custom(error.to_string()))
}

fn map_json_error(error: serde_json::Error) -> de::value::Error {
	de::Error::custom(error)
}

/// Legacy fallback for [`ActionCall::decode`]: reconstruct the externally tagged enum (`name`
/// for a unit variant, `{name: payload}` otherwise) as a CBOR value and decode it natively, so
/// payloads persisted by an older runtime still decode.
fn decode_legacy_action_enum<T: DeserializeOwned>(
	name: &str,
	args: &[u8],
) -> Result<T, de::value::Error> {
	let enum_value = if args.is_empty() {
		Value::Text(name.to_owned())
	} else {
		Value::Map(vec![(Value::Text(name.to_owned()), decode_action_value(args)?)])
	};

	let mut encoded = Vec::new();
	ciborium::into_writer(&enum_value, &mut encoded)
		.map_err(|error| de::Error::custom(format!("re-encode legacy action enum: {error}")))?;
	ciborium::from_reader(Cursor::new(encoded))
		.map_err(|error| de::Error::custom(format!("decode legacy action enum: {error}")))
}

fn decode_action_value(args: &[u8]) -> Result<Value, de::value::Error> {
	ciborium::from_reader(Cursor::new(args))
		.map_err(|error| de::Error::custom(format!("decode action args from cbor: {error}")))
}

pub(crate) fn deserialize_cbor_value<T: DeserializeOwned>(
	value: Value,
) -> Result<T, de::value::Error> {
	// Current format: decode through the JSON value model, symmetric with the encoder. Fall
	// back to the legacy native-binary codec for payloads persisted by an older runtime,
	// preferring the current-format error since it describes new payloads.
	match deserialize_json_value(value.clone()) {
		Ok(decoded) => Ok(decoded),
		Err(current_error) => deserialize_binary_value(&value).map_err(|_| current_error),
	}
}

fn deserialize_json_value<T: DeserializeOwned>(value: Value) -> Result<T, de::value::Error> {
	let json = action::cbor_to_json(value)
		.map_err(|error| de::Error::custom(format!("convert cbor envelope to json: {error}")))?;
	serde_json::from_value(json)
		.map_err(|error| de::Error::custom(format!("decode json value: {error}")))
}

fn deserialize_binary_value<T: DeserializeOwned>(value: &Value) -> Result<T, de::value::Error> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded)
		.map_err(|error| de::Error::custom(format!("re-encode value for binary decode: {error}")))?;
	ciborium::from_reader(Cursor::new(encoded))
		.map_err(|error| de::Error::custom(format!("decode value in binary mode: {error}")))
}

fn encode_cbor<T: Serialize>(value: &T, context: &'static str) -> AnyhowResult<Vec<u8>> {
	let value = action::to_json_value(value, context)?;
	action::encode_json_as_cbor(&value, context)
}

#[derive(Debug)]
#[must_use = "reply to the HTTP call or dropping it sends actor/dropped_reply"]
pub struct HttpCall {
	pub(crate) request: Option<Request>,
	pub(crate) reply: Option<Reply<ActorHttpResponse>>,
}

impl Drop for HttpCall {
	fn drop(&mut self) {
		if self.reply.is_some() {
			let identifying = self
				.request
				.as_ref()
				.map(|request| request.uri().to_string())
				.unwrap_or_else(|| "<moved-request>".into());
			warn_dropped_event("Http", identifying);
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the deferred HTTP call or dropping it sends actor/dropped_reply"]
pub struct HttpReply {
	reply: Option<Reply<ActorHttpResponse>>,
}

impl Drop for HttpReply {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("Http", "<deferred>");
		}
	}
}

impl HttpReply {
	pub fn reply(mut self, response: Response) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(ActorHttpResponse::Buffered(response)));
		}
	}

	pub fn reply_status(self, status: u16) {
		match Response::from_parts(status, Default::default(), Vec::new()) {
			Ok(response) => self.reply(response),
			Err(error) => self.reply_err(error),
		}
	}

	pub fn reply_err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

impl HttpCall {
	pub fn request(&self) -> Option<&Request> {
		self.request.as_ref()
	}

	pub fn request_mut(&mut self) -> Option<&mut Request> {
		self.request.as_mut()
	}

	pub fn into_request(mut self) -> AnyhowResult<(Request, HttpReply)> {
		let request = self.request.take().ok_or_else(|| {
			ActorRuntime::InvalidOperation {
				operation: "http.into_request".to_owned(),
				reason: "request was already moved".to_owned(),
			}
			.build()
		})?;
		Ok((
			request,
			HttpReply {
				reply: self.reply.take(),
			},
		))
	}

	pub fn reply(mut self, response: Response) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(ActorHttpResponse::Buffered(response)));
		}
	}

	pub fn reply_status(self, status: u16) {
		match Response::from_parts(status, Default::default(), Vec::new()) {
			Ok(response) => self.reply(response),
			Err(error) => self.reply_err(error),
		}
	}

	pub fn reply_err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the queue send or dropping it sends actor/dropped_reply"]
#[allow(dead_code)]
pub struct QueueSend<A: Actor> {
	pub(crate) name: String,
	pub(crate) body: Vec<u8>,
	pub(crate) conn: ConnCtx<A>,
	pub(crate) request: Request,
	pub(crate) wait: bool,
	pub(crate) timeout_ms: Option<u64>,
	pub(crate) reply: Option<Reply<QueueSendResult>>,
}

impl<A: Actor> Drop for QueueSend<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("QueueSend", self.name.as_str());
		}
	}
}

impl<A: Actor> QueueSend<A> {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn body(&self) -> &[u8] {
		&self.body
	}

	pub fn conn(&self) -> &ConnCtx<A> {
		&self.conn
	}

	pub fn request(&self) -> &Request {
		&self.request
	}

	pub fn should_wait(&self) -> bool {
		self.wait
	}

	pub fn timeout_ms(&self) -> Option<u64> {
		self.timeout_ms
	}

	pub fn complete(mut self, response: Option<Vec<u8>>) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(QueueSendResult {
				status: QueueSendStatus::Completed,
				response,
			}));
		}
	}

	pub fn timed_out(mut self) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(QueueSendResult {
				status: QueueSendStatus::TimedOut,
				response: None,
			}));
		}
	}

	pub fn err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the websocket open or dropping it sends actor/dropped_reply"]
#[allow(dead_code)]
pub struct WsOpen<A: Actor> {
	pub(crate) ws: WebSocket,
	pub(crate) request: Option<Request>,
	pub(crate) reply: Option<Reply<()>>,
	pub(crate) _p: PhantomData<fn() -> A>,
}

impl<A: Actor> Drop for WsOpen<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			let identifying = self
				.request
				.as_ref()
				.map(|request| request.uri().to_string())
				.unwrap_or_else(|| "<no-request>".into());
			warn_dropped_event("WebSocketOpen", identifying);
		}
	}
}

impl<A: Actor> WsOpen<A> {
	pub fn websocket(&self) -> &WebSocket {
		&self.ws
	}

	pub fn request(&self) -> Option<&Request> {
		self.request.as_ref()
	}

	pub fn accept(mut self) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(()));
		}
	}

	pub fn reject(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the connection open or dropping it sends actor/dropped_reply"]
#[allow(dead_code)]
pub struct ConnOpen<A: Actor> {
	pub(crate) conn: ConnCtx<A>,
	pub(crate) params: Vec<u8>,
	pub(crate) request: Option<Request>,
	pub(crate) reply: Option<Reply<()>>,
}

impl<A: Actor> Drop for ConnOpen<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("ConnOpen", self.conn.id());
		}
	}
}

impl<A: Actor> ConnOpen<A> {
	pub fn params(&self) -> AnyhowResult<A::ConnParams> {
		ciborium::from_reader(Cursor::new(self.params.as_slice()))
			.with_context(|| "decode connection params from cbor".to_string())
	}

	pub fn request(&self) -> Option<&Request> {
		self.request.as_ref()
	}

	pub fn conn(&self) -> &ConnCtx<A> {
		&self.conn
	}

	pub fn accept(mut self, state: A::ConnState) {
		let result = self.conn.set_state(&state);
		if let Some(reply) = self.reply.take() {
			reply.send(result);
		}
	}

	pub fn accept_default(self)
	where
		A::ConnState: Default,
	{
		self.accept(A::ConnState::default());
	}

	pub fn reject(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "handle the connection close before dropping it"]
pub struct ConnClosed<A: Actor> {
	pub conn: ConnCtx<A>,
}

#[derive(Debug)]
#[must_use = "reply to the subscribe request or dropping it sends actor/dropped_reply"]
#[allow(dead_code)]
pub struct Subscribe<A: Actor> {
	pub(crate) conn: ConnCtx<A>,
	pub(crate) event_name: String,
	pub(crate) reply: Option<Reply<()>>,
}

impl<A: Actor> Drop for Subscribe<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("Subscribe", self.conn.id());
		}
	}
}

impl<A: Actor> Subscribe<A> {
	pub fn conn(&self) -> &ConnCtx<A> {
		&self.conn
	}

	pub fn event_name(&self) -> &str {
		&self.event_name
	}

	pub fn allow(mut self) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(()));
		}
	}

	pub fn deny(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to the serialize-state request or dropping it sends actor/dropped_reply"]
pub struct SerializeState<A: Actor> {
	pub(crate) reason: SerializeStateReason,
	pub(crate) reply: Option<Reply<Vec<StateDelta>>>,
	pub(crate) _p: PhantomData<fn() -> A>,
}

impl<A: Actor> Drop for SerializeState<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("SerializeState", format_args!("{:?}", self.reason));
		}
	}
}

impl<A: Actor> SerializeState<A> {
	pub fn reason(&self) -> SerializeStateReason {
		self.reason
	}

	pub fn save<S: Serialize>(self, state: &S) {
		self.save_with_result(persist::state_deltas(state));
	}

	pub fn save_with(mut self, deltas: Vec<StateDelta>) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(deltas));
		}
	}

	pub fn save_state_and_conns<S: Serialize>(
		self,
		state: &S,
		conn_hibernation: Vec<(rivetkit_core::ConnId, Vec<u8>)>,
		conn_hibernation_removed: Vec<rivetkit_core::ConnId>,
	) {
		let mut deltas = match persist::state_deltas(state) {
			Ok(deltas) => deltas,
			Err(error) => {
				self.save_with_result(Err(error));
				return;
			}
		};
		deltas.extend(
			conn_hibernation
				.into_iter()
				.map(|(conn, bytes)| persist::conn_hibernation_delta(conn, bytes)),
		);
		deltas.extend(
			conn_hibernation_removed
				.into_iter()
				.map(persist::conn_hibernation_removed_delta),
		);
		self.save_with(deltas);
	}

	pub fn skip(self) {
		self.save_with(Vec::new());
	}

	fn save_with_result(mut self, result: AnyhowResult<Vec<StateDelta>>) {
		if let Some(reply) = self.reply.take() {
			reply.send(result);
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to sleep or dropping it sends actor/dropped_reply"]
pub struct Sleep<A: Actor> {
	pub(crate) reply: Option<Reply<()>>,
	pub(crate) _p: PhantomData<fn() -> A>,
}

impl<A: Actor> Drop for Sleep<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("Sleep", "terminal");
		}
	}
}

impl<A: Actor> Sleep<A> {
	pub fn ok(mut self) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(()));
		}
	}

	pub fn err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

#[derive(Debug)]
#[must_use = "reply to destroy or dropping it sends actor/dropped_reply"]
pub struct Destroy<A: Actor> {
	pub(crate) reply: Option<Reply<()>>,
	pub(crate) _p: PhantomData<fn() -> A>,
}

impl<A: Actor> Drop for Destroy<A> {
	fn drop(&mut self) {
		if self.reply.is_some() {
			warn_dropped_event("Destroy", "terminal");
		}
	}
}

impl<A: Actor> Destroy<A> {
	pub fn ok(mut self) {
		if let Some(reply) = self.reply.take() {
			reply.send(Ok(()));
		}
	}

	pub fn err(mut self, err: anyhow::Error) {
		if let Some(reply) = self.reply.take() {
			reply.send(Err(err));
		}
	}
}

fn warn_dropped_event(variant: &'static str, identifying: impl fmt::Display) {
	tracing::warn!(
		variant,
		identifying = %identifying,
		"rivetkit event dropped without replying"
	);
}

#[cfg(test)]
mod tests {
	use std::{
		collections::HashMap,
		io,
		sync::{Arc, Mutex},
	};

	use rivetkit_core::ConnHandle;
	use serde::{Deserialize, Serialize};
	use tokio::sync::mpsc::unbounded_channel;
	use tokio::sync::oneshot;
	use tracing_subscriber::fmt::MakeWriter;

	use super::*;
	use crate::{action, actor::Actor, start::wrap_start};

	const EVENT_SET_TUPLE_ARITY_MAX: usize = 16;

	struct EmptyActor;

	impl Actor for EmptyActor {
		type State = ();
		type Input = ();
		type Actions = ();
		type Events = ();
		type Queue = ();
		type ConnParams = ();
		type ConnState = ();
		type Action = action::Raw;
	}

	#[derive(Debug, PartialEq, Deserialize)]
	enum TestAction {
		Ping,
		Pong,
		Rename(String),
		Pair(String, u32),
		Send { text: String, count: u32 },
		Blob(serde_bytes::ByteBuf),
		Tagged(crate::test_fixtures::HrSensitive),
	}

	struct TestActor;

	impl Actor for TestActor {
		type State = ();
		type Input = ();
		type Actions = ();
		type Events = ();
		type Queue = ();
		type ConnParams = ();
		type ConnState = ();
		type Action = TestAction;
	}

	struct ConnActor;

	impl Actor for ConnActor {
		type State = ();
		type Input = ();
		type Actions = ();
		type Events = ();
		type Queue = ();
		type ConnParams = TestConnParams;
		type ConnState = TestConnState;
		type Action = action::Raw;
	}

	#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
	struct TestConnParams {
		label: String,
	}

	#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
	struct TestConnState {
		value: i64,
	}

	#[test]
	fn dropped_action_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("Action", "ping", || {
			let runtime = build_runtime();
			let (reply_tx, reply_rx) = oneshot::channel();
			let (event_tx, event_rx) = unbounded_channel();
			event_tx
				.send(ActorEvent::Action {
					name: "ping".into(),
					args: Vec::new(),
					conn: None,
					scheduled_fire: None,
					reply: reply_tx.into(),
				})
				.expect("queue action event");
			drop(event_tx);

			let mut events = runtime.block_on(async move {
				let start = wrap_start::<EmptyActor>(rivetkit_core::ActorStart {
					ctx: rivetkit_core::testing::actor_context(
						"actor-id",
						"test",
						Vec::new(),
						"local",
					),
					input: None,
					is_new: true,
					snapshot: None,
					hibernated: Vec::new(),
					events: event_rx.into(),
					startup_ready: None,
				})
				.expect("wrap start");

				start.events
			});

			runtime.block_on(async {
				let event = events.recv().await.expect("receive typed event");
				drop(event);
			});

			reply_rx
		});
	}

	#[test]
	fn dropped_conn_open_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("ConnOpen", "conn-drop-open", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(ConnOpen::<ConnActor> {
				conn: ConnCtx::from(test_conn_handle("conn-drop-open")),
				params: encode_test_cbor(&TestConnParams {
					label: "hello".into(),
				}),
				request: None,
				reply: Some(reply_tx.into()),
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_subscribe_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("Subscribe", "conn-drop-subscribe", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(Subscribe::<ConnActor> {
				conn: ConnCtx::from(test_conn_handle("conn-drop-subscribe")),
				event_name: "chat.message".into(),
				reply: Some(reply_tx.into()),
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_serialize_state_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("SerializeState", "Save", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(SerializeState::<EmptyActor> {
				reason: SerializeStateReason::Save,
				reply: Some(reply_tx.into()),
				_p: PhantomData,
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_sleep_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("Sleep", "terminal", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(Sleep::<EmptyActor> {
				reply: Some(reply_tx.into()),
				_p: PhantomData,
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_destroy_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("Destroy", "terminal", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(Destroy::<EmptyActor> {
				reply: Some(reply_tx.into()),
				_p: PhantomData,
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_http_call_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("Http", "/drop-http", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(HttpCall {
				request: Some(test_request("/drop-http")),
				reply: Some(reply_tx.into()),
			});
			reply_rx
		});
	}

	#[test]
	fn dropped_websocket_open_logs_warning_and_sends_dropped_reply() {
		assert_dropped_reply_logs("WebSocketOpen", "/drop-websocket", || {
			let (reply_tx, reply_rx) = oneshot::channel();
			drop(WsOpen::<EmptyActor> {
				ws: WebSocket::new(),
				request: Some(test_request("/drop-websocket")),
				reply: Some(reply_tx.into()),
				_p: PhantomData,
			});
			reply_rx
		});
	}

	#[test]
	fn action_decode_supports_unit_variant_with_empty_args() {
		let action = test_action("Ping", Vec::new());

		assert_eq!(action.name(), "Ping");
		assert!(action.conn().is_none());
		assert!(action.raw_args().is_empty());
		assert_eq!(
			action.decode().expect("decode unit action"),
			TestAction::Ping
		);
	}

	#[test]
	fn action_decode_supports_unit_variant_with_null_args() {
		let action = test_action("Pong", vec![0xf6]);

		assert_eq!(
			action.decode().expect("decode null unit action"),
			TestAction::Pong
		);
	}

	#[test]
	fn action_decode_supports_newtype_variant() {
		let action = test_action("Rename", encode_test_cbor(&"alice"));

		assert_eq!(
			action.decode().expect("decode newtype action"),
			TestAction::Rename("alice".into())
		);
		assert_eq!(
			action
				.decode_as::<String>()
				.expect("decode raw args as string"),
			"alice"
		);
	}

	#[test]
	fn action_decode_supports_tuple_variant() {
		let action = test_action("Pair", encode_test_cbor(&("alice", 7u32)));

		assert_eq!(
			action.decode().expect("decode tuple action"),
			TestAction::Pair("alice".into(), 7)
		);
	}

	#[test]
	fn action_decode_supports_struct_variant() {
		let action = test_action(
			"Send",
			encode_test_cbor(&SendPayload {
				text: "hello".into(),
				count: 2,
			}),
		);

		assert_eq!(
			action.decode().expect("decode struct action"),
			TestAction::Send {
				text: "hello".into(),
				count: 2,
			}
		);
	}

	#[test]
	fn action_decode_reports_unknown_variant() {
		let action = test_action("Nope", Vec::new());

		let err = action.decode().expect_err("unknown variant should fail");
		assert!(err.to_string().contains("unknown action variant: Nope"));
	}

	#[test]
	fn action_decode_accepts_legacy_binary_payload() {
		use crate::test_fixtures::HrSensitive;

		// Legacy payload: the hr-sensitive field is bytes, which the current path can't decode
		// (it requests a string), so the legacy fallback must.
		let value = HrSensitive([1, 2, 3, 4]);
		let legacy_args = encode_test_cbor(&value);
		let cbor: ciborium::Value =
			ciborium::from_reader(std::io::Cursor::new(&legacy_args)).expect("decode cbor value");
		assert!(
			matches!(cbor, ciborium::Value::Bytes(_)),
			"legacy form should encode the field as bytes",
		);

		let action = test_action("Tagged", legacy_args);
		assert_eq!(
			action.decode().expect("decode legacy binary action"),
			TestAction::Tagged(value)
		);
	}

	#[test]
	fn action_decode_accepts_json_reshaped_payload() {
		// A byte string reshapes to a JSON number array on the wire; the enum decode path must
		// accept that shape.
		let payload = serde_bytes::ByteBuf::from(vec![1u8, 2, 3, 255]);
		let json = action::to_json_value(&payload, "blob").expect("encode payload as json");
		let args = action::encode_json_as_cbor(&json, "blob").expect("encode payload as cbor");
		let action = test_action("Blob", args);

		assert_eq!(
			action.decode().expect("decode json-reshaped action"),
			TestAction::Blob(payload)
		);
	}

	#[test]
	fn action_decode_as_ignores_action_name() {
		let action = test_action("DefinitelyNotRename", encode_test_cbor(&"alice"));

		assert_eq!(
			action
				.decode_as::<String>()
				.expect("decode raw args as string regardless of action name"),
			"alice"
		);
	}

	#[test]
	fn conn_open_accept_decodes_params_and_sets_typed_state() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let conn = ConnHandle::new(
			"conn-id",
			encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			encode_test_cbor(&TestConnState::default()),
			true,
		);
		let request = Request::new(b"hello".to_vec());
		let (reply_tx, reply_rx) = oneshot::channel();
		let conn_open = ConnOpen::<ConnActor> {
			conn: ConnCtx::from(conn.clone()),
			params: encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			request: Some(request.clone()),
			reply: Some(reply_tx.into()),
		};

		assert_eq!(
			conn_open.params().expect("decode conn params"),
			TestConnParams {
				label: "hello".into(),
			}
		);
		assert_eq!(conn_open.request().expect("request").body(), request.body());
		assert_eq!(conn_open.conn().id(), "conn-id");

		conn_open.accept(TestConnState { value: 7 });

		assert_eq!(
			ConnCtx::<ConnActor>::from(conn.clone())
				.state()
				.expect("decode updated conn state"),
			TestConnState { value: 7 }
		);
		runtime
			.block_on(reply_rx)
			.expect("receive conn-open reply")
			.expect("conn-open accept should succeed");
	}

	#[test]
	fn conn_open_accept_default_uses_default_state() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let conn = ConnHandle::new(
			"conn-id",
			encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			encode_test_cbor(&TestConnState { value: 9 }),
			true,
		);
		let (reply_tx, reply_rx) = oneshot::channel();

		ConnOpen::<ConnActor> {
			conn: ConnCtx::from(conn.clone()),
			params: encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			request: None,
			reply: Some(reply_tx.into()),
		}
		.accept_default();

		assert_eq!(
			ConnCtx::<ConnActor>::from(conn.clone())
				.state()
				.expect("decode reset conn state"),
			TestConnState::default()
		);
		runtime
			.block_on(reply_rx)
			.expect("receive conn-open reply")
			.expect("conn-open accept_default should succeed");
	}

	#[test]
	fn conn_open_reject_sends_error_reply() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		ConnOpen::<ConnActor> {
			conn: ConnCtx::from(ConnHandle::new(
				"conn-id",
				encode_test_cbor(&TestConnParams {
					label: "hello".into(),
				}),
				encode_test_cbor(&TestConnState::default()),
				true,
			)),
			params: encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			request: None,
			reply: Some(reply_tx.into()),
		}
		.reject(anyhow::anyhow!("reject conn"));

		let err = runtime
			.block_on(reply_rx)
			.expect("receive conn-open reject reply")
			.expect_err("conn-open reject should fail");
		assert!(err.to_string().contains("reject conn"));
	}

	#[test]
	fn subscribe_allow_replies_ok() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let conn = ConnHandle::new(
			"conn-id",
			encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			encode_test_cbor(&TestConnState::default()),
			true,
		);
		let (reply_tx, reply_rx) = oneshot::channel();
		let subscribe = Subscribe::<ConnActor> {
			conn: ConnCtx::from(conn),
			event_name: "chat.message".into(),
			reply: Some(reply_tx.into()),
		};

		assert_eq!(subscribe.conn().id(), "conn-id");
		assert_eq!(subscribe.event_name(), "chat.message");

		subscribe.allow();

		runtime
			.block_on(reply_rx)
			.expect("receive subscribe reply")
			.expect("subscribe allow should succeed");
	}

	#[test]
	fn subscribe_deny_replies_err() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		Subscribe::<ConnActor> {
			conn: ConnCtx::from(ConnHandle::new(
				"conn-id",
				encode_test_cbor(&TestConnParams {
					label: "hello".into(),
				}),
				encode_test_cbor(&TestConnState::default()),
				true,
			)),
			event_name: "chat.message".into(),
			reply: Some(reply_tx.into()),
		}
		.deny(anyhow::anyhow!("deny subscribe"));

		let err = runtime
			.block_on(reply_rx)
			.expect("receive subscribe deny reply")
			.expect_err("subscribe deny should fail");
		assert!(err.to_string().contains("deny subscribe"));
	}

	#[test]
	fn http_call_reply_status_builds_expected_response() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		let http = HttpCall {
			request: Some(Request::new(b"hello".to_vec())),
			reply: Some(reply_tx.into()),
		};
		assert_eq!(http.request().expect("request").body(), b"hello");

		http.reply_status(404);

		let response = runtime
			.block_on(reply_rx)
			.expect("receive http reply")
			.expect("http reply_status should succeed");
		let ActorHttpResponse::Buffered(response) = response else {
			panic!("http reply_status should return buffered response");
		};
		assert_eq!(response.status().as_u16(), 404);
		assert!(response.body().is_empty());
	}

	#[test]
	fn ws_open_reject_sends_error_reply() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		let ws_open = WsOpen::<EmptyActor> {
			ws: WebSocket::new(),
			request: Some(Request::new(Vec::new())),
			reply: Some(reply_tx.into()),
			_p: PhantomData,
		};
		assert!(ws_open.request().is_some());
		let _ = ws_open.websocket();

		ws_open.reject(anyhow::anyhow!("reject websocket"));

		let err = runtime
			.block_on(reply_rx)
			.expect("receive websocket reject reply")
			.expect_err("websocket reject should fail");
		assert!(err.to_string().contains("reject websocket"));
	}

	#[test]
	fn serialize_state_save_encodes_actor_state_delta() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		SerializeState::<EmptyActor> {
			reason: SerializeStateReason::Save,
			reply: Some(reply_tx.into()),
			_p: PhantomData,
		}
		.save(&42u32);

		let deltas = runtime
			.block_on(reply_rx)
			.expect("receive serialize-state reply")
			.expect("serialize-state save should succeed");
		assert_eq!(
			deltas,
			vec![StateDelta::ActorState(encode_test_cbor(&42u32))]
		);
	}

	#[test]
	fn sleep_ok_replies_with_unit() {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime");
		let (reply_tx, reply_rx) = oneshot::channel();

		Sleep::<EmptyActor> {
			reply: Some(reply_tx.into()),
			_p: PhantomData,
		}
		.ok();

		runtime
			.block_on(reply_rx)
			.expect("receive sleep reply")
			.expect("sleep ok should succeed");
	}

	#[derive(Serialize)]
	struct SendPayload {
		text: String,
		count: u32,
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct FirstEvent;

	impl Event for FirstEvent {
		const NAME: &'static str = "first";
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct SecondEvent;

	impl Event for SecondEvent {
		const NAME: &'static str = "second";
	}

	#[test]
	fn event_set_unit_registers_nothing() {
		assert!(<() as EventSet>::entries().is_empty());
	}

	#[test]
	fn event_set_tuple_registers_names_in_order() {
		let entries = <(FirstEvent, SecondEvent) as EventSet>::entries();

		assert_eq!(
			entries.iter().map(|entry| entry.name).collect::<Vec<_>>(),
			["first", "second",]
		);
	}

	#[test]
	fn event_set_tuple_supports_one_and_max_arity() {
		assert_eq!(
			<(FirstEvent,) as EventSet>::entries()
				.iter()
				.map(|entry| entry.name)
				.collect::<Vec<_>>(),
			["first"]
		);

		type MaxEvents = (
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
			FirstEvent,
		);
		let entries = <MaxEvents as EventSet>::entries();

		assert_eq!(entries.len(), EVENT_SET_TUPLE_ARITY_MAX);
		assert!(entries.iter().all(|entry| entry.name == "first"));
	}

	fn test_action(name: &str, args: Vec<u8>) -> ActionCall<TestActor> {
		ActionCall {
			name: name.into(),
			args,
			conn: None,
			scheduled_fire: None,
			reply: None,
		}
	}

	#[test]
	fn action_call_exposes_scheduled_fire_metadata() {
		let fire = ScheduledFireInfo {
			kind: rivetkit_core::actor::schedule::ScheduleKind::Every,
			id: "heartbeat".into(),
			name: Some("heartbeat".into()),
			scheduled_at: 100,
			fired_at: 125,
		};
		let action = ActionCall::<TestActor> {
			name: "tick".into(),
			args: Vec::new(),
			conn: None,
			scheduled_fire: Some(fire.clone()),
			reply: None,
		};

		assert_eq!(action.scheduled_fire(), Some(&fire));
	}

	fn encode_test_cbor<T: Serialize>(value: &T) -> Vec<u8> {
		let mut encoded = Vec::new();
		ciborium::into_writer(value, &mut encoded).expect("encode test value as cbor");
		encoded
	}

	fn assert_dropped_reply_logs<T>(
		variant: &'static str,
		identifying: &str,
		drop_wrapper: impl FnOnce() -> oneshot::Receiver<anyhow::Result<T>>,
	) {
		let capture = LogCapture::default();
		let subscriber = tracing_subscriber::fmt()
			.with_ansi(false)
			.with_target(false)
			.with_level(true)
			.with_writer(capture.clone())
			.without_time()
			.finish();
		let _subscriber = tracing::subscriber::set_default(subscriber);

		let runtime = build_runtime();
		let err = match runtime
			.block_on(drop_wrapper())
			.expect("receive dropped reply result")
		{
			Ok(_) => panic!("dropping wrapper should send actor/dropped_reply"),
			Err(err) => err,
		};
		let err = rivet_error::RivetError::extract(&err);
		assert_eq!(err.group(), "actor");
		assert_eq!(err.code(), "dropped_reply");

		let logs = capture.contents();
		assert!(logs.contains("rivetkit event dropped without replying"));
		assert!(logs.contains(variant));
		assert!(logs.contains(identifying));
	}

	fn build_runtime() -> tokio::runtime::Runtime {
		tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("build runtime")
	}

	fn test_conn_handle(id: &str) -> ConnHandle {
		ConnHandle::new(
			id,
			encode_test_cbor(&TestConnParams {
				label: "hello".into(),
			}),
			encode_test_cbor(&TestConnState::default()),
			true,
		)
	}

	fn test_request(uri: &str) -> Request {
		Request::from_parts("GET", uri, HashMap::new(), Vec::new()).expect("build test request")
	}

	#[derive(Clone, Default)]
	struct LogCapture {
		inner: Arc<Mutex<Vec<u8>>>,
	}

	impl LogCapture {
		fn contents(&self) -> String {
			String::from_utf8(self.inner.lock().expect("lock captured logs").clone())
				.expect("captured logs should be utf-8")
		}
	}

	impl<'a> MakeWriter<'a> for LogCapture {
		type Writer = LogWriter;

		fn make_writer(&'a self) -> Self::Writer {
			LogWriter {
				inner: Arc::clone(&self.inner),
			}
		}
	}

	struct LogWriter {
		inner: Arc<Mutex<Vec<u8>>>,
	}

	impl io::Write for LogWriter {
		fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
			self.inner
				.lock()
				.expect("lock captured logs")
				.extend_from_slice(buf);
			Ok(buf.len())
		}

		fn flush(&mut self) -> io::Result<()> {
			Ok(())
		}
	}
}
