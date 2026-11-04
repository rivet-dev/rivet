//! Bounded WebSocket forwarding tied to one admitted actor generation and UPS stream.
use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocket};
use rivet_env::{Clock, ClockError};
use rivet_envoy_protocol as p;
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
	sender: &mut PacketSender,
	receiver: &mut PacketReceiver,
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
pub(crate) struct Tunnel {
	pub(crate) sender: PacketSender,
	pub(crate) receiver: PacketReceiver,
	pub(crate) identity: p::MessageId,
	pub(crate) reply: OpenReply,
}

/// Opens the actor side of a WebSocket. An open the actor's host refuses before admission, as
/// when a pipelined open arrives ahead of the actor's start, is sent once more on the stream
/// `reopen` acquires after the actor runs; a second refusal is the answer. Ordered UPS lanes do
/// not prevent the first refusal, because the start reaches the Envoy from the pegboard executor
/// while the open travels on this gateway's stream.
pub(crate) async fn open_tunnel<Reopen, ReopenFut>(
	stream: DuplexStream,
	mut identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
) -> Result<Tunnel>
where
	Reopen: FnOnce() -> ReopenFut,
	ReopenFut:
		std::future::Future<Output = Result<(DuplexStream, p::MessageId, p::ToEnvoyWebSocketOpen)>>,
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
pub async fn proxy<Reopen, ReopenFut>(
	socket: &mut WebSocket,
	stream: DuplexStream,
	identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
	clock: Arc<dyn Clock>,
	keepalive_interval_ms: u64,
) -> Result<()>
where
	Reopen: Fn() -> ReopenFut,
	ReopenFut:
		std::future::Future<Output = Result<(DuplexStream, p::MessageId, p::ToEnvoyWebSocketOpen)>>,
{
	let Tunnel {
		mut sender,
		mut receiver,
		mut identity,
		reply,
	} = open_tunnel(stream, identity, open, &reopen).await?;
	let can_hibernate = match reply {
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
	let mut pending = None;

	// The first tick completes at once and is skipped; the socket was just opened. Missed ticks
	// are skipped too, so a connection that was busy for a while sends one ping, not a burst.
	let mut keepalive = rivet_env::interval(clock, keepalive_interval_ms);
	keepalive.tick().await.context("keepalive clock failed")?;

	// Client frames to deliver first after a wake, and the last frame
	// sent to the actor that it has not acknowledged yet.
	let mut replay: std::collections::VecDeque<Message> = std::collections::VecDeque::new();
	let mut unacked: Option<Message> = None;

	loop {
		let input = if let Some(message) = replay.pop_front() {
			Input::Client(Some(Ok(message)))
		} else {
			rivet_env::select_biased!("actor replies release WebSocket send credit before more client input, and a keepalive ping never preempts either",
			packet = receiver.recv() => Input::Actor(packet),
			message = async {
				if pending.is_none() {
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
							pending == Some(ack.index),
							"unexpected WebSocket acknowledgement"
						);
						pending = None;
						unacked = None;
					}
					p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close)
						if can_hibernate
							&& close.code == Some(1012)
							&& close.reason.as_deref() == Some("actor.stopped") =>
					{
						// The actor went to sleep. Keep the client
						// socket, answer pings ourselves and wake the actor on the next client
						// frame, re-opening the tunnel with the same gateway/request ids so
						// the actor restores the hibernated connection.
						if let Err(error) = sender.finish().await {
							tracing::debug!(%error, "stopped WebSocket tunnel peer already closed");
						}
						if let Some(message) = unacked.take() {
							replay.push_back(message);
						}
						pending = None;
						tracing::debug!(
							"actor stopped; client WebSocket hibernating at the gateway"
						);
						let first = hibernate_wait(socket, &mut keepalive).await?;
						replay.push_back(first);
						let (new_stream, mut new_identity, new_open) = reopen().await?;
						new_identity.gateway_id = identity.gateway_id;
						new_identity.request_id = identity.request_id;
						let (mut new_sender, mut new_receiver) = new_stream.split();
						let reply = open_actor(
							&mut new_sender,
							&mut new_receiver,
							&mut new_identity,
							new_open,
						)
						.await?;
						match reply {
							OpenReply::Accepted { .. } => {}
							OpenReply::Closed(close) => {
								socket
									.send(Message::Close(close.code.map(|code| CloseFrame {
										code,
										reason: close.reason.unwrap_or_default().to_string().into(),
									})))
									.await?;
								new_sender.finish().await?;
								return Ok(());
							}
						}
						sender = new_sender;
						receiver = new_receiver;
						identity = new_identity;
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
							pending = Some(identity.message_index);
							unacked = Some(Message::Text(text.clone()));
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
							pending = Some(identity.message_index);
							unacked = Some(Message::Binary(data.clone()));
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
				let sent = socket.send(Message::Ping(bytes::Bytes::new())).await;
				let result = if sent.is_ok() { "ok" } else { "error" };
				crate::metrics::WEBSOCKET_KEEPALIVE_PING_TOTAL
					.with_label_values(&[result])
					.inc();
				sent.context("keepalive ping to the client failed")?;
			}
		}
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
				socket.send(Message::Ping(bytes::Bytes::new())).await.context("keepalive ping to the client failed")?;
			},
		);
	}
}

async fn send(
	sender: &mut PacketSender,
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
	sender.send(bytes).await?;
	identity.message_index = identity.message_index.wrapping_add(1);
	Ok(())
}

#[cfg(test)]
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
}
