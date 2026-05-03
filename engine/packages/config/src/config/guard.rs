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
	/// Enables TCP_NODELAY on accepted Guard sockets.
	pub tcp_nodelay: Option<bool>,
	/// Enables the internal websocket health route for debug and latency testing. This is intended
	/// for websocket ping/pong verification and should remain disabled in normal deployments.
	pub enable_websocket_health_route: Option<bool>,
	/// TTL for cached route lookups in milliseconds.
	pub route_cache_ttl_ms: Option<u64>,
	/// Timeout for route resolution in milliseconds.
	pub route_timeout_ms: Option<u64>,
	/// Timeout for waiting for an actor to become ready in milliseconds.
	pub actor_ready_timeout_ms: Option<u64>,
	/// Timeout sent with actor force-wake requests in milliseconds.
	pub actor_force_wake_pending_timeout_ms: Option<i64>,
	/// Enable & configure HTTPS
	pub https: Option<Https>,
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

	pub fn tcp_nodelay(&self) -> bool {
		self.tcp_nodelay.unwrap_or(false)
	}

	pub fn enable_websocket_health_route(&self) -> bool {
		self.enable_websocket_health_route.unwrap_or(false)
	}

	pub fn route_cache_ttl(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_cache_ttl_ms.unwrap_or(60 * 10 * 1000))
	}

	pub fn route_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_timeout_ms.unwrap_or(15_000))
	}

	pub fn actor_ready_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.actor_ready_timeout_ms.unwrap_or(10_000))
	}

	pub fn actor_force_wake_pending_timeout(&self) -> i64 {
		self.actor_force_wake_pending_timeout_ms
			.unwrap_or(60 * 1000)
	}

	pub fn http_max_request_body_size(&self) -> usize {
		self.http_max_request_body_size.unwrap_or(20 * 1024 * 1024) // 20 MiB
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
