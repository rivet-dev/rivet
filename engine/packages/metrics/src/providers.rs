// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs
// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use opentelemetry::trace::{SamplingResult, SpanKind};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	Resource,
	metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
	trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};
use std::sync::{Arc, OnceLock, RwLock};

/// Dynamic sampler that can be updated at runtime
#[derive(Clone, Debug)]
struct DynamicSampler {
	ratio: Arc<RwLock<f64>>,
}

impl DynamicSampler {
	fn new(ratio: f64) -> Self {
		Self {
			ratio: Arc::new(RwLock::new(ratio)),
		}
	}

	fn set_ratio(&self, ratio: f64) {
		if let Ok(mut r) = self.ratio.write() {
			*r = ratio;
		}
	}
}

impl opentelemetry_sdk::trace::ShouldSample for DynamicSampler {
	fn should_sample(
		&self,
		parent_context: Option<&opentelemetry::Context>,
		trace_id: opentelemetry::trace::TraceId,
		_name: &str,
		_span_kind: &SpanKind,
		_attributes: &[KeyValue],
		_links: &[opentelemetry::trace::Link],
	) -> SamplingResult {
		let ratio = self.ratio.read().ok().map(|r| *r).unwrap_or(0.001);

		// Use TraceIdRatioBased sampling logic
		let sampler = Sampler::TraceIdRatioBased(ratio);
		sampler.should_sample(
			parent_context,
			trace_id,
			_name,
			_span_kind,
			_attributes,
			_links,
		)
	}
}

static SAMPLER: OnceLock<DynamicSampler> = OnceLock::new();

/// Update the sampler ratio at runtime
pub fn set_sampler_ratio(ratio: f64) -> anyhow::Result<()> {
	let sampler = SAMPLER
		.get()
		.ok_or_else(|| anyhow::anyhow!("sampler not initialized"))?;

	sampler.set_ratio(ratio);
	tracing::debug!(?ratio, "updated sampler ratio");

	Ok(())
}

fn resource() -> Resource {
	let resource = Resource::builder()
		.with_service_name(rivet_env::service_name())
		.with_schema_url(
			[KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
			SCHEMA_URL,
		);

	resource.build()
}

fn otel_grpc_endpoint() -> String {
	std::env::var("RIVET_OTEL_GRPC_ENDPOINT")
		.unwrap_or_else(|_| "http://localhost:4317".to_string())
}

fn init_tracer_provider() -> SdkTracerProvider {
	let exporter = opentelemetry_otlp::SpanExporter::builder()
		.with_tonic()
		.with_protocol(opentelemetry_otlp::Protocol::Grpc)
		.with_endpoint(otel_grpc_endpoint())
		.build()
		.unwrap();

	// Create dynamic sampler with initial ratio from env
	let initial_ratio = std::env::var("RIVET_OTEL_SAMPLER_RATIO")
		.ok()
		.and_then(|s| s.parse::<f64>().ok())
		.unwrap_or(0.001);

	let dynamic_sampler = DynamicSampler::new(initial_ratio);

	// Store sampler globally for later updates
	let _ = SAMPLER.set(dynamic_sampler.clone());

	SdkTracerProvider::builder()
		// Customize sampling strategy with parent-based sampling using our dynamic sampler
		.with_sampler(Sampler::ParentBased(Box::new(dynamic_sampler)))
		// If export trace to AWS X-Ray, you can use XrayIdGenerator
		.with_id_generator(RandomIdGenerator::default())
		.with_resource(resource())
		.with_batch_exporter(exporter)
		.build()
}

fn init_meter_provider() -> SdkMeterProvider {
	let exporter = opentelemetry_otlp::MetricExporter::builder()
		.with_tonic()
		.with_temporality(opentelemetry_sdk::metrics::Temporality::Cumulative)
		.with_protocol(opentelemetry_otlp::Protocol::Grpc)
		.with_endpoint(otel_grpc_endpoint())
		.build()
		.unwrap();

	let reader = PeriodicReader::builder(exporter)
		.with_interval(std::time::Duration::from_secs(30))
		.build();

	// // For debugging in development
	// let stdout_reader =
	//     PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default()).build();

	let meter_provider = MeterProviderBuilder::default()
		.with_resource(resource())
		.with_reader(reader)
		// .with_reader(stdout_reader)
		.build();

	global::set_meter_provider(meter_provider.clone());

	meter_provider
}

/// Initialize OtelProviderGuard for opentelemetry-related termination processing.
pub fn init_otel_providers() -> Option<OtelProviderGuard> {
	// Check if otel is enabled
	let enable_otel = std::env::var("RIVET_OTEL_ENABLED").map_or(false, |x| x == "1");

	if enable_otel {
		let tracer_provider = init_tracer_provider();
		let meter_provider = init_meter_provider();

		Some(OtelProviderGuard {
			tracer_provider,
			meter_provider,
		})
	} else {
		// NOTE: OTEL's global::meters are no-op without
		// a meter provider configured, so its safe to
		// not set any meter provider
		None
	}
}

/// Guard opentelemetry-related providers termination processing.
pub struct OtelProviderGuard {
	pub tracer_provider: SdkTracerProvider,
	pub meter_provider: SdkMeterProvider,
}

impl Drop for OtelProviderGuard {
	fn drop(&mut self) {
		if let Err(err) = self.tracer_provider.shutdown() {
			eprintln!("{err:?}");
		}
		if let Err(err) = self.meter_provider.shutdown() {
			eprintln!("{err:?}");
		}
	}
}
