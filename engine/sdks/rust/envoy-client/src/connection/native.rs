use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use futures_util::future::Either;
use futures_util::{SinkExt, Stream, StreamExt};
use rivet_envoy_protocol as protocol;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio_tungstenite::tungstenite;
use tracing::Instrument;
use vbare::Versioned;

use crate::context::{HttpWsTxMessage, SharedContext, WsTxMessage};
use crate::envoy::ToEnvoyMessage;
use crate::handle::EnvoyHandle;
use crate::metrics;
use crate::time::Instant;
use crate::utils::{ParsedCloseReason, parse_ws_close_reason, reconnect_delay};

const STABLE_CONNECTION_MS: u64 = 60_000;
const OPEN_TIMEOUT: Duration = Duration::from_secs(30);
const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const CLOSE_GRACE: Duration = Duration::from_millis(100);
/// How long a node waits for the second engine's pre-connect once the first arrived. Both
/// connections open together, so this only absorbs the gap between two handshakes.
const PRE_CONNECT_GRACE: Duration = Duration::from_millis(250);
/// How long a node waits for a replacement connection after both reached the same engine.
const COLLISION_RETRY_TIMEOUT: Duration = Duration::from_secs(1);
/// The close reason sent on a connection the node did not choose.
const UNCHOSEN_CLOSE_REASON: &str = "envoy.unchosen";

enum SessionEnd {
	Closed(Option<ParsedCloseReason>, Option<Instant>),
	ContactExpired,
}

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
			Ok((session, SessionEnd::Closed(close_reason, contact_deadline))) => {
				retry_after_ms = close_reason
					.as_ref()
					.and_then(|reason| reason.retry_after_ms);
				if super::report_connection_close(&shared, session, close_reason, contact_deadline)
				{
					return;
				}
			}
			Ok((session, SessionEnd::ContactExpired)) => {
				let (resume, wait) = oneshot::channel();
				if crate::envoy::send_to_envoy_tx(
					&shared,
					ToEnvoyMessage::ContactExpired { session, resume },
				)
				.is_err() || wait.await.is_err()
				{
					return;
				}
			}
			Err(error) => {
				tracing::error!(?error, "connection failed");
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

/// Builds a fresh upgrade request; each connection needs its own `Sec-WebSocket-Key`.
fn upgrade_request(
	shared: &SharedContext,
	url: &str,
) -> anyhow::Result<tungstenite::http::Request<()>> {
	let protocols = {
		let mut p = vec!["rivet".to_string()];
		if let Some(token) = &shared.config.token {
			p.push(format!("rivet_token.{token}"));
		}
		p
	};

	tungstenite::http::Request::builder()
		.uri(url)
		.header("Sec-WebSocket-Protocol", protocols.join(", "))
		.header("Connection", "Upgrade")
		.header("Upgrade", "websocket")
		.header(
			"Sec-WebSocket-Key",
			tungstenite::handshake::client::generate_key(),
		)
		.header("Sec-WebSocket-Version", "13")
		.header("Host", extract_host(url))
		.body(())
		.map_err(|e| anyhow::anyhow!("failed to build ws request: {e}"))
}

fn server_connection(response: &tungstenite::handshake::client::Response) -> Option<uuid::Uuid> {
	super::sqlite_trace_connection(
		response
			.headers()
			.get("x-rivet-sqlite-trace-connection")
			.map(|value| value.as_bytes()),
	)
}

#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its WebSocket transport."
)]
async fn single_connection(shared: &Arc<SharedContext>) -> anyhow::Result<(u64, SessionEnd)> {
	let url = super::ws_url(shared);

	// Initialize with a default CryptoProvider for rustls
	let provider = rustls::crypto::ring::default_provider();
	if provider.install_default().is_err() {
		tracing::debug!("crypto provider already installed in this process");
	}

	// Storage replies and lifecycle messages are latency-sensitive, including
	// small frames sent immediately after another frame on this shared socket.
	match &shared.config.connector {
		Some(connector) => {
			let url = url.as_str();
			let dial = move || async move {
				let request = upgrade_request(shared, url)?;
				let stream = connector.connect(url).await?;
				let (ws_stream, response) = tokio_tungstenite::client_async_with_config(
					request,
					stream,
					Some(websocket_config()),
				)
				.await?;
				Ok((ws_stream, server_connection(&response)))
			};
			connect_balanced(shared, dial).await
		}
		None => {
			let url = url.as_str();
			let dial = move || async move {
				let request = upgrade_request(shared, url)?;
				let (ws_stream, response) = tokio_tungstenite::connect_async_with_config(
					request,
					Some(websocket_config()),
					true,
				)
				.await?;
				Ok((ws_stream, server_connection(&response)))
			};
			connect_balanced(shared, dial).await
		}
	}
}

