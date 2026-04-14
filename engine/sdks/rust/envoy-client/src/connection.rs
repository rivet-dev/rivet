use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures_util::{SinkExt, StreamExt};
use rivet_envoy_protocol as protocol;
use rivet_util_serde::HashableMap;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;
use vbare::OwnedVersionedData;

use crate::context::{SharedContext, WsTxMessage};
use crate::envoy::ToEnvoyMessage;
use crate::stringify::{stringify_to_envoy, stringify_to_rivet};
use crate::utils::{BackoffOptions, calculate_backoff, parse_ws_close_reason};

const STABLE_CONNECTION_MS: u64 = 60_000;

pub fn start_connection(shared: Arc<SharedContext>) {
	tokio::spawn(connection_loop(shared));
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
	let url = ws_url(shared);
	let protocols = {
		let mut p = vec!["rivet".to_string(), "rivet_target.envoy".to_string()];
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

	// Spawn write task
	let shared2 = shared.clone();
	let write_handle = tokio::spawn(async move {
		// Build prepopulate actor names map
		let mut prepopulate_map = HashableMap::new();
		for (name, actor) in &shared2.config.prepopulate_actor_names {
			prepopulate_map.insert(
				name.clone(),
				protocol::ActorName {
					metadata: actor.metadata.clone(),
				},
			);
		}

		// Serialize metadata HashMap to JSON string for the protocol
		let metadata_json = shared2
			.config
			.metadata
			.as_ref()
			.map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));

		// Send metadata
		ws_send(
			&shared2,
			protocol::ToRivet::ToRivetMetadata(protocol::ToRivetMetadata {
				prepopulate_actor_names: Some(prepopulate_map),
				metadata: metadata_json,
			}),
		)
		.await;

		while let Some(msg) = ws_rx.recv().await {
			match msg {
				WsTxMessage::Send(data) => {
					if let Err(e) = write.send(tungstenite::Message::Binary(data.into())).await {
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
	});

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

				if tracing::enabled!(tracing::Level::DEBUG) {
					tracing::debug!(data = stringify_to_envoy(&decoded), "received message");
				}

				forward_to_envoy(shared, decoded).await;
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

	Ok(result)
}

async fn forward_to_envoy(shared: &SharedContext, message: protocol::ToEnvoy) {
	match message {
		protocol::ToEnvoy::ToEnvoyPing(ping) => {
			ws_send(
				shared,
				protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: ping.ts }),
			)
			.await;
		}
		other => {
			let _ = shared
				.envoy_tx
				.send(ToEnvoyMessage::ConnMessage { message: other });
		}
	}
}

/// Send a message over the WebSocket. Returns true if the message could not be sent.
pub async fn ws_send(shared: &SharedContext, message: protocol::ToRivet) -> bool {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_rivet(&message), "sending message");
	}

	let guard = shared.ws_tx.lock().await;
	let Some(tx) = guard.as_ref() else {
		tracing::error!("websocket not available for sending");
		return true;
	};

	let encoded = crate::protocol::versioned::ToRivet::wrap_latest(message)
		.serialize(protocol::PROTOCOL_VERSION)
		.expect("failed to encode message");
	let _ = tx.send(WsTxMessage::Send(encoded));
	false
}

fn ws_url(shared: &SharedContext) -> String {
	let ws_endpoint = shared
		.config
		.endpoint
		.replace("http://", "ws://")
		.replace("https://", "wss://");
	let base_url = ws_endpoint.trim_end_matches('/');

	format!(
		"{}/envoys/connect?protocol_version={}&namespace={}&envoy_key={}&version={}&pool_name={}",
		base_url,
		protocol::PROTOCOL_VERSION,
		urlencoding::encode(&shared.config.namespace),
		urlencoding::encode(&shared.envoy_key),
		urlencoding::encode(&shared.config.version.to_string()),
		urlencoding::encode(&shared.config.pool_name),
	)
}

fn extract_host(url: &str) -> String {
	url.replace("ws://", "")
		.replace("wss://", "")
		.split('/')
		.next()
		.unwrap_or("localhost")
		.to_string()
}
