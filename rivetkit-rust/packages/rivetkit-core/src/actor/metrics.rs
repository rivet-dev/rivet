use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use parking_lot::Mutex;
use rivet_metrics::prometheus::{
	CounterVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
};

use crate::actor::task_types::{ShutdownKind, StateMutationReason, UserTaskKind};
use crate::time::Instant;

const ACTOR_LABELS: &[&str] = &["actor_name"];
const INBOX_LABELS: &[&str] = &["actor_name", "inbox"];
const USER_TASK_LABELS: &[&str] = &["actor_name", "kind"];
const WORK_LABELS: &[&str] = &["actor_name", "kind"];
const SHUTDOWN_LABELS: &[&str] = &["actor_name", "reason"];
const STATE_MUTATION_LABELS: &[&str] = &["actor_name", "reason"];
const DIRECT_SHUTDOWN_LABELS: &[&str] = &["actor_name", "subsystem", "operation"];
const STARTUP_PHASE_LABELS: &[&str] = &["actor_name", "phase", "is_new", "outcome"];
const STARTUP_KIND_UNKNOWN: u8 = 0;
const STARTUP_KIND_NEW: u8 = 1;
const STARTUP_KIND_EXISTING: u8 = 2;

pub(crate) mod startup_phase {
	#[derive(Clone, Copy, Debug)]
	#[repr(u8)]
	pub(crate) enum StartupPhase {
		Unknown = 0,
		LoadPersisted = 1,
		CoreInit = 2,
		RuntimePreamble = 3,
		PostReady = 4,
		Total = 5,
	}

	impl StartupPhase {
		pub(crate) fn as_label(self) -> &'static str {
			match self {
				StartupPhase::Unknown => "unknown",
				StartupPhase::LoadPersisted => "load_persisted",
				StartupPhase::CoreInit => "core_init",
				StartupPhase::RuntimePreamble => "runtime_preamble",
				StartupPhase::PostReady => "post_ready",
				StartupPhase::Total => "total",
			}
		}

		#[cfg(feature = "sqlite-local")]
		pub(super) fn from_id(id: u8) -> Self {
			match id {
				0 => StartupPhase::Unknown,
				1 => StartupPhase::LoadPersisted,
				2 => StartupPhase::CoreInit,
				3 => StartupPhase::RuntimePreamble,
				4 => StartupPhase::PostReady,
				5 => StartupPhase::Total,
				_ => StartupPhase::Unknown,
			}
		}
	}
}

#[cfg(feature = "sqlite-local")]
mod actor_lifecycle_bucket {
	use std::time::Duration;

	pub(super) const READY_0_1S: &str = "ready_0_1s";
	pub(super) const READY_1_5S: &str = "ready_1_5s";
	pub(super) const READY_5_30S: &str = "ready_5_30s";
	pub(super) const READY_30S_PLUS: &str = "ready_30s_plus";

	pub(super) fn ready_for_age(age: Duration) -> &'static str {
		if age < Duration::from_secs(1) {
			READY_0_1S
		} else if age < Duration::from_secs(5) {
			READY_1_5S
		} else if age < Duration::from_secs(30) {
			READY_5_30S
		} else {
			READY_30S_PLUS
		}
	}
}

#[cfg(feature = "sqlite-local")]
const SQLITE_COMMIT_PHASE_LABELS: &[&str] = &["actor_name", "phase"];
#[cfg(feature = "sqlite-local")]
const SQLITE_OPEN_PHASE_LABELS: &[&str] = &["actor_name", "phase", "is_new", "outcome"];
#[cfg(feature = "sqlite-local")]
const SQLITE_STARTUP_PRELOAD_PAGE_LABELS: &[&str] = &["actor_name", "is_new", "kind"];
#[cfg(feature = "sqlite-local")]
const SQLITE_VFS_LIFECYCLE_BUCKET_LABELS: &[&str] =
	&["actor_name", "actor_lifecycle_bucket", "is_new"];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_COMMAND_LABELS: &[&str] = &[
	"actor_name",
	"operation",
	"actor_lifecycle_bucket",
	"is_tx",
	"stmt_kind",
];
#[cfg(feature = "sqlite-local")]
const SQLITE_WORKER_ERROR_LABELS: &[&str] = &["actor_name", "operation", "code"];

