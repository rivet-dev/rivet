//! Bounded WebSocket forwarding tied to one admitted actor generation and UPS stream.
use anyhow::{Context, Result, bail, ensure};
use futures_util::{Sink, SinkExt, StreamExt};
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocket};
use rivet_env::{Clock, ClockError};
use rivet_envoy_protocol as p;
use std::future::Future;
use std::sync::Arc;
use universalpubsub::{DuplexStream, PacketReceiver, PacketSender};
use vbare::Versioned;

/// Whichever peer has made progress, or the keepalive interval; the loop owns both directions'
/// ordering.
enum Input {
	Actor(Result<Option<bytes::Bytes>>),
	Client(Option<Result<Message, axum::Error>>),
	Keepalive(Result<u64, ClockError>),
}

/// One stream to the actor's host as the tunnel uses it: a UPS `DuplexStream`, or an in-memory
/// actor in tests.
pub trait TunnelStream: Send {
	type Sender: TunnelSender;
	type Receiver: TunnelReceiver;

	fn split(self) -> (Self::Sender, Self::Receiver);
}

pub trait TunnelSender: Send {
	fn send(&mut self, packet: bytes::Bytes) -> impl Future<Output = Result<()>> + Send;
	fn finish(self) -> impl Future<Output = Result<()>> + Send;
}

pub trait TunnelReceiver: Send {
	fn recv(&mut self) -> impl Future<Output = Result<Option<bytes::Bytes>>> + Send;
}

impl TunnelStream for DuplexStream {
	type Sender = PacketSender;
	type Receiver = PacketReceiver;

	fn split(self) -> (PacketSender, PacketReceiver) {
		DuplexStream::split(self)
	}
}

impl TunnelSender for PacketSender {
	fn send(&mut self, packet: bytes::Bytes) -> impl Future<Output = Result<()>> + Send {
		PacketSender::send(self, packet)
	}

	fn finish(self) -> impl Future<Output = Result<()>> + Send {
		PacketSender::finish(self)
	}
}

impl TunnelReceiver for PacketReceiver {
	fn recv(&mut self) -> impl Future<Output = Result<Option<bytes::Bytes>>> + Send {
		PacketReceiver::recv(self)
	}
}

/// What `reopen` resolves to: a new stream to the actor, its tunnel identity and its open.
type Reopened<S> = (S, p::MessageId, p::ToEnvoyWebSocketOpen);

/// How the actor answered a WebSocket open.
pub(crate) enum OpenReply {
	Accepted { can_hibernate: bool },
	Closed(p::ToRivetWebSocketClose),
}

/// Whether the actor's host refused the open before the actor admitted it, so the same open
/// may succeed against the actor once it runs.
pub(crate) fn refused_before_admission(close: &p::ToRivetWebSocketClose) -> bool {
	close
		.reason
		.as_deref()
		.is_some_and(pegboard::envoy_check::refuses_admission)
}

/// Sends the open and waits for the actor's answer. The actor sends nothing else before it.
async fn open_actor(
	sender: &mut impl TunnelSender,
	receiver: &mut impl TunnelReceiver,
	identity: &mut p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
) -> Result<OpenReply> {
	send(
		sender,
		identity,
		p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open),
	)
	.await?;
	let bytes = receiver
		.recv()
		.await?
		.context("actor WebSocket tunnel ended before the open was answered")?;
	let message = decode_reply(bytes, identity)?;
	match message {
		p::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(open) => Ok(OpenReply::Accepted {
			can_hibernate: open.can_hibernate,
		}),
		p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => Ok(OpenReply::Closed(close)),
		_ => bail!("WebSocket open answered with another message"),
	}
}

