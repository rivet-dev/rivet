use anyhow::Result;
use futures_util::TryStreamExt;
use rivet_envoy_protocol as protocol;
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite::Message;

use super::LifecycleResult;
use crate::shared_state::InFlightRequestHandle;

#[tracing::instrument(skip_all)]
pub async fn task(
	in_flight_req: InFlightRequestHandle,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	ingress_bytes: Arc<AtomicU64>,
	mut ws_to_tunnel_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut ws_rx = ws_rx.lock().await;

	loop {
		tokio::select! {
			res = ws_rx.try_next() => {
				if let Some(msg) = res? {
					ingress_bytes.fetch_add(msg.len() as u64, Ordering::AcqRel);

					match msg {
						Message::Binary(data) => {
							let ws_message =
								protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
									protocol::ToEnvoyWebSocketMessage {
										data: data.into(),
										binary: true,
									},
								);
							in_flight_req.send_message(ws_message).await?;
						}
						Message::Text(text) => {
							let ws_message =
								protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
									protocol::ToEnvoyWebSocketMessage {
										data: text.as_bytes().to_vec(),
										binary: false,
									},
								);
							in_flight_req.send_message(ws_message).await?;
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
