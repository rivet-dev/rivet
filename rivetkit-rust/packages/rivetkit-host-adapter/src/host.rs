//! The interface a language binding implements.

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use rivetkit_core::websocket::WebSocket;
use rivetkit_core::{ActorContext, ActorForkStart, ConnHandle, Request};

use crate::http::ResponseBody;

/// The future half of a host call. Dropping it means the adapter stopped
/// waiting (timeout, caller gone, shutdown); the host should stop the work.
pub type HostFuture<T> = BoxFuture<'static, anyhow::Result<T>>;

/// One actor type, as the host defines it.
pub trait HostActor: Send + Sync + 'static {
	/// Called once per actor start, before any hook. Returns the host's
	/// per-instance object. Errors fail the start.
	fn start(&self, ctx: &ActorContext) -> anyhow::Result<Arc<dyn HostInstance>>;
}

/// An action call as core delivers it.
pub struct ActionCall {
	pub name: String,
	/// CBOR array of positional arguments, exactly as the caller sent it.
	pub args: Vec<u8>,
	pub conn: Option<ConnHandle>,
	/// Fired by a schedule or cron job rather than a client.
	pub scheduled: bool,
}

/// The host's answer to an HTTP request.
pub struct HttpReply {
	pub status: u16,
	pub headers: HashMap<String, String>,
	/// The whole body. Ignored when `streamed` is set.
	pub body: Vec<u8>,
	/// The body is written through the [`ResponseBody`] the request came with.
	pub streamed: bool,
}

/// One running actor, as the host sees it.
///
/// Each method has two halves. The synchronous part runs on the actor's event
/// loop task, in event arrival order; record or deliver the event there. The
/// returned future runs afterwards, concurrently with later events, and is
/// bounded by the hook's timeout. The adapter only calls hooks the host declared
/// in [`crate::Hooks`]; the defaults here are never reached for absent hooks.
pub trait HostInstance: Send + Sync + 'static {
	/// Before any other hook, on every start. Set up the database schema.
	fn on_migrate(&self, _is_new: bool) -> HostFuture<()> {
		ready()
	}
	/// Builds the host's per-start state (Java: runs the actor constructor). Always
	/// called.
	fn create_vars(&self) -> HostFuture<()>;
	/// First start only, after `create_vars`. `input` is the CBOR create input.
	fn on_create(&self, _input: Option<Vec<u8>>) -> HostFuture<()> {
		ready()
	}
	/// First start of a forked actor, after `on_create`.
	fn on_fork(&self, _fork: ActorForkStart) -> HostFuture<()> {
		ready()
	}
	/// Every start, last.
	fn on_wake(&self) -> HostFuture<()> {
		ready()
	}
	/// Returns the CBOR-encoded result.
	fn action(&self, call: ActionCall) -> HostFuture<Vec<u8>>;
	/// `params` is the CBOR connection params. An error rejects the connection.
	fn on_before_connect(
		&self,
		_conn: &ConnHandle,
		_params: Vec<u8>,
		_request: Option<&Request>,
	) -> HostFuture<()> {
		ready()
	}
	fn on_connect(&self, _conn: &ConnHandle, _request: Option<&Request>) -> HostFuture<()> {
		ready()
	}
	fn on_disconnect(&self, _conn: &ConnHandle) -> HostFuture<()> {
		ready()
	}
	/// An error refuses the subscription.
	fn on_subscribe(&self, _conn: &ConnHandle, _event: &str) -> HostFuture<()> {
		ready()
	}
	/// `body` streams the response when the reply sets `streamed`. A streaming
	/// request body is in `request.take_body_stream()`.
	fn on_request(&self, _request: Request, _body: ResponseBody) -> HostFuture<HttpReply> {
		Box::pin(async { Err(crate::errors::not_configured("onRequest")) })
	}
	/// Install `ws`'s message and close callbacks in the synchronous part, before
	/// returning: frames can arrive as soon as this returns.
	fn on_websocket(
		&self,
		_conn: &ConnHandle,
		_ws: WebSocket,
		_request: Option<&Request>,
	) -> HostFuture<()> {
		Box::pin(async { Err(crate::errors::not_configured("onWebSocket")) })
	}
	fn on_sleep(&self) -> HostFuture<()> {
		ready()
	}
	fn on_destroy(&self) -> HostFuture<()> {
		ready()
	}
	/// Core's abort signal fired: shutdown has started. No reply.
	fn on_abort(&self) {}
	/// The actor stopped and every hook has returned. Drop per-actor resources.
	fn released(&self);
}

fn ready() -> HostFuture<()> {
	Box::pin(async { Ok(()) })
}
