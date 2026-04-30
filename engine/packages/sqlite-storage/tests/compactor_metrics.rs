use std::sync::Arc;

use anyhow::Result;
use rivet_metrics::{
	REGISTRY,
	prometheus::{Encoder, TextEncoder, core::Collector},
};
use sqlite_storage::{
	compactor::{CompactorConfig, SqliteCompactPayload, metrics as compactor_metrics, worker},
	pump::metrics as pump_metrics,
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-metrics-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn assert_metric_name<C: Collector>(collector: &C, name: &str) {
	assert!(
		collector.desc().iter().any(|desc| desc.fq_name == name),
		"missing metric descriptor {name}"
	);
}

fn assert_has_label<C: Collector>(collector: &C, label: &str) {
	assert!(
		collector
			.desc()
			.iter()
			.any(|desc| desc.variable_labels.iter().any(|existing| existing == label)),
		"missing metric label {label}"
	);
}

fn histogram_sample_count<C: Collector>(collector: &C) -> u64 {
	collector
		.collect()
		.iter()
		.flat_map(|family| family.get_metric())
		.map(|metric| metric.get_histogram().get_sample_count())
		.sum()
}

fn scrape_metrics() -> String {
	let encoder = TextEncoder::new();
	let metric_families = REGISTRY.gather();
	let mut buffer = Vec::new();
	encoder
		.encode(&metric_families, &mut buffer)
		.expect("metrics scrape should encode");
	String::from_utf8(buffer).expect("metrics scrape should be utf8")
}

#[test]
fn metrics_register_without_panic() {
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_LAG,
		"sqlite_compactor_lag_seconds",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL,
		"sqlite_compactor_lease_take_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_LEASE_HELD_SECONDS,
		"sqlite_compactor_lease_held_seconds",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL,
		"sqlite_compactor_lease_renewal_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_PASS_DURATION,
		"sqlite_compactor_pass_duration_seconds",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL,
		"sqlite_compactor_pages_folded_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_DELTAS_FREED_TOTAL,
		"sqlite_compactor_deltas_freed_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL,
		"sqlite_compactor_compare_and_clear_noop_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_EVICTION_OCC_ABORT_TOTAL,
		"sqlite_eviction_occ_abort_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_SHARD_VERSIONS_PER_SHARD,
		"sqlite_shard_versions_per_shard",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COMPACTOR_UPS_PUBLISH_TOTAL,
		"sqlite_compactor_ups_publish_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_STORAGE_USED_BYTES,
		"sqlite_storage_used_bytes",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COLD_PASS_DURATION,
		"sqlite_cold_pass_duration_seconds",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL,
		"sqlite_cold_pass_layers_uploaded_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COLD_PASS_BYTES_UPLOADED_TOTAL,
		"sqlite_cold_pass_bytes_uploaded_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_COLD_LEASE_TAKE_TOTAL,
		"sqlite_cold_lease_take_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_EVICTION_PASS_DURATION,
		"sqlite_eviction_pass_duration_seconds",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_EVICTION_PASS_SHARDS_CLEARED_TOTAL,
		"sqlite_eviction_pass_shards_cleared_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_EVICTION_PASS_DELTAS_CLEARED_TOTAL,
		"sqlite_eviction_pass_deltas_cleared_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_PENDING_MARKER_ORPHAN_CLEANED_TOTAL,
		"sqlite_pending_marker_orphan_cleaned_total",
	);
	assert_metric_name(
		&*compactor_metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL,
		"sqlite_s3_request_failures_total",
	);
	assert_metric_name(&*pump_metrics::SQLITE_BRANCH_FORK_TOTAL, "sqlite_branch_fork_total");
	assert_metric_name(
		&*pump_metrics::SQLITE_BRANCH_DELETE_TOTAL,
		"sqlite_branch_delete_total",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BOOKMARK_CREATE_TOTAL,
		"sqlite_bookmark_create_total",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BOOKMARK_RESOLVE_TOTAL,
		"sqlite_bookmark_resolve_total",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BOOKMARK_RESOLVE_DURATION,
		"sqlite_bookmark_resolve_duration_seconds",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BRANCH_ANCESTRY_WALK_DEPTH,
		"sqlite_branch_ancestry_walk_depth",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BRANCH_PIN_ADVANCE_TOTAL,
		"sqlite_branch_pin_advance_total",
	);
	assert_metric_name(&*pump_metrics::SQLITE_PIN_STATUS, "sqlite_pin_status");
	assert_metric_name(&*pump_metrics::SQLITE_DR_POSTURE, "sqlite_dr_posture");
	assert_metric_name(
		&*pump_metrics::SQLITE_PINNED_BOOKMARK_COUNT_PER_NAMESPACE,
		"sqlite_pinned_bookmark_count_per_namespace",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_COLD_LAG_VERSIONSTAMPS,
		"sqlite_cold_lag_versionstamps",
	);
	assert_metric_name(
		&*pump_metrics::SQLITE_BOOKMARK_RESOLUTION_CHAIN_DEPTH,
		"sqlite_bookmark_resolution_chain_depth",
	);

	#[cfg(debug_assertions)]
	{
		assert_metric_name(
			&*compactor_metrics::SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL,
			"sqlite_quota_validate_mismatch_total",
		);
		assert_metric_name(
			&*compactor_metrics::SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL,
			"sqlite_takeover_invariant_violation_total",
		);
		assert_metric_name(
			&*compactor_metrics::SQLITE_FENCE_MISMATCH_TOTAL,
			"sqlite_fence_mismatch_total",
		);
	}
}

