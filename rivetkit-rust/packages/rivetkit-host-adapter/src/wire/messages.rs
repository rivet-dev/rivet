//! CBOR headers for every event and call. Bodies are not here: they travel next
//! to the header as raw bytes (user CBOR, HTTP or WebSocket data) and are never
//! re-encoded.
//!
//! Conventions: CBOR maps, camelCase keys, optional fields may be absent. Inputs
//! from the host reject unknown fields so a rename on one side fails loudly.
//! Golden samples of every header live in `fixtures/`; see the tests.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Hooks;

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
	let mut out = Vec::new();
	// Writing into a Vec cannot fail on I/O, and every header type serializes.
	ciborium::into_writer(value, &mut out).expect("encode cbor header");
	out
}

pub fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8], label: &str) -> anyhow::Result<T> {
	if bytes.is_empty() {
		// An empty header is an empty map.
		return ciborium::from_reader(&[0xa0u8][..])
			.map_err(|error| anyhow::anyhow!("decode {label}: {error}"));
	}
	ciborium::from_reader(bytes).map_err(|error| anyhow::anyhow!("decode {label}: {error}"))
}

/// The body of every non-zero status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
	pub group: String,
	pub code: String,
	pub message: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Empty {}

/// A `u64` handle returned by calls that create something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
	pub handle: u64,
}

// ---------------------------------------------------------------------------
// Events (Rust -> host)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorInfo {
	pub actor_id: String,
	pub name: String,
	pub key: Vec<String>,
	pub region: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateEvent {
	pub is_new: bool,
}

/// `create_vars`: build the per-start object (Java: run the constructor).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartEvent {
	pub info: ActorInfo,
}

/// Body: the CBOR create input, empty when there is none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEvent {
	pub has_input: bool,
}

/// Body: the fork's CBOR input, empty when there is none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkEvent {
	pub parent_actor_id: String,
	pub parent_key: Vec<String>,
	pub forked_at: i64,
	pub has_input: bool,
}

/// Body: CBOR array of positional arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionEvent {
	pub name: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub conn_id: Option<String>,
	pub scheduled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestHead {
	pub method: String,
	pub uri: String,
	pub headers: HashMap<String, String>,
}

/// Body: the CBOR connection params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnEvent {
	pub conn_id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub request: Option<RequestHead>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeEvent {
	pub conn_id: String,
	pub event: String,
}

/// Body: the request bytes received so far. With `bodyStream`, the rest arrives
/// as `STREAM_CHUNK` events on that stream. `responseBody` is the stream to
/// write a streamed response to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEvent {
	pub request: RequestHead,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub body_stream: Option<u64>,
	pub response_body: u64,
}

/// Reply to a `REQUEST` event. Body: the response body unless `streamed`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RequestReply {
	pub status: Option<u16>,
	pub headers: HashMap<String, String>,
	pub streamed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketEvent {
	pub conn_id: String,
	pub ws: u64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub request: Option<RequestHead>,
}

/// Body: the frame. `index` is set for hibernatable sockets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsMessageEvent {
	pub binary: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub index: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsCloseEvent {
	pub ws: u64,
	pub code: u16,
	pub reason: String,
	pub was_clean: bool,
}

/// End of an inbound stream. `error` is set when it was aborted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEndEvent {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub error: Option<ErrorBody>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelEvent {
	pub event: u64,
}

/// Body: CBOR array of event arguments, exactly as the actor sent them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientEventEvent {
	pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatusEvent {
	/// `idle`, `connecting`, `connected`, `disconnected`, or `error` (the server
	/// refused or dropped the connection; it keeps retrying).
	pub status: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub error: Option<ErrorBody>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
	/// `error`, `warn`, `info`, `debug` or `trace`.
	pub level: String,
	pub target: String,
	pub message: String,
}

// ---------------------------------------------------------------------------
// Calls (host -> Rust)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct InitOptions {
	/// `RUST_LOG`-style filter for logs forwarded as `LOG` events. Unset: `warn`.
	pub log_filter: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RegisterOptions {
	pub name: String,
	/// The host's id for this actor type, echoed in every event.
	pub actor_type: u64,
	pub hooks: Hooks,
	pub action_timeout_ms: Option<u32>,
	pub sleep_timeout_ms: Option<u32>,
	pub no_sleep: Option<bool>,
	/// Overrides every lifecycle hook timeout (migrate, start, create, wake,
	/// connect). Unset: napi's per-hook defaults.
	pub hook_timeout_ms: Option<u32>,
	pub request_timeout_ms: Option<u32>,
}

