mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_k2_picks_candidate_with_fewer_slots() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k2-min");

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"busy-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(5),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"quiet-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(2),
	)
	.await?;

	let (allocation, read_stats) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		2,
		8,
		vec![common::hash_pos(0x10), common::hash_pos(0x20)],
		0,
	)
	.await?;
	assert_eq!(allocation.as_deref(), Some("quiet-envoy"));
	assert_eq!(read_stats.slots_reads, 2);

	Ok(())
}
