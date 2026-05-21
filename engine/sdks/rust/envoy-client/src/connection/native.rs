use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures_util::{SinkExt, StreamExt};
use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;
use tracing::Instrument;
use vbare::OwnedVersionedData;

use crate::context::{SharedContext, WsTxMessage};
use crate::envoy::ToEnvoyMessage;
use crate::handle::EnvoyHandle;
use crate::utils::{BackoffOptions, calculate_backoff, parse_ws_close_reason};

const STABLE_CONNECTION_MS: u64 = 60_000;

pub fn start_connection(shared: Arc<SharedContext>) {
	let span = tracing::debug_span!("envoy_connection", envoy_key = %shared.envoy_key);
	tokio::spawn(connection_loop(shared).instrument(span));
}

async fn connection_loop(shared: Arc<SharedContext>) {
	let mut attempt = 0u32;

	loop {
		if shared.shutting_down.load(Ordering::Acquire) {
			tracing::debug!("stopping reconnect loop because envoy is shutting down");
			return;
		}

		let connected_at = std::time::Instant::now();

		match single_connection(&shared).await {
			Ok(close_reason) => {
				if let Some(reason) = &close_reason {
					if reason.group == "ws" && reason.error == "eviction" {
						tracing::debug!("connection evicted");
						let _ = shared
							.envoy_tx
							.send(ToEnvoyMessage::ConnClose { evict: true });
						return;
					}
				}
				let _ = shared
					.envoy_tx
					.send(ToEnvoyMessage::ConnClose { evict: false });
			}
			Err(error) => {
				tracing::error!(?error, "connection failed");
				let _ = shared
					.envoy_tx
					.send(ToEnvoyMessage::ConnClose { evict: false });
			}
		}

		if connected_at.elapsed().as_millis() >= STABLE_CONNECTION_MS as u128 {
			attempt = 0;
		}

		if shared.shutting_down.load(Ordering::Acquire) {
			tracing::debug!("skipping reconnect because envoy is shutting down");
			return;
		}

		let delay = calculate_backoff(attempt, &BackoffOptions::default());
		tracing::info!(attempt, delay_ms = delay.as_millis() as u64, "reconnecting");
		tokio::time::sleep(delay).await;
		attempt += 1;
	}
}

async fn single_connection(
	shared: &Arc<SharedContext>,
) -> anyhow::Result<Option<crate::utils::ParsedCloseReason>> {
	let url = super::ws_url(shared);
	let protocols = {
		let mut p = vec!["rivet".to_string()];
		if let Some(token) = &shared.config.token {
			p.push(format!("rivet_token.{token}"));
		}
		p
	};

	// Initialize with a default CryptoProvider for rustls
	let provider = rustls::crypto::ring::default_provider();
	if provider.install_default().is_err() {
		tracing::debug!("crypto provider already installed in this process");
	}

	let request = tungstenite::http::Request::builder()
		.uri(&url)
		.header("Sec-WebSocket-Protocol", protocols.join(", "))
		.header("Connection", "Upgrade")
		.header("Upgrade", "websocket")
		.header(
			"Sec-WebSocket-Key",
			tungstenite::handshake::client::generate_key(),
		)
		.header("Sec-WebSocket-Version", "13")
		.header("Host", extract_host(&url))
		.body(())
		.map_err(|e| anyhow::anyhow!("failed to build ws request: {e}"))?;

	let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;
	let (mut write, mut read) = ws_stream.split();

	let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<WsTxMessage>();
	{
		let mut guard = shared.ws_tx.lock().await;
		*guard = Some(ws_tx);
	}

	tracing::info!(
		endpoint = %shared.config.endpoint,
		namespace = %shared.config.namespace,
		envoy_key = %shared.envoy_key,
		has_token = shared.config.token.is_some(),
		"websocket connected"
	);
	shared
		.config
		.callbacks
		.on_connect(EnvoyHandle::from_shared(shared.clone()));

	// Spawn write task
	let shared2 = shared.clone();
	let write_span = tracing::debug_span!("envoy_ws_write", envoy_key = %shared2.envoy_key);
	let write_handle = tokio::spawn(
		async move {
			super::send_initial_metadata(&shared2).await;

			while let Some(msg) = ws_rx.recv().await {
				match msg {
					WsTxMessage::Send(data) => {
						let result = write
							.send(tungstenite::Message::Binary(data.into()))
							.await;
						if let Err(e) = result {
							tracing::error!(?e, "failed to send ws message");
							break;
						}
					}
					WsTxMessage::Close => {
						let _ = write
							.send(tungstenite::Message::Close(Some(
								tungstenite::protocol::CloseFrame {
									code: tungstenite::protocol::frame::coding::CloseCode::Normal,
									reason: "envoy.shutdown".into(),
								},
							)))
							.await;
						break;
					}
				}
			}
		}
		.instrument(write_span),
	);

	let mut result = None;

	let debug_latency_ms = shared.config.debug_latency_ms;

	while let Some(msg) = read.next().await {
		match msg {
			Ok(tungstenite::Message::Binary(data)) => {
				crate::utils::inject_latency(debug_latency_ms).await;

				let decoded = crate::protocol::versioned::ToEnvoy::deserialize(
					&data,
					protocol::PROTOCOL_VERSION,
				)?;

				super::forward_to_envoy(shared, decoded).await;
			}
			Ok(tungstenite::Message::Close(frame)) => {
				if let Some(frame) = frame {
					let reason_str = frame.reason.to_string();
					let code: u16 = frame.code.into();
					tracing::info!(
						code,
						reason = %reason_str,
						"websocket closed"
					);
					result = parse_ws_close_reason(&reason_str);
				}
				break;
			}
			Err(e) => {
				tracing::error!(?e, "websocket error");
				break;
			}
			_ => {}
		}
	}

	// Clean up
	{
		let mut guard = shared.ws_tx.lock().await;
		*guard = None;
	}
	write_handle.abort();
	shared
		.config
		.callbacks
		.on_disconnect(EnvoyHandle::from_shared(shared.clone()));

	Ok(result)
}

fn extract_host(url: &str) -> String {
	url.replace("ws://", "")
		.replace("wss://", "")
		.split('/')
		.next()
		.unwrap_or("localhost")
		.to_string()
}
