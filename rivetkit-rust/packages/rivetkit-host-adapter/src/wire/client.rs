//! Client calls: actions, connections with events, raw HTTP and WebSockets.
//! User values stay CBOR end to end (`action_cbor`); connection params and create
//! input go through JSON because that is how the client protocol carries them.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use rivetkit_client::{
	Client, ClientConfig, ConnectionStatus, GetOptions, GetOrCreateOptions, SubscriptionHandle,
	connection::ActorConnection, handle::ActorHandle,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tungstenite::Message;

use super::EventKind;
use super::host::Target;
use super::messages::{
	ActorTarget, ClientActionCall, ClientConnectCall, ClientEventEvent, ClientFetchCall,
	ClientOptions, ClientStatusEvent, ClientWebSocketCall, ConnParamsEvent, Created, FetchResult,
	WsCloseEvent, WsMessageEvent, encode,
};
use super::runtime::{Answer, Handle, Wire};
use super::streams;

pub(crate) fn new(wire: &Wire, options: ClientOptions) -> Result<Answer> {
	if options.endpoint.is_empty() {
		return Err(anyhow!("client endpoint must not be empty"));
	}
	let mut config = ClientConfig::new(options.endpoint).token_opt(options.token);
	if let Some(namespace) = options.namespace {
		config = config.namespace(namespace);
	}
	if let Some(pool_name) = options.pool_name {
		config = config.pool_name(pool_name);
	}
	Ok(created(
		wire,
		0,
		Handle::Client(Arc::new(Client::new(config))),
	))
}

/// A client configured like the actor's own (actor-to-actor calls).
pub(crate) fn for_actor(wire: &Wire, actor: u64) -> Result<Answer> {
	let slot = wire.actor(actor)?;
	let ctx = &slot.ctx;
	let endpoint = ctx
		.client_endpoint()
		.ok_or_else(|| anyhow!("actor has no client endpoint yet"))?;
	let mut config = ClientConfig::new(endpoint).token_opt(ctx.client_token().map(str::to_owned));
	if let Some(namespace) = ctx.client_namespace() {
		config = config.namespace(namespace);
	}
	if let Some(pool_name) = ctx.client_pool_name() {
		config = config.pool_name(pool_name);
	}
	Ok(created(
		wire,
		actor,
		Handle::Client(Arc::new(Client::new(config))),
	))
}

fn created(wire: &Wire, owner: u64, handle: Handle) -> Answer {
	Answer::header(encode(&Created {
		handle: wire.insert(owner, handle),
	}))
}

fn client(wire: &Wire, id: u64) -> Result<Arc<Client>> {
	wire.get(id, "client", |handle| match handle {
		Handle::Client(client) => Some(client.clone()),
		_ => None,
	})
}

fn cbor_to_json(bytes: &[u8], label: &str) -> Result<serde_json::Value> {
	ciborium::from_reader(bytes).with_context(|| format!("{label} must be JSON-compatible CBOR"))
}

fn handle_for(client: &Client, target: &ActorTarget) -> Result<ActorHandle> {
	let params = target
		.params
		.as_deref()
		.map(|bytes| cbor_to_json(bytes, "connection params"))
		.transpose()?;
	if let Some(actor_id) = &target.actor_id {
		return client.get_for_id(&target.actor, actor_id, GetOptions { params });
	}
	if target.no_create {
		return client.get(&target.actor, target.key.clone(), GetOptions { params });
	}
	let input = target
		.input
		.as_deref()
		.map(|bytes| cbor_to_json(bytes, "create input"))
		.transpose()?;
	client.get_or_create(
		&target.actor,
		target.key.clone(),
		GetOrCreateOptions {
			params,
			create_with_input: input,
			..GetOrCreateOptions::default()
		},
	)
}

pub(crate) fn action(
	wire: &Arc<Wire>,
	id: u64,
	call: ClientActionCall,
	args: &[u8],
	token: u64,
) -> Result<()> {
	let handle = handle_for(&*client(wire, id)?, &call.target)?;
	let args = args.to_vec();
	wire.spawn_complete(token, async move {
		let output = handle.action_cbor(&call.action, &args).await?;
		Ok(Answer {
			header: Vec::new(),
			body: output,
		})
	});
	Ok(())
}

/// A connection the host opened. Events go to the host as `CLIENT_EVENT`.
pub(crate) struct ConnSlot {
	conn: ActorConnection,
	subscriptions: Mutex<HashMap<String, SubscriptionHandle>>,
	tokio: tokio::runtime::Handle,
}

