use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures_util::{SinkExt, StreamExt};
use rivet_envoy_protocol as protocol;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio_tungstenite::tungstenite;
use tracing::Instrument;
use vbare::Versioned;

use crate::context::{HttpWsTxMessage, SharedContext, WsTxMessage};
use crate::envoy::ToEnvoyMessage;
use crate::handle::EnvoyHandle;
use crate::metrics::METRICS;
use crate::utils::{parse_ws_close_reason, reconnect_delay};

const STABLE_CONNECTION_MS: u64 = 60_000;

pub fn start_connection(shared: Arc<SharedContext>) -> Option<tokio::task::JoinHandle<()>> {
	let span = tracing::debug_span!("envoy_connection", envoy_key = %shared.envoy_key);
	Some(tokio::spawn(connection_loop(shared).instrument(span)))
}

/// Whether a read failed because the peer dropped the connection without a close frame.
/// Every other error, such as a protocol violation or an oversized frame, is a fault.
fn is_connection_reset(error: &tungstenite::Error) -> bool {
	let closed = matches!(
		error,
		tungstenite::Error::Protocol(
			tungstenite::error::ProtocolError::ResetWithoutClosingHandshake
		) | tungstenite::Error::ConnectionClosed
			| tungstenite::Error::AlreadyClosed
	);
	let reset = matches!(
		error,
		tungstenite::Error::Io(io) if matches!(
			io.kind(),
			std::io::ErrorKind::ConnectionReset
				| std::io::ErrorKind::ConnectionAborted
				| std::io::ErrorKind::BrokenPipe
				| std::io::ErrorKind::UnexpectedEof
		)
	);

	closed || reset
}

fn websocket_config() -> tungstenite::protocol::WebSocketConfig {
	tungstenite::protocol::WebSocketConfig::default()
		.max_message_size(None)
		.max_frame_size(None)
}

