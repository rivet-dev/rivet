use gas::prelude::*;

use crate::errors::WsError;

#[derive(Clone)]
pub struct UrlData {
	pub protocol_version: u16,
	pub namespace: String,
	pub pool_name: String,
	pub envoy_key: String,
	pub version: u32,
}

impl UrlData {
	pub fn parse_url(url: url::Url) -> Result<UrlData> {
		// Read protocol version from query parameters
		let protocol_version = url
			.query_pairs()
			.find_map(|(n, v)| (n == "protocol_version").then_some(v))
			.context("missing `protocol_version` query parameter")?
			.parse::<u16>()
			.context("invalid `protocol_version` query parameter")?;

		// Read namespace from query parameters
		let namespace = url
			.query_pairs()
			.find_map(|(n, v)| (n == "namespace").then_some(v))
			.context("missing `namespace` query parameter")?
			.to_string();

		// Read envoy pool name from query parameters
		let pool_name = url
			.query_pairs()
			.find_map(|(n, v)| (n == "pool_name").then_some(v))
			.context("missing `pool_name` query parameter")?
			.to_string();

		if pool_name.len() > 128 {
			return Err(
				WsError::InvalidInitialPacket("`pool_name` parameter too long (> 128)").build(),
			);
		}

		// Read envoy key from query parameters
		let envoy_key = url
			.query_pairs()
			.find_map(|(n, v)| (n == "envoy_key").then_some(v))
			.context("missing `envoy_key` query parameter")?
			.to_string();

		if envoy_key.len() < 3 {
			return Err(
				WsError::InvalidInitialPacket("`envoy_key` parameter too short (< 3)").build(),
			);
		}
		if envoy_key.len() > 128 {
			return Err(
				WsError::InvalidInitialPacket("`envoy_key` parameter too long (> 128)").build(),
			);
		}

		let version = url
			.query_pairs()
			.find_map(|(n, v)| (n == "version").then_some(v))
			.context("missing `version` query parameter")?
			.parse::<u32>()
			.context("invalid `version` query parameter")?;

		Ok(UrlData {
			protocol_version,
			namespace,
			pool_name,
			envoy_key,
			version,
		})
	}
}
