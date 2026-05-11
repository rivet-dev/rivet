use std::fmt;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use rivet_metrics::prometheus::{
	CounterVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
};

use crate::actor::task_types::{ShutdownKind, StateMutationReason, UserTaskKind};

const ACTOR_LABELS: &[&str] = &["actor_name"];
const USER_TASK_LABELS: &[&str] = &["actor_name", "kind"];
const SHUTDOWN_LABELS: &[&str] = &["actor_name", "reason"];
const STATE_MUTATION_LABELS: &[&str] = &["actor_name", "reason"];
const DIRECT_SHUTDOWN_LABELS: &[&str] = &["actor_name", "subsystem", "operation"];

#[cfg(feature = "sqlite-local")]
const SQLITE_COMMIT_PHASE_LABELS: &[&str] = &["actor_name", "phase"];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_COMMAND_LABELS: &[&str] = &["actor_name", "operation"];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_ERROR_LABELS: &[&str] = &["actor_name", "operation", "code"];

#[derive(Clone)]
pub(crate) struct ActorMetrics {
	inner: Arc<ActorMetricInner>,
}

#[derive(Debug)]
struct ActorMetricInner {
	labels: ActorMetricLabels,
}

#[derive(Debug)]
struct ActorMetricLabels {
	actor_name: String,
}

