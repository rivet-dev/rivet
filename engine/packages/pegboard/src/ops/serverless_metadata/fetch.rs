use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use reqwest::header::{HeaderMap as ReqwestHeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const RESPONSE_BODY_MAX_LEN: usize = 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Input {
	pub url: String,
	pub headers: HashMap<String, String>,
}

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
pub struct Output {
	pub runtime: String,
	pub version: String,
	pub actor_names: Vec<ActorNameMetadata>,
	pub runner_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ActorNameMetadata {
	pub name: String,
	pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ServerlessMetadataRunner {
	version: Option<u32>,
}

#[derive(Deserialize)]
struct ServerlessMetadataPayload {
	runtime: String,
	version: String,
	#[serde(rename = "actorNames", default)]
	actor_names: HashMap<String, serde_json::Value>,
	runner: Option<ServerlessMetadataRunner>,
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
#[operation]
#[tracing::instrument(skip_all)]
pub async fn pegboard_serverless_metadata_fetch(
	_ctx: &OperationCtx,
	input: &Input,
) -> Result<std::result::Result<Output, ServerlessMetadataError>> {
	tracing::debug!(url = ?input.url, "fetching serverless runner metadata");

	let trimmed_url = input.url.trim();
	if trimmed_url.is_empty() {
		return Ok(Err(ServerlessMetadataError::InvalidRequest {}));
	}

	let metadata_url = format!("{}/metadata", trimmed_url.trim_end_matches('/'));

	if reqwest::Url::parse(&metadata_url).is_err() {
		return Ok(Err(ServerlessMetadataError::InvalidRequest {}));
	}

	let mut header_map = ReqwestHeaderMap::new();
	for (name, value) in &input.headers {
		let header_name = match HeaderName::from_bytes(name.trim().as_bytes()) {
			Ok(n) => n,
			Err(_) => return Ok(Err(ServerlessMetadataError::InvalidRequest {})),
		};

		let header_value = match HeaderValue::from_str(value.trim()) {
			Ok(v) => v,
			Err(_) => return Ok(Err(ServerlessMetadataError::InvalidRequest {})),
		};

		header_map.insert(header_name, header_value);
	}

	let client = match rivet_pools::reqwest::client().await {
		Ok(c) => c,
		Err(_) => return Ok(Err(ServerlessMetadataError::RequestFailed {})),
	};

	tracing::debug!("sending metadata request");
	let response = match client
		.get(&metadata_url)
		.headers(header_map)
		.timeout(REQUEST_TIMEOUT)
		.send()
		.custom_instrument(tracing::info_span!("fetch_metadata_request"))
		.await
	{
		Ok(r) => r,
		Err(err) => {
			return Ok(Err(if err.is_timeout() {
				ServerlessMetadataError::RequestTimedOut {}
			} else {
				ServerlessMetadataError::RequestFailed {}
			}));
		}
	};

	let status = response.status();
	tracing::debug!(?status, "received metadata response");
	let body_raw = response
		.text()
		.await
		.unwrap_or_else(|_| String::from("<failed to read body>"));
	let body_for_user = truncate_response_body(&body_raw);

	if !status.is_success() {
		return Ok(Err(ServerlessMetadataError::NonSuccessStatus {
			status_code: status.as_u16(),
			body: body_for_user,
		}));
	}

	let payload = match serde_json::from_str::<ServerlessMetadataPayload>(&body_raw) {
		Ok(p) => p,
		Err(_) => {
			return Ok(Err(ServerlessMetadataError::InvalidResponseJson {
				body: body_for_user,
			}));
		}
	};

	let ServerlessMetadataPayload {
		runtime,
		version,
		actor_names,
		runner,
	} = payload;

	let runner_version = runner.and_then(|r| r.version);

	tracing::debug!(
		?runtime,
		?version,
		actor_names_count = actor_names.len(),
		?runner_version,
		"parsed metadata payload"
	);

	let trimmed_version = version.trim();
	if runtime != "rivetkit" || trimmed_version.is_empty() {
		return Ok(Err(ServerlessMetadataError::InvalidResponseSchema {
			runtime,
			version,
		}));
	}

	// Convert actor names, filtering out non-object metadata
	let actor_names: Vec<ActorNameMetadata> = actor_names
		.into_iter()
		.filter_map(|(name, value)| {
			let metadata = value.as_object()?.clone();
			Some(ActorNameMetadata { name, metadata })
		})
		.collect();

	Ok(Ok(Output {
		runtime,
		version: trimmed_version.to_owned(),
		actor_names,
		runner_version,
	}))
}
