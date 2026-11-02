//! Production reqwest builders with explicit redirect and destination policies.

use anyhow::Result;
use rivet_outbound_guard::Policy;
use std::sync::Arc;

const CLIENT_USER_AGENT: &str = concat!("RivetEngine/", env!("CARGO_PKG_VERSION"));

/// Trusted worker/peer destinations. Env owns the application's request deadline.
/// Redirects cannot change the selected worker identity.
pub fn peer_client() -> Result<Arc<dyn rivet_env::capabilities::http::HttpClient>> {
	peer_client_with(&Default::default())
}

/// [`peer_client`] with the configured peer connection sets.
pub fn peer_client_with(
	config: &rivet_config::config::pubsub::PeerTransport,
) -> Result<Arc<dyn rivet_env::capabilities::http::HttpClient>> {
	let defaults = rivet_env::backends::production::http::PeerTransport::default();
	let transport = rivet_env::backends::production::http::PeerTransport {
		request_connections: config
			.request_connections
			.unwrap_or(defaults.request_connections),
		stream_connections: config
			.stream_connections
			.unwrap_or(defaults.stream_connections),
		request_connection_window_bytes: config
			.request_connection_window_bytes
			.unwrap_or(defaults.request_connection_window_bytes),
		request_stream_window_bytes: config
			.request_stream_window_bytes
			.unwrap_or(defaults.request_stream_window_bytes),
		stream_connection_window_bytes: config
			.stream_connection_window_bytes
			.unwrap_or(defaults.stream_connection_window_bytes),
		stream_stream_window_bytes: config
			.stream_stream_window_bytes
			.unwrap_or(defaults.stream_stream_window_bytes),
	};
	Ok(Arc::new(
		rivet_env::backends::production::http::ReqwestClient::for_peers_with(
			CLIENT_USER_AGENT,
			&transport,
		)?,
	))
}

/// Build policy from this instance's configuration, without a process-global cache.
pub async fn outbound_policy(config: &rivet_config::Config) -> Result<Arc<Policy>> {
	Ok(Arc::new(Policy::from_config(config)?))
}
