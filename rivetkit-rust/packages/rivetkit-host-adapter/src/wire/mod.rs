//! The byte protocol between Rust and a foreign host, shared by every C ABI
//! binding.
//!
//! Rust → host: `event(kind, actor_type, actor, id, header, body, lease)` and
//! `complete(token, status, header, body, lease)`. Host → Rust:
//! `call(target, op, header, body, token)`, `reply(event, status, header, body)`,
//! `lease_release(lease)`, `release(handle)`.
//!
//! - **Ids** are `u64` handles into one table, never pointers. 0 is never valid.
//! - **Header**: small CBOR map, per kind/op ([`messages`]).
//! - **Body**: raw bytes passed through untouched: user CBOR (action args and
//!   results, event args, params, input), HTTP bodies, WebSocket frames.
//! - **Lent vs leased**: `lease == 0` means `body` is valid only during the
//!   callback; copy it. Otherwise Rust keeps the bytes alive until the host calls
//!   `lease_release(lease)`, so the host can read them in place later. Bodies over
//!   [`LEASE_THRESHOLD`] and every stream chunk are leased. Releasing a stream
//!   chunk's lease returns its bytes to the stream's credit window.
//! - **Callbacks must only enqueue.** They run on runtime threads; blocking them
//!   stalls the runtime. Every call returns immediately; results come back through
//!   `complete`, possibly before `call` returns.
//!
//! Event `id`: the event handle to `reply` to for kinds that take a reply,
//! otherwise the resource the event is about (stream, socket, connection) or 0.

mod body;
mod client;
mod host;
pub mod messages;
mod ops;
mod registry;
mod runtime;
mod streams;
#[cfg(test)]
mod tests;

pub use host::WireHost;
pub use runtime::{LEASE_THRESHOLD, Wire};

/// Bumped on any incompatible change to kinds, ops, headers, or rules above.
pub const ABI_VERSION: u32 = 3;

/// What an event is. Kinds marked "reply" carry an event handle in `id` and must
/// be answered with `reply`, even with an error.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
	/// Reply. Header [`messages::MigrateEvent`].
	Migrate = 1,
	/// Reply. Build the per-start object. Header [`messages::StartEvent`].
	Start = 2,
	/// Reply. First start only. Header [`messages::CreateEvent`]; body the input.
	Create = 3,
	/// Reply. Header [`messages::ForkEvent`]; body the fork input.
	Fork = 4,
	/// Reply.
	Wake = 5,
	/// Reply with the CBOR result. Header [`messages::ActionEvent`]; body the args.
	Action = 10,
	/// Reply; an error rejects. Header [`messages::ConnEvent`]; body the params.
	BeforeConnect = 11,
	/// Reply. Header [`messages::ConnEvent`].
	Connect = 12,
	/// Reply. Header [`messages::ConnEvent`].
	Disconnect = 13,
	/// Reply; an error refuses. Header [`messages::SubscribeEvent`].
	Subscribe = 14,
	/// Reply [`messages::RequestReply`]. Header [`messages::RequestEvent`].
	Request = 15,
	/// Reply. Header [`messages::WebSocketEvent`].
	WebSocket = 16,
	/// Reply.
	Sleep = 17,
	/// Reply.
	Destroy = 18,
	/// Core's abort signal fired: shutdown started. `id` 0.
	Abort = 30,
	/// The actor stopped; its handle is gone. `id` 0.
	Released = 31,
	/// Rust stopped waiting for event `id`; stop the work. Header
	/// [`messages::CancelEvent`].
	Cancel = 32,
	/// A chunk of inbound stream `id`. Always leased.
	StreamChunk = 40,
	/// Inbound stream `id` ended. Header [`messages::StreamEndEvent`].
	StreamEnd = 41,
	/// A frame on WebSocket `id`. Header [`messages::WsMessageEvent`].
	WsMessage = 42,
	/// Reply. The socket closed. Header [`messages::WsCloseEvent`].
	WsClose = 43,
	/// An event on client connection `id`. Header
	/// [`messages::ClientEventEvent`]; body the CBOR args.
	ClientEvent = 50,
	/// Client connection `id` changed status. Header
	/// [`messages::ClientStatusEvent`].
	ClientStatus = 51,
	/// Reply with the CBOR connection params as the body. A connection opened with
	/// `params_source` asks before every dial, reconnects included. Header
	/// [`messages::ConnParamsEvent`].
	ConnParams = 52,
	/// A log record. Header [`messages::LogEvent`]. `id` 0.
	Log = 60,
}