#[derive(Clone)]
pub(crate) struct ActorMetrics {
	inner: Arc<ActorMetricInner>,
}

/// Records the total startup metric when the startup attempt leaves scope.
///
/// Startup phases record their own durations at the phase boundary. This guard
/// owns the total duration so early returns cannot forget the total error metric.
pub(crate) struct StartupTimer {
	metrics: ActorMetrics,
	started_at: Instant,
	is_new: Option<bool>,
	finished: bool,
}

#[derive(Debug)]
struct ActorMetricInner {
	labels: ActorMetricLabels,
	state: Mutex<ActorMetricState>,
	active: AtomicBool,
	startup_is_new: AtomicU8,
	startup_complete: AtomicBool,
	current_startup_phase: AtomicU8,
	ready_at: Mutex<Option<Instant>>,
}

#[derive(Debug)]
struct ActorMetricLabels {
	actor_name: String,
}

#[derive(Debug, Default)]
struct ActorMetricState {
	queue_depth: i64,
	active_connections: i64,
	lifecycle_inbox_depth: i64,
	dispatch_inbox_depth: i64,
	lifecycle_event_inbox_depth: i64,
	user_tasks_active: BTreeMap<&'static str, i64>,
	http_requests_active: i64,
	keep_awake_active: i64,
	internal_keep_awake_active: i64,
	shutdown_tasks_active: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_worker_queue_depth: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_workers_active: i64,
}

