use std::net::IpAddr;

use anyhow::{Context, Result};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{Map as JsonMap, json};

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

	let client = rivet_pools::reqwest::client()
		.await
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
		.context("upsert local runner config")?;
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

async fn get_datacenters(client: &Client, config: &ServeConfig) -> Result<DatacentersResponse> {
	let url = engine_api_url(&config.endpoint, &["datacenters"], &config.namespace)?;
	let response = apply_auth(client.get(url), config)
		.send()
		.await
		.context("get local datacenters")?;
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
