use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	pub static ref FENCES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_fences_total", "Times this process's topology fenced its own lease, by reason."),
		&["reason"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_RENEWAL_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"topology_heartbeat_renewal_seconds",
			"Time from sending a heartbeat renewal to the store confirming it. The lease fences when no renewal confirms within the serving window (ttl minus safety margin) of its send time.",
		).buckets(rivet_metrics::BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LEASE_REMAINING_MS: IntGauge = register_int_gauge_with_registry!(
		"topology_lease_remaining_ms",
		"Time left on this worker's heartbeat lease, sampled at each confirmed renewal and late-renewal warning; 0 once fenced.",
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref EXITED_PREDECESSOR_CLAIMS: IntCounter = register_int_counter_with_registry!(
		Opts::new("topology_exited_predecessor_claims_total", "Registrations that claimed a previous process's heartbeat at once, because this process holds the log directory lock that process held."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WRITE_VERSION_FALLBACKS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_write_version_fallbacks_total", "Write versions chosen while the storage view was expired or never listed, so the writer used its lowest write version, by family."),
		&["family"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_RENEWAL_ATTEMPTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_heartbeat_renewal_attempts_total", "Slow heartbeat renewal writes, by result: superseded (a fresh attempt started beside it) or late_confirmed (a superseded write confirmed the lease)."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WATCHES: IntGauge = register_int_gauge_with_registry!(
		"topology_watches", "Worker processes watched for cleanup, one per process however many subscribers hold it.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WATCH_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"topology_watch_capacity", "Configured cap on watched worker processes.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WATCH_REJECTED: IntCounter = register_int_counter_with_registry!(
		Opts::new("topology_watch_rejected_total", "New process watches refused at the watch capacity."),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WATCH_POLLS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_watch_polls_total", "Heartbeat reads of watched stateless processes, by result: found, missing, unknown or overloaded."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WATCH_GONE: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_watch_gone_total", "Watched processes found gone, by reason: expired, released, replaced or missing."),
		&["reason"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORKER_LEASE_LOOKUPS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_worker_lease_lookups_total", "Exact-id worker lease status lookups, by result: live, expired, released or unknown."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORKER_LEASE_CACHE_ENTRIES: IntGauge = register_int_gauge_with_registry!(
		"topology_worker_lease_cache_entries", "Cached worker lease verdicts.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORKER_LEASE_CACHE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"topology_worker_lease_cache_capacity", "Configured worker lease verdict cache capacity.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LOOKUP_EXPIRY_VISITS: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"topology_lookup_expiry_visits_total",
			"Cached endpoint entries removed by expiry. Expiry visits only due entries.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref DISCOVERY_HEARTBEAT_READS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"topology_discovery_heartbeat_reads_total",
			"Storage heartbeats seen by discovery listings: read again (read) or reused because the listed ETag was unchanged (reused).",
		),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref DISCOVERY_REFRESHES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"topology_refresh_total",
			"Storage discovery listings started, by trigger.",
		),
		&["trigger"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REFRESHES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_membership_refresh_total",
			"Membership refresh outcomes and throttled misses.",
		),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REFRESH_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_membership_refresh_seconds",
			"Time spent reading membership from object storage.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref ENDPOINT_CACHE_ENTRIES: IntGauge = register_int_gauge_with_registry!(
		"topology_endpoint_cache_entries",
		"Retained endpoint hints for exact-id lookups of workers discovery does not list (stateless workers). Storage workers resolve from discovery and are never cached here, so this stays 0 when only storage workers are looked up.",
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref ENDPOINT_CACHE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"topology_endpoint_cache_capacity",
		"Total configured endpoint-entry capacity across active caches.",
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LOOKUP_REJECTED: IntCounter = register_int_counter_with_registry!(
		"topology_lookup_rejected_total",
		"Endpoint requests rejected at configured admission or concurrency limits.",
		*rivet_metrics::REGISTRY
	).unwrap();
	/// Endpoint invalidations dropped because the invalidation admission was full.
	pub static ref INVALIDATIONS_DROPPED: IntCounter = register_int_counter_with_registry!(
		"topology_endpoint_invalidations_dropped_total",
		"Endpoint invalidations dropped because the bounded invalidation admission was full.",
		*rivet_metrics::REGISTRY
	).unwrap();
	/// Opening timeouts reported to topology, by how they were handled.
	pub static ref ENDPOINT_TIMEOUTS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_endpoint_timeouts_total", "Stream opening timeouts to stateless workers reported to topology, by result."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LOOKUP_READS: IntGauge = register_int_gauge_with_registry!(
		"topology_lookup_reads", "Pending endpoint reads.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LOOKUP_READ_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"topology_lookup_read_capacity", "Configured endpoint-read capacity.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LOOKUP_REQUEST_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"topology_lookup_request_capacity", "Configured endpoint-request admission capacity.", *rivet_metrics::REGISTRY
	).unwrap();

	pub static ref HEARTBEAT_GC_SCAN: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_scan_total", "Heartbeat LIST entries and scan failures."),
		&["worker_role", "result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_DELETE: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_delete_total", "Conditional heartbeat delete outcomes."),
		&["worker_role", "result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_CAP_HITS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_cap_hits_total", "Cleanup batches reaching configured caps."),
		&["resource"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_capacity", "Configured heartbeat cleanup limits."),
		&["resource"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_BATCH_ENTRIES: IntGauge = register_int_gauge_with_registry!(
		"topology_heartbeat_gc_batch_entries", "Entries examined in the most recent cleanup batch.", *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_DELETES_IN_FLIGHT: IntGauge = register_int_gauge_with_registry!(
		"topology_heartbeat_gc_deletes_in_flight", "Pending conditional heartbeat deletions.", *rivet_metrics::REGISTRY
	).unwrap();

	pub static ref HEARTBEAT_GC_BATCH_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("topology_heartbeat_gc_batch_seconds", "Time spent in cleanup batches, including timed-out batches.").buckets(rivet_metrics::BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_PASSES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_passes_total", "Cleanup passes over both heartbeat prefixes: completed, or clock_unknown when topology had no store clock evidence yet."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_PASS_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("topology_heartbeat_gc_pass_seconds", "Time spent in one cleanup pass over both heartbeat prefixes, including batch pacing.").buckets(rivet_metrics::LIFETIME_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref HEARTBEAT_GC_PASS_ENTRIES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("topology_heartbeat_gc_pass_entries", "What the most recent completed cleanup pass found: scanned, candidates, deleted, changed_or_missing or failed entries."),
		&["result"], *rivet_metrics::REGISTRY
	).unwrap();
}
