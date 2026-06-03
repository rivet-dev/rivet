mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_k1_reads_last_ping_once_and_never_reads_slots() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k1-short-circuit");

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

	let (allocation, read_stats) = common::allocate_hash(
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
	assert_eq!(read_stats.last_ping_ts_reads, 1);
	assert_eq!(read_stats.slots_reads, 0);

	Ok(())
}
