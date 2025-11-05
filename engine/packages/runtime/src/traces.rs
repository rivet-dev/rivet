// Based off of https://github.com/tokio-rs/tracing-opentelemetry/blob/v0.1.x/examples/opentelemetry-otlp.rs

use console_subscriber;
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use rivet_metrics::OtelProviderGuard;
use std::sync::OnceLock;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, reload, util::SubscriberInitExt};

type ReloadHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

static RELOAD_HANDLE: OnceLock<ReloadHandle> = OnceLock::new();

/// Initialize tracing-subscriber
pub fn init_tracing_subscriber(otel_providers: &Option<OtelProviderGuard>) {
	// Create reloadable env filter for RUST_LOG
	let (reload_layer, reload_handle) = reload::Layer::new(build_filter_from_env_var("RUST_LOG"));

	// Store handle globally for later reloading
	let _ = RELOAD_HANDLE.set(reload_handle);

	let registry = tracing_subscriber::registry();

	// Build and apply otel layers to the registry if otel is enabled
	let (otel_trace_layer, otel_metric_layer) = match otel_providers {
		Some(providers) => {
			let tracer = providers.tracer_provider.tracer("tracing-otel-subscriber");

			let otel_trace_layer = OpenTelemetryLayer::new(tracer)
				.with_filter(build_filter_from_env_var("RUST_TRACE"));

			let otel_metric_layer = MetricsLayer::new(providers.meter_provider.clone())
				.with_filter(build_filter_from_env_var("RUST_TRACE"));

			(Some(otel_trace_layer), Some(otel_metric_layer))
		}
		None => (None, None),
	};

	let registry = registry
		.with(reload_layer)
		.with(otel_metric_layer)
		.with(otel_trace_layer)
		.with(sentry::integrations::tracing::layer())
		.with(SentryOtelLayer);

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
				.layer(),
		)
		.init()
}

/// Replaces sentry's trace id with otel's trace id
struct SentryOtelLayer;

impl<S> Layer<S> for SentryOtelLayer
where
	S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
	fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
		if let Some(span) = ctx.event_span(event) {
			// The OTel layer stores the OTel context in span extensions
			let extensions = span.extensions();

			if let Some(otel_data) = extensions.get::<tracing_opentelemetry::OtelData>() {
				let span = otel_data.parent_cx.span();
				let span_context = span.span_context();
				let trace_id = if span_context.is_valid() {
					Some(span_context.trace_id())
				} else {
					otel_data.builder.trace_id
				};

				if let (Some(trace_id), Some(span_id)) = (trace_id, otel_data.builder.span_id) {
					sentry::configure_scope(|scope| {
						scope.set_context(
							"trace",
							sentry::protocol::TraceContext {
								trace_id: trace_id.to_bytes().into(),
								span_id: span_id.to_bytes().into(),
								..Default::default()
							},
						);
					});
				}
			}
		}
	}
}

/// Build an EnvFilter from a filter specification string
fn build_filter_from_spec(filter_spec: &str) -> anyhow::Result<EnvFilter> {
	// Create env filter with defaults
	let mut env_filter = EnvFilter::default()
		// Default filter
		.add_directive("info".parse()?)
		// Disable verbose logs
		.add_directive("tokio_cron_scheduler=warn".parse()?)
		.add_directive("tokio=warn".parse()?)
		.add_directive("hyper=warn".parse()?)
		.add_directive("h2=warn".parse()?);

	// Add user-provided directives
	for s in filter_spec.split(',').filter(|x| !x.is_empty()) {
		env_filter = env_filter.add_directive(s.parse()?);
	}

	Ok(env_filter)
}

/// Build an EnvFilter by reading from an environment variable
fn build_filter_from_env_var(env_var_name: &str) -> EnvFilter {
	let filter_spec = std::env::var(env_var_name).unwrap_or_default();
	build_filter_from_spec(&filter_spec).expect("invalid env filter")
}

/// Reload the log filter with a new specification
pub fn reload_log_filter(filter_spec: &str) -> anyhow::Result<()> {
	let handle = RELOAD_HANDLE
		.get()
		.ok_or_else(|| anyhow::anyhow!("reload handle not initialized"))?;

	// Build the new filter
	let env_filter = build_filter_from_spec(filter_spec)?;

	// Reload the filter
	handle.reload(env_filter)?;

	tracing::debug!(?filter_spec, "reloaded log filter");

	Ok(())
}
