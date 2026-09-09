use std::sync::atomic::AtomicU64;

use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, prometheus::*};

/// Epoch ms of the most recently successfully completed UDB transaction on this pod.
/// Updated by `Database::txn` on every successful commit, observed by the heartbeat
/// background task as `udb_fdb_last_tx_completed_age_seconds`. A value of 0 means no
/// successful tx has been recorded since process start.
pub static LAST_TX_COMPLETED_EPOCH_MS: AtomicU64 = AtomicU64::new(0);

lazy_static::lazy_static! {
	pub static ref FDB_PING_DURATION: Histogram = register_histogram_with_registry!(
		"udb_fdb_ping_duration",
		"Total duration to retrieve a single value from fdb.",
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref FDB_MISSED_PING: IntGauge = register_int_gauge_with_registry!(
		"udb_fdb_missed_ping",
		"1 if fdb missed the last ping.",
		*REGISTRY
	).unwrap();
	pub static ref FDB_LAST_TX_COMPLETED_AGE_SECONDS: Gauge = register_gauge_with_registry!(
		"udb_fdb_last_tx_completed_age_seconds",
		"Seconds since the most recent successful UDB tx completion on this pod. Diverges when all UDB writes on the pod stall (e.g. FDB client thread starvation); a single per-pod signal for global degradation.",
		*REGISTRY
	).unwrap();

	/// Current size of a Postgres driver connection pool, labelled `follower` or `leader`.
	pub static ref POSTGRES_POOL_SIZE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_postgres_pool_size",
		"Connections currently held by a UniversalDB Postgres connection pool.",
		&["pool"],
		*REGISTRY
	).unwrap();
	pub static ref POSTGRES_POOL_AVAILABLE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_postgres_pool_available",
		"Idle connections available for checkout in a UniversalDB Postgres connection pool.",
		&["pool"],
		*REGISTRY
	).unwrap();
	/// Paired with `udb_postgres_pool_available`: available at 0 while waiting climbs is pool
	/// starvation, where every slot is checked out and new transactions queue behind them.
	pub static ref POSTGRES_POOL_WAITING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_postgres_pool_waiting",
		"Callers queued waiting for a connection from a UniversalDB Postgres connection pool.",
		&["pool"],
		*REGISTRY
	).unwrap();

	pub static ref KEY_PACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_pack_count",
		"How many times a key has been packed.",
		&["type"],
		*REGISTRY
	).unwrap();
	pub static ref KEY_UNPACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_unpack_count",
		"How many times a key has been unpacked.",
		&["type"],
		*REGISTRY
	).unwrap();

	pub static ref TRANSACTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_transaction_total",
		"How many transactions have been started.",
		&["name"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_PENDING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_transaction_pending",
		"How many transactions have been started.",
		&["name"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"udb_transaction_duration",
		"Duration of a transaction.",
		&["name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_ATTEMPTS: HistogramVec = register_histogram_vec_with_registry!(
		"udb_transaction_attempts",
		"Amount of attempts (1 + retries) taken for a transaction.",
		&["name"],
		vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 12.0, 14.0, 16.0],
		*REGISTRY
	).unwrap();

	pub static ref OPERATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_total",
		"How many UniversalDB operations have completed.",
		&["op", "isolation", "result"],
		*REGISTRY
	).unwrap();
	pub static ref OPERATION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"udb_operation_duration_seconds",
		"Duration of UniversalDB operations.",
		&["op", "isolation", "result"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	/// Carries the transaction name alongside the operation, so byte volume can be attributed either
	/// to the kind of operation that moved it or to the transaction that asked for it. Both dimensions
	/// are load-bearing: `op` distinguishes a range scan from a point read, and `name` is the only way
	/// to tell which caller a given rate belongs to.
	pub static ref OPERATION_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_bytes",
		"Bytes read or written by UniversalDB operations.",
		&["op", "direction", "name"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_CHARGED_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_throttle_charged_bytes",
		"Bytes charged against a named UniversalDB throttle axis by this process.",
		&["throttle", "kind"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_DROPPED_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_throttle_dropped_bytes",
		"Bytes charged against a throttle axis that aged out before they could be flushed, and so were never counted against the budget. Non-zero means the throttle is under-counting.",
		&["throttle", "kind"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_PENDING_BYTES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_throttle_pending_bytes",
		"Bytes charged on this process and not yet flushed to the shared counters, sampled at each flush. Sustained growth means flushing is not keeping up or is failing.",
		&["throttle", "kind"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_ESTIMATE_BYTES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_throttle_estimate_bytes",
		"Cluster-wide sliding-window estimate of bytes charged to a throttle axis, as this process last refreshed it. Compare against udb_throttle_budget_bytes to see saturation.",
		&["throttle", "kind"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_BUDGET_BYTES: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_throttle_budget_bytes",
		"Configured per-window byte budget for a throttle axis, before any class multiplier. Distinguishes a budget that is too low from load that is too high.",
		&["throttle", "kind"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_DECISIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_throttle_decisions_total",
		"Throttle admission checks by outcome. The denied ratio is the controller's backpressure; an axis at budget denies most checks.",
		&["throttle", "kind", "result"],
		*REGISTRY
	).unwrap();
	pub static ref THROTTLE_FLUSH: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_throttle_flush_total",
		"Throttle flush transactions by outcome. A process that is charging but not flushing is invisible to every other worker.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref OPERATION_KEYS: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_keys",
		"Keys read or written by UniversalDB operations.",
		&["op"],
		*REGISTRY
	).unwrap();
}
