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
use crate::utils::{BackoffOptions, ParsedCloseReason, calculate_backoff, parse_ws_close_reason};

const STABLE_CONNECTION_MS: u64 = 60_000;

enum SingleConnectionResult {
	Closed(Option<ParsedCloseReason>),
	RetryLowerProtocol {
		from: u16,
		to: u16,
		reason: &'static str,
	},
}

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
			Ok(SingleConnectionResult::Closed(close_reason)) => {
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
			Ok(SingleConnectionResult::RetryLowerProtocol { from, to, reason }) => {
				tracing::warn!(
					from_protocol_version = from,
					to_protocol_version = to,
					reason,
					"retrying envoy connection with lower protocol version"
				);
				attempt = 0;
				continue;
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

async fn single_connection(shared: &Arc<SharedContext>) -> anyhow::Result<SingleConnectionResult> {
	let protocol_version = current_protocol_version(shared);
	let url = ws_url(shared, protocol_version);
	let protocols = {
		let mut p = vec!["rivet".to_string()];
		if let Some(token) = &shared.config.token {
			p.push(format!("rivet_token.{token}"));
		}
		p
	};

	// Initialize with a default CryptoProvider for rustls.
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
		protocol_version,
		"websocket connected"
	);

	let shared2 = shared.clone();
	let write_handle = tokio::spawn(async move {
		let mut prepopulate_map = HashableMap::new();
		for (name, actor) in &shared2.config.prepopulate_actor_names {
			prepopulate_map.insert(
				name.clone(),
				protocol::ActorName {
					metadata: actor.metadata.clone(),
				},
			);
		}

		let metadata_json = shared2
			.config
			.metadata
			.as_ref()
			.map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));

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

	let mut received_init = false;
	let mut result = SingleConnectionResult::Closed(None);
	let debug_latency_ms = shared.config.debug_latency_ms;

	while let Some(msg) = read.next().await {
		match msg {
			Ok(tungstenite::Message::Binary(data)) => {
				crate::utils::inject_latency(debug_latency_ms).await;

				match crate::protocol::versioned::ToEnvoy::deserialize(&data, protocol_version) {
					Ok(decoded) => {
						if matches!(decoded, protocol::ToEnvoy::ToEnvoyInit(_)) {
							received_init = true;
						}

						if tracing::enabled!(tracing::Level::DEBUG) {
							tracing::debug!(
								data = stringify_to_envoy(&decoded),
								"received message"
							);
						}

						forward_to_envoy(shared, decoded).await;
					}
					Err(error) => {
						if let Some(fallback) = fallback_protocol_version(
							shared,
							protocol_version,
							received_init,
							"failed to decode init payload",
						) {
							result = fallback;
							break;
						}

						return Err(error);
					}
				}
			}
			Ok(tungstenite::Message::Close(frame)) => {
				if let Some(frame) = frame {
					let reason_str = frame.reason.to_string();
					let code: u16 = frame.code.into();
					tracing::info!(code, reason = %reason_str, "websocket closed");
					result = if let Some(fallback) = fallback_protocol_version(
						shared,
						protocol_version,
						received_init,
						"connection closed before init",
					) {
						fallback
					} else {
						SingleConnectionResult::Closed(parse_ws_close_reason(&reason_str))
					};
				} else if let Some(fallback) = fallback_protocol_version(
					shared,
					protocol_version,
					received_init,
					"connection closed before init",
				) {
					result = fallback;
				}
				break;
			}
			Err(error) => {
				if let Some(fallback) = fallback_protocol_version(
					shared,
					protocol_version,
					received_init,
					"websocket error before init",
				) {
					result = fallback;
					break;
				}

				return Err(error.into());
			}
			_ => {}
		}
	}

	{
		let mut guard = shared.ws_tx.lock().await;
		*guard = None;
	}
	write_handle.abort();

	if matches!(result, SingleConnectionResult::RetryLowerProtocol { .. }) {
		let mut guard = shared.protocol_metadata.lock().await;
		*guard = None;
	}

	Ok(result)
}

fn fallback_protocol_version(
	shared: &SharedContext,
	current_version: u16,
	received_init: bool,
	reason: &'static str,
) -> Option<SingleConnectionResult> {
	if received_init {
		return None;
	}

	next_lower_protocol_version(current_version).map(|next_version| {
		shared
			.protocol_version
			.store(next_version, Ordering::Release);
		SingleConnectionResult::RetryLowerProtocol {
			from: current_version,
			to: next_version,
			reason,
		}
	})
}

fn next_lower_protocol_version(current_version: u16) -> Option<u16> {
	(current_version > 1).then_some(current_version - 1)
}

fn current_protocol_version(shared: &SharedContext) -> u16 {
	shared.protocol_version.load(Ordering::Acquire)
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
		.serialize(current_protocol_version(shared))
		.expect("failed to encode message");
	let _ = tx.send(WsTxMessage::Send(encoded));
	false
}

fn ws_url(shared: &SharedContext, protocol_version: u16) -> String {
	let ws_endpoint = shared
		.config
		.endpoint
		.replace("http://", "ws://")
		.replace("https://", "wss://");
	let base_url = ws_endpoint.trim_end_matches('/');

	format!(
		"{}/envoys/connect?protocol_version={}&namespace={}&envoy_key={}&version={}&pool_name={}",
		base_url,
		protocol_version,
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn next_lower_protocol_version_stops_at_v1() {
		assert_eq!(next_lower_protocol_version(2), Some(1));
		assert_eq!(next_lower_protocol_version(1), None);
	}
}
