use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Configuration for the public API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiPublic {
	/// The ip on which the API service listens.
	pub host: Option<IpAddr>,
	/// The host on which the API service is accessible to Guard.
	pub lan_host: Option<String>,
	/// The port on which the API service listens.
	pub port: Option<u16>,
	/// Flag to enable verbose error reporting.
	pub verbose_errors: Option<bool>,
	/// Flag to respect the X-Forwarded-For header for client IP addresses.
	///
	/// Will be ignored in favor of CF-Connecting-IP if DNS provider is
	/// configured as Cloudflare.
	pub respect_forwarded_for: Option<bool>,
}

impl ApiPublic {
	pub fn lan_host(&self) -> &str {
		self.lan_host
			.as_deref()
			.unwrap_or(crate::defaults::hosts::API_PUBLIC_LAN)
	}

	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::API_PUBLIC)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::API_PUBLIC)
	}

	pub fn verbose_errors(&self) -> bool {
		self.verbose_errors.unwrap_or(true)
	}

	pub fn respect_forwarded_for(&self) -> bool {
		self.respect_forwarded_for.unwrap_or(false)
	}
}
