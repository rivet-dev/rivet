use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
#[cfg(feature = "sqlite-local")]
use std::sync::atomic::{AtomicU64, AtomicUsize};
#[cfg(feature = "sqlite-local")]
use std::sync::{LazyLock, OnceLock, mpsc};
use std::time::Duration;
#[cfg(feature = "sqlite-local")]
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
#[cfg(feature = "sqlite-local")]
use rivet_metrics::prometheus::{Counter, Histogram, IntCounter, IntGauge};
use rivet_metrics::{
	REGISTRY,
	prometheus::{
		CounterVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts,
		register_counter_vec_with_registry, register_histogram_vec_with_registry,
		register_int_counter_vec_with_registry, register_int_gauge_vec_with_registry,
	},
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
const RUNTIME_CALLBACK_LABELS: &[&str] = &["actor_name", "callback"];
const STARTUP_PHASE_LABELS: &[&str] = &["actor_name", "phase", "is_new", "result"];
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
		/// `ensure_startup_storage`, before persisted state loads.
		StartupStorage = 6,
		/// Probing for the internal SQLite schema.
		SchemaCheck = 7,
		/// Probing legacy KV before bootstrapping a fresh actor.
		KvProbe = 8,
		/// The fresh actor's inspector token KV mirror write. It runs in the background once
		/// the startup commit is durable.
		InspectorToken = 10,
		/// Opening a fresh actor's database. Its bootstrap commits later, with its first
		/// transaction.
		DatabaseOpen = 11,
		/// From the runtime receiving the start to its first storage request, the start of
		/// `ensure_startup_storage`: context build, task spawn and the start handoff.
		BeforeStorage = 12,
		/// Making a new actor's startup writes durable as one commit: its bootstrap, schema
		/// migrations and initial state.
		StartupCommit = 13,
		/// The startup commit's round trip from the VFS to the engine and back, the part of
		/// `StartupCommit` spent outside this process.
		StartupCommitTransport = 14,
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
				StartupPhase::StartupStorage => "startup_storage",
				StartupPhase::SchemaCheck => "schema_check",
				StartupPhase::KvProbe => "kv_probe",
				StartupPhase::InspectorToken => "inspector_token",
				StartupPhase::DatabaseOpen => "database_open",
				StartupPhase::BeforeStorage => "before_storage",
				StartupPhase::StartupCommit => "startup_commit",
				StartupPhase::StartupCommitTransport => "startup_commit_transport",
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
				6 => StartupPhase::StartupStorage,
				7 => StartupPhase::SchemaCheck,
				8 => StartupPhase::KvProbe,
				10 => StartupPhase::InspectorToken,
				11 => StartupPhase::DatabaseOpen,
				12 => StartupPhase::BeforeStorage,
				13 => StartupPhase::StartupCommit,
				14 => StartupPhase::StartupCommitTransport,
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
const SQLITE_OPEN_PHASE_LABELS: &[&str] = &["actor_name", "phase", "is_new", "result"];
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
#[cfg(feature = "sqlite-local")]
const SQLITE_QUERY_LABELS: &[&str] = &[
	"actor_name",
	"type",
	"fingerprint",
	"fingerprint_source",
	"transaction_mode",
	"storage_transport",
];

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
	#[cfg(feature = "sqlite-local")]
	sqlite_profiling: crate::SqliteProfilingConfig,
	#[cfg(feature = "sqlite-local")]
	sqlite_profile_selection: super::sqlite::profiling::QuerySelection,
	#[cfg(feature = "sqlite-local")]
	sqlite_profile_low_card_handles: OnceLock<Option<Arc<SqliteLowCardMetricHandles>>>,
	state: Mutex<ActorMetricState>,
	/// Counts this actor on `actor_active_count` until it stops.
	active: Mutex<Option<rivet_metrics::IntGaugeGuard>>,
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
	sqlite_profile_worker_queue_depth: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_profile_worker_inflight: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_profile_coordinator_queue_depth: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_workers_active: i64,
	#[cfg(feature = "sqlite-local")]
	sqlite_vfs_cached_bytes: i64,
}

// Actor lifecycle, connection, queue and shutdown metrics.
lazy_static::lazy_static! {
	static ref ACTOR_ACTIVE_COUNT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_active_count", "current active actors in this process"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_STARTED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_started_total", "total actors started in this process"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_STOPPED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_stopped_total", "total actors stopped in this process"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_STARTUP_PHASE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_startup_phase_duration_seconds",
			"actor startup phase duration in seconds",
		)
		.buckets(startup_duration_buckets()),
		STARTUP_PHASE_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_CREATE_STATE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_create_state_duration_seconds",
			"typed actor state creation time during startup in seconds",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_CREATE_VARS_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_create_vars_duration_seconds",
			"typed actor vars creation time during startup in seconds",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_RUNTIME_CALLBACK_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_runtime_callback_duration_seconds",
			"foreign runtime lifecycle callback time during the startup preamble in seconds",
		)
		.buckets(startup_duration_buckets()),
		RUNTIME_CALLBACK_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_queue_depth", "current actor queue depth"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_QUEUE_MESSAGES_SENT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_queue_messages_sent_total", "total queue messages sent"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_QUEUE_MESSAGES_RECEIVED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_queue_messages_received_total", "total queue messages received"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_CONNECTIONS_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_connections_active", "current active actor connections"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_CONNECTIONS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_connections_total",
			"total successfully established actor connections",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_INBOX_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_inbox_depth", "current actor inbox depth"),
		INBOX_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_USER_TASKS_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_user_tasks_active", "current active actor user tasks"),
		USER_TASK_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_USER_TASK_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_user_task_duration_seconds",
			"actor user task execution time in seconds",
		),
		USER_TASK_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_HTTP_REQUESTS_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_http_requests_active", "current actor-scoped HTTP requests"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_KEEP_AWAKE_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_keep_awake_active", "current actor keep-awake work"),
		WORK_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SHUTDOWN_TASKS_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_shutdown_tasks_active",
			"current actor work draining during shutdown",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SHUTDOWN_WAIT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_shutdown_wait_seconds",
			"actor shutdown wait time in seconds",
		),
		SHUTDOWN_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SHUTDOWN_TIMEOUT_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_shutdown_timeout_total", "total actor shutdown timeout events"),
		SHUTDOWN_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_STATE_MUTATION_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_state_mutation_total", "total actor state mutations"),
		STATE_MUTATION_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_DIRECT_SUBSYSTEM_SHUTDOWN_WARNING_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_direct_subsystem_shutdown_warning_total",
			"total actor shutdown warnings emitted by direct subsystem drains",
		),
		DIRECT_SHUTDOWN_LABELS,
		*REGISTRY
	).unwrap();
}

// Native SQLite VFS and worker metrics.
#[cfg(feature = "sqlite-local")]
lazy_static::lazy_static! {
	static ref ACTOR_SQLITE_OPEN_PHASE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_open_phase_duration_seconds",
			"native SQLite open phase duration in seconds",
		)
		.buckets(startup_duration_buckets()),
		SQLITE_OPEN_PHASE_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_STARTUP_PRELOAD_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_startup_preload_pages_total",
			"total SQLite startup preload pages requested or loaded",
		),
		SQLITE_STARTUP_PRELOAD_PAGE_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_PREFETCH_UNUSED_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_prefetch_unused_pages_total",
			"prefetched SQLite pages discarded before first use, including close and TTL expiry",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_RESOLVE_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_resolve_pages_total",
			"total VFS page resolution attempts",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_RESOLVE_PAGES_REQUESTED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_resolve_pages_requested_total",
			"total pages requested by VFS page resolution attempts",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_RESOLVE_PAGES_CACHE_HITS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_resolve_pages_cache_hits_total",
			"total pages resolved from the VFS page buffers or write buffer",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_RESOLVE_PAGES_CACHE_MISSES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_resolve_pages_cache_misses_total",
			"total pages missing from the VFS page buffers and write buffer",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_GET_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_get_pages_total",
			"total VFS to engine get_pages requests",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_PAGES_FETCHED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_pages_fetched_total",
			"total pages requested from the engine by VFS get_pages calls",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_PREFETCH_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_prefetch_pages_total",
			"total pages requested speculatively by VFS prefetch",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_BYTES_FETCHED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_bytes_fetched_total",
			"total bytes requested from the engine by VFS get_pages calls",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_PREFETCH_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_prefetch_bytes_total",
			"total bytes requested speculatively by VFS prefetch",
		),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_GET_PAGES_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_vfs_get_pages_duration_seconds",
			"VFS get_pages request duration in seconds",
		)
		.buckets(vec![0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
		SQLITE_VFS_LIFECYCLE_BUCKET_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_COMMIT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_actor_sqlite_vfs_commit_total", "total successful VFS commits"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_COMMIT_DIRTY_PAGES: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_vfs_commit_dirty_pages",
			"dirty pages written by each successful VFS commit",
		)
		.buckets(rivet_metrics::PAGE_COUNT_BUCKETS.to_vec()),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_COMMIT_PHASE_DURATION_SECONDS_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_commit_phase_duration_seconds_total",
			"cumulative VFS commit phase duration in seconds",
		),
		SQLITE_COMMIT_PHASE_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_COMMIT_DURATION_SECONDS_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_commit_duration_seconds_total",
			"cumulative VFS commit duration in seconds",
		),
		SQLITE_COMMIT_PHASE_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_queue_depth",
			"current native SQLite worker SQL command queue depth",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_VFS_CACHED_BYTES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_vfs_cached_bytes",
			"bytes held by native SQLite VFS page buffers",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKERS_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_actor_sqlite_workers_active", "current active native SQLite workers"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_QUEUE_OVERLOAD_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_queue_overload_total",
			"total native SQLite worker SQL command queue overloads",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_COMMAND_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_worker_command_duration_seconds",
			"native SQLite worker SQL command duration in seconds",
		)
		.buckets(sqlite_worker_duration_buckets()),
		SQLITE_WORKER_COMMAND_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_TRANSACTION_ROUND_TRIPS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_transaction_round_trips",
			"network round trips (get_pages + commit) per native SQLite transaction",
		)
		.buckets(sqlite_round_trip_count_buckets()),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_COMMAND_ERROR_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_command_error_total",
			"total native SQLite worker SQL command errors",
		),
		SQLITE_WORKER_ERROR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_CLOSE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_actor_sqlite_worker_close_duration_seconds",
			"native SQLite worker close duration in seconds",
		)
		.buckets(sqlite_worker_duration_buckets()),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_CLOSE_TIMEOUT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_close_timeout_total",
			"total native SQLite worker close timeouts",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_CRASH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_crash_total",
			"total native SQLite worker crashes",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref ACTOR_SQLITE_WORKER_UNCLEAN_CLOSE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_actor_sqlite_worker_unclean_close_total",
			"total native SQLite worker channel drops without clean close",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();
}