/// One opened WebSocket and what its engine sent first.
struct Candidate<S> {
	ws: tokio_tungstenite::WebSocketStream<S>,
	server_connection: Option<uuid::Uuid>,
	/// The engine's pre-connect, absent when it sent something else first, such as the close
	/// frame of a rejected connect.
	pre_connect: Option<protocol::ToEnvoyPreConnect>,
	/// The first message when it was not a pre-connect. The session replays it, so a rejection
	/// is handled as if the connection had been served from the start.
	replay: Option<tungstenite::Message>,
}

/// Which of two engines that both sent a pre-connect a node keeps.
#[derive(Debug, PartialEq, Eq)]
enum Pick {
	First,
	Second,
	/// Both connections reached the same engine.
	SameEngine,
}

/// Keeps the engine holding fewer node connections, and the first on a tie.
fn pick(first: &protocol::ToEnvoyPreConnect, second: &protocol::ToEnvoyPreConnect) -> Pick {
	if first.worker_id == second.worker_id {
		return Pick::SameEngine;
	}

	if second.envoy_connections < first.envoy_connections {
		Pick::Second
	} else {
		Pick::First
	}
}

/// Opens connections to up to two engines, keeps the one holding fewer node connections, closes
/// the other and serves the kept one. Nothing is sent before the choice, so the engine not
/// chosen holds no state for this node.
async fn connect_balanced<S, D, Fut>(
	shared: &Arc<SharedContext>,
	dial: D,
) -> anyhow::Result<(u64, SessionEnd)>
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
	D: Fn() -> Fut,
	Fut: Future<
		Output = anyhow::Result<(tokio_tungstenite::WebSocketStream<S>, Option<uuid::Uuid>)>,
	>,
{
	let (chosen, result) = choose(&dial).await?;
	metrics::WS_PRE_CONNECT.with_label_values(&[result]).inc();
	if let Some(pre_connect) = &chosen.pre_connect {
		tracing::debug!(
			worker_id = %uuid::Uuid::from_bytes(pre_connect.worker_id),
			envoy_connections = pre_connect.envoy_connections,
			result,
			"chose engine"
		);
	}

	serve_connection(shared, chosen.ws, chosen.server_connection, chosen.replay).await
}

