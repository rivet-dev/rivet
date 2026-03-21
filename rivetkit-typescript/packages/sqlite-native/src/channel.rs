//! WebSocket KV channel client.
//!
//! Manages a persistent WebSocket connection to the KV channel endpoint,
//! sends requests with correlation IDs, and handles reconnection with
//! exponential backoff.
//!
//! One channel per process, shared across all actors.
//! See `docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md` for the full spec.
//!
//! End-to-end tests are in the driver-test-suite
//! (`rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::time;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::{
	decode_to_client, encode_to_server, ErrorResponse, RequestData, ResponseData, ToClient,
	ToServer, ToServerPong, ToServerRequest,
};

// MARK: Constants

/// Timeout for individual KV operations in milliseconds.
/// Matches KV_EXPIRE in engine/sdks/typescript/runner/src/mod.ts.
const KV_EXPIRE_MS: u64 = 30_000;

/// Initial reconnect delay in milliseconds.
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Maximum reconnect delay in milliseconds.
const MAX_BACKOFF_MS: u64 = 30_000;

/// Backoff multiplier (exponential).
const BACKOFF_MULTIPLIER: f64 = 2.0;

/// Maximum jitter fraction added to each backoff delay (0-25%).
const JITTER_MAX: f64 = 0.25;

/// KV channel protocol version sent in the connection URL.
const PROTOCOL_VERSION: u32 = 1;

// MARK: Error

/// Errors returned by KV channel operations.
#[derive(Debug)]
pub enum ChannelError {
	/// The WebSocket connection is not established.
	ConnectionClosed,
	/// The operation timed out (KV_EXPIRE exceeded).
	Timeout,
	/// Protocol serialization/deserialization error.
	Protocol(String),
	/// WebSocket transport error.
	WebSocket(String),
	/// Server returned an error response.
	ServerError(ErrorResponse),
	/// The channel has been shut down.
	Shutdown,
}

impl fmt::Display for ChannelError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ConnectionClosed => write!(f, "kv channel connection closed"),
			Self::Timeout => write!(f, "kv channel operation timed out"),
			Self::Protocol(msg) => write!(f, "kv channel protocol error: {msg}"),
			Self::WebSocket(msg) => write!(f, "kv channel websocket error: {msg}"),
			Self::ServerError(e) => {
				write!(f, "kv channel server error: {} - {}", e.code, e.message)
			}
			Self::Shutdown => write!(f, "kv channel shut down"),
		}
	}
}

impl std::error::Error for ChannelError {}

// MARK: Config

/// Configuration for connecting to the KV channel endpoint.
#[derive(Debug, Clone)]
pub struct KvChannelConfig {
	/// Base WebSocket endpoint URL (e.g., "ws://localhost:6420").
	pub url: String,
	/// Authentication token. Engine uses admin_token, manager uses config.token.
	pub token: Option<String>,
	/// Namespace for actor scoping.
	pub namespace: String,
}

// MARK: KvChannel

/// A persistent WebSocket connection to the KV channel server.
///
/// One channel per process, shared across all actors. Handles reconnection
/// with exponential backoff and re-opens actors after reconnect.
pub struct KvChannel {
	inner: Arc<Inner>,
}

struct Inner {
	config: KvChannelConfig,

	/// Sender for outgoing WebSocket binary frames. None when disconnected.
	outgoing_tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,

	/// In-flight requests awaiting responses, keyed by requestId.
	in_flight: Mutex<HashMap<u32, oneshot::Sender<Result<ResponseData, ChannelError>>>>,

	/// Next requestId to allocate. Resets to 0 on reconnect.
	next_request_id: Mutex<u32>,

	/// Actor IDs that are currently open. Re-opened on reconnect.
	open_actors: Mutex<HashSet<String>>,

	/// Actors pending re-open on reconnect. KV requests for these actors
	/// wait until the watch value becomes true (ActorOpenResponse received).
	/// Empty during initial connection (optimistic open).
	reconnect_ready: Mutex<HashMap<String, watch::Sender<bool>>>,

	/// Request IDs of reconnect ActorOpenRequests. Maps request_id -> actor_id
	/// so the response handler can mark actors as ready.
	reconnect_request_ids: Mutex<HashMap<u32, String>>,

	/// Signal to shut down background tasks.
	shutdown_tx: watch::Sender<bool>,
}

