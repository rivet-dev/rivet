//! Migrator metrics. Labels are bounded: the target namespace name, the result and the reason.
use std::time::Duration;

use rivet_metrics::{BUCKETS, BYTE_SIZE_BUCKETS, REGISTRY, prometheus::*};

use crate::migrator_machine::types::wire;

lazy_static::lazy_static! {
	pub static ref ACTOR_RESULT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"migrate_from_v2_actor_result_total",
		"V2 actors the migrator finished or skipped, by result (confirmed, woke, destroyed, skipped, failed) and reason.",
		&["namespace", "result", "reason"],
		*REGISTRY
	).unwrap();

	pub static ref ACTORS: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"migrate_from_v2_actors",
		"Actor counts from the last full pass, by state: v2_remaining, eligible_pending, migrated, skipped, too_large or needs_recovery.",
		&["namespace", "state"],
		*REGISTRY
	).unwrap();

	pub static ref MIGRATE_RATIO: GaugeVec = register_gauge_vec_with_registry!(
		"migrate_from_v2_target_ratio",
		"The configured share of existing V2 actors to migrate (migrate_from_v2.migrate_ratio).",
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref PASS_COMPLETED_AT_SECONDS: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"migrate_from_v2_pass_completed_at_seconds",
		"Unix time at which the last migration pass ended.",
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref PASS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("migrate_from_v2_pass_duration_seconds", "Length of one migration pass.").buckets(BUCKETS.to_vec()),
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref COPY_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("migrate_from_v2_copy_bytes", "Bytes copied per migrated actor, by kind: kv or sqlite.")
			.buckets(BYTE_SIZE_BUCKETS.to_vec()),
		&["namespace", "kind"],
		*REGISTRY
	).unwrap();

	pub static ref COPY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("migrate_from_v2_copy_duration_seconds", "Time to copy one actor's export into V3.").buckets(BUCKETS.to_vec()),
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref IN_FLIGHT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"migrate_from_v2_in_flight",
		"Actors the migrator has in flight, by phase: copying, reserving, cutting_over, confirming, activating or discarding.",
		&["namespace", "phase"],
		*REGISTRY
	).unwrap();

	pub static ref IN_FLIGHT_OLDEST_STARTED_AT_SECONDS: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"migrate_from_v2_in_flight_oldest_started_at_seconds",
		"Unix time at which the oldest in-flight actor was admitted, or 0 with none in flight. An actor stuck retrying a step keeps it old.",
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref EFFECT_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"migrate_from_v2_effect_failures_total",
		"Failed or timed-out migration steps that are retried, by the phase that failed.",
		&["namespace", "phase"],
		*REGISTRY
	).unwrap();

	pub static ref COPY_ADMITTED_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"migrate_from_v2_copy_admitted_bytes_total",
		"Copy bytes charged against the copy byte budget, by V2's reported size, when an actor is admitted. Its rate is the budget in use.",
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref COPY_BUDGET_BYTES_PER_SECOND: IntGauge = register_int_gauge_with_registry!(
		"migrate_from_v2_copy_budget_bytes_per_second",
		"Configured migrate_from_v2.max_copy_bytes_per_sec.",
		*REGISTRY
	).unwrap();

	pub static ref IN_FLIGHT_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"migrate_from_v2_in_flight_capacity",
		"Configured migrate_from_v2.max_in_flight.",
		*REGISTRY
	).unwrap();
}

pub fn observe_copy(namespace: &str, kv_bytes: u64, sqlite_bytes: u64, elapsed: Duration) {
	COPY_BYTES
		.with_label_values(&[namespace, "kv"])
		.observe(kv_bytes as f64);
	COPY_BYTES
		.with_label_values(&[namespace, "sqlite"])
		.observe(sqlite_bytes as f64);
	COPY_DURATION
		.with_label_values(&[namespace])
		.observe(elapsed.as_secs_f64());
}

/// Sets the pass gauges from a finished pass's counts.
pub fn report_pass_end(namespace: &str, pass: &wire::MigrateFromV2Pass) {
	let counts = &pass.counts;
	// Actors the pass selected and then confirmed already count as migrated, so only the selected
	// ones still unconfirmed remain in V2.
	let selected_unconfirmed = counts.selected.saturating_sub(counts.confirmed);
	ACTORS
		.with_label_values(&[namespace, "v2_remaining"])
		.set((counts.not_selected + counts.skipped + selected_unconfirmed) as i64);
	ACTORS
		.with_label_values(&[namespace, "eligible_pending"])
		.set(selected_unconfirmed as i64);
	ACTORS
		.with_label_values(&[namespace, "migrated"])
		.set((counts.already_migrated + counts.confirmed) as i64);
	ACTORS
		.with_label_values(&[namespace, "skipped"])
		.set(counts.skipped as i64);
	ACTORS
		.with_label_values(&[namespace, "too_large"])
		.set(counts.too_large as i64);
	ACTORS
		.with_label_values(&[namespace, "needs_recovery"])
		.set(counts.needs_recovery as i64);
	if let Some(ended_at) = pass.ended_at {
		PASS_COMPLETED_AT_SECONDS
			.with_label_values(&[namespace])
			.set(ended_at / 1000);
		PASS_DURATION
			.with_label_values(&[namespace])
			.observe(ended_at.saturating_sub(pass.started_at).max(0) as f64 / 1000.0);
	}
}
