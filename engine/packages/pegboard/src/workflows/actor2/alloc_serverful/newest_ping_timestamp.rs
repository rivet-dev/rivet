use futures_util::TryStreamExt;
use gas::prelude::*;
use universaldb::prelude::*;

use super::read_isolation;
use crate::{envoy_expire_scheduler, keys};

pub(super) async fn allocate(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	now: i64,
	envoy_eligible_threshold: i64,
	use_snapshot_read: bool,
) -> Result<Option<String>> {
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

	let Some(entry) = head_stream.try_next().await? else {
		return Ok(None);
	};
	let (head_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

	let version_subspace =
		keys::subspace().subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace_with_version(
			namespace_id,
			pool_name.to_string(),
			head_key.version,
		));
	let mut newest_stream = tx.get_ranges_keyvalues(
		(RangeOption {
			mode: StreamingMode::Iterator,
			limit: Some(1),
			..(&version_subspace).into()
		})
		.rev(),
		read_isolation,
	);

	let Some(entry) = newest_stream.try_next().await? else {
		return Ok(None);
	};
	let (lb_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

	if lb_key.last_ping_ts < ping_threshold_ts {
		// Stale envoy observed at the head of the by-pool index. Fire-and-forget
		// a BG expire via the read-path scheduler so the entry is reclaimed
		// instead of remaining at the head forever and starving the allocator.
		// The scheduler invokes pegboard_envoy_expire { skip_if_fresh: true }
		// which re-checks LastPingTsKey + ExpiredTsKey (Serializable) inside its
		// own FDB transaction, closing the TOCTOU window if a heartbeat lands
		// between our observation here and the op's commit.
		envoy_expire_scheduler::get(pools).try_enqueue(namespace_id, lb_key.envoy_key.clone());
		Ok(None)
	} else {
		Ok(Some(lb_key.envoy_key))
	}
}
