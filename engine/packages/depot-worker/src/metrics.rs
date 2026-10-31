//! Process-wide cold-store observations. Tablet identities label the runner busy counter
//! and one scrape-time info series per worst-tablet signal. Open completions aggregate
//! across all local maintainers, including those that have stopped.

use parking_lot::Mutex;
use rivet_metrics::{
	GaugeGuardExt,
	prometheus::{core::Collector, *},
};
use slatedb::VersionedManifest;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
};

lazy_static::lazy_static! {
	pub(crate) static ref READ_SESSIONS: IntGauge = register_int_gauge_with_registry!("depot_filesystem_read_sessions", "Filesystem read sessions open across the tablets this process maintains.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref READ_SESSION_ROWS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_filesystem_read_rows_total", "Rows tablet maintainer read sessions returned, by snapshot: cold when the session's position was already in cold, so every row came from SSTs, or includes_applied when its snapshot also held applied rows still in the memtable."), &["snapshot"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref READ_SESSION_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_filesystem_read_bytes_total", "Value bytes tablet maintainer read sessions returned, by the same snapshot label as depot_tablet_filesystem_read_rows_total."), &["snapshot"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref READ_SESSION_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_filesystem_read_session_capacity", "Configured filesystem read session capacity across the tablets this process maintains.", *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref COORDINATOR_PLACEMENT: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_coordinator_placement", "Confirmed tablet placement counts by kind."), &["kind"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref COORDINATOR_WIDTH_IMBALANCE: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_coordinator_width_imbalance_ppm", "Maximum excess over capacity-proportional hash width, in parts per million."), &["worker_role"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref COORDINATOR_PLACEMENT_BLOCKED: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_coordinator_placement_blocked", "Current placement block reason; exactly one reason is 1."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref COORDINATOR_DISK_PROBES: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_coordinator_disk_probes", "Disk admission probes in flight, waiting workers, and the configured capacity."), &["kind"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref RETENTION_CHECKS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_maintainer_retention_checks_total", "Due retention checks of registered volumes by result: logged when the check logged a retention checkpoint op, parked when the volume was unchanged since its newest retention checkpoint so the check logged nothing and the next apply arms it again, or deferred when store work of the volume or the maintainer log made it retry."), &["result"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref STATUS_OBSERVATIONS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_maintainer_status_observations_total", "Replica status observations by source: a pushed notice, or a poll that found a change or none."), &["source"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MEMBER_REPAIRS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_maintainer_member_repairs_total", "Maintainer reloads of a member missing from a loaded volume, by result: started, loaded or failed."), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_LOOP_WAKES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_loop_wakes_total", "Tablet service loop wakes by reason."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MAINTAINER_RECOVERY_SCANS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_maintainer_recovery_scans_total", "Tablet maintainer recovery scans by scan and the event that started them."), &["scan", "trigger"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref SLATEDB_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_cold_store_slatedb_bytes_total", "SlateDB bytes by stat: memtable writes, L0 flushes and compaction output."), &["stat"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref COLD_STORE_BACKPRESSURE: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_cold_store_backpressure_total", "SlateDB writes held because a database's unflushed bytes reached its limit: each check that made a write wait (wait), and each wait that hit SlateDB's 30 s timeout with no flush finished (timeout). SlateDB logs these only at debug."), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref SLATEDB_FLUSHES: IntCounter = register_int_counter_with_registry!("depot_cold_store_slatedb_memtable_flushes_total", "Immutable memtables SlateDB uploaded as L0 SSTs.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref STOP_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_stop_duration_seconds", "Coordinated shutdown wait, including cancellation."), &["result"], *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref STOP_BLOCKING: IntGauge = register_int_gauge_with_registry!("depot_stop_blocking_tablets", "Tablets currently blocking this worker's shutdown.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref STOP_NO_PROGRESS: Gauge = register_gauge_with_registry!("depot_stop_seconds_since_progress", "Seconds without shutdown progress.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COORDINATOR_STOP_REQUESTS: IntGauge = register_int_gauge_with_registry!("depot_coordinator_stop_requests", "Durable stop entries in the coordinator manifest.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref TABLETS: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_tablets", "Tablets this worker maintains (maintained) or holds a replica of (replica), from its latest manifest."), &["worker_role"], *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COLD_STORE_UNFLUSHED: IntGauge = register_int_gauge_with_registry!("depot_cold_store_unflushed_bytes", "SlateDB unflushed memtable and WAL bytes across every database of this process, as of each database's last write.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COLD_STORE_UNFLUSHED_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_cold_store_unflushed_capacity_bytes", "Process budget for SlateDB unflushed bytes, split across maintained tablets.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COLD_STORE_COMPACTIONS: IntGauge = register_int_gauge_with_registry!("depot_cold_store_compactions_in_flight", "SlateDB compactions running across every database of this process.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COLD_STORE_COMPACTIONS_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_cold_store_compactions_capacity", "Most SlateDB compactions this process runs at once: maintained tablets times the per-database limit.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COORDINATOR_REGRANTS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_coordinator_regrants_total", "Tablets whose maintainer left storage discovery: re-granted to a live member (granted) or left without one (no_member)."), &["result"], *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COORDINATOR_WRITE_COMMANDS: Histogram = register_histogram_with_registry!("depot_coordinator_write_commands", "Queued coordinator commands applied by one manifest write.", rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref COORDINATOR_STOP_PERMITS: IntCounter = register_int_counter_with_registry!("depot_coordinator_stop_permits_total", "Durably granted worker stop permits.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_COMMAND_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_kv_command_phase_duration_seconds", "Storage-side KV command time waiting for its volume to open and running its handler.").buckets(rivet_metrics::BUCKETS.to_vec()), &["command", "phase"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FS_READ_REJECTED: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_filesystem_read_rejected_total", "Client filesystem reads a tablet maintainer refused as overloaded, by cause."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FS_READ_EXPIRED: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_filesystem_read_expired_total", "Client filesystem reads a tablet maintainer answered unavailable at their deadline, by where they waited and by operation."), &["stage", "operation"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_STORE_LOCK_WAIT: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_filesystem_store_lock_wait_seconds", "Time a filesystem store operation waited for a store lock, by lock mode, operation and scope (the tablet lock or volume stripes).").buckets(rivet_metrics::MICRO_BUCKETS.to_vec()), &["mode", "operation", "scope"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FLUSH_DEFERRED: IntCounter = register_int_counter_with_registry!("depot_tablet_flush_deferred_volumes_total", "Requested volumes a tablet filesystem flush skipped because their own work was in flight or queued ahead, and owes to the next flush.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FLUSH_PIGGYBACKED: IntCounter = register_int_counter_with_registry!("depot_tablet_flush_piggybacked_volumes_total", "Volumes a tablet filesystem flush published without their own flush request, because their applied data was already in the flushed memtable.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FLUSH_RECORDS: Histogram = register_histogram_with_registry!("depot_tablet_flush_records", "Volume records carried by one tablet filesystem flush.", rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_STORE_OPERATION_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_tablet_store_operation_duration_seconds", "Execution time of one exclusive tablet store operation, excluding its admission wait.").buckets(rivet_metrics::BUCKETS.to_vec()), &["operation"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_REPLICA_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_tablet_replica_request_duration_seconds", "Maintainer replica request latency, including timeouts, by whether the replica is on the maintainer's own worker (local) or another (remote).").buckets(rivet_metrics::BUCKETS.to_vec()), &["request", "result", "locality"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref LOG_SHARD_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_log_shard_request_duration_seconds", "Storage-side log shard request latency from admission to reply, by request.").buckets(rivet_metrics::BUCKETS.to_vec()), &["request"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_VOLUME_OPEN_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_tablet_volume_open_duration_seconds", "Maintainer-side volume open latency from admission to reply, by reply.").buckets(rivet_metrics::BUCKETS.to_vec()), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_VOLUME_OPEN_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(HistogramOpts::new("depot_kv_volume_open_phase_duration_seconds", "Storage-side KV volume open time by the command whose admission started the open and by phase: volume client recovery, filesystem open and control inode lookup.").buckets(rivet_metrics::BUCKETS.to_vec()), &["command", "phase"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_VOLUME_OPENS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_kv_volume_opens_total", "Finished KV volume opens by the command whose admission started the open and by result; an unavailable open reports its reason."), &["command", "result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_SPECULATIVE_CREATES: IntCounter = register_int_counter_with_registry!(Opts::new("depot_kv_speculative_creates_total", "First commands of a created volume answered from their speculative run, their writes committed in the volume's creating frame."), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_VOLUME_CLOSES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_kv_volume_closes_total", "KV volume closes by the command whose admission started the open and by reason."), &["command", "reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_WRITERLESS_READS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_kv_writerless_reads_total", "Read commands for a volume the host had not opened, by command and result: served on a read session, volume not found, refused as overloaded, or unavailable."), &["command", "result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_UNLOAD_DURATION: Histogram = register_histogram_with_registry!("depot_tablet_unload_duration_seconds", "Time from a maintainer unload's begin to the last confirmed replica forget.", rivet_metrics::BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_USAGE: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_kv_usage", "Current KV volumes, handlers and request bytes."), &["kind"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("depot_kv_capacity", "Configured KV capacity across active maintainers."), &["kind"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_VOLUME_EVICTIONS: IntCounter = register_int_counter_with_registry!(Opts::new("kv_volume_evictions_total", "Idle KV volumes evicted at capacity."), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref KV_VOLUME_EVICTED_IDLE: Histogram = register_histogram_with_registry!("kv_volume_evicted_idle_seconds", "Idle time before a capacity eviction.", rivet_metrics::LIFETIME_BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();

 pub(crate) static ref TABLET_MANIFEST_IO: IntCounterVec = register_int_counter_vec_with_registry!(
  Opts::new("depot_tablet_manifest_io_total", "Manifest read and conditional-write results, including uncertain outcomes."),
  &["operation", "result"],
  *rivet_metrics::REGISTRY
 ).unwrap();

	pub(crate) static ref CATCH_UP_BLOCKED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_learner_catch_up_blocked_total",
			"Learner copies blocked by an indivisible entry exceeding read or append limits.",
		),
		&["stage"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref L0_STALLS: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_cold_store_l0_stalls_total",
			"Times SlateDB held a memtable upload because a tree's L0 was full. Tablet flushes and writes past the unflushed budget wait for the next compaction while this rises.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref COMPACTION_WAIT_PASSES: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_cold_store_compaction_wait_passes_total",
			"Cold write or deletion passes with an observed SlateDB L0 stall, including failed or cancelled passes. Each pass counts once.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DELETE_SWEEP_RESULTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_cold_delete_sweep_results_total",
			"Bounded cold deletion sweep attempts by result.",
		),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DELETE_ROWS: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_filesystem_delete_rows_total",
			"Filesystem rows removed by successfully flushed deletion sweeps.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DELETE_SWEEP_SECONDS: Histogram = register_histogram_with_registry!(
		"depot_cold_delete_sweep_seconds",
		"Elapsed storage time for a bounded deletion sweep including its flush barrier.",
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_tablet_store_queue_depth",
			"Tablet store work waiting for admission across active maintainers, by queue.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_IN_FLIGHT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_tablet_store_in_flight",
			"Tablet store operations in flight across active maintainers, by access class.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_KEYED_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_tablet_store_keyed_capacity",
			"Configured filesystem apply and load slots summed across active maintainers.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_ADMISSION_BLOCKED: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_store_admission_blocked_seconds_total",
			"Time the oldest queued tablet store item could not start, summed across active maintainers, by reason.",
		),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_object_store_request_duration_seconds",
			"Object store request latency through the production adapter, by operation, the tablet store purpose that issued it, the object it touches (SlateDB sources, or heartbeat, lease, config, tablet_manifest, tablet_other and other for cluster objects), and outcome. A request its caller dropped, such as on a deadline, is recorded as result abandoned at its elapsed time. A foreground sst GET is a block cache miss.",
		).buckets(rivet_metrics::BUCKETS.to_vec()),
		&["op", "purpose", "source", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_REQUEST_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_object_store_request_bytes",
			"Payload bytes of object store puts and gets through the production adapter, by operation and the object source it touches.",
		).buckets(exponential_buckets(1024.0, 4.0, 10).unwrap()),
		&["op", "source"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_READ_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_tablet_store_read_capacity",
			"Configured filesystem read slots summed across active maintainers.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref STORES: Mutex<StoreObservations> = Mutex::new(StoreObservations::default());
	static ref L0_MAX: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_cold_store_l0_ssts_max",
			"Highest sampled per-tree L0 SST count among open tablet handles.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref RUNS_MAX: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_cold_store_sorted_runs_max",
			"Highest sampled sorted-run count among open tablet handles.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref L0_CAP_MAX: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_cold_store_l0_limit_max",
			"Highest configured L0 SST limit among open tablet handles.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref SST_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_cold_store_referenced_sst_bytes",
			"Estimated physical SST bytes referenced by open tablet handles; clone sharing and overlapping owners can count the same file more than once.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref LAG_MAX: Gauge = register_gauge_with_registry!(
		Opts::new(
			"depot_cold_store_compaction_lag_seconds_max",
			"Age at publication of the oldest sampled L0 SST; includes idle small runs and parent files visible in clones.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref CACHE_BYTES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_cold_store_cache_bytes",
			"Estimated bytes held by the process's shared SlateDB caches, by cache.",
		),
		&["cache"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();

	static ref CACHE_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_cold_store_cache_capacity_bytes",
			"Configured capacity of the process's shared SlateDB caches, by cache.",
		),
		&["cache"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();

}

lazy_static::lazy_static! {
	static ref DISK_CACHE_PARTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_cold_store_disk_cache_parts_total",
			"Object part reads through the cold-store disk cache: every access, and those that hit. An access that does not hit is an object-store GET.",
		),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_CACHE_USAGE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_cold_store_disk_cache_usage",
			"Contents of the cold-store disk cache, by unit: bytes or keys.",
		),
		&["unit"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_CACHE_EVICTED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_cold_store_disk_cache_evicted_total",
			"Entries evicted from the cold-store disk cache to stay under its capacity, by unit: bytes or keys.",
		),
		&["unit"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_disk_capacity_bytes",
			"Resolved local disk capacity of this storage worker, by resource: the log shard and the cold-store disk cache.",
		),
		&["resource"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref COLD_STORE_CACHE_ACCESSES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_cold_store_cache_accesses_total",
			"SlateDB cache lookups by every cold-store database, by entry kind and hit or miss. A data block miss is an object-store GET.",
		),
		&["entry_kind", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref COLD_STORE_FILTER_CHECKS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_cold_store_filter_checks_total",
			"SlateDB SST filter checks by query kind and verdict. A negative skips the SST without reading a data block.",
		),
		&["kind", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

static NEXT_OBSERVER: AtomicU64 = AtomicU64::new(1);

/// One manifest observation; these values never control writes, admission or retry behavior.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct StoreSample {
	l0: u64,
	runs: u64,
	l0_limit: u64,
	referenced_bytes: u64,
	oldest_l0_ms: Option<u64>,
}

impl StoreSample {
	/// Deduplicate physical SSTs within a manifest even when it has multiple projected views.
	fn from_manifest(manifest: &VersionedManifest, l0_limit: usize) -> Self {
		let l0 = manifest
			.l0()
			.iter()
			.chain(manifest.segments().iter().flat_map(|segment| segment.l0()));
		let mut sample = Self {
			l0_limit: l0_limit as u64,
			l0: manifest.l0().len() as u64,
			runs: manifest.compacted().len() as u64,
			..Default::default()
		};
		for view in l0 {
			let timestamp = view.sst.id.value().timestamp_ms();
			sample.oldest_l0_ms = Some(
				sample
					.oldest_l0_ms
					.map_or(timestamp, |old| old.min(timestamp)),
			);
		}
		for segment in manifest.segments() {
			// SlateDB applies the cap independently to each tree.
			sample.l0 = sample.l0.max(segment.l0().len() as u64);
			sample.runs += segment.compacted().len() as u64;
		}

		// Multiple projected views can refer to one physical file.
		let mut seen = BTreeSet::new();
		for view in manifest.all_sst_views() {
			if seen.insert(view.sst.id.value()) {
				sample.referenced_bytes = sample
					.referenced_bytes
					.saturating_add(view.sst.estimate_size());
			}
		}
		sample
	}

	/// Near is three quarters of the L0 limit, rounded up, so the warning fires while
	/// compaction still has room to catch up.
	fn pressure(&self) -> Pressure {
		let near = self.l0_limit.saturating_mul(3).div_ceil(4);
		if self.l0 >= self.l0_limit {
			Pressure::Full
		} else if self.l0 > 0 && self.l0 >= near {
			Pressure::Near
		} else {
			Pressure::Normal
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Pressure {
	Normal,
	Near,
	Full,
}

/// Registers one open handle’s samples; dropping it removes them from the process aggregates.
pub(crate) struct ColdStoreObserver {
	id: u64,
	tablet_id: uuid::Uuid,
	l0_limit: usize,
	/// The caches every database of this process shares, if its factory has them.
	shared_cache: Option<Arc<dyn rivet_env::capabilities::slatedb::SharedCacheUsage>>,
	pressure: Pressure,
}

impl ColdStoreObserver {
	pub(crate) fn new(
		tablet_id: uuid::Uuid,
		l0_limit: usize,
		shared_cache: Option<Arc<dyn rivet_env::capabilities::slatedb::SharedCacheUsage>>,
	) -> Self {
		let observer = Self {
			id: NEXT_OBSERVER.fetch_add(1, Ordering::Relaxed),
			tablet_id,
			l0_limit,
			shared_cache,
			pressure: Pressure::Normal,
		};
		let mut stores = STORES.lock();
		stores.update(
			observer.id,
			StoreSample {
				l0_limit: l0_limit as u64,
				..Default::default()
			},
		);
		publish(&stores);
		observer
	}

	/// Samples the manifest of `db`, replacing this handle's last sample, and publishes the
	/// shared cache usage. Warns once per upward pressure transition; sampling a low count
	/// rearms the warning.
	pub(crate) fn observe(&mut self, db: &slatedb::Db) {
		if let Some(shared) = &self.shared_cache {
			publish_cache(shared.usage());
		}
		let sample = StoreSample::from_manifest(&db.manifest(), self.l0_limit);
		COLD_STORE_L0_SSTS_SAMPLED.observe(sample.l0 as f64);
		COLD_STORE_SORTED_RUNS_SAMPLED.observe(sample.runs as f64);
		let pressure = sample.pressure();
		if pressure > self.pressure {
			match pressure {
				Pressure::Near => tracing::warn!(
					tablet_id = %self.tablet_id,
					l0_ssts = sample.l0,
					l0_limit = sample.l0_limit,
					"cold-store L0 count near its limit; memtable flushes are outpacing compaction"
				),
				Pressure::Full => tracing::warn!(
					tablet_id = %self.tablet_id,
					l0_ssts = sample.l0,
					l0_limit = sample.l0_limit,
					"cold-store L0 count at its limit; memtable flushes now wait for compaction"
				),
				Pressure::Normal => {}
			}
		}

		self.pressure = pressure;
		let mut stores = STORES.lock();
		stores.update(self.id, sample);
		publish(&stores);
	}
}

impl ColdStoreObserver {
	/// Whether this handle's last sample had L0 at its limit, so a memtable flush must
	/// wait for a compaction before it can upload.
	pub(crate) fn l0_full(&self) -> bool {
		self.pressure == Pressure::Full
	}
}



impl Drop for ColdStoreObserver {
	fn drop(&mut self) {
		let mut stores = STORES.lock();
		stores.remove(self.id);
		publish(&stores);
	}
}

/// Aggregation runs under the observation lock so closing one tablet cannot overwrite a newer sample.
fn publish(stores: &StoreObservations) {
	let aggregate = stores.aggregate();
	let gauge = |value: u64| value.min(i64::MAX as u64) as i64;
	L0_MAX.set(gauge(aggregate.l0));
	RUNS_MAX.set(gauge(aggregate.runs));
	L0_CAP_MAX.set(gauge(aggregate.l0_limit));
	SST_BYTES.set(gauge(aggregate.referenced_bytes));
	LAG_MAX.set(aggregate.oldest_l0_ms.map_or(0.0, |oldest| {
		telemetry_now_ms().saturating_sub(oldest) as f64 / 1000.0
	}));
}

/// Forwards the cold-store disk cache's counters and gauges.
struct DiskCacheRecorder;

/// Sets one gauge of the process-wide disk cache.
struct UsageGauge(IntGauge);

impl slatedb_common::metrics::GaugeFn for UsageGauge {
	fn set(&self, value: i64) {
		self.0.set(value);
	}
}

impl slatedb_common::metrics::MetricsRecorder for DiskCacheRecorder {
	fn register_counter(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::CounterFn> {
		use slatedb::cached_object_store_stats as stats;

		if name == stats::PART_ACCESS_COUNT {
			Arc::new(PrometheusCounter(
				DISK_CACHE_PARTS.with_label_values(&["access"]),
			))
		} else if name == stats::PART_HIT_COUNT {
			Arc::new(PrometheusCounter(
				DISK_CACHE_PARTS.with_label_values(&["hit"]),
			))
		} else if name == stats::EVICTED_BYTES {
			Arc::new(PrometheusCounter(
				DISK_CACHE_EVICTED.with_label_values(&["bytes"]),
			))
		} else if name == stats::EVICTED_KEYS {
			Arc::new(PrometheusCounter(
				DISK_CACHE_EVICTED.with_label_values(&["keys"]),
			))
		} else {
			slatedb_common::metrics::NoopMetricsRecorder.register_counter(name, description, labels)
		}
	}

	fn register_gauge(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::GaugeFn> {
		use slatedb::cached_object_store_stats as stats;

		if name == stats::CACHE_BYTES {
			Arc::new(UsageGauge(DISK_CACHE_USAGE.with_label_values(&["bytes"])))
		} else if name == stats::CACHE_KEYS {
			Arc::new(UsageGauge(DISK_CACHE_USAGE.with_label_values(&["keys"])))
		} else {
			slatedb_common::metrics::NoopMetricsRecorder.register_gauge(name, description, labels)
		}
	}

	fn register_up_down_counter(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::UpDownCounterFn> {
		slatedb_common::metrics::NoopMetricsRecorder.register_up_down_counter(
			name,
			description,
			labels,
		)
	}

	fn register_histogram(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
		boundaries: &[f64],
	) -> Arc<dyn slatedb_common::metrics::HistogramFn> {
		slatedb_common::metrics::NoopMetricsRecorder.register_histogram(
			name,
			description,
			labels,
			boundaries,
		)
	}
}

/// The recorder for the process's cold-store disk cache. Exports
/// `depot_cold_store_disk_cache_parts_total{result="access"|"hit"}`; the hit ratio is hits
/// over accesses.
pub(crate) fn disk_cache_recorder() -> Arc<dyn slatedb_common::metrics::MetricsRecorder> {
	Arc::new(DiskCacheRecorder)
}

/// Publishes the capacities resolved at startup. A disabled cache reports zero.
pub(crate) fn publish_disk_budget(budget: &crate::disk_budget::DiskBudget) {
	let gauge = |value: u64| value.min(i64::MAX as u64) as i64;
	DISK_CAPACITY
		.with_label_values(&["log"])
		.set(gauge(budget.log_disk_bytes));
	let cache = budget
		.cache
		.as_ref()
		.map_or(0, |cache| cache.capacity_bytes);
	DISK_CAPACITY
		.with_label_values(&["cache"])
		.set(gauge(cache));
}

/// The shared caches are process-wide, so the latest observation replaces the gauges.
fn publish_cache(usage: rivet_env::capabilities::slatedb::SlateDbCacheUsage) {
	let gauge = |value: u64| value.min(i64::MAX as u64) as i64;
	for (cache, bytes, capacity) in [
		("block", usage.block_bytes, usage.block_capacity_bytes),
		("meta", usage.meta_bytes, usage.meta_capacity_bytes),
	] {
		CACHE_BYTES.with_label_values(&[cache]).set(gauge(bytes));
		CACHE_CAPACITY
			.with_label_values(&[cache])
			.set(gauge(capacity));
	}
}

/// Ordered indexes keep each update logarithmic in the number of open handles.
/// The byte total uses a wider accumulator so removal remains exact after a gauge saturates.
#[derive(Default)]
struct StoreObservations {
	samples: BTreeMap<u64, StoreSample>,
	l0: BTreeSet<(u64, u64)>,
	runs: BTreeSet<(u64, u64)>,
	limits: BTreeSet<(u64, u64)>,
	oldest: BTreeSet<(u64, u64)>,
	referenced_bytes: u128,
}

impl StoreObservations {
	fn update(&mut self, id: u64, sample: StoreSample) {
		self.remove(id);
		self.l0.insert((sample.l0, id));
		self.runs.insert((sample.runs, id));
		self.limits.insert((sample.l0_limit, id));
		if let Some(oldest) = sample.oldest_l0_ms {
			self.oldest.insert((oldest, id));
		}
		self.referenced_bytes += u128::from(sample.referenced_bytes);
		self.samples.insert(id, sample);
	}

	fn remove(&mut self, id: u64) {
		let Some(sample) = self.samples.remove(&id) else {
			return;
		};
		self.l0.remove(&(sample.l0, id));
		self.runs.remove(&(sample.runs, id));
		self.limits.remove(&(sample.l0_limit, id));
		if let Some(oldest) = sample.oldest_l0_ms {
			self.oldest.remove(&(oldest, id));
		}
		self.referenced_bytes -= u128::from(sample.referenced_bytes);
	}

	/// Read the aggregate without scanning registrations or retaining closed owners' maxima.
	fn aggregate(&self) -> StoreSample {
		StoreSample {
			l0: self.l0.last().map_or(0, |&(value, _)| value),
			runs: self.runs.last().map_or(0, |&(value, _)| value),
			l0_limit: self.limits.last().map_or(0, |&(value, _)| value),
			oldest_l0_ms: self.oldest.first().map(|&(value, _)| value),
			referenced_bytes: self.referenced_bytes.min(u128::from(u64::MAX)) as u64,
		}
	}
}

/// Telemetry uses real time and never feeds the deterministic state machine.
#[allow(clippy::disallowed_methods, reason = "real-time telemetry only")]
fn telemetry_now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis()
		.min(u64::MAX as u128) as u64
}

/// Observes SlateDB's actual L0-stall callbacks and unflushed total for one database handle.
/// This is not a metric registry: exported collectors remain process-global. The stall
/// observations never influence state-machine behavior; the unflushed limit is reported to
/// the maintainer, which sheds reads that would wait for applies the full budget holds back.
#[derive(Default)]
pub(crate) struct CompactionStalls {
	count: Arc<StallCounter>,
	tablet_id: uuid::Uuid,
	/// Whether the last finished pass stalled, so a run of stalled passes warns once.
	stalled: Arc<std::sync::atomic::AtomicBool>,
	unflushed_limit: Option<UnflushedLimit>,
}

/// Reports whether one database's unflushed bytes reached SlateDB's `max_unflushed_bytes`.
/// SlateDB sets its unflushed gauge before every write's backpressure check, and again each
/// time a write waiting for a memtable upload checks again, so the reported value is true
/// exactly while writes wait for an upload and false once one goes through.
#[derive(Clone)]
struct UnflushedLimit {
	max_unflushed_bytes: i64,
	full: tokio::sync::watch::Sender<bool>,
}

/// The database's share of the process unflushed gauge, plus its unflushed limit report.
struct UnflushedGauge {
	share: ProcessShare,
	limit: Option<UnflushedLimit>,
}

impl slatedb_common::metrics::GaugeFn for UnflushedGauge {
	fn set(&self, value: i64) {
		self.share.set(value);
		let Some(limit) = &self.limit else {
			return;
		};
		let full = value >= limit.max_unflushed_bytes;
		limit.full.send_if_modified(|current| {
			let changed = *current != full;
			*current = full;
			changed
		});
	}
}

#[derive(Default)]
struct StallCounter(AtomicU64);

impl slatedb_common::metrics::CounterFn for StallCounter {
	fn increment(&self, value: u64) {
		self.0.fetch_add(value, Ordering::Relaxed);
		L0_STALLS.inc_by(value);
	}
}

/// One database's contribution to a process-wide gauge. Setting it moves the process gauge by
/// the change, and dropping it with its database removes what is left.
struct ProcessShare {
	gauge: &'static IntGauge,
	value: std::sync::atomic::AtomicI64,
}

impl ProcessShare {
	fn new(gauge: &'static IntGauge) -> Self {
		Self {
			gauge,
			value: std::sync::atomic::AtomicI64::new(0),
		}
	}

	fn add(&self, delta: i64) {
		self.value.fetch_add(delta, Ordering::Relaxed);
		self.gauge.add(delta);
		warn_near_capacity();
	}
}

impl slatedb_common::metrics::GaugeFn for ProcessShare {
	fn set(&self, value: i64) {
		let previous = self.value.swap(value, Ordering::Relaxed);
		self.gauge.add(value - previous);
		warn_near_capacity();
	}
}

impl slatedb_common::metrics::UpDownCounterFn for ProcessShare {
	fn increment(&self, value: i64) {
		self.add(value);
	}
}

impl Drop for ProcessShare {
	fn drop(&mut self) {
		self.gauge.sub(self.value.load(Ordering::Relaxed));
	}
}

/// Whether the unflushed total was at 90% or more of its budget at the last check, so the
/// warning fires once per upward crossing.
static UNFLUSHED_NEAR_CAPACITY: std::sync::atomic::AtomicBool =
	std::sync::atomic::AtomicBool::new(false);

fn warn_near_capacity() {
	let used = COLD_STORE_UNFLUSHED.get();
	let capacity = COLD_STORE_UNFLUSHED_CAPACITY.get();
	let near = capacity > 0 && used >= capacity / 10 * 9;
	if near && !UNFLUSHED_NEAR_CAPACITY.swap(true, Ordering::Relaxed) {
		tracing::warn!(
			used_bytes = used,
			capacity_bytes = capacity,
			"cold-store unflushed bytes near the process budget; uploads may be behind or storage.cold_store.unflushed_budget_bytes is too small"
		);
	} else if !near {
		UNFLUSHED_NEAR_CAPACITY.store(false, Ordering::Relaxed);
	}
}

/// Publishes the cold-store capacities for a worker that maintains `maintained` tablets.
pub(crate) fn publish_cold_store_capacity(
	config: &crate::tablet_maintainer_machine::executor::cold_store::ColdStoreConfig,
	maintained: usize,
) {
	let gauge = |value: usize| value.min(i64::MAX as usize) as i64;
	COLD_STORE_UNFLUSHED_CAPACITY.set(gauge(config.unflushed_budget_bytes));
	let per_database = config
		.for_maintained_tablets(maintained)
		.max_concurrent_compactions;
	COLD_STORE_COMPACTIONS_CAPACITY.set(gauge(maintained.saturating_mul(per_database)));
}

/// Forwards one SlateDB counter into a process-global collector shared by every tablet.
struct PrometheusCounter(IntCounter);

impl slatedb_common::metrics::CounterFn for PrometheusCounter {
	fn increment(&self, value: u64) {
		self.0.inc_by(value);
	}
}

/// SlateDB's labelled read-path counters, exported only for the label values SlateDB
/// defines so the exported series stay bounded.
fn slatedb_labelled_counter(name: &str, labels: &[(&str, &str)]) -> Option<IntCounter> {
	let label = |key: &str| {
		labels
			.iter()
			.find(|(label, _)| *label == key)
			.map(|(_, value)| *value)
	};
	if name == slatedb::db_cache_stats::ACCESS_COUNT {
		let entry_kind = label("entry_kind")?;
		let result = label("result")?;
		let known_kind = matches!(entry_kind, "filter" | "index" | "data_block" | "stats");
		let known_result = matches!(result, "hit" | "miss");
		if !known_kind || !known_result {
			return None;
		}
		return Some(COLD_STORE_CACHE_ACCESSES.with_label_values(&[entry_kind, result]));
	}
	let result = if name == slatedb::db_stats::SST_FILTER_POSITIVE_COUNT {
		"positive"
	} else if name == slatedb::db_stats::SST_FILTER_NEGATIVE_COUNT {
		"negative"
	} else if name == slatedb::db_stats::SST_FILTER_FALSE_POSITIVE_COUNT {
		"false_positive"
	} else {
		return None;
	};
	let kind = label(slatedb::db_stats::FILTER_KIND_LABEL)?;
	let known_kind = kind == slatedb::db_stats::FILTER_KIND_POINT
		|| kind == slatedb::db_stats::FILTER_KIND_PREFIX
		|| kind == slatedb::db_stats::FILTER_KIND_RANGE;
	if !known_kind {
		return None;
	}
	Some(COLD_STORE_FILTER_CHECKS.with_label_values(&[kind, result]))
}

/// SlateDB counters exported for write-amplification accounting and backpressure.
fn slatedb_counter(name: &str) -> Option<IntCounter> {
	let stat = if name == slatedb::db_stats::MEMTABLE_WRITE_BYTES {
		"memtable_write"
	} else if name == slatedb::db_stats::L0_FLUSH_BYTES {
		"l0_flush"
	} else if name == slatedb::compactor::stats::BYTES_COMPACTED {
		"compacted"
	} else if name == slatedb::db_stats::IMMUTABLE_MEMTABLE_FLUSHES {
		return Some(SLATEDB_FLUSHES.clone());
	} else if name == slatedb::db_stats::BACKPRESSURE_COUNT {
		return Some(COLD_STORE_BACKPRESSURE.with_label_values(&["wait"]));
	} else if name == slatedb::db_stats::BACKPRESSURE_TIMEOUT_COUNT {
		return Some(COLD_STORE_BACKPRESSURE.with_label_values(&["timeout"]));
	} else {
		return None;
	};
	Some(SLATEDB_BYTES.with_label_values(&[stat]))
}

impl slatedb_common::metrics::MetricsRecorder for CompactionStalls {
	fn register_counter(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::CounterFn> {
		if name == slatedb::db_stats::L0_STALL_COUNT {
			// Both the total-SST cap and per-key cap prevent the same cold pass
			// from progressing. Count the pass once regardless of stall reason.
			self.count.clone()
		} else if let Some(counter) = slatedb_counter(name) {
			Arc::new(PrometheusCounter(counter))
		} else if let Some(counter) = slatedb_labelled_counter(name, labels) {
			Arc::new(PrometheusCounter(counter))
		} else {
			slatedb_common::metrics::NoopMetricsRecorder.register_counter(name, description, labels)
		}
	}

	fn register_gauge(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::GaugeFn> {
		if name == slatedb::db_stats::TOTAL_MEM_SIZE_BYTES {
			Arc::new(UnflushedGauge {
				share: ProcessShare::new(&COLD_STORE_UNFLUSHED),
				limit: self.unflushed_limit.clone(),
			})
		} else {
			slatedb_common::metrics::NoopMetricsRecorder.register_gauge(name, description, labels)
		}
	}

	fn register_up_down_counter(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
	) -> Arc<dyn slatedb_common::metrics::UpDownCounterFn> {
		if name == slatedb::compactor::stats::RUNNING_COMPACTIONS {
			Arc::new(ProcessShare::new(&COLD_STORE_COMPACTIONS))
		} else {
			slatedb_common::metrics::NoopMetricsRecorder.register_up_down_counter(
				name,
				description,
				labels,
			)
		}
	}

	fn register_histogram(
		&self,
		name: &str,
		description: &str,
		labels: &[(&str, &str)],
		boundaries: &[f64],
	) -> Arc<dyn slatedb_common::metrics::HistogramFn> {
		slatedb_common::metrics::NoopMetricsRecorder.register_histogram(
			name,
			description,
			labels,
			boundaries,
		)
	}
}

impl CompactionStalls {
	

	/// The cold-store owner serializes passes, so a stall belongs to at most one
	/// active pass. Opening, reads and closing never create a pass observation.
	/// Counts callbacks observed during this attempt; the owner must retain an
	/// uncertain write until it completes and close the handle after failure.
	pub(crate) fn for_tablet(tablet_id: uuid::Uuid) -> Self {
		Self {
			tablet_id,
			..Self::default()
		}
	}

	/// Reports through the returned receiver whether the database's unflushed bytes are at
	/// `max_unflushed_bytes`, the limit its settings give SlateDB. The report starts false
	/// and ends, with the sender, when the database and this recorder are dropped.
	pub(crate) fn with_unflushed_limit(
		mut self,
		max_unflushed_bytes: usize,
	) -> (Self, tokio::sync::watch::Receiver<bool>) {
		let (full, receiver) = tokio::sync::watch::channel(false);
		self.unflushed_limit = Some(UnflushedLimit {
			max_unflushed_bytes: i64::try_from(max_unflushed_bytes).unwrap_or(i64::MAX),
			full,
		});
		(self, receiver)
	}

	pub(crate) fn begin_pass(&self) -> CompactionWait {
		CompactionWait {
			count: self.count.clone(),
			before: self.count.0.load(Ordering::Relaxed),
			tablet_id: self.tablet_id,
			stalled: self.stalled.clone(),
		}
	}
}

/// Finishes telemetry even on errors or cancellation; it does not resolve the
/// underlying write or release any storage ownership.
pub(crate) struct CompactionWait {
	count: Arc<StallCounter>,
	before: u64,
	tablet_id: uuid::Uuid,
	stalled: Arc<std::sync::atomic::AtomicBool>,
}

impl CompactionWait {
	pub(crate) fn waited(&self) -> bool {
		self.count.0.load(Ordering::Relaxed) != self.before
	}
}

impl Drop for CompactionWait {
	/// Warns on the first stalled pass after one that did not stall, so a sustained stall
	/// logs once and a recovery rearms the warning.
	fn drop(&mut self) {
		let waited = self.waited();
		if waited {
			COMPACTION_WAIT_PASSES.inc();
		}
		let was_stalled = self.stalled.swap(waited, Ordering::Relaxed);
		if waited && !was_stalled {
			tracing::warn!(
				tablet_id = %self.tablet_id,
				"cold-store pass waited for compaction because L0 was full; memtable flushes are outpacing compaction"
			);
		}
	}
}

/// One maintainer's contribution to the process-wide store gauges. Each observation
/// applies only the change since the last one, and Drop withdraws the remainder, so
/// gauges stay correct across many maintainers and abrupt task cancellation.
#[derive(Default)]
pub(crate) struct StoreGauges {
	last: crate::tablet_maintainer_machine::StoreSnapshot,
	stalls: AdmissionStallClock,
	/// When this maintainer last warned that flushes wait for a flush slot.
	flush_slot_warned_at: Option<std::time::Instant>,
}

/// Least time between two flush slot warnings of one maintainer. A busy tablet keeps its
/// slots full for long stretches, so a warning per transition would flood the log.
const FLUSH_SLOT_WARNING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

impl StoreGauges {
	/// Applies one published snapshot. `now` is a telemetry clock reading.
	pub(crate) fn observe(
		&mut self,
		next: crate::tablet_maintainer_machine::StoreSnapshot,
		now: std::time::Instant,
	) {
		let previous = std::mem::replace(&mut self.last, next);
		apply_store_delta(&previous, &next);
		if let Some((reason, elapsed)) = self.stalls.publish(now, next.head_block) {
			TABLET_STORE_ADMISSION_BLOCKED
				.with_label_values(&[reason.label()])
				.inc_by(elapsed.as_secs_f64());
		}
		let at_cap = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
			snapshot.keyed_capacity > 0 && snapshot.keyed_in_flight >= snapshot.keyed_capacity
		};
		let initial_full = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
			snapshot.initial_bytes_capacity > 0
				&& snapshot.initial_bytes_held >= snapshot.initial_bytes_capacity / 10 * 9
		};
		if initial_full(&next) && !initial_full(&previous) {
			tracing::warn!(
				initial_bytes_held = next.initial_bytes_held,
				initial_bytes_capacity = next.initial_bytes_capacity,
				"unacknowledged creations hold most of their initial write budget; new creates fall back to a claim entry first, raise max_pending_initial_bytes if creates stall"
			);
		}
		let loaded_near_cap = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
			snapshot.loaded_volume_capacity > 0
				&& snapshot.loaded_volumes >= snapshot.loaded_volume_capacity / 10 * 9
		};
		if loaded_near_cap(&next) && !loaded_near_cap(&previous) {
			tracing::warn!(
				loaded_volumes = next.loaded_volumes,
				loaded_volume_capacity = next.loaded_volume_capacity,
				"tablet loaded volumes near capacity; raise depot.maintainer_max_loaded_volumes if opens start failing"
			);
		}
		warn_near_read_caps(&previous, &next);
		if at_cap(&next) && !at_cap(&previous) {
			tracing::warn!(
				keyed_in_flight = next.keyed_in_flight,
				keyed_capacity = next.keyed_capacity,
				"tablet store keyed work reached its cap; raise depot.store_max_keyed_in_flight if applies queue"
			);
		}
		let flush_waits = next.head_block
			== Some(crate::tablet_maintainer_machine::state::store::AdmissionBlock::FlushSlots);
		let warn_due = self
			.flush_slot_warned_at
			.is_none_or(|at| now.saturating_duration_since(at) >= FLUSH_SLOT_WARNING_INTERVAL);
		if flush_waits && warn_due {
			self.flush_slot_warned_at = Some(now);
			tracing::warn!(
				flush_staging_in_flight = next.flush_staging_in_flight,
				flush_uploading_in_flight = next.flush_uploading_in_flight,
				flush_staging_capacity = next.flush_staging_capacity,
				flush_upload_capacity = next.flush_upload_capacity,
				"tablet flushes wait for a flush slot; raise depot.store_max_flush_uploads or depot.store_max_flush_staging if flushes back up"
			);
		}
		if next.scan_limited && !previous.scan_limited {
			tracing::warn!(
				queued_work = next.queued_work,
				admission_scan_limit = next.admission_scan_limit,
				"tablet store queue is longer than one admission pass looks; raise depot.store_admission_scan_limit if work behind blocked items waits"
			);
		}
	}
}

/// Warns once when a cold-read window of one tablet rises to 90% of its cap: the client
/// window and each background window. Under sustained pressure it warns again only after
/// the window drops below that mark.
fn warn_near_read_caps(
	previous: &crate::tablet_maintainer_machine::StoreSnapshot,
	next: &crate::tablet_maintainer_machine::StoreSnapshot,
) {
	let near =
		|used: u32, capacity: u32| capacity > 0 && u64::from(used) * 10 >= u64::from(capacity) * 9;
	let client = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
		snapshot.ordered_reads_in_flight + snapshot.session_reads_in_flight
	};
	if near(client(next), next.read_capacity) && !near(client(previous), previous.read_capacity) {
		tracing::warn!(
			reads_in_flight = client(next),
			read_capacity = next.read_capacity,
			queued_session_reads = next.queued_session_reads,
			"tablet client cold reads near their window; cold wakes and scans queue behind it, raise depot.max_cold_reads if reads wait"
		);
	}
	let background = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
		[
			("held_image_read", snapshot.held_image_reads_in_flight),
			("replica_log_read", snapshot.replica_log_reads_in_flight),
			("fork_job", snapshot.fork_jobs),
			("fork_cleanup_job", snapshot.fork_cleanup_jobs),
		]
	};
	for ((window, before), (_, after)) in background(previous).into_iter().zip(background(next)) {
		if near(after, next.background_read_capacity)
			&& !near(before, previous.background_read_capacity)
		{
			tracing::warn!(
				window,
				in_flight = after,
				capacity = next.background_read_capacity,
				"tablet background cold-read window near its cap; forks are refused and applies and cleanups wait, raise depot.max_background_cold_reads if they back up"
			);
		}
	}
}

impl Drop for StoreGauges {
	fn drop(&mut self) {
		apply_store_delta(&self.last, &Default::default());
	}
}

/// Integrates admission stall time from published snapshots. The core emits no
/// metrics, so the time between two publishes is credited to the reason the earlier
/// snapshot reported.
#[derive(Debug, Default)]
pub(crate) struct AdmissionStallClock {
	last: Option<(
		std::time::Instant,
		Option<crate::tablet_maintainer_machine::state::store::AdmissionBlock>,
	)>,
}

impl AdmissionStallClock {
	/// Records a publish at `now` and returns the time the previous snapshot's reason
	/// held, if it named one.
	pub(crate) fn publish(
		&mut self,
		now: std::time::Instant,
		next: Option<crate::tablet_maintainer_machine::state::store::AdmissionBlock>,
	) -> Option<(
		crate::tablet_maintainer_machine::state::store::AdmissionBlock,
		std::time::Duration,
	)> {
		let previous = self.last.replace((now, next));
		let (at, reason) = previous?;
		Some((reason?, now.saturating_duration_since(at)))
	}
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_INITIAL_BYTES: IntGauge = register_int_gauge_with_registry!("depot_tablet_initial_write_bytes", "Payload bytes of initial writes that unacknowledged volume creations hold for their retries, across this process's maintainers.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_LOADED_VOLUMES: IntGauge = register_int_gauge_with_registry!("depot_tablet_loaded_volumes", "Volumes loaded across this process's tablet maintainers.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_LOADED_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_tablet_loaded_volume_capacity", "Configured loaded volume bound, summed across this process's tablet maintainers.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_INITIAL_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_tablet_initial_write_capacity_bytes", "Configured bound on initial write bytes held by unacknowledged creations, summed across this process's maintainers.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_INITIAL_WITHHELD: IntCounter = register_int_counter_with_registry!("depot_tablet_initial_writes_withheld_total", "Creating opens sent without their initial writes, because the writes exceeded a budget or the creation had granted another open. The opener writes them after its claim entry.", *rivet_metrics::REGISTRY).unwrap();
}

fn apply_store_delta(
	previous: &crate::tablet_maintainer_machine::StoreSnapshot,
	next: &crate::tablet_maintainer_machine::StoreSnapshot,
) {
	let delta = |before: u32, after: u32| i64::from(after) - i64::from(before);
	TABLET_STORE_QUEUE_DEPTH
		.with_label_values(&["store"])
		.add(delta(previous.queued_work, next.queued_work));
	let kinds = |sample: &crate::tablet_maintainer_machine::state::store::QueueSample| {
		[
			("sampled_open", sample.opens),
			("sampled_load", sample.loads),
			("sampled_apply", sample.applies),
			("sampled_flush", sample.flushes),
			("sampled_row_read", sample.row_reads),
			("sampled_pass", sample.passes),
			("sampled_other", sample.other),
		]
	};
	for ((kind, before), (_, after)) in kinds(&previous.queued_by_kind)
		.into_iter()
		.zip(kinds(&next.queued_by_kind))
	{
		TABLET_STORE_QUEUE_DEPTH
			.with_label_values(&[kind])
			.add(delta(before, after));
	}
	TABLET_STORE_QUEUE_DEPTH
		.with_label_values(&["session_read"])
		.add(delta(
			previous.queued_session_reads,
			next.queued_session_reads,
		));
	let in_flight = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
		[
			("exclusive", snapshot.exclusive_in_flight),
			("keyed", snapshot.keyed_in_flight),
			("pass", snapshot.passes_in_flight),
			("flush_staging", snapshot.flush_staging_in_flight),
			("flush_uploading", snapshot.flush_uploading_in_flight),
			("ordered_read", snapshot.ordered_reads_in_flight),
			("session_read", snapshot.session_reads_in_flight),
			("held_image_read", snapshot.held_image_reads_in_flight),
			("replica_log_read", snapshot.replica_log_reads_in_flight),
			("fork_job", snapshot.fork_jobs),
			("fork_cleanup_job", snapshot.fork_cleanup_jobs),
		]
	};
	for ((kind, before), (_, after)) in in_flight(previous).into_iter().zip(in_flight(next)) {
		TABLET_STORE_IN_FLIGHT
			.with_label_values(&[kind])
			.add(delta(before, after));
	}
	TABLET_STORE_KEYED_CAPACITY.add(delta(previous.keyed_capacity, next.keyed_capacity));
	let capacities = |snapshot: &crate::tablet_maintainer_machine::StoreSnapshot| {
		[
			("flush_staging", snapshot.flush_staging_capacity),
			("flush_uploading", snapshot.flush_upload_capacity),
			("admission_scan", snapshot.admission_scan_limit),
			("cold_reads_per_volume", snapshot.per_volume_read_capacity),
			("background_read", snapshot.background_read_capacity),
		]
	};
	for ((kind, before), (_, after)) in capacities(previous).into_iter().zip(capacities(next)) {
		TABLET_STORE_CAPACITY
			.with_label_values(&[kind])
			.add(delta(before, after));
	}
	TABLET_STORE_SCAN_LIMITED.add(delta(
		u32::from(previous.scan_limited),
		u32::from(next.scan_limited),
	));
	TABLET_STORE_READ_CAPACITY.add(delta(previous.read_capacity, next.read_capacity));
	let delta_u64 = |before: u64, after: u64| after as i64 - before as i64;
	TABLET_INITIAL_BYTES.add(delta_u64(
		previous.initial_bytes_held,
		next.initial_bytes_held,
	));
	TABLET_LOADED_VOLUMES.add(delta(previous.loaded_volumes, next.loaded_volumes));
	TABLET_LOADED_CAPACITY.add(delta(
		previous.loaded_volume_capacity,
		next.loaded_volume_capacity,
	));
	TABLET_INITIAL_CAPACITY.add(delta_u64(
		previous.initial_bytes_capacity,
		next.initial_bytes_capacity,
	));
	TABLET_INITIAL_WITHHELD.inc_by(
		next.initial_writes_withheld
			.saturating_sub(previous.initial_writes_withheld),
	);
}

/// Counts one request rejected because a tablet maintainer's inbox was full. The warning fires
/// at power-of-two counts so sustained pressure does not flood the log.
pub(crate) fn record_maintainer_inbox_full(request: &'static str) {
	let counter = TABLET_MAINTAINER_INBOX_FULL.with_label_values(&[request]);
	counter.inc();
	let count = counter.get();
	if count.is_power_of_two() {
		tracing::warn!(
			request,
			count,
			capacity = TABLET_MAINTAINER_INBOX_CAPACITY.get(),
			"tablet maintainer inbox is full; raise depot.maintainer_inbox_capacity or add tablets"
		);
	}
}

/// Counts of tablets in `manifest` that `worker` maintains and holds a replica of.
pub(crate) fn placement_counts(
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	worker: &[u8; 16],
) -> (usize, usize) {
	let maintained = manifest
		.tablets
		.iter()
		.filter(|tablet| tablet.maintainer.as_ref() == Some(worker))
		.count();
	let replica = manifest
		.tablets
		.iter()
		.filter(|tablet| tablet.replica_set.members.contains(worker))
		.count();
	(maintained, replica)
}

/// Publishes `depot_tablets` for this worker's latest manifest.
pub(crate) fn publish_placement(
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	worker: &[u8; 16],
) {
	let (maintained, replica) = placement_counts(manifest, worker);
	let gauge = |value: usize| value.min(i64::MAX as usize) as i64;
	TABLETS
		.with_label_values(&["maintained"])
		.set(gauge(maintained));
	TABLETS.with_label_values(&["replica"]).set(gauge(replica));
}

/// Length of one busy-time window for the busiest-tablet gauges.
pub(crate) const TABLET_LOAD_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

/// One tablet maintainer's serial-resource time and work over one window.
#[derive(Clone, Copy, Debug, Default)]
struct LoadBucket {
	index: u64,
	/// Memtable flush time while level 0 had room, plus retention: real upload work.
	upload_busy: std::time::Duration,
	/// Memtable flush time that started with level 0 at its SST limit, which is waiting
	/// for compaction rather than uploading.
	l0_stall: std::time::Duration,
	runner_busy: std::time::Duration,
	store_busy: std::time::Duration,
	applies: u64,
	cold_get_time: std::time::Duration,
	cold_gets: u64,
}

/// One tablet's load over its last complete window, plus current waiting ages.
/// Ratios are capped at 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TabletLoadSample {
	pub(crate) upload_ratio: f64,
	pub(crate) l0_stall_ratio: f64,
	pub(crate) runner_ratio: f64,
	pub(crate) store_ratio: f64,
	pub(crate) apply_rate: f64,
	/// Mean latency of the object-store GETs the tablet's store operations issued, in seconds.
	pub(crate) cold_get_seconds: f64,
	pub(crate) oldest_waiting_open_seconds: f64,
	pub(crate) apply_lag_seconds: f64,
}

/// Signal order shared by the worst values and their tablet identities.
const TABLET_LOAD_SIGNALS: [&str; 8] = [
	"upload",
	"l0_stall",
	"runner",
	"store",
	"apply_rate",
	"cold_get",
	"oldest_waiting_open",
	"apply_lag",
];

impl TabletLoadSample {
	fn values(self) -> [f64; 8] {
		[
			self.upload_ratio,
			self.l0_stall_ratio,
			self.runner_ratio,
			self.store_ratio,
			self.apply_rate,
			self.cold_get_seconds,
			self.oldest_waiting_open_seconds,
			self.apply_lag_seconds,
		]
	}
}

/// Outstanding work is tracked only until its forwarding or apply task ends. A scrape
/// computes ages from real telemetry time even if the runner or store stops progressing.
#[derive(Default)]
struct TabletWaits {
	next_id: u64,
	opens: BTreeSet<(std::time::Duration, u64)>,
	applies: BTreeMap<u64, (std::time::Duration, std::time::Duration)>,
}

#[derive(Clone, Copy)]
enum TabletWaitKind {
	Open,
	Apply,
}

/// Removes telemetry on completion, cancellation or panic, without driving application work.
pub(crate) struct TabletWait {
	load: Arc<TabletLoad>,
	id: u64,
	started: std::time::Duration,
	kind: TabletWaitKind,
}

impl Drop for TabletWait {
	fn drop(&mut self) {
		let mut waits = self.load.waits.lock();
		match self.kind {
			TabletWaitKind::Open => {
				waits.opens.remove(&(self.started, self.id));
			}
			TabletWaitKind::Apply => {
				waits.applies.remove(&self.id);
			}
		}
	}
}

/// Diagnostic thresholds, evaluated once per scrape. Busy signals use the last complete
/// 60 s window. A stuck open waits at least 5 s with an idle runner (at most 10% busy);
/// lagging means an outstanding apply's oldest known entry is at least 5 s old.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct TabletHealth {
	saturated: bool,
	stuck: bool,
	lagging: bool,
}

impl TabletHealth {
	fn from_sample(sample: TabletLoadSample) -> Self {
		Self {
			saturated: sample
				.runner_ratio
				.max(sample.store_ratio)
				.max(sample.upload_ratio)
				.max(sample.l0_stall_ratio)
				>= 0.85,
			stuck: sample.oldest_waiting_open_seconds >= 5.0 && sample.runner_ratio <= 0.1,
			lagging: sample.apply_lag_seconds >= 5.0,
		}
	}
}

/// One local maintainer's load on its serial resources: the maintainer runner, the store
/// operations and the uploading flush. Recording attributes a whole operation to the
/// window it finished in.
#[derive(Default)]
pub(crate) struct TabletLoad {
	buckets: Mutex<(LoadBucket, LoadBucket)>,
	waits: Mutex<TabletWaits>,
	health: Mutex<TabletHealth>,
	/// Empty for loads outside a maintainer, which record no labelled series.
	tablet_label: String,
}

impl TabletLoad {
	/// Counts one finished volume open of this tablet by its reply label.
	pub(crate) fn record_open_completed(&self, result: &'static str) {
		if self.tablet_label.is_empty() {
			return;
		}
		TABLET_VOLUME_OPENS_COMPLETED
			.with_label_values(&[result])
			.inc();
	}

	pub(crate) fn begin_open(self: &Arc<Self>) -> TabletWait {
		self.begin_wait_at(
			TabletWaitKind::Open,
			telemetry_elapsed(),
			std::time::Duration::ZERO,
		)
	}

	/// The supplied lag uses the writer's wall clock, so skew shifts it. With no inline
	/// entries, report time waiting for the apply, including its replica-log read.
	pub(crate) fn begin_apply(self: &Arc<Self>, lag: std::time::Duration) -> TabletWait {
		self.begin_wait_at(TabletWaitKind::Apply, telemetry_elapsed(), lag)
	}

	fn begin_wait_at(
		self: &Arc<Self>,
		kind: TabletWaitKind,
		started: std::time::Duration,
		lag: std::time::Duration,
	) -> TabletWait {
		let mut waits = self.waits.lock();
		let id = waits.next_id;
		waits.next_id += 1;
		match kind {
			TabletWaitKind::Open => {
				waits.opens.insert((started, id));
			}
			TabletWaitKind::Apply => {
				waits.applies.insert(id, (started, lag));
			}
		}
		TabletWait {
			load: self.clone(),
			id,
			started,
			kind,
		}
	}

	fn observe_health(&self, sample: TabletLoadSample) {
		let next = TabletHealth::from_sample(sample);
		let previous = std::mem::replace(&mut *self.health.lock(), next);
		for (condition, was, active) in [
			("saturated", previous.saturated, next.saturated),
			("stuck", previous.stuck, next.stuck),
			("lagging", previous.lagging, next.lagging),
		] {
			if was != active {
				tracing::warn!(tablet_id = %self.tablet_label, condition, active,
					runner_busy_ratio = sample.runner_ratio, store_busy_ratio = sample.store_ratio,
					upload_busy_ratio = sample.upload_ratio, l0_stall_ratio = sample.l0_stall_ratio,
					oldest_waiting_open_seconds = sample.oldest_waiting_open_seconds,
					apply_lag_seconds = sample.apply_lag_seconds,
					"tablet load threshold crossed");
			}
		}
	}

	fn bucket(
		&self,
		elapsed: std::time::Duration,
	) -> parking_lot::MutexGuard<'_, (LoadBucket, LoadBucket)> {
		let index = (elapsed.as_millis() / TABLET_LOAD_WINDOW.as_millis()) as u64;
		let mut buckets = self.buckets.lock();
		let (previous, current) = &mut *buckets;
		if index != current.index {
			*previous = if index == current.index + 1 {
				*current
			} else {
				LoadBucket::default()
			};
			*current = LoadBucket {
				index,
				..Default::default()
			};
		}
		buckets
	}

	/// Records one flush's memtable time, as level 0 stall time when level 0 was full at
	/// its start, and its retention time as upload work.
	pub(crate) fn record_flush_at(&self, elapsed: std::time::Duration, flush: &FlushTimings) {
		let mut bucket = self.bucket(elapsed);
		if flush.l0_full {
			bucket.1.l0_stall += flush.memtable;
		} else {
			bucket.1.upload_busy += flush.memtable;
		}
		bucket.1.upload_busy += flush.retention;
	}

	pub(crate) fn record_store_at(&self, elapsed: std::time::Duration, busy: std::time::Duration) {
		self.bucket(elapsed).1.store_busy += busy;
	}

	pub(crate) fn record_runner_at(&self, elapsed: std::time::Duration, busy: std::time::Duration) {
		self.bucket(elapsed).1.runner_busy += busy;
	}

	pub(crate) fn record_apply_at(&self, elapsed: std::time::Duration) {
		self.bucket(elapsed).1.applies += 1;
	}

	pub(crate) fn record_cold_get_at(
		&self,
		elapsed: std::time::Duration,
		latency: std::time::Duration,
	) {
		let mut bucket = self.bucket(elapsed);
		bucket.1.cold_get_time += latency;
		bucket.1.cold_gets += 1;
	}

	/// Completed-window load and live ages of outstanding opens and applies.
	pub(crate) fn load_at(&self, elapsed: std::time::Duration) -> TabletLoadSample {
		let complete = self.bucket(elapsed).0;
		let waits = self.waits.lock();
		let oldest_waiting_open_seconds = waits.opens.first().map_or(0.0, |(started, _)| {
			elapsed.saturating_sub(*started).as_secs_f64()
		});
		let apply_lag_seconds = waits
			.applies
			.values()
			.map(|(started, lag)| {
				lag.saturating_add(elapsed.saturating_sub(*started))
					.as_secs_f64()
			})
			.fold(0.0, f64::max);
		let window = TABLET_LOAD_WINDOW.as_secs_f64();
		let ratio = |busy: std::time::Duration| (busy.as_secs_f64() / window).min(1.0);
		let cold_get_seconds = if complete.cold_gets == 0 {
			0.0
		} else {
			complete.cold_get_time.as_secs_f64() / complete.cold_gets as f64
		};
		TabletLoadSample {
			upload_ratio: ratio(complete.upload_busy),
			l0_stall_ratio: ratio(complete.l0_stall),
			runner_ratio: ratio(complete.runner_busy),
			store_ratio: ratio(complete.store_busy),
			apply_rate: complete.applies as f64 / window,
			cold_get_seconds,
			oldest_waiting_open_seconds,
			apply_lag_seconds,
		}
	}

	pub(crate) fn record_flush(&self, flush: &FlushTimings) {
		self.record_flush_at(telemetry_elapsed(), flush);
	}

	pub(crate) fn record_store(&self, busy: std::time::Duration) {
		self.record_store_at(telemetry_elapsed(), busy);
	}

	pub(crate) fn record_apply(&self) {
		self.record_apply_at(telemetry_elapsed());
	}
}

impl rivet_state_machine_ephemeral::BusyObserver for TabletLoad {
	fn record_busy(&self, busy: std::time::Duration) {
		self.record_runner_at(telemetry_elapsed(), busy);
		if !self.tablet_label.is_empty() {
			TABLET_RUNNER_BUSY_SECONDS
				.with_label_values(&[self.tablet_label.as_str()])
				.inc_by(busy.as_secs_f64());
		}
	}
}

/// Where one tablet flush's `finish_flush` spent its time.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FlushTimings {
	/// Level 0 was at its SST limit when the memtable flush started, so SlateDB held the
	/// upload until a compaction made room.
	pub(crate) l0_full: bool,
	pub(crate) memtable: std::time::Duration,
	pub(crate) retention: std::time::Duration,
}

