mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_allocator_wraps_past_last_position_to_first_envoy() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-wrap");

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"first-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(0),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"second-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(0),
	)
	.await?;

	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		1,
		8,
		vec![common::hash_pos(u128::MAX)],
		0,
	)
	.await?;
	assert_eq!(allocation.as_deref(), Some("first-envoy"));

	Ok(())
}