/// Unset fields come from the environment (`ServeConfig::from_env`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ServeOptions {
	pub endpoint: Option<String>,
	pub token: Option<String>,
	pub namespace: Option<String>,
	pub pool_name: Option<String>,
	/// `auto`, `always` or `never`.
	pub engine_spawn: Option<String>,
}

/// Body: CBOR array of event arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BroadcastCall {
	pub name: String,
}

/// Body: CBOR array of event arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnSendCall {
	pub conn_id: String,
	pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnDisconnectCall {
	pub conn_id: String,
	#[serde(default)]
	pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnInfo {
	pub id: String,
	#[serde(with = "serde_bytes")]
	pub params: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnList {
	pub conns: Vec<ConnInfo>,
}

/// A SQLite value. CBOR null, integer, float, text or byte string.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SqlValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(#[serde(with = "serde_bytes")] Vec<u8>),
}

// Hand-written: serde's untagged matching lets text and byte strings stand in for
// each other, which would turn blobs into text.
impl<'de> Deserialize<'de> for SqlValue {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		struct Visitor;

		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = SqlValue;

			fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				f.write_str("null, an integer, a float, a text string or a byte string")
			}

			fn visit_unit<E>(self) -> Result<SqlValue, E> {
				Ok(SqlValue::Null)
			}

			fn visit_none<E>(self) -> Result<SqlValue, E> {
				Ok(SqlValue::Null)
			}

			fn visit_bool<E>(self, value: bool) -> Result<SqlValue, E> {
				Ok(SqlValue::Integer(i64::from(value)))
			}

			fn visit_i64<E>(self, value: i64) -> Result<SqlValue, E> {
				Ok(SqlValue::Integer(value))
			}

			fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<SqlValue, E> {
				i64::try_from(value)
					.map(SqlValue::Integer)
					.map_err(|_| E::custom(format!("integer {value} does not fit in i64")))
			}

			fn visit_f64<E>(self, value: f64) -> Result<SqlValue, E> {
				Ok(SqlValue::Float(value))
			}

			fn visit_str<E>(self, value: &str) -> Result<SqlValue, E> {
				Ok(SqlValue::Text(value.to_owned()))
			}

			fn visit_string<E>(self, value: String) -> Result<SqlValue, E> {
				Ok(SqlValue::Text(value))
			}

			fn visit_bytes<E>(self, value: &[u8]) -> Result<SqlValue, E> {
				Ok(SqlValue::Blob(value.to_vec()))
			}

			fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<SqlValue, E> {
				Ok(SqlValue::Blob(value))
			}
		}

		deserializer.deserialize_any(Visitor)
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqlCall {
	pub sql: String,
	#[serde(default)]
	pub params: Option<Vec<SqlValue>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SqlBatchCall {
	pub statements: Vec<SqlCall>,
}

/// Result of a SQL call. Body: CBOR array of rows, each an array of values in
/// `columns` order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlResult {
	pub columns: Vec<String>,
	pub changes: i64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub last_insert_row_id: Option<i64>,
}

/// Result of a batch. Body: CBOR array with one row array per statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlBatchResult {
	pub results: Vec<SqlResult>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SqlBeginCall {
	pub name: Option<String>,
	pub timeout_ms: Option<u32>,
}

