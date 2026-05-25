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
use crate::metrics::METRICS;
use crate::utils::{BackoffOptions, calculate_backoff, display_id, parse_ws_close_reason};

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
						let _ = crate::envoy::send_to_envoy_tx(
							&shared,
							ToEnvoyMessage::ConnClose {
								evict: true,
								was_error: false,
							},
						);
						return;
					}
				}
				let _ = crate::envoy::send_to_envoy_tx(
					&shared,
					ToEnvoyMessage::ConnClose {
						evict: false,
						was_error: false,
					},
				);
			}
			Err(error) => {
				tracing::error!(?error, "connection failed");
				let _ = crate::envoy::send_to_envoy_tx(
					&shared,
					ToEnvoyMessage::ConnClose {
						evict: false,
						was_error: true,
					},
				);
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

	// Disable Nagle on the underlying TCP socket. Envoy traffic is small,
	// bidirectional, and latency-sensitive (pings, acks, events); without this each
	// sub-MSS write can stall up to 200ms behind delayed ACKs. Set
	// RIVET_ENVOY_CLIENT_TCP_NODELAY=false to opt out.
	let nodelay_enabled = std::env::var("RIVET_ENVOY_CLIENT_TCP_NODELAY")
		.map(|v| !matches!(v.as_str(), "0" | "false" | "no" | "off"))
		.unwrap_or(true);
	if nodelay_enabled {
		use tokio_tungstenite::MaybeTlsStream;
		let result = match ws_stream.get_ref() {
			MaybeTlsStream::Plain(tcp) => tcp.set_nodelay(true),
			MaybeTlsStream::Rustls(tls) => tls.get_ref().0.set_nodelay(true),
			// MaybeTlsStream is #[non_exhaustive]. Hitting this arm means
			// upstream added a variant (or a feature flag we don't enable
			// today is now in effect — e.g. native-tls). Surface loudly so
			// we add explicit handling instead of silently leaving Nagle on.
			other => {
				tracing::warn!(
					discriminant = ?std::mem::discriminant(other),
					"envoy ws using unsupported MaybeTlsStream variant; TCP_NODELAY skipped"
				);
				Ok(())
			}
		};
		if let Err(err) = result {
			tracing::debug!(?err, "failed to enable TCP_NODELAY on envoy ws");
		}
	}

	let (mut write, mut read) = ws_stream.split();

	let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<WsTxMessage>();
	shared.ws_tx.store(Some(Arc::new(ws_tx)));

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

	let session_start = std::time::Instant::now();
	let mut disconnect_reason: &'static str = "stream_end";

	// Spawn write task
	let shared2 = shared.clone();
	let write_span = tracing::debug_span!("envoy_ws_write", envoy_key = %shared2.envoy_key);
	let write_handle = tokio::spawn(
		async move {
			super::send_initial_metadata(&shared2).await;

			// Threshold above which we log per-message timing diagnostics. Picked to surface
			// backpressure that could plausibly contribute to engine ping timeouts (15s default)
			// without spamming on normal operation.
			const SLOW_WRITE_THRESHOLD_MS: i64 = 1_000;

			while let Some(msg) = ws_rx.recv().await {
				match msg {
					WsTxMessage::Send {
						data,
						enqueue_ts,
						is_pong,
						message_kind,
						gateway_id,
						request_id,
						message_index,
						inner_data_len,
					} => {
						let depth_after_recv =
							shared2.ws_tx_depth.fetch_sub(1, Ordering::AcqRel) - 1;
						METRICS.ws_tx_depth.dec();
						let payload_len = data.len();
						let dequeue_ts = crate::time::now_millis();
						let queue_wait_ms = dequeue_ts - enqueue_ts;
						let write_start = std::time::Instant::now();
						let result = write
							.send(tungstenite::Message::Binary(data.into()))
							.await;
						let write_elapsed_ms = write_start.elapsed().as_millis() as i64;
						if let Err(e) = result {
							tracing::error!(?e, "failed to send ws message");
							break;
						}
						let now = crate::time::now_millis();
						if is_pong {
							shared2.last_pong_sent_ts.store(now, Ordering::Release);
						}
						let total_latency_ms = now - enqueue_ts;
						if let (Some(gateway_id), Some(request_id), Some(message_index)) =
							(gateway_id.as_ref(), request_id.as_ref(), message_index)
						{
							tracing::trace!(
								envoy_key = %shared2.envoy_key,
								message_kind,
								gateway_id = %display_id(gateway_id),
								request_id = %display_id(request_id),
								message_index,
								inner_data_len,
								payload_len,
								queue_wait_ms,
								write_elapsed_ms,
								total_latency_ms,
								ws_tx_depth = depth_after_recv,
								"wrote websocket message to engine"
							);
						} else {
							tracing::trace!(
								envoy_key = %shared2.envoy_key,
								message_kind,
								inner_data_len,
								payload_len,
								queue_wait_ms,
								write_elapsed_ms,
								total_latency_ms,
								ws_tx_depth = depth_after_recv,
								"wrote websocket message to engine"
							);
						}
						if is_pong && total_latency_ms >= SLOW_WRITE_THRESHOLD_MS {
							tracing::warn!(
								envoy_key = %shared2.envoy_key,
								queue_wait_ms,
								write_elapsed_ms,
								total_latency_ms,
								ws_tx_depth = depth_after_recv,
								"pong write exceeded slow threshold"
							);
						} else if write_elapsed_ms >= SLOW_WRITE_THRESHOLD_MS {
							tracing::warn!(
								envoy_key = %shared2.envoy_key,
								is_pong,
								queue_wait_ms,
								write_elapsed_ms,
								total_latency_ms,
								ws_tx_depth = depth_after_recv,
								"ws outbound write exceeded slow threshold"
							);
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
				disconnect_reason = "close";
				if let Some(frame) = frame {
					let reason_str = frame.reason.to_string();
					let code: u16 = frame.code.into();
					let now = crate::time::now_millis();
					let since_last_pong_sent_ms =
						now - shared.last_pong_sent_ts.load(Ordering::Acquire);
					let ws_tx_depth = shared.ws_tx_depth.load(Ordering::Acquire);
					tracing::warn!(
						envoy_key = %shared.envoy_key,
						code,
						reason = %reason_str,
						since_last_pong_sent_ms,
						ws_tx_depth,
						"websocket closed"
					);
					result = parse_ws_close_reason(&reason_str);
				}
				break;
			}
			Err(e) => {
				disconnect_reason = "error";
				let last_ping_ts = shared.last_ping_ts.load(std::sync::atomic::Ordering::Acquire);
				let time_since_last_ping_ms = if last_ping_ts == 0 {
					None
				} else {
					Some(crate::time::now_millis() - last_ping_ts)
				};
				tracing::error!(
					?e,
					?time_since_last_ping_ms,
					"websocket error"
				);
				break;
			}
			_ => {}
		}
	}

	let session_duration = session_start.elapsed();
	METRICS
		.ws_session_duration_seconds
		.observe(session_duration.as_secs_f64());
	METRICS
		.ws_reconnect_total
		.with_label_values(&[disconnect_reason])
		.inc();
	super::observe_ping_unhealthy_on_close(shared);
	tracing::info!(
		envoy_key = %shared.envoy_key,
		reason = disconnect_reason,
		session_duration_ms = session_duration.as_millis() as u64,
		"websocket session ended"
	);

	// Clean up
	shared.ws_tx.store(None);
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
