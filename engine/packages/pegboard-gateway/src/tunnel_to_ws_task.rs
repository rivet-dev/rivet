use std::collections::VecDeque;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};

use anyhow::Result;
use gas::prelude::*;
use rivet_guard_core::{
	WebSocketHandle,
	errors::{WebSocketServiceHibernate, WebSocketServiceTimeout, WebSocketServiceUnavailable},
};
use rivet_runner_protocol as protocol;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

use super::LifecycleResult;
use crate::shared_state::{MsgGcReason, SharedState};

pub async fn task(
	ctx: StandaloneCtx,
	shared_state: SharedState,
	client_ws: WebSocketHandle,
	request_id: protocol::RequestId,
	actor_id: Id,
	runner_id: Id,
	mut stopped_sub: message::SubscriptionHandle<pegboard::workflows::actor::Stopped>,
	mut msg_rx: mpsc::Receiver<protocol::mk2::ToServerTunnelMessageKind>,
	pending_msgs: Vec<protocol::mk2::ToServerTunnelMessageKind>,
	mut drop_rx: watch::Receiver<Option<MsgGcReason>>,
	can_hibernate: bool,
	egress_bytes: Arc<AtomicU64>,
	mut tunnel_to_ws_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	// Drain any runner messages buffered during the open handshake before consuming new ones.
	let mut pending_msgs = VecDeque::from(pending_msgs);

	loop {
		if let Some(msg) = pending_msgs.pop_front() {
			if let Some(result) = handle_runner_message(
				&shared_state,
				&client_ws,
				request_id,
				msg,
				can_hibernate,
				&egress_bytes,
			)
			.await?
			{
				return Ok(result);
			}
			continue;
		}

		tokio::select! {
			res = msg_rx.recv() => {
				if let Some(msg) = res {
					if let Some(result) = handle_runner_message(
						&shared_state,
						&client_ws,
						request_id,
						msg,
						can_hibernate,
						&egress_bytes,
					)
					.await?
					{
						return Ok(result);
					}
				} else {
					tracing::debug!("tunnel sub closed");
					return Err(WebSocketServiceHibernate.build());
				}
			}
			_ = stopped_sub.next() => {
				let actor_state = ctx
					.op(pegboard::ops::actor::get_for_gateway::Input { actor_id })
					.await?;

				let stale_stopped_message = actor_state.as_ref().is_some_and(|actor| {
					actor.connectable && actor.runner_id == Some(runner_id)
				});
				// The actor may already be reconnected on this runner, so ignore old stop events in that case.
				if stale_stopped_message {
					tracing::debug!(
						?actor_id,
						?runner_id,
						"ignoring stale actor stopped message during websocket handler loop"
					);
					continue;
				}

				tracing::debug!("actor stopped during websocket handler loop");

				if can_hibernate {
					return Err(WebSocketServiceHibernate.build());
				} else {
					return Err(WebSocketServiceUnavailable.build());
				}
			}
			_ = drop_rx.changed() => {
				tracing::warn!(reason=?drop_rx.borrow().as_ref(), "garbage collected");
				return Err(WebSocketServiceTimeout.build());
			}
			_ = tunnel_to_ws_abort_rx.changed() => {
				tracing::debug!("task aborted");
				return Ok(LifecycleResult::Aborted);
			}
		}
	}
}

async fn handle_runner_message(
	shared_state: &SharedState,
	client_ws: &WebSocketHandle,
	request_id: protocol::RequestId,
	msg: protocol::mk2::ToServerTunnelMessageKind,
	can_hibernate: bool,
	egress_bytes: &Arc<AtomicU64>,
) -> Result<Option<LifecycleResult>> {
	match msg {
		protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketMessage(ws_msg) => {
			tracing::trace!(
				request_id=%protocol::util::id_to_string(&request_id),
				data_len=ws_msg.data.len(),
				binary=ws_msg.binary,
				"forwarding websocket message to client"
			);
			let msg = if ws_msg.binary {
				Message::Binary(ws_msg.data.into())
			} else {
				Message::Text(String::from_utf8_lossy(&ws_msg.data).into_owned().into())
			};

			egress_bytes.fetch_add(msg.len() as u64, Ordering::AcqRel);
			client_ws.send(msg).await?;
			Ok(None)
		}
		protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(ack) => {
			tracing::trace!(
				request_id=%protocol::util::id_to_string(&request_id),
				ack_index=?ack.index,
				"received WebSocketMessageAck from runner"
			);
			shared_state
				.ack_pending_websocket_messages(request_id, ack.index)
				.await?;
			Ok(None)
		}
		protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
			tracing::debug!(?close, "server closed websocket");

			if can_hibernate && close.hibernate {
				Err(WebSocketServiceHibernate.build())
			} else {
				Ok(Some(LifecycleResult::ServerClose(close)))
			}
		}
		_ => Ok(None),
	}
}