tokio::task_local! {
	/// The tablet whose store operation is running on this task, so object-store GETs it
	/// issues inline are charged to it.
	static CURRENT_TABLET: Arc<TabletLoad>;
}

/// Runs one store operation with its object-store GETs charged to `load`. Telemetry only.
pub(crate) async fn with_tablet_load<F: std::future::Future>(
	load: Arc<TabletLoad>,
	future: F,
) -> F::Output {
	CURRENT_TABLET.scope(load, future).await
}

/// Charges one GET's latency to the tablet whose store operation issued it, if any. GETs
/// from SlateDB's own tasks, such as compaction, run outside any tablet scope.
pub(crate) fn record_tablet_cold_get(latency: std::time::Duration) {
	let _ = CURRENT_TABLET.try_with(|load| load.record_cold_get_at(telemetry_elapsed(), latency));
}

/// Process-relative real time for load windows. Telemetry only; it never drives behavior.
#[allow(clippy::disallowed_methods, reason = "real-time telemetry only")]
pub(crate) fn telemetry_elapsed() -> std::time::Duration {
	lazy_static::lazy_static! {
		static ref STARTED: std::time::Instant = std::time::Instant::now();
	}
	std::time::Instant::now().saturating_duration_since(*STARTED)
}

lazy_static::lazy_static! {
	/// Live local maintainers' load, read by the busiest-tablet collector at scrape time.
	static ref TABLET_LOADS: scc::HashMap<u64, Arc<TabletLoad>> = scc::HashMap::new();
	static ref BUSIEST_TABLET_COLLECTOR: () = {
		rivet_metrics::REGISTRY
			.register(Box::new(BusiestTabletCollector::new()))
			.expect("busiest tablet collector registers once");
	};
	pub(crate) static ref KV_HOST_LOCK_WAIT: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_kv_host_lock_wait_seconds",
			"Time a KV command volume host operation waited for the host's volume cache lock; zero when uncontended."
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	/// Runner time per tablet, so a 5 s rate tells a saturated maintainer (near 1) from a
	/// stuck one (near 0 while opens wait).
	pub(crate) static ref TABLET_RUNNER_BUSY_SECONDS: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_runner_busy_seconds_total",
			"Wall time one local tablet maintainer runner spent on transitions, snapshots and effect dispatch, by tablet. Its rate is the runner's busy share."
		),
		&["tablet_id"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	pub(crate) static ref TABLET_VOLUME_OPENS_COMPLETED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_volume_opens_completed_total",
			"Volume opens a local tablet maintainer answered after taking them into its inbox, by reply: ok, a reject reason, unexpected, or cancelled when the maintainer stopped first. Opens refused at submit are not counted."
		),
		&["result"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	pub(crate) static ref TABLET_STORE_QUEUE_WAIT: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_tablet_store_queue_wait_seconds",
			"Time a tablet store effect waited in the store loop's queue before the loop picked it up."
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	)
	.unwrap();
}