impl ConnSlot {
	pub fn close(&self) {
		let conn = self.conn.clone();
		self.tokio.spawn(async move { conn.disconnect().await });
	}
}

fn conn(wire: &Wire, id: u64) -> Result<Arc<ConnSlot>> {
	wire.get(id, "client connection", |handle| match handle {
		Handle::ClientConn(conn) => Some(conn.clone()),
		_ => None,
	})
}

fn client_closed() -> anyhow::Error {
	anyhow!("the client was closed")
}

fn status_name(status: ConnectionStatus) -> &'static str {
	match status {
		ConnectionStatus::Idle => "idle",
		ConnectionStatus::Connecting => "connecting",
		ConnectionStatus::Connected => "connected",
		ConnectionStatus::Disconnected => "disconnected",
	}
}

pub(crate) fn connect(
	wire: &Arc<Wire>,
	id: u64,
	call: ClientConnectCall,
	token: u64,
) -> Result<()> {
	let handle = handle_for(&*client(wire, id)?, &call.target)?;
	// The client owns what it opens, so releasing it closes them.
	let owner = id;
	let wire = wire.clone();
	let pending = wire.pending(token);
	wire.tokio.clone().spawn(async move {
		let conn = match call.params_source {
			None => handle.connect(),
			Some(source) => {
				// Asked before every dial; an error retries with backoff like a failed dial.
				let ask = Target {
					wire: wire.clone(),
					actor_type: 0,
					actor: owner,
				};
				handle.connect_with_params(move || {
					let answer = ask.call(
						EventKind::ConnParams,
						encode(&ConnParamsEvent { source }),
						Vec::new(),
					);
					async move { cbor_to_json(&answer.await?.body, "connection params") }
				})
			}
		};
		let slot = Arc::new(ConnSlot {
			conn: conn.clone(),
			subscriptions: Mutex::new(HashMap::new()),
			tokio: wire.tokio.clone(),
		});
		let conn_id = match wire.insert_owned(owner, Handle::ClientConn(slot)) {
			Ok(id) => id,
			Err(orphan) => {
				wire.discard(orphan);
				return pending.complete(Err(client_closed()));
			}
		};
		// The host learns `conn_id` from the completion, which travels apart from events.
		// Hold status events until it is out, then flush them in order and add a snapshot
		// of the current status (transitions before the callbacks existed are not reported).
		let status = StatusGate::new(wire.clone(), owner, conn_id);
		let errors = status.clone();
		conn.on_error(move |message| {
			errors.send(ClientStatusEvent {
				status: "error".to_owned(),
				error: Some(super::messages::ErrorBody {
					group: "connection".to_owned(),
					code: "error".to_owned(),
					message: message.to_owned(),
					metadata: None,
				}),
			});
		})
		.await;
		let changes = status.clone();
		conn.on_status_change(move |current| {
			changes.send(ClientStatusEvent {
				status: status_name(current).to_owned(),
				error: None,
			});
		})
		.await;
		pending.complete(Ok(Answer::header(encode(&Created { handle: conn_id }))));
		status.open(|| ClientStatusEvent {
			status: status_name(conn.conn_status()).to_owned(),
			error: None,
		});
	});
	Ok(())
}

/// Status events for one connection, held until the host knows the connection.
#[derive(Clone)]
pub(super) struct StatusGate {
	wire: Arc<Wire>,
	owner: u64,
	conn_id: u64,
	/// `Some` until the host knows the connection: events wait here, in order.
	held: Arc<Mutex<Option<Vec<ClientStatusEvent>>>>,
}

impl StatusGate {
	pub(super) fn new(wire: Arc<Wire>, owner: u64, conn_id: u64) -> Self {
		Self {
			wire,
			owner,
			conn_id,
			held: Arc::new(Mutex::new(Some(Vec::new()))),
		}
	}

	pub(super) fn send(&self, event: ClientStatusEvent) {
		let mut held = self.held.lock();
		match held.as_mut() {
			Some(waiting) => waiting.push(event),
			None => self.notify(&event),
		}
	}

