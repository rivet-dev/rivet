mod common;

use anyhow::Result;
use gas::prelude::*;
use pegboard::metrics;

#[tokio::test]
async fn hash_k1_does_not_emit_k2_only_metrics() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let namespace_id_label = namespace_id.to_string();
	let pool_name = common::unique_pool_name("hash-k1-no-dedupe-metrics");

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

	let samples_before = metrics::ENVOY_LB_SAMPLES_EFFECTIVE
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get_sample_count();
	let dedupe_before = metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();
	let tied_before = metrics::ENVOY_LB_TIED_MIN_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();

	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		1,
		8,
		vec![common::hash_pos(0x10)],
		0,
	)
	.await?;
	assert_eq!(allocation.as_deref(), Some("envoy-a"));

	let samples_after = metrics::ENVOY_LB_SAMPLES_EFFECTIVE
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get_sample_count();
	let dedupe_after = metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();
	let tied_after = metrics::ENVOY_LB_TIED_MIN_TOTAL
		.with_label_values(&[namespace_id_label.as_str(), &pool_name])
		.get();

	assert_eq!(samples_after - samples_before, 0);
	assert_eq!(dedupe_after - dedupe_before, 0);
	assert_eq!(tied_after - tied_before, 0);

	Ok(())
}