/// Decodes one actor reply on the tunnel of `identity`.
fn decode_reply(
	bytes: bytes::Bytes,
	identity: &p::MessageId,
) -> Result<p::ToRivetTunnelMessageKind> {
	// Decoding shares the packet, so message data reaches the client without a copy.
	let p::ToRivet::ToRivetTunnelMessage(message) = p::versioned::ToRivet::decode(bytes)? else {
		bail!("unexpected WebSocket tunnel response");
	};
	ensure!(
		message.message_id.gateway_id == identity.gateway_id
			&& message.message_id.request_id == identity.request_id,
		"WebSocket response identity changed"
	);
	Ok(message.message_kind)
}

/// The actor side of a WebSocket once the actor answered its open.
pub(crate) struct Tunnel<S: TunnelStream> {
	pub(crate) sender: S::Sender,
	pub(crate) receiver: S::Receiver,
	pub(crate) identity: p::MessageId,
	pub(crate) reply: OpenReply,
}

/// Opens the actor side of a WebSocket. An open the actor's host refuses before admission, as
/// when a pipelined open arrives ahead of the actor's start, is sent once more on the stream
/// `reopen` acquires after the actor runs; a second refusal is the answer. Ordered UPS lanes do
/// not prevent the first refusal, because the start reaches the Envoy from the pegboard executor
/// while the open travels on this gateway's stream.
pub(crate) async fn open_tunnel<S, Reopen, ReopenFut>(
	stream: S,
	mut identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
) -> Result<Tunnel<S>>
where
	S: TunnelStream,
	Reopen: FnOnce() -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	let (mut sender, mut receiver) = stream.split();
	let mut reply = open_actor(&mut sender, &mut receiver, &mut identity, open).await?;
	if let OpenReply::Closed(close) = &reply
		&& refused_before_admission(close)
	{
		tracing::debug!(
			reason = ?close.reason,
			"actor refused a WebSocket open before admission; opening it again once the actor runs"
		);
		if let Err(error) = sender.finish().await {
			tracing::debug!(%error, "refused WebSocket tunnel peer already closed");
		}
		let (stream, next_identity, open) = reopen().await?;
		(sender, receiver) = stream.split();
		identity = next_identity;
		reply = open_actor(&mut sender, &mut receiver, &mut identity, open).await?;
	}
	Ok(Tunnel {
		sender,
		receiver,
		identity,
		reply,
	})
}

