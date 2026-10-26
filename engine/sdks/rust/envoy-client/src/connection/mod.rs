use std::sync::Arc;
use std::sync::atomic::Ordering;

use rivet_envoy_protocol as protocol;
#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
use std::collections::HashMap;
use vbare::Versioned;

use crate::context::{HttpConnectionTx, HttpWsTxMessage, SharedContext, WsTxMessage};
#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
use crate::envoy::ToEnvoyMessage;
use crate::metrics::METRICS;
#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
use crate::stringify::stringify_to_envoy;
use crate::stringify::stringify_to_rivet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WsSendResult {
	Sent { session: u64 },
	Unavailable,
	StaleSession { current: Option<u64> },
}

pub(crate) const HTTP_WS_MESSAGE_CAPACITY: usize = 256;
pub(crate) const HTTP_WS_BYTE_CAPACITY: usize = 16 * 1024 * 1024;
/// Maximum detached terminal-frame sends waiting on one saturated HTTP session.
/// Frames beyond it are dropped, and the engine times out those requests.
pub(crate) const HTTP_WS_DETACHED_SEND_CAPACITY: usize = 1024;

/// Uses the last advertised threshold while disconnected. Before the first init, no actors
/// can be lost and the envoy loop uses the same 10 second fallback for its lost timer.
pub(crate) async fn reconnect_lost_threshold_ms(shared: &SharedContext) -> u64 {
	shared
		.protocol_metadata
		.lock()
		.await
		.as_ref()
		.map(|metadata| metadata.envoy_lost_threshold as u64)
		.unwrap_or(10_000)
}



/// Installs a session, or returns zero if shutdown has already started.
pub(crate) async fn install_connection_with_http(
	shared: &SharedContext,
	tx: tokio::sync::mpsc::UnboundedSender<WsTxMessage>,
	http_tx: tokio::sync::mpsc::Sender<HttpWsTxMessage>,
	http_byte_budget: Arc<tokio::sync::Semaphore>,
) -> u64 {
	let mut guard = shared.ws_tx.lock().await;
	let mut http_guard = shared.http_ws_tx.lock().await;
	if shared.shutting_down.load(Ordering::Acquire) {
		return 0;
	}
	record_contact_before_install(shared);
	let session = shared
		.next_connection_session
		.fetch_add(1, Ordering::AcqRel)
		.saturating_add(1);
	shared.connection_session.store(session, Ordering::Release);
	*guard = Some(tx);
	*http_guard = Some(HttpConnectionTx {
		session,
		tx: http_tx,
		byte_budget: http_byte_budget,
		detached_send_budget: Arc::new(tokio::sync::Semaphore::new(HTTP_WS_DETACHED_SEND_CAPACITY)),
	});
	METRICS
		.http_detached_sends_capacity
		.set(HTTP_WS_DETACHED_SEND_CAPACITY as i64);
	drop(http_guard);
	drop(guard);
	shared.connection_session_tx.send_replace(session);
	session
}

/// Records the envoy's last contact with the engine before the new connection can deliver a
/// ping or init. The engine pings a connection as soon as it opens, ahead of its init, so the
/// contact must be captured here; the envoy loop judges the silence later, when it handles the
/// init and would resume its actors. The earliest contact wins if connections are installed
/// again before an init is handled.
fn record_contact_before_install(shared: &SharedContext) {
	let last_contact_ms = shared.last_contact_ms.load(Ordering::Acquire);
	if last_contact_ms == 0 {
		return;
	}
	let _ = shared.contact_before_install_ms.try_update(
		Ordering::AcqRel,
		Ordering::Acquire,
		|pending| match pending {
			0 => Some(last_contact_ms),
			pending => Some(pending.min(last_contact_ms)),
		},
	);
}



pub(crate) async fn remove_connection_for_session(shared: &SharedContext, session: u64) {
	let mut guard = shared.ws_tx.lock().await;
	let mut http_guard = shared.http_ws_tx.lock().await;
	if shared.connection_session.load(Ordering::Acquire) != session {
		return;
	}
	*guard = None;
	*http_guard = None;
	shared.connection_session.store(0, Ordering::Release);
	drop(http_guard);
	drop(guard);
	shared.connection_session_tx.send_replace(0);
}