// SQLite query profiling metrics. The duration histogram is registered separately by
// `configure_sqlite_duration_histogram`.
#[cfg(feature = "sqlite-local")]
lazy_static::lazy_static! {
	static ref SQLITE_OPERATIONS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_operations_total",
			"SQLite operation count without duration buckets",
		),
		SQLITE_QUERY_LABELS,
		*REGISTRY
	).unwrap();

	static ref SQLITE_DURATION_SECONDS_TOTAL: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_duration_seconds_total",
			"cumulative SQLite operation seconds without duration buckets",
		),
		SQLITE_QUERY_LABELS,
		*REGISTRY
	).unwrap();

	static ref SQLITE_PAGES_FETCHED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_pages_fetched_total",
			"pages returned from storage per SQLite statement",
		),
		SQLITE_QUERY_LABELS,
		*REGISTRY
	).unwrap();

	static ref SQLITE_PHASE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_phase_duration_seconds",
			"SQLite operation phase duration in seconds",
		)
		.buckets(sqlite_worker_duration_buckets()),
		&["actor_name", "type", "fingerprint", "fingerprint_source", "transaction_mode", "storage_transport", "phase"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_GET_PAGES_ROUND_TRIPS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_get_pages_round_trips",
			"physical get_pages calls per SQLite operation",
		)
		.buckets(sqlite_round_trip_count_buckets()),
		&["actor_name", "type", "fingerprint", "fingerprint_source", "transaction_mode", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_TRANSACTION_STATEMENT_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_transaction_statement_count",
			"user statements per explicit SQLite transaction",
		)
		.buckets(sqlite_round_trip_count_buckets()),
		&["actor_name", "type", "fingerprint", "fingerprint_source", "transaction_mode", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_OUTCOME_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_sqlite_outcome_total", "SQLite operation outcomes"),
		&["actor_name", "type", "fingerprint", "fingerprint_source", "transaction_mode", "storage_transport", "outcome"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_LOCAL_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_sqlite_local_pages_total", "local SQLite page totals"),
		&["actor_name", "type", "page_kind", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_LOCAL_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_sqlite_local_bytes_total", "local SQLite logical byte totals"),
		&["actor_name", "type", "byte_kind", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_GET_PAGES_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_get_pages_duration_seconds",
			"physical get_pages request duration in seconds",
		)
		.buckets(sqlite_worker_duration_buckets()),
		&["actor_name", "request_ordinal", "outcome_class", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_GET_PAGES_PAGES: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_get_pages_pages",
			"pages per physical get_pages request",
		)
		.buckets(sqlite_round_trip_count_buckets()),
		&["actor_name", "request_ordinal", "page_kind", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_GET_PAGES_RESPONSE_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_get_pages_response_bytes",
			"bytes per physical get_pages response",
		)
		.buckets(vec![512.0, 4096.0, 16_384.0, 65_536.0, 262_144.0, 1_048_576.0]),
		&["actor_name", "request_ordinal", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_GET_PAGES_MISSING_PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_get_pages_missing_pages_total",
			"missing pages in physical get_pages responses",
		),
		&["actor_name", "request_ordinal", "storage_transport"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_FINGERPRINT_OVERFLOW_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_fingerprint_overflow_total",
			"SQLite observations routed to the shared other fingerprint",
		),
		&["actor_name", "type", "reason"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_EVENT_DROPPED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_event_dropped_total",
			"SQLite diagnostic events dropped before export",
		),
		&["actor_name", "reason"],
		*REGISTRY
	).unwrap();

	static ref SQLITE_WORKER_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_sqlite_worker_queue_depth", "queued native SQLite commands"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref SQLITE_WORKER_INFLIGHT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("rivetkit_sqlite_worker_inflight", "native SQLite commands executing"),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();

	static ref SQLITE_COORDINATOR_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_coordinator_queue_depth",
			"operations waiting for SQLite transaction coordination",
		),
		ACTOR_LABELS,
		*REGISTRY
	).unwrap();
}

/// The configured SQLite duration histogram and its buckets. The buckets come from runtime
/// configuration, so the first actor that enables the histogram registers it.
#[cfg(feature = "sqlite-local")]
static SQLITE_DURATION_SECONDS: Mutex<Option<(Vec<f64>, HistogramVec)>> = Mutex::new(None);

#[cfg(feature = "sqlite-local")]
const SQLITE_PROFILE_PAGE_KINDS: [&str; 11] = [
	"sqlite_requested",
	"cache_hit",
	"cache_miss",
	"depot_demand_requested",
	"response_present",
	"response_missing",
	"overflow_expansion_extra",
	"btree",
	"non_btree",
	"prefetch_consumed",
	"prefetch_unused",
];
#[cfg(feature = "sqlite-local")]
const SQLITE_PROFILE_BYTE_KINDS: [&str; 4] = [
	"bind_logical",
	"result_logical",
	"storage_response",
	"dirty",
];
#[cfg(feature = "sqlite-local")]
const SQLITE_PROFILE_REQUEST_ORDINALS: [&str; 6] = ["1", "2", "3", "4", "5-8", "9+"];
#[cfg(feature = "sqlite-local")]
const SQLITE_PROFILE_REQUEST_PAGE_KINDS: [&str; 3] = [
	"demand_requested",
	"response_present",
	"overflow_expansion_extra",
];
#[cfg(feature = "sqlite-local")]
const SQLITE_PROFILE_OUTCOMES: [&str; 7] = [
	"success",
	"error",
	"rollback",
	"timeout",
	"expired",
	"connection_lost",
	"cancelled",
];

#[cfg(feature = "sqlite-local")]
trait OptionalHistogram {
	fn observe(&self, value: f64);
}
#[cfg(feature = "sqlite-local")]
impl OptionalHistogram for Option<Histogram> {
	fn observe(&self, value: f64) {
		if let Some(metric) = self {
			metric.observe(value);
		}
	}
}
#[cfg(feature = "sqlite-local")]
trait OptionalCounter {
	fn inc(&self);
	fn inc_by(&self, value: u64);
}
#[cfg(feature = "sqlite-local")]
impl OptionalCounter for Option<IntCounter> {
	fn inc(&self) {
		self.inc_by(1);
	}
	fn inc_by(&self, value: u64) {
		if let Some(metric) = self {
			metric.inc_by(value);
		}
	}
}
#[cfg(feature = "sqlite-local")]
fn set_optional_aggregated_gauge(current: &mut i64, next: i64, metric: &Option<IntGauge>) {
	if let Some(metric) = metric {
		metric.add(next - *current);
		*current = next;
	}
}

/// Registry-owned histogram configuration, validated before actors start.
#[cfg(feature = "sqlite-local")]
pub(crate) fn configure_sqlite_duration_histogram(buckets: &[f64]) -> anyhow::Result<()> {
	let mut configured = SQLITE_DURATION_SECONDS.lock();
	if let Some((existing, _)) = configured.as_ref() {
		anyhow::ensure!(
			existing == buckets,
			"SQLite duration histogram buckets must match across actors in a process"
		);
	} else {
		let metric = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_sqlite_duration_seconds",
				"complete SQLite operation wall duration in seconds",
			)
			.buckets(buckets.to_vec()),
			&[
				"actor_name",
				"type",
				"fingerprint",
				"fingerprint_source",
				"transaction_mode",
				"storage_transport",
				"outcome_class",
			],
		)?;
		// Registered by hand because its buckets come from runtime configuration.
		REGISTRY.register(Box::new(metric.clone()))?;
		*configured = Some((buckets.to_vec(), metric));
	}
	Ok(())
}

