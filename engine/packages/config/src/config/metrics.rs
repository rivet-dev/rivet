use std::net::IpAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the metrics service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Metrics {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
}

impl Metrics {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::METRICS)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::METRICS)
	}
}
