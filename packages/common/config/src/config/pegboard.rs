use std::net::IpAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The service that manages runner ws connections.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pegboard {
	/// The host on which the ws service listens.
	pub host: Option<IpAddr>,
	/// The host on which the ws service is accessible to Guard.
	pub lan_host: Option<String>,
	/// The port on which the ws service listens.
	pub port: Option<u16>,
}

impl Pegboard {
	pub fn lan_host(&self) -> &str {
		self.lan_host
			.as_deref()
			.unwrap_or(crate::defaults::hosts::PEGBOARD_RUNNER_LAN)
	}

	pub fn host(&self) -> IpAddr {
		self.host
			.unwrap_or(crate::defaults::hosts::PEGBOARD_RUNNER_WS)
	}

	pub fn port(&self) -> u16 {
		self.port
			.unwrap_or(crate::defaults::ports::PEGBOARD_RUNNER_WS)
	}
}