#[cfg(feature = "sqlite-local")]
struct SqliteFingerprintMetricHandles {
	count: Option<IntCounter>,
	duration_total: Option<Counter>,
	pages_fetched: Option<IntCounter>,
	duration: [Option<Histogram>; 2],

	transaction_wait: Option<Histogram>,
	worker_wait: Option<Histogram>,
	storage: Option<Histogram>,
	local_work: Option<Histogram>,
	application_time: Option<Histogram>,
	commit: Option<Histogram>,
	get_pages_round_trips: Option<Histogram>,
	transaction_statement_count: Option<Histogram>,
	outcomes: [Option<IntCounter>; SQLITE_PROFILE_OUTCOMES.len()],
}

#[cfg(feature = "sqlite-local")]
impl SqliteFingerprintMetricHandles {
	fn new(
		actor_name: &str,
		operation_type: &'static str,
		fingerprint: &str,
		fingerprint_source: &'static str,
		transaction_mode: &'static str,
		storage_transport: &'static str,
		config: &crate::SqliteProfilingConfig,
	) -> Self {
		let metrics = &config.metrics;
		let base = [
			actor_name,
			operation_type,
			fingerprint,
			fingerprint_source,
			transaction_mode,
			storage_transport,
		];
		let phase = |name| {
			metrics.phase_duration_histogram.then(|| {
				SQLITE_PHASE_DURATION_SECONDS.with_label_values(&[
					base[0], base[1], base[2], base[3], base[4], base[5], name,
				])
			})
		};
		let is_transaction = operation_type == "transaction";
		let duration_histogram = metrics.duration && metrics.duration_histogram;
		let counter_duration = metrics.duration && !metrics.duration_histogram;
		Self {
			count: counter_duration.then(|| SQLITE_OPERATIONS_TOTAL.with_label_values(&base)),
			duration_total: counter_duration
				.then(|| SQLITE_DURATION_SECONDS_TOTAL.with_label_values(&base)),
			pages_fetched: (metrics.pages_fetched && !is_transaction)
				.then(|| SQLITE_PAGES_FETCHED_TOTAL.with_label_values(&base)),
			duration: std::array::from_fn(|index| {
				duration_histogram.then(|| {
					SQLITE_DURATION_SECONDS
						.lock()
						.as_ref()
						.expect("validated duration histogram")
						.1
						.with_label_values(&[
							base[0],
							base[1],
							base[2],
							base[3],
							base[4],
							base[5],
							if index == 0 { "success" } else { "non_success" },
						])
				})
			}),
			transaction_wait: phase("transaction_wait"),
			worker_wait: phase("worker_wait"),
			storage: phase("storage"),
			local_work: phase("local_work"),
			application_time: is_transaction.then(|| phase("application_time")).flatten(),
			commit: is_transaction.then(|| phase("commit")).flatten(),
			get_pages_round_trips: metrics
				.get_pages_round_trips_histogram
				.then(|| SQLITE_GET_PAGES_ROUND_TRIPS.with_label_values(&base)),
			transaction_statement_count: (is_transaction
				&& metrics.transaction_statement_count_histogram)
				.then(|| SQLITE_TRANSACTION_STATEMENT_COUNT.with_label_values(&base)),
			outcomes: std::array::from_fn(|index| {
				metrics.outcomes.then(|| {
					SQLITE_OUTCOME_TOTAL.with_label_values(&[
						base[0],
						base[1],
						base[2],
						base[3],
						base[4],
						base[5],
						SQLITE_PROFILE_OUTCOMES[index],
					])
				})
			}),
		}
	}

	fn observe_duration(&self, outcome: &str, duration: f64) {
		self.duration[usize::from(outcome != "success")].observe(duration);
		self.count.inc();
		if let Some(total) = &self.duration_total {
			total.inc_by(duration);
		}
	}

	fn observe_phase(&self, phase: &str, duration: f64) {
		let handle = match phase {
			"transaction_wait" => self.transaction_wait.as_ref(),
			"worker_wait" => self.worker_wait.as_ref(),
			"storage" => self.storage.as_ref(),
			"local_work" => self.local_work.as_ref(),
			"application_time" => self.application_time.as_ref(),
			"commit" => self.commit.as_ref(),
			_ => None,
		};
		if let Some(handle) = handle {
			handle.observe(duration);
		}
	}

	fn record_outcome(&self, outcome: &str) {
		let index = match outcome {
			"success" => 0,
			"error" => 1,
			"rollback" => 2,
			"timeout" => 3,
			"expired" => 4,
			"connection_lost" => 5,
			"cancelled" => 6,
			_ => 1,
		};
		self.outcomes[index].inc();
	}
}

#[cfg(feature = "sqlite-local")]
struct SqliteRequestMetricHandles {
	duration: [Option<Histogram>; 2],
	pages: [Option<Histogram>; SQLITE_PROFILE_REQUEST_PAGE_KINDS.len()],
	response_bytes: Option<Histogram>,
	missing_pages: Option<IntCounter>,
}

#[cfg(feature = "sqlite-local")]
struct SqliteLowCardMetricHandles {
	overflow: [[Option<IntCounter>; 2]; 2],
	local_pages: [[Option<IntCounter>; SQLITE_PROFILE_PAGE_KINDS.len()]; 2],
	local_bytes: [[Option<IntCounter>; SQLITE_PROFILE_BYTE_KINDS.len()]; 2],
	requests: [SqliteRequestMetricHandles; SQLITE_PROFILE_REQUEST_ORDINALS.len()],
	event_dropped: [Option<IntCounter>; 2],
	worker_queue_depth: Option<IntGauge>,
	worker_inflight: Option<IntGauge>,
	coordinator_queue_depth: Option<IntGauge>,
}

#[cfg(feature = "sqlite-local")]
impl fmt::Debug for SqliteLowCardMetricHandles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SqliteLowCardMetricHandles")
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "sqlite-local")]
impl SqliteLowCardMetricHandles {
	fn new(
		actor_name: &str,
		storage_transport: &'static str,
		metrics: &crate::SqliteProfilingMetrics,
	) -> Self {
		let operation_types = ["statement", "transaction"];
		Self {
			overflow: std::array::from_fn(|operation| {
				std::array::from_fn(|reason| {
					metrics.fingerprint_overflow.then(|| {
						SQLITE_FINGERPRINT_OVERFLOW_TOTAL.with_label_values(&[
							actor_name,
							operation_types[operation],
							["logical_cap", "series_budget"][reason],
						])
					})
				})
			}),

			local_pages: std::array::from_fn(|operation_index| {
				std::array::from_fn(|kind_index| {
					metrics.local_pages.then(|| {
						SQLITE_LOCAL_PAGES_TOTAL.with_label_values(&[
							actor_name,
							operation_types[operation_index],
							SQLITE_PROFILE_PAGE_KINDS[kind_index],
							storage_transport,
						])
					})
				})
			}),
			local_bytes: std::array::from_fn(|operation_index| {
				std::array::from_fn(|kind_index| {
					metrics.local_bytes.then(|| {
						SQLITE_LOCAL_BYTES_TOTAL.with_label_values(&[
							actor_name,
							operation_types[operation_index],
							SQLITE_PROFILE_BYTE_KINDS[kind_index],
							storage_transport,
						])
					})
				})
			}),
			requests: std::array::from_fn(|ordinal_index| {
				let ordinal = SQLITE_PROFILE_REQUEST_ORDINALS[ordinal_index];
				SqliteRequestMetricHandles {
					duration: std::array::from_fn(|outcome_index| {
						metrics.get_pages_duration_histogram.then(|| {
							SQLITE_GET_PAGES_DURATION_SECONDS.with_label_values(&[
								actor_name,
								ordinal,
								if outcome_index == 0 {
									"success"
								} else {
									"non_success"
								},
								storage_transport,
							])
						})
					}),
					pages: std::array::from_fn(|kind_index| {
						metrics.get_pages_pages_histogram.then(|| {
							SQLITE_GET_PAGES_PAGES.with_label_values(&[
								actor_name,
								ordinal,
								SQLITE_PROFILE_REQUEST_PAGE_KINDS[kind_index],
								storage_transport,
							])
						})
					}),
					response_bytes: metrics.get_pages_response_bytes_histogram.then(|| {
						SQLITE_GET_PAGES_RESPONSE_BYTES.with_label_values(&[
							actor_name,
							ordinal,
							storage_transport,
						])
					}),
					missing_pages: metrics.get_pages_missing_pages.then(|| {
						SQLITE_GET_PAGES_MISSING_PAGES_TOTAL.with_label_values(&[
							actor_name,
							ordinal,
							storage_transport,
						])
					}),
				}
			}),
			event_dropped: std::array::from_fn(|index| {
				metrics.event_dropped.then(|| {
					SQLITE_EVENT_DROPPED_TOTAL.with_label_values(&[
						actor_name,
						if index == 0 {
							"rate_limit"
						} else {
							"backpressure"
						},
					])
				})
			}),
			worker_queue_depth: metrics
				.worker_queue_depth
				.then(|| SQLITE_WORKER_QUEUE_DEPTH.with_label_values(&[actor_name])),
			worker_inflight: metrics
				.worker_inflight
				.then(|| SQLITE_WORKER_INFLIGHT.with_label_values(&[actor_name])),
			coordinator_queue_depth: metrics
				.coordinator_queue_depth
				.then(|| SQLITE_COORDINATOR_QUEUE_DEPTH.with_label_values(&[actor_name])),
		}
	}
}

