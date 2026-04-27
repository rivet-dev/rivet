use std::collections::HashMap;
use std::net::IpAddr;

use anyhow::{Context, Result};
use reqwest::{Client, Url};
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::ServeConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineAdminConfig {
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	#[serde(default)]
	pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatacentersResponse {
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Datacenter {
	pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerConfigRequest {
	pub datacenters: HashMap<String, RunnerConfigDatacenterRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerConfigDatacenterRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub normal: Option<HashMap<String, JsonValue>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub serverless: Option<ServerlessRunnerConfig>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<HashMap<String, JsonValue>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub drain_on_version_upgrade: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessRunnerConfig {
	pub url: String,
	pub headers: HashMap<String, String>,
	pub max_runners: u32,
	pub min_runners: u32,
	pub request_lifespan: u32,
	pub runners_margin: u32,
	pub slots_per_runner: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata_poll_interval: Option<u32>,
}

#[derive(RivetError, Debug, Clone, Serialize, Deserialize)]
#[error("engine")]
enum RunnerConfigError {
	#[error(
		"invalid_endpoint",
		"Engine endpoint is invalid.",
		"Engine endpoint '{endpoint}' is invalid: {reason}"
	)]
	InvalidEndpoint { endpoint: String, reason: String },

	#[error(
		"request_failed",
		"Engine admin request failed.",
		"Engine admin request '{operation}' failed: {status} {response_body}"
	)]
	RequestFailed {
		operation: String,
		status: String,
		response_body: String,
	},
}

pub async fn get_datacenters(config: &EngineAdminConfig) -> Result<DatacentersResponse> {
	let client = build_client().await?;
	let url = engine_api_url(&config.endpoint, &["datacenters"], &config.namespace)?;
	let response = apply_headers(client.get(url), config)
		.send()
		.await
		.context("get engine datacenters")?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.context("read failed datacenters response body")?;
		return Err(request_failed("get datacenters", status, response_body));
	}

	response
		.json::<DatacentersResponse>()
		.await
		.context("decode datacenters response")
}

pub async fn update_runner_config(
	config: &EngineAdminConfig,
	runner_name: &str,
	request: &RunnerConfigRequest,
) -> Result<()> {
	let client = build_client().await?;
	let url = engine_api_url(
		&config.endpoint,
		&["runner-configs", runner_name],
		&config.namespace,
	)?;
	let response = apply_headers(client.put(url), config)
		.json(request)
		.send()
		.await
		.context("upsert runner config")?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.context("read failed runner config response body")?;
		return Err(request_failed(
			&format!("upsert runner config `{runner_name}`"),
			status,
			response_body,
		));
	}

	Ok(())
}

pub async fn upsert_runner_config_for_all_datacenters(
	config: &EngineAdminConfig,
	runner_name: &str,
	datacenter_request: RunnerConfigDatacenterRequest,
) -> Result<()> {
	let datacenters = get_datacenters(config).await?;
	let request = RunnerConfigRequest {
		datacenters: datacenters
			.datacenters
			.into_iter()
			.map(|datacenter| (datacenter.name, datacenter_request.clone()))
			.collect(),
	};

	update_runner_config(config, runner_name, &request).await
}

pub(super) async fn ensure_local_normal_runner_config(config: &ServeConfig) -> Result<()> {
	if !is_local_engine_endpoint(&config.endpoint) {
		return Ok(());
	}

	upsert_runner_config_for_all_datacenters(
		&EngineAdminConfig::from(config),
		&config.pool_name,
		RunnerConfigDatacenterRequest {
			normal: Some(HashMap::new()),
			drain_on_version_upgrade: Some(true),
			..Default::default()
		},
	)
	.await?;

	tracing::debug!(
		namespace = %config.namespace,
		pool_name = %config.pool_name,
		"ensured local normal runner config"
	);

	Ok(())
}

impl From<&ServeConfig> for EngineAdminConfig {
	fn from(value: &ServeConfig) -> Self {
		Self {
			endpoint: value.endpoint.clone(),
			token: value.token.clone(),
			namespace: value.namespace.clone(),
			headers: HashMap::new(),
		}
	}
}

async fn build_client() -> Result<Client> {
	rivet_pools::reqwest::client()
		.await
		.context("build reqwest client for runner config")
}

fn apply_headers(
	request: reqwest::RequestBuilder,
	config: &EngineAdminConfig,
) -> reqwest::RequestBuilder {
	let mut request = request;
	for (key, value) in &config.headers {
		request = request.header(key, value);
	}

	match config.token.as_deref() {
		Some(token) => request.bearer_auth(token),
		None => request,
	}
}

fn engine_api_url(endpoint: &str, path: &[&str], namespace: &str) -> Result<Url> {
	let mut url = Url::parse(endpoint).map_err(|error| {
		RunnerConfigError::InvalidEndpoint {
			endpoint: endpoint.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})?;
	url.set_path("");
	url.path_segments_mut()
		.map_err(|_| {
			RunnerConfigError::InvalidEndpoint {
				endpoint: endpoint.to_owned(),
				reason: "endpoint cannot be a base URL".to_owned(),
			}
			.build()
		})?
		.extend(path);
	url.query_pairs_mut()
		.clear()
		.append_pair("namespace", namespace);
	Ok(url)
}

fn request_failed(
	operation: &str,
	status: reqwest::StatusCode,
	response_body: String,
) -> anyhow::Error {
	RunnerConfigError::RequestFailed {
		operation: operation.to_owned(),
		status: status.to_string(),
		response_body,
	}
	.build()
}

fn is_local_engine_endpoint(endpoint: &str) -> bool {
	let Ok(url) = Url::parse(endpoint) else {
		return false;
	};
	let Some(host) = url.host_str() else {
		return false;
	};

	if host == "localhost" || host.ends_with(".localhost") {
		return true;
	}

	host.parse::<IpAddr>()
		.map(|ip| ip.is_loopback() || ip.is_unspecified())
		.unwrap_or(false)
}
