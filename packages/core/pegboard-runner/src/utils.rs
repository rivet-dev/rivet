use gas::prelude::*;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_util::rt::TokioIo;
use rivet_error::*;
use rivet_runner_protocol as protocol;
use tokio_tungstenite::{
	WebSocketStream,
	tungstenite::protocol::frame::{CloseFrame, coding::CloseCode},
};

#[derive(Clone)]
pub struct UrlData {
	pub protocol_version: u16,
	pub namespace: String,
	pub runner_key: String,
}

impl UrlData {
	pub fn parse_url(url: url::Url) -> Result<UrlData> {
		// Read protocol version from query parameters (required)
		let protocol_version = url
			.query_pairs()
			.find_map(|(n, v)| (n == "protocol_version").then_some(v))
			.context("missing `protocol_version` query parameter")?
			.parse::<u16>()
			.context("invalid `protocol_version` query parameter")?;

		// Read namespace from query parameters
		let namespace = url
			.query_pairs()
			.find_map(|(n, v)| (n == "namespace").then_some(v))
			.context("missing `namespace` query parameter")?
			.to_string();

		// Read runner key from query parameters (required)
		let runner_key = url
			.query_pairs()
			.find_map(|(n, v)| (n == "runner_key").then_some(v))
			.context("missing `runner_key` query parameter")?
			.to_string();

		Ok(UrlData {
			protocol_version,
			namespace,
			runner_key,
		})
	}
}

/// Determines if a given message kind will terminate the request.
pub fn is_to_server_tunnel_message_kind_request_close(
	kind: &protocol::ToServerTunnelMessageKind,
) -> bool {
	match kind {
		// HTTP terminal states
		protocol::ToServerTunnelMessageKind::ToServerResponseStart(resp) => !resp.stream,
		protocol::ToServerTunnelMessageKind::ToServerResponseChunk(chunk) => chunk.finish,
		protocol::ToServerTunnelMessageKind::ToServerResponseAbort => true,
		// WebSocket terminal states (either side closes)
		protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(_) => true,
		_ => false,
	}
}

/// Determines if a given message kind will terminate the request.
pub fn is_to_client_tunnel_message_kind_request_close(
	kind: &protocol::ToClientTunnelMessageKind,
) -> bool {
	match kind {
		// WebSocket terminal states (either side closes)
		protocol::ToClientTunnelMessageKind::ToClientWebSocketClose(_) => true,
		_ => false,
	}
}
