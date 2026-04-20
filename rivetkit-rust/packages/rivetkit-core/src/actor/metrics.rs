use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use prometheus::{
	CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, IntCounter,
	IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};

use crate::actor::task_types::{StateMutationReason, StopReason, UserTaskKind};

#[derive(Clone)]
pub(crate) struct ActorMetrics(Arc<ActorMetricsInner>);

struct ActorMetricsInner {
	actor_id: String,
	registry: Registry,
	create_state_ms: Gauge,
	create_vars_ms: Gauge,
	queue_depth: IntGauge,
	queue_messages_sent_total: IntCounter,
	queue_messages_received_total: IntCounter,
	active_connections: IntGauge,
	connections_total: IntCounter,
	lifecycle_inbox_depth: IntGauge,
	lifecycle_inbox_overload_total: CounterVec,
	dispatch_inbox_depth: IntGauge,
	dispatch_inbox_overload_total: CounterVec,
	lifecycle_event_inbox_depth: IntGauge,
	lifecycle_event_overload_total: CounterVec,
	user_tasks_active: IntGaugeVec,
	user_task_duration_seconds: HistogramVec,
	shutdown_wait_seconds: HistogramVec,
	shutdown_timeout_total: CounterVec,
	state_mutation_total: CounterVec,
	state_mutation_overload_total: CounterVec,
	direct_subsystem_shutdown_warning_total: CounterVec,
}

