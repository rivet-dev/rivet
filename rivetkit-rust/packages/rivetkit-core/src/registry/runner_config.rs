use std::net::IpAddr;

use anyhow::{Context, Result};
use crate::time::sleep;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{Map as JsonMap, json};
use std::time::{Duration, Instant};

use super::ServeConfig;

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
	let datacenters = get_datacenters(&client, config).await?;
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
		.with_context(|| {
			format!(
				"cannot reach Rivet Engine at {} while updating runner config",
				config.endpoint
			)
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.context("read failed runner config response body")?;
		anyhow::bail!(
			"failed to upsert local runner config `{}`: {} {}",
			config.pool_name,
			status,
			response_body
		);
	}

	tracing::debug!(
		namespace = %config.namespace,
		pool_name = %config.pool_name,
		"ensured local normal runner config"
	);

	Ok(())
}

pub(super) async fn ensure_local_serverless_runner_config(config: &ServeConfig) -> Result<()> {
	let Some(url) = config.dev_serverless_url.as_deref() else {
		return Ok(());
	};
	if !is_local_engine_endpoint(&config.endpoint) {
		return Ok(());
	}

	let client = Client::builder()
		.build()
		.context("build reqwest client for serverless runner config")?;
	let timeout_ms = std::env::var("RIVET_SERVERLESS_CONFIGURE_TIMEOUT_MS")
		.ok()
		.and_then(|value| value.parse::<u64>().ok())
		.unwrap_or(60_000);
	let started_at = Instant::now();
	let mut attempts = 0_u32;

	loop {
		attempts += 1;
		match try_ensure_local_serverless_runner_config(&client, config, url).await {
			Ok(()) => {
				tracing::info!(
					namespace = %config.namespace,
					pool_name = %config.pool_name,
					attempts,
					"ensured local serverless runner config"
				);
				return Ok(());
			}
			Err(error) => {
				tracing::warn!(
					namespace = %config.namespace,
					pool_name = %config.pool_name,
					attempts,
					error = %error,
					"serverless runner config attempt failed"
				);
				if started_at.elapsed() >= Duration::from_millis(timeout_ms) {
					return Err(error).context("failed to configure local serverless runner config");
				}
			}
		}

		sleep(Duration::from_secs(1)).await;
	}
}

async fn try_ensure_local_serverless_runner_config(
	client: &Client,
	config: &ServeConfig,
	serverless_url: &str,
) -> Result<()> {
	let datacenters = get_datacenters(client, config).await?;
	let mut runner_datacenters = JsonMap::new();
	let serverless_token = config
		.token
		.as_ref()
		.or(config.serverless_client_token.as_ref());
	let headers = match serverless_token {
		Some(token) => json!({ "x-rivet-token": token }),
		None => json!({}),
	};
	let request_lifespan = config
		.dev_serverless_request_timeout
		.map(|timeout| (timeout.saturating_add(999) / 1000).max(1))
		.unwrap_or(60 * 60);
	let drain_grace_period = config
		.dev_serverless_drain_timeout
		.map(|timeout| (timeout.saturating_add(999) / 1000).max(1));

	for datacenter in datacenters.datacenters {
		runner_datacenters.insert(
			datacenter.name,
			json!({
				"serverless": {
					"url": serverless_url,
					"headers": headers,
					"request_lifespan": request_lifespan,
					"drain_grace_period": drain_grace_period,
					"metadata_poll_interval": 1000,
					"max_runners": 100000,
					"min_runners": 0,
					"runners_margin": 0,
					"slots_per_runner": 1,
				},
				"metadata": {},
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
		.with_context(|| {
			format!(
				"cannot reach Rivet Engine at {} while updating serverless runner config",
				config.endpoint
			)
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.context("read failed serverless runner config response body")?;
		anyhow::bail!(
			"failed to upsert local serverless runner config `{}`: {} {}",
			config.pool_name,
			status,
			response_body
		);
	}

	Ok(())
}

async fn get_datacenters(client: &Client, config: &ServeConfig) -> Result<DatacentersResponse> {
	let url = engine_api_url(&config.endpoint, &["datacenters"], &config.namespace)?;
	let response = apply_auth(client.get(url), config)
		.send()
		.await
		.with_context(|| {
			format!(
				"cannot reach Rivet Engine at {} while listing datacenters",
				config.endpoint
			)
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.context("read failed datacenters response body")?;
		anyhow::bail!(
			"failed to get local datacenters for runner config: {} {}",
			status,
			response_body
		);
	}

	response
		.json::<DatacentersResponse>()
		.await
		.context("decode datacenters response")
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