#[cfg(feature = "sqlite-local")]
struct AdmittedSqliteProfile {
	fingerprint: String,
	fingerprint_handles: Arc<SqliteFingerprintMetricHandles>,
}

#[cfg(feature = "sqlite-local")]
struct SqliteProfileAdmission {
	statements: scc::HashSet<String>,
	transactions: scc::HashSet<String>,
	tuples: scc::HashMap<String, Arc<SqliteFingerprintMetricHandles>>,
	low_card_tuples: scc::HashMap<String, Arc<SqliteLowCardMetricHandles>>,
	statement_count: AtomicUsize,
	transaction_count: AtomicUsize,
	series: AtomicUsize,
}

#[cfg(feature = "sqlite-local")]
static SQLITE_PROFILE_ADMISSION: LazyLock<SqliteProfileAdmission> =
	LazyLock::new(|| SqliteProfileAdmission {
		statements: scc::HashSet::new(),
		transactions: scc::HashSet::new(),
		tuples: scc::HashMap::new(),
		low_card_tuples: scc::HashMap::new(),
		statement_count: AtomicUsize::new(0),
		transaction_count: AtomicUsize::new(0),
		series: AtomicUsize::new(0),
	});

#[cfg(feature = "sqlite-local")]
#[derive(Debug)]
enum SqliteDiagnosticEvent {
	Operation {
		invocation_id: u64,
		actor_id: String,
		generation: Option<u64>,
		actor_name: String,
		profile: sqlite_embedded::vfs::SqliteOperationMetric,
	},
	Transaction {
		invocation_id: u64,
		actor_id: String,
		generation: Option<u64>,
		actor_name: String,
		profile: sqlite_embedded::vfs::SqliteTransactionMetric,
	},
}

#[cfg(feature = "sqlite-local")]
static SQLITE_DIAGNOSTIC_SENDER: OnceLock<Option<mpsc::SyncSender<SqliteDiagnosticEvent>>> =
	OnceLock::new();
#[cfg(feature = "sqlite-local")]
static SQLITE_DIAGNOSTIC_SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[cfg(feature = "sqlite-local")]
static SQLITE_DIAGNOSTIC_INVOCATION_ID: AtomicU64 = AtomicU64::new(1);
#[cfg(feature = "sqlite-local")]
static SQLITE_DIAGNOSTIC_RATE_STATE: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "sqlite-local")]
static SQLITE_PROFILE_CAPACITY_WARNINGS: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "sqlite-local")]
fn sqlite_diagnostic_sender(
	capacity: usize,
) -> Option<&'static mpsc::SyncSender<SqliteDiagnosticEvent>> {
	SQLITE_DIAGNOSTIC_SENDER
		.get_or_init(|| {
			let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
			match std::thread::Builder::new()
				.name("rivetkit-sqlite-diagnostics".to_owned())
				.spawn(move || {
					while let Ok(event) = receiver.recv() {
						match event {
							SqliteDiagnosticEvent::Operation {
								invocation_id,
								actor_id,
								generation,
								actor_name,
								profile,
							} => tracing::info!(
								target: "rivetkit_sqlite_profile",
								event_type = "operation",
								invocation_id,
								actor_id,
								generation,
								actor_name,
								operation_type = profile.operation_type,
								fingerprint = profile.fingerprint,
								fingerprint_source = profile.fingerprint_source,
								transaction_mode = profile.transaction_mode,
								storage_transport = profile.storage_transport,
								outcome = profile.outcome,
								sql_bytes = profile.sql_bytes,
								total_ns = profile.total_ns,
								transaction_wait_ns = profile.transaction_wait_ns,
								worker_wait_ns = profile.profile.worker_wait_ns,
								storage_ns = profile.profile.storage_ns,
								sqlite_execution_ns = profile.profile.sqlite_execution_ns,
								bind_count = profile.profile.bind_count,
								bind_logical_bytes = profile.profile.bind_logical_bytes,
								result_rows = profile.profile.result_rows,
								result_columns = profile.profile.result_columns,
								result_logical_bytes = profile.profile.result_logical_bytes,
								sqlite_requested_pages = profile.profile.sqlite_requested_pages,
								cache_hit_pages = profile.profile.cache_hit_pages,
								cache_miss_pages = profile.profile.cache_miss_pages,
								response_present_pages = profile.profile.response_present_pages,
								response_missing_pages = profile.profile.response_missing_pages,
								overflow_expansion_extra_pages = profile.profile.overflow_expansion_extra_pages,
								prefetch_consumed_pages = profile.profile.prefetch_consumed_pages,
								prefetch_unused_pages = profile.profile.prefetch_unused_pages,
								dirty_pages = profile.profile.dirty_pages,
								dirty_bytes = profile.profile.dirty_bytes,
								get_pages_requests = ?profile.profile.get_pages_requests,
								omitted_get_pages_requests = profile.profile.omitted_get_pages_requests,
								"sampled SQLite operation profile"
							),
							SqliteDiagnosticEvent::Transaction {
								invocation_id,
								actor_id,
								generation,
								actor_name,
								profile,
							} => tracing::info!(
								target: "rivetkit_sqlite_profile",
								event_type = "transaction",
								invocation_id,
								actor_id,
								generation,
								actor_name,
								fingerprint = profile.fingerprint,
								fingerprint_source = profile.fingerprint_source,
								shape_fingerprint = profile.shape_fingerprint,
								statement_fingerprint_hashes = ?profile.statement_fingerprint_hashes,
								omitted_statement_fingerprints = profile.omitted_statement_fingerprints,
								storage_transport = profile.storage_transport,
								outcome = profile.outcome,
								total_ns = profile.total_ns,
								transaction_wait_ns = profile.transaction_wait_ns,
								worker_wait_ns = profile.worker_wait_ns,
								storage_ns = profile.storage_ns,
								local_work_ns = profile.local_work_ns,
								application_time_ns = profile.application_time_ns,
								commit_ns = profile.commit_ns,
								statement_count = profile.statement_count,
								dirty_pages = profile.dirty_pages,
								dirty_bytes = profile.dirty_bytes,
								"sampled SQLite transaction profile"
							),
						}
					}
				}) {
				Ok(_) => Some(sender),
				Err(error) => {
					tracing::error!(%error, "failed to start SQLite diagnostic exporter");
					None
				}
			}
		})
		.as_ref()
}

#[cfg(feature = "sqlite-local")]
fn sqlite_baseline_sample_selected(rate: f64) -> bool {
	if rate <= 0.0 {
		return false;
	}
	if rate >= 1.0 {
		return true;
	}
	let sequence = SQLITE_DIAGNOSTIC_SAMPLE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
	let mixed = sequence.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17);
	(mixed as f64 / u64::MAX as f64) < rate
}

#[cfg(feature = "sqlite-local")]
fn try_acquire_sqlite_diagnostic_rate(max_events_per_minute: usize) -> bool {
	const COUNT_BITS: u32 = 20;
	const COUNT_MASK: u64 = (1 << COUNT_BITS) - 1;
	let limit = (max_events_per_minute as u64).min(COUNT_MASK);
	if limit == 0 {
		return false;
	}
	let minute = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
		/ 60;
	loop {
		let current = SQLITE_DIAGNOSTIC_RATE_STATE.load(Ordering::Acquire);
		let current_minute = current >> COUNT_BITS;
		let current_count = current & COUNT_MASK;
		let next_count = if current_minute == minute {
			if current_count >= limit {
				return false;
			}
			current_count + 1
		} else {
			1
		};
		let next = (minute << COUNT_BITS) | next_count;
		if SQLITE_DIAGNOSTIC_RATE_STATE
			.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire)
			.is_ok()
		{
			return true;
		}
	}
}

impl ActorMetrics {
	pub(crate) fn new(actor_name: impl Into<String>) -> Self {
		Self::new_with_sqlite_profiling(actor_name, crate::SqliteProfilingConfig::default())
	}

