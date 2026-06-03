mod common;

use anyhow::{Context, Result};
use gas::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

const ENVOY_COUNT: usize = 100;
const VIRTUAL_NODE_COUNT: usize = 8;
const ALLOCATION_COUNT: usize = 1_000;
const PAIR_COUNT: usize = ENVOY_COUNT / 2;
const REPEATS_PER_PAIR: usize = ALLOCATION_COUNT / PAIR_COUNT;

#[tokio::test]
async fn hash_k2_cold_pool() -> Result<()> {
	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-k2-cold-pool");
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

	let mut allocations = Vec::with_capacity(ALLOCATION_COUNT);
	for pair_index in 0..PAIR_COUNT {
		for repeat_index in 0..REPEATS_PER_PAIR {
			let desired_tied_index = repeat_index % 2;
			allocations.push(common::HashAllocationInput {
				pivots: vec![
					common::deterministic_hash_pos(pair_index, 0, ENVOY_COUNT, VIRTUAL_NODE_COUNT),
					common::deterministic_hash_pos(
						pair_index + PAIR_COUNT,
						0,
						ENVOY_COUNT,
						VIRTUAL_NODE_COUNT,
					),
				],
				rng_seed: seed_for_tied_index(
					desired_tied_index,
					(pair_index * REPEATS_PER_PAIR + repeat_index) as u64,
				),
			});
		}
	}

	let results =
		common::allocate_hash_batch(&test_deps, namespace_id, &pool_name, 2, 8, allocations)
			.await?;
	let counts = common::count_deterministic_envoy_allocations(results, ENVOY_COUNT)?;
	let max_load = *counts.iter().max().context("expected at least one envoy")?;
	let max_allowed = ALLOCATION_COUNT.div_ceil(ENVOY_COUNT) + 2;

	assert!(
		max_load <= max_allowed,
		"expected max load <= {max_allowed}, got {max_load}; counts={counts:?}"
	);

	Ok(())
}

fn seed_for_tied_index(desired_tied_index: usize, salt: u64) -> u64 {
	let mut seed = salt << 8;
	loop {
		let mut rng = StdRng::seed_from_u64(seed);
		if rng.gen_range(0..2) == desired_tied_index {
			return seed;
		}
		seed += 1;
	}
}