/// Sends a streaming HTTP frame on the exact WebSocket session that accepted its request start.
/// Success means the socket writer, not merely the local queue, accepted the frame.
pub(crate) async fn ws_send_http_for_session(
	shared: &SharedContext,
	message: protocol::ToRivet,
	expected_session: u64,
) -> WsSendResult {
	let (encoded, encoded_len, connection) =
		match prepare_http_send(shared, message, expected_session).await {
			Ok(prepared) => prepared,
			Err(result) => return result,
		};

	let Ok(byte_permit) = connection
		.byte_budget
		.clone()
		.acquire_many_owned(encoded_len)
		.await
	else {
		return WsSendResult::Unavailable;
	};
	let (written, written_rx) = tokio::sync::oneshot::channel();
	if connection
		.tx
		.send(HttpWsTxMessage {
			data: encoded,
			_byte_permit: byte_permit,
			written,
		})
		.await
		.is_err()
	{
		return WsSendResult::Unavailable;
	}

	match written_rx.await {
		Ok(Ok(())) => WsSendResult::Sent {
			session: expected_session,
		},
		Ok(Err(error)) => {
			tracing::debug!(?error, expected_session, "HTTP WebSocket write failed");
			WsSendResult::Unavailable
		}
		Err(_) => WsSendResult::Unavailable,
	}
}

/// Sends a fire-and-forget HTTP frame without waiting on the HTTP byte budget,
/// the bounded writer queue, or the socket write.
///
/// The envoy main loop uses this for terminal frames such as 503 responses and
/// aborts, so a saturated HTTP stream cannot stall KV, SQLite, and command
/// dispatch for every actor. When the queue has capacity the frame is enqueued
/// in order on its session. Otherwise a detached task performs the waiting send,
/// so that frame may be written after frames enqueued later. At most
/// [`HTTP_WS_DETACHED_SEND_CAPACITY`] detached sends wait per session; beyond
/// that the frame is dropped.
///
/// Callers only send frames for requests that have no live route on this
/// session, so no body chunks of the same stream are queued behind the byte
/// budget. Tokio semaphores also hand released permits to queued waiters
/// before `try_acquire`, so the fast path does not take budget from a queued
/// sender.
pub(crate) async fn ws_send_http_for_session_detached(
	shared: &Arc<SharedContext>,
	message: protocol::ToRivet,
	expected_session: u64,
) {
	let (encoded, encoded_len, connection) =
		match prepare_http_send(shared, message.clone(), expected_session).await {
			Ok(prepared) => prepared,
			Err(_) => return,
		};

	let byte_permit = match connection
		.byte_budget
		.clone()
		.try_acquire_many_owned(encoded_len)
	{
		Ok(byte_permit) => byte_permit,
		Err(tokio::sync::TryAcquireError::Closed) => return,
		Err(tokio::sync::TryAcquireError::NoPermits) => {
			spawn_waiting_http_send(shared, &connection, message, expected_session);
			return;
		}
	};
	let (written, _written_rx) = tokio::sync::oneshot::channel();
	match connection.tx.try_send(HttpWsTxMessage {
		data: encoded,
		_byte_permit: byte_permit,
		written,
	}) {
		Ok(()) => {}
		Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {}
		Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
			spawn_waiting_http_send(shared, &connection, message, expected_session);
		}
	}
}

/// Sends a terminal frame from a detached task bounded by the session's
/// detached send budget, or drops it when that budget is exhausted.
fn spawn_waiting_http_send(
	shared: &Arc<SharedContext>,
	connection: &HttpConnectionTx,
	message: protocol::ToRivet,
	expected_session: u64,
) {
	let permit = match connection.detached_send_budget.clone().try_acquire_owned() {
		Ok(permit) => permit,
		Err(_) => {
			METRICS.http_detached_send_dropped_total.inc();
			tracing::debug!(
				expected_session,
				capacity = HTTP_WS_DETACHED_SEND_CAPACITY,
				"detached HTTP send budget exhausted, dropping terminal frame"
			);
			return;
		}
	};
	if connection.detached_send_budget.available_permits() == 0 {
		tracing::warn!(
			expected_session,
			capacity = HTTP_WS_DETACHED_SEND_CAPACITY,
			"detached HTTP sends reached capacity; the HTTP stream to the engine is saturated and further terminal frames are dropped"
		);
	} else {
		tracing::debug!(
			expected_session,
			"HTTP WebSocket queue is full, sending terminal frame from a detached task"
		);
	}

	let shared = shared.clone();
	let inflight = rivet_metrics::GaugeGuardExt::inc_guard(&METRICS.http_detached_sends_inflight);
	crate::utils::spawn_detached(async move {
		let _ = ws_send_http_for_session(&shared, message, expected_session).await;
		drop(inflight);
		drop(permit);
	});
}