struct ActorMetricCollectors {
	actor_active_count: IntGaugeVec,
	actor_started_total: IntCounterVec,
	actor_stopped_total: IntCounterVec,
	startup_phase_duration_seconds: HistogramVec,
	create_state_duration_seconds: HistogramVec,
	create_vars_duration_seconds: HistogramVec,
	queue_depth: IntGaugeVec,
	queue_messages_sent_total: IntCounterVec,
	queue_messages_received_total: IntCounterVec,
	active_connections: IntGaugeVec,
	connections_total: IntCounterVec,
	inbox_depth: IntGaugeVec,
	user_tasks_active: IntGaugeVec,
	user_task_duration_seconds: HistogramVec,
	http_requests_active: IntGaugeVec,
	keep_awake_active: IntGaugeVec,
	shutdown_tasks_active: IntGaugeVec,
	shutdown_wait_seconds: HistogramVec,
	shutdown_timeout_total: CounterVec,
	state_mutation_total: CounterVec,
	direct_subsystem_shutdown_warning_total: CounterVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_open_phase_duration_seconds: HistogramVec,
	#[cfg(feature = "sqlite-local")]
	sqlite_startup_preload_pages_total: IntCounterVec,
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
	sqlite_workers_active: IntGaugeVec,
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
		let actor_started_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_started_total",
				"total actors started in this process",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_started_total counter");
		let actor_stopped_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_stopped_total",
				"total actors stopped in this process",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_stopped_total counter");
		let startup_phase_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_startup_phase_duration_seconds",
				"actor startup phase duration in seconds",
			)
			.buckets(startup_duration_buckets()),
			STARTUP_PHASE_LABELS,
		)
		.expect("create actor_startup_phase_duration_seconds histogram");
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
		let queue_depth = IntGaugeVec::new(
			Opts::new("rivetkit_actor_queue_depth", "current actor queue depth"),
			ACTOR_LABELS,
		)
		.expect("create actor_queue_depth gauge");
		let queue_messages_sent_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_queue_messages_sent_total",
				"total queue messages sent",
			),
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
		let active_connections = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_connections_active",
				"current active actor connections",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_connections_active gauge");
		let connections_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_connections_total",
				"total successfully established actor connections",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_connections_total counter");
		let inbox_depth = IntGaugeVec::new(
			Opts::new("rivetkit_actor_inbox_depth", "current actor inbox depth"),
			INBOX_LABELS,
		)
		.expect("create actor_inbox_depth gauge");
		let user_tasks_active = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_user_tasks_active",
				"current active actor user tasks",
			),
			USER_TASK_LABELS,
		)
		.expect("create actor_user_tasks_active gauge");
		let user_task_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_user_task_duration_seconds",
				"actor user task execution time in seconds",
			),
			USER_TASK_LABELS,
		)
		.expect("create actor_user_task_duration_seconds histogram");
		let http_requests_active = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_http_requests_active",
				"current actor-scoped HTTP requests",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_http_requests_active gauge");
		let keep_awake_active = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_keep_awake_active",
				"current actor keep-awake work",
			),
			WORK_LABELS,
		)
		.expect("create actor_keep_awake_active gauge");
		let shutdown_tasks_active = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_shutdown_tasks_active",
				"current actor work draining during shutdown",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_shutdown_tasks_active gauge");
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
			Opts::new(
				"rivetkit_actor_state_mutation_total",
				"total actor state mutations",
			),
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
		let sqlite_open_phase_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_actor_sqlite_open_phase_duration_seconds",
				"native SQLite open phase duration in seconds",
			)
			.buckets(startup_duration_buckets()),
			SQLITE_OPEN_PHASE_LABELS,
		)
		.expect("create actor_sqlite_open_phase_duration_seconds histogram");
		#[cfg(feature = "sqlite-local")]
		let sqlite_startup_preload_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_startup_preload_pages_total",
				"total SQLite startup preload pages requested or loaded",
			),
			SQLITE_STARTUP_PRELOAD_PAGE_LABELS,
		)
		.expect("create actor_sqlite_startup_preload_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_total",
				"total VFS page resolution attempts",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_requested_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_requested_total",
				"total pages requested by VFS page resolution attempts",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_requested_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_hits_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_cache_hits_total",
				"total pages resolved from the VFS page cache or write buffer",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_hits_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_resolve_pages_cache_misses_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_resolve_pages_cache_misses_total",
				"total pages missing from the VFS page cache and write buffer",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_resolve_pages_cache_misses_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_get_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_get_pages_total",
				"total VFS to engine get_pages requests",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_get_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_pages_fetched_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_pages_fetched_total",
				"total pages requested from the engine by VFS get_pages calls",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_pages_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_pages_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_prefetch_pages_total",
				"total pages requested speculatively by VFS prefetch",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_prefetch_pages_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_bytes_fetched_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_bytes_fetched_total",
				"total bytes requested from the engine by VFS get_pages calls",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		)
		.expect("create actor_sqlite_vfs_bytes_fetched_total counter");
		#[cfg(feature = "sqlite-local")]
		let sqlite_vfs_prefetch_bytes_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_vfs_prefetch_bytes_total",
				"total bytes requested speculatively by VFS prefetch",
			),
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
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
			SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
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
		let sqlite_worker_queue_depth = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_worker_queue_depth",
				"current native SQLite worker SQL command queue depth",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_worker_queue_depth gauge");
		#[cfg(feature = "sqlite-local")]
		let sqlite_workers_active = IntGaugeVec::new(
			Opts::new(
				"rivetkit_actor_sqlite_workers_active",
				"current active native SQLite workers",
			),
			ACTOR_LABELS,
		)
		.expect("create actor_sqlite_workers_active gauge");
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
		register_metric(&rivet_metrics::REGISTRY, actor_started_total.clone());
		register_metric(&rivet_metrics::REGISTRY, actor_stopped_total.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			create_state_duration_seconds.clone(),
		);
		register_metric(
			&rivet_metrics::REGISTRY,
			startup_phase_duration_seconds.clone(),
		);
		register_metric(
			&rivet_metrics::REGISTRY,
			create_vars_duration_seconds.clone(),
		);
		register_metric(&rivet_metrics::REGISTRY, queue_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, queue_messages_sent_total.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			queue_messages_received_total.clone(),
		);
		register_metric(&rivet_metrics::REGISTRY, active_connections.clone());
		register_metric(&rivet_metrics::REGISTRY, connections_total.clone());
		register_metric(&rivet_metrics::REGISTRY, inbox_depth.clone());
		register_metric(&rivet_metrics::REGISTRY, user_tasks_active.clone());
		register_metric(&rivet_metrics::REGISTRY, user_task_duration_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, http_requests_active.clone());
		register_metric(&rivet_metrics::REGISTRY, keep_awake_active.clone());
		register_metric(&rivet_metrics::REGISTRY, shutdown_tasks_active.clone());
		register_metric(&rivet_metrics::REGISTRY, shutdown_wait_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, shutdown_timeout_total.clone());
		register_metric(&rivet_metrics::REGISTRY, state_mutation_total.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			direct_subsystem_shutdown_warning_total.clone(),
		);
		#[cfg(feature = "sqlite-local")]
		{
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_open_phase_duration_seconds.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_startup_preload_pages_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_resolve_pages_total.clone(),
			);
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
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_pages_fetched_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_prefetch_pages_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_bytes_fetched_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_vfs_prefetch_bytes_total.clone(),
			);
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
			register_metric(&rivet_metrics::REGISTRY, sqlite_workers_active.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_queue_overload_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_command_duration_seconds.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_command_error_total.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_close_duration_seconds.clone(),
			);
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_close_timeout_total.clone(),
			);
			register_metric(&rivet_metrics::REGISTRY, sqlite_worker_crash_total.clone());
			register_metric(
				&rivet_metrics::REGISTRY,
				sqlite_worker_unclean_close_total.clone(),
			);
		}

		Self {
			actor_active_count,
			actor_started_total,
			actor_stopped_total,
			startup_phase_duration_seconds,
			create_state_duration_seconds,
			create_vars_duration_seconds,
			queue_depth,
			queue_messages_sent_total,
			queue_messages_received_total,
			active_connections,
			connections_total,
			inbox_depth,
			user_tasks_active,
			user_task_duration_seconds,
			http_requests_active,
			keep_awake_active,
			shutdown_tasks_active,
			shutdown_wait_seconds,
			shutdown_timeout_total,
			state_mutation_total,
			direct_subsystem_shutdown_warning_total,
			#[cfg(feature = "sqlite-local")]
			sqlite_open_phase_duration_seconds,
			#[cfg(feature = "sqlite-local")]
			sqlite_startup_preload_pages_total,
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
			sqlite_workers_active,
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
		let metrics = &*METRICS;
		metrics
			.actor_active_count
			.with_label_values(&labels.as_label_values())
			.inc();
		metrics
			.actor_started_total
			.with_label_values(&labels.as_label_values())
			.inc();
		Self {
			inner: Arc::new(ActorMetricInner {
				labels,
				state: Mutex::new(ActorMetricState::default()),
				active: AtomicBool::new(true),
				startup_is_new: AtomicU8::new(STARTUP_KIND_UNKNOWN),
				startup_complete: AtomicBool::new(false),
				current_startup_phase: AtomicU8::new(startup_phase::StartupPhase::Unknown as u8),
				ready_at: Mutex::new(None),
			}),
		}
	}

	fn labels(&self) -> &ActorMetricLabels {
		&self.inner.labels
	}

	fn actor_labels(&self) -> [&str; 1] {
		self.labels().as_label_values()
	}

	#[cfg(feature = "sqlite-local")]
	fn startup_is_new_label(&self) -> &'static str {
		match self.inner.startup_is_new.load(Ordering::Acquire) {
			STARTUP_KIND_NEW => "true",
			STARTUP_KIND_EXISTING => "false",
			STARTUP_KIND_UNKNOWN => "unknown",
			_ => "unknown",
		}
	}

	#[cfg(feature = "sqlite-local")]
	fn actor_lifecycle_bucket_label(&self) -> &'static str {
		if !self.inner.startup_complete.load(Ordering::Acquire) {
			return startup_phase::StartupPhase::from_id(
				self.inner.current_startup_phase.load(Ordering::Acquire),
			)
			.as_label();
		}
		let ready_age = self
			.inner
			.ready_at
			.lock()
			.as_ref()
			.map(|ready_at| ready_at.elapsed())
			.unwrap_or(Duration::ZERO);
		actor_lifecycle_bucket::ready_for_age(ready_age)
	}

	#[cfg(feature = "sqlite-local")]
	fn sqlite_vfs_labels(&self) -> [&str; 3] {
		let labels = self.actor_labels();
		[
			labels[0],
			self.actor_lifecycle_bucket_label(),
			self.startup_is_new_label(),
		]
	}

	pub(crate) fn begin_startup(&self) {
		self.inner
			.startup_is_new
			.store(STARTUP_KIND_UNKNOWN, Ordering::Release);
		self.inner.current_startup_phase.store(
			startup_phase::StartupPhase::LoadPersisted as u8,
			Ordering::Release,
		);
		*self.inner.ready_at.lock() = None;
		self.inner.startup_complete.store(false, Ordering::Release);
	}

	/// Begins a timed startup attempt and records `total,error` unless it succeeds.
	pub(crate) fn begin_startup_timer(&self) -> StartupTimer {
		self.begin_startup();
		StartupTimer {
			metrics: self.clone(),
			started_at: Instant::now(),
			is_new: None,
			finished: false,
		}
	}

	pub(crate) fn set_startup_phase(&self, phase: startup_phase::StartupPhase) {
		self.inner
			.current_startup_phase
			.store(phase as u8, Ordering::Release);
	}

	pub(crate) fn set_startup_is_new(&self, is_new: bool) {
		let kind = if is_new {
			STARTUP_KIND_NEW
		} else {
			STARTUP_KIND_EXISTING
		};
		self.inner.startup_is_new.store(kind, Ordering::Release);
	}

	pub(crate) fn finish_startup(&self) {
		*self.inner.ready_at.lock() = Some(crate::time::Instant::now());
		self.inner.startup_complete.store(true, Ordering::Release);
	}

	pub(crate) fn observe_startup_phase(
		&self,
		phase: startup_phase::StartupPhase,
		is_new: Option<bool>,
		outcome: &'static str,
		duration: Duration,
	) {
		let labels = self.actor_labels();
		METRICS
			.startup_phase_duration_seconds
			.with_label_values(&[
				labels[0],
				phase.as_label(),
				optional_is_new_label(is_new),
				outcome,
			])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_startup_phase_result<T, E>(
		&self,
		phase: startup_phase::StartupPhase,
		is_new: Option<bool>,
		started_at: Instant,
		result: std::result::Result<T, E>,
	) -> std::result::Result<T, E> {
		let outcome = if result.is_ok() { "success" } else { "error" };
		self.observe_startup_phase(phase, is_new, outcome, started_at.elapsed());
		result
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

	pub(crate) fn set_queue_depth(&self, depth: u32) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.queue_depth,
			i64::from(depth),
			&METRICS.queue_depth,
			&labels,
		);
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
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.active_connections,
			usize_to_i64(count),
			&METRICS.active_connections,
			&labels,
		);
	}

	pub(crate) fn inc_connections_total(&self) {
		METRICS
			.connections_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	pub(crate) fn set_lifecycle_inbox_depth(&self, depth: usize) {
		self.set_inbox_depth("lifecycle", depth);
	}

	pub(crate) fn set_dispatch_inbox_depth(&self, depth: usize) {
		self.set_inbox_depth("dispatch", depth);
	}

	pub(crate) fn set_lifecycle_event_inbox_depth(&self, depth: usize) {
		self.set_inbox_depth("lifecycle_event", depth);
	}

	fn set_inbox_depth(&self, inbox: &'static str, depth: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		let current = match inbox {
			"lifecycle" => &mut state.lifecycle_inbox_depth,
			"dispatch" => &mut state.dispatch_inbox_depth,
			"lifecycle_event" => &mut state.lifecycle_event_inbox_depth,
			_ => unreachable!("unknown inbox metric label"),
		};
		set_aggregated_gauge(
			current,
			usize_to_i64(depth),
			&METRICS.inbox_depth,
			&[labels[0], inbox],
		);
	}

	pub(crate) fn begin_user_task(&self, kind: UserTaskKind) {
		let labels = self.actor_labels();
		let kind = kind.as_metric_label();
		let mut state = self.inner.state.lock();
		let current = state.user_tasks_active.entry(kind).or_default();
		let next = (*current).saturating_add(1);
		set_aggregated_gauge(
			current,
			next,
			&METRICS.user_tasks_active,
			&[labels[0], kind],
		);
	}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		let labels = self.actor_labels();
		let kind = kind.as_metric_label();
		{
			let mut state = self.inner.state.lock();
			let current = state.user_tasks_active.entry(kind).or_default();
			let next = (*current).saturating_sub(1);
			set_aggregated_gauge(
				current,
				next,
				&METRICS.user_tasks_active,
				&[labels[0], kind],
			);
		}
		METRICS
			.user_task_duration_seconds
			.with_label_values(&[labels[0], kind])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn set_http_requests_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.http_requests_active,
			usize_to_i64(count),
			&METRICS.http_requests_active,
			&labels,
		);
	}

	pub(crate) fn set_keep_awake_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.keep_awake_active,
			usize_to_i64(count),
			&METRICS.keep_awake_active,
			&[labels[0], "keep_awake"],
		);
	}

	pub(crate) fn set_internal_keep_awake_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.internal_keep_awake_active,
			usize_to_i64(count),
			&METRICS.keep_awake_active,
			&[labels[0], "internal_keep_awake"],
		);
	}

	pub(crate) fn set_shutdown_tasks_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.shutdown_tasks_active,
			usize_to_i64(count),
			&METRICS.shutdown_tasks_active,
			&labels,
		);
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

	pub(crate) fn record_actor_stopped(&self) {
		self.inner.record_actor_stopped();
	}
}