/// Picks the connection to serve and the `result` label recording how. Unchosen connections
/// are closed. The labels:
///
/// - `balanced`: two engines answered and the lighter one was kept.
/// - `one_answered`: one engine answered; the other connection failed, was rejected or missed
///   the grace period.
/// - `none_answered`: no engine answered, so a rejection is served for its close reason.
/// - `retried_balanced`: both connections reached the same engine and the retry reached another.
/// - `retried_same_engine`: both connections reached the same engine and the retry did too, or
///   failed. With one engine every connect ends here.
#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its Tokio transport deadline."
)]
async fn choose<S, D, Fut>(dial: &D) -> anyhow::Result<(Candidate<S>, &'static str)>
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
	D: Fn() -> Fut,
	Fut: Future<
		Output = anyhow::Result<(tokio_tungstenite::WebSocketStream<S>, Option<uuid::Uuid>)>,
	>,
{
	// Both connections open together. Once one engine sent its pre-connect, the other gets a
	// short grace; a connection that failed or was rejected waits for the other in full.
	let both = futures_util::future::select(
		Box::pin(open_candidate(dial())),
		Box::pin(open_candidate(dial())),
	)
	.await;
	let (early, late) = match both {
		Either::Left((early, late)) | Either::Right((early, late)) => (early, late),
	};
	let early_ready = matches!(&early, Ok(candidate) if candidate.pre_connect.is_some());
	let late = if early_ready {
		tokio::time::timeout(PRE_CONNECT_GRACE, late).await.ok()
	} else {
		Some(late.await)
	};

	let (early, late) = match (early, late) {
		(Ok(early), Some(Ok(late))) => (early, late),
		(Ok(early), None | Some(Err(_))) => {
			let result = if early.pre_connect.is_some() {
				"one_answered"
			} else {
				"none_answered"
			};
			return Ok((early, result));
		}
		(Err(_), Some(Ok(late))) => {
			let result = if late.pre_connect.is_some() {
				"one_answered"
			} else {
				"none_answered"
			};
			return Ok((late, result));
		}
		(Err(error), None | Some(Err(_))) => return Err(error),
	};

	let (Some(early_pre_connect), Some(late_pre_connect)) = (&early.pre_connect, &late.pre_connect)
	else {
		// At most one engine sent a pre-connect. A rejected connection is served only when
		// neither did, so its close reason reaches the envoy.
		return Ok(if late.pre_connect.is_some() {
			close_unchosen(early);
			(late, "one_answered")
		} else {
			let result = if early.pre_connect.is_some() {
				"one_answered"
			} else {
				"none_answered"
			};
			close_unchosen(late);
			(early, result)
		});
	};

	match pick(early_pre_connect, late_pre_connect) {
		Pick::First => {
			close_unchosen(late);
			Ok((early, "balanced"))
		}
		Pick::Second => {
			close_unchosen(early);
			Ok((late, "balanced"))
		}
		Pick::SameEngine => {
			// Under random balancing two connections reach the same engine with probability
			// 1/N. One more try usually reaches another; otherwise the node keeps what it has,
			// which is no worse than a single random connection. With one engine the retry
			// always reaches it again, at the cost of one more handshake.
			close_unchosen(late);
			let retry = tokio::time::timeout(COLLISION_RETRY_TIMEOUT, open_candidate(dial())).await;
			let Ok(Ok(retry)) = retry else {
				return Ok((early, "retried_same_engine"));
			};
			let Some(retry_pre_connect) = &retry.pre_connect else {
				close_unchosen(retry);
				return Ok((early, "retried_same_engine"));
			};
			match pick(early_pre_connect, retry_pre_connect) {
				Pick::Second => {
					close_unchosen(early);
					Ok((retry, "retried_balanced"))
				}
				Pick::First => {
					close_unchosen(retry);
					Ok((early, "retried_balanced"))
				}
				Pick::SameEngine => {
					close_unchosen(retry);
					Ok((early, "retried_same_engine"))
				}
			}
		}
	}
}

/// Dials one connection and reads the engine's first message.
#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its Tokio transport deadline."
)]
async fn open_candidate<S, Fut>(dial: Fut) -> anyhow::Result<Candidate<S>>
where
	Fut: Future<
		Output = anyhow::Result<(tokio_tungstenite::WebSocketStream<S>, Option<uuid::Uuid>)>,
	>,
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
	tokio::time::timeout(OPEN_TIMEOUT, async {
		let (mut ws, server_connection) = dial.await?;
		let first = loop {
			match ws.next().await {
				Some(Ok(message @ tungstenite::Message::Binary(_)))
				| Some(Ok(message @ tungstenite::Message::Close(_))) => break Some(message),
				Some(Ok(_)) => {}
				Some(Err(error)) => return Err(anyhow::Error::from(error)),
				None => break None,
			}
		};

		let pre_connect = match &first {
			Some(tungstenite::Message::Binary(data)) => {
				match crate::protocol::versioned::ToEnvoy::decode_body(
					data.clone(),
					protocol::PROTOCOL_VERSION,
				) {
					Ok(protocol::ToEnvoy::ToEnvoyPreConnect(pre_connect)) => Some(pre_connect),
					Ok(_) | Err(_) => None,
				}
			}
			_ => None,
		};
		let replay = match pre_connect {
			Some(_) => None,
			None => first,
		};

		Ok(Candidate {
			ws,
			server_connection,
			pre_connect,
			replay,
		})
	})
	.await
	.context("node WebSocket dial, upgrade or pre-connect timed out")?
}

