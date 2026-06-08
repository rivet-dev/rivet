mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_allocator_returns_none_when_all_entries_are_stale() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-all-stale");

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"stale-a",
		common::stale_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(0),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"stale-b",
		common::stale_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(0),
	)
	.await?;

	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		2,
		8,
		vec![common::hash_pos(0x10), common::hash_pos(0x20)],
		0,
	)
	.await?;
	assert_eq!(allocation, None);

	Ok(())
}
