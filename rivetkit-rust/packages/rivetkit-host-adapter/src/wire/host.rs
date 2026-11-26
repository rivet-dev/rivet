//! [`HostInstance`] over the wire: each hook becomes an event, delivered in the
//! synchronous half of the call (so in arrival order) and answered by `reply`.

use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use rivetkit_core::websocket::WebSocket;
use rivetkit_core::{ActorContext, ActorForkStart, ConnHandle, Request, WsMessage};
use tokio::sync::oneshot;

use super::messages::{
	ActionEvent, ActorInfo, CancelEvent, ConnEvent, CreateEvent, ForkEvent, MigrateEvent,
	RequestEvent, RequestHead, RequestReply, StartEvent, SubscribeEvent, WebSocketEvent,
	WsCloseEvent, WsMessageEvent, decode, encode,
};
use super::runtime::{ActorSlot, Answer, Handle, Wire};
use super::{EventKind, streams};
use crate::host::{ActionCall, HostActor, HostFuture, HostInstance, HttpReply};
use crate::http::ResponseBody;

/// How long a WebSocket close listener may run. The socket's callback region
/// keeps the actor awake meanwhile.
const WS_CLOSE_TIMEOUT: Duration = Duration::from_secs(30);

/// One actor type served over the wire.
pub struct WireHost {
	wire: Weak<Wire>,
	actor_type: u64,
}

impl WireHost {
	pub fn new(wire: &Arc<Wire>, actor_type: u64) -> Self {
		Self {
			wire: Arc::downgrade(wire),
			actor_type,
		}
	}
}

impl HostActor for WireHost {
	fn start(&self, ctx: &ActorContext) -> Result<Arc<dyn HostInstance>> {
		let wire = self
			.wire
			.upgrade()
			.ok_or_else(|| anyhow!("rivetkit wire runtime is gone"))?;
		let actor = wire.insert(0, Handle::Actor(Arc::new(ActorSlot { ctx: ctx.clone() })));
		Ok(Arc::new(WireInstance {
			to: Target {
				wire,
				actor_type: self.actor_type,
				actor,
			},
			ctx: ctx.clone(),
		}))
	}
}

/// Where events for one actor go.
#[derive(Clone)]
pub(crate) struct Target {
	pub(crate) wire: Arc<Wire>,
	pub(crate) actor_type: u64,
	pub(crate) actor: u64,
}

impl Target {
	/// Delivers `kind` now and returns a future for the reply. Dropping the future
	/// before the reply cancels the event on the host.
	pub(crate) fn call(
		&self,
		kind: EventKind,
		header: Vec<u8>,
		body: Vec<u8>,
	) -> HostFuture<Answer> {
		let (tx, rx) = oneshot::channel();
		let event = self.wire.insert(self.actor, Handle::Event(tx));
		let pending = PendingEvent {
			to: self.clone(),
			event,
			armed: true,
		};
		self.wire
			.emit(kind, self.actor_type, self.actor, event, &header, body);
		Box::pin(async move {
			// Move the whole guard in (closures capture single fields otherwise), so
			// it lives exactly as long as this future.
			let mut pending = pending;
			let result = rx.await;
			pending.armed = false;
			match result {
				Ok(result) => result,
				Err(_) => Err(anyhow!("host dropped event {event} without replying")),
			}
		})
	}

	fn call_unit(&self, kind: EventKind, header: Vec<u8>, body: Vec<u8>) -> HostFuture<()> {
		let future = self.call(kind, header, body);
		Box::pin(async move { future.await.map(|_| ()) })
	}
}

/// Removes an unanswered event and tells the host to stop it.
struct PendingEvent {
	to: Target,
	event: u64,
	armed: bool,
}

impl Drop for PendingEvent {
	fn drop(&mut self) {
		if !self.armed {
			return;
		}
		if matches!(self.to.wire.remove(self.event), Some(Handle::Event(_))) {
			self.to.wire.notify(
				EventKind::Cancel,
				self.to.actor_type,
				self.to.actor,
				0,
				&encode(&CancelEvent { event: self.event }),
			);
		}
	}
}

struct WireInstance {
	to: Target,
	ctx: ActorContext,
}

