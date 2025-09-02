use std::net::IpAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The gateway service that proxies WebSocket connections to pegboard.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PegboardGateway {
	/// The host on which the gateway service listens.
	pub host: Option<IpAddr>,
	/// The host on which the gateway service is accessible to Guard.
	pub lan_host: Option<String>,
	/// The port on which the gateway service listens.
	pub port: Option<u16>,
}

impl PegboardGateway {
	pub fn lan_host(&self) -> &str {
		self.lan_host
			.as_deref()
			.unwrap_or(crate::defaults::hosts::PEGBOARD_GATEWAY_LAN)
	}

	pub fn host(&self) -> IpAddr {
		self.host
			.unwrap_or(crate::defaults::hosts::PEGBOARD_GATEWAY)
	}

	pub fn port(&self) -> u16 {
		self.port
			.unwrap_or(crate::defaults::ports::PEGBOARD_GATEWAY)
	}
}