/// Registration of one maintainer's load; dropping it removes the maintainer from the gauges.
pub(crate) struct TabletLoadHandle {
	id: u64,
	pub(crate) load: Arc<TabletLoad>,
}

impl TabletLoadHandle {
	pub(crate) fn register(tablet_id: [u8; 16]) -> Self {
		lazy_static::initialize(&BUSIEST_TABLET_COLLECTOR);
		let id = NEXT_OBSERVER.fetch_add(1, Ordering::Relaxed);
		let load = Arc::new(TabletLoad {
			tablet_label: uuid::Uuid::from_bytes(tablet_id).to_string(),
			..Default::default()
		});
		// Registration runs in sync maintainer setup and the scrape-time collector, and no
		// async code waits on this map, so the sync methods cannot park a waiter.
		let _ = TABLET_LOADS.insert_sync(id, load.clone());
		Self { id, load }
	}
}

impl Drop for TabletLoadHandle {
	fn drop(&mut self) {
		TABLET_LOADS.remove_sync(&self.id);
		// A later grant of the same tablet on this worker may already run beside this one
		// while it stops; its series stay.
		let label = self.load.tablet_label.as_str();
		let mut shared = false;
		TABLET_LOADS.iter_sync(|_, load| {
			shared |= load.tablet_label == label;
			!shared
		});
		if shared {
			return;
		}
		let _ = TABLET_RUNNER_BUSY_SECONDS.remove_label_values(&[label]);
		self.load.observe_health(TabletLoadSample::default());
	}
}

