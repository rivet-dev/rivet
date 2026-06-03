use anyhow::Result;
use gas::prelude::*;
use rivet_config::config::pegboard::EnvoyLoadBalancer;

use crate::common;

#[tokio::test]
async fn envoy_hash_k1_allocates_without_reading_slots() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("envoy-hash-k1");
	let strategy = EnvoyLoadBalancer::Hash {
		virtual_nodes: 1,
		samples: 1,
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
		"envoy-a",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(9),
	)
	.await?;

	let (allocation, read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		samples,
		max_scan,
		vec![common::hash_pos(0x10)],
		0,
	)
	.await?;

	assert_eq!(allocation.as_deref(), Some("envoy-a"));
	assert_eq!(read_stats.slots_reads, 0);

	Ok(())
}
