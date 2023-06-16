use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
	pub id: Uuid,
	pub provider: String,
	pub provider_region: String,
	// pub vpc_subnet: String,
}

const READ_CONFIG_ONCE: tokio::sync::OnceCell<HashMap<String, Region>> =
	tokio::sync::OnceCell::const_new();

pub async fn read() -> HashMap<String, Region> {
	READ_CONFIG_ONCE
		.get_or_init(|| async {
			let task_dir = std::env::var("NOMAD_TASK_DIR").expect("NOMAD_TASK_DIR");
			let config_path = format!("{task_dir}/region-config.json");
			let config_buf = tokio::fs::read(config_path)
				.await
				.expect("failed to read region config");
			let config = serde_json::from_slice::<HashMap<String, Region>>(config_buf.as_slice())
				.expect("invalid region config");
			config
		})
		.await
		.clone()
}