/// Every signal's highest value and its tablet, taken independently. Equal values choose
/// the lowest tablet ID so ties do not churn the info series between scrapes.
#[derive(Default)]
struct BusiestTabletLoad {
	sample: TabletLoadSample,
	tablets: [Option<String>; 8],
}

impl BusiestTabletLoad {
	fn observe(&mut self, load: &TabletLoad, sample: TabletLoadSample) {
		let mut values = self.sample.values();
		for (index, value) in sample.values().into_iter().enumerate() {
			if value > values[index]
				|| (value == values[index]
					&& self.tablets[index]
						.as_ref()
						.is_none_or(|id| load.tablet_label < *id))
			{
				values[index] = value;
				self.tablets[index] = Some(load.tablet_label.clone());
			}
		}
		self.sample = TabletLoadSample {
			upload_ratio: values[0],
			l0_stall_ratio: values[1],
			runner_ratio: values[2],
			store_ratio: values[3],
			apply_rate: values[4],
			cold_get_seconds: values[5],
			oldest_waiting_open_seconds: values[6],
			apply_lag_seconds: values[7],
		};
	}
}

fn busiest_tablet_load_at(
	elapsed: std::time::Duration,
	loads: impl IntoIterator<Item = Arc<TabletLoad>>,
) -> BusiestTabletLoad {
	let mut busiest = BusiestTabletLoad::default();
	for load in loads {
		let sample = load.load_at(elapsed);
		load.observe_health(sample);
		busiest.observe(&load, sample);
	}
	busiest
}