impl KvChannel {
	/// Create a new KV channel and spawn the background connection loop.
	///
	/// The initial WebSocket connection is established asynchronously in the
	/// background. KV operations fail with `ConnectionClosed` until connected.
	pub fn connect(config: KvChannelConfig) -> Self {
		let (shutdown_tx, shutdown_rx) = watch::channel(false);

		let inner = Arc::new(Inner {
			config,
			outgoing_tx: Mutex::new(None),
			in_flight: Mutex::new(HashMap::new()),
			next_request_id: Mutex::new(0),
			open_actors: Mutex::new(HashSet::new()),
			reconnect_ready: Mutex::new(HashMap::new()),
			reconnect_request_ids: Mutex::new(HashMap::new()),
			shutdown_tx,
		});

		let inner_clone = inner.clone();
		tokio::spawn(async move {
			connection_loop(inner_clone, shutdown_rx).await;
		});

		KvChannel { inner }
	}

	/// Send a request and wait for the correlated response.
	///
	/// Times out after KV_EXPIRE (30 seconds).
	pub async fn send_request(
		&self,
		actor_id: &str,
		data: RequestData,
	) -> Result<ResponseData, ChannelError> {
		if *self.inner.shutdown_tx.borrow() {
			return Err(ChannelError::Shutdown);
		}

		// On reconnect, wait for ActorOpenResponse before sending KV requests.
		// The initial open (first connection) has no reconnect_ready entries,
		// so this is a no-op. See docs-internal/engine/NATIVE_SQLITE_REVIEW_FINDINGS.md
		// Finding 4 'Client-side change' section.
		let pending_rx = {
			let ready = self.inner.reconnect_ready.lock().await;
			ready.get(actor_id).map(|tx| tx.subscribe())
		};
		if let Some(mut rx) = pending_rx {
			match rx.wait_for(|v| *v).await {
				Ok(_) => {}
				Err(_) => return Err(ChannelError::ConnectionClosed),
			}
		}

		let (resp_tx, resp_rx) = oneshot::channel();

		// Allocate a request ID.
		let request_id = {
			let mut id = self.inner.next_request_id.lock().await;
			let rid = *id;
			*id = rid.wrapping_add(1);
			rid
		};

		// Serialize the message.
		let msg = ToServer::ToServerRequest(ToServerRequest {
			request_id,
			actor_id: actor_id.to_string(),
			data,
		});
		let bytes =
			encode_to_server(&msg).map_err(|e| ChannelError::Protocol(e.to_string()))?;

		// Register in-flight before sending to avoid response racing ahead.
		self.inner
			.in_flight
			.lock()
			.await
			.insert(request_id, resp_tx);

		// Send via WebSocket. If not connected, fail immediately.
		let send_result = {
			let tx_guard = self.inner.outgoing_tx.lock().await;
			match tx_guard.as_ref() {
				Some(tx) => tx.send(bytes).map_err(|_| ChannelError::ConnectionClosed),
				None => Err(ChannelError::ConnectionClosed),
			}
		};

		if let Err(e) = send_result {
			self.inner.in_flight.lock().await.remove(&request_id);
			return Err(e);
		}

		// Wait for correlated response with timeout.
		match time::timeout(Duration::from_millis(KV_EXPIRE_MS), resp_rx).await {
			Ok(Ok(result)) => result,
			Ok(Err(_)) => Err(ChannelError::ConnectionClosed),
			Err(_) => {
				self.inner.in_flight.lock().await.remove(&request_id);
				Err(ChannelError::Timeout)
			}
		}
	}

	/// Open an actor, registering it for re-open on reconnect.
	pub async fn open_actor(&self, actor_id: &str) -> Result<ResponseData, ChannelError> {
		{
			let mut open = self.inner.open_actors.lock().await;
			open.insert(actor_id.to_string());
		}
		self.send_request(actor_id, RequestData::ActorOpenRequest)
			.await
	}

	/// Close an actor, removing it from the re-open set.
	pub async fn close_actor(&self, actor_id: &str) -> Result<ResponseData, ChannelError> {
		{
			let mut open = self.inner.open_actors.lock().await;
			open.remove(actor_id);
		}
		self.send_request(actor_id, RequestData::ActorCloseRequest)
			.await
	}

	/// Shut down the channel, closing the WebSocket and failing in-flight requests.
	pub async fn disconnect(&self) {
		let _ = self.inner.shutdown_tx.send(true);
		*self.inner.outgoing_tx.lock().await = None;
		fail_all_in_flight(&self.inner).await;
	}
}

// MARK: Connection Loop

