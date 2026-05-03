use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
#[cfg(feature = "sqlite-local")]
use prometheus::Histogram;
use prometheus::{
	CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, IntCounter, IntGauge, IntGaugeVec,
	Opts, Registry, TextEncoder,
};

use crate::actor::task_types::{ShutdownKind, StateMutationReason, UserTaskKind};

#[derive(Clone)]
pub(crate) struct ActorMetrics {
	actor_id: Arc<str>,
	inner: Arc<Option<ActorMetricsInner>>,
}

struct ActorMetricsInner {
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
	direct_subsystem_shutdown_warning_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_requested_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_cache_hits_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_cache_misses_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_get_pages_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_pages_fetched_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_prefetch_pages_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_bytes_fetched_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_prefetch_bytes_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_get_pages_duration_seconds: Histogram,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_phase_duration_seconds_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_duration_seconds_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_queue_depth: IntGauge,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_queue_overload_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_command_duration_seconds: HistogramVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_command_error_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_close_duration_seconds: Histogram,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_close_timeout_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_crash_total: IntCounter,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_unclean_close_total: IntCounter,
}

impl ActorMetrics {
	pub(crate) fn new(actor_id: impl Into<String>, actor_name: impl Into<String>) -> Self {
		let actor_id = actor_id.into();
		let actor_name = actor_name.into();
		let inner = match Self::try_new_inner(&actor_id, actor_name) {
			Ok(inner) => Some(inner),
			Err(error) => {
				tracing::warn!(
					actor_id,
					?error,
					"actor metrics disabled after initialization failure"
				);
				None
			}
		};

		Self {
			actor_id: Arc::from(actor_id),
			inner: Arc::new(inner),
		}
	}

