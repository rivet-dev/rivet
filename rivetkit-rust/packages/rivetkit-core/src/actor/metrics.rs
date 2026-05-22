use std::fmt;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use rivet_metrics::prometheus::{
	CounterVec, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
};
use scc::HashMap as SccHashMap;

use crate::actor::task_types::{ShutdownKind, StateMutationReason, UserTaskKind};
use crate::time::Instant;

const ACTOR_LABELS: &[&str] = &["actor_id_gen", "actor_key", "envoy_key"];
const USER_TASK_LABELS: &[&str] = &["actor_id_gen", "actor_key", "envoy_key", "kind"];
const SHUTDOWN_LABELS: &[&str] = &["actor_id_gen", "actor_key", "envoy_key", "reason"];
const STATE_MUTATION_LABELS: &[&str] = &["actor_id_gen", "actor_key", "envoy_key", "reason"];
const ACTOR_METRIC_RETENTION: Duration = Duration::from_secs(10 * 60);
const DIRECT_SHUTDOWN_LABELS: &[&str] = &[
	"actor_id_gen",
	"actor_key",
	"envoy_key",
	"subsystem",
	"operation",
];

#[cfg(feature = "sqlite-local")]
const SQLITE_COMMIT_PHASE_LABELS: &[&str] = &["actor_id_gen", "actor_key", "envoy_key", "phase"];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_COMMAND_LABELS: &[&str] =
	&["actor_id_gen", "actor_key", "envoy_key", "operation"];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_ERROR_LABELS: &[&str] =
	&["actor_id_gen", "actor_key", "envoy_key", "operation", "code"];

