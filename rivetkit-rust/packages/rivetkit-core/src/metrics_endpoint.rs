use std::collections::HashMap;

use anyhow::{Context, Result};
use rivet_metrics::prometheus::{Encoder, TextEncoder};
use subtle::ConstantTimeEq;

const METRICS_ENABLED_ENV: &str = "RIVETKIT_METRICS_ENABLED";
const METRICS_TOKEN_ENV: &str = "RIVETKIT_METRICS_TOKEN";

pub struct RenderedMetrics {
	pub content_type: String,
	pub body: Vec<u8>,
}

pub enum MetricsAccessError {
	NotEnabled,
	Unauthorized,
}

pub fn authorize_metrics_request(
	bearer_token: Option<&str>,
) -> std::result::Result<(), MetricsAccessError> {
	let Some(configured_token) = configured_metrics_token() else {
		return Err(MetricsAccessError::NotEnabled);
	};

	let Some(bearer_token) = bearer_token.filter(|token| !token.is_empty()) else {
		return Err(MetricsAccessError::Unauthorized);
	};

	if bearer_token.as_bytes().ct_eq(configured_token.as_bytes()).into() {
		Ok(())
	} else {
		Err(MetricsAccessError::Unauthorized)
	}
}

pub fn render_prometheus_metrics() -> Result<RenderedMetrics> {
	let encoder = TextEncoder::new();
	let metric_families = rivet_metrics::REGISTRY.gather();
	let mut body = Vec::new();
	encoder
		.encode(&metric_families, &mut body)
		.context("encode prometheus metrics")?;

	Ok(RenderedMetrics {
		content_type: encoder.format_type().to_owned(),
		body,
	})
}

pub fn authorization_bearer_token(headers: &http::HeaderMap) -> Option<&str> {
	headers
		.get(http::header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(bearer_token_from_authorization)
}

pub fn authorization_bearer_token_map(headers: &HashMap<String, String>) -> Option<&str> {
	headers
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case(http::header::AUTHORIZATION.as_str()))
		.and_then(|(_, value)| bearer_token_from_authorization(value))
}

fn configured_metrics_token() -> Option<String> {
	let enabled = std::env::var(METRICS_ENABLED_ENV).ok()?;
	if enabled != "1" {
		return None;
	}

	std::env::var(METRICS_TOKEN_ENV)
		.ok()
		.filter(|token| !token.is_empty())
}

fn bearer_token_from_authorization(value: &str) -> Option<&str> {
	let value = value.trim_start();
	let scheme = value.get(..6)?;
	if !scheme.eq_ignore_ascii_case("bearer") {
		return None;
	}

	let rest = value.get(6..)?;
	if !rest.chars().next().is_some_and(char::is_whitespace) {
		return None;
	}

	let token = rest.trim_start();
	if token.is_empty() { None } else { Some(token) }
}
