use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

#[operation(name = "region-config-get")]
pub async fn handle(
	_ctx: OperationContext<region::config_get::Request>,
) -> GlobalResult<region::config_get::Response> {
	Ok(region::config_get::Response {
		regions: read().await,
	})
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Region {
	pub id: Uuid,
	pub provider: String,
	pub provider_region: String,
}

const READ_CONFIG_ONCE: tokio::sync::OnceCell<HashMap<String, region::config_get::Region>> =
	tokio::sync::OnceCell::const_new();

// TODO: Building the region config in to the binary is not clean. We should find a way to
// dynamically configure this. We can't use the env since the config is too large.
// This will be removed anyways once we have dynamically provisioned clusters.
pub async fn read() -> HashMap<String, region::config_get::Region> {
	READ_CONFIG_ONCE
		.get_or_init(|| async {
			// Read config
			let config_buf = fs::read("/etc/rivet/region_config.json")
				.await
				.expect("failed to read /region_config.json");
			let config = serde_json::from_slice::<HashMap<String, Region>>(config_buf.as_slice())
				.expect("invalid region config");

			// Convert to proto
			config
				.into_iter()
				.map(|(k, v)| {
					(
						k,
						region::config_get::Region {
							id: Some(v.id.into()),
							provider: v.provider,
							provider_region: v.provider_region,
						},
					)
				})
				.collect::<HashMap<String, _>>()
		})
		.await
		.clone()
}
