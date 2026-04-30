use std::{
	collections::{BTreeMap, VecDeque},
	sync::Arc,
};

use anyhow::{Context, Result};
use gas::prelude::Id;
use parking_lot::Mutex;
use rivet_pools::NodeId;
use tokio::time::Instant;
use universaldb::Database;

use crate::{
	cold_tier::ColdTier,
	compactor::Ups,
	page_index::DeltaPageIndex,
};

use super::{
	branch,
	constants::{ACCESS_TOUCH_THROTTLE_MS, MAX_FORK_DEPTH},
	error::SqliteStorageError,
	keys,
	types::{ActorBranchId, ColdManifestChunk, NamespaceId},
};

const COLD_MANIFEST_CACHE_BRANCHES: usize = 16;

#[derive(Debug, Clone)]
pub(super) struct CachedColdManifest {
	pub chunks: Vec<ColdManifestChunk>,
}

#[derive(Debug, Default)]
pub(super) struct ColdManifestCache {
	entries: BTreeMap<ActorBranchId, CachedColdManifest>,
	lru: VecDeque<ActorBranchId>,
}

impl ColdManifestCache {
	pub(super) fn get(&mut self, branch_id: ActorBranchId) -> Option<CachedColdManifest> {
		let manifest = self.entries.get(&branch_id)?.clone();
		self.touch(branch_id);
		Some(manifest)
	}

	pub(super) fn insert(
		&mut self,
		branch_id: ActorBranchId,
		manifest: CachedColdManifest,
	) {
		self.entries.insert(branch_id, manifest);
		self.touch(branch_id);

		while self.entries.len() > COLD_MANIFEST_CACHE_BRANCHES {
			let Some(evict) = self.lru.pop_front() else {
				break;
			};
			if self.entries.remove(&evict).is_some() {
				self.lru.retain(|cached| *cached != evict);
			}
		}
	}