pub(crate) struct ActorMetrics {
	labels: Arc<ActorMetricLabels>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ActorMetricLabels {
	actor_id_gen: String,
	actor_key: String,
	envoy_key: String,
}

#[derive(Default)]
struct RetainedActorMetrics {
	active_refs: usize,
	expires_at: Option<Instant>,
	user_task_kinds: Vec<&'static str>,
	shutdown_reasons: Vec<&'static str>,
	state_mutation_reasons: Vec<&'static str>,
	direct_shutdown_labels: Vec<(String, String)>,
	#[cfg(feature = "sqlite-local")]
	sqlite_commit_phases: Vec<&'static str>,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_operations: Vec<&'static str>,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_error_labels: Vec<(&'static str, &'static str)>,
}

struct ActorMetricCollectors {
	actor_active: IntGaugeVec,
	create_state_ms: GaugeVec,
	create_vars_ms: GaugeVec,
	queue_depth: IntGaugeVec,
	queue_messages_sent_total: IntCounterVec,
	queue_messages_received_total: IntCounterVec,
	active_connections: IntGaugeVec,
	connections_total: IntCounterVec,
	lifecycle_inbox_depth: IntGaugeVec,
	dispatch_inbox_depth: IntGaugeVec,
	lifecycle_event_inbox_depth: IntGaugeVec,
	user_tasks_active: IntGaugeVec,
	user_task_duration_seconds: HistogramVec,
	shutdown_wait_seconds: HistogramVec,
	shutdown_timeout_total: CounterVec,
	state_mutation_total: CounterVec,
	direct_subsystem_shutdown_warning_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_requested_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_cache_hits_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_resolve_pages_cache_misses_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_get_pages_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_pages_fetched_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_prefetch_pages_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_bytes_fetched_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_prefetch_bytes_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_get_pages_duration_seconds: HistogramVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_phase_duration_seconds_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_commit_duration_seconds_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_queue_depth: IntGaugeVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_queue_overload_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_command_duration_seconds: HistogramVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_command_error_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_close_duration_seconds: HistogramVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_close_timeout_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_crash_total: IntCounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_unclean_close_total: IntCounterVec,
}

static METRICS: LazyLock<ActorMetricCollectors> = LazyLock::new(ActorMetricCollectors::new);
static RETAINED_ACTORS: LazyLock<SccHashMap<ActorMetricLabels, RetainedActorMetrics>> =
	LazyLock::new(SccHashMap::new);

impl ActorMetricCollectors {
	fn new() -> Self {
		let actor_active = IntGaugeVec::new(
			Opts::new(
				"actor_active",
				"whether an actor is currently active, retained briefly after shutdown",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_active gauge");
		let create_state_ms = GaugeVec::new(
			Opts::new(
				"actor_create_state_ms",
				"time spent creating typed actor state during startup",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_create_state_ms gauge");
		let create_vars_ms = GaugeVec::new(
			Opts::new(
				"actor_create_vars_ms",
				"time spent creating typed actor vars during startup",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_create_vars_ms gauge");
		let queue_depth = IntGaugeVec::new(
			Opts::new("actor_queue_depth", "current actor queue depth"),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_depth gauge");
		let queue_messages_sent_total = IntCounterVec::new(
			Opts::new("actor_queue_messages_sent_total", "total queue messages sent"),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_messages_sent_total counter");
		let queue_messages_received_total = IntCounterVec::new(
			Opts::new(
				"actor_queue_messages_received_total",
				"total queue messages received",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_messages_received_total counter");
		let active_connections = IntGaugeVec::new(
			Opts::new(
				"actor_active_connections",
				"current active actor connections",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_active_connections gauge");
		let connections_total = IntCounterVec::new(
			Opts::new(
				"actor_connections_total",
				"total successfully established actor connections",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_connections_total counter");
		let lifecycle_inbox_depth = IntGaugeVec::new(
			Opts::new(
				"actor_lifecycle_inbox_depth",
				"current actor lifecycle command inbox depth",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_lifecycle_inbox_depth gauge");
		let dispatch_inbox_depth = IntGaugeVec::new(
			Opts::new(
				"actor_dispatch_inbox_depth",
				"current actor dispatch command inbox depth",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_dispatch_inbox_depth gauge");
		let lifecycle_event_inbox_depth = IntGaugeVec::new(
			Opts::new(
				"actor_lifecycle_event_inbox_depth",
				"current actor lifecycle event inbox depth",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_lifecycle_event_inbox_depth gauge");
		let user_tasks_active = IntGaugeVec::new(
			Opts::new("actor_user_tasks_active", "current active actor user tasks"),
			USER_TASK_LABELS,
		)
		.expect("create actor_user_tasks_active gauge");
		let user_task_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"actor_user_task_duration_seconds",
				"actor user task execution time in seconds",
			),
			USER_TASK_LABELS,
		)
		.expect("create actor_user_task_duration_seconds histogram");
		let shutdown_wait_seconds = HistogramVec::new(
			HistogramOpts::new(
				"actor_shutdown_wait_seconds",
				"actor shutdown wait time in seconds",
			),
			SHUTDOWN_LABELS,
		)
		.expect("create actor_shutdown_wait_seconds histogram");
		let shutdown_timeout_total = CounterVec::new(
			Opts::new(
				"actor_shutdown_timeout_total",
				"total actor shutdown timeout events",
			),
			SHUTDOWN_LABELS,
		)
		.expect("create actor_shutdown_timeout_total counter");
		let state_mutation_total = CounterVec::new(
			Opts::new("actor_state_mutation_total", "total actor state mutations"),
			STATE_MUTATION_LABELS,
		)
		.expect("create actor_state_mutation_total counter");
		let direct_subsystem_shutdown_warning_total = CounterVec::new(
			Opts::new(
				"actor_direct_subsystem_shutdown_warning_total",
				"total actor shutdown warnings emitted by direct subsystem drains",
			),
			DIRECT_SHUTDOWN_LABELS,
		)
		.expect("create actor_direct_subsystem_shutdown_warning_total counter");

		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_resolve_pages_total",
				"total VFS page resolution attempts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_requested_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_resolve_pages_requested_total",
				"total pages requested by VFS page resolution attempts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_requested_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_hits_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_resolve_pages_cache_hits_total",
				"total pages resolved from the VFS page cache or write buffer",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_hits_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_misses_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_resolve_pages_cache_misses_total",
				"total pages missing from the VFS page cache and write buffer",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_misses_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_get_pages_total",
				"total VFS to engine get_pages requests",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_get_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_pages_fetched_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_pages_fetched_total",
				"total pages requested from the engine by VFS get_pages calls",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_pages_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_pages_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_prefetch_pages_total",
				"total pages requested speculatively by VFS prefetch",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_prefetch_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_bytes_fetched_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_bytes_fetched_total",
				"total bytes requested from the engine by VFS get_pages calls",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_bytes_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_bytes_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_prefetch_bytes_total",
				"total bytes requested speculatively by VFS prefetch",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_prefetch_bytes_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"actor_sqlite_vfs_get_pages_duration_seconds",
				"VFS get_pages request duration in seconds",
			)
			.buckets(vec![
				0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
			]),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_get_pages_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_commit_total",
				"total successful VFS commits",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_phase_duration_seconds_total = CounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_commit_phase_duration_seconds_total",
				"cumulative VFS commit phase duration in seconds",
			),
			SQLITE_COMMIT_PHASE_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_phase_duration_seconds_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_duration_seconds_total = CounterVec::new(
			Opts::new(
				"actor_sqlite_vfs_commit_duration_seconds_total",
				"cumulative VFS commit duration in seconds",
			),
			SQLITE_COMMIT_PHASE_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_duration_seconds_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_queue_depth = IntGaugeVec::new(
			Opts::new(
				"actor_sqlite_worker_queue_depth",
				"current native SQLite worker SQL command queue depth",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_queue_depth gauge");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_queue_overload_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_worker_queue_overload_total",
				"total native SQLite worker SQL command queue overloads",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_queue_overload_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"actor_sqlite_worker_command_duration_seconds",
				"native SQLite worker SQL command duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
			SQLITE_WORKER_COMMAND_LABELS,
		)
		.expect("create actor_sqlite_worker_command_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_error_total = CounterVec::new(
			Opts::new(
				"actor_sqlite_worker_command_error_total",
				"total native SQLite worker SQL command errors",
			),
			SQLITE_WORKER_ERROR_LABELS,
		)
		.expect("create actor_sqlite_worker_command_error_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"actor_sqlite_worker_close_duration_seconds",
				"native SQLite worker close duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_close_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_timeout_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_worker_close_timeout_total",
				"total native SQLite worker close timeouts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_close_timeout_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_crash_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_worker_crash_total",
				"total native SQLite worker crashes",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_crash_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_unclean_close_total = IntCounterVec::new(
			Opts::new(
				"actor_sqlite_worker_unclean_close_total",
				"total native SQLite worker channel drops without clean close",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_unclean_close_total counter");

		register_metric(&rivet_metrics::REGISTRY, actor_active.clone());
		register_metric(&rivet_metrics::REGISTRY, create_state_ms.clone());
		register_metric(&rivet_metrics::REGISTRY, create_vars_ms.clone());
		register_metric(&rivet_metrics::REGISTRY, queue_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, queue_messages_sent_total.clone());
		register_metric(&rivet_metrics::REGISTRY, queue_messages_received_total.clone());
		register_metric(&rivet_metrics::REGISTRY, active_connections.clone());
		register_metric(&rivet_metrics::REGISTRY, connections_total.clone());
		register_metric(&rivet_metrics::REGISTRY, lifecycle_inbox_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, dispatch_inbox_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, lifecycle_event_inbox_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, user_tasks_active.clone());
		register_metric(&rivet_metrics::REGISTRY, user_task_duration_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, shutdown_wait_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, shutdown_timeout_total.clone());
		register_metric(&rivet_metrics::REGISTRY, state_mutation_total.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			direct_subsystem_shutdown_warning_total.clone(),
		);
		#[cfg(feature = "sqlite-local")]
		{
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_resolve_pages_total.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_resolve_pages_requested_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_resolve_pages_cache_hits_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_resolve_pages_cache_misses_total.clone(),
			);
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_get_pages_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_pages_fetched_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_prefetch_pages_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_bytes_fetched_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_prefetch_bytes_total.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_get_pages_duration_seconds.clone(),
			);
			register_metric(&rivet_metrics::REGISTRY, sqlite_vfs_commit_total.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_commit_phase_duration_seconds_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_commit_duration_seconds_total.clone(),
			);
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_queue_depth.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_queue_overload_total.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_command_duration_seconds.clone(),
			);
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_command_error_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_close_duration_seconds.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_close_timeout_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_crash_total.clone());
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_unclean_close_total.clone());
		}

		Self {
			actor_active,
			create_state_ms,
			create_vars_ms,
			queue_depth,
			queue_messages_sent_total,
			queue_messages_received_total,
			active_connections,
			connections_total,
			lifecycle_inbox_depth,
			dispatch_inbox_depth,
			lifecycle_event_inbox_depth,
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
		}
	}
}

impl ActorMetrics {
	pub(crate) fn new(
		actor_id: impl Into<String>,
		generation: Option<u32>,
		actor_key: impl Into<String>,
		envoy_key: impl Into<String>,
	) -> Self {
		let actor_id = actor_id.into();
		let labels = Arc::new(ActorMetricLabels {
			actor_id_gen: generation
				.map(|generation| format!("{actor_id}:{generation}"))
				.unwrap_or_else(|| format!("{actor_id}:")),
			actor_key: actor_key.into(),
			envoy_key: envoy_key.into(),
		});
		retain_actor_metrics(&labels);
		Self { labels }
	}

	fn actor_labels(&self) -> [&str; 3] {
		[
			self.labels.actor_id_gen.as_str(),
			self.labels.actor_key.as_str(),
			self.labels.envoy_key.as_str(),
		]
	}

	pub(crate) fn observe_create_state(&self, duration: Duration) {
		METRICS
			.create_state_ms
			.with_label_values(&self.actor_labels())
			.set(duration_ms(duration));
	}

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		METRICS
			.create_vars_ms
			.with_label_values(&self.actor_labels())
			.set(duration_ms(duration));
	}

	pub(crate) fn set_queue_depth(&self, depth: u32) {
		METRICS
			.queue_depth
			.with_label_values(&self.actor_labels())
			.set(i64::from(depth));
	}

	pub(crate) fn add_queue_messages_sent(&self, count: u64) {
		METRICS
			.queue_messages_sent_total
			.with_label_values(&self.actor_labels())
			.inc_by(count);
	}

	pub(crate) fn add_queue_messages_received(&self, count: u64) {
		METRICS
			.queue_messages_received_total
			.with_label_values(&self.actor_labels())
			.inc_by(count);
	}

	pub(crate) fn set_active_connections(&self, count: usize) {
		METRICS
			.active_connections
			.with_label_values(&self.actor_labels())
			.set(count.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn inc_connections_total(&self) {
		METRICS
			.connections_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	pub(crate) fn set_lifecycle_inbox_depth(&self, depth: usize) {
		METRICS
			.lifecycle_inbox_depth
			.with_label_values(&self.actor_labels())
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn set_dispatch_inbox_depth(&self, depth: usize) {
		METRICS
			.dispatch_inbox_depth
			.with_label_values(&self.actor_labels())
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn set_lifecycle_event_inbox_depth(&self, depth: usize) {
		METRICS
			.lifecycle_event_inbox_depth
			.with_label_values(&self.actor_labels())
			.set(depth.try_into().unwrap_or(i64::MAX));
	}

	pub(crate) fn begin_user_task(&self, kind: UserTaskKind) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.user_task_kinds, kind.as_metric_label());
		});
		let labels = self.actor_labels();
		METRICS
			.user_tasks_active
			.with_label_values(&[labels[0], labels[1], labels[2], kind.as_metric_label()])
			.inc();
	}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.user_task_kinds, kind.as_metric_label());
		});
		let labels = self.actor_labels();
		let labels = [labels[0], labels[1], labels[2], kind.as_metric_label()];
		METRICS
			.user_tasks_active
			.with_label_values(&labels)
			.dec();
		METRICS
			.user_task_duration_seconds
			.with_label_values(&labels)
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_shutdown_wait(&self, reason: ShutdownKind, duration: Duration) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.shutdown_reasons, reason.as_metric_label());
		});
		let labels = self.actor_labels();
		METRICS
			.shutdown_wait_seconds
			.with_label_values(&[labels[0], labels[1], labels[2], reason.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn inc_shutdown_timeout(&self, reason: ShutdownKind) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.shutdown_reasons, reason.as_metric_label());
		});
		let labels = self.actor_labels();
		METRICS
			.shutdown_timeout_total
			.with_label_values(&[labels[0], labels[1], labels[2], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation(&self, reason: StateMutationReason) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(
				&mut retained.state_mutation_reasons,
				reason.as_metric_label(),
			);
		});
		let labels = self.actor_labels();
		METRICS
			.state_mutation_total
			.with_label_values(&[labels[0], labels[1], labels[2], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_direct_subsystem_shutdown_warning(&self, subsystem: &str, operation: &str) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(
				&mut retained.direct_shutdown_labels,
				(subsystem.to_owned(), operation.to_owned()),
			);
		});
		let labels = self.actor_labels();
		METRICS
			.direct_subsystem_shutdown_warning_total
			.with_label_values(&[labels[0], labels[1], labels[2], subsystem, operation])
			.inc();
	}
}

#[cfg(feature = "sqlite-local")]
impl depot_client::vfs::SqliteVfsMetrics for ActorMetrics {
	fn record_resolve_pages(&self, requested_pages: u64) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_vfs_resolve_pages_total
			.with_label_values(&labels)
			.inc();
		METRICS
			.sqlite_vfs_resolve_pages_requested_total
			.with_label_values(&labels)
			.inc_by(requested_pages);
	}

	fn record_resolve_cache_hits(&self, pages: u64) {
		METRICS
			.sqlite_vfs_resolve_pages_cache_hits_total
			.with_label_values(&self.actor_labels())
			.inc_by(pages);
	}

	fn record_resolve_cache_misses(&self, pages: u64) {
		METRICS
			.sqlite_vfs_resolve_pages_cache_misses_total
			.with_label_values(&self.actor_labels())
			.inc_by(pages);
	}

	fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_vfs_get_pages_total
			.with_label_values(&labels)
			.inc();
		METRICS
			.sqlite_vfs_pages_fetched_total
			.with_label_values(&labels)
			.inc_by(pages);
		METRICS
			.sqlite_vfs_prefetch_pages_total
			.with_label_values(&labels)
			.inc_by(prefetch_pages);
		METRICS
			.sqlite_vfs_bytes_fetched_total
			.with_label_values(&labels)
			.inc_by(pages.saturating_mul(page_size));
		METRICS
			.sqlite_vfs_prefetch_bytes_total
			.with_label_values(&labels)
			.inc_by(prefetch_pages.saturating_mul(page_size));
	}

	fn observe_get_pages_duration(&self, duration_ns: u64) {
		METRICS
			.sqlite_vfs_get_pages_duration_seconds
			.with_label_values(&self.actor_labels())
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_commit(&self) {
		METRICS
			.sqlite_vfs_commit_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn observe_commit_phases(
		&self,
		request_build_ns: u64,
		serialize_ns: u64,
		transport_ns: u64,
		state_update_ns: u64,
		total_ns: u64,
	) {
		record_retained_actor_metrics(&self.labels, |retained| {
			for phase in ["request_build", "serialize", "transport", "state_update", "total"] {
				push_unique(&mut retained.sqlite_commit_phases, phase);
			}
		});
		let labels = self.actor_labels();
		for (phase, duration_ns) in [
			("request_build", request_build_ns),
			("serialize", serialize_ns),
			("transport", transport_ns),
			("state_update", state_update_ns),
		] {
			METRICS
				.sqlite_vfs_commit_phase_duration_seconds_total
				.with_label_values(&[labels[0], labels[1], labels[2], phase])
				.inc_by(ns_to_seconds(duration_ns));
		}
		METRICS
			.sqlite_vfs_commit_duration_seconds_total
			.with_label_values(&[labels[0], labels[1], labels[2], "total"])
			.inc_by(ns_to_seconds(total_ns));
	}

	fn set_worker_queue_depth(&self, depth: u64) {
		METRICS
			.sqlite_worker_queue_depth
			.with_label_values(&self.actor_labels())
			.set(depth as i64);
	}

	fn record_worker_queue_overload(&self) {
		METRICS
			.sqlite_worker_queue_overload_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn observe_worker_command_duration(&self, operation: &'static str, duration_ns: u64) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.sqlite_worker_operations, operation);
		});
		let labels = self.actor_labels();
		METRICS
			.sqlite_worker_command_duration_seconds
			.with_label_values(&[labels[0], labels[1], labels[2], operation])
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_command_error(&self, operation: &'static str, code: &'static str) {
		record_retained_actor_metrics(&self.labels, |retained| {
			push_unique(&mut retained.sqlite_worker_operations, operation);
			push_unique(&mut retained.sqlite_worker_error_labels, (operation, code));
		});
		let labels = self.actor_labels();
		METRICS
			.sqlite_worker_command_error_total
			.with_label_values(&[labels[0], labels[1], labels[2], operation, code])
			.inc();
	}

	fn observe_worker_close_duration(&self, duration_ns: u64) {
		METRICS
			.sqlite_worker_close_duration_seconds
			.with_label_values(&self.actor_labels())
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_close_timeout(&self) {
		METRICS
			.sqlite_worker_close_timeout_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn record_worker_crash(&self) {
		METRICS
			.sqlite_worker_crash_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn record_worker_unclean_close(&self) {
		METRICS
			.sqlite_worker_unclean_close_total
			.with_label_values(&self.actor_labels())
			.inc();
	}
}

impl Clone for ActorMetrics {
	fn clone(&self) -> Self {
		retain_actor_metrics(&self.labels);
		Self {
			labels: self.labels.clone(),
		}
	}
}

impl Drop for ActorMetrics {
	fn drop(&mut self) {
		release_actor_metrics(&self.labels);
	}
}

impl Default for ActorMetrics {
	fn default() -> Self {
		Self::new("", None, "", "")
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

fn retain_actor_metrics(labels: &Arc<ActorMetricLabels>) {
	cleanup_expired_actor_metrics(Instant::now());
	let mut retained = RETAINED_ACTORS
		.entry_sync(labels.as_ref().clone())
		.or_default();
	retained.active_refs = retained.active_refs.saturating_add(1);
	retained.expires_at = None;
	METRICS
		.actor_active
		.with_label_values(&labels.as_label_values())
		.set(1);
}

fn release_actor_metrics(labels: &Arc<ActorMetricLabels>) {
	let now = Instant::now();
	let mut inactive = false;
	if let Some(mut retained) = RETAINED_ACTORS.get_sync(labels.as_ref()) {
		retained.active_refs = retained.active_refs.saturating_sub(1);
		if retained.active_refs == 0 {
			retained.expires_at = Some(now + ACTOR_METRIC_RETENTION);
			inactive = true;
		}
	}
	if inactive {
		METRICS
			.actor_active
			.with_label_values(&labels.as_label_values())
			.set(0);
	}
	cleanup_expired_actor_metrics(now);
}

fn record_retained_actor_metrics(
	labels: &Arc<ActorMetricLabels>,
	record: impl FnOnce(&mut RetainedActorMetrics),
) {
	let mut retained = RETAINED_ACTORS
		.entry_sync(labels.as_ref().clone())
		.or_default();
	record(&mut retained);
}

fn cleanup_expired_actor_metrics(now: Instant) {
	RETAINED_ACTORS.retain_sync(|labels, retained| {
		let expired = retained
			.expires_at
			.is_some_and(|expires_at| now >= expires_at);
		if expired {
			remove_retained_actor_metrics(labels, retained);
		}
		!expired
	});
}

fn remove_retained_actor_metrics(labels: &ActorMetricLabels, retained: &RetainedActorMetrics) {
	let actor_labels = labels.as_label_values();
	let metrics = &*METRICS;
	macro_rules! remove_actor_labels {
		($($metric:ident),+ $(,)?) => {
			$(
				ignore_missing_labels(metrics.$metric.remove_label_values(&actor_labels));
			)+
		};
	}

	remove_actor_labels!(
		actor_active,
		create_state_ms,
		create_vars_ms,
		queue_depth,
		queue_messages_sent_total,
		queue_messages_received_total,
		active_connections,
		connections_total,
		lifecycle_inbox_depth,
		dispatch_inbox_depth,
		lifecycle_event_inbox_depth,
	);

	for kind in &retained.user_task_kinds {
		let labels = [actor_labels[0], actor_labels[1], actor_labels[2], *kind];
		ignore_missing_labels(metrics.user_tasks_active.remove_label_values(&labels));
		ignore_missing_labels(
			metrics
				.user_task_duration_seconds
				.remove_label_values(&labels),
		);
	}
	for reason in &retained.shutdown_reasons {
		let labels = [actor_labels[0], actor_labels[1], actor_labels[2], *reason];
		ignore_missing_labels(metrics.shutdown_wait_seconds.remove_label_values(&labels));
		ignore_missing_labels(metrics.shutdown_timeout_total.remove_label_values(&labels));
	}
	for reason in &retained.state_mutation_reasons {
		let labels = [actor_labels[0], actor_labels[1], actor_labels[2], *reason];
		ignore_missing_labels(metrics.state_mutation_total.remove_label_values(&labels));
	}
	for (subsystem, operation) in &retained.direct_shutdown_labels {
		let labels = [
			actor_labels[0],
			actor_labels[1],
			actor_labels[2],
			subsystem.as_str(),
			operation.as_str(),
		];
		ignore_missing_labels(
			metrics
				.direct_subsystem_shutdown_warning_total
				.remove_label_values(&labels),
		);
	}

	#[cfg(feature = "sqlite-local")]
	{
		remove_actor_labels!(
			sqlite_vfs_resolve_pages_total,
			sqlite_vfs_resolve_pages_requested_total,
			sqlite_vfs_resolve_pages_cache_hits_total,
			sqlite_vfs_resolve_pages_cache_misses_total,
			sqlite_vfs_get_pages_total,
			sqlite_vfs_pages_fetched_total,
			sqlite_vfs_prefetch_pages_total,
			sqlite_vfs_bytes_fetched_total,
			sqlite_vfs_prefetch_bytes_total,
			sqlite_vfs_get_pages_duration_seconds,
			sqlite_vfs_commit_total,
			sqlite_worker_queue_depth,
			sqlite_worker_queue_overload_total,
			sqlite_worker_close_duration_seconds,
			sqlite_worker_close_timeout_total,
			sqlite_worker_crash_total,
			sqlite_worker_unclean_close_total,
		);

		for phase in &retained.sqlite_commit_phases {
			let labels = [actor_labels[0], actor_labels[1], actor_labels[2], *phase];
			ignore_missing_labels(
				metrics
					.sqlite_vfs_commit_phase_duration_seconds_total
					.remove_label_values(&labels),
			);
			ignore_missing_labels(
				metrics
					.sqlite_vfs_commit_duration_seconds_total
					.remove_label_values(&labels),
			);
		}
		for operation in &retained.sqlite_worker_operations {
			let labels = [actor_labels[0], actor_labels[1], actor_labels[2], *operation];
			ignore_missing_labels(
				metrics
					.sqlite_worker_command_duration_seconds
					.remove_label_values(&labels),
			);
		}
		for (operation, code) in &retained.sqlite_worker_error_labels {
			let labels = [
				actor_labels[0],
				actor_labels[1],
				actor_labels[2],
				*operation,
				*code,
			];
			ignore_missing_labels(
				metrics
					.sqlite_worker_command_error_total
					.remove_label_values(&labels),
			);
		}
	}
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
	if !values.contains(&value) {
		values.push(value);
	}
}

fn ignore_missing_labels(result: rivet_metrics::prometheus::Result<()>) {
	match result {
		Ok(()) => {}
		Err(error) if is_missing_labels_error(&error) => {}
		Err(error) => {
			tracing::debug!(?error, "failed to remove retained actor metric labels");
		}
	}
}

fn is_missing_labels_error(error: &rivet_metrics::prometheus::Error) -> bool {
	matches!(
		error,
		rivet_metrics::prometheus::Error::Msg(message)
			if message.starts_with("missing label values ")
				|| message.starts_with("missing labels ")
	)
}

impl ActorMetricLabels {
	fn as_label_values(&self) -> [&str; 3] {
		[
			self.actor_id_gen.as_str(),
			self.actor_key.as_str(),
			self.envoy_key.as_str(),
		]
	}
}

fn register_metric<M>(registry: &Registry, metric: M)
where
	M: rivet_metrics::prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	if let Err(error) = registry.register(Box::new(metric)) {
		tracing::warn!(
			?error,
			"actor metric registration failed, using existing collector"
		);
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/metrics.rs"]
mod tests;
