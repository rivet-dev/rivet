use anyhow::Result;
use std::collections::HashMap;

use gas::prelude::*;

use crate::ctx::ApiCtx;

pub use pegboard::ops::serverless_metadata::fetch::{
	ServerlessMetadataError, ServerlessMetadataErrorEnvelope,
};

/// Serverless metadata returned from a runner.
#[derive(Debug, Clone)]
pub struct ServerlessMetadata {
	pub runtime: String,
	pub version: String,
	pub actor_names: HashMap<String, serde_json::Value>,
	pub envoy_version: Option<u32>,
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
			envoy_version: output.envoy_version,
		}
	}
}

/// Fetches metadata from a serverless runner at the given URL.
///
/// Returns metadata including runtime, version, and actor names if available.
#[tracing::instrument(skip_all)]
pub async fn fetch_serverless_metadata(
	ctx: &ApiCtx,
	url: String,
	headers: HashMap<String, String>,
) -> std::result::Result<ServerlessMetadata, ServerlessMetadataError> {
	ctx.op(pegboard::ops::serverless_metadata::fetch::Input { url, headers })
		.await
		.map_err(|_| ServerlessMetadataError::RequestFailed {})?
		.map(ServerlessMetadata::from)
}

/// Fetches metadata from the given URL and populates actor names in the database.
#[tracing::instrument(skip_all)]
pub async fn refresh_runner_pool_metadata(
	ctx: ApiCtx,
	namespace_id: Id,
	pool_name: String,
	url: String,
	headers: HashMap<String, String>,
) -> Result<()> {
	tracing::debug!(?namespace_id, ?pool_name, "refreshing envoy pool metadata");

	ctx.op(pegboard::ops::envoy_pool::refresh_metadata::Input {
		namespace_id,
		pool_name,
		url,
		headers,
	})
	.await?
	.map_err(|e| {
		pegboard::errors::ServerlessRunnerPool::FailedToFetchMetadata {
			reason: ServerlessMetadataErrorEnvelope::from(e),
		}
		.build()
	})?;

	Ok(())
}
