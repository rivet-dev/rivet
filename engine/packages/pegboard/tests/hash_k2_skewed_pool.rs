mod common;

use anyhow::{Context, Result};
use gas::prelude::*;

const ENVOY_COUNT: usize = 100;
const VIRTUAL_NODE_COUNT: usize = 8;
const ALLOCATION_COUNT: usize = 1_000;
const LOADED_ENVOY_INDEX: usize = 0;

#[tokio::test]
async fn hash_k2_skewed_pool() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k2-skewed-pool");
	let envoys = (0..ENVOY_COUNT)
		.map(|envoy_index| common::HashEnvoyRegistration {
			envoy_key: common::deterministic_envoy_key(envoy_index),
			hash_positions: common::deterministic_hash_positions(
				envoy_index,
				ENVOY_COUNT,
				VIRTUAL_NODE_COUNT,
			),
			slots: Some(if envoy_index == LOADED_ENVOY_INDEX {
				50
			} else {
				0
			}),
		})
		.collect();
	common::write_hash_envoys(&test_deps, namespace_id, &pool_name, envoys).await?;

	let allocations = (0..ALLOCATION_COUNT)
		.map(|allocation_index| {
			let quiet_envoy_index = 1 + (allocation_index % (ENVOY_COUNT - 1));
			common::HashAllocationInput {
				pivots: vec![
					common::deterministic_hash_pos(
						LOADED_ENVOY_INDEX,
						0,
						ENVOY_COUNT,
						VIRTUAL_NODE_COUNT,
					),
					common::deterministic_hash_pos(
						quiet_envoy_index,
						0,
						ENVOY_COUNT,
						VIRTUAL_NODE_COUNT,
					),
				],
				rng_seed: allocation_index as u64,
			}
		})
		.collect();

	let results =
		common::allocate_hash_batch(&test_deps, namespace_id, &pool_name, 2, 8, allocations)
			.await?;
	let counts = common::count_deterministic_envoy_allocations(results, ENVOY_COUNT)?;
	let loaded_allocations = *counts
		.get(LOADED_ENVOY_INDEX)
		.context("expected loaded envoy index in range")?;

	assert!(
		loaded_allocations * 100 < ALLOCATION_COUNT,
		"expected loaded envoy to receive < 1% of allocations, got {loaded_allocations}/{ALLOCATION_COUNT}"
	);

	Ok(())
}
