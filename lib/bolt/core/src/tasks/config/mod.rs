use anyhow::*;
use tokio::fs;
use toml_edit::value;

use crate::context;

mod generate;
mod generate_default_regions;

pub use generate::{generate, ConfigGenerator};
pub use generate_default_regions::generate_default_regions;

/// Updates the namespace in `Bolt.local.toml`.
pub async fn set_namespace(namespace: &str) -> Result<()> {
	let project_root = context::ProjectContextData::seek_project_root().await;
	let config_local_path = project_root.join("Bolt.local.toml");

	let mut doc = if config_local_path.exists() {
		let toml = fs::read_to_string(&config_local_path).await?;
		toml.parse::<toml_edit::Document>()?
	} else {
		toml_edit::Document::new()
	};

	// Edit config
	doc["namespace"] = value(namespace);

	// Write new config
	fs::write(&config_local_path, doc.to_string().as_bytes()).await?;

	rivet_term::status::info("Updated namespace in Bolt.local.toml", namespace);

	Ok(())
}