/// Main background loop that manages the WebSocket connection lifecycle.
///
/// Connects to the server, runs read/write tasks, and reconnects with
/// exponential backoff on disconnect.
async fn connection_loop(inner: Arc<Inner>, mut shutdown_rx: watch::Receiver<bool>) {
	let mut attempt: u32 = 0;

	loop {
		if *shutdown_rx.borrow() {
			return;
		}

		let url = build_ws_url(&inner.config);

		match connect_async(&url).await {
			Ok((ws_stream, _)) => {
				// Reset backoff on successful connection.
				attempt = 0;

				let (ws_write, ws_read) = ws_stream.split();
				let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<Vec<u8>>();

				// Reset request ID counter and reconnect state.
				*inner.next_request_id.lock().await = 0;
				inner.reconnect_ready.lock().await.clear();
				inner.reconnect_request_ids.lock().await.clear();

				// Re-open all previously open actors. On reconnect, KV requests
				// wait for each ActorOpenResponse before proceeding. On initial
				// connection (actors empty), this is a no-op and open_actor
				// proceeds optimistically.
				// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FINDINGS.md Finding 4.
				let actors: Vec<String> =
					inner.open_actors.lock().await.iter().cloned().collect();
				let mut next_id = 0u32;
				{
					let mut ready = inner.reconnect_ready.lock().await;
					let mut req_ids = inner.reconnect_request_ids.lock().await;
					for actor_id in &actors {
						let (tx, _rx) = watch::channel(false);
						ready.insert(actor_id.clone(), tx);
						req_ids.insert(next_id, actor_id.clone());

						let msg = ToServer::ToServerRequest(ToServerRequest {
							request_id: next_id,
							actor_id: actor_id.clone(),
							data: RequestData::ActorOpenRequest,
						});
						if let Ok(bytes) = encode_to_server(&msg) {
							let _ = outgoing_tx.send(bytes);
						}
						next_id = next_id.wrapping_add(1);
					}
				}
				*inner.next_request_id.lock().await = next_id;

				// Store the outgoing sender so send_request can use it.
				*inner.outgoing_tx.lock().await = Some(outgoing_tx);

				// Run read/write tasks until disconnect.
				run_connection(
					inner.clone(),
					ws_read,
					ws_write,
					outgoing_rx,
					&mut shutdown_rx,
				)
				.await;

				// Connection ended. Clear sender and fail in-flight requests.
				*inner.outgoing_tx.lock().await = None;
				fail_all_in_flight(&inner).await;
			}
			Err(e) => {
				tracing::warn!(%e, "kv channel connection failed");
			}
		}

		if *shutdown_rx.borrow() {
			return;
		}

		// Exponential backoff before next reconnect attempt.
		let delay = calculate_backoff(attempt);
		attempt = attempt.saturating_add(1);

		tokio::select! {
			_ = time::sleep(delay) => {}
			_ = shutdown_rx.changed() => { return; }
		}
	}
}