	fn touch(&mut self, branch_id: ActorBranchId) {
		self.lru.retain(|cached| *cached != branch_id);
		self.lru.push_back(branch_id);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BranchAncestry {
	pub root_branch_id: ActorBranchId,
	pub ancestors: Vec<BranchAncestor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BranchAncestor {
	pub branch_id: ActorBranchId,
	pub parent_versionstamp_cap: Option<[u8; 16]>,
}

impl BranchAncestry {
	pub(super) fn root(branch_id: ActorBranchId) -> Self {
		Self {
			root_branch_id: branch_id,
			ancestors: vec![BranchAncestor {
				branch_id,
				parent_versionstamp_cap: None,
			}],
		}
	}
}

#[allow(dead_code)]
pub struct ActorDb {
	pub(super) udb: Arc<Database>,
	pub(super) ups: Ups,
	pub(super) namespace_id: Id,
	pub(super) actor_id: String,
	pub(super) node_id: NodeId,
	/// Cached actor branch id. This is a perf cache; FDB remains the source of truth.
	pub(super) branch_id: Mutex<Option<ActorBranchId>>,
	/// Cached immutable branch ancestry. This is a perf cache for read planning.
	pub(super) ancestors: Mutex<Option<BranchAncestry>>,
	pub(super) cold_tier: Option<Arc<dyn ColdTier>>,
	pub(super) cold_manifest_cache: Mutex<ColdManifestCache>,
	/// Cached access bucket for throttling manifest and eviction-index touches.
	pub(super) last_access_bucket: Mutex<Option<i64>>,
	pub(super) cache: Mutex<DeltaPageIndex>,
	/// Cached `/META/quota`. Loaded once on the first UDB tx.
	pub(super) storage_used: Mutex<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: Mutex<u64>,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: Mutex<u64>,
	/// Last time this actor published a compaction trigger.
	pub(super) last_trigger_at: Mutex<Option<Instant>>,
}

impl ActorDb {
	pub fn new(
		udb: Arc<Database>,
		ups: Ups,
		namespace_id: Id,
		actor_id: String,
		node_id: NodeId,
	) -> Self {
		Self::new_inner(udb, ups, namespace_id, actor_id, node_id, None)
	}

	pub fn new_with_cold_tier(
		udb: Arc<Database>,
		ups: Ups,
		namespace_id: Id,
		actor_id: String,
		node_id: NodeId,
		cold_tier: Arc<dyn ColdTier>,
	) -> Self {
		Self::new_inner(udb, ups, namespace_id, actor_id, node_id, Some(cold_tier))
	}

	fn new_inner(
		udb: Arc<Database>,
		ups: Ups,
		namespace_id: Id,
		actor_id: String,
		node_id: NodeId,
		cold_tier: Option<Arc<dyn ColdTier>>,
	) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile_blocking(udb.clone(), actor_id.clone(), node_id);

		Self {
			udb,
			ups,
			namespace_id,
			actor_id,
			node_id,
			branch_id: Mutex::new(None),
			ancestors: Mutex::new(None),
			cold_tier,
			cold_manifest_cache: Mutex::new(ColdManifestCache::default()),
			last_access_bucket: Mutex::new(None),
			cache: Mutex::new(DeltaPageIndex::new()),
			storage_used: Mutex::new(None),
			commit_bytes_since_rollup: Mutex::new(0),
			read_bytes_since_rollup: Mutex::new(0),
			last_trigger_at: Mutex::new(None),
		}
	}

	pub(super) fn sqlite_namespace_id(&self) -> NamespaceId {
		NamespaceId::from_gas_id(self.namespace_id)
	}

	pub fn take_metering_snapshot(&self) -> (u64, u64) {
		let mut commit_bytes = self.commit_bytes_since_rollup.lock();
		let mut read_bytes = self.read_bytes_since_rollup.lock();
		let snapshot = (*commit_bytes, *read_bytes);

		*commit_bytes = 0;
		*read_bytes = 0;

		snapshot
	}
}

pub(super) fn access_bucket(now_ms: i64) -> i64 {
	now_ms.div_euclid(ACCESS_TOUCH_THROTTLE_MS)
}

pub(super) async fn touch_access_if_bucket_advanced(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	cached_bucket: Option<i64>,
	now_ms: i64,
) -> Result<Option<i64>> {
	let bucket = access_bucket(now_ms);
	if cached_bucket == Some(bucket) {
		return Ok(None);
	}

	let bucket_key = keys::branch_manifest_last_access_bucket_key(branch_id);
	let stored_bucket = tx
		.informal()
		.get(&bucket_key, universaldb::utils::IsolationLevel::Serializable)
		.await?
		.map(|bytes| decode_i64_le(&bytes))
		.transpose()
		.context("decode sqlite last access bucket")?;
	if stored_bucket == Some(bucket) {
		return Ok(Some(bucket));
	}

	if let Some(stored_bucket) = stored_bucket {
		tx.informal()
			.clear(&keys::ctr_eviction_index_key(stored_bucket, branch_id));
	}
	tx.informal()
		.set(&keys::branch_manifest_last_access_ts_ms_key(branch_id), &now_ms.to_le_bytes());
	tx.informal().set(&bucket_key, &bucket.to_le_bytes());
	tx.informal()
		.set(&keys::ctr_eviction_index_key(bucket, branch_id), &[]);

	Ok(Some(bucket))
}

fn decode_i64_le(bytes: &[u8]) -> Result<i64> {
	Ok(i64::from_le_bytes(
		bytes
			.try_into()
			.context("sqlite access bucket should be exactly 8 bytes")?,
	))
}

pub(super) async fn load_branch_ancestry(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<BranchAncestry> {
	let mut ancestors = vec![BranchAncestor {
		branch_id,
		parent_versionstamp_cap: None,
	}];
	let mut current_branch_id = branch_id;

	for depth in 0..=MAX_FORK_DEPTH {
		let record = branch::read_actor_branch_record(tx, current_branch_id).await?;
		let Some(parent_branch_id) = record.parent else {
			return Ok(BranchAncestry {
				root_branch_id: branch_id,
				ancestors,
			});
		};
		if depth == MAX_FORK_DEPTH {
			return Err(SqliteStorageError::ForkChainTooDeep.into());
		}

		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite actor branch parent versionstamp is missing")?;
		ancestors.push(BranchAncestor {
			branch_id: parent_branch_id,
			parent_versionstamp_cap: Some(parent_versionstamp),
		});
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::ForkChainTooDeep.into())
}
