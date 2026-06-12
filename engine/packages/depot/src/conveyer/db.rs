use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result};
use futures_util::future::BoxFuture;
use gas::prelude::Id;
use rivet_pools::NodeId;
use tokio::sync::RwLock;
use universaldb::Database;

#[cfg(feature = "test-faults")]
use crate::fault::DepotFaultController;
use crate::workflows::compaction::DeltasAvailable;

use super::{
	branch,
	constants::{ACCESS_TOUCH_THROTTLE_MS, MAX_FORK_DEPTH},
	error::SqliteStorageError,
	keys,
	types::{BucketId, DatabaseBranchId},
};

/// Soft byte budget for the per-database decoded-LTX cache. Blobs are immutable
/// (their source key includes the owning txid), so caching them across reads is
/// a pure perf cache that never acts as a correctness fence.
const LTX_BLOB_CACHE_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// Bounded cache of parsed LTX blobs keyed by their immutable source key. Holds
/// the raw bytes plus the parsed page index so repeated reads of the same shard
/// or delta within a connection avoid both the FDB re-fetch and the index
/// re-parse. Bounded by total blob bytes via a moka weigher.
pub(super) type LtxBlobCache = moka::future::Cache<Vec<u8>, Arc<super::ltx::LtxBlob>>;

pub(super) fn new_ltx_blob_cache() -> LtxBlobCache {
	moka::future::Cache::builder()
		.max_capacity(LTX_BLOB_CACHE_MAX_BYTES)
		.weigher(|key: &Vec<u8>, blob: &Arc<super::ltx::LtxBlob>| {
			u32::try_from(key.len() + blob.bytes().len()).unwrap_or(u32::MAX)
		})
		.build()
}

pub type CompactionSignaler =
	Arc<dyn Fn(DeltasAvailable) -> BoxFuture<'static, Result<()>> + Send + Sync>;

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
	/// Head txid the cached PIDX reflects. The cache is trusted only when this matches the
	/// head resolved in the read transaction, so a foreign writer advancing the head
	/// invalidates the cache instead of letting it serve stale page ownership.
	#[cfg_attr(not(feature = "pidx-cache"), allow(dead_code))]
	pub(super) cache_head_txid: u64,
}

pub struct Db {
	pub(super) udb: Arc<Database>,
	pub(super) bucket_id: Id,
	pub(super) database_id: String,
	pub(super) node_id: NodeId,
	/// Cached branch read state. This is a perf cache; FDB remains the source of truth.
	pub(super) cache_snapshot: RwLock<Option<CacheSnapshot>>,
	/// Cached parsed LTX blobs keyed by immutable source key. Perf cache only.
	pub(super) ltx_blob_cache: LtxBlobCache,
	/// Cached `/META/quota`. Loaded once on the first UDB tx.
	pub(super) storage_used: RwLock<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: AtomicU64,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: AtomicU64,
	/// Last wall-clock time this database sent a workflow compaction wakeup.
	pub(super) last_deltas_available_at_ms: RwLock<Option<i64>>,
	pub(super) compaction_signaler: Option<CompactionSignaler>,
	#[cfg(feature = "test-faults")]
	pub(super) fault_controller: Option<DepotFaultController>,
}

impl Db {
	pub fn new(udb: Arc<Database>, bucket_id: Id, database_id: String, node_id: NodeId) -> Self {
		Self::new_inner(udb, bucket_id, database_id, node_id, None)
	}

	pub fn new_with_compaction_signaler(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		compaction_signaler: CompactionSignaler,
	) -> Self {
		Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			Some(compaction_signaler),
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
		let mut db = Self::new_inner(udb, bucket_id, database_id, node_id, None);
		db.fault_controller = Some(fault_controller);
		db
	}

	#[cfg(feature = "test-faults")]
	pub fn new_with_compaction_signaler_and_fault_controller_for_test(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		compaction_signaler: CompactionSignaler,
		fault_controller: DepotFaultController,
	) -> Self {
		let mut db = Self::new_inner(
			udb,
			bucket_id,
			database_id,
			node_id,
			Some(compaction_signaler),
		);
		db.fault_controller = Some(fault_controller);
		db
	}

	fn new_inner(
		udb: Arc<Database>,
		bucket_id: Id,
		database_id: String,
		node_id: NodeId,
		compaction_signaler: Option<CompactionSignaler>,
	) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile_nonblocking(udb.clone(), database_id.clone(), node_id);

		Self {
			udb,
			bucket_id,
			database_id,
			node_id,
			cache_snapshot: RwLock::new(None),
			ltx_blob_cache: new_ltx_blob_cache(),
			storage_used: RwLock::new(None),
			commit_bytes_since_rollup: AtomicU64::new(0),
			read_bytes_since_rollup: AtomicU64::new(0),
			last_deltas_available_at_ms: RwLock::new(None),
			compaction_signaler,
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