struct ActorMetricCollectors {
	actor_active_count: IntGaugeVec,
	create_state_duration_seconds: HistogramVec,
	create_vars_duration_seconds: HistogramVec,
	queue_messages_sent_total: IntCounterVec,
	queue_messages_received_total: IntCounterVec,
	connections_total: IntCounterVec,
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

impl ActorMetricCollectors {
	fn new() -> Self {
		let actor_active_count = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_active_count",
				"current active actors in this process",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_active_count gauge");
		let create_state_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_create_state_duration_seconds",
				"typed actor state creation time during startup in seconds",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_create_state_duration_seconds histogram");
		let create_vars_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_create_vars_duration_seconds",
				"typed actor vars creation time during startup in seconds",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_create_vars_duration_seconds histogram");
		let queue_messages_sent_total = IntCounterVec::new(
			Opts::new("rivetkit_actor_queue_messages_sent_total", "total queue messages sent"),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_messages_sent_total counter");
		let queue_messages_received_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_queue_messages_received_total",
				"total queue messages received",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_messages_received_total counter");
		let connections_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_connections_total",
				"total successfully established actor connections",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_connections_total counter");
		let user_task_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_user_task_duration_seconds",
				"actor user task execution time in seconds",
			),
			USER_TASK_LABELS,
		)
		.expect("create actor_user_task_duration_seconds histogram");
		let shutdown_wait_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_shutdown_wait_seconds",
				"actor shutdown wait time in seconds",
			),
			SHUTDOWN_LABELS,
		)
		.expect("create actor_shutdown_wait_seconds histogram");
		let shutdown_timeout_total = CounterVec::new(
			Opts::new(
				"rivetkit_actor_shutdown_timeout_total",
				"total actor shutdown timeout events",
			),
			SHUTDOWN_LABELS,
		)
		.expect("create actor_shutdown_timeout_total counter");
		let state_mutation_total = CounterVec::new(
			Opts::new("rivetkit_actor_state_mutation_total", "total actor state mutations"),
			STATE_MUTATION_LABELS,
		)
		.expect("create actor_state_mutation_total counter");
		let direct_subsystem_shutdown_warning_total = CounterVec::new(
			Opts::new(
				"rivetkit_actor_direct_subsystem_shutdown_warning_total",
				"total actor shutdown warnings emitted by direct subsystem drains",
			),
			DIRECT_SHUTDOWN_LABELS,
		)
		.expect("create actor_direct_subsystem_shutdown_warning_total counter");

		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_total",
				"total VFS page resolution attempts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_requested_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_requested_total",
				"total pages requested by VFS page resolution attempts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_requested_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_hits_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_cache_hits_total",
				"total pages resolved from the VFS page cache or write buffer",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_hits_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_misses_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_cache_misses_total",
				"total pages missing from the VFS page cache and write buffer",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_misses_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_get_pages_total",
				"total VFS to engine get_pages requests",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_get_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_pages_fetched_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_pages_fetched_total",
				"total pages requested from the engine by VFS get_pages calls",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_pages_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_prefetch_pages_total",
				"total pages requested speculatively by VFS prefetch",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_prefetch_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_bytes_fetched_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_bytes_fetched_total",
				"total bytes requested from the engine by VFS get_pages calls",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_bytes_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_bytes_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_prefetch_bytes_total",
				"total bytes requested speculatively by VFS prefetch",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_prefetch_bytes_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_sqlite_vfs_get_pages_duration_seconds",
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
				"rivetkit_actor_sqlite_vfs_commit_total",
				"total successful VFS commits",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_phase_duration_seconds_total = CounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_commit_phase_duration_seconds_total",
				"cumulative VFS commit phase duration in seconds",
			),
			SQLITE_COMMIT_PHASE_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_phase_duration_seconds_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_commit_duration_seconds_total = CounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_commit_duration_seconds_total",
				"cumulative VFS commit duration in seconds",
			),
			SQLITE_COMMIT_PHASE_LABELS,
		)
		.expect("create actor_sqlite_vfs_commit_duration_seconds_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_queue_overload_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_queue_overload_total",
				"total native SQLite worker SQL command queue overloads",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_queue_overload_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_sqlite_worker_command_duration_seconds",
				"native SQLite worker SQL command duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
			SQLITE_WORKER_COMMAND_LABELS,
		)
		.expect("create actor_sqlite_worker_command_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_command_error_total = CounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_command_error_total",
				"total native SQLite worker SQL command errors",
			),
			SQLITE_WORKER_ERROR_LABELS,
		)
		.expect("create actor_sqlite_worker_command_error_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_sqlite_worker_close_duration_seconds",
				"native SQLite worker close duration in seconds",
			)
			.buckets(sqlite_worker_duration_buckets()),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_close_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_close_timeout_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_close_timeout_total",
				"total native SQLite worker close timeouts",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_close_timeout_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_crash_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_crash_total",
				"total native SQLite worker crashes",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_crash_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_worker_unclean_close_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_unclean_close_total",
				"total native SQLite worker channel drops without clean close",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_unclean_close_total counter");

		register_metric(&rivet_metrics::REGISTRY, actor_active_count.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			create_state_duration_seconds.clone(),
		);
		register_metric(
			&rivet_metrics::REGISTRY,
			create_vars_duration_seconds.clone(),
		);
		register_metric(&rivet_metrics::REGISTRY, queue_messages_sent_total.clone());
		register_metric(&rivet_metrics::REGISTRY, queue_messages_received_total.clone());
		register_metric(&rivet_metrics::REGISTRY, connections_total.clone());
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
			actor_active_count,
			create_state_duration_seconds,
			create_vars_duration_seconds,
			queue_messages_sent_total,
			queue_messages_received_total,
			connections_total,
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
	pub(crate) fn new(actor_name: impl Into<String>) -> Self {
		let labels = ActorMetricLabels {
			actor_name: actor_name.into(),
		};
		METRICS
			.actor_active_count
			.with_label_values(&labels.as_label_values())
			.inc();
		Self {
			inner: Arc::new(ActorMetricInner { labels }),
		}
	}

	fn labels(&self) -> &ActorMetricLabels {
		&self.inner.labels
	}

	fn actor_labels(&self) -> [&str; 1] {
		self.labels().as_label_values()
	}

	pub(crate) fn observe_create_state(&self, duration: Duration) {
		METRICS
			.create_state_duration_seconds
			.with_label_values(&self.actor_labels())
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		METRICS
			.create_vars_duration_seconds
			.with_label_values(&self.actor_labels())
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn set_queue_depth(&self, _depth: u32) {}

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

	pub(crate) fn set_active_connections(&self, _count: usize) {}

	pub(crate) fn inc_connections_total(&self) {
		METRICS
			.connections_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	pub(crate) fn set_lifecycle_inbox_depth(&self, _depth: usize) {}

	pub(crate) fn set_dispatch_inbox_depth(&self, _depth: usize) {}

	pub(crate) fn set_lifecycle_event_inbox_depth(&self, _depth: usize) {}

	pub(crate) fn begin_user_task(&self, _kind: UserTaskKind) {}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		let labels = self.actor_labels();
		METRICS
			.user_task_duration_seconds
			.with_label_values(&[labels[0], kind.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_shutdown_wait(&self, reason: ShutdownKind, duration: Duration) {
		let labels = self.actor_labels();
		METRICS
			.shutdown_wait_seconds
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn inc_shutdown_timeout(&self, reason: ShutdownKind) {
		let labels = self.actor_labels();
		METRICS
			.shutdown_timeout_total
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation(&self, reason: StateMutationReason) {
		let labels = self.actor_labels();
		METRICS
			.state_mutation_total
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_direct_subsystem_shutdown_warning(&self, subsystem: &str, operation: &str) {
		let labels = self.actor_labels();
		METRICS
			.direct_subsystem_shutdown_warning_total
			.with_label_values(&[labels[0], subsystem, operation])
			.inc();
	}
}

impl Drop for ActorMetricInner {
	fn drop(&mut self) {
		METRICS
			.actor_active_count
			.with_label_values(&self.labels.as_label_values())
			.dec();
	}
}

impl Default for ActorMetrics {
	fn default() -> Self {
		Self::new("")
	}
}

impl fmt::Debug for ActorMetrics {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorMetrics").finish()
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
		let labels = self.actor_labels();
		for (phase, duration_ns) in [
			("request_build", request_build_ns),
			("serialize", serialize_ns),
			("transport", transport_ns),
			("state_update", state_update_ns),
		] {
			METRICS
				.sqlite_vfs_commit_phase_duration_seconds_total
				.with_label_values(&[labels[0], phase])
				.inc_by(ns_to_seconds(duration_ns));
		}
		METRICS
			.sqlite_vfs_commit_duration_seconds_total
			.with_label_values(&[labels[0], "total"])
			.inc_by(ns_to_seconds(total_ns));
	}

	fn set_worker_queue_depth(&self, _depth: u64) {}

	fn record_worker_queue_overload(&self) {
		METRICS
			.sqlite_worker_queue_overload_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn observe_worker_command_duration(&self, operation: &'static str, duration_ns: u64) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_worker_command_duration_seconds
			.with_label_values(&[labels[0], operation])
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_command_error(&self, operation: &'static str, code: &'static str) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_worker_command_error_total
			.with_label_values(&[labels[0], operation, code])
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

impl ActorMetricLabels {
	fn as_label_values(&self) -> [&str; 1] {
		[self.actor_name.as_str()]
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
