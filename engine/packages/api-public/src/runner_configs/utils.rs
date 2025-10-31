use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use gas::prelude::*;
use reqwest::header::{HeaderMap as ReqwestHeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

const RESPONSE_BODY_MAX_LEN: usize = 1024;

#[derive(Deserialize, Serialize, ToSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[schema(as = RunnerConfigsServerlessMetadataError)]
pub enum ServerlessMetadataError {
	InvalidRequest {},
	RequestFailed {},
	RequestTimedOut {},
	NonSuccessStatus { status_code: u16, body: String },
	InvalidResponseJson { body: String },
	InvalidResponseSchema { runtime: String, version: String },
}

#[derive(Debug, Clone)]
pub struct ServerlessMetadata {
	pub runtime: String,
	pub version: String,
	pub actor_names: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ServerlessMetadataPayload {
	runtime: String,
	version: String,
	#[serde(rename = "actorNames", default)]
	actor_names: HashMap<String, serde_json::Value>,
}

fn truncate_response_body(body: &str) -> String {
	let mut chars = body.chars();
	let mut truncated: String = chars.by_ref().take(RESPONSE_BODY_MAX_LEN).collect();
	if chars.next().is_some() {
		truncated.push_str("...[truncated]");
	}

	truncated
}

/// Fetches metadata from a serverless runner at the given URL.
///
/// Returns metadata including runtime, version, and actor names if available.
#[tracing::instrument(skip_all)]
pub async fn fetch_serverless_runner_metadata(
	url: String,
	headers: HashMap<String, String>,
) -> Result<ServerlessMetadata, ServerlessMetadataError> {
	tracing::debug!(?url, "fetching serverless runner metadata");

	let trimmed_url = url.trim();
	if trimmed_url.is_empty() {
		return Err(ServerlessMetadataError::InvalidRequest {});
	}

	let metadata_url = format!("{}/metadata", trimmed_url.trim_end_matches('/'));

	if reqwest::Url::parse(&metadata_url).is_err() {
		return Err(ServerlessMetadataError::InvalidRequest {});
	}

	let mut header_map = ReqwestHeaderMap::new();
	for (name, value) in headers {
		let header_name = HeaderName::from_bytes(name.trim().as_bytes())
			.map_err(|_| ServerlessMetadataError::InvalidRequest {})?;

		let header_value = HeaderValue::from_str(value.trim())
			.map_err(|_| ServerlessMetadataError::InvalidRequest {})?;

		header_map.insert(header_name, header_value);
	}

	let client = rivet_pools::reqwest::client()
		.await
		.map_err(|_| ServerlessMetadataError::RequestFailed {})?;

	tracing::debug!("sending metadata request");
	let response = client
		.get(&metadata_url)
		.headers(header_map)
		.timeout(Duration::from_secs(10))
		.send()
		.custom_instrument(tracing::info_span!("fetch_metadata_request"))
		.await
		.map_err(|err| {
			if err.is_timeout() {
				ServerlessMetadataError::RequestTimedOut {}
			} else {
				ServerlessMetadataError::RequestFailed {}
			}
		})?;

	let status = response.status();
	tracing::debug!(?status, "received metadata response");
	let body_raw = response
		.text()
		.await
		.unwrap_or_else(|_| String::from("<failed to read body>"));
	let body_for_user = truncate_response_body(&body_raw);

	if !status.is_success() {
		return Err(ServerlessMetadataError::NonSuccessStatus {
			status_code: status.as_u16(),
			body: body_for_user,
		});
	}

	let payload = serde_json::from_str::<ServerlessMetadataPayload>(&body_raw).map_err(|_| {
		ServerlessMetadataError::InvalidResponseJson {
			body: body_for_user,
		}
	})?;

	let ServerlessMetadataPayload {
		runtime,
		version,
		actor_names,
	} = payload;

	tracing::debug!(
		?runtime,
		?version,
		actor_names_count = actor_names.len(),
		"parsed metadata payload"
	);

	let trimmed_version = version.trim();
	if runtime != "rivetkit" || trimmed_version.is_empty() {
		return Err(ServerlessMetadataError::InvalidResponseSchema { runtime, version });
	}

	Ok(ServerlessMetadata {
		runtime,
		version: trimmed_version.to_owned(),
		actor_names,
	})
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

	// Fetch metadata
	let metadata = fetch_serverless_runner_metadata(url, headers)
		.await
		.map_err(|e| anyhow!("failed to fetch serverless runner metadata: {:?}", e))?;

	if !metadata.actor_names.is_empty() {
		tracing::debug!(
			actor_names_count = metadata.actor_names.len(),
			"storing actor names metadata"
		);
		// Convert actor names to the format needed for database operations
		let actor_names: Vec<(String, serde_json::Map<String, serde_json::Value>)> = metadata
			.actor_names
			.into_iter()
			.map(|(name, value)| {
				if let serde_json::Value::Object(map) = value {
					Ok((name, map))
				} else {
					Err(anyhow!(
						"actor name '{}' metadata must be an object, got: {:?}",
						name,
						value
					))
				}
			})
			.collect::<anyhow::Result<Vec<_>>>()?;

		// Store actor names metadata with the runner config
		ctx.udb()?
			.run(|tx| {
				let actor_names = actor_names.clone();
				async move {
					let tx = tx.with_subspace(pegboard::keys::subspace());

					// Write actor names
					for (name, metadata) in actor_names {
						tx.write(
							&pegboard::keys::ns::ActorNameKey::new(namespace_id, name.clone()),
							rivet_data::converted::ActorNameKeyData { metadata },
						)?;
					}

					Ok(())
				}
			})
			.custom_instrument(tracing::info_span!("runner_config_populate_actor_names_tx"))
			.await?;

		tracing::debug!("successfully stored actor names metadata");
	} else {
		tracing::debug!("no actor names to store");
	}

	Ok(())
}