impl Drop for ActorMetricInner {
	fn drop(&mut self) {
		self.record_actor_stopped();
	}
}

impl ActorMetricInner {
	fn record_actor_stopped(&self) {
		if !self.active.swap(false, Ordering::AcqRel) {
			return;
		}

		self.clear_aggregated_gauges();
		let metrics = &*METRICS;
		metrics
			.actor_active_count
			.with_label_values(&self.labels.as_label_values())
			.dec();
		metrics
			.actor_stopped_total
			.with_label_values(&self.labels.as_label_values())
			.inc();
	}

	fn clear_aggregated_gauges(&self) {
		let labels = self.labels.as_label_values();
		let mut state = self.state.lock();
		set_aggregated_gauge(&mut state.queue_depth, 0, &METRICS.queue_depth, &labels);
		set_aggregated_gauge(
			&mut state.active_connections,
			0,
			&METRICS.active_connections,
			&labels,
		);
		set_aggregated_gauge(
			&mut state.lifecycle_inbox_depth,
			0,
			&METRICS.inbox_depth,
			&[labels[0], "lifecycle"],
		);
		set_aggregated_gauge(
			&mut state.dispatch_inbox_depth,
			0,
			&METRICS.inbox_depth,
			&[labels[0], "dispatch"],
		);
		set_aggregated_gauge(
			&mut state.lifecycle_event_inbox_depth,
			0,
			&METRICS.inbox_depth,
			&[labels[0], "lifecycle_event"],
		);
		for (kind, current) in state.user_tasks_active.iter_mut() {
			set_aggregated_gauge(current, 0, &METRICS.user_tasks_active, &[labels[0], *kind]);
		}
		set_aggregated_gauge(
			&mut state.http_requests_active,
			0,
			&METRICS.http_requests_active,
			&labels,
		);
		set_aggregated_gauge(
			&mut state.keep_awake_active,
			0,
			&METRICS.keep_awake_active,
			&[labels[0], "keep_awake"],
		);
		set_aggregated_gauge(
			&mut state.internal_keep_awake_active,
			0,
			&METRICS.keep_awake_active,
			&[labels[0], "internal_keep_awake"],
		);
		set_aggregated_gauge(
			&mut state.shutdown_tasks_active,
			0,
			&METRICS.shutdown_tasks_active,
			&labels,
		);
		#[cfg(feature = "sqlite-local")]
		{
			set_aggregated_gauge(
				&mut state.sqlite_worker_queue_depth,
				0,
				&METRICS.sqlite_worker_queue_depth,
				&labels,
			);
			set_aggregated_gauge(
				&mut state.sqlite_workers_active,
				0,
				&METRICS.sqlite_workers_active,
				&labels,
			);
		}
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
		let labels = self.sqlite_vfs_labels();
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
			.with_label_values(&self.sqlite_vfs_labels())
			.inc_by(pages);
	}