	pub(crate) fn new_with_sqlite_profiling(
		actor_name: impl Into<String>,
		_sqlite_profiling: crate::SqliteProfilingConfig,
	) -> Self {
		let mut _sqlite_profiling = _sqlite_profiling.with_process_override();
		#[cfg(feature = "sqlite-local")]
		if _sqlite_profiling.metrics.duration && _sqlite_profiling.metrics.duration_histogram {
			if let Err(error) = _sqlite_profiling.validate_buckets().and_then(|_| {
				configure_sqlite_duration_histogram(
					&_sqlite_profiling.duration_histogram_buckets_seconds,
				)
			}) {
				tracing::error!(%error, "invalid SQLite histogram configuration; using counters only");
				_sqlite_profiling.metrics.duration_histogram = false;
			}
		}
		let labels = ActorMetricLabels {
			actor_name: actor_name.into(),
		};
		#[cfg(feature = "sqlite-local")]
		let _ = sqlite_diagnostic_sender(_sqlite_profiling.diagnostic_event_queue_capacity);
		let active = rivet_metrics::GaugeGuardExt::inc_guard(
			&ACTOR_ACTIVE_COUNT.with_label_values(&labels.as_label_values()),
		);
		ACTOR_STARTED_TOTAL
			.with_label_values(&labels.as_label_values())
			.inc();
		Self {
			inner: Arc::new(ActorMetricInner {
				labels,
				#[cfg(feature = "sqlite-local")]
				sqlite_profile_selection: super::sqlite::profiling::QuerySelection::new(
					&_sqlite_profiling,
				),
				#[cfg(feature = "sqlite-local")]
				sqlite_profiling: _sqlite_profiling,
				#[cfg(feature = "sqlite-local")]
				sqlite_profile_low_card_handles: OnceLock::new(),
				state: Mutex::new(ActorMetricState::default()),
				active: Mutex::new(Some(active)),
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
		ACTOR_STARTUP_PHASE_DURATION_SECONDS
			.with_label_values(&[
				labels[0],
				phase.as_label(),
				optional_is_new_label(is_new),
				outcome,
			])
			.observe(duration.as_secs_f64());
	}

	/// Records the storage bootstrap, its steps and the persisted-state load once the start
	/// knows whether the actor is new. They run before it does, so they are held until then.
	pub(crate) fn observe_startup_storage(
		&self,
		is_new: bool,
		storage: Duration,
		storage_phases: &[(startup_phase::StartupPhase, Duration)],
		load_persisted: Duration,
	) {
		let is_new = Some(is_new);
		self.observe_startup_phase(
			startup_phase::StartupPhase::StartupStorage,
			is_new,
			"success",
			storage,
		);
		for (phase, duration) in storage_phases {
			self.observe_startup_phase(*phase, is_new, "success", *duration);
		}
		self.observe_startup_phase(
			startup_phase::StartupPhase::LoadPersisted,
			is_new,
			"success",
			load_persisted,
		);
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
		ACTOR_CREATE_STATE_DURATION_SECONDS
			.with_label_values(&self.actor_labels())
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_create_vars(&self, duration: Duration) {
		ACTOR_CREATE_VARS_DURATION_SECONDS
			.with_label_values(&self.actor_labels())
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn observe_runtime_callback(&self, callback: &'static str, duration: Duration) {
		ACTOR_RUNTIME_CALLBACK_DURATION_SECONDS
			.with_label_values(&[self.actor_labels()[0], callback])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn set_queue_depth(&self, depth: u32) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.queue_depth,
			i64::from(depth),
			&ACTOR_QUEUE_DEPTH,
			&labels,
		);
	}

	pub(crate) fn add_queue_messages_sent(&self, count: u64) {
		ACTOR_QUEUE_MESSAGES_SENT_TOTAL
			.with_label_values(&self.actor_labels())
			.inc_by(count);
	}

	pub(crate) fn add_queue_messages_received(&self, count: u64) {
		ACTOR_QUEUE_MESSAGES_RECEIVED_TOTAL
			.with_label_values(&self.actor_labels())
			.inc_by(count);
	}

	pub(crate) fn set_active_connections(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.active_connections,
			usize_to_i64(count),
			&ACTOR_CONNECTIONS_ACTIVE,
			&labels,
		);
	}

	pub(crate) fn inc_connections_total(&self) {
		ACTOR_CONNECTIONS_TOTAL
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
			&ACTOR_INBOX_DEPTH,
			&[labels[0], inbox],
		);
	}

	pub(crate) fn begin_user_task(&self, kind: UserTaskKind) {
		let labels = self.actor_labels();
		let kind = kind.as_metric_label();
		let mut state = self.inner.state.lock();
		let current = state.user_tasks_active.entry(kind).or_default();
		let next = (*current).saturating_add(1);
		set_aggregated_gauge(current, next, &ACTOR_USER_TASKS_ACTIVE, &[labels[0], kind]);
	}

	pub(crate) fn end_user_task(&self, kind: UserTaskKind, duration: Duration) {
		let labels = self.actor_labels();
		let kind = kind.as_metric_label();
		{
			let mut state = self.inner.state.lock();
			let current = state.user_tasks_active.entry(kind).or_default();
			let next = (*current).saturating_sub(1);
			set_aggregated_gauge(current, next, &ACTOR_USER_TASKS_ACTIVE, &[labels[0], kind]);
		}
		ACTOR_USER_TASK_DURATION_SECONDS
			.with_label_values(&[labels[0], kind])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn set_http_requests_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.http_requests_active,
			usize_to_i64(count),
			&ACTOR_HTTP_REQUESTS_ACTIVE,
			&labels,
		);
	}

	pub(crate) fn set_keep_awake_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.keep_awake_active,
			usize_to_i64(count),
			&ACTOR_KEEP_AWAKE_ACTIVE,
			&[labels[0], "keep_awake"],
		);
	}

	pub(crate) fn set_internal_keep_awake_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.internal_keep_awake_active,
			usize_to_i64(count),
			&ACTOR_KEEP_AWAKE_ACTIVE,
			&[labels[0], "internal_keep_awake"],
		);
	}

	pub(crate) fn set_shutdown_tasks_active(&self, count: usize) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.shutdown_tasks_active,
			usize_to_i64(count),
			&ACTOR_SHUTDOWN_TASKS_ACTIVE,
			&labels,
		);
	}

	pub(crate) fn observe_shutdown_wait(&self, reason: ShutdownKind, duration: Duration) {
		let labels = self.actor_labels();
		ACTOR_SHUTDOWN_WAIT_SECONDS
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.observe(duration.as_secs_f64());
	}

	pub(crate) fn inc_shutdown_timeout(&self, reason: ShutdownKind) {
		let labels = self.actor_labels();
		ACTOR_SHUTDOWN_TIMEOUT_TOTAL
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_state_mutation(&self, reason: StateMutationReason) {
		let labels = self.actor_labels();
		ACTOR_STATE_MUTATION_TOTAL
			.with_label_values(&[labels[0], reason.as_metric_label()])
			.inc();
	}

	pub(crate) fn inc_direct_subsystem_shutdown_warning(&self, subsystem: &str, operation: &str) {
		let labels = self.actor_labels();
		ACTOR_DIRECT_SUBSYSTEM_SHUTDOWN_WARNING_TOTAL
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
		let Some(active) = self.active.lock().take() else {
			return;
		};
		drop(active);

		self.clear_aggregated_gauges();
		ACTOR_STOPPED_TOTAL
			.with_label_values(&self.labels.as_label_values())
			.inc();
	}

	fn clear_aggregated_gauges(&self) {
		let labels = self.labels.as_label_values();
		let mut state = self.state.lock();
		set_aggregated_gauge(&mut state.queue_depth, 0, &ACTOR_QUEUE_DEPTH, &labels);
		set_aggregated_gauge(
			&mut state.active_connections,
			0,
			&ACTOR_CONNECTIONS_ACTIVE,
			&labels,
		);
		set_aggregated_gauge(
			&mut state.lifecycle_inbox_depth,
			0,
			&ACTOR_INBOX_DEPTH,
			&[labels[0], "lifecycle"],
		);
		set_aggregated_gauge(
			&mut state.dispatch_inbox_depth,
			0,
			&ACTOR_INBOX_DEPTH,
			&[labels[0], "dispatch"],
		);
		set_aggregated_gauge(
			&mut state.lifecycle_event_inbox_depth,
			0,
			&ACTOR_INBOX_DEPTH,
			&[labels[0], "lifecycle_event"],
		);
		for (kind, current) in state.user_tasks_active.iter_mut() {
			set_aggregated_gauge(current, 0, &ACTOR_USER_TASKS_ACTIVE, &[labels[0], *kind]);
		}
		set_aggregated_gauge(
			&mut state.http_requests_active,
			0,
			&ACTOR_HTTP_REQUESTS_ACTIVE,
			&labels,
		);
		set_aggregated_gauge(
			&mut state.keep_awake_active,
			0,
			&ACTOR_KEEP_AWAKE_ACTIVE,
			&[labels[0], "keep_awake"],
		);
		set_aggregated_gauge(
			&mut state.internal_keep_awake_active,
			0,
			&ACTOR_KEEP_AWAKE_ACTIVE,
			&[labels[0], "internal_keep_awake"],
		);
		set_aggregated_gauge(
			&mut state.shutdown_tasks_active,
			0,
			&ACTOR_SHUTDOWN_TASKS_ACTIVE,
			&labels,
		);
		#[cfg(feature = "sqlite-local")]
		{
			set_aggregated_gauge(
				&mut state.sqlite_worker_queue_depth,
				0,
				&ACTOR_SQLITE_WORKER_QUEUE_DEPTH,
				&labels,
			);
			set_aggregated_gauge(
				&mut state.sqlite_vfs_cached_bytes,
				0,
				&ACTOR_SQLITE_VFS_CACHED_BYTES,
				&labels,
			);
			set_aggregated_gauge(
				&mut state.sqlite_workers_active,
				0,
				&ACTOR_SQLITE_WORKERS_ACTIVE,
				&labels,
			);
			if let Some(Some(handles)) = self.sqlite_profile_low_card_handles.get() {
				set_optional_aggregated_gauge(
					&mut state.sqlite_profile_worker_queue_depth,
					0,
					&handles.worker_queue_depth,
				);
				set_optional_aggregated_gauge(
					&mut state.sqlite_profile_worker_inflight,
					0,
					&handles.worker_inflight,
				);
				set_optional_aggregated_gauge(
					&mut state.sqlite_profile_coordinator_queue_depth,
					0,
					&handles.coordinator_queue_depth,
				);
			}
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
impl ActorMetrics {
	fn sqlite_low_card_series_cost(&self) -> usize {
		let m = &self.inner.sqlite_profiling.metrics;
		22 * usize::from(m.local_pages)
			+ 8 * usize::from(m.local_bytes)
			+ 240 * usize::from(m.get_pages_duration_histogram)
			+ 252 * usize::from(m.get_pages_pages_histogram)
			+ 54 * usize::from(m.get_pages_response_bytes_histogram)
			+ 6 * usize::from(m.get_pages_missing_pages)
			+ 2 * usize::from(m.event_dropped)
			+ 4 * usize::from(m.fingerprint_overflow)
			+ usize::from(m.worker_queue_depth)
			+ usize::from(m.worker_inflight)
			+ usize::from(m.coordinator_queue_depth)
	}
	fn sqlite_query_series_cost(&self, transaction: bool) -> usize {
		let c = &self.inner.sqlite_profiling;
		let m = &c.metrics;
		let duration = if !m.duration {
			0
		} else if m.duration_histogram {
			2 * (c.duration_histogram_buckets_seconds.len() + 3)
		} else {
			2
		};
		duration
			+ usize::from(m.outcomes) * 7
			+ usize::from(m.pages_fetched && !transaction)
			+ usize::from(m.phase_duration_histogram) * (if transaction { 6 } else { 4 }) * 20
			+ usize::from(m.get_pages_round_trips_histogram) * 14
			+ usize::from(m.transaction_statement_count_histogram && transaction) * 14
	}

	fn reserve_sqlite_series(&self, cost: usize) -> bool {
		SQLITE_PROFILE_ADMISSION
			.series
			.try_update(Ordering::AcqRel, Ordering::Acquire, |current| {
				(current.saturating_add(cost) <= self.inner.sqlite_profiling.max_prometheus_series)
					.then_some(current + cost)
			})
			.is_ok()
	}

	fn sqlite_low_card_handles(
		&self,
		storage_transport: &'static str,
	) -> Option<&SqliteLowCardMetricHandles> {
		self.inner
			.sqlite_profile_low_card_handles
			.get_or_init(|| {
				let tuple = format!(
					"{}\0{storage_transport}\0{:x}",
					self.actor_labels()[0],
					self.inner.sqlite_profiling.metrics.cache_key()
				);
				if let Some(handles) = SQLITE_PROFILE_ADMISSION
					.low_card_tuples
					.read_sync(&tuple, |_, handles| Arc::clone(handles))
				{
					return Some(handles);
				}
				if !self.reserve_sqlite_series(self.sqlite_low_card_series_cost()) {
					self.warn_sqlite_profile_capacity("low_card_series_budget", "profile");
					return None;
				}
				let handles = Arc::new(SqliteLowCardMetricHandles::new(
					self.actor_labels()[0],
					storage_transport,
					&self.inner.sqlite_profiling.metrics,
				));
				match SQLITE_PROFILE_ADMISSION.low_card_tuples.entry_sync(tuple) {
					scc::hash_map::Entry::Occupied(entry) => {
						SQLITE_PROFILE_ADMISSION
							.series
							.fetch_sub(self.sqlite_low_card_series_cost(), Ordering::AcqRel);
						Some(Arc::clone(entry.get()))
					}
					scc::hash_map::Entry::Vacant(entry) => {
						entry.insert_entry(Arc::clone(&handles));
						Some(handles)
					}
				}
			})
			.as_deref()
	}

	fn warn_sqlite_profile_capacity(&self, reason: &'static str, operation_type: &'static str) {
		let count = SQLITE_PROFILE_CAPACITY_WARNINGS.fetch_add(1, Ordering::Relaxed) + 1;
		if count == 1 || count.is_power_of_two() {
			tracing::warn!(
				actor_name = self.actor_labels()[0],
				operation_type,
				reason,
				overflow_observations = count,
				"SQLite profiling capacity reached; observations are being dropped"
			);
		}
	}

	fn record_sqlite_fingerprint_overflow(
		&self,
		operation_type: &'static str,
		reason: &'static str,
	) {
		if !self.inner.sqlite_profiling.metrics.fingerprint_overflow {
			return;
		}
		if let Some(handles) = self.sqlite_low_card_handles("proxy") {
			handles.overflow[usize::from(operation_type == "transaction")]
				[usize::from(reason == "series_budget")]
			.inc();
		}
		self.warn_sqlite_profile_capacity(reason, operation_type);
	}

	/// Explicit selections are admitted immediately, within the existing series caps.
	fn select_sqlite_fingerprint(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
	) -> Option<bool> {
		if !self.inner.sqlite_profile_selection.contains(fingerprint) {
			return None;
		}
		let (set, count, cap) = if operation_type == "transaction" {
			(
				&SQLITE_PROFILE_ADMISSION.transactions,
				&SQLITE_PROFILE_ADMISSION.transaction_count,
				self.inner
					.sqlite_profiling
					.max_tracked_transaction_fingerprints,
			)
		} else {
			(
				&SQLITE_PROFILE_ADMISSION.statements,
				&SQLITE_PROFILE_ADMISSION.statement_count,
				self.inner
					.sqlite_profiling
					.max_tracked_statement_fingerprints,
			)
		};
		if set.contains_sync(fingerprint) {
			return Some(false);
		}
		if count
			.try_update(Ordering::AcqRel, Ordering::Acquire, |current| {
				(current < cap).then_some(current + 1)
			})
			.is_err()
		{
			self.record_sqlite_fingerprint_overflow(operation_type, "logical_cap");
			return None;
		}
		if set.insert_sync(fingerprint.to_owned()).is_err() {
			count.fetch_sub(1, Ordering::AcqRel);
			Some(false)
		} else {
			Some(true)
		}
	}

	fn rollback_sqlite_logical_admission(&self, operation_type: &'static str, fingerprint: &str) {
		let (set, count) = if operation_type == "transaction" {
			(
				&SQLITE_PROFILE_ADMISSION.transactions,
				&SQLITE_PROFILE_ADMISSION.transaction_count,
			)
		} else {
			(
				&SQLITE_PROFILE_ADMISSION.statements,
				&SQLITE_PROFILE_ADMISSION.statement_count,
			)
		};
		if set.remove_sync(fingerprint).is_some() {
			count.fetch_sub(1, Ordering::AcqRel);
		}
	}

	fn admit_sqlite_fingerprint_tuple(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
		fingerprint_source: &'static str,
		transaction_mode: &'static str,
		storage_transport: &'static str,
	) -> Option<Arc<SqliteFingerprintMetricHandles>> {
		let cost = self.sqlite_query_series_cost(operation_type == "transaction");
		if cost == 0 {
			return None;
		}
		let tuple = format!(
			"{}\0{operation_type}\0{fingerprint}\0{fingerprint_source}\0{transaction_mode}\0{storage_transport}\0{:x}",
			self.actor_labels()[0],
			self.inner.sqlite_profiling.metrics.cache_key()
		);
		if let Some(handles) = SQLITE_PROFILE_ADMISSION
			.tuples
			.read_sync(&tuple, |_, handles| Arc::clone(handles))
		{
			return Some(handles);
		}
		if !self.reserve_sqlite_series(cost) {
			return None;
		}
		let handles = Arc::new(SqliteFingerprintMetricHandles::new(
			self.actor_labels()[0],
			operation_type,
			fingerprint,
			fingerprint_source,
			transaction_mode,
			storage_transport,
			&self.inner.sqlite_profiling,
		));
		match SQLITE_PROFILE_ADMISSION.tuples.entry_sync(tuple) {
			scc::hash_map::Entry::Occupied(entry) => {
				SQLITE_PROFILE_ADMISSION
					.series
					.fetch_sub(cost, Ordering::AcqRel);
				Some(Arc::clone(entry.get()))
			}
			scc::hash_map::Entry::Vacant(entry) => {
				entry.insert_entry(Arc::clone(&handles));
				Some(handles)
			}
		}
	}

	fn admitted_sqlite_fingerprint(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
		fingerprint_source: &'static str,
		transaction_mode: &'static str,
		storage_transport: &'static str,
	) -> Option<AdmittedSqliteProfile> {
		if self.sqlite_query_series_cost(operation_type == "transaction") == 0 {
			return None;
		}

		let (fingerprint, fingerprint_source, newly_admitted) =
			if self.inner.sqlite_profile_selection.contains(fingerprint) {
				(
					fingerprint,
					fingerprint_source,
					self.select_sqlite_fingerprint(operation_type, fingerprint)?,
				)
			} else {
				return None;
			};
		if let Some(fingerprint_handles) = self.admit_sqlite_fingerprint_tuple(
			operation_type,
			fingerprint,
			fingerprint_source,
			transaction_mode,
			storage_transport,
		) {
			return Some(AdmittedSqliteProfile {
				fingerprint: fingerprint.to_owned(),
				fingerprint_handles,
			});
		}
		if newly_admitted {
			self.rollback_sqlite_logical_admission(operation_type, fingerprint);
		}
		self.record_sqlite_fingerprint_overflow(operation_type, "series_budget");
		None
	}

	fn observe_profile_common(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
		fingerprint_source: &'static str,
		transaction_mode: &'static str,
		storage_transport: &'static str,
		outcome: &'static str,
		total_ns: u64,
		phases: &[(&'static str, u64)],
		get_pages_round_trips: u64,
	) -> Option<AdmittedSqliteProfile> {
		let _ = self.sqlite_low_card_handles(storage_transport);

		let admitted = self.admitted_sqlite_fingerprint(
			operation_type,
			fingerprint,
			fingerprint_source,
			transaction_mode,
			storage_transport,
		)?;
		admitted
			.fingerprint_handles
			.observe_duration(outcome, ns_to_seconds(total_ns));
		for (phase, duration_ns) in phases {
			admitted
				.fingerprint_handles
				.observe_phase(phase, ns_to_seconds(*duration_ns));
		}
		admitted
			.fingerprint_handles
			.get_pages_round_trips
			.observe(get_pages_round_trips as f64);
		admitted.fingerprint_handles.record_outcome(outcome);
		Some(admitted)
	}

	fn record_local_profile_totals(
		&self,
		operation_type: &'static str,
		handles: &SqliteLowCardMetricHandles,
		profile: &sqlite_embedded::vfs::SqliteOperationProfile,
	) {
		let operation_index = usize::from(operation_type == "transaction");
		for (index, pages) in [
			profile.sqlite_requested_pages,
			profile.cache_hit_pages,
			profile.cache_miss_pages,
			profile.depot_demand_requested_pages,
			profile.response_present_pages,
			profile.response_missing_pages,
			profile.overflow_expansion_extra_pages,
			profile.btree_pages,
			profile.non_btree_pages,
			profile.prefetch_consumed_pages,
			profile.prefetch_unused_pages,
		]
		.into_iter()
		.enumerate()
		{
			if pages > 0 {
				handles.local_pages[operation_index][index].inc_by(pages);
			}
		}
		for (index, bytes) in [
			profile.bind_logical_bytes,
			profile.result_logical_bytes,
			profile.storage_response_bytes,
			profile.dirty_bytes,
		]
		.into_iter()
		.enumerate()
		{
			if bytes > 0 {
				handles.local_bytes[operation_index][index].inc_by(bytes);
			}
		}
	}

	fn emit_sqlite_diagnostic_event(&self, event: SqliteDiagnosticEvent) {
		let low_card_handles = self
			.inner
			.sqlite_profile_low_card_handles
			.get()
			.and_then(|handles| handles.as_deref());
		if !try_acquire_sqlite_diagnostic_rate(
			self.inner.sqlite_profiling.max_diagnostic_events_per_minute,
		) {
			if let Some(handles) = low_card_handles {
				handles.event_dropped[0].inc();
			}
			return;
		}
		let Some(sender) =
			sqlite_diagnostic_sender(self.inner.sqlite_profiling.diagnostic_event_queue_capacity)
		else {
			if let Some(handles) = low_card_handles {
				handles.event_dropped[1].inc();
			}
			return;
		};
		if sender.try_send(event).is_err()
			&& let Some(handles) = low_card_handles
		{
			handles.event_dropped[1].inc();
		}
	}
}

#[cfg(feature = "sqlite-local")]
impl sqlite_embedded::vfs::SqliteVfsMetrics for ActorMetrics {
	fn operation_diagnostic_selected(
		&self,
		profile: &sqlite_embedded::vfs::SqliteOperationMetric,
	) -> bool {
		let slow = profile.total_ns
			>= self
				.inner
				.sqlite_profiling
				.slow_operation_threshold_ms
				.saturating_mul(1_000_000);
		let page_amplification = profile.profile.response_present_pages
			> profile
				.profile
				.sqlite_requested_pages
				.saturating_mul(8)
				.saturating_add(16);
		let byte_amplification = profile.profile.storage_response_bytes > 64 * 1024 * 1024
			|| profile.profile.dirty_bytes > 64 * 1024 * 1024;
		slow || profile.outcome != "success"
			|| page_amplification
			|| byte_amplification
			|| sqlite_baseline_sample_selected(self.inner.sqlite_profiling.baseline_sample_rate)
	}

	fn transaction_diagnostic_selected(
		&self,
		profile: &sqlite_embedded::vfs::SqliteTransactionMetric,
	) -> bool {
		profile.total_ns
			>= self
				.inner
				.sqlite_profiling
				.slow_operation_threshold_ms
				.saturating_mul(1_000_000)
			|| profile.outcome != "success"
			|| profile.dirty_bytes > 64 * 1024 * 1024
			|| sqlite_baseline_sample_selected(self.inner.sqlite_profiling.baseline_sample_rate)
	}

	fn profiling_enabled(&self) -> bool {
		true
	}

	fn max_profiled_get_pages_requests(&self) -> usize {
		self.inner.sqlite_profiling.max_get_pages_requests_per_trace
	}

	fn observe_operation_profile(
		&self,
		profile: &sqlite_embedded::vfs::SqliteOperationMetric,
	) -> bool {
		let local_work_ns = profile
			.total_ns
			.saturating_sub(profile.transaction_wait_ns)
			.saturating_sub(profile.profile.worker_wait_ns)
			.saturating_sub(profile.profile.storage_ns);
		let admitted = self.observe_profile_common(
			profile.operation_type,
			&profile.fingerprint,
			profile.fingerprint_source,
			profile.transaction_mode,
			profile.storage_transport,
			profile.outcome,
			profile.total_ns,
			&[
				("transaction_wait", profile.transaction_wait_ns),
				("worker_wait", profile.profile.worker_wait_ns),
				("storage", profile.profile.storage_ns),
				("local_work", local_work_ns),
			],
			profile.profile.get_pages_round_trips,
		);
		if let Some(admitted) = &admitted {
			admitted
				.fingerprint_handles
				.pages_fetched
				.inc_by(profile.profile.response_present_pages);
		}
		let Some(low_card_handles) = self.sqlite_low_card_handles(profile.storage_transport) else {
			return admitted.is_some();
		};
		self.record_local_profile_totals(
			profile.operation_type,
			low_card_handles,
			&profile.profile,
		);
		for request in profile.profile.get_pages_requests.iter().flatten() {
			let handles = &low_card_handles.requests[sqlite_request_ordinal_index(request.ordinal)];
			handles.duration[usize::from(!request.success)]
				.observe(ns_to_seconds(request.duration_ns));
			for (index, pages) in [
				request.demand_requested,
				request.response_present,
				request.overflow_expansion_extra,
			]
			.into_iter()
			.enumerate()
			{
				handles.pages[index].observe(pages as f64);
			}
			handles
				.response_bytes
				.observe(request.response_bytes as f64);
			if request.response_missing > 0 {
				handles.missing_pages.inc_by(request.response_missing);
			}
		}
		admitted.is_some_and(|admitted| admitted.fingerprint == profile.fingerprint)
	}

	fn observe_transaction_profile(
		&self,
		profile: &sqlite_embedded::vfs::SqliteTransactionMetric,
	) -> bool {
		let admitted = self.observe_profile_common(
			"transaction",
			&profile.fingerprint,
			profile.fingerprint_source,
			"explicit",
			profile.storage_transport,
			profile.outcome,
			profile.total_ns,
			&[
				("application_time", profile.application_time_ns),
				("transaction_wait", profile.transaction_wait_ns),
				("worker_wait", profile.worker_wait_ns),
				("storage", profile.storage_ns),
				("local_work", profile.local_work_ns),
				("commit", profile.commit_ns),
			],
			profile.get_pages_round_trips,
		);
		if let Some(admitted) = &admitted {
			admitted
				.fingerprint_handles
				.transaction_statement_count
				.observe(profile.statement_count as f64);
		}
		let Some(low_card_handles) = self.sqlite_low_card_handles(profile.storage_transport) else {
			return admitted.is_some();
		};
		let mut aggregate = sqlite_embedded::vfs::SqliteOperationProfile::default();
		aggregate.dirty_pages = profile.dirty_pages;
		aggregate.dirty_bytes = profile.dirty_bytes;
		self.record_local_profile_totals("transaction", low_card_handles, &aggregate);
		admitted.is_some_and(|admitted| admitted.fingerprint == profile.fingerprint)
	}

	fn emit_operation_diagnostic_event(
		&self,
		actor_id: &str,
		generation: Option<u64>,
		profile: &sqlite_embedded::vfs::SqliteOperationMetric,
	) {
		self.emit_sqlite_diagnostic_event(SqliteDiagnosticEvent::Operation {
			invocation_id: SQLITE_DIAGNOSTIC_INVOCATION_ID.fetch_add(1, Ordering::Relaxed),
			actor_id: actor_id.to_owned(),
			generation,
			actor_name: self.actor_labels()[0].to_owned(),
			profile: profile.clone(),
		});
	}

	fn emit_transaction_diagnostic_event(
		&self,
		actor_id: &str,
		generation: Option<u64>,
		profile: &sqlite_embedded::vfs::SqliteTransactionMetric,
	) {
		self.emit_sqlite_diagnostic_event(SqliteDiagnosticEvent::Transaction {
			invocation_id: SQLITE_DIAGNOSTIC_INVOCATION_ID.fetch_add(1, Ordering::Relaxed),
			actor_id: actor_id.to_owned(),
			generation,
			actor_name: self.actor_labels()[0].to_owned(),
			profile: profile.clone(),
		});
	}

	fn record_fingerprint_catalog(
		&self,
		operation_type: &'static str,
		fingerprint: &str,
		identity: &str,
		format_version: u8,
	) {
		tracing::info!(
			actor_name = self.actor_labels()[0],
			operation_type,
			fingerprint,
			identity,
			format_version,
			"sqlite fingerprint catalog"
		);
	}
	fn record_resolve_pages(&self, requested_pages: u64) {
		let labels = self.sqlite_vfs_labels();
		ACTOR_SQLITE_VFS_RESOLVE_PAGES_TOTAL
			.with_label_values(&labels)
			.inc();
		ACTOR_SQLITE_VFS_RESOLVE_PAGES_REQUESTED_TOTAL
			.with_label_values(&labels)
			.inc_by(requested_pages);
	}

	fn record_resolve_cache_hits(&self, pages: u64) {
		ACTOR_SQLITE_VFS_RESOLVE_PAGES_CACHE_HITS_TOTAL
			.with_label_values(&self.sqlite_vfs_labels())
			.inc_by(pages);
	}

	fn record_resolve_cache_misses(&self, pages: u64) {
		ACTOR_SQLITE_VFS_RESOLVE_PAGES_CACHE_MISSES_TOTAL
			.with_label_values(&self.sqlite_vfs_labels())
			.inc_by(pages);
	}

	fn record_get_pages_request(&self, pages: u64, prefetch_pages: u64, page_size: u64) {
		let labels = self.sqlite_vfs_labels();
		ACTOR_SQLITE_VFS_GET_PAGES_TOTAL
			.with_label_values(&labels)
			.inc();
		ACTOR_SQLITE_VFS_PAGES_FETCHED_TOTAL
			.with_label_values(&labels)
			.inc_by(pages);
		ACTOR_SQLITE_VFS_PREFETCH_PAGES_TOTAL
			.with_label_values(&labels)
			.inc_by(prefetch_pages);
		ACTOR_SQLITE_VFS_BYTES_FETCHED_TOTAL
			.with_label_values(&labels)
			.inc_by(pages.saturating_mul(page_size));
		ACTOR_SQLITE_VFS_PREFETCH_BYTES_TOTAL
			.with_label_values(&labels)
			.inc_by(prefetch_pages.saturating_mul(page_size));
	}

	fn observe_get_pages_duration(&self, duration_ns: u64) {
		ACTOR_SQLITE_VFS_GET_PAGES_DURATION_SECONDS
			.with_label_values(&self.sqlite_vfs_labels())
			.observe(ns_to_seconds(duration_ns));
	}

	fn observe_open_phase(
		&self,
		phase: sqlite_embedded::vfs::SqliteOpenPhase,
		outcome: &'static str,
		duration_ns: u64,
	) {
		let labels = self.actor_labels();
		ACTOR_SQLITE_OPEN_PHASE_DURATION_SECONDS
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
		ACTOR_SQLITE_STARTUP_PRELOAD_PAGES_TOTAL
			.with_label_values(&[labels[0], self.startup_is_new_label(), kind])
			.inc_by(pages);
	}

	fn record_commit(&self, dirty_pages: u64) {
		let labels = self.actor_labels();
		ACTOR_SQLITE_VFS_COMMIT_TOTAL
			.with_label_values(&labels)
			.inc();
		ACTOR_SQLITE_VFS_COMMIT_DIRTY_PAGES
			.with_label_values(&labels)
			.observe(dirty_pages as f64);
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
			ACTOR_SQLITE_VFS_COMMIT_PHASE_DURATION_SECONDS_TOTAL
				.with_label_values(&[labels[0], phase])
				.inc_by(ns_to_seconds(duration_ns));
		}
		ACTOR_SQLITE_VFS_COMMIT_DURATION_SECONDS_TOTAL
			.with_label_values(&[labels[0], "total"])
			.inc_by(ns_to_seconds(total_ns));
	}

	fn set_worker_queue_depth(&self, depth: u64) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.sqlite_worker_queue_depth,
			u64_to_i64(depth),
			&ACTOR_SQLITE_WORKER_QUEUE_DEPTH,
			&labels,
		);
		if let Some(Some(handles)) = self.inner.sqlite_profile_low_card_handles.get() {
			set_optional_aggregated_gauge(
				&mut state.sqlite_profile_worker_queue_depth,
				u64_to_i64(depth),
				&handles.worker_queue_depth,
			);
		}
	}

	fn set_worker_active(&self, active: bool) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.sqlite_workers_active,
			if active { 1 } else { 0 },
			&ACTOR_SQLITE_WORKERS_ACTIVE,
			&labels,
		);
	}

	fn set_worker_inflight(&self, active: bool) {
		if let Some(Some(handles)) = self.inner.sqlite_profile_low_card_handles.get() {
			set_optional_aggregated_gauge(
				&mut self.inner.state.lock().sqlite_profile_worker_inflight,
				i64::from(active),
				&handles.worker_inflight,
			);
		}
	}

	fn record_prefetch_unused_pages(&self, pages: u64) {
		ACTOR_SQLITE_VFS_PREFETCH_UNUSED_PAGES_TOTAL
			.with_label_values(&self.actor_labels())
			.inc_by(pages);
	}

	fn set_vfs_cached_bytes(&self, bytes: u64) {
		let labels = self.actor_labels();
		let mut state = self.inner.state.lock();
		set_aggregated_gauge(
			&mut state.sqlite_vfs_cached_bytes,
			u64_to_i64(bytes),
			&ACTOR_SQLITE_VFS_CACHED_BYTES,
			&labels,
		);
	}

	fn set_coordinator_queue_depth(&self, depth: u64) {
		if let Some(Some(handles)) = self.inner.sqlite_profile_low_card_handles.get() {
			set_optional_aggregated_gauge(
				&mut self
					.inner
					.state
					.lock()
					.sqlite_profile_coordinator_queue_depth,
				u64_to_i64(depth),
				&handles.coordinator_queue_depth,
			);
		}
	}

	fn record_worker_queue_overload(&self) {
		ACTOR_SQLITE_WORKER_QUEUE_OVERLOAD_TOTAL
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
		ACTOR_SQLITE_WORKER_COMMAND_DURATION_SECONDS
			.with_label_values(&[
				labels[0],
				operation,
				self.actor_lifecycle_bucket_label(),
				if in_tx { "true" } else { "false" },
				stmt_kind,
			])
			.observe(ns_to_seconds(duration_ns));
	}

	fn observe_transaction_round_trips(&self, get_pages_round_trips: u64, commit_round_trips: u64) {
		ACTOR_SQLITE_TRANSACTION_ROUND_TRIPS
			.with_label_values(&self.actor_labels())
			.observe((get_pages_round_trips.saturating_add(commit_round_trips)) as f64);
	}

	fn record_worker_command_error(&self, operation: &'static str, code: &'static str) {
		let labels = self.actor_labels();
		ACTOR_SQLITE_WORKER_COMMAND_ERROR_TOTAL
			.with_label_values(&[labels[0], operation, code])
			.inc();
	}

	fn observe_worker_close_duration(&self, duration_ns: u64) {
		ACTOR_SQLITE_WORKER_CLOSE_DURATION_SECONDS
			.with_label_values(&self.actor_labels())
			.observe(ns_to_seconds(duration_ns));
	}

	fn record_worker_close_timeout(&self) {
		ACTOR_SQLITE_WORKER_CLOSE_TIMEOUT_TOTAL
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn record_worker_crash(&self) {
		ACTOR_SQLITE_WORKER_CRASH_TOTAL
			.with_label_values(&self.actor_labels())
			.inc();
	}

	fn record_worker_unclean_close(&self) {
		ACTOR_SQLITE_WORKER_UNCLEAN_CLOSE_TOTAL
			.with_label_values(&self.actor_labels())
			.inc();
	}
}

#[cfg(feature = "sqlite-local")]
fn ns_to_seconds(duration_ns: u64) -> f64 {
	Duration::from_nanos(duration_ns).as_secs_f64()
}

#[cfg(feature = "sqlite-local")]
fn sqlite_request_ordinal_index(ordinal: u64) -> usize {
	match ordinal {
		1 => 0,
		2 => 1,
		3 => 2,
		4 => 3,
		5..=8 => 4,
		_ => 5,
	}
}

#[cfg(feature = "sqlite-local")]
fn sqlite_worker_duration_buckets() -> Vec<f64> {
	vec![
		0.000_1, 0.000_5, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
		10.0, 25.0, 50.0,
	]
}

#[cfg(feature = "sqlite-local")]
fn sqlite_round_trip_count_buckets() -> Vec<f64> {
	vec![
		1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0,
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.
