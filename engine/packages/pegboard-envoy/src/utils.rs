use gas::prelude::*;

use crate::errors::WsError;

/// Oldest envoy protocol version this engine still speaks.
const MIN_PROTOCOL_VERSION: u16 = 2;

#[derive(Clone)]
pub struct UrlData {
	pub protocol_version: u16,
	pub namespace: String,
	pub pool_name: String,
	pub envoy_key: String,
	pub version: u32,
}

impl UrlData {
	/// Parses the envoy handshake URL.
	///
	/// `max_protocol_version` is the version negotiated across the engine fleet, not this binary's
	/// compiled ceiling. An envoy accepted at a version an older pod cannot speak would break as
	/// soon as its actor commands landed on that pod.
	pub fn parse_url(url: url::Url, max_protocol_version: u16) -> Result<UrlData> {
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
		if protocol_version < MIN_PROTOCOL_VERSION || protocol_version > max_protocol_version {
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
		// The envoy indexes store versions with `encode_reverse_u32`, which only keeps
		// higher versions first up to `i32::MAX`.
		if version > i32::MAX as u32 {
			return Err(WsError::InvalidRequest(
				"`version` query parameter too large (> 2147483647)",
			)
			.build());
		}

		Ok(UrlData {
			protocol_version,
			namespace,
			pool_name,
			envoy_key,
			version,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_version(version: u64) -> Result<UrlData> {
		let url = url::Url::parse(&format!(
			"ws://localhost/?protocol_version={MIN_PROTOCOL_VERSION}&namespace=default&pool_name=default&envoy_key=envoy&version={version}"
		))
		.unwrap();
		UrlData::parse_url(url, MIN_PROTOCOL_VERSION)
	}

	#[test]
	fn version_must_fit_reverse_index_encoding() {
		assert_eq!(
			parse_version(i32::MAX as u64).unwrap().version,
			i32::MAX as u32
		);
		assert!(parse_version(i32::MAX as u64 + 1).is_err());
		assert!(parse_version(u32::MAX as u64).is_err());
	}
}
