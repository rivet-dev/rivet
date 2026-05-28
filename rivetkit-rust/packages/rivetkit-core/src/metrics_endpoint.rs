use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use rivet_metrics::prometheus::{Encoder, Gauge, IntGauge, Opts, Registry, TextEncoder};
use subtle::ConstantTimeEq;

use crate::registry::CoreEnvoyStatus;

const METRICS_ENABLED_ENV: &str = "RIVETKIT_METRICS_ENABLED";
const METRICS_TOKEN_ENV: &str = "RIVETKIT_METRICS_TOKEN";

struct EnvoyMetricCollectors {
	last_ping_timestamp_seconds: Gauge,
	ping_healthy: IntGauge,
}

static ENVOY_METRICS: LazyLock<EnvoyMetricCollectors> = LazyLock::new(EnvoyMetricCollectors::new);

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

	if bearer_token
		.as_bytes()
		.ct_eq(configured_token.as_bytes())
		.into()
	{
		Ok(())
	} else {
		Err(MetricsAccessError::Unauthorized)
	}
}

pub fn render_prometheus_metrics(
	envoy_status: Option<&CoreEnvoyStatus>,
) -> Result<RenderedMetrics> {
	ENVOY_METRICS.refresh(envoy_status);

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

impl EnvoyMetricCollectors {
	fn new() -> Self {
		let last_ping_timestamp_seconds = Gauge::with_opts(Opts::new(
			"rivetkit_envoy_last_ping_timestamp_seconds",
			"unix timestamp of the most recent engine ping received by rivetkit",
		))
		.expect("create envoy last ping timestamp gauge");
		let ping_healthy = IntGauge::with_opts(Opts::new(
			"rivetkit_envoy_ping_healthy",
			"whether rivetkit has received a recent engine ping",
		))
		.expect("create envoy ping healthy gauge");

		register_metric(
			&rivet_metrics::REGISTRY,
			last_ping_timestamp_seconds.clone(),
		);
		register_metric(&rivet_metrics::REGISTRY, ping_healthy.clone());

		Self {
			last_ping_timestamp_seconds,
			ping_healthy,
		}
	}

	fn refresh(&self, envoy_status: Option<&CoreEnvoyStatus>) {
		let last_ping_timestamp_seconds = envoy_status
			.and_then(|status| status.last_ping_at_ms)
			.map(|ts| ts as f64 / 1_000.0)
			.unwrap_or(0.0);
		let ping_healthy = envoy_status
			.map(|status| if status.ping_healthy { 1 } else { 0 })
			.unwrap_or(0);

		self.last_ping_timestamp_seconds
			.set(last_ping_timestamp_seconds);
		self.ping_healthy.set(ping_healthy);
	}
}

fn register_metric<M>(registry: &Registry, metric: M)
where
	M: rivet_metrics::prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	if let Err(error) = registry.register(Box::new(metric)) {
		tracing::warn!(
			?error,
			"envoy metric registration failed, using existing collector"
		);
	}
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