/// Encodes an HTTP frame and resolves the connection for its session.
///
/// The `http_ws_tx` guard is held only for the session check and is released
/// before returning, so callers never hold it across the budget or queue waits.
async fn prepare_http_send(
	shared: &SharedContext,
	message: protocol::ToRivet,
	expected_session: u64,
) -> Result<(Vec<u8>, u32, HttpConnectionTx), WsSendResult> {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(
			data = stringify_to_rivet(&message),
			"sending HTTP tunnel message"
		);
	}

	let encoded = match crate::protocol::versioned::ToRivet::serialize(
		&message,
		protocol::PROTOCOL_VERSION,
	) {
		Ok(encoded) => encoded,
		Err(error) => {
			tracing::error!(?error, "failed to encode HTTP tunnel message");
			return Err(WsSendResult::Unavailable);
		}
	};
	let Ok(encoded_len) = u32::try_from(encoded.len()) else {
		return Err(WsSendResult::Unavailable);
	};
	if encoded.len() > HTTP_WS_BYTE_CAPACITY {
		return Err(WsSendResult::Unavailable);
	}

	let guard = shared.http_ws_tx.lock().await;
	let current = shared.connection_session.load(Ordering::Acquire);
	if current != expected_session {
		return Err(WsSendResult::StaleSession {
			current: (current != 0).then_some(current),
		});
	}
	let Some(connection) = guard.as_ref() else {
		return Err(WsSendResult::Unavailable);
	};
	if connection.session != expected_session {
		return Err(WsSendResult::StaleSession {
			current: Some(connection.session),
		});
	}
	Ok((encoded, encoded_len, connection.clone()))
}

#[cfg(all(feature = "native-transport", feature = "wasm-transport"))]
compile_error!(
	"`native-transport` and `wasm-transport` are mutually exclusive. Enable exactly one envoy-client transport."
);

#[cfg(not(any(feature = "native-transport", feature = "wasm-transport")))]
compile_error!(
	"rivet-envoy-client requires a WebSocket transport. Enable `native-transport` or `wasm-transport`."
);

#[cfg(feature = "native-transport")]
mod native;
#[cfg(feature = "wasm-transport")]
mod wasm;

#[cfg(feature = "native-transport")]
pub use native::start_connection;
#[cfg(feature = "wasm-transport")]
pub use wasm::start_connection;

#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
async fn send_initial_metadata(shared: &SharedContext) {
	let mut prepopulate_map = HashMap::new();
	for (name, actor) in &shared.config.prepopulate_actor_names {
		prepopulate_map.insert(
			name.clone(),
			protocol::ActorName {
				metadata: (serde_json::to_string(&actor.metadata)
					.unwrap_or_else(|_| "{}".to_string()))
				.into(),
			},
		);
	}

	let metadata_json = shared
		.config
		.metadata
		.as_ref()
		.map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));

	ws_send(
		shared,
		protocol::ToRivet::ToRivetMetadata(protocol::ToRivetMetadata {
			prepopulate_actor_names: Some(
				prepopulate_map
					.into_iter()
					.map(|(k, v)| (k.into(), v))
					.collect(),
			),
			metadata: metadata_json.map(Into::into),
		}),
	)
	.await;
}

#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
async fn forward_to_envoy(shared: &SharedContext, session: u64, message: protocol::ToEnvoy) {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_envoy(&message), "received message");
	}

	match message {
		protocol::ToEnvoy::ToEnvoyInit(init) => {
			shared
				.last_contact_ms
				.store(crate::time::now_millis(), Ordering::Release);
			let _ = crate::envoy::send_to_envoy_tx(
				shared,
				ToEnvoyMessage::ConnMessage {
					message: protocol::ToEnvoy::ToEnvoyInit(init),
					session,
				},
			);
		}
		protocol::ToEnvoy::ToEnvoyPing(ping) => {
			let now = crate::time::now_millis();
			shared.last_ping_ts.store(now, Ordering::Release);
			shared.last_contact_ms.store(now, Ordering::Release);
			ws_send(
				shared,
				protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: ping.ts }),
			)
			.await;
		}
		other => {
			let _ = crate::envoy::send_to_envoy_tx(
				shared,
				ToEnvoyMessage::ConnMessage {
					message: other,
					session,
				},
			);
		}
	}
}

/// Records a ping and encodes its pong, for a transport that writes pongs ahead of its queue.
#[cfg(feature = "native-transport")]
fn encode_pong(shared: &SharedContext, ping: protocol::ToEnvoyPing) -> Option<Vec<u8>> {
	shared
		.last_ping_ts
		.store(crate::time::now_millis(), Ordering::Release);
	let pong = protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: ping.ts });
	match crate::protocol::versioned::ToRivet::serialize(&pong, protocol::PROTOCOL_VERSION) {
		Ok(encoded) => Some(encoded),
		Err(error) => {
			tracing::error!(?error, "failed to encode pong");
			None
		}
	}
}

