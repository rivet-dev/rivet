// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use console_subscriber;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
	trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
	Resource,
};
use opentelemetry_semantic_conventions::{attribute::SERVICE_VERSION, SCHEMA_URL};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

fn resource() -> Resource {
	let mut resource = Resource::builder()
		.with_service_name(rivet_env::service_name())
		.with_schema_url(
			[KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
			SCHEMA_URL,
		);

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

fn otel_endpoint() -> String {
	std::env::var("RIVET_OTEL_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string())
}

fn init_tracer_provider() -> SdkTracerProvider {
	let exporter = opentelemetry_otlp::SpanExporter::builder()
		.with_tonic()
		.with_protocol(opentelemetry_otlp::Protocol::Grpc)
		.with_endpoint(otel_endpoint())
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
		.with_endpoint(otel_endpoint())
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

// TODO: Ugly function
/// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing.
pub fn init_tracing_subscriber() -> Option<OtelGuard> {
	let registry = tracing_subscriber::registry();

	// Check if otel is enabled
	let enable_otel = std::env::var("RIVET_OTEL_ENABLED").map_or(false, |x| x == "1");

	// Check if tokio console is enabled
	let enable_tokio_console = std::env::var("TOKIO_CONSOLE_ENABLE").map_or(false, |x| x == "1");

	// This macro exists because .layer() has weird type semantics
	macro_rules! logfmt_layer {
		() => {
			tracing_logfmt::builder()
				.with_span_name(std::env::var("RUST_LOG_SPAN_NAME").map_or(false, |x| x == "1"))
				.with_span_path(std::env::var("RUST_LOG_SPAN_PATH").map_or(false, |x| x == "1"))
				.with_target(std::env::var("RUST_LOG_TARGET").map_or(false, |x| x == "1"))
				.with_location(std::env::var("RUST_LOG_LOCATION").map_or(false, |x| x == "1"))
				.with_module_path(std::env::var("RUST_LOG_MODULE_PATH").map_or(false, |x| x == "1"))
				.with_ansi_color(std::env::var("RUST_LOG_ANSI_COLOR").map_or(false, |x| x == "1"))
				.layer()
				.with_filter(env_filter("RUST_LOG"))
		};
	}

	if enable_otel {
		let tracer_provider = init_tracer_provider();
		let meter_provider = init_meter_provider();
		let tracer = tracer_provider.tracer("tracing-otel-subscriber");

		let registry = registry
			.with(OpenTelemetryLayer::new(tracer).with_filter(env_filter("RUST_TRACE")))
			.with(MetricsLayer::new(meter_provider.clone()).with_filter(env_filter("RUST_TRACE")));

		if enable_tokio_console {
			let console_layer = console_subscriber::ConsoleLayer::builder()
				.with_default_env()
				.spawn();

			registry.with(console_layer).with(logfmt_layer!()).init();
		} else {
			registry.with(logfmt_layer!()).init();
		}

		Some(OtelGuard {
			tracer_provider,
			meter_provider,
		})
	} else {
		if enable_tokio_console {
			let console_layer = console_subscriber::ConsoleLayer::builder()
				.with_default_env()
				.spawn();

			registry.with(console_layer).with(logfmt_layer!()).init();
		} else {
			registry.with(logfmt_layer!()).init();
		}

		None
	}
}

pub struct OtelGuard {
	tracer_provider: SdkTracerProvider,
	meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
	fn drop(&mut self) {
		if let Err(err) = self.tracer_provider.shutdown() {
			eprintln!("{err:?}");
		}
		if let Err(err) = self.meter_provider.shutdown() {
			eprintln!("{err:?}");
		}
	}
}

fn env_filter(env_var: &str) -> EnvFilter {
	// Create env filter
	let mut env_filter = EnvFilter::default()
		// Default filter
		.add_directive("info".parse().unwrap())
		// Disable verbose logs
		.add_directive("tokio_cron_scheduler=warn".parse().unwrap())
		.add_directive("tokio=warn".parse().unwrap())
		.add_directive("hyper=warn".parse().unwrap())
		.add_directive("h2=warn".parse().unwrap());

	if let Ok(filter) = std::env::var(env_var) {
		for s in filter.split(',').filter(|x| !x.is_empty()) {
			env_filter = env_filter.add_directive(s.parse().expect("invalid env filter"));
		}
	}

	env_filter
}
