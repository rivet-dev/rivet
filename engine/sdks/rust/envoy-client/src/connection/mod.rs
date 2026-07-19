use std::sync::atomic::Ordering;

use rivet_envoy_protocol as protocol;
#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
use std::collections::HashMap;
use vbare::OwnedVersionedData;

use crate::context::SharedContext;
use crate::context::WsTxMessage;
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

pub(crate) async fn install_connection(
	shared: &SharedContext,
	tx: tokio::sync::mpsc::UnboundedSender<WsTxMessage>,
) -> u64 {
	let mut guard = shared.ws_tx.lock().await;
	let session = shared
		.next_connection_session
		.fetch_add(1, Ordering::AcqRel)
		.saturating_add(1);
	shared.connection_session.store(session, Ordering::Release);
	*guard = Some(tx);
	drop(guard);
	shared.connection_session_tx.send_replace(session);
	session
}

pub(crate) async fn remove_connection(shared: &SharedContext) {
	let mut guard = shared.ws_tx.lock().await;
	*guard = None;
	shared.connection_session.store(0, Ordering::Release);
	drop(guard);
	shared.connection_session_tx.send_replace(0);
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
				metadata: serde_json::to_string(&actor.metadata)
					.unwrap_or_else(|_| "{}".to_string()),
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
			prepopulate_actor_names: Some(prepopulate_map),
			metadata: metadata_json,
		}),
	)
	.await;
}

#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
async fn forward_to_envoy(shared: &SharedContext, message: protocol::ToEnvoy) {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_envoy(&message), "received message");
	}

	match message {
		protocol::ToEnvoy::ToEnvoyPing(ping) => {
			shared
				.last_ping_ts
				.store(crate::time::now_millis(), Ordering::Release);
			ws_send(
				shared,
				protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: ping.ts }),
			)
			.await;
		}
		other => {
			let _ = crate::envoy::send_to_envoy_tx(
				shared,
				ToEnvoyMessage::ConnMessage { message: other },
			);
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
		tracing::error!("websocket not available for sending");
		return WsSendResult::Unavailable;
	};

	let encoded = crate::protocol::versioned::ToRivet::wrap_latest(message)
		.serialize(protocol::PROTOCOL_VERSION)
		.expect("failed to encode message");
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
