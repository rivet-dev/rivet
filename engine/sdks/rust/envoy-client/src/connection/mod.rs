use rivet_envoy_protocol as protocol;
#[cfg(any(
	rivet_envoy_native_transport,
	all(rivet_envoy_wasm_transport, target_arch = "wasm32")
))]
use rivet_util_serde::HashableMap;
use vbare::OwnedVersionedData;

use crate::context::SharedContext;
use crate::context::WsTxMessage;
#[cfg(any(
	rivet_envoy_native_transport,
	all(rivet_envoy_wasm_transport, target_arch = "wasm32")
))]
use crate::envoy::ToEnvoyMessage;
#[cfg(any(
	rivet_envoy_native_transport,
	all(rivet_envoy_wasm_transport, target_arch = "wasm32")
))]
use crate::stringify::stringify_to_envoy;
use crate::stringify::stringify_to_rivet;

#[cfg(rivet_envoy_native_transport)]
mod native;
mod transport;
#[cfg(rivet_envoy_wasm_transport)]
mod wasm;

pub use transport::start_connection;

#[cfg(any(
	rivet_envoy_native_transport,
	all(rivet_envoy_wasm_transport, target_arch = "wasm32")
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
	rivet_envoy_native_transport,
	all(rivet_envoy_wasm_transport, target_arch = "wasm32")
))]
async fn forward_to_envoy(shared: &SharedContext, message: protocol::ToEnvoy) {
	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::debug!(data = stringify_to_envoy(&message), "received message");
	}

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