/// Run the read and write tasks for an active WebSocket connection.
///
/// Returns when the connection is lost or a shutdown signal is received.
async fn run_connection<S, W>(
	inner: Arc<Inner>,
	mut ws_read: S,
	mut ws_write: W,
	mut outgoing_rx: mpsc::UnboundedReceiver<Vec<u8>>,
	shutdown_rx: &mut watch::Receiver<bool>,
) where
	S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin + Send,
	W: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin + Send + 'static,
{
	// Write task: forward outgoing messages from the mpsc channel to the WebSocket.
	let write_shutdown_tx = inner.shutdown_tx.clone();
	let write_task = tokio::spawn(async move {
		let mut write_shutdown_rx = write_shutdown_tx.subscribe();
		loop {
			tokio::select! {
				msg = outgoing_rx.recv() => {
					match msg {
						Some(bytes) => {
							if ws_write
								.send(Message::Binary(bytes.into()))
								.await
								.is_err()
							{
								return;
							}
						}
						None => return,
					}
				}
				_ = write_shutdown_rx.changed() => { return; }
			}
		}
	});

	// Read loop: dispatch responses, handle pings, and detect close.
	loop {
		tokio::select! {
			msg = ws_read.next() => {
				match msg {
					Some(Ok(Message::Binary(data))) => {
						match decode_to_client(&data) {
							Ok(ToClient::ToClientResponse(response)) => {
								// Check if this is a reconnect ActorOpenResponse.
								let reconnect_actor = {
									inner
										.reconnect_request_ids
										.lock()
										.await
										.remove(&response.request_id)
								};

								if let Some(actor_id) = reconnect_actor {
									match response.data {
										ResponseData::ActorOpenResponse => {
											// Mark actor as ready for KV requests.
											if let Some(tx) = inner
												.reconnect_ready
												.lock()
												.await
												.remove(&actor_id)
											{
												let _ = tx.send(true);
											}
										}
										ResponseData::ErrorResponse(err) => {
											// Re-open failed. Remove actor and drop
											// the watch sender so waiters get
											// RecvError -> ConnectionClosed.
											inner
												.open_actors
												.lock()
												.await
												.remove(&actor_id);
											inner
												.reconnect_ready
												.lock()
												.await
												.remove(&actor_id);
											tracing::warn!(
												%actor_id,
												code = %err.code,
												message = %err.message,
												"kv channel reconnect open failed"
											);
										}
										_ => {
											inner
												.reconnect_ready
												.lock()
												.await
												.remove(&actor_id);
										}
									}
								} else {
									let mut in_flight =
										inner.in_flight.lock().await;
									if let Some(tx) =
										in_flight.remove(&response.request_id)
									{
										let result = match response.data {
											ResponseData::ErrorResponse(err) => {
												Err(ChannelError::ServerError(
													err,
												))
											}
											data => Ok(data),
										};
										let _ = tx.send(result);
									}
									// Ignore responses for unknown request IDs.
								}
							}
							Ok(ToClient::ToClientPing(ping)) => {
								// Respond with pong echoing the timestamp.
								let pong =
									ToServer::ToServerPong(ToServerPong { ts: ping.ts });
								if let Ok(bytes) = encode_to_server(&pong) {
									let tx_guard = inner.outgoing_tx.lock().await;
									if let Some(tx) = tx_guard.as_ref() {
										let _ = tx.send(bytes);
									}
								}
							}
							Ok(ToClient::ToClientClose) => {
								// Server requested close. Break to trigger reconnect.
								break;
							}
							Err(e) => {
								tracing::warn!(%e, "kv channel failed to decode message");
							}
						}
					}
					Some(Ok(Message::Close(_))) | None => {
						// Connection closed by server or stream ended.
						break;
					}
					Some(Ok(_)) => {
						// Ignore text, ping/pong frames. Tungstenite handles
						// WebSocket-level ping/pong automatically.
					}
					Some(Err(_)) => {
						// Read error. Connection is broken.
						break;
					}
				}
			}
			_ = shutdown_rx.changed() => { break; }
		}
	}

	write_task.abort();
}

// MARK: Helpers

/// Build the full WebSocket URL with query parameters.
fn build_ws_url(config: &KvChannelConfig) -> String {
	let base = config.url.trim_end_matches('/');
	let ns_encoded = urlencoding::encode(&config.namespace);
	let mut url = format!(
		"{base}/kv/connect?namespace={ns_encoded}&protocol_version={PROTOCOL_VERSION}",
	);
	if let Some(ref token) = config.token {
		let token_encoded = urlencoding::encode(token);
		url.push_str(&format!("&token={token_encoded}"));
	}
	url
}

/// Calculate exponential backoff delay with jitter.
///
/// Matches the runner protocol reconnect strategy from
/// engine/sdks/typescript/runner/src/utils.ts.
fn calculate_backoff(attempt: u32) -> Duration {
	let delay = INITIAL_BACKOFF_MS as f64 * BACKOFF_MULTIPLIER.powi(attempt as i32);
	let delay = delay.min(MAX_BACKOFF_MS as f64);

	// Add 0-25% jitter using nanosecond-based pseudo-random value.
	let nanos = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.subsec_nanos();
	let jitter_frac = (nanos as f64 / u32::MAX as f64) * JITTER_MAX;
	let delay_with_jitter = delay * (1.0 + jitter_frac);

	Duration::from_millis(delay_with_jitter as u64)
}

