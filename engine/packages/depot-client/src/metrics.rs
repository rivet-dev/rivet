//! Process-wide client ownership bounds; volume keys never become metric labels.

use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	/// Filesystem, journal and volume client fences by reason; see `fences::FenceReason`.
	pub(crate) static ref FENCES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_client_fences_total", "Depot filesystem, journal and volume client fences, and fenced refusals, by reason."),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Filesystem read requests to the maintainer, including routing retries.
	pub(crate) static ref FS_READ_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("depot_filesystem_read_seconds", "Filesystem read request latency from the client, by operation: open, get, scan, chunk, close, base_chunks, fork_metadata, base_keys or point.")
			.buckets(rivet_metrics::START_BUCKETS.to_vec()),
		&["operation"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Rows a completed filesystem read used, by the tier that held them; see
	/// `filesystem::cache::ReadSource`.
	pub(crate) static ref FS_READ_SERVED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_read_served_total", "Rows completed filesystem reads used, by where they came from: layer (an overlay row in memory), entry_cache (a row of an index-only layer the row cache held), log (a log entry read from the log shards), cold_cache (a snapshot row the row cache held) or remote (a tablet maintainer read session)."),
		&["source"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub(crate) static ref FS_READ_SERVED_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_read_served_bytes_total", "Value bytes of the rows completed filesystem reads used, by the same sources as depot_filesystem_read_served_total."),
		&["source"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref FS_COMMIT_QUEUE_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("depot_filesystem_commit_queue_seconds", "Time a filesystem commit waited while another commit group on its volume was in flight, including its own append when that group carried it.")
			.buckets(rivet_metrics::START_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref COMMAND_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("depot_client_command_duration_seconds", "Typed command latency from submission to final reply, by result: ok, overloaded, read_interrupted, deadline or error.")
			.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["command", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub(crate) static ref COMMAND_REJECTIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_client_command_rejections_total", "Typed command attempts the storage host refused or transport failed, by reason. A not_maintainer attempt follows the refreshed route; every other reason returns to the caller."),
		&["command", "reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// The bounded cause behind each filesystem `Overloaded` error a client returns.
	pub(crate) static ref FS_OVERLOADED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_overloaded_total", "Filesystem requests refused as overloaded, by cause."),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Filesystem opens started again because the cold store moved past the history they read.
	pub(crate) static ref FS_OPEN_RESTARTS: IntCounter = register_int_counter_with_registry!(
		"depot_filesystem_open_restarts_total",
		"Filesystem opens started again from a fresh history because the cold store moved past the one they read.",
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Views rebuilt on a newer cold snapshot after the log trimmed an entry they needed, by
	/// result: rebased (read on at the same sequence) or moved_past (the cold store passed it).
	pub(crate) static ref FS_VIEW_REBASES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_view_rebases_total", "Filesystem views rebuilt on a newer cold snapshot after the log trimmed an entry they needed, by result: rebased or moved_past."),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Bytes charged to filesystem memory budgets across the process: the indexes of committed
	/// layers above each filesystem's snapshot and the rows cached beside them.
	pub(crate) static ref FS_MEMORY_BYTES: IntGauge = register_int_gauge_with_registry!(
		"depot_filesystem_memory_bytes",
		"Bytes charged to open filesystems' memory budgets: committed layer indexes above their snapshots and cached rows.",
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Combined memory budget of the process's open filesystems.
	pub(crate) static ref FS_MEMORY_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"depot_filesystem_memory_capacity_bytes",
		"Combined memory budget of open filesystems.",
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Moves of a filesystem's snapshot floor, which send no request, by what triggered them:
	/// expired notes, memory pressure (`memory_unapplied` when past the applied position), applied
	/// held layers, or cold passing the snapshot.
	pub(crate) static ref FS_SNAPSHOT_MOVES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_snapshot_moves_total", "Filesystem snapshot floor moves by trigger."),
		&["trigger"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref FS_OPEN_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("depot_filesystem_open_phase_duration_seconds", "Filesystem open time by phase: log recovery, cold snapshot open, replay, root setup, orphan reclaim, and staging the format of an empty volume.")
			.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["phase"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref FS_PROCESS_CACHE_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_filesystem_process_cache_bytes", "Bytes retained by filesystem row caches across the process."),
		*rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_PROCESS_CACHE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_filesystem_process_cache_capacity_bytes", "Configured process-wide filesystem cache budget."),
		*rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_IDLE_CACHE_RELEASES: IntCounter = register_int_counter_with_registry!(
		Opts::new("depot_filesystem_idle_cache_releases_total", "Filesystem caches released after an idle period."),
		*rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_NOTE_EXPIRIES: IntCounter = register_int_counter_with_registry!(
		Opts::new("depot_filesystem_note_expiries_total", "Overlay layers dropped by the note timer; misses on their rows read them from the maintainer at or past them."),
		*rivet_metrics::REGISTRY).unwrap();

	pub(crate) static ref FS_TRANSACTIONS: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_filesystem_transactions", "Open filesystem transaction handles."), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_TRANSACTION_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_filesystem_transaction_bytes", "Application bytes admitted in open filesystem transactions."), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_TRANSACTION_ABORTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_transaction_aborts_total", "Filesystem transactions aborted by reason."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_UNKNOWN_OUTCOMES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_filesystem_unknown_outcomes_total", "Filesystem writes whose outcome was unknown at the caller's deadline while their volume client kept serving, by stage: admit (the admission reply), commit (the commit wait) or reconciled (entries later read back from the log and installed)."), &["stage"], *rivet_metrics::REGISTRY).unwrap();

	pub(crate) static ref KV_COMMITTED: IntCounter = register_int_counter_with_registry!(
		Opts::new("depot_filesystem_kv_committed_total","Filesystem KV transactions acknowledged by the volume log."),
		*rivet_metrics::REGISTRY).unwrap();

	pub(crate) static ref DIRTY_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_client_dirty_bytes", "Log bytes above cold across running volume clients."),
		*rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref VOLUME_SLOTS: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_client_volume_slots",
			"Owned volume cache slots, including unresolved cleanup.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref VOLUME_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_client_volume_capacity",
			"Combined cache capacity of running Depot clients.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref VOLUME_REQUESTS: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_client_volume_requests",
			"Accepted queued and running volume requests, including abandoned callers.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref REQUEST_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_client_request_capacity",
			"Combined request capacity of running Depot clients.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Per-volume visits made by recurring idle loops. Each loop should cost O(changed
	/// volumes), so a rate that grows with loaded volumes instead of writes is a regression.
	static ref IDLE_VISITS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_idle_visits_total", "Per-volume visits made by recurring idle loops, by loop."),
		&["component"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref REJECTED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_client_rejected_total", "Local volume admission rejections by bounded resource."),
		&["resource"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

/// Records `visits` per-volume visits by the idle loop `component`. The worker's command host
/// shares this counter, so it is registered once here.
pub fn record_idle_visits(component: &'static str, visits: usize) {
	if visits > 0 {
		IDLE_VISITS
			.with_label_values(&[component])
			.inc_by(visits as u64);
	}
}
