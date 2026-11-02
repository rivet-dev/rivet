use anyhow::Context;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

/// Configuration for the private API service.
/// This port is plaintext and unauthenticated. It must only be reachable on a private network.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiPeer {
	/// Bind IP. Defaults to the host of advertise_address, or loopback for single-node development.
	/// DNS advertisements require an explicit bind IP. Explicit wildcard binds expose every interface.
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
	/// Address reachable by other nodes. Defaults to a concrete bind address. Wildcard binds require an explicit advertisement.
	/// Explicit loopback is valid only when all participating workers share the same host.
	pub advertise_address: Option<String>,
	#[serde(default)]
	pub volumes: VolumesConfig,
}

impl ApiPeer {
	pub fn host(&self) -> anyhow::Result<IpAddr> {
		if let Some(host) = self.host {
			return Ok(host);
		}
		let Some(address) = &self.advertise_address else {
			return Ok(crate::defaults::hosts::API_PEER);
		};
		let authority = address
			.parse::<http::uri::Authority>()
			.context("api_peer.advertise_address must be a host:port authority")?;
		authority.host().trim_start_matches('[').trim_end_matches(']')
			.parse()
			.context("api_peer.advertise_address must use an IP address when api_peer.host is unset; set api_peer.host explicitly for a DNS advertisement")
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::API_PEER)
	}

	/// Returns the advertised URL after startup configuration validation.
	pub fn advertised_url(&self) -> String {
		format!("http://{}", self.advertised_address())
	}

	fn advertised_address(&self) -> String {
		self.advertise_address.clone().unwrap_or_else(|| {
			SocketAddr::new(
				self.host.unwrap_or(crate::defaults::hosts::API_PEER),
				self.port(),
			)
			.to_string()
		})
	}

	pub fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.advertise_address.is_some() || !self.host()?.is_unspecified(),
			"api_peer.advertise_address is required when binding a wildcard address"
		);

		let address = self.advertised_address();
		let authority = address
			.parse::<http::uri::Authority>()
			.context("api_peer.advertise_address must be a host:port authority")?;
		anyhow::ensure!(
			authority.port_u16().is_some_and(|port| port > 0),
			"api_peer.advertise_address must include a nonzero port"
		);

		let url = url::Url::parse(&self.advertised_url())
			.context("invalid api_peer.advertise_address")?;
		anyhow::ensure!(
			url.host_str().is_some()
				&& url.username().is_empty()
				&& url.password().is_none()
				&& url.path() == "/"
				&& url.query().is_none()
				&& url.fragment().is_none(),
			"api_peer.advertise_address must be a host:port authority"
		);
		anyhow::ensure!(
			!matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_unspecified())
				&& !matches!(url.host(), Some(url::Host::Ipv6(ip)) if ip.is_unspecified()),
			"api_peer.advertise_address must not be an unspecified address"
		);

		self.host()?;
		self.volumes.validate()?;
		Ok(())
	}
}

/// Bounds for internal volume reads served on the API-peer listener.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct VolumesConfig {
	pub request_timeout_ms: u64,
	pub max_requests: usize,
	pub directory_page_size: u32,
	pub max_path_bytes: usize,
}
impl Default for VolumesConfig {
	fn default() -> Self {
		Self {
			request_timeout_ms: 10_000,
			max_requests: 128,
			directory_page_size: 256,
			max_path_bytes: 4096,
		}
	}
}
impl VolumesConfig {
	pub fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.request_timeout_ms > 0
				&& self.max_requests > 0
				&& (1..=1024).contains(&self.directory_page_size)
				&& (1..=65_536).contains(&self.max_path_bytes),
			"invalid API-peer volume bounds"
		);
		Ok(())
	}
}
