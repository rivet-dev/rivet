use anyhow::Result;
use gas::prelude::*;
use pegboard::tunnel::id as tunnel_id;
use rivet_guard_core::{
	WebSocketHandle,
	errors::{WebSocketServiceHibernate, WebSocketServiceTimeout, WebSocketServiceUnavailable},
};
use rivet_runner_protocol as protocol;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

use super::LifecycleResult;
use crate::shared_state::SharedState;

pub async fn task(
	shared_state: SharedState,
	client_ws: WebSocketHandle,
	request_id: protocol::RequestId,
	mut stopped_sub: message::SubscriptionHandle<pegboard::workflows::actor::Stopped>,
	mut msg_rx: mpsc::Receiver<protocol::ToServerTunnelMessageKind>,
	mut drop_rx: watch::Receiver<()>,
	can_hibernate: bool,
	mut tunnel_to_ws_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		tokio::select! {
			res = msg_rx.recv() => {
				if let Some(msg) = res {
					match msg {
						protocol::ToServerTunnelMessageKind::ToServerWebSocketMessage(ws_msg) => {
							let msg = if ws_msg.binary {
								Message::Binary(ws_msg.data.into())
							} else {
								Message::Text(
									String::from_utf8_lossy(&ws_msg.data).into_owned().into(),
								)
							};
							client_ws.send(msg).await?;
						}
						protocol::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(ack) => {
							tracing::debug!(
								request_id=%tunnel_id::request_id_to_string(&request_id),
								ack_index=?ack.index,
								"received WebSocketMessageAck from runner"
							);
							shared_state
								.ack_pending_websocket_messages(request_id, ack.index)
								.await?;
						}
						protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
							tracing::debug!(?close, "server closed websocket");

							if can_hibernate && close.hibernate {
								return Err(WebSocketServiceHibernate.build());
							} else {
								// Successful closure
								return Ok(LifecycleResult::ServerClose(close));
							}
						}
						_ => {}
					}
				} else {
					tracing::debug!("tunnel sub closed");
					return Err(WebSocketServiceHibernate.build());
				}
			}
			_ = stopped_sub.next() => {
				tracing::debug!("actor stopped during websocket handler loop");

				if can_hibernate {
					return Err(WebSocketServiceHibernate.build());
				} else {
					return Err(WebSocketServiceUnavailable.build());
				}
			}
			_ = drop_rx.changed() => {
				tracing::warn!("websocket message timeout");
				return Err(WebSocketServiceTimeout.build());
			}
			_ = tunnel_to_ws_abort_rx.changed() => {
				tracing::debug!("task aborted");
				return Ok(LifecycleResult::Aborted);
			}
		}
	}
}
