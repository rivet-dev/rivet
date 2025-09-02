use std::net::IpAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The tunnel service that forwards tunnel-protocol messages between NATS and WebSocket connections.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PegboardTunnel {
	/// The host on which the tunnel service listens.
	pub host: Option<IpAddr>,
	/// The host on which the tunnel service is accessible to runners.
	pub lan_host: Option<String>,
	/// The port on which the tunnel service listens.
	pub port: Option<u16>,
}

impl PegboardTunnel {
	pub fn lan_host(&self) -> &str {
		self.lan_host
			.as_deref()
			.unwrap_or(crate::defaults::hosts::PEGBOARD_TUNNEL_LAN)
	}

	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::PEGBOARD_TUNNEL)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::PEGBOARD_TUNNEL)
	}
}
