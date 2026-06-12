use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "pidx-cache")]
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};

#[cfg(feature = "pidx-cache")]
use crate::conveyer::page_index::DeltaPageIndex;
use crate::conveyer::types::DatabaseBranchId;

use super::plan::{ReadSource, StorageScope};

#[cfg(feature = "pidx-cache")]
pub(super) fn snapshot_pidx_cache(
	cache: &DeltaPageIndex,
	pgnos: &[u32],
) -> Option<BTreeMap<u32, Option<u64>>> {
	let cached_rows = cache.range(0, u32::MAX);
	if cached_rows.is_empty() {
		None
	} else {
		Some(
			pgnos
				.iter()
				.map(|pgno| (*pgno, cache.get(*pgno)))
				.collect::<BTreeMap<_, _>>(),
		)
	}
}

pub(super) fn cache_source_for_scope(scope: &StorageScope) -> Option<ReadSource> {
	match scope {
		StorageScope::Branch(plan) if plan.sources.len() == 1 => Some(plan.sources[0]),
		StorageScope::Branch(_) => None,
	}
}

#[cfg(feature = "pidx-cache")]
pub(super) fn store_loaded_pidx_rows(
	cache: &DeltaPageIndex,
	loaded_pidx_rows: Vec<(u32, u64)>,
	stale_pidx_pgnos: &BTreeSet<u32>,
) {
	let loaded_index = DeltaPageIndex::new();
	for (pgno, txid) in loaded_pidx_rows {
		if !stale_pidx_pgnos.contains(&pgno) {
			loaded_index.insert(pgno, txid);
		}
	}

	for (pgno, txid) in loaded_index.range(0, u32::MAX) {
		cache.insert(pgno, txid);
	}
}

#[cfg(feature = "pidx-cache")]
pub(super) fn clear_stale_pidx_rows(cache: &DeltaPageIndex, stale_pidx_pgnos: BTreeSet<u32>) {
	for pgno in stale_pidx_pgnos {
		cache.remove(pgno);
	}
}

pub(super) fn branch_cache_changed(
	cached_branch_id: Option<DatabaseBranchId>,
	branch_id: DatabaseBranchId,
) -> bool {
	cached_branch_id.is_some_and(|cached_branch_id| cached_branch_id != branch_id)
}

pub(super) fn now_ms() -> Result<i64> {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system time is before unix epoch")?;
	i64::try_from(duration.as_millis()).context("current time exceeded i64 milliseconds")
}