/// Forward an upgraded socket until either peer closes. Hibernatable connections
/// acknowledge durable admission; ordinary connections use transport backpressure.
///
/// The client socket is not read until the actor accepts the open, so a refused open that
/// `open_tunnel` sends again is invisible to the client.
///
/// A ping frame goes to the client every `keepalive_interval_ms`. Proxies in front of guard
/// close WebSockets that carry no frame for a while (Cloudflare after roughly 100 seconds), and
/// neither a RivetKit client nor an idle actor sends anything on its own. The ping is sent
/// whether or not the connection was busy: a proxy's idle timer resets on any frame, and
/// watching for the last frame would cost a clock read per forwarded message.
pub async fn proxy<S, Reopen, ReopenFut>(
	socket: &mut WebSocket,
	stream: S,
	identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
	clock: Arc<dyn Clock>,
	keepalive_interval_ms: u64,
) -> Result<()>
where
	S: TunnelStream,
	Reopen: Fn() -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	let Tunnel {
		mut sender,
		mut receiver,
		mut identity,
		reply,
	} = open_tunnel(stream, identity, open, &reopen).await?;
	let mut can_hibernate = match reply {
		OpenReply::Accepted { can_hibernate } => can_hibernate,
		OpenReply::Closed(close) => {
			socket
				.send(Message::Close(close.code.map(|code| CloseFrame {
					code,
					reason: close.reason.unwrap_or_default().to_string().into(),
				})))
				.await?;
			sender.finish().await?;
			return Ok(());
		}
	};
	// The first tick completes at once and is skipped; the socket was just opened. Missed ticks
	// are skipped too, so a connection that was busy for a while sends one ping, not a burst.
	let mut keepalive = rivet_env::interval(clock, keepalive_interval_ms);
	keepalive.tick().await.context("keepalive clock failed")?;

	let mut credit = ClientCredit::default();

	loop {
		// Replayed frames use the same one-message credit as live client input.
		let input = if let Some(message) = credit.next_replay() {
			Input::Client(Some(Ok(message)))
		} else {
			rivet_env::select_biased!("actor replies release WebSocket send credit before more client input, and a keepalive ping never preempts either",
			packet = receiver.recv() => Input::Actor(packet),
			message = async {
				if credit.is_free() {
					socket.next().await
				} else {
					futures_util::future::pending().await
				}
			} => Input::Client(message),
			tick = keepalive.tick() => Input::Keepalive(tick),
			)
		};

		match input {
			// Runtime acknowledgements release one message of client-side credit.
			Input::Actor(packet) => {
				let bytes = packet?.context("actor WebSocket tunnel ended")?;
				match decode_reply(bytes, &identity)? {
					p::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_) => {
						bail!("duplicate WebSocket open");
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message) => {
						let message = if message.binary {
							Message::Binary(message.data.into())
						} else {
							Message::Text(message.data.try_into()?)
						};
						socket.send(message).await?;
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(ack) => {
						ensure!(
							credit.acknowledge(ack.index),
							"unexpected WebSocket acknowledgement"
						);
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close)
						if can_hibernate && actor_stopped(&close) =>
					{
						// The actor went to sleep. Keep the client socket, answer pings
						// ourselves and wake the actor on the next client frame, re-opening the
						// tunnel with the same gateway/request ids so the actor restores the
						// hibernated connection.
						if let Err(error) = sender.finish().await {
							tracing::debug!(%error, "stopped WebSocket tunnel peer already closed");
						}
						tracing::debug!(
							"actor stopped; client WebSocket hibernating at the gateway"
						);
						// A frame the actor never acknowledged must be replayed, so wake
						// at once instead of waiting for another client frame.
						if !credit.requeue_unacked() {
							let first = hibernate_wait(socket, &mut keepalive).await?;
							credit.replay.push_back(first);
						}

						// The wake opens the tunnel like the first open: `reopen` retries refused
						// acquisitions within the retry budget, and `open_tunnel` opens once more
						// when the woken actor's host refuses the open before admission. Every
						// attempt carries the hibernated connection's ids.
						let hibernated = identity.clone();
						let (stream, woken, open) = reopen_hibernated(&reopen, &hibernated).await?;
						let tunnel = open_tunnel(stream, woken, open, || {
							reopen_hibernated(&reopen, &hibernated)
						})
						.await?;
						match tunnel.reply {
							OpenReply::Accepted {
								can_hibernate: still_hibernatable,
							} => can_hibernate = still_hibernatable,
							OpenReply::Closed(close) => {
								socket
									.send(Message::Close(close.code.map(|code| CloseFrame {
										code,
										reason: close.reason.unwrap_or_default().to_string().into(),
									})))
									.await?;
								tunnel.sender.finish().await?;
								return Ok(());
							}
						}
						sender = tunnel.sender;
						receiver = tunnel.receiver;
						identity = tunnel.identity;
						tracing::debug!("client WebSocket woke the actor and was re-attached");
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
						socket
							.send(Message::Close(close.code.map(|code| CloseFrame {
								code,
								reason: close.reason.unwrap_or_default().to_string().into(),
							})))
							.await?;
						sender.finish().await?;
						return Ok(());
					}
					_ => bail!("unexpected WebSocket response kind"),
				}
			}
			// Client close follows all messages already accepted by the runtime.
			Input::Client(message) => {
				match message.transpose()? {
					Some(Message::Text(text)) => {
						if can_hibernate {
							credit.sent(identity.message_index, Message::Text(text.clone()));
						}
						send(
							&mut sender,
							&mut identity,
							p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
								p::ToEnvoyWebSocketMessage {
									data: text.into(),
									binary: false,
								},
							),
						)
						.await?;
					}
					Some(Message::Binary(data)) => {
						if can_hibernate {
							credit.sent(identity.message_index, Message::Binary(data.clone()));
						}
						send(
							&mut sender,
							&mut identity,
							p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
								p::ToEnvoyWebSocketMessage { data, binary: true },
							),
						)
						.await?;
					}
					Some(Message::Close(close)) => {
						send(
							&mut sender,
							&mut identity,
							p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
								p::ToEnvoyWebSocketClose {
									code: close.as_ref().map(|c| c.code),
									reason: (close.map(|c| c.reason.to_string())).map(Into::into),
								},
							),
						)
						.await?;
						sender.finish().await?;
						return Ok(());
					}
					None => return Ok(()),
					// Axum queues protocol pong/close responses automatically.
					Some(Message::Ping(_) | Message::Pong(_)) => {}
				}
			}
			// A keepalive interval passed. A ping that cannot be sent means the socket is
			// gone, which ends the tunnel like any other failed send. The client's WebSocket
			// library answers the pong on its own.
			Input::Keepalive(tick) => {
				tick.context("keepalive clock failed")?;
				send_keepalive_ping(socket).await?;
			}
		}
	}
}

