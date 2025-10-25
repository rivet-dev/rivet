use anyhow::Result;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use rivet_runner_protocol::{self as protocol, versioned};
use std::sync::Arc;
use universalpubsub::{NextOutput, Subscriber};
use vbare::OwnedVersionedData;

use crate::{
	conn::{Conn, TunnelActiveRequest},
	errors,
};

#[tracing::instrument(skip_all, fields(runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(conn: Arc<Conn>, mut sub: Subscriber) -> Result<()> {
	while let NextOutput::Message(ups_msg) = sub
		.next()
		.await
		.context("pubsub_to_client_task sub failed")?
	{
		tracing::debug!(
			payload_len = ups_msg.payload.len(),
			"received message from pubsub, forwarding to WebSocket"
		);

		// Parse message
		let mut msg = match versioned::ToClient::deserialize_with_embedded_version(&ups_msg.payload)
		{
			Result::Ok(x) => x,
			Err(err) => {
				tracing::error!(?err, "failed to parse tunnel message");
				continue;
			}
		};

		match &mut msg {
			protocol::ToClient::ToClientClose => return Err(errors::WsError::Eviction.build()),
			// Handle tunnel messages
			protocol::ToClient::ToClientTunnelMessage(tunnel_msg) => {
				// Save active request
				//
				// This will remove gateway_reply_to from the message since it does not need to be sent to the
				// client
				if let Some(reply_to) = tunnel_msg.gateway_reply_to.take() {
					tracing::debug!(request_id=?Uuid::from_bytes(tunnel_msg.request_id), ?reply_to, "creating active request");
					let mut active_requests = conn.tunnel_active_requests.lock().await;
					active_requests.insert(
						tunnel_msg.request_id,
						TunnelActiveRequest {
							gateway_reply_to: reply_to,
						},
					);
				}

				match tunnel_msg.message_kind {
					// If terminal, remove active request tracking
					protocol::ToClientTunnelMessageKind::ToClientWebSocketClose(_) => {
						tracing::debug!(request_id=?Uuid::from_bytes(tunnel_msg.request_id), "removing active conn due to close message");
						let mut active_requests = conn.tunnel_active_requests.lock().await;
						active_requests.remove(&tunnel_msg.request_id);
					}
					_ => {}
				}
			}
			_ => {}
		}

		// Forward raw message to WebSocket
		let serialized_msg =
			match versioned::ToClient::latest(msg).serialize_version(conn.protocol_version) {
				Result::Ok(x) => x,
				Err(err) => {
					tracing::error!(?err, "failed to serialize tunnel message");
					continue;
				}
			};
		let ws_msg = WsMessage::Binary(serialized_msg.into());
		conn.ws_handle
			.send(ws_msg)
			.await
			.context("failed to send message to WebSocket")?
	}

	Ok(())
}
