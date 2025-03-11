// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	logs::SdkLoggerProvider,
	metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
	trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
	Resource,
};
use opentelemetry_semantic_conventions::{attribute::SERVICE_VERSION, SCHEMA_URL};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn resource() -> Resource {
	Resource::builder()
		.with_service_name(rivet_env::service_name())
		.with_schema_url(
			[KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
			SCHEMA_URL,
		)
		.build()
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
			1.0,
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

// TODO: This causes the runtime to hang, unsure why
//fn init_logger_provider() -> SdkLoggerProvider {
//	let exporter = opentelemetry_otlp::LogExporter::builder()
//		.with_tonic()
//		.with_protocol(opentelemetry_otlp::Protocol::Grpc)
//		.with_endpoint(otel_endpoint())
//		.build()
//		.unwrap();
//
//	SdkLoggerProvider::builder()
//		.with_resource(resource())
//		//.with_batch_exporter(exporter)
//		.with_simple_exporter(exporter)
//		.build()
//}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
pub fn init_tracing_subscriber() -> OtelGuard {
	let tracer_provider = init_tracer_provider();
	let meter_provider = init_meter_provider();
	//let logger_provider = init_logger_provider();

	let tracer = tracer_provider.tracer("tracing-otel-subscriber");

	// For the OpenTelemetry layer, add a tracing filter to filter events from
	// OpenTelemetry and its dependent crates (opentelemetry-otlp uses crates
	// like reqwest/tonic etc.) from being sent back to OTel itself, thus
	// preventing infinite telemetry generation. The filter levels are set as
	// follows:
	// - Allow `info` level and above by default.
	// - Restrict `opentelemetry`, `hyper`, `tonic`, and `reqwest` completely.
	// Note: This will also drop events from crates like `tonic` etc. even when
	// they are used outside the OTLP Exporter. For more details, see:
	// https://github.com/open-telemetry/opentelemetry-rust/issues/761
	//let filter_otel = EnvFilter::new("info")
	//	.add_directive("hyper=off".parse().unwrap())
	//	.add_directive("opentelemetry=off".parse().unwrap())
	//	.add_directive("tonic=off".parse().unwrap())
	//	.add_directive("h2=off".parse().unwrap())
	//	.add_directive("reqwest=off".parse().unwrap());
	//let logger =
	//	opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider)
	//		.with_filter(filter_otel);

	// Create logfmt logger
	let logfmt_layer = tracing_logfmt::builder()
		.with_span_name(std::env::var("RUST_LOG_SPAN_NAME").map_or(false, |x| x == "1"))
		.with_span_path(std::env::var("RUST_LOG_SPAN_PATH").map_or(false, |x| x == "1"))
		.with_target(std::env::var("RUST_LOG_TARGET").map_or(false, |x| x == "1"))
		.with_location(std::env::var("RUST_LOG_LOCATION").map_or(false, |x| x == "1"))
		.with_module_path(std::env::var("RUST_LOG_MODULE_PATH").map_or(false, |x| x == "1"))
		.with_ansi_color(std::env::var("RUST_LOG_ANSI_COLOR").map_or(false, |x| x == "1"))
		.layer();

	// Create env filter
	let mut env_filter = EnvFilter::default()
		// Default filter
		.add_directive("info".parse().unwrap())
		// Disable verbose logs
		.add_directive("tokio_cron_scheduler=warn".parse().unwrap());

	if let Ok(filter) = std::env::var("RUST_LOG") {
		for s in filter.split(',').filter(|x| !x.is_empty()) {
			env_filter = env_filter.add_directive(s.parse().expect("invalid env filter"));
		}
	}

	tracing_subscriber::registry()
		.with(env_filter)
		.with(OpenTelemetryLayer::new(tracer))
		.with(MetricsLayer::new(meter_provider.clone()))
		//.with(logger)
		.with(logfmt_layer)
		.init();

	OtelGuard {
		tracer_provider,
		meter_provider,
		//logger_provider,
	}
}

pub struct OtelGuard {
	tracer_provider: SdkTracerProvider,
	meter_provider: SdkMeterProvider,
	//logger_provider: SdkLoggerProvider,
}

impl Drop for OtelGuard {
	fn drop(&mut self) {
		if let Err(err) = self.tracer_provider.shutdown() {
			eprintln!("{err:?}");
		}
		if let Err(err) = self.meter_provider.shutdown() {
			eprintln!("{err:?}");
		}
		//if let Err(err) = self.logger_provider.shutdown() {
		//	eprintln!("{err:?}");
		//}
	}
}