fn key_strings(key: &rivetkit_core::ActorKey) -> Vec<String> {
	key.iter()
		.map(|segment| match segment {
			rivetkit_core::ActorKeySegment::String(value) => value.clone(),
			rivetkit_core::ActorKeySegment::Number(value) => value.to_string(),
		})
		.collect()
}

pub(crate) fn request_head(request: &Request) -> RequestHead {
	RequestHead {
		method: request.method().to_string(),
		uri: request.uri().to_string(),
		headers: request
			.headers()
			.iter()
			.map(|(name, value)| {
				(
					name.to_string(),
					String::from_utf8_lossy(value.as_bytes()).into_owned(),
				)
			})
			.collect(),
	}
}

impl HostInstance for WireInstance {
	fn on_migrate(&self, is_new: bool) -> HostFuture<()> {
		self.to.call_unit(
			EventKind::Migrate,
			encode(&MigrateEvent { is_new }),
			Vec::new(),
		)
	}

	fn create_vars(&self) -> HostFuture<()> {
		let info = ActorInfo {
			actor_id: self.ctx.actor_id().to_owned(),
			name: self.ctx.name().to_owned(),
			key: key_strings(self.ctx.key()),
			region: self.ctx.region().to_owned(),
		};
		self.to
			.call_unit(EventKind::Start, encode(&StartEvent { info }), Vec::new())
	}

	fn on_create(&self, input: Option<Vec<u8>>) -> HostFuture<()> {
		let header = encode(&CreateEvent {
			has_input: input.is_some(),
		});
		self.to
			.call_unit(EventKind::Create, header, input.unwrap_or_default())
	}

	fn on_fork(&self, fork: ActorForkStart) -> HostFuture<()> {
		let header = encode(&ForkEvent {
			parent_actor_id: fork.link.actor_id.clone(),
			parent_key: key_strings(&fork.link.key),
			forked_at: fork.link.forked_at,
			has_input: fork.input.is_some(),
		});
		self.to
			.call_unit(EventKind::Fork, header, fork.input.unwrap_or_default())
	}

	fn on_wake(&self) -> HostFuture<()> {
		self.to.call_unit(EventKind::Wake, Vec::new(), Vec::new())
	}

	fn action(&self, call: ActionCall) -> HostFuture<Vec<u8>> {
		let header = encode(&ActionEvent {
			name: call.name,
			conn_id: call.conn.map(|conn| conn.id().to_owned()),
			scheduled: call.scheduled,
		});
		let future = self.to.call(EventKind::Action, header, call.args);
		Box::pin(async move { future.await.map(|answer| answer.body) })
	}

	fn on_before_connect(
		&self,
		conn: &ConnHandle,
		params: Vec<u8>,
		request: Option<&Request>,
	) -> HostFuture<()> {
		let header = encode(&ConnEvent {
			conn_id: conn.id().to_owned(),
			request: request.map(request_head),
		});
		self.to.call_unit(EventKind::BeforeConnect, header, params)
	}

	fn on_connect(&self, conn: &ConnHandle, request: Option<&Request>) -> HostFuture<()> {
		let header = encode(&ConnEvent {
			conn_id: conn.id().to_owned(),
			request: request.map(request_head),
		});
		self.to.call_unit(EventKind::Connect, header, Vec::new())
	}

	fn on_disconnect(&self, conn: &ConnHandle) -> HostFuture<()> {
		let header = encode(&ConnEvent {
			conn_id: conn.id().to_owned(),
			request: None,
		});
		self.to.call_unit(EventKind::Disconnect, header, Vec::new())
	}

	fn on_subscribe(&self, conn: &ConnHandle, event: &str) -> HostFuture<()> {
		let header = encode(&SubscribeEvent {
			conn_id: conn.id().to_owned(),
			event: event.to_owned(),
		});
		self.to.call_unit(EventKind::Subscribe, header, Vec::new())
	}

