mod common;

use anyhow::{Context, Result};
use gas::prelude::*;
use rand::{RngCore, SeedableRng, rngs::StdRng};

const ENVOY_COUNT: usize = 100;
const VIRTUAL_NODE_COUNT: usize = 8;
const ALLOCATION_COUNT: usize = 100_000;
const BATCH_SIZE: usize = 5_000;

#[tokio::test]
async fn hash_k1_distribution() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k1-distribution");
	let envoys = (0..ENVOY_COUNT)
		.map(|envoy_index| common::HashEnvoyRegistration {
			envoy_key: common::deterministic_envoy_key(envoy_index),
			hash_positions: common::deterministic_hash_positions(
				envoy_index,
				ENVOY_COUNT,
				VIRTUAL_NODE_COUNT,
			),
			slots: Some(0),
		})
		.collect();
	common::write_hash_envoys(&test_deps, namespace_id, &pool_name, envoys).await?;

	let mut rng = StdRng::seed_from_u64(0x51f0_6d15_7a1b_u64);
	let mut counts = vec![0usize; ENVOY_COUNT];
	let mut remaining = ALLOCATION_COUNT;

	while remaining > 0 {
		let batch_len = remaining.min(BATCH_SIZE);
		let mut allocations = Vec::with_capacity(batch_len);

		for _ in 0..batch_len {
			let mut pivot = [0; 16];
			rng.fill_bytes(&mut pivot);
			allocations.push(common::HashAllocationInput {
				pivots: vec![pivot],
				rng_seed: 0,
			});
		}

		let results =
			common::allocate_hash_batch(&test_deps, namespace_id, &pool_name, 1, 8, allocations)
				.await?;
		let batch_counts = common::count_deterministic_envoy_allocations(results, ENVOY_COUNT)?;
		for (count, batch_count) in counts.iter_mut().zip(batch_counts) {
			*count += batch_count;
		}
		remaining -= batch_len;
	}

	let max_load = *counts.iter().max().context("expected at least one envoy")?;
	let mean_load = ALLOCATION_COUNT as f64 / ENVOY_COUNT as f64;
	let max_mean_ratio = max_load as f64 / mean_load;

	assert!(
		max_mean_ratio <= 2.0,
		"expected max/mean ratio <= 2.0, got {max_mean_ratio:.3}; max={max_load}, mean={mean_load:.1}"
	);

	Ok(())
}