impl ActorMetrics {
	pub(crate) fn new(actor_id: impl Into<String>, actor_name: impl Into<String>) -> Self {
		let actor_id = actor_id.into();
		let registry = Registry::new_custom(
			None,
			Some(HashMap::from([
				("actor_id".to_owned(), actor_id.clone()),
				("actor_name".to_owned(), actor_name.into()),
			])),
		)
		.expect("create actor metrics registry");

		let create_state_ms = Gauge::with_opts(Opts::new(
			"create_state_ms",
			"time spent creating typed actor state during startup",
		))
		.expect("create create_state_ms gauge");
		let create_vars_ms = Gauge::with_opts(Opts::new(
			"create_vars_ms",
			"time spent creating typed actor vars during startup",
		))
		.expect("create create_vars_ms gauge");
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
		let lifecycle_inbox_depth = IntGauge::with_opts(Opts::new(
			"lifecycle_inbox_depth",
			"current actor lifecycle command inbox depth",
		))
		.expect("create lifecycle_inbox_depth gauge");
		let lifecycle_inbox_overload_total = CounterVec::new(
			Opts::new(
				"lifecycle_inbox_overload_total",
				"total actor lifecycle command inbox overloads",
			),
			&["command"],
		)
		.expect("create lifecycle_inbox_overload_total counter");
		let dispatch_inbox_depth = IntGauge::with_opts(Opts::new(
			"dispatch_inbox_depth",
			"current actor dispatch command inbox depth",
		))
		.expect("create dispatch_inbox_depth gauge");
		let dispatch_inbox_overload_total = CounterVec::new(
			Opts::new(
				"dispatch_inbox_overload_total",
				"total actor dispatch command inbox overloads",
			),
			&["command"],
		)
		.expect("create dispatch_inbox_overload_total counter");
		let lifecycle_event_inbox_depth = IntGauge::with_opts(Opts::new(
			"lifecycle_event_inbox_depth",
			"current actor lifecycle event inbox depth",
		))
		.expect("create lifecycle_event_inbox_depth gauge");
		let lifecycle_event_overload_total = CounterVec::new(
			Opts::new(
				"lifecycle_event_overload_total",
				"total actor lifecycle event inbox overloads",
			),
			&["event"],
		)
		.expect("create lifecycle_event_overload_total counter");
		let user_tasks_active = IntGaugeVec::new(
			Opts::new("user_tasks_active", "current active actor user tasks"),
			&["kind"],
		)
		.expect("create user_tasks_active gauge");
		let user_task_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"user_task_duration_seconds",
				"actor user task execution time in seconds",
			),
			&["kind"],
		)
		.expect("create user_task_duration_seconds histogram");
		let shutdown_wait_seconds = HistogramVec::new(
			HistogramOpts::new(
				"shutdown_wait_seconds",
				"actor shutdown wait time in seconds",
			),
			&["reason"],
		)
		.expect("create shutdown_wait_seconds histogram");
		let shutdown_timeout_total = CounterVec::new(
			Opts::new(
				"shutdown_timeout_total",
				"total actor shutdown timeout events",
			),
			&["reason"],
		)
		.expect("create shutdown_timeout_total counter");
		let state_mutation_total = CounterVec::new(
			Opts::new("state_mutation_total", "total actor state mutations"),
			&["reason"],
		)
		.expect("create state_mutation_total counter");
		let state_mutation_overload_total = CounterVec::new(
			Opts::new(
				"state_mutation_overload_total",
				"total actor state mutations rejected by lifecycle event overload",
			),
			&["reason"],
		)
		.expect("create state_mutation_overload_total counter");
		let direct_subsystem_shutdown_warning_total = CounterVec::new(
			Opts::new(
				"direct_subsystem_shutdown_warning_total",
				"total actor shutdown warnings emitted by direct subsystem drains",
			),
			&["subsystem", "operation"],
		)
		.expect("create direct_subsystem_shutdown_warning_total counter");

		register_metric(&registry, create_state_ms.clone());
		register_metric(&registry, create_vars_ms.clone());
		register_metric(&registry, queue_depth.clone());
		register_metric(&registry, queue_messages_sent_total.clone());
		register_metric(&registry, queue_messages_received_total.clone());
		register_metric(&registry, active_connections.clone());
		register_metric(&registry, connections_total.clone());
		register_metric(&registry, lifecycle_inbox_depth.clone());
		register_metric(&registry, lifecycle_inbox_overload_total.clone());
		register_metric(&registry, dispatch_inbox_depth.clone());
		register_metric(&registry, dispatch_inbox_overload_total.clone());
		register_metric(&registry, lifecycle_event_inbox_depth.clone());
		register_metric(&registry, lifecycle_event_overload_total.clone());
		register_metric(&registry, user_tasks_active.clone());
		register_metric(&registry, user_task_duration_seconds.clone());
		register_metric(&registry, shutdown_wait_seconds.clone());
		register_metric(&registry, shutdown_timeout_total.clone());
		register_metric(&registry, state_mutation_total.clone());
		register_metric(&registry, state_mutation_overload_total.clone());
		register_metric(
			&registry,
			direct_subsystem_shutdown_warning_total.clone(),
		);

		for kind in UserTaskKind::ALL {
			user_tasks_active
				.with_label_values(&[kind.as_metric_label()])
				.set(0);
			user_task_duration_seconds
				.with_label_values(&[kind.as_metric_label()]);
		}
		for reason in StateMutationReason::ALL {
			state_mutation_total
				.with_label_values(&[reason.as_metric_label()]);
			state_mutation_overload_total
				.with_label_values(&[reason.as_metric_label()]);
		}
		for reason in [StopReason::Sleep, StopReason::Destroy] {
			shutdown_wait_seconds
				.with_label_values(&[reason.as_metric_label()]);
			shutdown_timeout_total
				.with_label_values(&[reason.as_metric_label()]);
		}

		Self(Arc::new(ActorMetricsInner {
			actor_id,
			registry,
			create_state_ms,
			create_vars_ms,
			queue_depth,
			queue_messages_sent_total,
			queue_messages_received_total,
			active_connections,
			connections_total,
			lifecycle_inbox_depth,
			lifecycle_inbox_overload_total,
			dispatch_inbox_depth,
			dispatch_inbox_overload_total,
			lifecycle_event_inbox_depth,
			lifecycle_event_overload_total,
			user_tasks_active,
			user_task_duration_seconds,
			shutdown_wait_seconds,
			shutdown_timeout_total,
			state_mutation_total,
			state_mutation_overload_total,
			direct_subsystem_shutdown_warning_total,
		}))
	}

	pub(crate) fn actor_id(&self) -> &str {
		&self.0.actor_id
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

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		self.0.create_vars_ms.set(duration_ms(duration));
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

	pub(crate) fn set_lifecycle_inbox_depth(&self, depth: usize) {
		self.0
			.lifecycle_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_lifecycle_inbox_overload(&self, command: &str) {
		self.0
			.lifecycle_inbox_overload_total
			.with_label_values(&[command])
			.inc();
	}

	pub(crate) fn set_dispatch_inbox_depth(&self, depth: usize) {
		self.0
			.dispatch_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_dispatch_inbox_overload(&self, command: &str) {
		self.0
			.dispatch_inbox_overload_total
			.with_label_values(&[command])
			.inc();
	}

	pub(crate) fn set_lifecycle_event_inbox_depth(&self, depth: usize) {
		self.0
			.lifecycle_event_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_lifecycle_event_overload(&self, event: &str) {
		self.0
			.lifecycle_event_overload_total
			.with_label_values(&[event])
			.inc();
	}

	pub(crate) fn begin_user_task(&self, kind: UserTaskKind) {
		self.0
			.user_tasks_active
			.with_label_values(&[kind.as_metric_label()])
			.inc();
	}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		self.0
			.user_tasks_active
			.with_label_values(&[kind.as_metric_label()])
			.dec();
		self.0
			.user_task_duration_seconds
			.with_label_values(&[kind.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_shutdown_wait(&self, reason: StopReason, duration: Duration) {
		self.0
			.shutdown_wait_seconds
			.with_label_values(&[reason.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn inc_shutdown_timeout(&self, reason: StopReason) {
		self.0
			.shutdown_timeout_total
			.with_label_values(&[reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation(&self, reason: StateMutationReason) {
		self.0
			.state_mutation_total
			.with_label_values(&[reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation_overload(&self, reason: StateMutationReason) {
		self.0
			.state_mutation_overload_total
			.with_label_values(&[reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_direct_subsystem_shutdown_warning(
		&self,
		subsystem: &str,
		operation: &str,
	) {
		self.0
			.direct_subsystem_shutdown_warning_total
			.with_label_values(&[subsystem, operation])
			.inc();
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