/// Constructed afresh for each scrape so an old winner never leaves a retained label set.
fn worst_tablet_info() -> IntGaugeVec {
	IntGaugeVec::new(
		Opts::new(
			"depot_busiest_tablet_info",
			"Worst local tablet per signal; absent when no local maintainer is registered.",
		),
		&["signal", "tablet_id"],
	)
	.expect("valid gauge")
}

/// Computes the busiest-tablet gauges at scrape time, so no timer loop rescans maintainers.
/// Value gauges report local maxima; the info family names only each signal's winner.
struct BusiestTabletCollector {
	scrape: Mutex<()>,
	info: IntGaugeVec,
	oldest_waiting_open_seconds: Gauge,
	apply_lag_seconds: Gauge,
	upload_busy_ratio: GaugeVec,
	runner_busy_ratio: Gauge,
	store_busy_ratio: Gauge,
	apply_rate: Gauge,
	cold_get_seconds: Gauge,
}

impl BusiestTabletCollector {
	fn new() -> Self {
		Self {
			scrape: Mutex::new(()),
			info: worst_tablet_info(),
			oldest_waiting_open_seconds: Gauge::new(
				"depot_busiest_tablet_oldest_waiting_open_seconds",
				"Age of the oldest admitted open still waiting for its reply on any local tablet.",
			)
			.expect("valid gauge"),
			apply_lag_seconds: Gauge::new(
				"depot_busiest_tablet_apply_lag_seconds",
				"Largest age of an outstanding apply's oldest inline entry on any local tablet, or its wait age when entries must be read. Writer clock skew shifts entry ages.",
			)
			.expect("valid gauge"),
			upload_busy_ratio: GaugeVec::new(
				Opts::new(
					"depot_busiest_tablet_upload_busy_ratio",
					"Highest share of the last 60 s window any local tablet's flushes spent uploading (upload) or held by a full level 0 (l0_stall).",
				),
				&["phase"],
			)
			.expect("valid gauge"),
			runner_busy_ratio: Gauge::new(
				"depot_busiest_tablet_runner_busy_ratio",
				"Highest share of the last 60 s window any local tablet maintainer runner spent on transitions and effect dispatch.",
			)
			.expect("valid gauge"),
			store_busy_ratio: Gauge::new(
				"depot_busiest_tablet_store_busy_ratio",
				"Highest share of the last 60 s window any local tablet spent in store operations other than flush uploads.",
			)
			.expect("valid gauge"),
			apply_rate: Gauge::new(
				"depot_busiest_tablet_apply_rate",
				"Highest filesystem apply rate per second of any local tablet maintainer over the last 60 s window.",
			)
			.expect("valid gauge"),
			cold_get_seconds: Gauge::new(
				"depot_busiest_tablet_cold_get_seconds",
				"Highest mean object-store GET latency, in seconds, of the GETs any local tablet's store operations issued over the last 60 s window.",
			)
			.expect("valid gauge"),
		}
	}
}

