use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guard {
	/// Host for HTTP traffic
	pub host: Option<IpAddr>,
	/// Port for HTTP traffic
	pub port: Option<u16>,
	/// Enable & configure HTTPS
	pub https: Option<Https>,
	/// Route cache TTL in milliseconds.
	pub route_cache_ttl_ms: Option<u64>,
	/// Proxy state cache TTL in milliseconds.
	pub proxy_state_cache_ttl_ms: Option<u64>,
	/// Time to keep TCP connection open after WebSocket close, in milliseconds.
	pub websocket_close_linger_ms: Option<u64>,
	/// Max incoming WebSocket message size in bytes.
	pub websocket_max_message_size: Option<usize>,
	/// Max outgoing WebSocket message size in bytes.
	pub websocket_max_outgoing_message_size: Option<usize>,
	/// Max HTTP request body size in bytes (first line of defense).
	pub http_max_request_body_size: Option<usize>,
}

impl Guard {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::GUARD)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::GUARD)
	}

	pub fn route_cache_ttl_ms(&self) -> u64 {
		self.route_cache_ttl_ms.unwrap_or(10 * 60 * 1000) // 10 minutes
	}

	pub fn proxy_state_cache_ttl_ms(&self) -> u64 {
		self.proxy_state_cache_ttl_ms.unwrap_or(60 * 60 * 1000) // 1 hour
	}

	pub fn websocket_close_linger_ms(&self) -> u64 {
		self.websocket_close_linger_ms.unwrap_or(100)
	}

	pub fn websocket_max_message_size(&self) -> usize {
		self.websocket_max_message_size.unwrap_or(32 * 1024 * 1024) // 32 MiB
	}

	pub fn websocket_max_outgoing_message_size(&self) -> usize {
		self.websocket_max_outgoing_message_size
			.unwrap_or(32 * 1024 * 1024) // 32 MiB
	}

	pub fn http_max_request_body_size(&self) -> usize {
		self.http_max_request_body_size.unwrap_or(256 * 1024 * 1024) // 256 MiB
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Https {
	pub port: u16, // Port for HTTPS traffic
	pub tls: Tls,  // TLS configuration
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Tls {
	pub actor_cert_path: PathBuf,
	pub actor_key_path: PathBuf,
	pub api_cert_path: PathBuf,
	pub api_key_path: PathBuf,
}
