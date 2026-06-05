//! Metrics definitions for the stateless depot conveyer.

use std::{future::Future, time::Instant};

use anyhow::Result;
use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

const SLOW_PHASE_WARN_THRESHOLD_SECONDS: f64 = 1.0;

pub const SHARD_CACHE_READ_FDB_HIT: &str = "fdb_hit";
pub const SHARD_CACHE_READ_MISS: &str = "miss";

lazy_static::lazy_static! {
	pub static ref SQLITE_PUMP_COMMIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_duration_seconds",
		"Duration of stateless sqlite conveyer commit operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_duration_seconds",
		"Duration of stateless sqlite conveyer get_pages operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_GET_PAGES_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_phase_duration_seconds",
		"Duration of stateless sqlite conveyer get_pages transaction-attempt phases.",
		&["node_id", "phase", "attempt_result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_phase_duration_seconds",
		"Duration of stateless sqlite conveyer commit transaction-attempt phases.",
		&["node_id", "phase", "attempt_result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_dirty_page_count",
		"Number of dirty pages written per stateless sqlite conveyer commit.",
		&["node_id"],
		vec![0.0, 1.0, 4.0, 16.0, 64.0, 256.0, 1024.0, 4096.0, 8192.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_PGNO_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_pgno_count",
		"Number of pages requested per stateless sqlite conveyer get_pages call.",
		&["node_id"],
		vec![0.0, 1.0, 4.0, 16.0, 64.0, 256.0, 1024.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_PIDX_FDB_LOAD_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_pidx_fdb_load_total",
		"Total stateless sqlite conveyer get_pages calls that loaded PIDX rows from FDB.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_READ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_shard_cache_read_total",
		"Total sqlite shard cache read outcomes.",
		&["outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_RESIDENT_BYTES: IntGauge = register_int_gauge_with_registry!(
		"sqlite_shard_cache_resident_bytes",
		"Sampled sqlite shard cache resident bytes.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_FORK_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_fork_total",
		"Total sqlite branch fork operations.",
		&["node_id", "op", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_DELETE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_delete_total",
		"Total sqlite branch delete operations.",
		&["node_id", "op", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_CREATE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_restore_point_create_total",
		"Total sqlite restore point create operations.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLVE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_restore_point_resolve_total",
		"Total sqlite restore point resolve operations.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLVE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_restore_point_resolve_duration_seconds",
		"Duration of sqlite restore point resolve operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_ANCESTRY_WALK_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_branch_ancestry_walk_depth",
		"Observed sqlite database branch ancestry walk depth.",
		&["node_id"],
		vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_PIN_ADVANCE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_pin_advance_total",
		"Total sqlite branch pin advances.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PIN_STATUS: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_pin_status",
		"Sampled sqlite restore point status count.",
		&["node_id", "status"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_DR_POSTURE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_dr_posture",
		"Sampled sqlite disaster-recovery posture.",
		&["node_id", "recoverable_from"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_COUNT_PER_BUCKET: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_restore_point_count_per_bucket",
		"Sampled sqlite restore point count per bucket.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLUTION_CHAIN_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_restore_point_resolution_chain_depth",
		"Observed sqlite restore point resolution parent-chain depth.",
		&["node_id"],
		vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
		*REGISTRY
	).unwrap();
}

pub fn observe_get_pages_phase(
	node_id: &str,
	phase: &'static str,
	start: Instant,
	result: &'static str,
) {
	let elapsed = start.elapsed();
	SQLITE_GET_PAGES_PHASE_DURATION
		.with_label_values(&[node_id, phase, result])
		.observe(elapsed.as_secs_f64());
	if elapsed.as_secs_f64() >= SLOW_PHASE_WARN_THRESHOLD_SECONDS {
		tracing::warn!(
			node_id,
			phase,
			result,
			duration_ms = elapsed.as_millis() as u64,
			"slow depot get_pages phase"
		);
	}
}

pub fn observe_commit_phase(
	node_id: &str,
	phase: &'static str,
	start: Instant,
	result: &'static str,
) {
	let elapsed = start.elapsed();
	SQLITE_COMMIT_PHASE_DURATION
		.with_label_values(&[node_id, phase, result])
		.observe(elapsed.as_secs_f64());
	if elapsed.as_secs_f64() >= SLOW_PHASE_WARN_THRESHOLD_SECONDS {
		tracing::warn!(
			node_id,
			phase,
			result,
			duration_ms = elapsed.as_millis() as u64,
			"slow depot commit phase"
		);
	}
}

pub async fn observe_get_pages_phase_result<T>(
	node_id: &str,
	phase: &'static str,
	future: impl Future<Output = Result<T>>,
) -> Result<T> {
	let start = Instant::now();
	let result = future.await;
	observe_get_pages_phase(node_id, phase, start, result_label(&result));
	result
}

pub async fn observe_commit_phase_result<T>(
	node_id: &str,
	phase: &'static str,
	future: impl Future<Output = Result<T>>,
) -> Result<T> {
	let start = Instant::now();
	let result = future.await;
	observe_commit_phase(node_id, phase, start, result_label(&result));
	result
}

fn result_label<T>(result: &Result<T>) -> &'static str {
	if result.is_ok() { "ok" } else { "error" }
}
