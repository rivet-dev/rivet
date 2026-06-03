mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_k2_returns_surviving_candidate_when_one_sample_finds_no_fresh_envoy() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k2-partial-failure");

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
		vec![common::hash_pos(0x11)],
		Some(0),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"fresh-envoy",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x80)],
		Some(7),
	)
	.await?;

	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		2,
		2,
		vec![common::hash_pos(0x10), common::hash_pos(0x80)],
		0,
	)
	.await?;
	assert_eq!(allocation.as_deref(), Some("fresh-envoy"));

	Ok(())
}
