mod common;

use anyhow::Result;
use gas::prelude::*;
use pegboard::metrics;

#[tokio::test]
async fn hash_scan_circuit_breaker_stops_short_and_allows_larger_scan() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let namespace_id_label = namespace_id.to_string();
	let short_pool_name = common::unique_pool_name("hash-scan-breaker-short");

	write_stale_prefix_with_fresh_tail(&test_deps, namespace_id, &short_pool_name).await?;
	let before = metrics::ENVOY_LB_SCAN_CIRCUIT_BREAKER_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &short_pool_name, "hash"])
		.get();
	let (short_allocation, short_read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&short_pool_name,
		1,
		8,
		vec![common::hash_pos(0)],
		0,
	)
	.await?;
	let after = metrics::ENVOY_LB_SCAN_CIRCUIT_BREAKER_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &short_pool_name, "hash"])
		.get();

	assert_eq!(short_allocation, None);
	assert_eq!(short_read_stats.last_ping_ts_reads, 8);
	assert_eq!(after - before, 1);

	let long_pool_name = common::unique_pool_name("hash-scan-breaker-long");
	write_stale_prefix_with_fresh_tail(&test_deps, namespace_id, &long_pool_name).await?;
	let (long_allocation, long_read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&long_pool_name,
		1,
		64,
		vec![common::hash_pos(0)],
		0,
	)
	.await?;

	assert_eq!(long_allocation.as_deref(), Some("fresh-tail"));
	assert_eq!(long_read_stats.last_ping_ts_reads, 51);

	Ok(())
}

async fn write_stale_prefix_with_fresh_tail(
	test_deps: &rivet_test_deps::TestDeps,
	namespace_id: Id,
	pool_name: &str,
) -> Result<()> {
	for i in 1..=50 {
		common::write_hash_envoy(
			test_deps,
			namespace_id,
			pool_name,
			&format!("stale-{i}"),
			common::stale_ping_ts(),
			vec![common::hash_pos(i)],
			Some(0),
		)
		.await?;
	}
	common::write_hash_envoy(
		test_deps,
		namespace_id,
		pool_name,
		"fresh-tail",
		common::fresh_ping_ts(),
		vec![common::hash_pos(100)],
		Some(0),
	)
	.await
}
