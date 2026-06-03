mod common;

use anyhow::Result;
use gas::prelude::*;

#[tokio::test]
async fn hash_allocator_returns_none_when_subspace_is_empty() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-empty");

	let (k1_allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		1,
		8,
		vec![common::hash_pos(0)],
		0,
	)
	.await?;
	assert_eq!(k1_allocation, None);

	let (k2_allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		2,
		8,
		vec![common::hash_pos(0), common::hash_pos(1)],
		0,
	)
	.await?;
	assert_eq!(k2_allocation, None);

	Ok(())
}
