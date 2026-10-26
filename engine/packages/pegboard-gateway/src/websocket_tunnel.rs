//! Bounded WebSocket forwarding tied to one admitted actor generation and UPS stream.
use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use rivet_env::capabilities::websocket::{CloseFrame, Message, WebSocket};
use rivet_envoy_protocol as p;
use universalpubsub::{DuplexStream, PacketReceiver, PacketSender};
use vbare::Versioned;

/// Whichever peer has made progress; the loop owns both directions' ordering.
enum Input {
	Actor(Result<Option<bytes::Bytes>>),
	Client(Option<Result<Message, axum::Error>>),
}

/// How the actor answered a WebSocket open.
pub(crate) enum OpenReply {
	Accepted { can_hibernate: bool },
	Closed(p::ToRivetWebSocketClose),
}

/// Whether the actor's host refused the open before the actor admitted it, so the same open
/// may succeed against the actor once it runs.
pub(crate) fn refused_before_admission(close: &p::ToRivetWebSocketClose) -> bool {
	close.reason.as_deref().is_some_and(|reason| {
		matches!(
			reason,
			"envoy.actor_not_found"
				| "envoy.actor_generation_mismatch"
				| "runner.actor_not_found"
				| "runner.actor_generation_mismatch"
		)
	})
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
pub async fn proxy<Reopen, ReopenFut>(
	socket: &mut WebSocket,
	stream: DuplexStream,
	identity: p::MessageId,
	open: p::ToEnvoyWebSocketOpen,
	reopen: Reopen,
) -> Result<()>
where
	Reopen: FnOnce() -> ReopenFut,
	ReopenFut:
		std::future::Future<Output = Result<(DuplexStream, p::MessageId, p::ToEnvoyWebSocketOpen)>>,
{
	let Tunnel {
		mut sender,
		mut receiver,
		mut identity,
		reply,
	} = open_tunnel(stream, identity, open, reopen).await?;
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

	loop {
		let input = rivet_env::select_biased!("actor replies release WebSocket send credit before more client input",
			packet = receiver.recv() => Input::Actor(packet),
			message = async {
				if pending.is_none() {
					socket.next().await
				} else {
					futures_util::future::pending().await
				}
			} => Input::Client(message),
		);

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
		}
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
	identity.message_index = identity
		.message_index
		.checked_add(1)
		.context("WebSocket message index overflow")?;
	Ok(())
}


