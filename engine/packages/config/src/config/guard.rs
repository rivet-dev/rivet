use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guard {
	/// Host for HTTP traffic
	pub host: Option<IpAddr>,
	/// Port for HTTP traffic
	pub port: Option<u16>,
	/// Enables TCP_NODELAY on accepted Guard sockets. Defaults to true.
	///
	/// Guard carries actor nodes' WebSockets and client WebSocket and HTTP traffic,
	/// where most writes are small frames. With Nagle's algorithm on, a small frame waits for
	/// the peer to acknowledge the previous one, and a peer that delays its ACK holds it for
	/// about 40 ms. That delay landed on every actor's first response.
	pub tcp_nodelay: Option<bool>,
	/// Timeout for resolving compute routes in milliseconds.
	pub route_compute_timeout_ms: Option<u64>,
	/// Timeout for guard-owned route authorization checks in milliseconds.
	pub route_auth_check_timeout_ms: Option<u64>,
	/// Timeout for pegboard actor route authorization checks in milliseconds.
	pub route_pegboard_auth_check_timeout_ms: Option<u64>,

}

impl Guard {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::GUARD)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::GUARD)
	}

	pub fn tcp_nodelay(&self) -> bool {
		self.tcp_nodelay.unwrap_or(true)
	}

	pub fn route_compute_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_compute_timeout_ms.unwrap_or(10_000))
	}

	pub fn route_auth_check_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_auth_check_timeout_ms.unwrap_or(5_000))
	}

	pub fn route_pegboard_auth_check_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_pegboard_auth_check_timeout_ms.unwrap_or(5_000))
	}
}
