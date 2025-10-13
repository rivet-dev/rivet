use gas::prelude::*;

#[derive(Clone)]
pub struct UrlData {
	pub protocol_version: u16,
	pub namespace: String,
	pub runner_key: String,
}

impl UrlData {
	pub fn parse_url(url: url::Url) -> Result<UrlData> {
		// Read protocol version from query parameters (required)
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

		// Read runner key from query parameters (required)
		let runner_key = url
			.query_pairs()
			.find_map(|(n, v)| (n == "runner_key").then_some(v))
			.context("missing `runner_key` query parameter")?
			.to_string();

		Ok(UrlData {
			protocol_version,
			namespace,
			runner_key,
		})
	}
}