/// Body: CBOR array of action arguments. Exactly one of `delayMs` and
/// `timestampMs` is set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ScheduleCall {
	pub action: String,
	pub delay_ms: Option<u64>,
	pub timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleId {
	pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
	pub removed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEvent {
	pub id: String,
	pub action: String,
	#[serde(with = "serde_bytes")]
	pub args: Vec<u8>,
	pub run_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleList {
	pub events: Vec<ScheduledEvent>,
}

/// Body: CBOR array of action arguments. Exactly one of `intervalMs` and
/// `expression` is set. Setting a job with an existing name replaces it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CronSetCall {
	pub name: String,
	pub action: String,
	pub interval_ms: Option<i64>,
	pub expression: Option<String>,
	pub timezone: Option<String>,
	pub max_history: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NameCall {
	pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
	pub name: String,
	pub action: String,
	#[serde(with = "serde_bytes")]
	pub args: Vec<u8>,
	pub next_run_at: i64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub last_run_at: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub expression: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timezone: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub interval_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronList {
	pub jobs: Vec<CronJob>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct StreamWriteCall {
	/// Ends the stream after this chunk.
	pub fin: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamErrorCall {
	pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct WsSendCall {
	pub binary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct WsCloseCall {
	pub code: Option<u16>,
	pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClientOptions {
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: Option<String>,
	pub pool_name: Option<String>,
}

/// Which actor a client call targets. `params` are connection params (CBOR,
/// sent to the actor as JSON). `input` is the create input (CBOR).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ActorTarget {
	pub actor: String,
	pub key: Vec<String>,
	/// Look up by id instead of name and key.
	pub actor_id: Option<String>,
	/// Fail instead of creating the actor when it does not exist.
	pub no_create: bool,
	#[serde(with = "serde_bytes")]
	pub params: Option<Vec<u8>>,
	#[serde(with = "serde_bytes")]
	pub input: Option<Vec<u8>>,
}

/// Body: CBOR array of positional arguments. Completes with the CBOR result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClientActionCall {
	pub target: ActorTarget,
	pub action: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClientConnectCall {
	pub target: ActorTarget,
}

/// Body: CBOR array of positional arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnActionCall {
	pub action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnSubscribeCall {
	pub event: String,
}

/// Body: the request body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClientFetchCall {
	pub target: ActorTarget,
	pub method: String,
	/// Path under the actor, e.g. `/hello?x=1`.
	pub path: String,
	pub headers: HashMap<String, String>,
}

/// Result of a fetch. The body follows as `STREAM_CHUNK` events on `body`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResult {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClientWebSocketCall {
	pub target: ActorTarget,
	pub path: String,
	pub protocols: Vec<String>,
}

/// Golden samples of every header. `fixtures_are_current` fails when a fixture is
/// stale or missing (regenerate with `UPDATE_FIXTURES=1`). The Java
/// `FixturesTest` decodes each one and checks its own encoding matches.
#[cfg(test)]
pub(crate) mod fixtures {
	use super::*;
	use crate::wire::{EventKind, Op};

	fn info() -> ActorInfo {
		ActorInfo {
			actor_id: "abc".into(),
			name: "chatRoom".into(),
			key: vec!["room-1".into()],
			region: "local".into(),
		}
	}

	fn head() -> RequestHead {
		RequestHead {
			method: "POST".into(),
			uri: "/upload?x=1".into(),
			headers: HashMap::from([("content-type".into(), "text/plain".into())]),
		}
	}

	fn target() -> ActorTarget {
		ActorTarget {
			actor: "chatRoom".into(),
			key: vec!["room-1".into()],
			actor_id: None,
			no_create: false,
			params: Some(encode(&serde_json::json!({ "user": "alice" }))),
			input: None,
		}
	}

	/// The header sample for each event kind. Exhaustive: a new kind fails to
	/// compile until it has a fixture.
	fn event(kind: EventKind) -> (&'static str, Vec<u8>) {
		match kind {
			EventKind::Migrate => ("event_migrate", encode(&MigrateEvent { is_new: true })),
			EventKind::Start => ("event_start", encode(&StartEvent { info: info() })),
			EventKind::Create => ("event_create", encode(&CreateEvent { has_input: true })),
			EventKind::Fork => (
				"event_fork",
				encode(&ForkEvent {
					parent_actor_id: "parent".into(),
					parent_key: vec!["k".into()],
					forked_at: 1_700_000_000_000,
					has_input: false,
				}),
			),
			EventKind::Wake => ("event_wake", Vec::new()),
			EventKind::Action => (
				"event_action",
				encode(&ActionEvent {
					name: "send".into(),
					conn_id: Some("conn-1".into()),
					scheduled: false,
				}),
			),
			EventKind::BeforeConnect | EventKind::Connect | EventKind::Disconnect => (
				"event_conn",
				encode(&ConnEvent {
					conn_id: "conn-1".into(),
					request: Some(head()),
				}),
			),
			EventKind::Subscribe => (
				"event_subscribe",
				encode(&SubscribeEvent {
					conn_id: "conn-1".into(),
					event: "newMessage".into(),
				}),
			),
			EventKind::Request => (
				"event_request",
				encode(&RequestEvent {
					request: head(),
					body_stream: Some(7),
					response_body: 8,
				}),
			),
			EventKind::WebSocket => (
				"event_websocket",
				encode(&WebSocketEvent {
					conn_id: "conn-1".into(),
					ws: 9,
					request: None,
				}),
			),
			EventKind::Sleep | EventKind::Destroy | EventKind::Abort | EventKind::Released => {
				("event_empty", Vec::new())
			}
			EventKind::Cancel => ("event_cancel", encode(&CancelEvent { event: 42 })),
			EventKind::StreamChunk => ("event_stream_chunk", Vec::new()),
			EventKind::StreamEnd => (
				"event_stream_end",
				encode(&StreamEndEvent {
					error: Some(ErrorBody {
						group: "actor".into(),
						code: "aborted".into(),
						message: "client went away".into(),
						metadata: None,
					}),
				}),
			),
			EventKind::WsMessage => (
				"event_ws_message",
				encode(&WsMessageEvent {
					binary: true,
					index: Some(3),
				}),
			),
			EventKind::WsClose => (
				"event_ws_close",
				encode(&WsCloseEvent {
					ws: 9,
					code: 1000,
					reason: "bye".into(),
					was_clean: true,
				}),
			),
			EventKind::ClientEvent => (
				"event_client_event",
				encode(&ClientEventEvent {
					name: "newMessage".into(),
				}),
			),
			EventKind::ClientStatus => (
				"event_client_status",
				encode(&ClientStatusEvent {
					status: "connected".into(),
					error: None,
				}),
			),
			EventKind::Log => (
				"event_log",
				encode(&LogEvent {
					level: "warn".into(),
					target: "rivetkit_core".into(),
					message: "hello".into(),
				}),
			),
		}
	}

	/// The header sample for each op. Exhaustive, like [`event`].
	fn op(op: Op) -> (&'static str, Vec<u8>) {
		match op {
			Op::RegistryNew
			| Op::ServeDone
			| Op::Stop
			| Op::ConnList
			| Op::Sleep
			| Op::Destroy
			| Op::KeepAwake
			| Op::WaitUntil
			| Op::ActorClient
			| Op::SqlCommit
			| Op::SqlRollback
			| Op::ScheduleList
			| Op::CronList => ("call_empty", Vec::new()),
			Op::ClientNew => (
				"call_client_new",
				encode(&ClientOptions {
					endpoint: "http://127.0.0.1:6420".into(),
					token: Some("dev".into()),
					namespace: Some("default".into()),
					pool_name: None,
				}),
			),
			Op::Register => (
				"call_register",
				encode(&RegisterOptions {
					name: "chatRoom".into(),
					actor_type: 1,
					hooks: Hooks {
						on_migrate: true,
						on_wake: true,
						on_connect: true,
						..Hooks::default()
					},
					action_timeout_ms: Some(1000),
					sleep_timeout_ms: Some(2000),
					no_sleep: Some(false),
					hook_timeout_ms: None,
					request_timeout_ms: Some(3000),
				}),
			),
			Op::Serve => (
				"call_serve",
				encode(&ServeOptions {
					endpoint: Some("http://127.0.0.1:6420".into()),
					token: None,
					namespace: Some("default".into()),
					pool_name: Some("pool".into()),
					engine_spawn: Some("never".into()),
				}),
			),
			Op::Broadcast => (
				"call_broadcast",
				encode(&BroadcastCall {
					name: "newMessage".into(),
				}),
			),
			Op::ConnSend => (
				"call_conn_send",
				encode(&ConnSendCall {
					conn_id: "conn-1".into(),
					name: "welcome".into(),
				}),
			),
			Op::ConnDisconnect => (
				"call_conn_disconnect",
				encode(&ConnDisconnectCall {
					conn_id: "conn-1".into(),
					reason: Some("kicked".into()),
				}),
			),
			Op::SqlExec | Op::SqlQuery | Op::SqlRun | Op::SqlExecute => (
				"call_sql",
				encode(&SqlCall {
					sql: "SELECT ?, ?, ?, ?, ?".into(),
					params: Some(vec![
						SqlValue::Null,
						SqlValue::Integer(-5),
						SqlValue::Float(1.5),
						SqlValue::Text("hi".into()),
						SqlValue::Blob(vec![0, 255]),
					]),
				}),
			),
			Op::SqlBatch => (
				"call_sql_batch",
				encode(&SqlBatchCall {
					statements: vec![SqlCall {
						sql: "DELETE FROM t".into(),
						params: None,
					}],
				}),
			),
			Op::SqlBegin => (
				"call_sql_begin",
				encode(&SqlBeginCall {
					name: Some("transfer".into()),
					timeout_ms: Some(5000),
				}),
			),
			Op::Schedule => (
				"call_schedule",
				encode(&ScheduleCall {
					action: "purge".into(),
					delay_ms: Some(60_000),
					timestamp_ms: None,
				}),
			),
			Op::ScheduleCancel => ("call_schedule_id", encode(&ScheduleId { id: "s1".into() })),
			Op::CronSet => (
				"call_cron_set",
				encode(&CronSetCall {
					name: "purge".into(),
					action: "purge".into(),
					interval_ms: Some(3_600_000),
					expression: None,
					timezone: None,
					max_history: Some(10),
				}),
			),
			Op::CronDelete => (
				"call_name",
				encode(&NameCall {
					name: "purge".into(),
				}),
			),
			Op::StreamWrite => ("call_stream_write", encode(&StreamWriteCall { fin: true })),
			Op::StreamError => (
				"call_stream_error",
				encode(&StreamErrorCall {
					message: "boom".into(),
				}),
			),
			Op::WsSend => ("call_ws_send", encode(&WsSendCall { binary: false })),
			Op::WsClose => (
				"call_ws_close",
				encode(&WsCloseCall {
					code: Some(1000),
					reason: Some("done".into()),
				}),
			),
			Op::ClientAction => (
				"call_client_action",
				encode(&ClientActionCall {
					target: target(),
					action: "send".into(),
				}),
			),
			Op::ClientConnect => (
				"call_client_connect",
				encode(&ClientConnectCall { target: target() }),
			),
			Op::ClientFetch => (
				"call_client_fetch",
				encode(&ClientFetchCall {
					target: target(),
					method: "GET".into(),
					path: "/hello".into(),
					headers: HashMap::new(),
				}),
			),
			Op::ClientWebSocket => (
				"call_client_websocket",
				encode(&ClientWebSocketCall {
					target: target(),
					path: "/ws".into(),
					protocols: vec![],
				}),
			),
			Op::ConnAction => (
				"call_conn_action",
				encode(&ConnActionCall {
					action: "send".into(),
				}),
			),
			Op::ConnSubscribe | Op::ConnUnsubscribe => (
				"call_conn_subscribe",
				encode(&ConnSubscribeCall {
					event: "newMessage".into(),
				}),
			),
		}
	}

	/// Replies and results the host or Rust produce.
	fn others() -> Vec<(&'static str, Vec<u8>)> {
		vec![
			(
				"reply_request",
				encode(&RequestReply {
					status: Some(201),
					headers: HashMap::from([("x-a".into(), "b".into())]),
					streamed: true,
				}),
			),
			("result_created", encode(&Created { handle: 12 })),
			(
				"result_sql",
				encode(&SqlResult {
					columns: vec!["id".into(), "text".into()],
					changes: 1,
					last_insert_row_id: Some(4),
				}),
			),
			(
				"result_sql_rows",
				encode(&vec![vec![
					SqlValue::Integer(4),
					SqlValue::Text("hi".into()),
					SqlValue::Blob(vec![1, 2]),
					SqlValue::Null,
					SqlValue::Float(0.25),
				]]),
			),
			(
				"result_conn_list",
				encode(&ConnList {
					conns: vec![ConnInfo {
						id: "conn-1".into(),
						params: encode(&serde_json::json!({ "user": "alice" })),
					}],
				}),
			),
			(
				"result_schedule_list",
				encode(&ScheduleList {
					events: vec![ScheduledEvent {
						id: "s1".into(),
						action: "purge".into(),
						args: encode(&vec![5]),
						run_at: 1_700_000_000_000,
					}],
				}),
			),
			(
				"result_cron_list",
				encode(&CronList {
					jobs: vec![CronJob {
						name: "purge".into(),
						action: "purge".into(),
						args: encode(&Vec::<i32>::new()),
						next_run_at: 1_700_000_000_000,
						last_run_at: None,
						expression: None,
						timezone: None,
						interval_ms: Some(3_600_000),
					}],
				}),
			),
			(
				"result_fetch",
				encode(&FetchResult {
					status: 200,
					headers: HashMap::from([("content-type".into(), "text/plain".into())]),
					body: 13,
				}),
			),
			("result_removed", encode(&Removed { removed: true })),
			(
				"error_body",
				encode(&ErrorBody {
					group: "user".into(),
					code: "empty".into(),
					message: "text must not be blank".into(),
					metadata: Some(serde_json::json!({ "field": "text" })),
				}),
			),
			(
				"init_options",
				encode(&InitOptions {
					log_filter: Some("info".into()),
				}),
			),
		]
	}

	const EVENTS: [EventKind; 24] = [
		EventKind::Migrate,
		EventKind::Start,
		EventKind::Create,
		EventKind::Fork,
		EventKind::Wake,
		EventKind::Action,
		EventKind::BeforeConnect,
		EventKind::Connect,
		EventKind::Disconnect,
		EventKind::Subscribe,
		EventKind::Request,
		EventKind::WebSocket,
		EventKind::Sleep,
		EventKind::Destroy,
		EventKind::Abort,
		EventKind::Released,
		EventKind::Cancel,
		EventKind::StreamChunk,
		EventKind::StreamEnd,
		EventKind::WsMessage,
		EventKind::WsClose,
		EventKind::ClientEvent,
		EventKind::ClientStatus,
		EventKind::Log,
	];

	pub(crate) fn samples() -> Vec<(&'static str, Vec<u8>)> {
		let mut samples: Vec<(&'static str, Vec<u8>)> = Vec::new();
		let mut push = |(name, bytes): (&'static str, Vec<u8>)| {
			if !bytes.is_empty() && !samples.iter().any(|(existing, _)| *existing == name) {
				samples.push((name, bytes));
			}
		};
		for kind in EVENTS {
			push(event(kind));
		}
		for value in 0..=u32::from(u8::MAX) {
			if let Some(op_value) = Op::from_u32(value) {
				push(op(op_value));
			}
		}
		for sample in others() {
			push(sample);
		}
		samples
	}

	#[test]
	fn every_kind_and_op_is_listed() {
		// `event` and `op` are exhaustive matches; this checks the lists feeding them.
		assert_eq!(EVENTS.len(), 24);
		let ops = (0..=255).filter_map(Op::from_u32).count();
		assert_eq!(ops, 40, "update the op list in Op::from_u32");
	}

	#[test]
	fn fixtures_are_current() {
		let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
		let update = std::env::var_os("UPDATE_FIXTURES").is_some();
		if update {
			let _ = std::fs::remove_dir_all(&dir);
			std::fs::create_dir_all(&dir).unwrap();
		}
		for (name, bytes) in samples() {
			let path = dir.join(format!("{name}.cbor"));
			if update {
				std::fs::write(&path, &bytes).unwrap();
				continue;
			}
			let committed = std::fs::read(&path).unwrap_or_else(|_| {
				panic!("fixture {name} is missing; rerun with UPDATE_FIXTURES=1")
			});
			assert_eq!(
				committed, bytes,
				"fixture {name} is stale; rerun with UPDATE_FIXTURES=1"
			);
		}
	}

	#[test]
	fn sql_values_keep_their_cbor_types() {
		let values = vec![
			SqlValue::Null,
			SqlValue::Integer(7),
			SqlValue::Float(1.5),
			SqlValue::Text("a".into()),
			SqlValue::Blob(vec![1]),
		];
		let decoded: Vec<SqlValue> = decode(&encode(&values), "values").unwrap();
		assert_eq!(decoded, values);
	}

	#[test]
	fn unknown_input_fields_are_rejected() {
		let bytes = encode(&serde_json::json!({ "endpoint": "x", "pool": "renamed" }));
		assert!(decode::<ClientOptions>(&bytes, "client options").is_err());
	}

	#[test]
	fn empty_header_is_an_empty_map() {
		let options: InitOptions = decode(&[], "init").unwrap();
		assert_eq!(options, InitOptions::default());
	}
}
