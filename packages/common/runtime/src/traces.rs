// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use console_subscriber;
use opentelemetry::trace::TracerProvider as _;
use rivet_metrics::OtelProviderGuard;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing-subscriber
pub fn init_tracing_subscriber(otel_providers: &Option<OtelProviderGuard>) {
	let registry = tracing_subscriber::registry();

	// Build and apply otel layers to the registry if otel is enabled
	let (otel_trace_layer, otel_metric_layer) = match otel_providers {
		Some(providers) => {
			let tracer = providers.tracer_provider.tracer("tracing-otel-subscriber");

			let otel_trace_layer =
				OpenTelemetryLayer::new(tracer).with_filter(env_filter("RUST_TRACE"));

			let otel_metric_layer = MetricsLayer::new(providers.meter_provider.clone())
				.with_filter(env_filter("RUST_TRACE"));

			(Some(otel_trace_layer), Some(otel_metric_layer))
		}
		None => (None, None),
	};

	let registry = registry.with(otel_metric_layer).with(otel_trace_layer);

	// Check if tokio console is enabled
	let enable_tokio_console = std::env::var("TOKIO_CONSOLE_ENABLE").map_or(false, |x| x == "1");

	registry
		.with(
			// Add tokio console if its enabled
			//
			// This code is here because console layer depends
			// on tracing_subscriber's weird layered registry type.
			if enable_tokio_console {
				Some(
					console_subscriber::ConsoleLayer::builder()
						.with_default_env()
						.spawn(),
				)
			} else {
				None
			},
		)
		.with(
			tracing_logfmt::builder()
				.with_span_name(std::env::var("RUST_LOG_SPAN_NAME").map_or(false, |x| x == "1"))
				.with_span_path(std::env::var("RUST_LOG_SPAN_PATH").map_or(false, |x| x == "1"))
				.with_target(std::env::var("RUST_LOG_TARGET").map_or(false, |x| x == "1"))
				.with_location(std::env::var("RUST_LOG_LOCATION").map_or(false, |x| x == "1"))
				.with_module_path(std::env::var("RUST_LOG_MODULE_PATH").map_or(false, |x| x == "1"))
				.with_ansi_color(std::env::var("RUST_LOG_ANSI_COLOR").map_or(false, |x| x == "1"))
				.layer()
				.with_filter(env_filter("RUST_LOG")),
		)
		.init()
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
