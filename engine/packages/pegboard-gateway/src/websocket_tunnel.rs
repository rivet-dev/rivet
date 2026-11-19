//! Bounded WebSocket forwarding tied to one admitted actor generation and UPS stream.
use anyhow::{Context, Result, bail, ensure};
use futures_util::{Sink, SinkExt, StreamExt};
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocket};
use rivet_env::{Clock, ClockError};
use rivet_envoy_protocol as p;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use universalpubsub::{DuplexStream, PacketReceiver, PacketSender};
use vbare::Versioned;

use crate::acquire::ActorNotRunning;
use crate::sibling_wake::SiblingWake;

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

/// Why a WebSocket opens a new stream to its actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReopenCause {
	/// The socket's own client is behind it: the first open, a frame sent to a sleeping actor,
	/// or a frame the actor never acknowledged. The re-open starts the actor if it must.
	Client,
	/// Another request reached the actor at a newer epoch and the client sent nothing. The
	/// re-open only attaches to an actor that runs. It must fail with `ActorNotRunning`, and
	/// start nothing, when the actor went back to sleep meanwhile.
	Sibling,
}

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
	version: u16,
	open: p::ToEnvoyWebSocketOpen,
) -> Result<OpenReply> {
	send(
		sender,
		identity,
		version,
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
	/// The envoy protocol version of every message to the actor's host on this tunnel.
	pub(crate) version: u16,
	pub(crate) sender: S::Sender,
	pub(crate) receiver: S::Receiver,
	pub(crate) identity: p::MessageId,
	pub(crate) reply: OpenReply,
}

/// Opens the actor side of a WebSocket. An open the actor's host refuses before admission, as
/// when a pipelined open arrives ahead of the actor's start, is sent once more on the stream
/// `reopen` acquires after the actor runs; a second refusal is the answer. Ordered UPS lanes do
/// not prevent the first refusal, because the start reaches the Envoy from the pegboard executor
/// while the open travels on this gateway's stream. Messages to the actor's host are encoded at
/// the version `version_of` gives for the stream they travel on, fixed when it opens; see
/// `pegboard::envoy_protocol`.
pub(crate) async fn open_tunnel<S, VersionOf, Reopen, ReopenFut>(
	stream: S,
	version_of: VersionOf,
	mut identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
) -> Result<Tunnel<S>>
where
	S: TunnelStream,
	VersionOf: Fn(&S) -> u16,
	Reopen: FnOnce() -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	let mut version = version_of(&stream);
	let (mut sender, mut receiver) = stream.split();
	let mut reply = open_actor(&mut sender, &mut receiver, &mut identity, version, open).await?;
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
		version = version_of(&stream);
		(sender, receiver) = stream.split();
		identity = next_identity;
		reply = open_actor(&mut sender, &mut receiver, &mut identity, version, open).await?;
	}
	Ok(Tunnel {
		version,
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
///
/// `wake` is the actor's record of the latest epoch this gateway reached it at. A hibernated
/// socket re-opens its tunnel when its own client sends a frame, and also when that epoch moves
/// past the one it went to sleep on: another request woke the actor, which restored this
/// connection and may already have sent on it.
///
/// `reopen` is told which of the two it is. Only a re-open for the socket's own client may
/// start the actor. A re-open after another request's wake finds the actor asleep again when
/// the actor slept before the socket re-attached, and then the socket goes back to waiting,
/// for its client or for an epoch newer than the one it just looked for. So an actor that
/// sleeps soon after it wakes is never started by its own hibernated sockets.
pub async fn proxy<S, VersionOf, Reopen, ReopenFut>(
	socket: &mut WebSocket,
	stream: S,
	version_of: VersionOf,
	identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
	wake: SiblingWake,
	clock: Arc<dyn Clock>,
	keepalive_interval_ms: u64,
) -> Result<()>
where
	S: TunnelStream,
	VersionOf: Fn(&S) -> u16,
	Reopen: Fn(ReopenCause) -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	// The newest epoch this socket reached the actor at, or looked for it at after another
	// request's wake. Every stream the socket acquires moves it and tells the actor's other
	// sockets on this gateway. A re-open that only attaches cannot have started the epoch it
	// reaches, so telling the others never spreads a wake that no client asked for.
	let epoch = AtomicU32::new(open.actor_generation.unwrap_or_default());
	wake.publish(epoch.load(Ordering::Relaxed));
	let reopen = |cause: ReopenCause| {
		let reopened = reopen(cause);
		let (epoch, wake) = (&epoch, &wake);
		async move {
			let reopened = reopened.await?;
			let reached = reopened.2.actor_generation.unwrap_or_default();
			epoch.fetch_max(reached, Ordering::Relaxed);
			wake.publish(reached);
			anyhow::Ok(reopened)
		}
	};
	let Tunnel {
		mut version,
		mut sender,
		mut receiver,
		mut identity,
		reply,
	} = open_tunnel(stream, &version_of, identity, open, || {
		reopen(ReopenCause::Client)
	})
	.await?;
	let mut can_hibernate = match reply {
		OpenReply::Accepted { can_hibernate } => can_hibernate,
		OpenReply::Closed(close) => {
			return forward_actor_close(socket, sender, close).await;
		}
	};
	// The first tick completes at once and is skipped; the socket was just opened. Missed ticks
	// are skipped too, so a connection that was busy for a while sends one ping, not a burst.
	let mut keepalive = rivet_env::interval(clock.clone(), keepalive_interval_ms);
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
						// ourselves and re-open the tunnel with the same gateway/request ids,
						// so the actor restores the hibernated connection, on the next client
						// frame or as soon as another request has woken the actor.
						if let Err(error) = sender.finish().await {
							tracing::debug!(%error, "stopped WebSocket tunnel peer already closed");
						}
						tracing::debug!(
							"actor stopped; client WebSocket hibernating at the gateway"
						);
						let hibernated = identity.clone();
						let tunnel = loop {
							// A frame the actor never acknowledged must be replayed, so wake
							// at once instead of waiting for another client frame.
							let cause = if credit.requeue_unacked() {
								ReopenCause::Client
							} else {
								let asleep = epoch.load(Ordering::Relaxed);
								match hibernate_wait(
									socket,
									&mut keepalive,
									wake.newer_than(asleep),
								)
								.await?
								{
									Some(first) => {
										credit.replay.push_back(first);
										ReopenCause::Client
									}
									// Nothing to replay: the socket only re-attaches. Whatever
									// the re-open finds, the socket has now looked for the
									// actor at this epoch and next waits for a newer one.
									None => {
										epoch.fetch_max(wake.latest(), Ordering::Relaxed);
										tracing::debug!(
											"another request woke the actor; re-attaching the hibernated client WebSocket"
										);
										ReopenCause::Sibling
									}
								}
							};

							// The wake opens the tunnel like the first open: `reopen` retries
							// refused acquisitions within the retry budget, and `open_tunnel`
							// opens once more when the woken actor's host refuses the open
							// before admission. Every attempt carries the hibernated
							// connection's ids and the same cause, so an actor that stopped
							// between a sibling's two attempts is not started by the second.
							let reopened = async {
								let (stream, woken, open) =
									reopen_hibernated(&reopen, cause, &hibernated).await?;
								open_tunnel(stream, &version_of, woken, open, || {
									reopen_hibernated(&reopen, cause, &hibernated)
								})
								.await
							}
							.await;
							match reopened {
								Ok(tunnel) => break tunnel,
								Err(error)
									if cause == ReopenCause::Sibling
										&& error.is::<ActorNotRunning>() =>
								{
									tracing::debug!(
										"the actor went back to sleep before its hibernated client WebSocket re-attached; waiting again"
									);
								}
								Err(error) => return Err(error),
							}
						};
						match tunnel.reply {
							OpenReply::Accepted {
								can_hibernate: still_hibernatable,
							} => can_hibernate = still_hibernatable,
							OpenReply::Closed(close) => {
								return forward_actor_close(socket, tunnel.sender, close).await;
							}
						}
						version = tunnel.version;
						sender = tunnel.sender;
						receiver = tunnel.receiver;
						identity = tunnel.identity;
						tracing::debug!("hibernated client WebSocket was re-attached to the actor");
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
						return forward_actor_close(socket, sender, close).await;
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
						let sent = send(
							&mut sender,
							&mut identity,
							version,
							p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
								p::ToEnvoyWebSocketMessage {
									data: text.into(),
									binary: false,
								},
							),
						)
						.await;
						if let Err(error) = sent {
							return close_after_failed_send(
								socket,
								sender,
								&mut receiver,
								&identity,
								can_hibernate,
								clock.as_ref(),
								error,
							)
							.await;
						}
					}
					Some(Message::Binary(data)) => {
						if can_hibernate {
							credit.sent(identity.message_index, Message::Binary(data.clone()));
						}
						let sent = send(
							&mut sender,
							&mut identity,
							version,
							p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
								p::ToEnvoyWebSocketMessage { data, binary: true },
							),
						)
						.await;
						if let Err(error) = sent {
							return close_after_failed_send(
								socket,
								sender,
								&mut receiver,
								&identity,
								can_hibernate,
								clock.as_ref(),
								error,
							)
							.await;
						}
					}
					Some(Message::Close(close)) => {
						send(
							&mut sender,
							&mut identity,
							version,
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

/// How long a failed send to the actor waits for a close the actor sent before its host ended
/// the stream. The close and the end of the stream travel separately, so the close can arrive
/// just after the send fails; on a real tunnel failure this only delays the error close.
const ACTOR_CLOSE_GRACE_MS: u64 = 250;

/// The client close frame for the actor's close.
fn client_close(close: p::ToRivetWebSocketClose) -> Message {
	Message::Close(close.code.map(|code| CloseFrame {
		code,
		reason: close.reason.unwrap_or_default().to_string().into(),
	}))
}

/// Sends the actor's close to the client and ends the actor side. The actor's host ends its
/// stream once it has forwarded a final close, so a failed finish here only means that peer is
/// already gone; the close reached the client and the tunnel ends normally.
async fn forward_actor_close(
	socket: &mut WebSocket,
	sender: impl TunnelSender,
	close: p::ToRivetWebSocketClose,
) -> Result<()> {
	socket.send(client_close(close)).await?;
	if let Err(error) = sender.finish().await {
		tracing::debug!(%error, "actor WebSocket tunnel peer already closed after the actor's close");
	}
	Ok(())
}

/// A client frame could not be sent to the actor. When the actor closed the WebSocket, its host
/// ends the stream after forwarding that close, and the frame, often one replayed right after a
/// wake, can find the stream ended before the gateway has read the close. Reads what the actor
/// sent, for up to `ACTOR_CLOSE_GRACE_MS`; if it ends with the actor's close, the client gets
/// the actor's messages before it, in order, then that close, and the tunnel ends normally.
/// Otherwise nothing reaches the client and `error` is returned as before. A stop to sleep is
/// not such a close: the frame that failed could not be kept for the wake, so it stays an error.
async fn close_after_failed_send(
	socket: &mut WebSocket,
	sender: impl TunnelSender,
	receiver: &mut impl TunnelReceiver,
	identity: &p::MessageId,
	can_hibernate: bool,
	clock: &dyn Clock,
	error: anyhow::Error,
) -> Result<()> {
	let Ok(now) = clock.boot_ms() else {
		return Err(error);
	};
	let deadline = now.saturating_add(ACTOR_CLOSE_GRACE_MS);
	let mut messages = Vec::new();
	let close = loop {
		let Ok(Ok(Some(bytes))) = rivet_env::timeout_at(clock, deadline, receiver.recv()).await
		else {
			return Err(error);
		};
		match decode_reply(bytes, identity) {
			Ok(p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message)) => {
				let message = if message.binary {
					Message::Binary(message.data)
				} else {
					let Ok(text) = message.data.try_into() else {
						return Err(error);
					};
					Message::Text(text)
				};
				messages.push(message);
			}
			Ok(p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_)) => {}
			Ok(p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close))
				if !(can_hibernate && actor_stopped(&close)) =>
			{
				break close;
			}
			_ => return Err(error),
		}
	};
	tracing::debug!(%error, "send to the actor failed after the actor closed the WebSocket; forwarding its close");
	for message in messages {
		socket.send(message).await?;
	}
	forward_actor_close(socket, sender, close).await
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
/// reach the actor, or for `woken`, which resolves when another request has woken the actor.
/// Returns the frame, or `None` when the actor was woken without one. Pings are answered by the
/// WebSocket layer and the gateway keeps pinging the client, so neither wakes the actor. A
/// client close or disconnect is returned as a close frame so the actor still learns that the
/// connection ended.
async fn hibernate_wait(
	socket: &mut WebSocket,
	keepalive: &mut rivet_env::tokio::time::Interval,
	woken: impl Future<Output = ()>,
) -> Result<Option<Message>> {
	let mut woken = std::pin::pin!(woken);
	loop {
		rivet_env::select_biased!("a client frame wakes the actor and is taken before a wake by another request; keepalive pings only touch the client",
			message = socket.next() => match message {
				Some(Ok(message @ (Message::Text(_) | Message::Binary(_) | Message::Close(_)))) => return Ok(Some(message)),
				// A dropped connection still tells the actor that the client is gone.
				None | Some(Err(_)) => return Ok(Some(Message::Close(None))),
				Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
			},
			_ = &mut woken => return Ok(None),
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
	cause: ReopenCause,
	hibernated: &p::MessageId,
) -> Result<Reopened<S>>
where
	Reopen: Fn(ReopenCause) -> ReopenFut,
	ReopenFut: Future<Output = Result<Reopened<S>>>,
{
	let (stream, mut identity, open) = reopen(cause).await?;
	identity.gateway_id = hibernated.gateway_id;
	identity.request_id = hibernated.request_id;
	Ok((stream, identity, open))
}

async fn send(
	sender: &mut impl TunnelSender,
	identity: &mut p::MessageId,
	version: u16,
	kind: p::ToEnvoyTunnelMessageKind,
) -> Result<()> {
	let bytes = p::versioned::ToEnvoy::serialize_with_embedded_version(
		&p::ToEnvoy::ToEnvoyTunnelMessage(p::ToEnvoyTunnelMessage {
			message_id: identity.clone(),
			message_kind: kind,
		}),
		version,
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
						let result =
							hibernate_wait(&mut socket, &mut keepalive, std::future::pending())
								.await
								.map(|frame| frame.expect("nothing else wakes the actor"));
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

		/// Fails like a UPS `finish` once the actor's host stopped consuming the stream.
		async fn finish(self) -> Result<()> {
			ensure!(!self.0.is_closed(), "fake actor stream closed");
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
			group: None,
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
						let reopen = move |_cause: ReopenCause| {
							let stream = streams.lock().unwrap().pop_front();
							reopens.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
							async move {
								let stream = stream.context("no stream left to reopen")?;
								anyhow::Ok((stream, message_id(9, 9), websocket_open()))
							}
						};
						let clock = Arc::new(TokioClock::new().unwrap());
						let wake = crate::sibling_wake::SiblingWakes::new()
							.subscribe(rivet_util::Id::nil())
							.await;
						let result = proxy(
							&mut socket,
							stream,
							|_: &FakeStream| p::PROTOCOL_VERSION,
							message_id(1, 2),
							websocket_open(),
							reopen,
							wake,
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

	fn websocket_open_at(epoch: u32) -> p::ToEnvoyWebSocketOpen {
		p::ToEnvoyWebSocketOpen {
			actor_generation: Some(epoch),
			..websocket_open()
		}
	}

	/// Runs `proxy` for one client on epoch 1 of `actor_id`. Each reopen takes the next stream of
	/// `reopened` and reaches epoch 2. Returns the client, the result of `proxy`, and the server.
	async fn proxied_client(
		wakes: Arc<crate::sibling_wake::SiblingWakes>,
		actor_id: rivet_util::Id,
		identity: p::MessageId,
		first: FakeStream,
		reopened: Vec<FakeStream>,
	) -> (
		Client,
		oneshot::Receiver<Result<()>>,
		tokio::task::JoinHandle<()>,
	) {
		let reopened = reopened
			.into_iter()
			.map(|stream| Reopening::Running(stream, 2))
			.collect();
		let (client, done, server, _causes) =
			scripted_client(wakes, actor_id, identity, first, reopened).await;
		(client, done, server)
	}

	/// What the actor's state is when a socket re-opens its tunnel.
	enum Reopening {
		/// The actor runs at this epoch: the re-open gets a stream to it.
		Running(FakeStream, u32),
		/// The actor sleeps. A re-open for the socket's client would start it; the test scripts
		/// that as the next `Running`. A re-open after another request's wake leaves it asleep.
		Asleep,
	}

	/// The cause of every re-open a scripted client made, in order.
	type Causes = Arc<std::sync::Mutex<Vec<ReopenCause>>>;

	/// Runs `proxy` for one client on epoch 1 of `actor_id`. Each reopen takes the next step of
	/// `reopened` and records its cause. Asking a sleeping actor for a stream on behalf of the
	/// client is a scripting error, since that re-open would have started the actor.
	async fn scripted_client(
		wakes: Arc<crate::sibling_wake::SiblingWakes>,
		actor_id: rivet_util::Id,
		identity: p::MessageId,
		first: FakeStream,
		reopened: Vec<Reopening>,
	) -> (
		Client,
		oneshot::Receiver<Result<()>>,
		tokio::task::JoinHandle<()>,
		Causes,
	) {
		let causes = Causes::default();
		let recorded = causes.clone();
		let streams = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from(
			reopened,
		)));
		let (done_tx, done_rx) = oneshot::channel();
		let done_tx = Arc::new(std::sync::Mutex::new(Some(done_tx)));
		let first = Arc::new(std::sync::Mutex::new(Some(first)));
		let app = Router::new().route(
			"/",
			get(move |upgrade: WebSocketUpgrade| async move {
				upgrade.on_upgrade(move |mut socket| async move {
					let stream = first.lock().unwrap().take().unwrap();
					let reopen = move |cause: ReopenCause| {
						let step = streams.lock().unwrap().pop_front();
						recorded.lock().unwrap().push(cause);
						async move {
							match step.context("no stream left to reopen")? {
								Reopening::Running(stream, epoch) => {
									anyhow::Ok((stream, message_id(9, 9), websocket_open_at(epoch)))
								}
								Reopening::Asleep => {
									ensure!(
										cause == ReopenCause::Sibling,
										"a re-open for the client would have started the actor"
									);
									Err(ActorNotRunning.into())
								}
							}
						}
					};
					let clock = Arc::new(TokioClock::new().unwrap());
					let wake = wakes.subscribe(actor_id).await;
					let result = proxy(
						&mut socket,
						stream,
						|_: &FakeStream| p::PROTOCOL_VERSION,
						identity,
						websocket_open_at(1),
						reopen,
						wake,
						clock,
						60_000,
					)
					.await;
					if let Some(tx) = done_tx.lock().unwrap().take() {
						let _ = tx.send(result);
					}
				})
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
		(client, done_rx, server, causes)
	}

	/// Whether the gateway sends nothing on `actor`'s stream for a while.
	async fn stays_silent(actor: &mut FakeActor) -> bool {
		tokio::time::timeout(
			std::time::Duration::from_millis(300),
			actor.from_gateway.recv(),
		)
		.await
		.is_err()
	}

	/// Polls until `causes` holds `expected` re-opens.
	async fn reopened(causes: &Causes, expected: usize) {
		tokio::time::timeout(std::time::Duration::from_secs(5), async {
			while causes.lock().unwrap().len() < expected {
				tokio::time::sleep(std::time::Duration::from_millis(5)).await;
			}
		})
		.await
		.expect("the socket did not re-open");
	}

	/// The next frame the client receives that is not a ping or pong.
	async fn next_data(client: &mut Client) -> ClientMessage {
		loop {
			let frame = tokio::time::timeout(std::time::Duration::from_secs(5), client.next())
				.await
				.expect("the client received nothing")
				.unwrap()
				.unwrap();
			if !matches!(frame, ClientMessage::Ping(_) | ClientMessage::Pong(_)) {
				break frame;
			}
		}
	}

	fn ws_text(text: &str) -> p::ToRivetTunnelMessageKind {
		p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(p::ToRivetWebSocketMessage {
			data: text.as_bytes().to_vec().into(),
			binary: false,
		})
	}

	/// Two clients of one actor hibernate. The first client's frame wakes the actor; the second
	/// client sends nothing, yet its tunnel is re-opened under its own ids and it receives what
	/// the woken actor sends, with no frame replayed to the actor on its behalf.
	#[tokio::test]
	async fn a_wake_by_one_client_reattaches_its_hibernated_sibling() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (waker_first, mut waker_asleep) = fake_stream();
		let (waker_second, mut waker_woken) = fake_stream();
		let (sibling_first, mut sibling_asleep) = fake_stream();
		let (sibling_second, mut sibling_woken) = fake_stream();
		let (mut waker, _waker_done, waker_server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 1),
			waker_first,
			vec![waker_second],
		)
		.await;
		let (mut sibling, sibling_done, sibling_server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			sibling_first,
			vec![sibling_second],
		)
		.await;

		// The actor accepts both connections as hibernatable, then goes to sleep.
		for asleep in [&mut waker_asleep, &mut sibling_asleep] {
			let id = asleep.recv_open().await;
			asleep.send(&id, ws_accept());
			asleep.send(&id, ws_close(1012, "actor.stopped"));
		}

		// Only the first client sends a frame.
		waker
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();
		let waker_id = waker_woken.recv_open().await;
		assert_eq!(waker_id.request_id, [1; 4]);
		waker_woken.send(&waker_id, ws_accept());

		// The sibling re-opens under its own hibernated ids without a frame from its client.
		let sibling_id = sibling_woken.recv_open().await;
		assert_eq!(sibling_id.gateway_id, [1; 4]);
		assert_eq!(sibling_id.request_id, [2; 4]);
		sibling_woken.send(&sibling_id, ws_accept());

		// What the woken actor sends reaches the sibling, in order.
		sibling_woken.send(&sibling_id, ws_text("broadcast-1"));
		sibling_woken.send(&sibling_id, ws_text("broadcast-2"));
		for expected in ["broadcast-1", "broadcast-2"] {
			let frame = next_data(&mut sibling).await;
			assert!(
				matches!(&frame, ClientMessage::Text(text) if text.as_str() == expected),
				"unexpected frame {frame:?}"
			);
		}

		// Nothing was replayed for the sibling: the next thing the actor sees from it is the
		// frame its client sends now.
		sibling
			.send(ClientMessage::Text("mine".into()))
			.await
			.unwrap();
		let message = sibling_woken.recv().await;
		let p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(data) = message.message_kind
		else {
			panic!("expected the sibling's own frame");
		};
		assert_eq!(data.data.as_ref(), b"mine");
		sibling_woken.send(
			&message.message_id,
			p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				p::ToRivetWebSocketMessageAck {
					index: message.message_id.message_index,
				},
			),
		);

		sibling.close(None).await.unwrap();
		assert!(matches!(
			sibling_woken.recv().await.message_kind,
			p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_)
		));
		tokio::time::timeout(std::time::Duration::from_secs(5), sibling_done)
			.await
			.unwrap()
			.unwrap()
			.unwrap();
		waker_server.abort();
		sibling_server.abort();
	}

	/// The sibling learns that its actor stopped only after another request already woke the
	/// actor. It must not wait for a wake that has passed: it re-opens at once.
	#[tokio::test]
	async fn a_sibling_that_learns_of_the_stop_after_the_wake_still_reattaches() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (mut client, _done, server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![second],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		// The actor already runs at epoch 2 when this socket's stop of epoch 1 arrives.
		wakes.publish(actor_id, 2).await;
		asleep.send(&id, ws_close(1012, "actor.stopped"));

		let woken_id = woken.recv_open().await;
		assert_eq!(woken_id.request_id, [2; 4]);
		woken.send(&woken_id, ws_accept());
		woken.send(&woken_id, ws_text("broadcast"));
		let frame = next_data(&mut client).await;
		assert!(
			matches!(&frame, ClientMessage::Text(text) if text.as_str() == "broadcast"),
			"unexpected frame {frame:?}"
		);
		server.abort();
	}

	/// An actor that sleeps again stays asleep: a socket re-attached on the latest epoch waits
	/// for its client or for a newer epoch, and does not wake the actor by itself.
	#[tokio::test]
	async fn a_reattached_sibling_does_not_wake_an_actor_that_sleeps_again() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (third, mut woken_again) = fake_stream();
		let (mut client, _done, server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![second, third],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));
		wakes.publish(actor_id, 2).await;
		let woken_id = woken.recv_open().await;
		woken.send(&woken_id, ws_accept());

		// Epoch 2 goes to sleep. No request reached a newer epoch, so the socket waits.
		woken.send(&woken_id, ws_close(1012, "actor.stopped"));
		let reopened = tokio::time::timeout(
			std::time::Duration::from_millis(300),
			woken_again.from_gateway.recv(),
		)
		.await;
		assert!(
			reopened.is_err(),
			"the socket woke a sleeping actor by itself"
		);

		// Its own client still wakes the actor.
		client
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();
		let again_id = woken_again.recv_open().await;
		assert_eq!(again_id.request_id, [2; 4]);
		server.abort();
	}

	/// The actor is asleep again by the time a hibernated socket re-opens after another
	/// request's wake. The socket leaves it asleep and goes back to waiting: it does not start
	/// the actor, does not look again for the epoch it just missed, and still re-attaches on
	/// the next wake.
	#[tokio::test]
	async fn a_sibling_reopen_leaves_an_actor_that_went_back_to_sleep_asleep() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (mut client, _done, server, causes) = scripted_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![Reopening::Asleep, Reopening::Running(second, 3)],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));

		// Another request woke the actor at epoch 2, and the actor slept again before this
		// socket re-opened.
		wakes.publish(actor_id, 2).await;
		reopened(&causes, 1).await;

		// The socket waits. It makes no second attempt for epoch 2, and the record still holds
		// epoch 2, so the failed re-open told no other socket anything.
		tokio::time::sleep(std::time::Duration::from_millis(300)).await;
		assert_eq!(*causes.lock().unwrap(), [ReopenCause::Sibling]);
		assert!(stays_silent(&mut woken).await);
		let observer = wakes.subscribe(actor_id).await;
		assert_eq!(observer.latest(), 2);

		// The next wake by another request re-attaches it.
		wakes.publish(actor_id, 3).await;
		let woken_id = woken.recv_open().await;
		assert_eq!(woken_id.request_id, [2; 4]);
		woken.send(&woken_id, ws_accept());
		woken.send(&woken_id, ws_text("broadcast"));
		let frame = next_data(&mut client).await;
		assert!(
			matches!(&frame, ClientMessage::Text(text) if text.as_str() == "broadcast"),
			"unexpected frame {frame:?}"
		);
		assert_eq!(
			*causes.lock().unwrap(),
			[ReopenCause::Sibling, ReopenCause::Sibling]
		);
		server.abort();
	}

	/// The actor falls asleep while a hibernated socket is re-opening after another request's
	/// wake: the socket found the actor running and got a stream, and the actor's host then
	/// refuses the open because the actor stopped meanwhile. The second attempt must not start
	/// the actor. The socket goes back to waiting with its client still connected, and its own
	/// client's next frame wakes the actor as before.
	#[tokio::test]
	async fn an_actor_that_sleeps_while_its_sibling_reopens_is_not_woken() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut stopping) = fake_stream();
		let (third, mut woken) = fake_stream();
		let (mut client, _done, server, causes) = scripted_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![
				Reopening::Running(second, 2),
				Reopening::Asleep,
				Reopening::Running(third, 3),
			],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));

		// Another request woke the actor at epoch 2. The socket finds it running and opens.
		wakes.publish(actor_id, 2).await;
		let refused_id = stopping.recv_open().await;
		assert_eq!(refused_id.request_id, [2; 4]);

		// The actor stopped before the open arrived, so its host refuses the open before
		// admission. The gateway opens once more, and that attempt finds the actor asleep.
		stopping.send(
			&refused_id,
			ws_close(1011, "envoy.actor_generation_mismatch"),
		);
		reopened(&causes, 2).await;
		assert_eq!(
			*causes.lock().unwrap(),
			[ReopenCause::Sibling, ReopenCause::Sibling],
			"the second attempt of a sibling re-open may not start the actor"
		);

		// Nothing else happens until a client sends a frame: no third attempt, no stream to
		// the actor.
		assert!(stays_silent(&mut woken).await);
		assert_eq!(causes.lock().unwrap().len(), 2);

		// The client was not closed. Its next frame wakes the actor, under the hibernated ids.
		client
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();
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
		assert_eq!(
			*causes.lock().unwrap(),
			[
				ReopenCause::Sibling,
				ReopenCause::Sibling,
				ReopenCause::Client
			]
		);
		server.abort();
	}

	/// Two hibernated sockets of one actor re-open at once after another request's wake, and
	/// the actor is asleep again. Neither starts it, and neither sets the other off: each makes
	/// one attempt and waits. When one client then sends a frame, that socket wakes the actor
	/// and the other re-attaches without starting anything.
	#[tokio::test]
	async fn two_siblings_reopening_at_once_do_not_wake_a_sleeping_actor() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (waker_first, mut waker_asleep) = fake_stream();
		let (waker_second, mut waker_woken) = fake_stream();
		let (sibling_first, mut sibling_asleep) = fake_stream();
		let (sibling_second, mut sibling_woken) = fake_stream();
		let (mut waker, _waker_done, waker_server, waker_causes) = scripted_client(
			wakes.clone(),
			actor_id,
			message_id(1, 1),
			waker_first,
			vec![Reopening::Asleep, Reopening::Running(waker_second, 3)],
		)
		.await;
		let (mut sibling, _sibling_done, sibling_server, sibling_causes) = scripted_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			sibling_first,
			vec![Reopening::Asleep, Reopening::Running(sibling_second, 3)],
		)
		.await;

		for asleep in [&mut waker_asleep, &mut sibling_asleep] {
			let id = asleep.recv_open().await;
			asleep.send(&id, ws_accept());
			asleep.send(&id, ws_close(1012, "actor.stopped"));
		}

		// Another request woke the actor at epoch 2 and it slept again. Both sockets look for
		// it once and neither starts it.
		wakes.publish(actor_id, 2).await;
		reopened(&waker_causes, 1).await;
		reopened(&sibling_causes, 1).await;
		tokio::time::sleep(std::time::Duration::from_millis(300)).await;
		assert_eq!(*waker_causes.lock().unwrap(), [ReopenCause::Sibling]);
		assert_eq!(*sibling_causes.lock().unwrap(), [ReopenCause::Sibling]);
		assert!(stays_silent(&mut waker_woken).await);
		assert!(stays_silent(&mut sibling_woken).await);

		// One client sends a frame. Its socket starts the actor at epoch 3.
		waker
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();
		let waker_id = waker_woken.recv_open().await;
		assert_eq!(waker_id.request_id, [1; 4]);
		waker_woken.send(&waker_id, ws_accept());

		// The other re-attaches to the running actor and receives what it sends.
		let sibling_id = sibling_woken.recv_open().await;
		assert_eq!(sibling_id.request_id, [2; 4]);
		sibling_woken.send(&sibling_id, ws_accept());
		sibling_woken.send(&sibling_id, ws_text("broadcast"));
		let frame = next_data(&mut sibling).await;
		assert!(
			matches!(&frame, ClientMessage::Text(text) if text.as_str() == "broadcast"),
			"unexpected frame {frame:?}"
		);
		assert_eq!(
			*waker_causes.lock().unwrap(),
			[ReopenCause::Sibling, ReopenCause::Client]
		);
		assert_eq!(
			*sibling_causes.lock().unwrap(),
			[ReopenCause::Sibling, ReopenCause::Sibling]
		);
		waker_server.abort();
		sibling_server.abort();
	}

	/// The actor closed a parked connection because it sent more than the connection may hold
	/// while no tunnel was open. The re-open is answered with that close, and the gateway hands
	/// it to the client so the client reconnects.
	#[tokio::test]
	async fn a_parked_overflow_close_reaches_the_client_on_reopen() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (mut client, done, server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![second],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));
		wakes.publish(actor_id, 2).await;

		let woken_id = woken.recv_open().await;
		woken.send(&woken_id, ws_close(1011, "ws.parked_overflow"));
		let frame = next_data(&mut client).await;
		let ClientMessage::Close(Some(close)) = &frame else {
			panic!("unexpected frame {frame:?}");
		};
		assert_eq!(u16::from(close.code), 1011);
		assert_eq!(close.reason.as_str(), "ws.parked_overflow");
		tokio::time::timeout(std::time::Duration::from_secs(5), done)
			.await
			.unwrap()
			.unwrap()
			.unwrap();
		server.abort();
	}

	/// The result `proxy` ended with.
	async fn finished(done: oneshot::Receiver<Result<()>>) -> Result<()> {
		tokio::time::timeout(std::time::Duration::from_secs(5), done)
			.await
			.expect("proxy did not end")
			.unwrap()
	}

	/// The client's frame wakes the actor, which closes its restored connections at once. Its
	/// host forwards the close and ends the stream before the gateway sends the frame queued for
	/// replay, so that send fails. The client still gets the actor's close, not the tunnel's
	/// error close.
	#[tokio::test]
	async fn the_waking_client_gets_the_actors_close_when_the_replay_send_fails() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (mut client, done, server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![second],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));

		client
			.send(ClientMessage::Text("wake".into()))
			.await
			.unwrap();
		let woken_id = woken.recv_open().await;
		// The host stops consuming the stream before the gateway reads the accept, so the
		// replayed frame cannot be sent; the actor's close is already on its way.
		woken.from_gateway.close();
		woken.send(&woken_id, ws_accept());
		woken.send(&woken_id, ws_close(1008, "restored.rejected"));

		let frame = next_data(&mut client).await;
		let ClientMessage::Close(Some(close)) = &frame else {
			panic!("unexpected frame {frame:?}");
		};
		assert_eq!(u16::from(close.code), 1008);
		assert_eq!(close.reason.as_str(), "restored.rejected");
		finished(done).await.unwrap();
		server.abort();
	}

	/// A re-attached sibling receives the actor's close; the host already ended the stream, so
	/// ending the gateway's side fails. The close was delivered and the tunnel ends without an
	/// error.
	#[tokio::test]
	async fn a_sibling_ends_normally_when_finish_fails_after_the_actors_close() {
		let wakes = crate::sibling_wake::SiblingWakes::new();
		let actor_id = rivet_util::Id::nil();
		let (first, mut asleep) = fake_stream();
		let (second, mut woken) = fake_stream();
		let (mut client, done, server) = proxied_client(
			wakes.clone(),
			actor_id,
			message_id(1, 2),
			first,
			vec![second],
		)
		.await;

		let id = asleep.recv_open().await;
		asleep.send(&id, ws_accept());
		asleep.send(&id, ws_close(1012, "actor.stopped"));
		wakes.publish(actor_id, 2).await;

		let woken_id = woken.recv_open().await;
		woken.from_gateway.close();
		woken.send(&woken_id, ws_accept());
		woken.send(&woken_id, ws_close(1008, "restored.rejected"));

		let frame = next_data(&mut client).await;
		let ClientMessage::Close(Some(close)) = &frame else {
			panic!("unexpected frame {frame:?}");
		};
		assert_eq!(u16::from(close.code), 1008);
		assert_eq!(close.reason.as_str(), "restored.rejected");
		finished(done).await.unwrap();
		server.abort();
	}

	/// A send to the actor fails and the actor sent no close: the tunnel still ends with the
	/// error, which the route turns into its error close, and the actor's pending message is
	/// not delivered, as before.
	#[tokio::test]
	async fn a_failed_send_without_an_actor_close_is_still_an_error() {
		let (first, mut actor) = fake_stream();
		let (mut client, done, server) = proxied_client(
			crate::sibling_wake::SiblingWakes::new(),
			rivet_util::Id::nil(),
			message_id(1, 2),
			first,
			vec![],
		)
		.await;

		let id = actor.recv_open().await;
		actor.send(&id, ws_accept());
		actor.send(&id, ws_text("before"));
		let frame = next_data(&mut client).await;
		assert!(
			matches!(&frame, ClientMessage::Text(text) if text.as_str() == "before"),
			"unexpected frame {frame:?}"
		);
		actor.from_gateway.close();
		actor.send(&id, ws_text("pending"));
		client
			.send(ClientMessage::Text("lost".into()))
			.await
			.unwrap();

		let error = finished(done).await.unwrap_err();
		assert!(
			error.to_string().contains("fake actor stream closed"),
			"unexpected error {error:#}"
		);
		server.abort();
	}
}
