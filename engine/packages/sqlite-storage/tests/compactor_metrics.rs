use std::sync::Arc;

use anyhow::Result;
use rivet_metrics::prometheus::core::Collector;
use sqlite_storage::compactor::{
	CompactorConfig, SqliteCompactPayload, metrics, worker,
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

#[test]
fn metrics_register_without_panic() {
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_LAG,
		"sqlite_compactor_lag_seconds",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL,
		"sqlite_compactor_lease_take_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_LEASE_HELD_SECONDS,
		"sqlite_compactor_lease_held_seconds",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL,
		"sqlite_compactor_lease_renewal_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_PASS_DURATION,
		"sqlite_compactor_pass_duration_seconds",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL,
		"sqlite_compactor_pages_folded_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_DELTAS_FREED_TOTAL,
		"sqlite_compactor_deltas_freed_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL,
		"sqlite_compactor_compare_and_clear_noop_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_COMPACTOR_UPS_PUBLISH_TOTAL,
		"sqlite_compactor_ups_publish_total",
	);
	assert_metric_name(
		&*metrics::SQLITE_STORAGE_USED_BYTES,
		"sqlite_storage_used_bytes",
	);

	#[cfg(debug_assertions)]
	{
		assert_metric_name(
			&*metrics::SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL,
			"sqlite_quota_validate_mismatch_total",
		);
		assert_metric_name(
			&*metrics::SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL,
			"sqlite_takeover_invariant_violation_total",
		);
		assert_metric_name(
			&*metrics::SQLITE_FENCE_MISMATCH_TOTAL,
			"sqlite_fence_mismatch_total",
		);
	}
}

#[test]
fn metric_label_set_includes_node_id() {
	assert_has_label(&*metrics::SQLITE_COMPACTOR_LAG, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_LEASE_HELD_SECONDS, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_PASS_DURATION, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL, "node_id");
	assert_has_label(&*metrics::SQLITE_COMPACTOR_DELTAS_FREED_TOTAL, "node_id");
	assert_has_label(
		&*metrics::SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL,
		"node_id",
	);
	assert_has_label(&*metrics::SQLITE_COMPACTOR_UPS_PUBLISH_TOTAL, "node_id");
	assert_has_label(&*metrics::SQLITE_STORAGE_USED_BYTES, "node_id");

	#[cfg(debug_assertions)]
	{
		assert_has_label(&*metrics::SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL, "node_id");
		assert_has_label(
			&*metrics::SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL,
			"node_id",
		);
		assert_has_label(&*metrics::SQLITE_FENCE_MISMATCH_TOTAL, "node_id");
	}
}

#[test]
fn lease_take_outcome_labels() {
	let node_id = "test-node";
	for outcome in ["acquired", "skipped", "conflict"] {
		metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id, outcome])
			.inc();
	}
}

#[tokio::test]
async fn compactor_service_starts() -> Result<()> {
	let before = histogram_sample_count(&*metrics::SQLITE_COMPACTOR_LAG);
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

	let after = histogram_sample_count(&*metrics::SQLITE_COMPACTOR_LAG);
	assert!(after > before, "compactor did not emit a lag sample");

	Ok(())
}