impl core::Collector for BusiestTabletCollector {
	fn desc(&self) -> Vec<&core::Desc> {
		let mut descs = self.upload_busy_ratio.desc();
		descs.extend(self.info.desc());
		descs.extend(self.oldest_waiting_open_seconds.desc());
		descs.extend(self.apply_lag_seconds.desc());
		descs.extend(self.runner_busy_ratio.desc());
		descs.extend(self.store_busy_ratio.desc());
		descs.extend(self.apply_rate.desc());
		descs.extend(self.cold_get_seconds.desc());
		descs
	}

	fn collect(&self) -> Vec<proto::MetricFamily> {
		let mut loads = Vec::new();
		TABLET_LOADS.iter_sync(|_, load| {
			loads.push(load.clone());
			true
		});
		self.collect_at(telemetry_elapsed(), loads)
	}
}

impl BusiestTabletCollector {
	fn collect_at(
		&self,
		elapsed: std::time::Duration,
		loads: impl IntoIterator<Item = Arc<TabletLoad>>,
	) -> Vec<proto::MetricFamily> {
		let _scrape = self.scrape.lock();
		let busiest = busiest_tablet_load_at(elapsed, loads);
		let sample = busiest.sample;
		let info = worst_tablet_info();
		for (signal, tablet) in TABLET_LOAD_SIGNALS.iter().zip(&busiest.tablets) {
			if let Some(tablet) = tablet {
				info.with_label_values(&[*signal, tablet.as_str()]).set(1);
			}
		}
		self.oldest_waiting_open_seconds
			.set(sample.oldest_waiting_open_seconds);
		self.apply_lag_seconds.set(sample.apply_lag_seconds);
		self.upload_busy_ratio
			.with_label_values(&["upload"])
			.set(sample.upload_ratio);
		self.upload_busy_ratio
			.with_label_values(&["l0_stall"])
			.set(sample.l0_stall_ratio);
		self.runner_busy_ratio.set(sample.runner_ratio);
		self.store_busy_ratio.set(sample.store_ratio);
		self.apply_rate.set(sample.apply_rate);
		self.cold_get_seconds.set(sample.cold_get_seconds);
		let mut families = self.upload_busy_ratio.collect();
		families.extend(info.collect());
		families.extend(self.oldest_waiting_open_seconds.collect());
		families.extend(self.apply_lag_seconds.collect());
		families.extend(self.runner_busy_ratio.collect());
		families.extend(self.store_busy_ratio.collect());
		families.extend(self.apply_rate.collect());
		families.extend(self.cold_get_seconds.collect());
		families
	}
}





