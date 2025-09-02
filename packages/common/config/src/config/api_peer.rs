use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Configuration for the private API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiPeer {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
}

impl ApiPeer {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::API_PEER)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::API_PEER)
	}
}