/// Send a message over the WebSocket. Returns true if the message could not be sent.
pub async fn ws_send(shared: &SharedContext, message: protocol::ToRivet) -> bool {
	!matches!(
		ws_send_for_session(shared, message, None).await,
		WsSendResult::Sent { .. }
	)
}

/// Atomically checks a request's WebSocket affinity and admits it to the
/// writer. The session comparison deliberately happens while holding the same
/// mutex used by connect/disconnect. A coordinator-side check alone would have
/// a race where the old connection disappears immediately before the send and
/// the transaction statement is accidentally queued for its replacement.
pub(crate) async fn ws_send_for_session(
	shared: &SharedContext,
	message: protocol::ToRivet,
	expected_session: Option<u64>,
) -> WsSendResult {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_rivet(&message), "sending message");
	}

	let message_kind = to_rivet_kind(&message);
	let wait_start = crate::time::Instant::now();
	let guard = shared.ws_tx.lock().await;
	let wait_elapsed = wait_start.elapsed();
	METRICS
		.ws_tx_lock_wait_duration_seconds
		.with_label_values(&[message_kind])
		.observe(wait_elapsed.as_secs_f64());

	let hold_start = crate::time::Instant::now();
	let current = shared.connection_session.load(Ordering::Acquire);
	if let Some(expected) = expected_session
		&& current != expected
	{
		return WsSendResult::StaleSession {
			current: (current != 0).then_some(current),
		};
	}
	let Some(tx) = guard.as_ref() else {
		// Still observe hold duration on the early-return path.
		METRICS
			.ws_tx_lock_hold_duration_seconds
			.with_label_values(&[message_kind])
			.observe(hold_start.elapsed().as_secs_f64());
		tracing::debug!("websocket not available for sending");
		return WsSendResult::Unavailable;
	};

	let encoded = match crate::protocol::versioned::ToRivet::serialize(
		&message,
		protocol::PROTOCOL_VERSION,
	) {
		Ok(encoded) => encoded,
		Err(error) => {
			tracing::error!(?error, "failed to encode message");
			return WsSendResult::Unavailable;
		}
	};
	if tx.send(WsTxMessage::Send(encoded)).is_err() {
		return WsSendResult::Unavailable;
	}
	drop(guard);
	METRICS
		.ws_tx_lock_hold_duration_seconds
		.with_label_values(&[message_kind])
		.observe(hold_start.elapsed().as_secs_f64());
	WsSendResult::Sent { session: current }
}

/// Bounded label set for `ws_tx` send paths.
fn to_rivet_kind(message: &protocol::ToRivet) -> &'static str {
	match message {
		protocol::ToRivet::ToRivetMetadata(_) => "metadata",
		protocol::ToRivet::ToRivetEvents(_) => "events",
		protocol::ToRivet::ToRivetAckCommands(_) => "ack_commands",
		protocol::ToRivet::ToRivetStopping => "stopping",
		protocol::ToRivet::ToRivetPong(_) => "pong",
		protocol::ToRivet::ToRivetKvRequest(_) => "kv_request",
		protocol::ToRivet::ToRivetSqliteGetPagesRequest(_) => "sqlite_get_pages",
		protocol::ToRivet::ToRivetSqliteCommitRequest(_) => "sqlite_commit",
		protocol::ToRivet::ToRivetSqliteCommitStageBeginRequest(_) => "sqlite_commit_stage_begin",
		protocol::ToRivet::ToRivetSqliteCommitStageSegmentRequest(_) => {
			"sqlite_commit_stage_segment"
		}
		protocol::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => "sqlite_commit_finalize",
		protocol::ToRivet::ToRivetSqliteExecRequest(_) => "sqlite_exec",
		protocol::ToRivet::ToRivetSqliteExecuteRequest(_) => "sqlite_execute",
		protocol::ToRivet::ToRivetSqliteExecuteBatchRequest(_) => "sqlite_execute_batch",
		protocol::ToRivet::ToRivetTunnelMessage(_) => "tunnel_message",
	}
}

#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.


/// A missing or malformed optional diagnostic header leaves session attribution unknown.
#[cfg(all(not(target_arch = "wasm32"), any(feature = "native-transport", test)))]
fn sqlite_trace_connection(value: Option<&[u8]>) -> Option<uuid::Uuid> {
	value
		.filter(|value| value.len() == 36)
		.and_then(|value| uuid::Uuid::try_parse_ascii(value).ok())
}