	/// Flushes the held events, then the current status unless it repeats the last
	/// one. Everything happens under the lock: a concurrent `send` cannot overtake
	/// the flush, and `current` is read after every held event. The client updates
	/// its status before running the callbacks, so it is never older than they are.
	pub(super) fn open(&self, current: impl FnOnce() -> ClientStatusEvent) {
		let mut held = self.held.lock();
		let events = held.take().unwrap_or_default();
		for event in &events {
			self.notify(event);
		}
		let snapshot = current();
		let last_status = events.iter().rev().find(|event| event.error.is_none());
		if last_status != Some(&snapshot) {
			self.notify(&snapshot);
		}
	}

	fn notify(&self, event: &ClientStatusEvent) {
		self.wire.notify(
			EventKind::ClientStatus,
			0,
			self.owner,
			self.conn_id,
			&encode(event),
		);
	}
}

pub(crate) fn conn_action(
	wire: &Arc<Wire>,
	id: u64,
	action: String,
	args: &[u8],
	token: u64,
) -> Result<()> {
	let slot = conn(wire, id)?;
	let args = args.to_vec();
	wire.spawn_complete(token, async move {
		let output = slot.conn.action_cbor(&action, args).await?;
		Ok(Answer {
			header: Vec::new(),
			body: output,
		})
	});
	Ok(())
}

pub(crate) fn subscribe(wire: &Arc<Wire>, id: u64, event: String, token: u64) -> Result<()> {
	let slot = conn(wire, id)?;
	let owner = wire.owner_of(id);
	let events = wire.clone();
	wire.spawn_complete(token, async move {
		if slot.subscriptions.lock().contains_key(&event) {
			return Ok(Answer::empty());
		}
		let header = encode(&ClientEventEvent {
			name: event.clone(),
		});
		let subscription = slot
			.conn
			.on_event_raw(&event, move |event| {
				events.emit(
					EventKind::ClientEvent,
					0,
					owner,
					id,
					&header,
					event.raw_args,
				);
			})
			.await;
		slot.subscriptions.lock().insert(event, subscription);
		Ok(Answer::empty())
	});
	Ok(())
}

pub(crate) fn unsubscribe(wire: &Arc<Wire>, id: u64, event: String, token: u64) -> Result<()> {
	let slot = conn(wire, id)?;
	wire.spawn_complete(token, async move {
		let subscription = slot.subscriptions.lock().remove(&event);
		if let Some(subscription) = subscription {
			subscription.unsubscribe().await;
		}
		Ok(Answer::empty())
	});
	Ok(())
}

pub(crate) fn fetch(
	wire: &Arc<Wire>,
	id: u64,
	call: ClientFetchCall,
	body: &[u8],
	token: u64,
) -> Result<()> {
	let handle = handle_for(&*client(wire, id)?, &call.target)?;
	let method = http::Method::from_bytes(call.method.as_bytes())
		.with_context(|| format!("invalid http method `{}`", call.method))?;
	let mut headers = http::HeaderMap::new();
	for (name, value) in &call.headers {
		headers.insert(
			http::header::HeaderName::from_bytes(name.as_bytes())
				.with_context(|| format!("invalid header name `{name}`"))?,
			http::HeaderValue::from_str(value)
				.with_context(|| format!("invalid value for header `{name}`"))?,
		);
	}
	let body = (!body.is_empty()).then(|| bytes::Bytes::copy_from_slice(body));
	// The client owns what it opens, so releasing it closes them.
	let owner = id;
	let wire = wire.clone();
	let task_wire = wire.clone();
	let pending = wire.pending(token);
	wire.tokio.spawn(async move {
		let result = handle.fetch(&call.path, method, headers, body).await;
		let response = match result {
			Ok(response) => response,
			Err(error) => return pending.complete(Err(error)),
		};
		let status = response.status().as_u16();
		let headers = response
			.headers()
			.iter()
			.map(|(name, value)| {
				(
					name.to_string(),
					String::from_utf8_lossy(value.as_bytes()).into_owned(),
				)
			})
			.collect();
		let Some(reserved) = streams::reserve_owned(&task_wire, owner) else {
			return pending.complete(Err(client_closed()));
		};
		pending.complete(Ok(Answer::header(encode(&FetchResult {
			status,
			headers,
			body: reserved.id,
		}))));
		let source = futures::stream::unfold(response, |mut response| async move {
			match response.chunk().await {
				Ok(Some(chunk)) => Some((Ok(chunk), response)),
				Ok(None) => None,
				Err(error) => Some((Err(anyhow!("response body failed: {error}")), response)),
			}
		});
		streams::start(&task_wire, 0, reserved, Box::pin(source));
	});
	Ok(())
}