/// Sends one keepalive ping to the client and counts it. A failed send means the client socket
/// is gone, so the caller ends its forwarding loop on the returned error.
pub(crate) async fn send_keepalive_ping<S>(client: &mut S) -> Result<()>
where
	S: Sink<Message, Error = axum::Error> + Unpin,
{
	let sent = client.send(Message::Ping(bytes::Bytes::new())).await;
	let result = if sent.is_ok() { "ok" } else { "error" };
	crate::metrics::WEBSOCKET_KEEPALIVE_PING_TOTAL
		.with_label_values(&[result])
		.inc();
	sent.context("keepalive ping to the client failed")
}

/// One-message send credit for client frames to a hibernatable actor, and the frames to
/// deliver first after a wake. A frame holds the credit from send until the actor acknowledges
/// it; replayed frames are paced exactly like live client input.
#[derive(Default)]
struct ClientCredit {
	/// Message index awaiting acknowledgement.
	pending: Option<u16>,
	/// The frame behind `pending`, kept to replay it if the actor sleeps first.
	unacked: Option<Message>,
	/// Frames to deliver, in order, before reading the client again.
	replay: std::collections::VecDeque<Message>,
}

impl ClientCredit {
	fn is_free(&self) -> bool {
		self.pending.is_none()
	}

	/// The next replayed frame, only while no frame awaits acknowledgement.
	fn next_replay(&mut self) -> Option<Message> {
		if self.pending.is_some() {
			return None;
		}
		self.replay.pop_front()
	}

	fn sent(&mut self, index: u16, message: Message) {
		self.pending = Some(index);
		self.unacked = Some(message);
	}

	/// Returns whether `index` was the awaited acknowledgement.
	fn acknowledge(&mut self, index: u16) -> bool {
		if self.pending != Some(index) {
			return false;
		}
		self.pending = None;
		self.unacked = None;
		true
	}

	/// The actor stopped: the unacknowledged frame goes first in the replay. Returns whether
	/// frames are queued, in which case the actor must be woken without waiting for the client.
	fn requeue_unacked(&mut self) -> bool {
		self.pending = None;
		if let Some(message) = self.unacked.take() {
			self.replay.push_front(message);
		}
		!self.replay.is_empty()
	}
}

/// Waits, with the actor asleep, for the next client frame that must
/// reach the actor. Pings are answered by the WebSocket layer and the gateway keeps pinging the
/// client, so neither wakes the actor. A client close or disconnect is returned as a close frame
/// so the actor still learns that the connection ended.
async fn hibernate_wait(
	socket: &mut WebSocket,
	keepalive: &mut rivet_env::tokio::time::Interval,
) -> Result<Message> {
	loop {
		rivet_env::select_biased!("a client frame wakes the actor; keepalive pings only touch the client",
			message = socket.next() => match message {
				Some(Ok(message @ (Message::Text(_) | Message::Binary(_) | Message::Close(_)))) => return Ok(message),
				// A dropped connection still tells the actor that the client is gone.
				None | Some(Err(_)) => return Ok(Message::Close(None)),
				Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
			},
			tick = keepalive.tick() => {
				tick.context("keepalive clock failed")?;
				send_keepalive_ping(socket).await?;
			},
		);
	}
}

