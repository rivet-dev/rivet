//! Process-wide prefetch metrics. Labels are code-defined sources and kinds only.
use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	/// Pages replies carried, by the source that planned them.
	pub(crate) static ref PAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("sqlite_prefetch_pages_total", "SQLite pages page protocol replies carried, by the source that planned them: requested, chain, scan, early, frequent or window."),
		&["source"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Pages per reply.
	pub(crate) static ref REPLY_PAGES: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("sqlite_prefetch_reply_pages", "SQLite pages per page protocol reply, requested and prefetched.")
			.buckets(rivet_metrics::PAGE_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Overflow rows that disagreed with the file.
	pub(crate) static ref MISMATCH_TOTAL: IntCounter = register_int_counter_with_registry!(
		Opts::new("sqlite_prefetch_mismatch_total", "SQLite overflow chains whose rows disagreed with the pages read at the head; their rows are deleted by the next commit."),
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Overflow-row reads, by kind and by whether a commit waited for them.
	pub(crate) static ref LOOKUPS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("sqlite_prefetch_lookups_total", "SQLite overflow map reads, by kind (head, floor or trunk) and phase (read, after_reply or commit). A commit lookup is a read the commit waited for."),
		&["kind", "phase"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Overflow rows commits wrote, by op.
	pub(crate) static ref ROWS_WRITTEN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("sqlite_prefetch_overflow_rows_written_total", "SQLite overflow map row ops commits carried, by op: put, delete or purge."),
		&["op"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Overflow rows a database's opening held in memory when it closed.
	pub(crate) static ref ROWS_PER_DATABASE: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("sqlite_prefetch_overflow_rows_per_database", "SQLite overflow map rows an opening knew when it closed, one observation per database opening.")
			.buckets(rivet_metrics::PAGE_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();
}