/// Fail all in-flight requests with a connection closed error.
async fn fail_all_in_flight(inner: &Inner) {
	let mut in_flight = inner.in_flight.lock().await;
	for (_, tx) in in_flight.drain() {
		let _ = tx.send(Err(ChannelError::ConnectionClosed));
	}
	// Clear reconnect state. Dropping watch senders wakes waiters with
	// RecvError, which send_request maps to ConnectionClosed.
	inner.reconnect_ready.lock().await.clear();
	inner.reconnect_request_ids.lock().await.clear();
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn build_ws_url_with_token() {
		let config = KvChannelConfig {
			url: "ws://localhost:6420".into(),
			token: Some("secret123".into()),
			namespace: "test-ns".into(),
		};
		let url = build_ws_url(&config);
		assert_eq!(
			url,
			"ws://localhost:6420/kv/connect?namespace=test-ns&protocol_version=1&token=secret123"
		);
	}

	#[test]
	fn build_ws_url_without_token() {
		let config = KvChannelConfig {
			url: "ws://localhost:6420".into(),
			token: None,
			namespace: "my-ns".into(),
		};
		let url = build_ws_url(&config);
		assert_eq!(
			url,
			"ws://localhost:6420/kv/connect?namespace=my-ns&protocol_version=1"
		);
	}

	#[test]
	fn build_ws_url_strips_trailing_slash() {
		let config = KvChannelConfig {
			url: "ws://example.com/".into(),
			token: None,
			namespace: "ns".into(),
		};
		let url = build_ws_url(&config);
		assert!(url.starts_with("ws://example.com/kv/connect?"));
	}

	#[test]
	fn backoff_attempt_zero() {
		let delay = calculate_backoff(0);
		// Initial delay is 1000ms with 0-25% jitter -> 1000..1250ms.
		assert!(delay.as_millis() >= 1000);
		assert!(delay.as_millis() <= 1250);
	}

	#[test]
	fn backoff_attempt_one() {
		let delay = calculate_backoff(1);
		// 1000 * 2^1 = 2000ms with jitter -> 2000..2500ms.
		assert!(delay.as_millis() >= 2000);
		assert!(delay.as_millis() <= 2500);
	}

	#[test]
	fn backoff_attempt_two() {
		let delay = calculate_backoff(2);
		// 1000 * 2^2 = 4000ms with jitter -> 4000..5000ms.
		assert!(delay.as_millis() >= 4000);
		assert!(delay.as_millis() <= 5000);
	}

	#[test]
	fn backoff_caps_at_max() {
		let delay = calculate_backoff(100);
		// Capped at 30000ms with 0-25% jitter -> 30000..37500ms.
		assert!(delay.as_millis() >= 30000);
		assert!(delay.as_millis() <= 37500);
	}

	#[test]
	fn backoff_progression() {
		// Verify delay increases with attempt number (ignoring jitter variance).
		let d0_base = 1000u128;
		let d5_base = 32000u128;
		let d0 = calculate_backoff(0).as_millis();
		let d5 = calculate_backoff(5).as_millis();
		// d0 is in [1000, 1250], d5 is in [30000, 37500] (capped at 30s).
		assert!(d0 >= d0_base);
		assert!(d5 >= d5_base.min(30000));
	}

	#[test]
	fn channel_error_display() {
		assert_eq!(
			ChannelError::ConnectionClosed.to_string(),
			"kv channel connection closed"
		);
		assert_eq!(
			ChannelError::Timeout.to_string(),
			"kv channel operation timed out"
		);
		assert_eq!(
			ChannelError::Shutdown.to_string(),
			"kv channel shut down"
		);
		assert_eq!(
			ChannelError::Protocol("bad data".into()).to_string(),
			"kv channel protocol error: bad data"
		);
		assert_eq!(
			ChannelError::WebSocket("connect failed".into()).to_string(),
			"kv channel websocket error: connect failed"
		);
		assert_eq!(
			ChannelError::ServerError(ErrorResponse {
				code: "actor_locked".into(),
				message: "locked by another connection".into(),
			})
			.to_string(),
			"kv channel server error: actor_locked - locked by another connection"
		);
	}

	#[test]
	fn build_ws_url_encodes_special_chars() {
		let config = KvChannelConfig {
			url: "ws://localhost:6420".into(),
			token: Some("tok&en=val?ue#frag".into()),
			namespace: "ns with spaces&special".into(),
		};
		let url = build_ws_url(&config);
		assert_eq!(
			url,
			"ws://localhost:6420/kv/connect?namespace=ns%20with%20spaces%26special&protocol_version=1&token=tok%26en%3Dval%3Fue%23frag"
		);
	}

	#[test]
	fn protocol_version_is_one() {
		assert_eq!(PROTOCOL_VERSION, 1);
	}

	#[test]
	fn kv_expire_matches_spec() {
		assert_eq!(KV_EXPIRE_MS, 30_000);
	}

	#[test]
	fn backoff_constants_match_runner_protocol() {
		// These must match engine/sdks/typescript/runner/src/utils.ts.
		assert_eq!(INITIAL_BACKOFF_MS, 1000);
		assert_eq!(MAX_BACKOFF_MS, 30_000);
		assert!((BACKOFF_MULTIPLIER - 2.0).abs() < f64::EPSILON);
		assert!((JITTER_MAX - 0.25).abs() < f64::EPSILON);
	}
}