/// Whether the actor's close says it stopped to sleep rather than closed the connection.
fn actor_stopped(close: &p::ToRivetWebSocketClose) -> bool {
	close.code == Some(1012) && close.reason.as_deref() == Some("actor.stopped")
}

/// Opens a new stream to the woken actor under the hibernated connection's gateway and request
/// ids, which the actor's restore path looks the persisted connection up by.
async fn reopen_hibernated<S, Reopen, ReopenFut>(
	reopen: &Reopen,
	hibernated: &p::MessageId,
) -> Result<Reopened<S>>
where
	Reopen: Fn() -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	let (stream, mut identity, open) = reopen().await?;
	identity.gateway_id = hibernated.gateway_id;
	identity.request_id = hibernated.request_id;
	Ok((stream, identity, open))
}

async fn send(
	sender: &mut impl TunnelSender,
	identity: &mut p::MessageId,
	kind: p::ToEnvoyTunnelMessageKind,
) -> Result<()> {
	let bytes = p::versioned::ToEnvoy::serialize_with_embedded_version(
		&p::ToEnvoy::ToEnvoyTunnelMessage(p::ToEnvoyTunnelMessage {
			message_id: identity.clone(),
			message_kind: kind,
		}),
		p::PROTOCOL_VERSION,
	)?;
	sender.send(bytes.into()).await?;
	identity.message_index = identity.message_index.wrapping_add(1);
	Ok(())
}

#[cfg(test)]
#[allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "the tests drive a real client socket against a local axum server"
)]
mod tests {
	use super::*;
	use axum::{Router, extract::ws::WebSocketUpgrade, routing::get};
	use futures_util::SinkExt;
	use rivet_env::TokioClock;
	use tokio::net::TcpListener;
	use tokio::sync::oneshot;
	use tokio_tungstenite::tungstenite::Message as ClientMessage;

	type Client = tokio_tungstenite::WebSocketStream<
		tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
	>;

