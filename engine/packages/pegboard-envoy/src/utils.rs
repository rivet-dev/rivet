use gas::prelude::*;
use rivet_envoy_protocol::PROTOCOL_VERSION;

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
			.ok_or_else(|| {
				WsError::InvalidRequest("missing `protocol_version` query parameter").build()
			})?
			.parse::<u16>()
			.context(
				WsError::InvalidRequest("invalid `protocol_version` query parameter").build(),
			)?;
		if protocol_version < 2 || protocol_version > PROTOCOL_VERSION {
			return Err(
				WsError::InvalidRequest("unsupported `protocol_version` query parameter").build(),
			);
		}

		// Read namespace from query parameters
		let namespace = url
			.query_pairs()
			.find_map(|(n, v)| (n == "namespace").then_some(v))
			.ok_or_else(|| WsError::InvalidRequest("missing `namespace` query parameter").build())?
			.to_string();

		// Read envoy pool name from query parameters
		let pool_name = url
			.query_pairs()
			.find_map(|(n, v)| (n == "pool_name").then_some(v))
			.ok_or_else(|| WsError::InvalidRequest("missing `pool_name` query parameter").build())?
			.to_string();

		if pool_name.len() > 128 {
			return Err(WsError::InvalidRequest("`pool_name` parameter too long (> 128)").build());
		}

		// Read envoy key from query parameters
		let envoy_key = url
			.query_pairs()
			.find_map(|(n, v)| (n == "envoy_key").then_some(v))
			.ok_or_else(|| WsError::InvalidRequest("missing `envoy_key` query parameter").build())?
			.to_string();

		if envoy_key.len() < 3 {
			return Err(WsError::InvalidRequest("`envoy_key` parameter too short (< 3)").build());
		}
		if envoy_key.len() > 128 {
			return Err(WsError::InvalidRequest("`envoy_key` parameter too long (> 128)").build());
		}
		if !util::check::ident_with_len(&envoy_key, true, 128) {
			return Err(WsError::InvalidRequest("`envoy_key` parameter invalid").build());
		}

		let version = url
			.query_pairs()
			.find_map(|(n, v)| (n == "version").then_some(v))
			.ok_or_else(|| WsError::InvalidRequest("missing `version` query parameter").build())?
			.parse::<u32>()
			.context(WsError::InvalidRequest("invalid `version` query parameter").build())?;

		Ok(UrlData {
			protocol_version,
			namespace,
			pool_name,
			envoy_key,
			version,
		})
	}
}
