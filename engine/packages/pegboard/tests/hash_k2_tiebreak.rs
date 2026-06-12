mod common;

use anyhow::Result;
use gas::prelude::*;
use std::collections::BTreeSet;

#[tokio::test]
async fn hash_k2_tiebreak_can_choose_either_equal_slot_candidate() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k2-tiebreak");

	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"envoy-a",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x10)],
		Some(3),
	)
	.await?;
	common::write_hash_envoy(
		&test_deps,
		namespace_id,
		&pool_name,
		"envoy-b",
		common::fresh_ping_ts(),
		vec![common::hash_pos(0x20)],
		Some(3),
	)
	.await?;

	let mut chosen = BTreeSet::new();
	for seed in 0..128 {
		let (allocation, _) = common::allocate_hash(
			&test_deps,
			namespace_id,
			&pool_name,
			2,
			8,
			vec![common::hash_pos(0x10), common::hash_pos(0x20)],
			seed,
		)
		.await?;
		let Some(allocation) = allocation else {
			panic!("expected hash allocator to choose one tied candidate");
		};
		chosen.insert(allocation);
		if chosen.len() == 2 {
			break;
		}
	}

	assert_eq!(chosen.len(), 2);
	assert!(chosen.contains("envoy-a"));
	assert!(chosen.contains("envoy-b"));

	Ok(())
}
