use anyhow::Result;
use futures_util::TryStreamExt;
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use rivet_runner_protocol as protocol;
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite::Message;

use super::LifecycleResult;
use crate::shared_state::SharedState;

pub async fn task(
	shared_state: SharedState,
	request_id: protocol::RequestId,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	mut ws_to_tunnel_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut ws_rx = ws_rx.lock().await;

	loop {
		tokio::select! {
			res = ws_rx.try_next() => {
				if let Some(msg) = res? {
					match msg {
						Message::Binary(data) => {
							let ws_message =
								protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(
									protocol::ToClientWebSocketMessage {
										data: data.into(),
										binary: true,
									},
								);
							shared_state
								.send_message(request_id, ws_message)
								.await?;
						}
						Message::Text(text) => {
							let ws_message =
								protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(
									protocol::ToClientWebSocketMessage {
										data: text.as_bytes().to_vec(),
										binary: false,
									},
								);
							shared_state
								.send_message(request_id, ws_message)
								.await?;
						}
						Message::Close(close) => {
							return Ok(LifecycleResult::ClientClose(close));
						}
						_ => {}
					}
				} else {
					tracing::debug!("websocket stream closed");
					return Ok(LifecycleResult::ClientClose(None));
				}
			}
			_ = ws_to_tunnel_abort_rx.changed() => {
				tracing::debug!("task aborted");
				return Ok(LifecycleResult::Aborted);
			}
		};
	}
}
