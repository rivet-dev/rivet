use std::{collections::HashMap, sync::LazyLock};

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rivet_metrics::prometheus::{
	Encoder, IntGaugeVec, TextEncoder, register_int_gauge_vec_with_registry,
};
use subtle::ConstantTimeEq;

const METRICS_ENABLED_ENV: &str = "RIVETKIT_METRICS_ENABLED";
const METRICS_TOKEN_ENV: &str = "RIVETKIT_METRICS_TOKEN";

static RIVETKIT_INFO: LazyLock<IntGaugeVec> = LazyLock::new(|| {
	register_int_gauge_vec_with_registry!(
		"rivetkit_info",
		"Static RivetKit build information.",
		&[
			"runtime",
			"version",
			"type",
			"envoy_version",
			"envoy_kind",
			"pool_name",
		],
		*rivet_metrics::REGISTRY
	)
	.unwrap()
});

static CURRENT_RIVETKIT_INFO: LazyLock<Mutex<Option<RivetKitInfo>>> =
	LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug, Eq, PartialEq)]
struct RivetKitInfo {
	runtime: String,
	version: String,
	runtime_type: String,
	envoy_version: String,
	envoy_kind: String,
	pool_name: String,
}

impl RivetKitInfo {
	fn labels(&self) -> [&str; 6] {
		[
			&self.runtime,
			&self.version,
			&self.runtime_type,
			&self.envoy_version,
			&self.envoy_kind,
			&self.pool_name,
		]
	}
}

pub struct RenderedMetrics {
	pub content_type: String,
	pub body: Vec<u8>,
}

pub enum MetricsAccessError {
	NotEnabled,
	Unauthorized,
}

pub fn runtime_type() -> &'static str {
	if std::env::var("NODE_ENV").as_deref() == Ok("production") {
		"deployed"
	} else {
		"local"
	}
}

pub fn record_rivetkit_info(
	version: impl Into<String>,
	envoy_version: u32,
	envoy_kind: impl Into<String>,
	pool_name: impl Into<String>,
) {
	record_rivetkit_info_inner(RivetKitInfo {
		runtime: "rivetkit".to_owned(),
		version: version.into(),
		runtime_type: runtime_type().to_owned(),
		envoy_version: envoy_version.to_string(),
		envoy_kind: envoy_kind.into(),
		pool_name: pool_name.into(),
	});
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

pub fn render_prometheus_metrics() -> Result<RenderedMetrics> {
	ensure_rivetkit_info_recorded();

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

fn ensure_rivetkit_info_recorded() {
	if CURRENT_RIVETKIT_INFO.lock().is_some() {
		return;
	}

	let envoy_version = std::env::var("RIVET_ENVOY_VERSION")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(1);
	let pool_name = std::env::var("RIVET_POOL_NAME").unwrap_or_else(|_| "rivetkit-rust".to_owned());
	record_rivetkit_info(
		env!("CARGO_PKG_VERSION"),
		envoy_version,
		"unknown",
		pool_name,
	);
}

fn record_rivetkit_info_inner(info: RivetKitInfo) {
	let mut current = CURRENT_RIVETKIT_INFO.lock();
	if let Some(previous) = current.as_ref()
		&& previous != &info
	{
		RIVETKIT_INFO.with_label_values(&previous.labels()).set(0);
	}

	RIVETKIT_INFO.with_label_values(&info.labels()).set(1);
	*current = Some(info);
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
