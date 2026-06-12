use anyhow::Result;
use gas::prelude::*;
use rivet_config::config::pegboard::EnvoyLoadBalancer;

use crate::common;

#[tokio::test]
async fn envoy_hash_k2_reads_slots_and_picks_the_minimum() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("envoy-hash-k2");
	let strategy = EnvoyLoadBalancer::Hash {
		virtual_nodes: 1,
		samples: 2,
		max_scan: 8,
		slot_jitter: 0,
		use_snapshot_read: true,
	};
	let EnvoyLoadBalancer::Hash {
		samples, max_scan, ..
	} = strategy
	else {
		unreachable!("strategy is constructed as Hash");
	};

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"busy-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(7),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"quiet-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(1),
	)
	.await?;

	let (allocation, read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		samples,
		max_scan,
		vec![common::hash_pos(0x10), common::hash_pos(0x20)],
		0,
	)
	.await?;

	assert_eq!(allocation.as_deref(), Some("quiet-envoy"));
	assert_eq!(read_stats.slots_reads, 2);

	Ok(())
}
