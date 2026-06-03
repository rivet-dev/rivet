use futures_util::TryStreamExt;
use gas::prelude::*;
use rand::Rng;
use universaldb::prelude::*;

use super::{AllocStats, read_isolation};
use crate::{envoy_expire_scheduler, keys};

pub(super) async fn allocate(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	stats: &mut AllocStats,
	now: i64,
	envoy_eligible_threshold: i64,
	use_snapshot_read: bool,
) -> Result<Option<String>> {
	let ping_threshold_ts = now - envoy_eligible_threshold;
	let read_isolation = read_isolation(use_snapshot_read);

	let Some(head_key) = read_oldest_fresh_ping_timestamp_key(
		namespace_id,
		pool_name,
		tx,
		pools,
		stats,
		now,
		envoy_eligible_threshold,
		use_snapshot_read,
	)
	.await?
	else {
		return Ok(None);
	};

	let highest_version = head_key.version;
	let oldest_ping_ts = head_key.last_ping_ts;

	// Choose a random valid timestamp to seek from
	let random_ts = rand::thread_rng().gen_range(oldest_ping_ts..=now);

	let seek_start_key = keys::subspace()
		.subspace(
			&keys::ns::EnvoyLoadBalancerIdxKey::subspace_with_last_ping_ts(
				namespace_id,
				pool_name.to_string(),
				highest_version,
				random_ts,
			),
		)
		.range()
		.0;
	let seek_end_key = keys::subspace()
		.subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace_with_version(
			namespace_id,
			pool_name.to_string(),
			highest_version,
		))
		.range()
		.1;

	let mut seek_stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::Iterator,
			..(seek_start_key.as_slice(), seek_end_key.as_slice()).into()
		},
		read_isolation,
	);

	let candidate = loop {
		let Some(entry) = seek_stream.try_next().await? else {
			break None;
		};
		stats.scanned_one();

		let (lb_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

		// Ignore envoys without valid ping
		if lb_key.last_ping_ts < ping_threshold_ts {
			// Stale envoy. Skip-and-continue: we never return a stale envoy to
			// the allocator caller. Fire-and-forget a BG expire via the
			// read-path scheduler, then continue. No throw, no error. The
			// scheduler invokes pegboard_envoy_expire { skip_if_fresh: true },
			// which re-checks LastPingTsKey + ExpiredTsKey (Serializable)
			// inside its own FDB transaction, closing the TOCTOU window if a
			// heartbeat lands between our observation here and the op's commit.
			envoy_expire_scheduler::get(pools).try_enqueue(namespace_id, lb_key.envoy_key.clone());
			continue;
		}

		break Some(lb_key.envoy_key);
	};
	if candidate.is_none() {
		stats.wrapped();
	}

	// If no valid candidates found fall back to head key because it is valid (The head key likely isn't
	// included in the seek stream).
	Ok(Some(candidate.unwrap_or(head_key.envoy_key)))
}

async fn read_oldest_fresh_ping_timestamp_key(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	stats: &mut AllocStats,
	now: i64,
	envoy_eligible_threshold: i64,
	use_snapshot_read: bool,
) -> Result<Option<keys::ns::EnvoyLoadBalancerIdxKey>> {
	let ping_threshold_ts = now - envoy_eligible_threshold;
	let read_isolation = read_isolation(use_snapshot_read);

	let head_subspace = keys::subspace().subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace(
		namespace_id,
		pool_name.to_string(),
	));
	let mut head_stream = tx.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			..(&head_subspace).into()
		},
		read_isolation,
	);

	let mut highest_version = None;
	while let Some(entry) = head_stream.try_next().await? {
		stats.scanned_one();
		let (lb_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

		if let Some(highest_version) = highest_version {
			if lb_key.version < highest_version {
				return Ok(None);
			}
		} else {
			highest_version = Some(lb_key.version);
		}

		if lb_key.last_ping_ts < ping_threshold_ts {
			// Stale envoy. Skip-and-continue: keep scanning for the oldest
			// fresh head candidate. Fire-and-forget a BG expire via the
			// read-path scheduler, then continue. No throw, no error. The
			// scheduler invokes pegboard_envoy_expire { skip_if_fresh: true },
			// which re-checks LastPingTsKey + ExpiredTsKey (Serializable)
			// inside its own FDB transaction, closing the TOCTOU window if a
			// heartbeat lands between our observation here and the op's commit.
			envoy_expire_scheduler::get(pools).try_enqueue(namespace_id, lb_key.envoy_key.clone());
			continue;
		}

		return Ok(Some(lb_key));
	}

	Ok(None)
}