#[test]
fn metric_label_set_includes_node_id() {
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_LAG, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_LEASE_HELD_SECONDS, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_PASS_DURATION, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_DELTAS_FREED_TOTAL, "node_id");
	assert_has_label(
		&*compactor_metrics::SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL,
		"node_id",
	);
	assert_has_label(&*compactor_metrics::SQLITE_EVICTION_OCC_ABORT_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_SHARD_VERSIONS_PER_SHARD, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COMPACTOR_UPS_PUBLISH_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_STORAGE_USED_BYTES, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COLD_PASS_DURATION, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COLD_PASS_BYTES_UPLOADED_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_COLD_LEASE_TAKE_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_EVICTION_PASS_DURATION, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_EVICTION_PASS_SHARDS_CLEARED_TOTAL, "node_id");
	assert_has_label(&*compactor_metrics::SQLITE_EVICTION_PASS_DELTAS_CLEARED_TOTAL, "node_id");
	assert_has_label(
		&*compactor_metrics::SQLITE_PENDING_MARKER_ORPHAN_CLEANED_TOTAL,
		"node_id",
	);
	assert_has_label(
		&*compactor_metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL,
		"node_id",
	);
	assert_has_label(&*pump_metrics::SQLITE_BRANCH_FORK_TOTAL, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BRANCH_DELETE_TOTAL, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BOOKMARK_CREATE_TOTAL, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BOOKMARK_RESOLVE_TOTAL, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BOOKMARK_RESOLVE_DURATION, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BRANCH_ANCESTRY_WALK_DEPTH, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BRANCH_PIN_ADVANCE_TOTAL, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_PIN_STATUS, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_DR_POSTURE, "node_id");
	assert_has_label(
		&*pump_metrics::SQLITE_PINNED_BOOKMARK_COUNT_PER_NAMESPACE,
		"node_id",
	);
	assert_has_label(&*pump_metrics::SQLITE_COLD_LAG_VERSIONSTAMPS, "node_id");
	assert_has_label(&*pump_metrics::SQLITE_BOOKMARK_RESOLUTION_CHAIN_DEPTH, "node_id");

	#[cfg(debug_assertions)]
	{
		assert_has_label(&*compactor_metrics::SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL, "node_id");
		assert_has_label(
			&*compactor_metrics::SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL,
			"node_id",
		);
		assert_has_label(&*compactor_metrics::SQLITE_FENCE_MISMATCH_TOTAL, "node_id");
	}
}

#[test]
fn lease_take_outcome_labels() {
	let node_id = "test-node";
	for outcome in ["acquired", "skipped", "conflict"] {
		compactor_metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id, outcome])
			.inc();
		compactor_metrics::SQLITE_COLD_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id, outcome])
			.inc();
	}
}

