use std::sync::atomic::Ordering;

use rivet_envoy_protocol as protocol;
#[cfg(any(
	feature = "native-transport",
	all(feature = "wasm-transport", target_arch = "wasm32")
))]
use rivet_util_serde::HashableMap;
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
use crate::utils::display_id;

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
	let mut prepopulate_map = HashableMap::new();
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

fn observe_ping_unhealthy_on_close(shared: &SharedContext) {
	let last_ping_ts = shared.last_ping_ts.load(Ordering::Acquire);
	if last_ping_ts == 0 {
		return;
	}

	let unhealthy_ms = crate::time::now_millis()
		.saturating_sub(last_ping_ts)
		.saturating_sub(crate::handle::EnvoyHandle::PING_HEALTHY_THRESHOLD_MS);
	if unhealthy_ms > 0 {
		METRICS
			.ping_unhealthy_seconds_total
			.inc_by(unhealthy_ms as f64 / 1_000.0);
	}
}

/// Send a message over the WebSocket. Returns true if the message could not be sent.
pub async fn ws_send(shared: &SharedContext, message: protocol::ToRivet) -> bool {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_rivet(&message), "sending message");
	}

	let is_pong = matches!(message, protocol::ToRivet::ToRivetPong(_));
	let (message_kind, gateway_id, request_id, message_index, inner_data_len) =
		to_rivet_message_meta(&message);

	let Some(tx) = shared.ws_tx.load().as_ref().map(|tx| (**tx).clone()) else {
		if let (Some(gateway_id), Some(request_id), Some(message_index)) =
			(gateway_id.as_ref(), request_id.as_ref(), message_index)
		{
			tracing::error!(
				message_kind,
				gateway_id = %display_id(gateway_id),
				request_id = %display_id(request_id),
				message_index,
				inner_data_len,
				"websocket not available for sending"
			);
		} else {
			tracing::error!(
				message_kind,
				inner_data_len,
				"websocket not available for sending"
			);
		}
		return true;
	};

	let encoded = crate::protocol::versioned::ToRivet::wrap_latest(message)
		.serialize(protocol::PROTOCOL_VERSION)
		.expect("failed to encode message");
	let payload_len = encoded.len();
	let _ = tx.send(WsTxMessage::Send {
		data: encoded,
		enqueue_ts: crate::time::now_millis(),
		is_pong,
		message_kind,
		gateway_id,
		request_id,
		message_index,
		inner_data_len,
	});
	shared.ws_tx_depth.fetch_add(1, Ordering::Release);
	METRICS.ws_tx_depth.inc();
	if let (Some(gateway_id), Some(request_id), Some(message_index)) =
		(gateway_id.as_ref(), request_id.as_ref(), message_index)
	{
		tracing::trace!(
			envoy_key = %shared.envoy_key,
			message_kind,
			gateway_id = %display_id(gateway_id),
			request_id = %display_id(request_id),
			message_index,
			inner_data_len,
			payload_len,
			"queued websocket message to engine"
		);
	} else {
		tracing::trace!(
			envoy_key = %shared.envoy_key,
			message_kind,
			inner_data_len,
			payload_len,
			"queued websocket message to engine"
		);
	}
	false
}

fn to_rivet_message_meta(
	message: &protocol::ToRivet,
) -> (
	&'static str,
	Option<protocol::GatewayId>,
	Option<protocol::RequestId>,
	Option<u16>,
	usize,
) {
	match message {
		protocol::ToRivet::ToRivetMetadata(_) => ("ToRivetMetadata", None, None, None, 0),
		protocol::ToRivet::ToRivetEvents(_) => ("ToRivetEvents", None, None, None, 0),
		protocol::ToRivet::ToRivetAckCommands(_) => ("ToRivetAckCommands", None, None, None, 0),
		protocol::ToRivet::ToRivetStopping => ("ToRivetStopping", None, None, None, 0),
		protocol::ToRivet::ToRivetPong(_) => ("ToRivetPong", None, None, None, 0),
		protocol::ToRivet::ToRivetKvRequest(_) => ("ToRivetKvRequest", None, None, None, 0),
		protocol::ToRivet::ToRivetTunnelMessage(msg) => (
			to_rivet_tunnel_message_kind_name(&msg.message_kind),
			Some(msg.message_id.gateway_id),
			Some(msg.message_id.request_id),
			Some(msg.message_id.message_index),
			to_rivet_tunnel_message_inner_data_len(&msg.message_kind),
		),
		protocol::ToRivet::ToRivetSqliteGetPagesRequest(_) => {
			("ToRivetSqliteGetPagesRequest", None, None, None, 0)
		}
		protocol::ToRivet::ToRivetSqliteCommitRequest(_) => {
			("ToRivetSqliteCommitRequest", None, None, None, 0)
		}
		protocol::ToRivet::ToRivetSqliteExecRequest(_) => {
			("ToRivetSqliteExecRequest", None, None, None, 0)
		}
		protocol::ToRivet::ToRivetSqliteExecuteRequest(_) => {
			("ToRivetSqliteExecuteRequest", None, None, None, 0)
		}
	}
}

fn to_rivet_tunnel_message_kind_name(
	kind: &protocol::ToRivetTunnelMessageKind,
) -> &'static str {
	match kind {
		protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(_) => "ToRivetResponseStart",
		protocol::ToRivetTunnelMessageKind::ToRivetResponseChunk(_) => "ToRivetResponseChunk",
		protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort => "ToRivetResponseAbort",
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_) => "ToRivetWebSocketOpen",
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(_) => {
			"ToRivetWebSocketMessage"
		}
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_) => {
			"ToRivetWebSocketMessageAck"
		}
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(_) => "ToRivetWebSocketClose",
	}
}

fn to_rivet_tunnel_message_inner_data_len(kind: &protocol::ToRivetTunnelMessageKind) -> usize {
	match kind {
		protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(msg) => {
			msg.body.as_ref().map_or(0, Vec::len)
		}
		protocol::ToRivetTunnelMessageKind::ToRivetResponseChunk(msg) => msg.body.len(),
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(msg) => msg.data.len(),
		protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(_) => 0,
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
