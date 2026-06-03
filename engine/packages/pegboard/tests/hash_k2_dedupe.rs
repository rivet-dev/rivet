mod common;

use anyhow::Result;
use gas::prelude::*;
use pegboard::metrics;

#[tokio::test]
async fn hash_k2_dedupes_samples_that_resolve_to_same_envoy() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let namespace_id_label = namespace_id.to_string();
	let pool_name = common::unique_pool_name("hash-k2-dedupe");

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"envoy-a",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(5),
	)
	.await?;

	let before = metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();
	let (allocation, read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		2,
		8,
		vec![common::hash_pos(0x10), common::hash_pos(0x10)],
		0,
	)
	.await?;
	let after = metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();

	assert_eq!(allocation.as_deref(), Some("envoy-a"));
	assert_eq!(read_stats.slots_reads, 1);
	assert_eq!(after - before, 1);

	Ok(())
}
