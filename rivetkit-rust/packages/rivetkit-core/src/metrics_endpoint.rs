use std::sync::LazyLock;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rivet_metrics::prometheus::{
	Encoder, IntGaugeVec, TextEncoder, register_int_gauge_vec_with_registry,
};

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
	// Startup paths reach this from inside the runtime, which is the only place
	// a handle to sample can be taken.
	#[cfg(not(target_arch = "wasm32"))]
	crate::tokio_runtime_metrics::ensure_sampler_started();

	let mut current = CURRENT_RIVETKIT_INFO.lock();
	if let Some(previous) = current.as_ref()
		&& previous != &info
	{
		RIVETKIT_INFO.with_label_values(&previous.labels()).set(0);
	}

	RIVETKIT_INFO.with_label_values(&info.labels()).set(1);
	*current = Some(info);
}
