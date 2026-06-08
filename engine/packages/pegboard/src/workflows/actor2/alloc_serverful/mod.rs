use gas::prelude::*;
use rivet_config::config::pegboard::EnvoyLoadBalancer;
use std::time::Instant;
use universaldb::prelude::*;

use super::HashAllocatorReadStats;

mod hash;
mod newest_ping_timestamp;
mod random_full_range;
mod random_ping_timestamp;

pub(super) async fn allocate_serverful(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	now: i64,
	envoy_eligible_threshold: i64,
	envoy_load_balancer: &EnvoyLoadBalancer,
) -> Result<Option<String>> {
	let start = Instant::now();
	let mut stats = AllocStats::default();
	// `emits_scan_stats` is false for strategies that do not walk the index
	// (newest_ping_timestamp short-reads the head, so scan_depth/wrap_count
	// would always be 0). The cheap stable labels (alloc/no_envoy/duration)
	// still fire for every strategy.
	let (strategy, emits_scan_stats, allocation) = match envoy_load_balancer {
		EnvoyLoadBalancer::RandomPingTimestamp { use_snapshot_read } => (
			"random_ping_timestamp",
			true,
			random_ping_timestamp::allocate(
				namespace_id,
				pool_name,
				tx,
				pools,
				&mut stats,
				now,
				envoy_eligible_threshold,
				*use_snapshot_read,
			)
			.await?,
		),
		EnvoyLoadBalancer::NewestPingTimestamp { use_snapshot_read } => (
			"newest_ping_timestamp",
			false,
			newest_ping_timestamp::allocate(
				namespace_id,
				pool_name,
				tx,
				pools,
				now,
				envoy_eligible_threshold,
				*use_snapshot_read,
			)
			.await?,
		),
		EnvoyLoadBalancer::RandomFullRange { use_snapshot_read } => (
			"random_full_range",
			true,
			random_full_range::allocate(
				namespace_id,
				pool_name,
				tx,
				pools,
				&mut stats,
				now,
				envoy_eligible_threshold,
				*use_snapshot_read,
			)
			.await?,
		),
		EnvoyLoadBalancer::Hash {
			virtual_nodes: _,
			samples,
			max_scan,
			slot_jitter,
			use_snapshot_read,
		} => (
			"hash",
			true,
			hash::allocate(
				namespace_id,
				pool_name,
				tx,
				pools,
				&mut stats,
				now,
				envoy_eligible_threshold,
				*samples,
				*max_scan,
				*slot_jitter,
				*use_snapshot_read,
			)
			.await?,
		),
	};

	let namespace_id_str = namespace_id.to_string();
	let labels = [namespace_id_str.as_str(), pool_name, strategy];
	if allocation.is_some() {
		crate::metrics::ENVOY_LB_ALLOCATION_TOTAL
			.with_label_values(&labels)
			.inc();
	} else {
		crate::metrics::ENVOY_LB_NO_ENVOY_AVAILABLE_TOTAL
			.with_label_values(&labels)
			.inc();
	}
	crate::metrics::ENVOY_LB_ALLOC_DURATION
		.with_label_values(&labels)
		.observe(start.elapsed().as_secs_f64());
	if emits_scan_stats {
		crate::metrics::ENVOY_LB_SCAN_DEPTH
			.with_label_values(&labels)
			.observe(stats.scan_depth as f64);
		if stats.wrap_count > 0 {
			crate::metrics::ENVOY_LB_WRAP_TOTAL
				.with_label_values(&labels)
				.inc_by(stats.wrap_count as u64);
		}
	}

	Ok(allocation)
}

pub(super) async fn allocate_hash_for_tests(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	now: i64,
	envoy_eligible_threshold: i64,
	samples: u8,
	max_scan: u32,
	slot_jitter: u8,
	use_snapshot_read: bool,
	pivots: Vec<[u8; 16]>,
	rng_seed: u64,
) -> Result<(Option<String>, HashAllocatorReadStats)> {
	let mut stats = AllocStats::default();
	let mut read_stats = HashAllocatorReadStats::default();
	let mut rng = hash::SeededHashAllocatorRng::new(pivots, rng_seed);
	let allocation = hash::allocate_with_rng(
		namespace_id,
		pool_name,
		tx,
		pools,
		&mut stats,
		now,
		envoy_eligible_threshold,
		samples,
		max_scan,
		slot_jitter,
		use_snapshot_read,
		&mut rng,
		Some(&mut read_stats),
	)
	.await?;

	Ok((allocation, read_stats))
}

pub(super) fn read_isolation(use_snapshot_read: bool) -> universaldb::utils::IsolationLevel {
	if use_snapshot_read {
		Snapshot
	} else {
		Serializable
	}
}

#[derive(Default)]
pub(super) struct AllocStats {
	scan_depth: u32,
	wrap_count: u32,
}

impl AllocStats {
	pub(super) fn scanned_one(&mut self) {
		self.scan_depth = self.scan_depth.saturating_add(1);
	}

	pub(super) fn wrapped(&mut self) {
		self.wrap_count = self.wrap_count.saturating_add(1);
	}
}