	fn record_resolve_cache_misses(&self, pages: u64) {
		METRICS
			.sqlite_vfs_resolve_pages_cache_misses_total
			.with_label_values(&self.sqlite_vfs_labels())
			.inc_by(pages);
	}

	fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64) {
		let labels = self.sqlite_vfs_labels();
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
			.with_label_values(&self.sqlite_vfs_labels())
			.observe(ns_to_seconds(duration_ns));
	}

	fn observe_open_phase(
		&self,
		phase: depot_client::vfs::SqliteOpenPhase,
		outcome: &'static str,
		duration_ns: u64,
	) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_open_phase_duration_seconds
			.with_label_values(&[
				labels[0],
				phase.as_label(),
				self.startup_is_new_label(),
				outcome,
			])
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_startup_preload_pages(&self, kind: &'static str, pages: u64) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_startup_preload_pages_total
			.with_label_values(&[labels[0], self.startup_is_new_label(), kind])
			.inc_by(pages);
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

	fn set_worker_queue_depth(&self, depth: u64) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.sqlite_worker_queue_depth,
			u64_to_i64(depth),
			&METRICS.sqlite_worker_queue_depth,
			&labels,
		);
	}

	fn set_worker_active(&self, active: bool) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.sqlite_workers_active,
			if active { 1 } else { 0 },
			&METRICS.sqlite_workers_active,
			&labels,
		);
	}

	fn record_worker_queue_overload(&self) {
		METRICS
			.sqlite_worker_queue_overload_total
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn observe_worker_command_duration(
		&self,
		operation: &'static str,
		in_tx: bool,
		stmt_kind: &'static str,
		duration_ns: u64,
	) {
		let labels = self.actor_labels();
		METRICS
			.sqlite_worker_command_duration_seconds
			.with_label_values(&[
				labels[0],
				operation,
				self.actor_lifecycle_bucket_label(),
				if in_tx { "true" } else { "false" },
				stmt_kind,
			])
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
		10.0, 25.0, 50.0,
	]
}