/// Frames a client socket queues before `send` waits. The writer awaits each frame
/// on the socket, so a slow peer fills this, and then the host's send call waits:
/// TCP flow control reaches the caller instead of memory growing without bound.
const SOCKET_SEND_QUEUE: usize = 64;

/// A raw WebSocket the host opened to an actor.
pub(crate) struct SocketSlot {
	outgoing: mpsc::Sender<Message>,
	cancel: CancellationToken,
}

impl SocketSlot {
	/// Completes once the frame is queued; waits while the queue is full.
	pub async fn send(&self, message: Message) -> Result<()> {
		self.outgoing
			.send(message)
			.await
			.map_err(|_| anyhow!("websocket is closed"))
	}

	pub fn close(&self) {
		self.cancel.cancel();
	}
}

pub(crate) fn web_socket(
	wire: &Arc<Wire>,
	id: u64,
	call: ClientWebSocketCall,
	token: u64,
) -> Result<()> {
	let handle = handle_for(&*client(wire, id)?, &call.target)?;
	// The client owns what it opens, so releasing it closes them.
	let owner = id;
	let wire = wire.clone();
	let task_wire = wire.clone();
	let pending = wire.pending(token);
	wire.tokio.spawn(async move {
		let protocols = (!call.protocols.is_empty()).then_some(call.protocols);
		let socket = match handle.web_socket(&call.path, protocols).await {
			Ok(socket) => socket,
			Err(error) => return pending.complete(Err(error)),
		};
		let (mut sink, mut source) = socket.split();
		let (outgoing, mut outgoing_rx) = mpsc::channel(SOCKET_SEND_QUEUE);
		let cancel = CancellationToken::new();
		let slot = Handle::ClientSocket(Arc::new(SocketSlot {
			outgoing,
			cancel: cancel.clone(),
		}));
		let socket_id = match task_wire.insert_owned(owner, slot) {
			Ok(id) => id,
			Err(_orphan) => {
				let _ = sink.send(Message::Close(None)).await;
				return pending.complete(Err(client_closed()));
			}
		};
		pending.complete(Ok(Answer::header(encode(&Created { handle: socket_id }))));
		let (code, reason, was_clean) = 'socket: loop {
			tokio::select! {
				_ = cancel.cancelled() => {
					let _ = sink.send(Message::Close(None)).await;
					break (1000, String::new(), true);
				}
				message = outgoing_rx.recv() => match message {
					Some(message) => {
						let closing = matches!(message, Message::Close(_));
						// A peer that stopped reading must not keep a closed socket's
						// senders waiting until TCP gives up.
						let sent = tokio::select! {
							sent = sink.send(message) => sent,
							_ = cancel.cancelled() => break 'socket (1000, String::new(), true),
						};
						if sent.is_err() {
							break (1006, "send failed".to_owned(), false);
						}
						if closing {
							// Keep reading until the peer's close frame arrives.
							continue;
						}
					}
					None => break (1000, String::new(), true),
				},
				message = source.next() => match message {
					Some(Ok(Message::Text(text))) => emit_frame(&task_wire, owner, socket_id, false, text.as_bytes().to_vec()),
					Some(Ok(Message::Binary(bytes))) => emit_frame(&task_wire, owner, socket_id, true, bytes.to_vec()),
					Some(Ok(Message::Close(frame))) => break match frame {
						Some(frame) => (u16::from(frame.code), frame.reason.to_string(), true),
						None => (1005, String::new(), true),
					},
					Some(Ok(_)) => {}
					Some(Err(error)) => break (1006, error.to_string(), false),
					None => break (1006, "connection lost".to_owned(), false),
				},
			}
		};
		task_wire.remove(socket_id);
		// Client sockets take no reply: `id` is 0 and the socket is in the header.
		task_wire.notify(
			EventKind::WsClose,
			0,
			owner,
			0,
			&encode(&WsCloseEvent {
				ws: socket_id,
				code,
				reason,
				was_clean,
			}),
		);
	});
	Ok(())
}

fn emit_frame(wire: &Wire, owner: u64, socket: u64, binary: bool, data: Vec<u8>) {
	wire.emit(
		EventKind::WsMessage,
		0,
		owner,
		socket,
		&encode(&WsMessageEvent {
			binary,
			index: None,
		}),
		data,
	);
}
