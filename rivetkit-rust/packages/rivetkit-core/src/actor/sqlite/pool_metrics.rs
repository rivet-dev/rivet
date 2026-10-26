//! Process gauges for the runtime's shared native SQLite worker pool.

use std::sync::LazyLock;

use rivet_metrics::prometheus::{Histogram, HistogramOpts, HistogramVec, IntGauge, Opts};

struct PoolCollectors {
	threads: IntGauge,
	busy: IntGauge,
	runnable: IntGauge,
	queue_wait_seconds: Histogram,
	lock_wait_seconds: HistogramVec,
	page_buffer_pages: IntGauge,
	page_buffer_capacity_pages: IntGauge,
	pager_cache_bytes: IntGauge,
	pager_cache_pages: IntGauge,
	pager_cache_capacity_bytes: IntGauge,
	pager_cache_unbudgeted_bytes: IntGauge,
}

static POOL_METRICS: LazyLock<PoolCollectors> = LazyLock::new(|| {
	let threads = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_worker_pool_threads",
		"configured thread cap of the native SQLite worker pool",
	))
	.expect("create sqlite_worker_pool_threads gauge");
	let busy = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_worker_pool_busy",
		"native SQLite worker pool threads running a database action",
	))
	.expect("create sqlite_worker_pool_busy gauge");
	let runnable = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_worker_pool_runnable_databases",
		"databases waiting for a native SQLite worker pool thread",
	))
	.expect("create sqlite_worker_pool_runnable_databases gauge");
	let queue_wait_seconds = Histogram::with_opts(
		HistogramOpts::new(
			"rivetkit_sqlite_worker_pool_queue_wait_seconds",
			"time a runnable database waited for a native SQLite worker pool thread",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
	)
	.expect("create sqlite_worker_pool_queue_wait_seconds histogram");
	let lock_wait_seconds = HistogramVec::new(
		HistogramOpts::new(
			"rivetkit_sqlite_worker_pool_lock_wait_seconds",
			"time a caller waited for a native SQLite worker pool lock, by lock",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		&["lock"],
	)
	.expect("create sqlite_worker_pool_lock_wait_seconds histogram");
	let page_buffer_pages = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_page_buffer_pages",
		"pages the process-wide native SQLite VFS page budget holds across every database",
	))
	.expect("create sqlite_page_buffer_pages gauge");
	let page_buffer_capacity_pages = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_page_buffer_capacity_pages",
		"configured page capacity of the process-wide native SQLite VFS page budget",
	))
	.expect("create sqlite_page_buffer_capacity_pages gauge");
	let pager_cache_bytes = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_pager_cache_bytes",
		"bytes the process-wide SQLite pager cache holds across every connection",
	))
	.expect("create sqlite_pager_cache_bytes gauge");
	let pager_cache_pages = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_pager_cache_pages",
		"pages the process-wide SQLite pager cache holds across every connection",
	))
	.expect("create sqlite_pager_cache_pages gauge");
	let pager_cache_capacity_bytes = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_pager_cache_capacity_bytes",
		"configured byte budget of the process-wide SQLite pager cache",
	))
	.expect("create sqlite_pager_cache_capacity_bytes gauge");
	let pager_cache_unbudgeted_bytes = IntGauge::with_opts(Opts::new(
		"rivetkit_sqlite_pager_cache_unbudgeted_bytes",
		"bytes of in-memory and temporary database pages the process-wide SQLite pager cache holds outside its budget",
	))
	.expect("create sqlite_pager_cache_unbudgeted_bytes gauge");
	for collector in [
		Box::new(threads.clone()) as Box<dyn rivet_metrics::prometheus::core::Collector>,
		Box::new(busy.clone()),
		Box::new(runnable.clone()),
		Box::new(queue_wait_seconds.clone()),
		Box::new(lock_wait_seconds.clone()),
		Box::new(page_buffer_pages.clone()),
		Box::new(page_buffer_capacity_pages.clone()),
		Box::new(pager_cache_bytes.clone()),
		Box::new(pager_cache_pages.clone()),
		Box::new(pager_cache_capacity_bytes.clone()),
		Box::new(pager_cache_unbudgeted_bytes.clone()),
	] {
		if let Err(error) = rivet_metrics::REGISTRY.register(collector) {
			tracing::warn!(?error, "sqlite pool metric registration failed");
		}
	}
	PoolCollectors {
		threads,
		busy,
		runnable,
		queue_wait_seconds,
		lock_wait_seconds,
		page_buffer_pages,
		page_buffer_capacity_pages,
		pager_cache_bytes,
		pager_cache_pages,
		pager_cache_capacity_bytes,
		pager_cache_unbudgeted_bytes,
	}
});

/// Reports the pool through the process-global registry.
pub(crate) struct PoolMetrics;

impl sqlite_embedded::worker::SqlitePoolMetrics for PoolMetrics {
	fn set_pool_threads(&self, threads: u64) {
		POOL_METRICS.threads.set(threads as i64);
	}

	fn set_pool_busy(&self, busy: u64) {
		POOL_METRICS.busy.set(busy as i64);
	}

	fn set_pool_runnable(&self, runnable: u64) {
		POOL_METRICS.runnable.set(runnable as i64);
	}

	fn observe_pool_queue_wait(&self, wait_ns: u64) {
		POOL_METRICS
			.queue_wait_seconds
			.observe(wait_ns as f64 / 1_000_000_000.0);
	}

	fn observe_pool_lock_wait(&self, lock: &'static str, wait_ns: u64) {
		POOL_METRICS
			.lock_wait_seconds
			.with_label_values(&[lock])
			.observe(wait_ns as f64 / 1_000_000_000.0);
	}

	fn set_page_buffer_capacity_pages(&self, pages: u64) {
		POOL_METRICS.page_buffer_capacity_pages.set(pages as i64);
	}

	fn set_page_buffer_pages(&self, pages: u64) {
		POOL_METRICS.page_buffer_pages.set(pages as i64);
	}

	fn set_pager_cache_capacity_bytes(&self, bytes: u64) {
		POOL_METRICS.pager_cache_capacity_bytes.set(bytes as i64);
	}

	fn set_pager_cache_bytes(&self, bytes: u64) {
		POOL_METRICS.pager_cache_bytes.set(bytes as i64);
	}

	fn set_pager_cache_pages(&self, pages: u64) {
		POOL_METRICS.pager_cache_pages.set(pages as i64);
	}

	fn set_pager_cache_unbudgeted_bytes(&self, bytes: u64) {
		POOL_METRICS.pager_cache_unbudgeted_bytes.set(bytes as i64);
	}
}