fn set_aggregated_gauge(current: &mut i64, next: i64, gauge: &IntGaugeVec, labels: &[&str]) {
	let delta = next.saturating_sub(*current);
	if delta != 0 {
		gauge.with_label_values(labels).add(delta);
		*current = next;
	}
}

impl StartupTimer {
	pub(crate) fn set_is_new(&mut self, is_new: bool) {
		self.is_new = Some(is_new);
		self.metrics.set_startup_is_new(is_new);
	}

	pub(crate) fn finish_success(mut self) -> Duration {
		let duration = self.started_at.elapsed();
		self.metrics.finish_startup();
		self.metrics.observe_startup_phase(
			startup_phase::StartupPhase::Total,
			self.is_new,
			"success",
			duration,
		);
		self.finished = true;
		duration
	}
}

impl Drop for StartupTimer {
	fn drop(&mut self) {
		if self.finished {
			return;
		}

		self.metrics.observe_startup_phase(
			startup_phase::StartupPhase::Total,
			self.is_new,
			"error",
			self.started_at.elapsed(),
		);
	}
}

fn optional_is_new_label(is_new: Option<bool>) -> &'static str {
	match is_new {
		Some(true) => "true",
		Some(false) => "false",
		None => "unknown",
	}
}

fn startup_duration_buckets() -> Vec<f64> {
	vec![
		0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
	]
}

fn usize_to_i64(value: usize) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(feature = "sqlite-local")]
fn u64_to_i64(value: u64) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
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
