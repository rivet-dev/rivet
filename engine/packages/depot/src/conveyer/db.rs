use std::{
	collections::{BTreeMap, VecDeque},
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::{Context, Result};
use futures_util::future::BoxFuture;
use gas::prelude::Id;
use rivet_pools::NodeId;
use tokio::sync::RwLock;
use universaldb::Database;

#[cfg(feature = "test-faults")]
use crate::fault::DepotFaultController;
use crate::{cold_tier::ColdTier, workflows::compaction::DeltasAvailable};

use super::{
	branch,
	constants::{ACCESS_TOUCH_THROTTLE_MS, MAX_FORK_DEPTH},
	error::SqliteStorageError,
	keys,
	read::cache_fill::{ShardCacheFillOptions, ShardCacheFillQueue},
	types::{BucketId, ColdManifestChunk, DatabaseBranchId},
};

const COLD_MANIFEST_CACHE_BRANCHES: usize = 16;

pub type CompactionSignaler =
	Arc<dyn Fn(DeltasAvailable) -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[derive(Debug, Clone)]
pub(super) struct CachedColdManifest {
	pub chunks: Vec<ColdManifestChunk>,
}

#[derive(Debug, Default)]
pub(super) struct ColdManifestCache {
	entries: BTreeMap<DatabaseBranchId, CachedColdManifest>,
	lru: VecDeque<DatabaseBranchId>,
}

impl ColdManifestCache {
	pub(super) fn get(&mut self, branch_id: DatabaseBranchId) -> Option<CachedColdManifest> {
		let manifest = self.entries.get(&branch_id)?.clone();
		self.touch(branch_id);
		Some(manifest)
	}

	pub(super) fn insert(&mut self, branch_id: DatabaseBranchId, manifest: CachedColdManifest) {
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

	fn touch(&mut self, branch_id: DatabaseBranchId) {
		self.lru.retain(|cached| *cached != branch_id);
		self.lru.push_back(branch_id);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BranchAncestry {
	pub root_branch_id: DatabaseBranchId,
	pub ancestors: Vec<BranchAncestor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BranchAncestor {
	pub branch_id: DatabaseBranchId,
	pub parent_versionstamp_cap: Option<[u8; 16]>,
}

impl BranchAncestry {
	pub(super) fn root(branch_id: DatabaseBranchId) -> Self {
		Self {
			root_branch_id: branch_id,
			ancestors: vec![BranchAncestor {
				branch_id,
				parent_versionstamp_cap: None,
			}],
		}
	}
}

#[derive(Debug, Clone)]
pub(super) struct CacheSnapshot {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) ancestors: BranchAncestry,
	pub(super) last_access_bucket: Option<i64>,
	pub(super) pidx: Arc<super::page_index::DeltaPageIndex>,
}

pub struct Db {
	pub(super) udb: Arc<Database>,
	pub(super) bucket_id: Id,
	pub(super) database_id: String,
	pub(super) node_id: NodeId,
	/// Cached branch read state. This is a perf cache; FDB remains the source of truth.
	pub(super) cache_snapshot: RwLock<Option<CacheSnapshot>>,
	pub(super) cold_tier: Option<Arc<dyn ColdTier>>,
	pub(super) cold_manifest_cache: RwLock<ColdManifestCache>,
	/// Cached `/META/quota`. Loaded once on the first UDB tx.
	pub(super) storage_used: RwLock<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: AtomicU64,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: AtomicU64,
	/// Last wall-clock time this database sent a workflow compaction wakeup.
	pub(super) last_deltas_available_at_ms: RwLock<Option<i64>>,
	pub(super) compaction_signaler: Option<CompactionSignaler>,
	pub(super) shard_cache_fill: ShardCacheFillQueue,
	#[cfg(feature = "test-faults")]
	pub(super) fault_controller: Option<DepotFaultController>,
}

impl Db {
	pub fn new(udb: Arc<Database>, bucket_id: Id, database_id: String, node_id: NodeId) -> Self {
		Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			None,
			None,
			ShardCacheFillOptions::default(),
		)
	}

	pub fn new_with_cold_tier(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Arc<dyn ColdTier>,
	) -> Self {
		Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			Some(cold_tier),
			None,
			ShardCacheFillOptions::default(),
		)
	}

	pub fn new_with_compaction_signaler(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Option<Arc<dyn ColdTier>>,
		compaction_signaler: CompactionSignaler,
	) -> Self {
		Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			cold_tier,
			Some(compaction_signaler),
			ShardCacheFillOptions::default(),
		)
	}

	#[cfg(feature = "test-faults")]
	pub fn new_with_fault_controller_for_test(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		fault_controller: DepotFaultController,
	) -> Self {
		let mut db = Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			None,
			None,
			ShardCacheFillOptions::default(),
		);
		db.fault_controller = Some(fault_controller);
		db
	}

	#[cfg(feature = "test-faults")]
	pub fn new_with_cold_tier_and_fault_controller_for_test(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Arc<dyn ColdTier>,
		fault_controller: DepotFaultController,
	) -> Self {
		let mut db = Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			Some(cold_tier),
			None,
			ShardCacheFillOptions::default(),
		);
		db.fault_controller = Some(fault_controller);
		db
	}

	#[cfg(feature = "test-faults")]
	pub fn new_with_compaction_signaler_and_fault_controller_for_test(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Option<Arc<dyn ColdTier>>,
		compaction_signaler: CompactionSignaler,
		fault_controller: DepotFaultController,
	) -> Self {
		let mut db = Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			cold_tier,
			Some(compaction_signaler),
			ShardCacheFillOptions::default(),
		);
		db.fault_controller = Some(fault_controller);
		db
	}

	#[cfg(debug_assertions)]
	pub fn new_with_cold_tier_and_shard_cache_fill_limits_for_test(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Arc<dyn ColdTier>,
		queue_capacity: usize,
		worker_count: usize,
	) -> Self {
		Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			Some(cold_tier),
			None,
			ShardCacheFillOptions {
				queue_capacity,
				worker_count,
			},
		)
	}

	fn new_inner(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		cold_tier: Option<Arc<dyn ColdTier>>,
		compaction_signaler: Option<CompactionSignaler>,
		shard_cache_fill_options: ShardCacheFillOptions,
	) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile_nonblocking(udb.clone(), database_id.clone(), node_id);

		let mut shard_cache_fill_options = shard_cache_fill_options;
		if cold_tier.is_none() {
			shard_cache_fill_options.worker_count = 0;
		}
		let shard_cache_fill =
			ShardCacheFillQueue::new(udb.clone(), node_id, shard_cache_fill_options);

		Self {
			udb,
			bucket_id,
			database_id,
			node_id,
			cache_snapshot: RwLock::new(None),
			cold_tier,
			cold_manifest_cache: RwLock::new(ColdManifestCache::default()),
			storage_used: RwLock::new(None),
			commit_bytes_since_rollup: AtomicU64::new(0),
			read_bytes_since_rollup: AtomicU64::new(0),
			last_deltas_available_at_ms: RwLock::new(None),
			compaction_signaler,
			shard_cache_fill,
			#[cfg(feature = "test-faults")]
			fault_controller: None,
		}
	}

	pub(super) fn sqlite_bucket_id(&self) -> BucketId {
		BucketId::from_gas_id(self.bucket_id)
	}

	pub fn take_metering_snapshot(&self) -> (u64, u64) {
		(
			self.commit_bytes_since_rollup.swap(0, Ordering::Relaxed),
			self.read_bytes_since_rollup.swap(0, Ordering::Relaxed),
		)
	}

	#[cfg(debug_assertions)]
	pub async fn wait_for_shard_cache_fill_idle_for_test(&self) {
		self.shard_cache_fill.wait_idle_for_test().await
	}

	#[cfg(debug_assertions)]
	pub fn shard_cache_fill_outstanding_for_test(&self) -> usize {
		self.shard_cache_fill.outstanding_for_test()
	}

	#[cfg(debug_assertions)]
	pub fn set_shard_cache_fill_outstanding_for_test(&self, outstanding: usize) {
		self.shard_cache_fill.set_outstanding_for_test(outstanding);
	}

	#[cfg(debug_assertions)]
	pub fn complete_one_shard_cache_fill_outstanding_for_test(&self) {
		self.shard_cache_fill.complete_one_outstanding_for_test();
	}

	#[cfg(debug_assertions)]
	pub fn set_shard_cache_fill_after_nonzero_load_hook_for_test(
		&self,
		hook: Arc<dyn Fn() + Send + Sync>,
	) {
		self.shard_cache_fill
			.set_after_nonzero_load_hook_for_test(hook);
	}

	#[cfg(debug_assertions)]
	pub async fn branch_cache_snapshot_for_test(
		&self,
	) -> Option<(
		DatabaseBranchId,
		DatabaseBranchId,
		Option<i64>,
		Vec<(u32, u64)>,
	)> {
		let snapshot = self.cache_snapshot.read().await.clone()?;
		Some((
			snapshot.branch_id,
			snapshot.ancestors.root_branch_id,
			snapshot.last_access_bucket,
			snapshot.pidx.range(0, u32::MAX),
		))
	}

	#[cfg(debug_assertions)]
	pub async fn fill_shard_cache_once_for_test(
		&self,
		branch_id: DatabaseBranchId,
		reference: super::types::ColdShardRef,
		object_bytes: Vec<u8>,
	) -> Result<()> {
		self.shard_cache_fill
			.fill_once_for_test(branch_id, reference, object_bytes)
			.await
	}
}