/// Counts one tablet maintainer recovery scan. Both labels are code-defined names.
pub(crate) fn record_recovery_scan(scan: &'static str, trigger: &'static str) {
	MAINTAINER_RECOVERY_SCANS
		.with_label_values(&[scan, trigger])
		.inc();
}

/// Counts one tablet service loop wake. `reason` is a code-defined name.
pub(crate) fn record_tablet_loop_wake(reason: &'static str) {
	TABLET_LOOP_WAKES.with_label_values(&[reason]).inc();
}

/// Counts one replica status observation. `source` is a code-defined name.
pub(crate) fn record_status_observation(source: &'static str) {
	STATUS_OBSERVATIONS.with_label_values(&[source]).inc();
}

lazy_static::lazy_static! {
	static ref NOTICE_APPLIES: IntCounter = register_int_counter_with_registry!(
		"depot_maintainer_notice_applies_total",
		"Applies started from committed entries a status notice carried, without reading the replica.",
		*rivet_metrics::REGISTRY
	)
	.unwrap();
}

/// Counts one apply started from entries a status notice carried.
pub(crate) fn record_notice_apply() {
	NOTICE_APPLIES.inc();
}

/// Counts loaded volumes visited by a periodic or idle pass, through the depot client's shared
/// counter.
pub(crate) fn record_idle_visits(component: &'static str, visits: usize) {
	depot_client::metrics::record_idle_visits(component, visits);
}

/// Counts one due retention check. `result` is a code-defined name.
pub(crate) fn record_retention_check(result: &'static str) {
	RETENTION_CHECKS.with_label_values(&[result]).inc();
}

/// Counts one step of a missing member's reload. `result` is a code-defined name.
pub(crate) fn record_member_repair(result: &'static str) {
	MEMBER_REPAIRS.with_label_values(&[result]).inc();
}

