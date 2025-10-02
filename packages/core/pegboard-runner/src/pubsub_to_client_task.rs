use anyhow::Result;
use futures_util::SinkExt;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use rivet_runner_protocol::{self as protocol, versioned};
use std::sync::Arc;
use universalpubsub::{NextOutput, Subscriber};
use vbare::OwnedVersionedData;

use crate::{
	conn::{Conn, TunnelActiveRequest},
	utils,
};

#[tracing::instrument(skip_all, fields(runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(ctx: StandaloneCtx, conn: Arc<Conn>, sub: Subscriber) {
	match task_inner(ctx, conn, sub).await {
		Ok(_) => {}
		Err(err) => {
			tracing::error!(?err, "pubsub to client error");
		}
	}
}

#[tracing::instrument(skip_all)]
async fn task_inner(ctx: StandaloneCtx, conn: Arc<Conn>, mut sub: Subscriber) -> Result<()> {
	while let Result::Ok(NextOutput::Message(ups_msg)) = sub.next().await {
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
		let is_close = utils::is_to_client_close(&msg);

		// Handle tunnel messages
		if let protocol::ToClient::ToClientTunnelMessage(tunnel_msg) = &mut msg {
			handle_tunnel_message(&conn, tunnel_msg).await;
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
		if let Err(e) = conn.ws_handle.send(ws_msg).await {
			tracing::error!(?e, "failed to send message to WebSocket");
			break;
		}

		if is_close {
			tracing::debug!("manually closing websocket");
			break;
		}
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_tunnel_message(conn: &Arc<Conn>, msg: &mut protocol::ToClientTunnelMessage) {
	// Save active request
	//
	// This will remove gateway_reply_to from the message since it does not need to be sent to the
	// client
	if let Some(reply_to) = msg.gateway_reply_to.take() {
		tracing::debug!(?msg.request_id, ?reply_to, "creating active request");
		let mut active_requests = conn.tunnel_active_requests.lock().await;
		active_requests.insert(
			msg.request_id,
			TunnelActiveRequest {
				gateway_reply_to: reply_to,
			},
		);
	}

	// If terminal, remove active request tracking
	if utils::is_to_client_tunnel_message_kind_request_close(&msg.message_kind) {
		tracing::debug!(?msg.request_id, "removing active conn from close message");
		let mut active_requests = conn.tunnel_active_requests.lock().await;
		active_requests.remove(&msg.request_id);
	}
}