	fn on_request(&self, request: Request, body: ResponseBody) -> HostFuture<HttpReply> {
		let wire = &self.to.wire;
		let response_body = wire.insert(
			self.to.actor,
			Handle::ResponseBody(Arc::new(super::body::OrderedBody::new(wire, body))),
		);
		let stream = request.take_body_stream();
		let body_stream = stream
			.as_ref()
			.map(|_| streams::reserve(wire, self.to.actor));
		let header = encode(&RequestEvent {
			request: request_head(&request),
			body_stream: body_stream.as_ref().map(|reserved| reserved.id),
			response_body,
		});
		let initial = request.into_body();
		let future = self.to.call(EventKind::Request, header, initial);
		// Chunks follow the request event, never precede it.
		if let (Some(reserved), Some(stream)) = (body_stream, stream) {
			let source = futures::stream::unfold(stream, |mut stream| async move {
				match stream.recv().await {
					Ok(Some(chunk)) => Some((Ok(bytes::Bytes::from(chunk)), stream)),
					Ok(None) => None,
					Err(error) => Some((Err(anyhow!("request body aborted: {error}")), stream)),
				}
			});
			streams::start(wire, self.to.actor_type, reserved, Box::pin(source));
		}
		let wire = wire.clone();
		Box::pin(async move {
			let answer = future.await;
			let answer = match answer {
				Ok(answer) => answer,
				Err(error) => {
					wire.remove(response_body);
					return Err(error);
				}
			};
			let reply: RequestReply = decode(&answer.header, "request reply")?;
			if !reply.streamed {
				wire.remove(response_body);
			}
			Ok(HttpReply {
				status: reply.status.unwrap_or(200),
				headers: reply.headers,
				body: answer.body,
				streamed: reply.streamed,
			})
		})
	}

	fn on_websocket(
		&self,
		conn: &ConnHandle,
		ws: WebSocket,
		request: Option<&Request>,
	) -> HostFuture<()> {
		let wire = &self.to.wire;
		let ws_id = wire.insert(self.to.actor, Handle::WebSocket(ws.clone()));
		// Holds back frames until the open event is out, so the host never sees a
		// frame for a socket it does not know yet.
		let gate = Arc::new(Mutex::new(()));
		let opened = gate.lock();

		let to = self.to.clone();
		let message_gate = gate.clone();
		ws.configure_message_event_callback(Some(Arc::new(move |message, index| {
			let _open = message_gate.lock();
			let (binary, data) = match message {
				WsMessage::Text(text) => (false, text.into_bytes()),
				WsMessage::Binary(bytes) => (true, bytes),
			};
			to.wire.emit(
				EventKind::WsMessage,
				to.actor_type,
				to.actor,
				ws_id,
				&encode(&WsMessageEvent { binary, index }),
				data,
			);
			Ok(())
		})));
		let to = self.to.clone();
		ws.configure_close_event_callback(Some(Arc::new(move |code, reason, was_clean| {
			let header = encode(&WsCloseEvent {
				ws: ws_id,
				code,
				reason,
				was_clean,
			});
			let future = to.call(EventKind::WsClose, header, Vec::new());
			let wire = to.wire.clone();
			Box::pin(async move {
				let result = tokio::time::timeout(WS_CLOSE_TIMEOUT, future).await;
				wire.remove(ws_id);
				match result {
					Ok(result) => result.map(|_| ()),
					Err(_) => Err(anyhow!("websocket close listener timed out")),
				}
			})
		})));

		let header = encode(&WebSocketEvent {
			conn_id: conn.id().to_owned(),
			ws: ws_id,
			request: request.map(request_head),
		});
		let future = self.to.call_unit(EventKind::WebSocket, header, Vec::new());
		drop(opened);
		future
	}

	fn on_sleep(&self) -> HostFuture<()> {
		self.to.call_unit(EventKind::Sleep, Vec::new(), Vec::new())
	}

	fn on_destroy(&self) -> HostFuture<()> {
		self.to
			.call_unit(EventKind::Destroy, Vec::new(), Vec::new())
	}

	fn on_abort(&self) {
		self.to
			.wire
			.notify(EventKind::Abort, self.to.actor_type, self.to.actor, 0, &[]);
	}

	fn released(&self) {
		let wire = &self.to.wire;
		wire.release_owned(self.to.actor);
		wire.remove(self.to.actor);
		wire.notify(
			EventKind::Released,
			self.to.actor_type,
			self.to.actor,
			0,
			&[],
		);
	}
}
