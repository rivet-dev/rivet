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

	let subspace = keys::subspace().subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace(
		namespace_id,
		pool_name.to_string(),
	));
	let mut stream = tx.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(&subspace).into()
		},
		read_isolation,
	);

	let mut highest_version = None;
	let mut candidates = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		stats.scanned_one();
		let (lb_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

		if let Some(highest_version) = highest_version {
			if lb_key.version < highest_version {
				break;
			}
		} else {
			highest_version = Some(lb_key.version);
		}

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

		candidates.push(lb_key.envoy_key);
	}

	if candidates.is_empty() {
		Ok(None)
	} else {
		let index = rand::thread_rng().gen_range(0..candidates.len());
		Ok(Some(candidates.swap_remove(index)))
	}
}
