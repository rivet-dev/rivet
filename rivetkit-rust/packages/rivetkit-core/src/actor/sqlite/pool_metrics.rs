//! Process gauges for the runtime's shared native SQLite worker pool.

use rivet_metrics::{
	MICRO_BUCKETS, REGISTRY,
	prometheus::{
		Histogram, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts,
		register_histogram_vec_with_registry, register_histogram_with_registry,
		register_int_counter_vec_with_registry, register_int_gauge_with_registry,
	},
};

lazy_static::lazy_static! {
	static ref SQLITE_WORKER_POOL_THREADS: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_worker_pool_threads",
			"configured thread cap of the native SQLite worker pool",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_WORKER_POOL_BUSY: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_worker_pool_busy",
			"native SQLite worker pool threads running a database action",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_WORKER_POOL_RUNNABLE_DATABASES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_worker_pool_runnable_databases",
			"databases waiting for a native SQLite worker pool thread",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_WORKER_POOL_QUEUE_WAIT_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_worker_pool_queue_wait_seconds",
			"time a runnable database waited for a native SQLite worker pool thread",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_WORKER_POOL_LOCK_WAIT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_sqlite_worker_pool_lock_wait_seconds",
			"time a caller waited for a native SQLite worker pool lock, by lock",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["lock"],
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGE_BUFFER_PAGES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_page_buffer_pages",
			"pages the process-wide native SQLite VFS page budget holds across every database",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGE_BUFFER_CAPACITY_PAGES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_page_buffer_capacity_pages",
			"configured page capacity of the process-wide native SQLite VFS page budget",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGER_CACHE_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_pager_cache_bytes",
			"bytes the process-wide SQLite pager cache holds across every connection",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGER_CACHE_PAGES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_pager_cache_pages",
			"pages the process-wide SQLite pager cache holds across every connection",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGER_CACHE_CAPACITY_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_pager_cache_capacity_bytes",
			"configured byte budget of the process-wide SQLite pager cache",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_PAGER_CACHE_UNBUDGETED_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_pager_cache_unbudgeted_bytes",
			"bytes of in-memory and temporary database pages the process-wide SQLite pager cache holds outside its budget",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_DEFERRED_UNFLUSHED_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"rivetkit_sqlite_deferred_unflushed_bytes",
			"bytes every deferred-commit SQLite database in the process committed locally that storage has not acknowledged",
		),
		*REGISTRY
	)
	.unwrap();

	static ref SQLITE_DEFERRED_FLUSH_RETRIES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"rivetkit_sqlite_deferred_flush_retries_total",
			"failed deferred-commit flush attempts that were retried, by failure class",
		),
		&["class"],
		*REGISTRY
	)
	.unwrap();
}

/// Counts one retried deferred-commit flush attempt. The counter is process-wide, so its
/// cardinality is the handful of failure classes.
pub(crate) fn record_deferred_flush_retry(class: &'static str) {
	SQLITE_DEFERRED_FLUSH_RETRIES_TOTAL
		.with_label_values(&[class])
		.inc();
}

/// Reports the pool through the process-global registry.
pub(crate) struct PoolMetrics;

impl sqlite_embedded::worker::SqlitePoolMetrics for PoolMetrics {
	fn set_pool_threads(&self, threads: u64) {
		SQLITE_WORKER_POOL_THREADS.set(threads as i64);
	}

	fn set_pool_busy(&self, busy: u64) {
		SQLITE_WORKER_POOL_BUSY.set(busy as i64);
	}

	fn set_pool_runnable(&self, runnable: u64) {
		SQLITE_WORKER_POOL_RUNNABLE_DATABASES.set(runnable as i64);
	}

	fn observe_pool_queue_wait(&self, wait_ns: u64) {
		SQLITE_WORKER_POOL_QUEUE_WAIT_SECONDS.observe(wait_ns as f64 / 1_000_000_000.0);
	}

	fn observe_pool_lock_wait(&self, lock: &'static str, wait_ns: u64) {
		SQLITE_WORKER_POOL_LOCK_WAIT_SECONDS
			.with_label_values(&[lock])
			.observe(wait_ns as f64 / 1_000_000_000.0);
	}

	fn set_page_buffer_capacity_pages(&self, pages: u64) {
		SQLITE_PAGE_BUFFER_CAPACITY_PAGES.set(pages as i64);
	}

	fn set_deferred_unflushed_bytes(&self, bytes: u64) {
		SQLITE_DEFERRED_UNFLUSHED_BYTES.set(bytes as i64);
	}

	fn set_page_buffer_pages(&self, pages: u64) {
		SQLITE_PAGE_BUFFER_PAGES.set(pages as i64);
	}

	fn set_pager_cache_capacity_bytes(&self, bytes: u64) {
		SQLITE_PAGER_CACHE_CAPACITY_BYTES.set(bytes as i64);
	}

	fn set_pager_cache_bytes(&self, bytes: u64) {
		SQLITE_PAGER_CACHE_BYTES.set(bytes as i64);
	}

	fn set_pager_cache_pages(&self, pages: u64) {
		SQLITE_PAGER_CACHE_PAGES.set(pages as i64);
	}

	fn set_pager_cache_unbudgeted_bytes(&self, bytes: u64) {
		SQLITE_PAGER_CACHE_UNBUDGETED_BYTES.set(bytes as i64);
	}
}