/// What a call does. Target and header per op; every op completes its token
/// once, with a header (and body) or an error.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
	// Target 0.
	/// → [`messages::Created`] registry.
	RegistryNew = 1,
	/// [`messages::ClientOptions`] → [`messages::Created`] client.
	ClientNew = 2,

	// Target: registry.
	/// [`messages::RegisterOptions`].
	Register = 10,
	/// [`messages::ServeOptions`]. Completes once serving is ready.
	Serve = 11,
	/// Completes when serving stops.
	ServeDone = 12,
	/// Stops serving; actors get their sleep hooks.
	Stop = 13,

	// Target: actor.
	/// [`messages::BroadcastCall`]; body args.
	Broadcast = 20,
	/// [`messages::ConnSendCall`]; body args.
	ConnSend = 21,
	/// [`messages::ConnDisconnectCall`].
	ConnDisconnect = 22,
	/// → [`messages::ConnList`]. Host-neutral; the Java SDK keeps its own map instead.
	ConnList = 23,
	Sleep = 24,
	Destroy = 25,
	/// → [`messages::Created`] region; `release` ends it.
	KeepAwake = 26,
	/// → [`messages::Created`] region; `release` ends it.
	WaitUntil = 27,
	/// → [`messages::Created`] client, configured like the actor's own.
	ActorClient = 28,

	// Target: actor or transaction.
	/// [`messages::SqlCall`] (no params) → [`messages::SqlResult`]; body rows.
	SqlExec = 30,
	/// [`messages::SqlCall`] → [`messages::SqlResult`]; body rows. Host-neutral; the
	/// Java SDK uses `SqlExecute` for both reads and writes.
	SqlQuery = 31,
	/// [`messages::SqlCall`] → [`messages::SqlResult`] (no rows). Host-neutral, as above.
	SqlRun = 32,
	/// [`messages::SqlCall`] → [`messages::SqlResult`]; body rows.
	SqlExecute = 33,
	/// [`messages::SqlBatchCall`] → [`messages::SqlBatchResult`]; body rows per
	/// statement. Runs in one transaction.
	SqlBatch = 34,
	/// Target actor. [`messages::SqlBeginCall`] → [`messages::Created`]
	/// transaction.
	SqlBegin = 35,
	/// Target transaction.
	SqlCommit = 36,
	/// Target transaction.
	SqlRollback = 37,

	// Target: actor.
	/// [`messages::ScheduleCall`]; body args → [`messages::ScheduleId`].
	Schedule = 40,
	/// [`messages::ScheduleId`] → [`messages::Removed`].
	ScheduleCancel = 41,
	/// → [`messages::ScheduleList`].
	ScheduleList = 42,
	/// [`messages::CronSetCall`]; body args.
	CronSet = 43,
	/// [`messages::NameCall`] → [`messages::Removed`].
	CronDelete = 44,
	/// → [`messages::CronList`].
	CronList = 45,

	// Target: outbound stream (an HTTP response body).
	/// [`messages::StreamWriteCall`]; body the chunk. Completes once accepted.
	StreamWrite = 50,
	/// [`messages::StreamErrorCall`]. Aborts the stream.
	StreamError = 51,

	// Target: WebSocket.
	/// [`messages::WsSendCall`]; body the frame.
	WsSend = 52,
	/// [`messages::WsCloseCall`].
	WsClose = 53,

	// Target: client.
	/// [`messages::ClientActionCall`]; body args → body result.
	ClientAction = 60,
	/// [`messages::ClientConnectCall`] → [`messages::Created`] connection.
	ClientConnect = 61,
	/// [`messages::ClientFetchCall`]; body request → [`messages::FetchResult`].
	ClientFetch = 62,
	/// [`messages::ClientWebSocketCall`] → [`messages::Created`] socket.
	ClientWebSocket = 63,

	// Target: client connection.
	/// [`messages::ConnActionCall`]; body args → body result.
	ConnAction = 70,
	/// [`messages::ConnSubscribeCall`]. Events arrive as `CLIENT_EVENT`.
	ConnSubscribe = 71,
	/// [`messages::ConnSubscribeCall`].
	ConnUnsubscribe = 72,
}

impl Op {
	pub fn from_u32(value: u32) -> Option<Self> {
		use Op::*;
		Some(match value {
			1 => RegistryNew,
			2 => ClientNew,
			10 => Register,
			11 => Serve,
			12 => ServeDone,
			13 => Stop,
			20 => Broadcast,
			21 => ConnSend,
			22 => ConnDisconnect,
			23 => ConnList,
			24 => Sleep,
			25 => Destroy,
			26 => KeepAwake,
			27 => WaitUntil,
			28 => ActorClient,
			30 => SqlExec,
			31 => SqlQuery,
			32 => SqlRun,
			33 => SqlExecute,
			34 => SqlBatch,
			35 => SqlBegin,
			36 => SqlCommit,
			37 => SqlRollback,
			40 => Schedule,
			41 => ScheduleCancel,
			42 => ScheduleList,
			43 => CronSet,
			44 => CronDelete,
			45 => CronList,
			50 => StreamWrite,
			51 => StreamError,
			52 => WsSend,
			53 => WsClose,
			60 => ClientAction,
			61 => ClientConnect,
			62 => ClientFetch,
			63 => ClientWebSocket,
			70 => ConnAction,
			71 => ConnSubscribe,
			72 => ConnUnsubscribe,
			_ => return None,
		})
	}
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
	Ok = 0,
	/// Header is a [`messages::ErrorBody`].
	Error = 1,
}

/// One event for the host. Byte slices are valid only during the callback unless
/// `lease` is non-zero.
#[derive(Debug, Clone, Copy)]
pub struct Event<'a> {
	pub kind: EventKind,
	pub actor_type: u64,
	pub actor: u64,
	pub id: u64,
	pub header: &'a [u8],
	pub body: &'a [u8],
	pub lease: u64,
}

/// How events and completions reach the host. Implementations must not block and
/// must not call back into [`Wire`] while holding locks the host also takes.
pub trait Transport: Send + Sync + 'static {
	fn event(&self, event: Event<'_>);
	fn complete(&self, token: u64, status: Status, header: &[u8], body: &[u8], lease: u64);
}