	/// Connects a client to a server socket that runs `hibernate_wait` once and reports the
	/// result. Returns the connected client and the receiver for the server-side result.
	async fn connect(
		keepalive_ms: u64,
	) -> (
		Client,
		oneshot::Receiver<Result<Message>>,
		tokio::task::JoinHandle<()>,
	) {
		let (tx, rx) = oneshot::channel();
		let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
		let app = Router::new().route(
			"/",
			get(move |upgrade: WebSocketUpgrade| {
				let tx = tx.clone();
				async move {
					upgrade.on_upgrade(move |mut socket| async move {
						let clock = Arc::new(TokioClock::new().unwrap());
						let mut keepalive = rivet_env::interval(clock, keepalive_ms);
						keepalive.tick().await.unwrap();
						let result = hibernate_wait(&mut socket, &mut keepalive).await;
						if let Some(tx) = tx.lock().unwrap().take() {
							let _ = tx.send(result);
						}
					})
				}
			}),
		);
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			axum::serve(listener, app).await.unwrap();
		});
		let (client, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/"))
			.await
			.unwrap();
		(client, rx, server)
	}

	async fn result(rx: oneshot::Receiver<Result<Message>>) -> Message {
		tokio::time::timeout(std::time::Duration::from_secs(5), rx)
			.await
			.expect("hibernate_wait did not return")
			.unwrap()
			.unwrap()
	}

	#[tokio::test]
	async fn ping_and_pong_do_not_wake_the_actor() {
		let (mut client, mut rx, server) = connect(60_000).await;
		client
			.send(ClientMessage::Ping(vec![1].into()))
			.await
			.unwrap();
		client
			.send(ClientMessage::Pong(vec![2].into()))
			.await
			.unwrap();
		// The ping is answered by the server socket, and hibernate_wait keeps waiting.
		let pong = tokio::time::timeout(std::time::Duration::from_secs(5), client.next())
			.await
			.unwrap()
			.unwrap()
			.unwrap();
		assert!(matches!(pong, ClientMessage::Pong(_)));
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		assert!(rx.try_recv().is_err(), "ping/pong must not return");

		client.send(ClientMessage::Text("hi".into())).await.unwrap();
		assert!(matches!(result(rx).await, Message::Text(text) if text.as_str() == "hi"));
		server.abort();
	}

	#[tokio::test]
	async fn data_frames_are_returned() {
		let (mut client, rx, server) = connect(60_000).await;
		client
			.send(ClientMessage::Binary(vec![7, 8, 9].into()))
			.await
			.unwrap();
		assert!(matches!(result(rx).await, Message::Binary(data) if data.as_ref() == [7, 8, 9]));
		server.abort();
	}

	#[tokio::test]
	async fn client_close_is_returned() {
		let (mut client, rx, server) = connect(60_000).await;
		client.close(None).await.unwrap();
		assert!(matches!(result(rx).await, Message::Close(_)));
		server.abort();
	}

	#[tokio::test]
	async fn client_eof_becomes_close() {
		let (client, rx, server) = connect(60_000).await;
		drop(client);
		assert!(matches!(result(rx).await, Message::Close(_)));
		server.abort();
	}

	#[tokio::test]
	async fn keepalive_pings_the_client() {
		let (mut client, mut rx, server) = connect(50).await;
		let ping = tokio::time::timeout(std::time::Duration::from_secs(5), client.next())
			.await
			.unwrap()
			.unwrap()
			.unwrap();
		assert!(matches!(ping, ClientMessage::Ping(_)));
		assert!(rx.try_recv().is_err(), "keepalive must not return");
		server.abort();
	}

	fn text(value: &str) -> Message {
		Message::Text(value.into())
	}

	#[test]
	fn unacked_frame_wakes_without_a_new_client_frame() {
		let mut credit = ClientCredit::default();
		credit.sent(4, text("request"));
		// The actor stops before acknowledging: the frame needs replay, so no wait.
		assert!(credit.requeue_unacked());
		assert!(matches!(credit.next_replay(), Some(Message::Text(t)) if t.as_str() == "request"));
		// With nothing unacknowledged, the gateway must wait for a client frame.
		let mut idle = ClientCredit::default();
		assert!(!idle.requeue_unacked());
	}

	#[test]
	fn replay_is_delivered_in_order_one_frame_at_a_time() {
		let mut credit = ClientCredit::default();
		credit.sent(4, text("first"));
		assert!(credit.requeue_unacked());
		credit.replay.push_back(text("second"));

		let first = credit.next_replay().unwrap();
		assert!(matches!(&first, Message::Text(t) if t.as_str() == "first"));
		credit.sent(10, first);
		// The second frame waits for the first acknowledgement and cannot overwrite it.
		assert!(credit.next_replay().is_none());
		assert!(!credit.is_free());
		assert!(!credit.acknowledge(11));
		assert!(credit.acknowledge(10));

		let second = credit.next_replay().unwrap();
		assert!(matches!(&second, Message::Text(t) if t.as_str() == "second"));
		credit.sent(11, second);
		assert!(credit.next_replay().is_none());
		assert!(credit.acknowledge(11));
		assert!(credit.is_free());
		assert!(credit.next_replay().is_none());
	}
	/// One end of an in-memory stream between the gateway and a scripted actor.
	struct FakeStream {
		to_actor: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
		from_actor: tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>,
	}

	struct FakeSender(tokio::sync::mpsc::UnboundedSender<bytes::Bytes>);

	struct FakeReceiver(tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>);

	impl TunnelStream for FakeStream {
		type Sender = FakeSender;
		type Receiver = FakeReceiver;

		fn split(self) -> (FakeSender, FakeReceiver) {
			(FakeSender(self.to_actor), FakeReceiver(self.from_actor))
		}
	}

	impl TunnelSender for FakeSender {
		fn send(&mut self, packet: bytes::Bytes) -> impl Future<Output = Result<()>> + Send {
			let sent = self
				.0
				.send(packet)
				.map_err(|_| anyhow::anyhow!("fake actor stream closed"));
			async move { sent }
		}

		async fn finish(self) -> Result<()> {
			Ok(())
		}
	}

	impl TunnelReceiver for FakeReceiver {
		async fn recv(&mut self) -> Result<Option<bytes::Bytes>> {
			Ok(self.0.recv().await)
		}
	}

	/// The actor's end of a `FakeStream`.
	struct FakeActor {
		from_gateway: tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>,
		to_gateway: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
	}

	impl FakeActor {
		async fn recv(&mut self) -> p::ToEnvoyTunnelMessage {
			let bytes =
				tokio::time::timeout(std::time::Duration::from_secs(5), self.from_gateway.recv())
					.await
					.expect("gateway sent nothing")
					.expect("gateway stream ended");
			let p::ToEnvoy::ToEnvoyTunnelMessage(message) =
				p::versioned::ToEnvoy::decode(bytes).unwrap()
			else {
				panic!("not a tunnel message");
			};
			message
		}

		fn send(&self, id: &p::MessageId, kind: p::ToRivetTunnelMessageKind) {
			let bytes = p::versioned::ToRivet::serialize_with_embedded_version(
				&p::ToRivet::ToRivetTunnelMessage(p::ToRivetTunnelMessage {
					message_id: id.clone(),
					message_kind: kind,
				}),
				p::PROTOCOL_VERSION,
			)
			.unwrap();
			self.to_gateway.send(bytes.into()).unwrap();
		}

		/// Reads the gateway's open and returns its identity.
		async fn recv_open(&mut self) -> p::MessageId {
			let message = self.recv().await;
			assert!(matches!(
				message.message_kind,
				p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_)
			));
			message.message_id
		}
	}

	fn fake_stream() -> (FakeStream, FakeActor) {
		let (to_actor, from_gateway) = tokio::sync::mpsc::unbounded_channel();
		let (to_gateway, from_actor) = tokio::sync::mpsc::unbounded_channel();
		(
			FakeStream {
				to_actor,
				from_actor,
			},
			FakeActor {
				from_gateway,
				to_gateway,
			},
		)
	}

	fn message_id(gateway_id: u8, request_id: u8) -> p::MessageId {
		p::MessageId {
			gateway_id: [gateway_id; 4],
			request_id: [request_id; 4],
			message_index: 0,
		}
	}

	fn websocket_open() -> p::ToEnvoyWebSocketOpen {
		p::ToEnvoyWebSocketOpen {
			actor_id: "actor".to_owned().into(),
			actor_generation: Some(1),
			path: "/".to_owned().into(),
			headers: Default::default(),
		}
	}

	fn ws_close(code: u16, reason: &str) -> p::ToRivetTunnelMessageKind {
		p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(p::ToRivetWebSocketClose {
			code: Some(code),
			reason: Some(reason.to_owned().into()),
			hibernate: false,
		})
	}

	fn ws_accept() -> p::ToRivetTunnelMessageKind {
		p::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(p::ToRivetWebSocketOpen {
			can_hibernate: true,
		})
	}

	/// The woken actor's host refuses the first open before admission, as when the open overtakes
	/// the actor's start. The gateway opens once more on a new stream, under the hibernated ids,
	/// and the client is never closed.
	#[tokio::test]
	async fn wake_opens_again_when_the_woken_actor_refuses_before_admission() {
		let (first, mut asleep) = fake_stream();
		let (refused, mut refusing) = fake_stream();
		let (accepted, mut woken) = fake_stream();
		let streams = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from([
			refused, accepted,
		])));
		let reopens = Arc::new(std::sync::atomic::AtomicUsize::new(0));

		let (done_tx, done_rx) = oneshot::channel();
		let done_tx = Arc::new(std::sync::Mutex::new(Some(done_tx)));
		let first = Arc::new(std::sync::Mutex::new(Some(first)));
		let app = Router::new().route(
			"/",
			get({
				let streams = streams.clone();
				let reopens = reopens.clone();
				move |upgrade: WebSocketUpgrade| async move {
					upgrade.on_upgrade(move |mut socket| async move {
						let stream = first.lock().unwrap().take().unwrap();
						// Each reopen acquires a new stream with fresh ids; the wake must put
						// the hibernated ids back.
						let reopen = move || {
							let stream = streams.lock().unwrap().pop_front();
							reopens.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
							async move {
								let stream = stream.context("no stream left to reopen")?;
								anyhow::Ok((stream, message_id(9, 9), websocket_open()))
							}
						};
						let clock = Arc::new(TokioClock::new().unwrap());
						let result = proxy(
							&mut socket,
							stream,
							message_id(1, 2),
							websocket_open(),
							reopen,
							clock,
							60_000,
						)
						.await;
						if let Some(tx) = done_tx.lock().unwrap().take() {
							let _ = tx.send(result);
						}
					})
				}
			}),
		);
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			axum::serve(listener, app).await.unwrap();
		});
		let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/"))
			.await
			.unwrap();

		// The actor accepts the connection as hibernatable, then goes to sleep.
		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));

		// The client's next frame wakes the actor.
		client
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();

		// The woken actor's host refuses the first open before admission.
		let refused_id = refusing.recv_open().await;
		assert_eq!(refused_id.gateway_id, [1; 4]);
		assert_eq!(refused_id.request_id, [2; 4]);
		refusing.send(&refused_id, ws_close(1011, "envoy.actor_not_found"));

		// The second open, on a new stream and under the same ids, is accepted and receives
		// the frame that woke the actor.
		let woken_id = woken.recv_open().await;
		assert_eq!(woken_id.gateway_id, [1; 4]);
		assert_eq!(woken_id.request_id, [2; 4]);
		woken.send(&woken_id, ws_accept());
		let message = woken.recv().await;
		let p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(data) = message.message_kind
		else {
			panic!("expected the client's frame after the wake");
		};
		assert_eq!(data.data.as_ref(), b"wake");
		woken.send(
			&message.message_id,
			p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				p::ToRivetWebSocketMessageAck {
					index: message.message_id.message_index,
				},
			),
		);
		woken.send(
			&message.message_id,
			p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(p::ToRivetWebSocketMessage {
				data: b"awake".to_vec().into(),
				binary: false,
			}),
		);

		// The client was never closed: its next frame is the woken actor's reply.
		let reply = loop {
			let frame = tokio::time::timeout(std::time::Duration::from_secs(5), client.next())
				.await
				.expect("no reply from the woken actor")
				.unwrap()
				.unwrap();
			if !matches!(frame, ClientMessage::Ping(_) | ClientMessage::Pong(_)) {
				break frame;
			}
		};
		assert!(
			matches!(&reply, ClientMessage::Text(text) if text.as_str() == "awake"),
			"unexpected frame {reply:?}"
		);
		assert_eq!(reopens.load(std::sync::atomic::Ordering::SeqCst), 2);

		// A client close still ends the tunnel cleanly.
		client.close(None).await.unwrap();
		assert!(matches!(
			woken.recv().await.message_kind,
			p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_)
		));
		tokio::time::timeout(std::time::Duration::from_secs(5), done_rx)
			.await
			.unwrap()
			.unwrap()
			.unwrap();
		server.abort();
	}
}
