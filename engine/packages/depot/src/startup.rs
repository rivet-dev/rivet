//! Bounded, immutable SQLite snapshots for actor Start. Cache presence never grants ownership.
use crate::conveyer::{
	branch, keys,
	types::{BucketId, DBHead, DatabaseBranchId, decode_db_head},
};
use anyhow::Result;
use moka::future::Cache;
use std::{
	collections::BTreeMap,
	sync::{Arc, OnceLock},
};
use universaldb::utils::IsolationLevel::Serializable;

pub fn page_limit() -> u32 {
	static LIMIT: OnceLock<u32> = OnceLock::new();
	*LIMIT.get_or_init(|| {
		std::env::var("RIVET_ACTOR_START_PRELOAD_PAGES")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(0)
			.min(256)
	})
}

type Key = (BucketId, String, DatabaseBranchId, u64);
type Pages = BTreeMap<u32, Vec<u8>>;
fn cache() -> &'static Cache<Key, Arc<Pages>> {
	static CACHE: OnceLock<Cache<Key, Arc<Pages>>> = OnceLock::new();
	CACHE.get_or_init(|| {
		Cache::builder()
			.max_capacity(64 * 1024 * 1024)
			// Charge at least 64 KiB per snapshot to also bound the cache to 1,024 entries.
			.weigher(|_: &Key, pages: &Arc<Pages>| {
				pages.values().map(Vec::len).sum::<usize>().max(64 * 1024) as u32
			})
			.build()
	})
}

/// Read the exact branch/head in the caller's ownership transaction, including read conflicts.
pub async fn read_fence(
	tx: &universaldb::Transaction,
	bucket: BucketId,
	database: &str,
) -> Result<Option<DBHead>> {
	if page_limit() == 0 {
		return Ok(None);
	}
	let Some(branch) = branch::resolve_database_branch(tx, bucket, database, Serializable).await?
	else {
		return Ok(None);
	};
	let informal = tx.informal();
	let head_key = keys::branch_meta_head_key(branch);
	let fork_key = keys::branch_meta_head_at_fork_key(branch);
	let (head, fork) = tokio::try_join!(
		informal.get(&head_key, Serializable),
		informal.get(&fork_key, Serializable),
	)?;
	head.or(fork)
		.map(|raw| {
			let mut h = decode_db_head(&raw)?;
			h.branch_id = branch;
			Ok(h)
		})
		.transpose()
}

/// Cache only bytes read at this exact head. A miss safely falls back to normal Depot reads.
pub async fn remember(
	bucket: BucketId,
	database: &str,
	branch: DatabaseBranchId,
	head: u64,
	size: u32,
	parent: Option<u64>,
	pages: impl IntoIterator<Item = (u32, Vec<u8>)>,
) {
	let limit = page_limit();
	if limit == 0 {
		return;
	}
	let key = (bucket, database.to_owned(), branch, head);
	let cached = match cache().get(&key).await {
		Some(pages) => Some(pages),
		None => match parent {
			Some(parent) => {
				cache()
					.get(&(bucket, database.to_owned(), branch, parent))
					.await
			}
			None => None,
		},
	};
	let mut merged = cached.as_deref().cloned().unwrap_or_default();
	merged.retain(|pgno, _| *pgno <= size && *pgno <= limit);
	for (pgno, bytes) in pages {
		if pgno > 0 && pgno <= size.min(limit) && bytes.len() == 4096 {
			merged.insert(pgno, bytes);
		}
	}
	// Concurrent fills can lose a cache hit, but bytes at an exact head are immutable.
	// Missing pages always use the ordinary Depot read path.
	cache().insert(key, Arc::new(merged)).await;
}

pub async fn assemble(
	bucket: BucketId,
	database: &str,
	branch: DatabaseBranchId,
	head: u64,
	size: u32,
) -> (Pages, u32) {
	let count = size.min(page_limit());
	let pages: Pages = cache()
		.get(&(bucket, database.to_owned(), branch, head))
		.await
		.map(|p| {
			p.range(1..=count.max(1))
				.filter(|(n, _)| **n <= count)
				.map(|(n, b)| (*n, b.clone()))
				.collect()
		})
		.unwrap_or_default();
	let misses = count - pages.len() as u32;
	(pages, misses)
}

#[cfg(test)]
#[path = "../tests/inline/startup.rs"]
mod tests;
