use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use prometheus::{
	CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, IntCounter,
	IntGauge, Opts, Registry, TextEncoder,
};

#[derive(Clone)]
pub(crate) struct ActorMetrics(Arc<ActorMetricsInner>);

struct ActorMetricsInner {
	registry: Registry,
	create_state_ms: Gauge,
	on_migrate_ms: Gauge,
	on_wake_ms: Gauge,
	create_vars_ms: Gauge,
	total_startup_ms: Gauge,
	action_call_total: CounterVec,
	action_error_total: CounterVec,
	action_duration_seconds: HistogramVec,
	queue_depth: IntGauge,
	queue_messages_sent_total: IntCounter,
	queue_messages_received_total: IntCounter,
	active_connections: IntGauge,
	connections_total: IntCounter,
}

impl ActorMetrics {
	pub(crate) fn new(actor_id: impl Into<String>, actor_name: impl Into<String>) -> Self {
		let registry = Registry::new_custom(
			None,
			Some(HashMap::from([
				("actor_id".to_owned(), actor_id.into()),
				("actor_name".to_owned(), actor_name.into()),
			])),
		)
		.expect("create actor metrics registry");

		let create_state_ms = Gauge::with_opts(Opts::new(
			"create_state_ms",
			"time spent creating typed actor state during startup",
		))
		.expect("create create_state_ms gauge");
		let on_migrate_ms = Gauge::with_opts(Opts::new(
			"on_migrate_ms",
			"time spent running actor on_migrate during startup",
		))
		.expect("create on_migrate_ms gauge");
		let on_wake_ms = Gauge::with_opts(Opts::new(
			"on_wake_ms",
			"time spent running actor on_wake during startup",
		))
		.expect("create on_wake_ms gauge");
		let create_vars_ms = Gauge::with_opts(Opts::new(
			"create_vars_ms",
			"time spent creating typed actor vars during startup",
		))
		.expect("create create_vars_ms gauge");
		let total_startup_ms = Gauge::with_opts(Opts::new(
			"total_startup_ms",
			"total actor startup time for the current wake cycle",
		))
		.expect("create total_startup_ms gauge");
		let action_call_total = CounterVec::new(
			Opts::new("action_call_total", "total actor action calls"),
			&["action"],
		)
		.expect("create action_call_total counter");
		let action_error_total = CounterVec::new(
			Opts::new("action_error_total", "total actor action errors"),
			&["action"],
		)
		.expect("create action_error_total counter");
		let action_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"action_duration_seconds",
				"actor action execution time in seconds",
			),
			&["action"],
		)
		.expect("create action_duration_seconds histogram");
		let queue_depth = IntGauge::with_opts(Opts::new(
			"queue_depth",
			"current actor queue depth",
		))
		.expect("create queue_depth gauge");
		let queue_messages_sent_total = IntCounter::with_opts(Opts::new(
			"queue_messages_sent_total",
			"total queue messages sent",
		))
		.expect("create queue_messages_sent_total counter");
		let queue_messages_received_total = IntCounter::with_opts(Opts::new(
			"queue_messages_received_total",
			"total queue messages received",
		))
		.expect("create queue_messages_received_total counter");
		let active_connections = IntGauge::with_opts(Opts::new(
			"active_connections",
			"current active actor connections",
		))
		.expect("create active_connections gauge");
		let connections_total = IntCounter::with_opts(Opts::new(
			"connections_total",
			"total successfully established actor connections",
		))
		.expect("create connections_total counter");

		register_metric(&registry, create_state_ms.clone());
		register_metric(&registry, on_migrate_ms.clone());
		register_metric(&registry, on_wake_ms.clone());
		register_metric(&registry, create_vars_ms.clone());
		register_metric(&registry, total_startup_ms.clone());
		register_metric(&registry, action_call_total.clone());
		register_metric(&registry, action_error_total.clone());
		register_metric(&registry, action_duration_seconds.clone());
		register_metric(&registry, queue_depth.clone());
		register_metric(&registry, queue_messages_sent_total.clone());
		register_metric(&registry, queue_messages_received_total.clone());
		register_metric(&registry, active_connections.clone());
		register_metric(&registry, connections_total.clone());

		Self(Arc::new(ActorMetricsInner {
			registry,
			create_state_ms,
			on_migrate_ms,
			on_wake_ms,
			create_vars_ms,
			total_startup_ms,
			action_call_total,
			action_error_total,
			action_duration_seconds,
			queue_depth,
			queue_messages_sent_total,
			queue_messages_received_total,
			active_connections,
			connections_total,
		}))
	}

	pub(crate) fn render(&self) -> Result<String> {
		let metric_families = self.0.registry.gather();
		let mut encoded = Vec::new();
		TextEncoder::new()
			.encode(&metric_families, &mut encoded)
			.context("encode actor metrics in prometheus text format")?;
		String::from_utf8(encoded).context("actor metrics are not valid utf-8")
	}

	pub(crate) fn metrics_content_type(&self) -> String {
		TextEncoder::new().format_type().to_owned()
	}

	pub(crate) fn observe_create_state(&self, duration: Duration) {
		self.0.create_state_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_on_migrate(&self, duration: Duration) {
		self.0.on_migrate_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_on_wake(&self, duration: Duration) {
		self.0.on_wake_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		self.0.create_vars_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_total_startup(&self, duration: Duration) {
		self.0.total_startup_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_action_call(&self, action_name: &str) {
		self.0
			.action_call_total
			.with_label_values(&[action_name])
			.inc();
	}

	pub(crate) fn observe_action_error(&self, action_name: &str) {
		self.0
			.action_error_total
			.with_label_values(&[action_name])
			.inc();
	}

	pub(crate) fn observe_action_duration(&self, action_name: &str, duration: Duration) {
		self.0
			.action_duration_seconds
			.with_label_values(&[action_name])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn set_queue_depth(&self, depth: u32) {
		self.0.queue_depth.set(i64::from(depth));
	}

	pub(crate) fn add_queue_messages_sent(&self, count: u64) {
		self.0.queue_messages_sent_total.inc_by(count);
	}

	pub(crate) fn add_queue_messages_received(&self, count: u64) {
		self.0.queue_messages_received_total.inc_by(count);
	}

	pub(crate) fn set_active_connections(&self, count: usize) {
		self.0
			.active_connections
			.set(count.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_connections_total(&self) {
		self.0.connections_total.inc();
	}
}

impl Default for ActorMetrics {
	fn default() -> Self {
		Self::new("", "")
	}
}

impl fmt::Debug for ActorMetrics {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorMetrics").finish()
	}
}

fn duration_ms(duration: Duration) -> f64 {
	duration.as_secs_f64() * 1000.0
}

fn register_metric<M>(registry: &Registry, metric: M)
where
	M: prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	registry
		.register(Box::new(metric))
		.expect("register actor metric");
}
