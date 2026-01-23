use std::collections::HashMap;

use anyhow::anyhow;
use gas::prelude::*;

use crate::ctx::ApiCtx;

// Re-export types from pegboard for API schema
pub use pegboard::ops::serverless_metadata::fetch::ServerlessMetadataError;

/// Serverless metadata returned from a runner.
#[derive(Debug, Clone)]
pub struct ServerlessMetadata {
	pub runtime: String,
	pub version: String,
	pub actor_names: HashMap<String, serde_json::Value>,
	pub runner_version: Option<u32>,
}

impl From<pegboard::ops::serverless_metadata::fetch::Output> for ServerlessMetadata {
	fn from(output: pegboard::ops::serverless_metadata::fetch::Output) -> Self {
		ServerlessMetadata {
			runtime: output.runtime,
			version: output.version,
			actor_names: output
				.actor_names
				.into_iter()
				.map(|a| (a.name, serde_json::Value::Object(a.metadata)))
				.collect(),
			runner_version: output.runner_version,
		}
	}
}

/// Fetches metadata from a serverless runner at the given URL.
///
/// Returns metadata including runtime, version, and actor names if available.
#[tracing::instrument(skip_all)]
pub async fn fetch_serverless_runner_metadata(
	ctx: &ApiCtx,
	url: String,
	headers: HashMap<String, String>,
) -> Result<ServerlessMetadata, ServerlessMetadataError> {
	let result = ctx
		.op(pegboard::ops::serverless_metadata::fetch::Input { url, headers })
		.await
		.map_err(|_| ServerlessMetadataError::RequestFailed {})?;

	result.map(ServerlessMetadata::from)
}

/// Fetches metadata from the given URL and populates actor names in the database.
#[tracing::instrument(skip_all)]
pub async fn refresh_runner_config_metadata(
	ctx: ApiCtx,
	namespace_id: Id,
	runner_name: String,
	url: String,
	headers: HashMap<String, String>,
) -> anyhow::Result<()> {
	tracing::debug!(
		?namespace_id,
		?runner_name,
		"refreshing runner config metadata"
	);

	// Fetch metadata using the op
	let result = ctx
		.op(pegboard::ops::serverless_metadata::fetch::Input { url, headers })
		.await?;

	let metadata =
		result.map_err(|e| anyhow!("failed to fetch serverless runner metadata: {:?}", e))?;

	if !metadata.actor_names.is_empty() {
		tracing::debug!(
			actor_names_count = metadata.actor_names.len(),
			"storing actor names metadata"
		);

		// Convert and store actor names
		let actor_names: Vec<pegboard::ops::actor_name::upsert_batch::ActorNameEntry> = metadata
			.actor_names
			.into_iter()
			.map(
				|a| pegboard::ops::actor_name::upsert_batch::ActorNameEntry {
					name: a.name,
					metadata: a.metadata,
				},
			)
			.collect();

		ctx.op(pegboard::ops::actor_name::upsert_batch::Input {
			namespace_id,
			actor_names,
		})
		.await?;

		tracing::debug!("successfully stored actor names metadata");
	} else {
		tracing::debug!("no actor names to store");
	}

	Ok(())
}