#[test]
fn pitr_metrics_increment_and_scrape() {
	let node_id = "pitr-test-node";

	pump_metrics::SQLITE_BRANCH_FORK_TOTAL
		.with_label_values(&[node_id, "database", "ok"])
		.inc();
	pump_metrics::SQLITE_BRANCH_DELETE_TOTAL
		.with_label_values(&[node_id, "namespace", "err"])
		.inc();
	pump_metrics::SQLITE_BOOKMARK_CREATE_TOTAL
		.with_label_values(&[node_id, "pinned", "ok"])
		.inc();
	pump_metrics::SQLITE_BOOKMARK_RESOLVE_TOTAL
		.with_label_values(&[node_id, "expired"])
		.inc();
	pump_metrics::SQLITE_BOOKMARK_RESOLVE_DURATION
		.with_label_values(&[node_id])
		.observe(0.25);
	pump_metrics::SQLITE_BRANCH_ANCESTRY_WALK_DEPTH
		.with_label_values(&[node_id])
		.observe(3.0);
	pump_metrics::SQLITE_BRANCH_PIN_ADVANCE_TOTAL
		.with_label_values(&[node_id, "bookmark"])
		.inc();
	pump_metrics::SQLITE_PIN_STATUS
		.with_label_values(&[node_id, "pending"])
		.set(1.0);
	pump_metrics::SQLITE_DR_POSTURE
		.with_label_values(&[node_id, "s3"])
		.set(1.0);
	pump_metrics::SQLITE_PINNED_BOOKMARK_COUNT_PER_NAMESPACE
		.with_label_values(&[node_id])
		.set(7.0);
	pump_metrics::SQLITE_COLD_LAG_VERSIONSTAMPS
		.with_label_values(&[node_id, "actor-a"])
		.set(42.0);
	pump_metrics::SQLITE_BOOKMARK_RESOLUTION_CHAIN_DEPTH
		.with_label_values(&[node_id])
		.observe(4.0);
	compactor_metrics::SQLITE_COLD_PASS_DURATION
		.with_label_values(&[node_id, "A"])
		.observe(0.5);
	compactor_metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL
		.with_label_values(&[node_id, "image"])
		.inc();
	compactor_metrics::SQLITE_COLD_PASS_BYTES_UPLOADED_TOTAL
		.with_label_values(&[node_id])
		.inc_by(4096);
	compactor_metrics::SQLITE_COLD_LEASE_TAKE_TOTAL
		.with_label_values(&[node_id, "conflict"])
		.inc();
	compactor_metrics::SQLITE_EVICTION_PASS_DURATION
		.with_label_values(&[node_id])
		.observe(0.75);
	compactor_metrics::SQLITE_EVICTION_PASS_SHARDS_CLEARED_TOTAL
		.with_label_values(&[node_id])
		.inc();
	compactor_metrics::SQLITE_EVICTION_PASS_DELTAS_CLEARED_TOTAL
		.with_label_values(&[node_id])
		.inc();
	compactor_metrics::SQLITE_PENDING_MARKER_ORPHAN_CLEANED_TOTAL
		.with_label_values(&[node_id])
		.inc();
	compactor_metrics::SQLITE_EVICTION_OCC_ABORT_TOTAL
		.with_label_values(&[node_id, "bk_pin"])
		.inc();
	compactor_metrics::SQLITE_SHARD_VERSIONS_PER_SHARD
		.with_label_values(&[node_id])
		.observe(5.0);

	let scrape = scrape_metrics();
	for name in [
		"sqlite_branch_fork_total",
		"sqlite_bookmark_resolve_duration_seconds_bucket",
		"sqlite_cold_pass_layers_uploaded_total",
		"sqlite_eviction_occ_abort_total",
		"sqlite_shard_versions_per_shard_bucket",
		"sqlite_cold_lag_versionstamps",
	] {
		assert!(scrape.contains(name), "missing scraped metric {name}");
	}
	assert!(scrape.contains("node_id=\"pitr-test-node\""));
	assert!(scrape.contains("kind=\"image\""));
	assert!(scrape.contains("reason=\"bk_pin\""));
}

#[tokio::test]
async fn compactor_service_starts() -> Result<()> {
	let before = histogram_sample_count(&*compactor_metrics::SQLITE_COMPACTOR_LAG);
	let db = Arc::new(test_db().await?);

	worker::test_hooks::handle_payload_once(
		db,
		SqliteCompactPayload {
			actor_id: "metrics-actor".to_string(),
			namespace_id: None,
			actor_name: None,
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		CompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	let after = histogram_sample_count(&*compactor_metrics::SQLITE_COMPACTOR_LAG);
	assert!(after > before, "compactor did not emit a lag sample");

	Ok(())
}