/// Closes a connection the node did not choose. Its engine wrote nothing for it.
#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its Tokio transport deadline."
)]
fn close_unchosen<S>(candidate: Candidate<S>)
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
	let mut ws = candidate.ws;
	tokio::spawn(async move {
		let frame = tungstenite::protocol::CloseFrame {
			code: tungstenite::protocol::frame::coding::CloseCode::Normal,
			reason: UNCHOSEN_CLOSE_REASON.into(),
		};
		let _ = tokio::time::timeout(CLOSE_GRACE, ws.close(Some(frame))).await;
	});
}

/// Runs one connected WebSocket session until it closes or fails.
#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its Tokio transport deadline."
)]
async fn serve_connection<S>(
	shared: &Arc<SharedContext>,
	ws_stream: tokio_tungstenite::WebSocketStream<S>,
	server_connection: Option<uuid::Uuid>,
	replay: Option<tungstenite::Message>,
) -> anyhow::Result<(u64, SessionEnd)>
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
	let (mut write, read) = ws_stream.split();
	let mut read = futures_util::stream::iter(replay.map(Ok)).chain(read);

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
	let mut writer = tokio::task::JoinSet::new();
	writer.spawn(
		async move {
			super::send_initial_metadata(&shared2).await;

			let mut http_first = false;
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
					msg = next_queued_write(&mut ws_rx, &mut http_ws_rx, http_first) => {
						if let Some(msg) = &msg {
							http_first = matches!(msg, EitherWrite::Legacy(_));
						}
						msg
					}
				};
				let Some(msg) = msg else { break };
				match msg {
					EitherWrite::Legacy(WsTxMessage::Send(data)) => {
						let result = write.send(tungstenite::Message::Binary(data.into())).await;
						if let Err(e) = result {
							tracing::error!(?e, "failed to send ws message");
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
	let mut deadline = Instant::now() + INIT_TIMEOUT;
	let mut contact_threshold = None;

	let timed_out = loop {
		let msg = tokio::select! {
			biased;
			_ = tokio::time::sleep_until(deadline) => break true,
			_ = writer.join_next() => {
				disconnect_reason = "write_error";
				result = read_close_after_writer(&mut read, deadline).await
					.and_then(|frame| parse_ws_close_reason(&frame.reason));
				break Instant::now() >= deadline;
			}
			msg = read.next() => msg,
		};
		if Instant::now() >= deadline {
			break true;
		}
		let Some(msg) = msg else { break false };
		match msg {
			Ok(tungstenite::Message::Binary(data)) => {
				crate::utils::inject_latency(debug_latency_ms).await;

				// Everything below runs before the next `read.next()`, so this is time the socket
				// is not being drained. A slow reader shows up here rather than in the idle wait.
				let handler_start = std::time::Instant::now();
				metrics::WS_RX_BYTES.inc_by(data.len() as u64);
				metrics::WS_RX_MESSAGES.inc();

				let decode_start = std::time::Instant::now();
				// Decoding shares the frame: tunnel bodies and SQLite pages are slices of it.
				let decoded = match crate::protocol::versioned::ToEnvoy::decode_body(
					data.into(),
					protocol::PROTOCOL_VERSION,
				) {
					Ok(message) => message,
					Err(error) => {
						tracing::error!(?error, "invalid Engine WebSocket message");
						break false;
					}
				};
				metrics::WS_DECODE_DURATION.observe(decode_start.elapsed().as_secs_f64());

				let now = Instant::now();
				if now >= deadline {
					break true;
				}
				// Pre-Init pings get a pong but do not extend registration.
				let renew_with = match &decoded {
					protocol::ToEnvoy::ToEnvoyInit(init) => {
						let Ok(ms) = u64::try_from(init.metadata.envoy_lost_threshold) else {
							tracing::error!("negative node lost threshold");
							break false;
						};
						Some(Duration::from_millis(ms))
					}
					protocol::ToEnvoy::ToEnvoyPing(_) => contact_threshold,
					_ => None,
				};
				if let Some(threshold) = renew_with {
					let Some(next_deadline) = now.checked_add(threshold) else {
						tracing::error!("node lost threshold exceeds the clock range");
						break false;
					};
					deadline = next_deadline;
					contact_threshold = Some(threshold);
				}

				match decoded {
					protocol::ToEnvoy::ToEnvoyPing(ping) => {
						if let Some(pong) = super::encode_pong(shared, ping) {
							pong_tx.send_replace(Some(pong));
						}
					}
					other => super::forward_to_envoy(shared, session, other).await,
				}

				metrics::WS_READ_HANDLER_DURATION.observe(handler_start.elapsed().as_secs_f64());
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
				break false;
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
				break false;
			}
			_ => {}
		}
	};
	let end = if timed_out && contact_threshold.is_some() {
		shared
			.expired_connection_session
			.store(session, Ordering::Release);
		writer.abort_all();
		disconnect_reason = "contact_timeout";
		tracing::warn!(
			session,
			"node contact expired while the WebSocket remained open"
		);
		SessionEnd::ContactExpired
	} else {
		if timed_out {
			disconnect_reason = "init_timeout";
		}
		SessionEnd::Closed(result, contact_threshold.map(|_| deadline))
	};

	let session_duration = session_start.elapsed();
	metrics::WS_SESSION_DURATION_SECONDS.observe(session_duration.as_secs_f64());
	metrics::WS_RECONNECT
		.with_label_values(&[disconnect_reason])
		.inc();
	tracing::info!(
		envoy_key = %shared.envoy_key,
		reason = disconnect_reason,
		session_duration_ms = session_duration.as_millis() as u64,
		"websocket session ended"
	);

	// Clean up
	writer.shutdown().await;
	drop(read);
	super::remove_connection_for_session(shared, session).await;
	shared
		.config
		.callbacks
		.on_disconnect(EnvoyHandle::from_shared(shared.clone()));

	Ok((session, end))
}

// A structured close can be queued behind pings when the writer fails.
#[allow(
	clippy::disallowed_methods,
	reason = "The native SDK owns its Tokio transport deadline."
)]
async fn read_close_after_writer(
	read: &mut (impl Stream<Item = Result<tungstenite::Message, tungstenite::Error>> + Unpin),
	deadline: Instant,
) -> Option<tungstenite::protocol::CloseFrame> {
	let until = deadline.min(Instant::now() + CLOSE_GRACE);
	while Instant::now() < until {
		match tokio::time::timeout_at(until, read.next()).await {
			Ok(Some(Ok(tungstenite::Message::Close(frame)))) => return frame,
			Ok(Some(Ok(_))) => tokio::task::yield_now().await,
			_ => break,
		}
	}
	None
}

enum EitherWrite {
	Legacy(WsTxMessage),
	Http(HttpWsTxMessage),
}

/// The next frame from the writer queues. When both hold one, `http_first` picks the queue, and
/// the writer flips it after each frame so the two take turns. Storage requests can arrive
/// faster than the engine reads them, so a fixed order would hold every HTTP response until
/// storage traffic stops.
async fn next_queued_write(
	ws_rx: &mut mpsc::UnboundedReceiver<WsTxMessage>,
	http_ws_rx: &mut mpsc::Receiver<HttpWsTxMessage>,
	http_first: bool,
) -> Option<EitherWrite> {
	if http_first {
		// A closed HTTP queue never ends the writer ahead of a queued close frame.
		tokio::select! {
			biased;
			Some(msg) = http_ws_rx.recv() => Some(EitherWrite::Http(msg)),
			msg = ws_rx.recv() => msg.map(EitherWrite::Legacy),
		}
	} else {
		tokio::select! {
			biased;
			msg = ws_rx.recv() => msg.map(EitherWrite::Legacy),
			msg = http_ws_rx.recv() => msg.map(EitherWrite::Http),
		}
	}
}

fn extract_host(url: &str) -> String {
	url.replace("ws://", "")
		.replace("wss://", "")
		.split('/')
		.next()
		.unwrap_or("localhost")
		.to_string()
}
