// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs
// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	Resource,
	metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
	trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};

fn resource() -> Resource {
	let mut resource = Resource::builder()
		.with_service_name(rivet_env::service_name())
		.with_schema_url(
			[KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
			SCHEMA_URL,
		);

	if let Ok(v) = std::env::var("RIVET_NAMESPACE") {
		resource = resource.with_attribute(KeyValue::new("namespace", v));
	}
	if let Ok(v) = std::env::var("RIVET_CLUSTER_ID") {
		resource = resource.with_attribute(KeyValue::new("cluster_id", v));
	}
	if let Ok(v) = std::env::var("RIVET_DATACENTER_ID") {
		resource = resource.with_attribute(KeyValue::new("datacenter_id", v));
	}
	if let Ok(v) = std::env::var("RIVET_SERVER_ID") {
		resource = resource.with_attribute(KeyValue::new("server_id", v));
	}

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

	SdkTracerProvider::builder()
		// Customize sampling strategy
		.with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
			std::env::var("RIVET_OTEL_SAMPLER_RATIO")
				.ok()
				.and_then(|s| s.parse::<f64>().ok())
				.unwrap_or(0.001),
		))))
		// If export trace to AWS X-Ray, you can use XrayIdGenerator
		.with_id_generator(RandomIdGenerator::default())
		.with_resource(resource())
		.with_batch_exporter(exporter)
		.build()
}

fn init_meter_provider() -> SdkMeterProvider {
	let exporter = opentelemetry_otlp::MetricExporter::builder()
		.with_tonic()
		.with_temporality(opentelemetry_sdk::metrics::Temporality::default())
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
