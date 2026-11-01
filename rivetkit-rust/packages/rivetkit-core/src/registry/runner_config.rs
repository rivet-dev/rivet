use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, Url};
use rivetkit_shared_types::endpoint::is_local_engine_endpoint;
use serde_json::json;

use crate::time::sleep;

use super::ServeConfig;

const RUNNER_CONFIG_MAX_ATTEMPTS: usize = 60;
const RUNNER_CONFIG_RETRY_DELAY: Duration = Duration::from_millis(500);

pub(super) async fn ensure_local_normal_runner_config(config: &ServeConfig) -> Result<()> {
	if !is_local_engine_endpoint(&config.endpoint) {
		return Ok(());
	}

	let client = Client::builder()
		.build()
		.context("build reqwest client for node pool configuration")?;
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
			Err(AttemptError::Retryable { error, .. }) if attempt == max_attempts => {
				return Err(error).with_context(|| {
					format!(
						"local node pool configuration `{}` did not become ready after {max_attempts} attempts",
						config.pool_name
					)
				});
			}
			Err(AttemptError::Retryable { error, retry_after }) => {
				tracing::debug!(
					?error,
					attempt,
					max_attempts,
					namespace = %config.namespace,
					pool_name = %config.pool_name,
					"local Engine namespace or node pool configuration is not ready; retrying"
				);
				sleep(retry_delay.max(retry_after)).await;
			}
		}
	}

	unreachable!("node pool configuration retry loop always returns")
}

async fn ensure_local_normal_runner_config_once(
	client: &Client,
	config: &ServeConfig,
) -> Result<(), AttemptError> {
	let url = engine_api_url(
		&config.endpoint,
		&["pools", config.pool_name.as_str()],
		&config.namespace,
	)?;
	let body = json!({"drain_on_version_upgrade": true});

	let response = apply_auth(client.put(url), config)
		.json(&body)
		.send()
		.await
		.map_err(|error| AttemptError::Retryable {
			error: anyhow::Error::new(error).context("upsert local node pool configuration"),
			retry_after: Duration::ZERO,
		})?;
	let status = response.status();
	if !status.is_success() {
		let response_body = response
			.text()
			.await
			.map_err(|error| AttemptError::Retryable {
				error: anyhow::Error::new(error)
					.context("read failed node pool configuration response body"),
				retry_after: Duration::ZERO,
			})?;
		return Err(response_error(
			format!(
				"failed to upsert local node pool configuration `{}`",
				config.pool_name
			),
			status,
			response_body,
		));
	}

	tracing::debug!(
		namespace = %config.namespace,
		pool_name = %config.pool_name,
		"ensured local normal node pool configuration"
	);

	Ok(())
}

#[derive(Debug)]
enum AttemptError {
	Retryable {
		error: anyhow::Error,
		retry_after: Duration,
	},
	Fatal(anyhow::Error),
}

impl From<anyhow::Error> for AttemptError {
	fn from(error: anyhow::Error) -> Self {
		Self::Fatal(error)
	}
}

fn response_error(action: String, status: StatusCode, body: String) -> AttemptError {
	let error = anyhow::Error::msg(format!("{action}: {status} {body}"));
	let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();
	let group = parsed
		.as_ref()
		.and_then(|value| value.get("group")?.as_str());
	let code = parsed
		.as_ref()
		.and_then(|value| value.get("code")?.as_str());
	// Engines can report structured storage overloads as 400. The refusal is
	// retryable regardless of that status; other client errors remain fatal.
	let overloaded = group == Some("depot") && code == Some("overloaded");
	if status == StatusCode::NOT_FOUND
		|| status == StatusCode::TOO_MANY_REQUESTS
		|| status.is_server_error()
		|| code == Some("not_found")
		|| overloaded
	{
		let retry_after = if overloaded {
			parsed
				.as_ref()
				.and_then(|value| value.pointer("/metadata/retry_after_ms")?.as_u64())
				.map(Duration::from_millis)
				.unwrap_or_default()
				// A longer hint cannot lead to another attempt within the enclosing
				// startup timeout. Bound it before doing timer arithmetic.
				.min(super::ENVOY_START_TIMEOUT)
		} else {
			Duration::ZERO
		};
		AttemptError::Retryable { error, retry_after }
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