// Object-store HTTP attempts, retries and multipart uploads, in their own block to stay
// under the macro recursion limit of one `lazy_static!` invocation.
lazy_static::lazy_static! {
	pub(crate) static ref OBJECT_STORE_HTTP_ATTEMPTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_http_attempts_total",
			"Object-store HTTP attempts, including retries, multipart parts, copies, every list page and every batch delete, which is what a store bills. By billed method (get, head, list, put, post, delete or other), the object source of the request that issued it, and response status: 2xx, 304, 404, 409, 412, 429, 500, 503, other 4xx and 5xx, transport_error, abandoned when the caller dropped the attempt before its response headers, or cut_off when a short first-attempt deadline passed first and the client retried it.",
		),
		&["method", "source", "status"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_HTTP_ATTEMPT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_object_store_http_attempt_duration_seconds",
			"Duration of one object-store HTTP attempt until its response headers, by response status; an abandoned attempt records its elapsed time when dropped.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["status"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_RETRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_retries_total",
			"Object-store HTTP attempts past the first of one request, by operation.",
		),
		&["op"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_MULTIPART_UPLOADS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_multipart_uploads_total",
			"Object-store multipart upload steps (start, complete, abort) by object source and result.",
		),
		&["source", "step", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_MULTIPART_PARTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_multipart_parts_total",
			"Object-store multipart upload parts sent, by object source.",
		),
		&["source"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref OBJECT_STORE_MULTIPART_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_multipart_bytes_total",
			"Bytes sent in object-store multipart upload parts, by object source.",
		),
		&["source"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref OBJECT_STORE_FIRST_ATTEMPTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_object_store_first_attempts_total",
			"First HTTP attempts of small object-store requests that ran under a short response deadline, by billed method and result: answered before the deadline, cut_off and retried by the client with its normal timeouts, or waited past the deadline because the retry budget was spent. Every cut_off attempt adds at least one billed request; a cut-off conditional PUT also leads its caller to read the object back.",
		),
		&["method", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

// Cold-store shape and tablet flush timing, in their own block to stay under the macro
// recursion limit of one `lazy_static!` invocation.
lazy_static::lazy_static! {
	pub(crate) static ref COLD_STORE_L0_SSTS_SAMPLED: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_cold_store_l0_ssts",
			"L0 SSTs of one tablet cold store, observed at each manifest sample after a tablet flush. Every tablet SST spans the whole key range, so a scan visits each L0 SST its prefix filter does not reject.",
		)
		.buckets(rivet_metrics::SMALL_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref COLD_STORE_SORTED_RUNS_SAMPLED: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_cold_store_sorted_runs",
			"Sorted runs of one tablet cold store, observed at each manifest sample after a tablet flush. A read visits at most one SST per sorted run.",
		)
		.buckets(rivet_metrics::SMALL_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_FLUSH_MEMTABLE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_tablet_flush_memtable_seconds",
			"Time a tablet flush waits for SlateDB to make its memtable durable, by whether L0 was at its limit when the flush began (l0=full waits for a compaction before uploading, l0=open does not). The upload and manifest parts are rivet_depot_object_store_request_duration_seconds with source=sst and source=manifest.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["l0"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

// Store limit usage and capacity, in their own block to stay under the macro recursion
// limit of one `lazy_static!` invocation.
lazy_static::lazy_static! {
	pub(crate) static ref FS_STORE_LOCK_STRIPES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_filesystem_store_lock_stripes",
			"Volume lock stripes summed across open tablet filesystem stores.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref FS_STORE_LOCK_STRIPES_HELD: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_filesystem_store_lock_stripes_held",
			"Volume lock stripes held by store operations, summed across open tablet filesystem stores.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_MAINTAINER_INBOX_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_tablet_maintainer_inbox_capacity",
			"Configured event inbox capacity of one tablet maintainer.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_MAINTAINER_INBOX_FULL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_maintainer_inbox_full_total",
			"Requests a tablet maintainer rejected as overloaded because its event inbox was full, by request.",
		),
		&["request"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"depot_tablet_store_capacity",
			"Configured tablet store limits summed across active maintainers: flush_staging and flush_uploading slots, admission_scan items, and background_read, the size of each background cold-read window (held_image_read, replica_log_read, fork_job and fork_cleanup_job in depot_tablet_store_in_flight).",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref TABLET_STORE_SCAN_LIMITED: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_tablet_store_scan_limited",
			"Active maintainers whose store queue is longer than one admission pass looks.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_UNLOAD_FORGETS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_unload_forgets_total", "Answers to an unload's conditional forget of one member, by result: forgotten, not_loaded, mismatch (the member holds entries past the row), claim_changed (the member stayed at the row but holds a claim newer than the one the forget expected), deleted, refused (the member answered without forgetting; it waits for a retry, or the unload is cancelled when a newer writer holds the volume) failed (no answer, so the forget may still apply; it waits for a retry) or late (a forget sent before its unload was cancelled removed the member, which is then reloaded)."), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_VOLUME_OPEN_EXPIRED: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_volume_open_expired_total", "Volume opens the maintainer refused as overloaded at its answer deadline (half the caller's remaining time, or the caller's deadline once a creating open's loads are out), by the step they were waiting in: awaiting_deletion, waiting_for_repair, reading_row or loading."), &["step"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_VOLUME_OPEN_REJECTED: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_volume_open_rejected_total", "Volume opens a tablet maintainer rejected as overloaded at admission, by reason: inbox_queue (refused at submit because depot.maintainer_max_queued_opens opens were already queued in the inbox), deadline, invalid_retention, key_too_large, request_capacity, row_read_capacity, fork_hold, key_opening (another open of the key is in flight), key_row_read or memtable_full (its row lookup queued behind store work that the full memtable budget holds)."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_VOLUME_OPEN_INBOX_DEPTH: Histogram = register_histogram_with_registry!(
		"depot_tablet_volume_open_inbox_depth",
		"Volume opens already queued in the tablet maintainer's inbox, not yet taken by its runner, when another open was submitted. Other inbox events are not counted.",
		rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub(crate) static ref TABLET_VOLUME_OPEN_QUEUE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_tablet_volume_open_queue_capacity",
			"Configured volume opens one tablet maintainer queues in its inbox before it refuses more at submit (depot.maintainer_max_queued_opens).",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref FILESYSTEM_SHARED_FLUSHES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_filesystem_shared_flushes_total", "Fork, checkpoint and materialization changes that needed a durable memtable, by whether the caller ran the flush (led) or a flush another caller ran already covered it (joined)."), &["result"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref LOG_SHARD_BATCH_REQUESTS: Histogram = register_histogram_with_registry!("depot_log_shard_batch_requests", "Requests carried by one coalesced log shard batch a client sent to this worker.", rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref DELETE_DRAIN_DURATION: Histogram = register_histogram_with_registry!("depot_delete_drain_seconds", "Time a delete or restore spent claiming an incarnation and writing its final log tail to cold before stopping its apply work.", rivet_metrics::BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DELETE_DRAINS_APPLIED: IntCounter = register_int_counter_with_registry!("depot_delete_drains_applied_total", "Delete and restore drains that found unapplied entries above cold and wrote them before cleanup.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref RETIRED_LOGS_AWAITING_IMPORT: IntGauge = register_int_gauge_with_registry!("depot_retired_logs_awaiting_import", "Retired split parents in the manifest whose maintainer log waits for a child's import before it can be forgotten.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref RETIRED_LOG_REPLICAS_FORGOTTEN: IntCounter = register_int_counter_with_registry!("depot_retired_log_replicas_forgotten_total", "Replicas of a retired split parent's maintainer log forgotten by the parent's cleanup owner once every child imported the log.", *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_FS_VOLUME_RECORD_MISSING: IntCounter = register_int_counter_with_registry!("depot_tablet_volume_record_missing_total", "Loaded volume incarnations whose stored record the cold store does not hold although a replica reports a cold position above zero. Each stays loaded and unregistered for the maintainer lifetime and its name refuses requests until the incarnation is deleted; a successor maintainer counts it again.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_FORK_REPLAY_RECORD_MISSING: IntCounter = register_int_counter_with_registry!("depot_tablet_fork_replay_record_missing_total", "Acknowledged fork replay attempts paused because their image source record is missing. An operator must repair the source before replay can continue.", *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_MAINTAINER_FENCES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_maintainer_fences_total",
			"Tablet maintainer lifetimes ended by a fence, by the source file of the fencing call.",
		),
		&["site"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub(crate) static ref TABLET_STORE_OPERATIONS_SLOW: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_tablet_store_operations_slow_total",
			"Deadlines a tablet store operation outlived while the maintainer kept its grant and waited for the operation's own result, by operation.",
		),
		&["operation"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_STORE_FLUSHES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_store_flushes_total", "SlateDB memtable flushes a tablet store ran, by reason: its flush pass, a split import, or a checkpoint, fork or fork reference change of a tablet whose maintainer log never started."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	/// Read session opens a tablet maintainer held until it had applied their position.
	pub(crate) static ref TABLET_FS_OPEN_APPLY_WAITS: IntCounter = register_int_counter_with_registry!(
		"depot_tablet_filesystem_open_apply_waits_total",
		"Read session opens at an exact position a tablet maintainer held until it had applied that far, as after a maintainer move.",
		*rivet_metrics::REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref KV_WORKER_BUDGET_EVICTIONS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_kv_worker_budget_evictions_total", "Opens refused at a full worker KV volume budget, by where they started an idle volume's close: own_host, other_host, or none when every host was busy or at its eviction close limit."), &["result"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref FS_BULK_RUNS: IntCounter = register_int_counter_with_registry!("depot_tablet_filesystem_bulk_runs_total", "Chunk runs read sessions scanned with the bulk run settings (depot.bulk_read_min_chunks and above).", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_SESSION_READS_PASSED_OVER: IntCounter = register_int_counter_with_registry!("depot_tablet_session_reads_passed_over_total", "Session reads an admission pass left queued because their volume held its share of client cold-read slots (depot.max_cold_reads_per_volume); a read waiting across several passes counts once per pass.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_BULK_BUDGET_WAITS: IntCounter = register_int_counter_with_registry!("depot_tablet_filesystem_bulk_budget_waits_total", "Bulk chunk runs that waited for the process's bulk read budget (depot.bulk_read_bytes_per_sec).", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_BULK_BUDGET_WAIT_SECONDS: Histogram = register_histogram_with_registry!("depot_tablet_filesystem_bulk_budget_wait_seconds", "Time a bulk chunk run waited for the process's bulk read budget.", rivet_metrics::BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref FS_BULK_BUDGET_BYTES_PER_SEC: IntGauge = register_int_gauge_with_registry!("depot_tablet_filesystem_bulk_budget_bytes_per_second", "Configured bulk read budget of this process in bytes per second; 0 when off.", *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
 pub(crate) static ref ALARM_CANDIDATES: IntGauge = register_int_gauge_with_registry!("depot_alarm_candidates", "Cached alarm candidates across local tablet frontiers.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_CANDIDATE_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_alarm_candidate_capacity", "Configured candidate capacity across local tablet frontiers.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_OPERATIONS: IntGauge = register_int_gauge_with_registry!("depot_alarm_operations", "Active keyed alarm operations, including cancelled futures awaiting completion.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_OPERATION_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_alarm_operation_capacity", "Configured keyed alarm operation capacity across local tablets.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_PARKED: IntGauge = register_int_gauge_with_registry!("depot_alarm_parked", "Bounded alarm requests waiting for retry or store admission, without execution slots.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_PARKED_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_alarm_parked_capacity", "Configured parked request capacity across local tablets (depot.alarms.max_candidates per tablet).", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_CALLBACKS: IntGauge = register_int_gauge_with_registry!("depot_alarm_callbacks", "Alarm callbacks currently running across local workers.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_CALLBACK_CAPACITY: IntGauge = register_int_gauge_with_registry!("depot_alarm_callback_capacity", "Configured callback capacity across local workers.", *rivet_metrics::REGISTRY).unwrap();
 pub(crate) static ref ALARM_DISCOVERY_ROWS: IntCounter = register_int_counter_with_registry!("depot_alarm_discovery_rows_total", "Ordered alarm index rows returned by frontier refills.", *rivet_metrics::REGISTRY).unwrap();
}

/// One maintainer's sampled frontier contribution. Dropping its observer withdraws all counts.
#[derive(Default)]
pub(crate) struct AlarmGauges {
	last: crate::tablet_maintainer_machine::AlarmSnapshot,
	held: Option<[rivet_metrics::IntGaugeGuard; 6]>,
}
impl AlarmGauges {
	pub(crate) fn observe(&mut self, next: crate::tablet_maintainer_machine::AlarmSnapshot) {
		if next == self.last && self.held.is_some() {
			return;
		}
		let near = |used: u32, capacity: u32| {
			capacity > 0 && u64::from(used) * 10 >= u64::from(capacity) * 9
		};
		for (kind, used, cap, old, old_cap) in [
			(
				"candidates",
				next.candidates,
				next.candidate_capacity,
				self.last.candidates,
				self.last.candidate_capacity,
			),
			(
				"operations",
				next.operations,
				next.operation_capacity,
				self.last.operations,
				self.last.operation_capacity,
			),
			(
				"parked requests",
				next.parked,
				next.parked_capacity,
				self.last.parked,
				self.last.parked_capacity,
			),
		] {
			if near(used, cap) && !near(old, old_cap) {
				tracing::warn!(
					kind,
					used,
					capacity = cap,
					"volume alarm frontier is near capacity; delivery remains queued under the configured depot.alarms limits"
				);
			}
		}
		self.held.take();
		self.held = Some([
			ALARM_CANDIDATES.add_guard(i64::from(next.candidates)),
			ALARM_CANDIDATE_CAPACITY.add_guard(i64::from(next.candidate_capacity)),
			ALARM_OPERATIONS.add_guard(i64::from(next.operations)),
			ALARM_OPERATION_CAPACITY.add_guard(i64::from(next.operation_capacity)),
			ALARM_PARKED.add_guard(i64::from(next.parked)),
			ALARM_PARKED_CAPACITY.add_guard(i64::from(next.parked_capacity)),
		]);
		self.last = next;
	}
}

lazy_static::lazy_static! {
	pub(crate) static ref TABLET_ABOVE_COLD_BYTES: IntGauge = register_int_gauge_with_registry!("depot_tablet_above_cold_bytes", "Log bytes the tablets this process maintains hold above cold: applied bytes no pass has written to cold plus entries read for apply. Log entries no apply read has reached yet are not counted.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MAINTAINER_APPLY_LAG: Histogram = register_histogram_with_registry!("depot_maintainer_apply_lag_seconds", "Time from an entry's write to its apply by the tablet maintainer, for the oldest entry of each apply. Uses the writer's wall clock, so skew shifts it.", rivet_metrics::BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MAINTAINER_REPLAYS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_maintainer_replays_total", "Replays a newly granted maintainer finished, by result: caught_up once apply drained, interrupted when the maintainer stopped first."), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MAINTAINER_REPLAY_SECONDS: Histogram = register_histogram_with_registry!("depot_maintainer_replay_seconds", "Time from a newly granted maintainer serving to its replay ending.", rivet_metrics::BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MAINTAINER_REPLAYING: IntGauge = register_int_gauge_with_registry!("depot_maintainer_replaying_tablets", "Tablets this process maintains whose replay after a grant has not ended.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref LEARNER_PHASES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_learner_phases_total", "Learning phases a maintainer saw end, by result: promoted, aborted or interrupted."), &["result"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref LEARNER_PHASE_SECONDS: Histogram = register_histogram_with_registry!("depot_learner_phase_seconds", "Time a maintainer saw its tablet's replica set learning.", rivet_metrics::LIFETIME_BUCKETS.to_vec(), *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref LEARNER_UNCOVERED_VOLUMES: IntGauge = register_int_gauge_with_registry!("depot_learner_uncovered_volumes", "Loaded volumes still lacking proof for their tablet's membership phase, across the tablets this process maintains.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref TABLET_VOLUME_REFUSALS: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("depot_tablet_volume_refusals_total", "Volume requests a tablet maintainer refused to its client, by reason."), &["reason"], *rivet_metrics::REGISTRY).unwrap();
}

lazy_static::lazy_static! {
	pub(crate) static ref LEASE_DEGRADED_REFUSALS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_lease_degraded_refusals_total", "New ownership this worker refused because its topology lease renewal was late, by kind: volume_open, disk_probe, tablet_start or fresh_grant. The worker kept serving what it held."),
		&["kind"], *rivet_metrics::REGISTRY
	).unwrap();
}