async fn connection_loop(shared: Arc<SharedContext>) {
	let mut attempt = 0u32;

	loop {
		if shared.shutting_down.load(Ordering::Acquire) {
			tracing::debug!("stopping reconnect loop because envoy is shutting down");
			return;
		}

		// Stability is judged on the clock the backoff sleeps on.
		let connected_at = crate::time::Instant::now();

		let mut retry_after_ms = None;
		match single_connection(&shared).await {
			Ok((session, close_reason)) => {
				retry_after_ms = close_reason
					.as_ref()
					.and_then(|reason| reason.retry_after_ms);
				if let Some(reason) = close_reason {
					if reason.group == "pool" && reason.error == "not_found" {
						let _ = crate::envoy::send_to_envoy_tx(
							&shared,
							ToEnvoyMessage::PoolNotFound {
								session,
								ray_id: reason.ray_id,
							},
						);
						return;
					}
					if reason.group == "ws" && reason.error == "eviction" {
						tracing::debug!("connection evicted");
						let _ = crate::envoy::send_to_envoy_tx(
							&shared,
							ToEnvoyMessage::ConnClose {
								evict: true,
								session,
							},
						);
						return;
					}
				}
				let _ = crate::envoy::send_to_envoy_tx(
					&shared,
					ToEnvoyMessage::ConnClose {
						evict: false,
						session,
					},
				);
			}
			Err(error) => {
				tracing::error!(?error, "connection failed");
				let session = shared.connection_session.load(Ordering::Acquire);
				if session != 0 {
					super::remove_connection_for_session(&shared, session).await;
				}
				let _ = crate::envoy::send_to_envoy_tx(
					&shared,
					ToEnvoyMessage::ConnClose {
						evict: false,
						session,
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

		let lost_threshold_ms = super::reconnect_lost_threshold_ms(&shared).await;
		let delay = reconnect_delay(attempt, retry_after_ms, lost_threshold_ms);
		tracing::info!(attempt, delay_ms = delay.as_millis() as u64, "reconnecting");
		tokio::time::sleep(delay).await;
		attempt += 1;
	}
}

async fn single_connection(
	shared: &Arc<SharedContext>,
) -> anyhow::Result<(u64, Option<crate::utils::ParsedCloseReason>)> {
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

	// Storage replies and lifecycle messages are latency-sensitive, including
	// small frames sent immediately after another frame on this shared socket.
	match &shared.config.connector {
		Some(connector) => {
			let stream = connector.connect(&url).await?;
			let (ws_stream, response) = tokio_tungstenite::client_async_with_config(
				request,
				stream,
				Some(websocket_config()),
			)
			.await?;
			serve_connection(
				shared,
				ws_stream,
				super::sqlite_trace_connection(
					response
						.headers()
						.get("x-rivet-sqlite-trace-connection")
						.map(|value| value.as_bytes()),
				),
			)
			.await
		}
		None => {
			let (ws_stream, response) = tokio_tungstenite::connect_async_with_config(
				request,
				Some(websocket_config()),
				true,
			)
			.await?;
			serve_connection(
				shared,
				ws_stream,
				super::sqlite_trace_connection(
					response
						.headers()
						.get("x-rivet-sqlite-trace-connection")
						.map(|value| value.as_bytes()),
				),
			)
			.await
		}
	}
}

/// Runs one connected WebSocket session until it closes or fails.
async fn serve_connection<S>(
	shared: &Arc<SharedContext>,
	ws_stream: tokio_tungstenite::WebSocketStream<S>,
	server_connection: Option<uuid::Uuid>,
) -> anyhow::Result<(u64, Option<crate::utils::ParsedCloseReason>)>
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
	let (mut write, mut read) = ws_stream.split();

	// TODO: Bound shared WebSocket writer memory across protocol message types.
	// https://github.com/rivet-dev/rivet/issues/5468
	let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<WsTxMessage>();
	let (http_ws_tx, mut http_ws_rx) =
		mpsc::channel::<HttpWsTxMessage>(super::HTTP_WS_MESSAGE_CAPACITY);
	let http_byte_budget = Arc::new(Semaphore::new(super::HTTP_WS_BYTE_CAPACITY));
	// Pongs skip the shared writer queue: on a busy node that queue can hold seconds of
	// storage requests and events, and a pong waiting behind them gets the envoy declared
	// dead. Only the latest pong matters, so the slot holds one.
	let (pong_tx, mut pong_rx) = tokio::sync::watch::channel::<Option<Vec<u8>>>(None);
	let session =
		super::install_connection_with_http(shared, ws_tx, http_ws_tx, http_byte_budget).await;
	if session == 0 {
		anyhow::bail!("envoy shut down during websocket handshake");
	}

	tracing::trace!(target: "sqlite_prefetch::trace", event = "connection_session",
        envoy_key = %shared.envoy_key, session,
        server_connection = server_connection.as_ref().map(ToString::to_string),
        "SQLite client and server session mapping; None means unavailable");
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
	let (write_failed_tx, mut write_failed_rx) = oneshot::channel();
	let write_handle = tokio::spawn(
		async move {
			let mut write_failed_tx = Some(write_failed_tx);
			super::send_initial_metadata(&shared2).await;

			loop {
				let msg = tokio::select! {
					biased;
					changed = pong_rx.changed() => {
						if changed.is_err() {
							break;
						}
						let pong = pong_rx.borrow_and_update().clone();
						let Some(pong) = pong else { continue };
						Some(EitherWrite::Legacy(WsTxMessage::Send(pong)))
					}
					msg = ws_rx.recv() => msg.map(EitherWrite::Legacy),
					msg = http_ws_rx.recv() => msg.map(EitherWrite::Http),
				};
				let Some(msg) = msg else { break };
				match msg {
					EitherWrite::Legacy(WsTxMessage::Send(data)) => {
						let result = write.send(tungstenite::Message::Binary(data.into())).await;
						if let Err(e) = result {
							tracing::error!(?e, "failed to send ws message");
							if let Some(write_failed_tx) = write_failed_tx.take() {
								let _ = write_failed_tx.send(());
							}
							break;
						}
					}
					EitherWrite::Legacy(WsTxMessage::Close) => {
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
					EitherWrite::Http(message) => {
						let result = write
							.send(tungstenite::Message::Binary(message.data.into()))
							.await
							.map_err(anyhow::Error::from);
						let failed = result.is_err();
						let _ = message.written.send(result);
						if failed {
							if let Some(write_failed_tx) = write_failed_tx.take() {
								let _ = write_failed_tx.send(());
							}
							break;
						}
					}
				}
			}
		}
		.instrument(write_span),
	);

	let mut result = None;

	let debug_latency_ms = shared.config.debug_latency_ms;

	loop {
		let msg = tokio::select! {
			msg = read.next() => msg,
			_ = &mut write_failed_rx => {
				disconnect_reason = "write_error";
				break;
			}
		};
		let Some(msg) = msg else { break };
		match msg {
			Ok(tungstenite::Message::Binary(data)) => {
				crate::utils::inject_latency(debug_latency_ms).await;

				// Everything below runs before the next `read.next()`, so this is time the socket
				// is not being drained. A slow reader shows up here rather than in the idle wait.
				let handler_start = std::time::Instant::now();
				METRICS.ws_rx_bytes_total.inc_by(data.len() as u64);
				METRICS.ws_rx_messages_total.inc();

				let decode_start = std::time::Instant::now();
				// Decoding shares the frame: tunnel bodies and SQLite pages are slices of it.
				let decoded = crate::protocol::versioned::ToEnvoy::decode_body(
					data.into(),
					protocol::PROTOCOL_VERSION,
				)?;
				METRICS
					.ws_decode_duration
					.observe(decode_start.elapsed().as_secs_f64());

				match decoded {
					protocol::ToEnvoy::ToEnvoyPing(ping) => {
						if let Some(pong) = super::encode_pong(shared, ping) {
							pong_tx.send_replace(Some(pong));
						}
					}
					other => super::forward_to_envoy(shared, session, other).await,
				}

				METRICS
					.ws_read_handler_duration
					.observe(handler_start.elapsed().as_secs_f64());
			}
			Ok(tungstenite::Message::Close(frame)) => {
				disconnect_reason = "close";
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
				disconnect_reason = "error";
				if is_connection_reset(&e) {
					// The engine went away without a close frame, as when its process exits.
					// The envoy reconnects, so this is an expected disconnect, not a fault.
					tracing::warn!(?e, "websocket reset without a close frame; reconnecting");
				} else {
					tracing::error!(?e, "websocket error");
				}
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
	tracing::info!(
		envoy_key = %shared.envoy_key,
		reason = disconnect_reason,
		session_duration_ms = session_duration.as_millis() as u64,
		"websocket session ended"
	);

	// Clean up
	super::remove_connection_for_session(shared, session).await;
	write_handle.abort();
	shared
		.config
		.callbacks
		.on_disconnect(EnvoyHandle::from_shared(shared.clone()));

	Ok((session, result))
}

enum EitherWrite {
	Legacy(WsTxMessage),
	Http(HttpWsTxMessage),
}

fn extract_host(url: &str) -> String {
	url.replace("ws://", "")
		.replace("wss://", "")
		.split('/')
		.next()
		.unwrap_or("localhost")
		.to_string()
}


