use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::{Map as JsonMap, json};

use crate::time::sleep;

use super::ServeConfig;

const RUNNER_CONFIG_MAX_ATTEMPTS: usize = 60;
const RUNNER_CONFIG_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
struct DatacentersResponse {
	datacenters: Vec<Datacenter>,
}

#[derive(Debug, Deserialize)]
struct Datacenter {
	name: String,
}

pub(super) async fn ensure_local_normal_runner_config(config: &ServeConfig) -> Result<()> {
	if !is_local_engine_endpoint(&config.endpoint) {
		return Ok(());
	}

	let client = Client::builder()
		.build()
		.context("build reqwest client for runner config")?;
	ensure_local_normal_runner_config_with_retry(
		&client,
		config,
		RUNNER_CONFIG_MAX_ATTEMPTS,
		RUNNER_CONFIG_RETRY_DELAY,
	)
	.await
}

async fn ensure_local_normal_runner_config_with_retry(
	client: &Client,
	config: &ServeConfig,
	max_attempts: usize,
	retry_delay: Duration,
) -> Result<()> {
	let max_attempts = max_attempts.max(1);
	for attempt in 1..=max_attempts {
		match ensure_local_normal_runner_config_once(client, config).await {
			Ok(()) => return Ok(()),
			Err(AttemptError::Fatal(error)) => return Err(error),
			Err(AttemptError::Retryable(error)) if attempt == max_attempts => {
				return Err(error).with_context(|| {
					format!(
						"local runner config `{}` did not become ready after {max_attempts} attempts",
						config.pool_name
					)
				});
			}
			Err(AttemptError::Retryable(error)) => {
				tracing::debug!(
					?error,
					attempt,
					max_attempts,
					namespace = %config.namespace,
					pool_name = %config.pool_name,
					"local Engine namespace or runner config is not ready; retrying"
				);
				sleep(retry_delay).await;
			}
		}
	}

	unreachable!("runner config retry loop always returns")
}

async fn ensure_local_normal_runner_config_once(
	client: &Client,
	config: &ServeConfig,
) -> Result<(), AttemptError> {
	let datacenters = get_datacenters(&client, config).await?;
	if datacenters.datacenters.is_empty() {
		return Err(AttemptError::Retryable(anyhow::Error::msg(
			"local Engine returned no datacenters",
		)));
	}
	let mut runner_datacenters = JsonMap::new();

	for datacenter in datacenters.datacenters {
		runner_datacenters.insert(
			datacenter.name,
			json!({
				"normal": {},
				"drain_on_version_upgrade": true,
			}),
		);
	}

	let url = engine_api_url(
		&config.endpoint,
		&["runner-configs", config.pool_name.as_str()],
		&config.namespace,
	)?;
	let body = json!({
		"datacenters": runner_datacenters,
	});

	let response = apply_auth(client.put(url), config)
		.json(&body)
		.send()
		.await
		.map_err(|error| {
			AttemptError::Retryable(anyhow::Error::new(error).context("upsert local runner config"))
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.map_err(|error| {
				AttemptError::Retryable(
					anyhow::Error::new(error).context("read failed runner config response body"),
				)
			})?;
		return Err(response_error(
			format!("failed to upsert local runner config `{}`", config.pool_name),
			status,
			response_body,
		));
	}

	tracing::debug!(
		namespace = %config.namespace,
		pool_name = %config.pool_name,
		"ensured local normal runner config"
	);

	Ok(())
}

async fn get_datacenters(
	client: &Client,
	config: &ServeConfig,
) -> Result<DatacentersResponse, AttemptError> {
	let url = engine_api_url(&config.endpoint, &["datacenters"], &config.namespace)?;
	let response = apply_auth(client.get(url), config)
		.send()
		.await
		.map_err(|error| {
			AttemptError::Retryable(anyhow::Error::new(error).context("get local datacenters"))
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.map_err(|error| {
				AttemptError::Retryable(
					anyhow::Error::new(error).context("read failed datacenters response body"),
				)
			})?;
		return Err(response_error(
			"failed to get local datacenters for runner config".to_owned(),
			status,
			response_body,
		));
	}

	response
		.json::<DatacentersResponse>()
		.await
		.map_err(|error| {
			AttemptError::Fatal(anyhow::Error::new(error).context("decode datacenters response"))
		})
}

#[derive(Debug)]
enum AttemptError {
	Retryable(anyhow::Error),
	Fatal(anyhow::Error),
}

impl From<anyhow::Error> for AttemptError {
	fn from(error: anyhow::Error) -> Self {
		Self::Fatal(error)
	}
}

fn response_error(action: String, status: StatusCode, body: String) -> AttemptError {
	let error = anyhow::Error::msg(format!("{action}: {status} {body}"));
	let not_found = serde_json::from_str::<serde_json::Value>(&body)
		.ok()
		.and_then(|value| value.get("code")?.as_str().map(str::to_owned))
		.as_deref()
		== Some("not_found");
	if status == StatusCode::NOT_FOUND
		|| status == StatusCode::TOO_MANY_REQUESTS
		|| status.is_server_error()
		|| not_found
	{
		AttemptError::Retryable(error)
	} else {
		AttemptError::Fatal(error)
	}
}

fn apply_auth(request: reqwest::RequestBuilder, config: &ServeConfig) -> reqwest::RequestBuilder {
	match config.token.as_deref() {
		Some(token) => request.bearer_auth(token),
		None => request,
	}
}

fn engine_api_url(endpoint: &str, path: &[&str], namespace: &str) -> Result<Url> {
	let mut url =
		Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
	url.set_path("");
	url.path_segments_mut()
		.map_err(|_| anyhow::anyhow!("engine endpoint cannot be a base URL: {endpoint}"))?
		.extend(path);
	url.query_pairs_mut()
		.clear()
		.append_pair("namespace", namespace);
	Ok(url)
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

#[cfg(test)]
#[path = "../../tests/runner_config.rs"]
mod tests;