pub(super) fn access_bucket(now_ms: i64) -> i64 {
	now_ms.div_euclid(ACCESS_TOUCH_THROTTLE_MS)
}

pub(super) async fn touch_access_if_bucket_advanced(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
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
		.get(
			&bucket_key,
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
		.map(|bytes| decode_i64_le(&bytes))
		.transpose()
		.context("decode sqlite last access bucket")?;
	if stored_bucket == Some(bucket) {
		return Ok(Some(bucket));
	}

	tx.informal().set(
		&keys::branch_manifest_last_access_ts_ms_key(branch_id),
		&now_ms.to_le_bytes(),
	);
	tx.informal().set(&bucket_key, &bucket.to_le_bytes());

	Ok(Some(bucket))
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use super::*;

	pub async fn touch_access_if_bucket_advanced_for_test(
		tx: &universaldb::Transaction,
		branch_id: DatabaseBranchId,
		cached_bucket: Option<i64>,
		now_ms: i64,
	) -> Result<Option<i64>> {
		touch_access_if_bucket_advanced(tx, branch_id, cached_bucket, now_ms).await
	}
}

fn decode_i64_le(bytes: &[u8]) -> Result<i64> {
	let bytes: [u8; std::mem::size_of::<i64>()] = bytes.try_into().with_context(|| {
		format!(
			"sqlite access bucket should be exactly 8 bytes, got {}",
			bytes.len()
		)
	})?;

	Ok(i64::from_le_bytes(bytes))
}

pub(super) async fn load_branch_ancestry(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<BranchAncestry> {
	let mut ancestors = vec![BranchAncestor {
		branch_id,
		parent_versionstamp_cap: None,
	}];
	let mut current_branch_id = branch_id;

	for depth in 0..=MAX_FORK_DEPTH {
		let record = branch::read_database_branch_record(tx, current_branch_id).await?;
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
			.context("sqlite database branch parent versionstamp is missing")?;
		ancestors.push(BranchAncestor {
			branch_id: parent_branch_id,
			parent_versionstamp_cap: Some(parent_versionstamp),
		});
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::ForkChainTooDeep.into())
}