	fn try_new_inner(actor_id: &str, actor_name: String) -> Result<ActorMetricsInner> {
		let registry = Registry::new_custom(
			None,
			Some(HashMap::from([
				("actor_id".to_owned(), actor_id.to_owned()),
				("actor_name".to_owned(), actor_name),
			])),
		)
		.context("create actor metrics registry")?;

		let create_state_ms = Gauge::with_opts(Opts::new(
			"create_state_ms",
			"time spent creating typed actor state during startup",
		))
		.context("create create_state_ms gauge")?;
		let create_vars_ms = Gauge::with_opts(Opts::new(
			"create_vars_ms",
			"time spent creating typed actor vars during startup",
		))
		.context("create create_vars_ms gauge")?;
		let queue_depth =
			IntGauge::with_opts(Opts::new("queue_depth", "current actor queue depth"))
				.context("create queue_depth gauge")?;
		let queue_messages_sent_total = IntCounter::with_opts(Opts::new(
			"queue_messages_sent_total",
			"total queue messages sent",
		))
		.context("create queue_messages_sent_total counter")?;
		let queue_messages_received_total = IntCounter::with_opts(Opts::new(
			"queue_messages_received_total",
			"total queue messages received",
		))
		.context("create queue_messages_received_total counter")?;
		let active_connections = IntGauge::with_opts(Opts::new(
			"active_connections",
			"current active actor connections",
		))
		.context("create active_connections gauge")?;
		let connections_total = IntCounter::with_opts(Opts::new(
			"connections_total",
			"total successfully established actor connections",
		))
		.context("create connections_total counter")?;
		let lifecycle_inbox_depth = IntGauge::with_opts(Opts::new(
			"lifecycle_inbox_depth",
			"current actor lifecycle command inbox depth",
		))
		.context("create lifecycle_inbox_depth gauge")?;
		let lifecycle_inbox_overload_total = CounterVec::new(
			Opts::new(
				"lifecycle_inbox_overload_total",
				"total actor lifecycle command inbox overloads",
			),
			&["command"],
		)
		.context("create lifecycle_inbox_overload_total counter")?;
		let dispatch_inbox_depth = IntGauge::with_opts(Opts::new(
			"dispatch_inbox_depth",
			"current actor dispatch command inbox depth",
		))
		.context("create dispatch_inbox_depth gauge")?;
		let dispatch_inbox_overload_total = CounterVec::new(
			Opts::new(
				"dispatch_inbox_overload_total",
				"total actor dispatch command inbox overloads",
			),
			&["command"],
		)
		.context("create dispatch_inbox_overload_total counter")?;
		let lifecycle_event_inbox_depth = IntGauge::with_opts(Opts::new(
			"lifecycle_event_inbox_depth",
			"current actor lifecycle event inbox depth",
		))
		.context("create lifecycle_event_inbox_depth gauge")?;
		let lifecycle_event_overload_total = CounterVec::new(
			Opts::new(
				"lifecycle_event_overload_total",
				"total actor lifecycle event inbox overloads",
			),
			&["event"],
		)
		.context("create lifecycle_event_overload_total counter")?;
		let user_tasks_active = IntGaugeVec::new(
			Opts::new("user_tasks_active", "current active actor user tasks"),
			&["kind"],
		)
		.context("create user_tasks_active gauge")?;
		let user_task_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"user_task_duration_seconds",
				"actor user task execution time in seconds",
			),
			&["kind"],
		)
		.context("create user_task_duration_seconds histogram")?;
		let shutdown_wait_seconds = HistogramVec::new(
			HistogramOpts::new(
				"shutdown_wait_seconds",
				"actor shutdown wait time in seconds",
			),
			&["reason"],
		)
		.context("create shutdown_wait_seconds histogram")?;
		let shutdown_timeout_total = CounterVec::new(
			Opts::new(
				"shutdown_timeout_total",
				"total actor shutdown timeout events",
			),
			&["reason"],
		)
		.context("create shutdown_timeout_total counter")?;
		let state_mutation_total = CounterVec::new(
			Opts::new("state_mutation_total", "total actor state mutations"),
			&["reason"],
		)
		.context("create state_mutation_total counter")?;
		let direct_subsystem_shutdown_warning_total = CounterVec::new(
			Opts::new(
				"direct_subsystem_shutdown_warning_total",
				"total actor shutdown warnings emitted by direct subsystem drains",
			),
			&["subsystem", "operation"],
		)
		.context("create direct_subsystem_shutdown_warning_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_resolve_pages_total",
			"total VFS page resolution attempts",
		))
		.context("create sqlite_vfs_resolve_pages_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_requested_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_resolve_pages_requested_total",
			"total pages requested by VFS page resolution attempts",
		))
		.context("create sqlite_vfs_resolve_pages_requested_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_hits_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_resolve_pages_cache_hits_total",
			"total pages resolved from the VFS page cache or write buffer",
		))
		.context("create sqlite_vfs_resolve_pages_cache_hits_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_misses_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_resolve_pages_cache_misses_total",
			"total pages missing from the VFS page cache and write buffer",
		))
		.context("create sqlite_vfs_resolve_pages_cache_misses_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_get_pages_total",
			"total VFS to engine get_pages requests",
		))
		.context("create sqlite_vfs_get_pages_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_pages_fetched_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_pages_fetched_total",
			"total pages requested from the engine by VFS get_pages calls",
		))
		.context("create sqlite_vfs_pages_fetched_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_pages_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_prefetch_pages_total",
			"total pages requested speculatively by VFS prefetch",
		))
		.context("create sqlite_vfs_prefetch_pages_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_bytes_fetched_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_bytes_fetched_total",
			"total bytes requested from the engine by VFS get_pages calls",
		))
		.context("create sqlite_vfs_bytes_fetched_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_bytes_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_prefetch_bytes_total",
			"total bytes requested speculatively by VFS prefetch",
		))
		.context("create sqlite_vfs_prefetch_bytes_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_duration_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"sqlite_vfs_get_pages_duration_seconds",
				"VFS get_pages request duration in seconds",
			)
			.buckets(vec![
				0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
			]),
		)
		.context("create sqlite_vfs_get_pages_duration_seconds histogram")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_total = IntCounter::with_opts(Opts::new(
			"sqlite_vfs_commit_total",
			"total successful VFS commits",
		))
		.context("create sqlite_vfs_commit_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_phase_duration_seconds_total = CounterVec::new(
			Opts::new(
				"sqlite_vfs_commit_phase_duration_seconds_total",
				"cumulative VFS commit phase duration in seconds",
			),
			&["phase"],
		)
		.context("create sqlite_vfs_commit_phase_duration_seconds_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_duration_seconds_total = CounterVec::new(
			Opts::new(
				"sqlite_vfs_commit_duration_seconds_total",
				"cumulative VFS commit duration in seconds",
			),
			&["phase"],
		)
		.context("create sqlite_vfs_commit_duration_seconds_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_queue_depth = IntGauge::with_opts(Opts::new(
			"sqlite_worker_queue_depth",
			"current native SQLite worker SQL command queue depth",
		))
		.context("create sqlite_worker_queue_depth gauge")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_queue_overload_total = IntCounter::with_opts(Opts::new(
			"sqlite_worker_queue_overload_total",
			"total native SQLite worker SQL command queue overloads",
		))
		.context("create sqlite_worker_queue_overload_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"sqlite_worker_command_duration_seconds",
				"native SQLite worker SQL command duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
			&["operation"],
		)
		.context("create sqlite_worker_command_duration_seconds histogram")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_error_total = CounterVec::new(
			Opts::new(
				"sqlite_worker_command_error_total",
				"total native SQLite worker SQL command errors",
			),
			&["operation", "code"],
		)
		.context("create sqlite_worker_command_error_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_duration_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"sqlite_worker_close_duration_seconds",
				"native SQLite worker close duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
		)
		.context("create sqlite_worker_close_duration_seconds histogram")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_timeout_total = IntCounter::with_opts(Opts::new(
			"sqlite_worker_close_timeout_total",
			"total native SQLite worker close timeouts",
		))
		.context("create sqlite_worker_close_timeout_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_crash_total = IntCounter::with_opts(Opts::new(
			"sqlite_worker_crash_total",
			"total native SQLite worker crashes",
		))
		.context("create sqlite_worker_crash_total counter")?;
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_unclean_close_total = IntCounter::with_opts(Opts::new(
			"sqlite_worker_unclean_close_total",
			"total native SQLite worker channel drops without clean close",
		))
		.context("create sqlite_worker_unclean_close_total counter")?;
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
		register_metric(&registry, direct_subsystem_shutdown_warning_total.clone());
		#[cfg(feature = "sqlite-local")]
		{
			register_metric(&registry, sqlite_vfs_resolve_pages_total.clone());
			register_metric(&registry, sqlite_vfs_resolve_pages_requested_total.clone());
			register_metric(&registry, sqlite_vfs_resolve_pages_cache_hits_total.clone());
			register_metric(
				&registry,
				sqlite_vfs_resolve_pages_cache_misses_total.clone(),
			);
			register_metric(&registry, sqlite_vfs_get_pages_total.clone());
			register_metric(&registry, sqlite_vfs_pages_fetched_total.clone());
			register_metric(&registry, sqlite_vfs_prefetch_pages_total.clone());
			register_metric(&registry, sqlite_vfs_bytes_fetched_total.clone());
			register_metric(&registry, sqlite_vfs_prefetch_bytes_total.clone());
			register_metric(&registry, sqlite_vfs_get_pages_duration_seconds.clone());
			register_metric(&registry, sqlite_vfs_commit_total.clone());
			register_metric(
				&registry,
				sqlite_vfs_commit_phase_duration_seconds_total.clone(),
			);
			register_metric(&registry, sqlite_vfs_commit_duration_seconds_total.clone());
			register_metric(&registry, sqlite_worker_queue_depth.clone());
			register_metric(&registry, sqlite_worker_queue_overload_total.clone());
			register_metric(&registry, sqlite_worker_command_duration_seconds.clone());
			register_metric(&registry, sqlite_worker_command_error_total.clone());
			register_metric(&registry, sqlite_worker_close_duration_seconds.clone());
			register_metric(&registry, sqlite_worker_close_timeout_total.clone());
			register_metric(&registry, sqlite_worker_crash_total.clone());
			register_metric(&registry, sqlite_worker_unclean_close_total.clone());
		}

		for kind in UserTaskKind::ALL {
			user_tasks_active
				.with_label_values(&[kind.as_metric_label()])
				.set(0);
			user_task_duration_seconds.with_label_values(&[kind.as_metric_label()]);
		}
		for reason in StateMutationReason::ALL {
			state_mutation_total.with_label_values(&[reason.as_metric_label()]);
		}
		for reason in [ShutdownKind::Sleep, ShutdownKind::Destroy] {
			shutdown_wait_seconds.with_label_values(&[reason.as_metric_label()]);
			shutdown_timeout_total.with_label_values(&[reason.as_metric_label()]);
		}
		#[cfg(feature = "sqlite-local")]
		{
			for phase in ["request_build", "serialize", "transport", "state_update"] {
				sqlite_vfs_commit_phase_duration_seconds_total.with_label_values(&[phase]);
			}
			sqlite_vfs_commit_duration_seconds_total.with_label_values(&["total"]);
			for operation in ["exec", "execute"] {
				sqlite_worker_command_duration_seconds.with_label_values(&[operation]);
				for code in ["sqlite", "closing", "dead", "overloaded", "close_timeout"] {
					sqlite_worker_command_error_total.with_label_values(&[operation, code]);
				}
			}
		}

		Ok(ActorMetricsInner {
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
			direct_subsystem_shutdown_warning_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_resolve_pages_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_resolve_pages_requested_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_resolve_pages_cache_hits_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_resolve_pages_cache_misses_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_get_pages_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_pages_fetched_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_prefetch_pages_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_bytes_fetched_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_prefetch_bytes_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_get_pages_duration_seconds,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_commit_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_commit_phase_duration_seconds_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_vfs_commit_duration_seconds_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_queue_depth,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_queue_overload_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_command_duration_seconds,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_command_error_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_close_duration_seconds,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_close_timeout_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_crash_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_worker_unclean_close_total,
		})
	}

	pub(crate) fn actor_id(&self) -> &str {
		&self.actor_id
	}

	pub(crate) fn render(&self) -> Result<String> {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return Ok(String::new());
		};
		let metric_families = inner.registry.gather();
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
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.create_state_ms.set(duration_ms(duration));
	}

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.create_vars_ms.set(duration_ms(duration));
	}

	pub(crate) fn set_queue_depth(&self, depth: u32) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.queue_depth.set(i64::from(depth));
	}

	pub(crate) fn add_queue_messages_sent(&self, count: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.queue_messages_sent_total.inc_by(count);
	}

	pub(crate) fn add_queue_messages_received(&self, count: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.queue_messages_received_total.inc_by(count);
	}

	pub(crate) fn set_active_connections(&self, count: usize) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.active_connections
			.set(count.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_connections_total(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.connections_total.inc();
	}

	pub(crate) fn set_lifecycle_inbox_depth(&self, depth: usize) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.lifecycle_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_lifecycle_inbox_overload(&self, command: &str) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.lifecycle_inbox_overload_total
			.with_label_values(&[command])
			.inc();
	}

	pub(crate) fn set_dispatch_inbox_depth(&self, depth: usize) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.dispatch_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_dispatch_inbox_overload(&self, command: &str) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.dispatch_inbox_overload_total
			.with_label_values(&[command])
			.inc();
	}

	pub(crate) fn set_lifecycle_event_inbox_depth(&self, depth: usize) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.lifecycle_event_inbox_depth
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_lifecycle_event_overload(&self, event: &str) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.lifecycle_event_overload_total
			.with_label_values(&[event])
			.inc();
	}

	pub(crate) fn begin_user_task(&self, kind: UserTaskKind) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.user_tasks_active
			.with_label_values(&[kind.as_metric_label()])
			.inc();
	}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.user_tasks_active
			.with_label_values(&[kind.as_metric_label()])
			.dec();
		inner
			.user_task_duration_seconds
			.with_label_values(&[kind.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_shutdown_wait(&self, reason: ShutdownKind, duration: Duration) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.shutdown_wait_seconds
			.with_label_values(&[reason.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn inc_shutdown_timeout(&self, reason: ShutdownKind) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.shutdown_timeout_total
			.with_label_values(&[reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation(&self, reason: StateMutationReason) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.state_mutation_total
			.with_label_values(&[reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_direct_subsystem_shutdown_warning(&self, subsystem: &str, operation: &str) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.direct_subsystem_shutdown_warning_total
			.with_label_values(&[subsystem, operation])
			.inc();
	}
}

#[cfg(feature = "sqlite-local")]
impl depot_client::vfs::SqliteVfsMetrics for ActorMetrics {
	fn record_resolve_pages(&self, requested_pages: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_vfs_resolve_pages_total.inc();
		inner
			.sqlite_vfs_resolve_pages_requested_total
			.inc_by(requested_pages);
	}

	fn record_resolve_cache_hits(&self, pages: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_vfs_resolve_pages_cache_hits_total
			.inc_by(pages);
	}

	fn record_resolve_cache_misses(&self, pages: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_vfs_resolve_pages_cache_misses_total
			.inc_by(pages);
	}

	fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_vfs_get_pages_total.inc();
		inner.sqlite_vfs_pages_fetched_total.inc_by(pages);
		inner.sqlite_vfs_prefetch_pages_total.inc_by(prefetch_pages);
		inner
			.sqlite_vfs_bytes_fetched_total
			.inc_by(pages.saturating_mul(page_size));
		inner
			.sqlite_vfs_prefetch_bytes_total
			.inc_by(prefetch_pages.saturating_mul(page_size));
	}

	fn observe_get_pages_duration(&self, duration_ns: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_vfs_get_pages_duration_seconds
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_commit(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_vfs_commit_total.inc();
	}

	fn observe_commit_phases(
		&self,
		request_build_ns: u64,
		serialize_ns: u64,
		transport_ns: u64,
		state_update_ns: u64,
		total_ns: u64,
	) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		for (phase, duration_ns) in [
			("request_build", request_build_ns),
			("serialize", serialize_ns),
			("transport", transport_ns),
			("state_update", state_update_ns),
		] {
			inner
				.sqlite_vfs_commit_phase_duration_seconds_total
				.with_label_values(&[phase])
				.inc_by(ns_to_seconds(duration_ns));
		}
		inner
			.sqlite_vfs_commit_duration_seconds_total
			.with_label_values(&["total"])
			.inc_by(ns_to_seconds(total_ns));
	}

	fn set_worker_queue_depth(&self, depth: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_worker_queue_depth.set(depth as i64);
	}

	fn record_worker_queue_overload(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_worker_queue_overload_total.inc();
	}

	fn observe_worker_command_duration(&self, operation: &'static str, duration_ns: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_worker_command_duration_seconds
			.with_label_values(&[operation])
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_command_error(&self, operation: &'static str, code: &'static str) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_worker_command_error_total
			.with_label_values(&[operation, code])
			.inc();
	}

	fn observe_worker_close_duration(&self, duration_ns: u64) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner
			.sqlite_worker_close_duration_seconds
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_close_timeout(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_worker_close_timeout_total.inc();
	}

	fn record_worker_crash(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_worker_crash_total.inc();
	}

	fn record_worker_unclean_close(&self) {
		let Some(inner) = self.inner.as_ref().as_ref() else {
			return;
		};
		inner.sqlite_worker_unclean_close_total.inc();
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

#[cfg(feature = "sqlite-local")]
fn ns_to_seconds(duration_ns: u64) -> f64 {
	Duration::from_nanos(duration_ns).as_secs_f64()
}

#[cfg(feature = "sqlite-local")]
fn sqlite_worker_duration_buckets() -> Vec<f64> {
	vec![
		0.000_1, 0.000_5, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
	]
}

fn register_metric<M>(registry: &Registry, metric: M)
where
	M: prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	if let Err(error) = registry.register(Box::new(metric)) {
		tracing::warn!(
			?error,
			"actor metric registration failed, using no-op collector"
		);
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/metrics.rs"]
mod tests;
