#![allow(dead_code)]

use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use depot::conveyer::Db;
use gas::prelude::Id;
use rivet_pools::NodeId;
use tempfile::{Builder, TempDir};
use universaldb::utils::IsolationLevel::Snapshot;

pub async fn test_db(prefix: &str) -> Result<universaldb::Database> {
	let path = Builder::new().prefix(prefix).tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

pub async fn test_db_with_dir(prefix: &str) -> Result<(Arc<universaldb::Database>, TempDir)> {
	let dir = Builder::new().prefix(prefix).tempdir()?;
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(dir.path().to_path_buf()).await?;

	Ok((Arc::new(universaldb::Database::new(Arc::new(driver))), dir))
}

pub async fn test_db_arc(prefix: &str) -> Result<Arc<universaldb::Database>> {
	Ok(Arc::new(test_db(prefix).await?))
}

pub fn make_db(
	db: Arc<universaldb::Database>,
	bucket_id: Id,
	database_id: impl Into<String>,
) -> Db {
	Db::new(db, bucket_id, database_id.into(), NodeId::new())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMode {
	Disabled,
}

impl TierMode {
	pub fn label(self) -> &'static str {
		match self {
			TierMode::Disabled => "fdb_only",
		}
	}
}

pub struct TestDb {
	pub db: Db,
	pub udb: Arc<universaldb::Database>,
	pub bucket_id: Id,
	pub database_id: String,
	_udb_dir: TempDir,
}

impl TestDb {
	pub fn make_db(&self, bucket_id: Id, database_id: impl Into<String>) -> Db {
		Db::new(
			self.udb.clone(),
			bucket_id,
			database_id.into(),
			NodeId::new(),
		)
	}
}

pub async fn build_test_db(prefix: &str, tier: TierMode) -> Result<TestDb> {
	let (udb, udb_dir) = test_db_with_dir(prefix).await?;
	let bucket_id = Id::new_v1(1);
	let database_id = format!("{prefix}-db");

	let db = match tier {
		TierMode::Disabled => Db::new(udb.clone(), bucket_id, database_id.clone(), NodeId::new()),
	};

	Ok(TestDb {
		db,
		udb,
		bucket_id,
		database_id,
		_udb_dir: udb_dir,
	})
}

pub async fn test_matrix<F>(prefix: &str, body: F) -> Result<()>
where
	F: Fn(TierMode, TestDb) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
{
	let tier = TierMode::Disabled;
	let ctx = build_test_db(prefix, tier)
		.await
		.with_context(|| format!("[{}] failed to build TestDb", tier.label()))?;
	body(tier, ctx)
		.await
		.with_context(|| format!("[{}] body failed", tier.label()))?;

	Ok(())
}

pub use depot::history_snapshot::{BranchHistorySnapshot, branch_history_snapshot};

#[cfg(feature = "test-faults")]
pub mod compaction_harness {
	use anyhow::{Result, bail};
	use depot::{
		types::DatabaseBranchId,
		workflows::compaction::{
			DbHotCompacterWorkflow, DbManagerWorkflow, DbReclaimerWorkflow,
			DepotCompactionTestDriver, ForceCompactionResult, ForceCompactionWork,
		},
	};
	use gas::prelude::{Id, Registry, TestCtx};

	pub fn build_registry() -> Registry {
		let mut registry = Registry::new();
		registry.register_workflow::<DbManagerWorkflow>().unwrap();
		registry
			.register_workflow::<DbHotCompacterWorkflow>()
			.unwrap();
		registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
		registry
	}

	pub async fn force_hot(
		test_ctx: &TestCtx,
		manager_workflow_id: Id,
		branch_id: DatabaseBranchId,
	) -> Result<ForceCompactionResult> {
		force(
			test_ctx,
			manager_workflow_id,
			branch_id,
			ForceCompactionWork {
				hot: true,
				reclaim: false,
				final_settle: false,
			},
		)
		.await
	}

	pub async fn force_reclaim(
		test_ctx: &TestCtx,
		manager_workflow_id: Id,
		branch_id: DatabaseBranchId,
	) -> Result<ForceCompactionResult> {
		force(
			test_ctx,
			manager_workflow_id,
			branch_id,
			ForceCompactionWork {
				hot: false,
				reclaim: true,
				final_settle: false,
			},
		)
		.await
	}

	/// Forces reclaim repeatedly until the manager reports no actionable work,
	/// so batch-budgeted reclaim passes drain completely. Returns the number of
	/// passes that attempted work.
	pub async fn force_reclaim_until_idle(
		test_ctx: &TestCtx,
		manager_workflow_id: Id,
		branch_id: DatabaseBranchId,
	) -> Result<usize> {
		let mut attempted_passes = 0;
		for _ in 0..10 {
			let result = force_reclaim(test_ctx, manager_workflow_id, branch_id).await?;
			if let Some(error) = result.terminal_error {
				bail!("forced reclaim failed: {error}");
			}
			if result.attempted_job_kinds.is_empty() {
				return Ok(attempted_passes);
			}
			attempted_passes += 1;
		}

		bail!("reclaim did not drain after 10 forced passes")
	}

	pub async fn force(
		test_ctx: &TestCtx,
		manager_workflow_id: Id,
		branch_id: DatabaseBranchId,
		work: ForceCompactionWork,
	) -> Result<ForceCompactionResult> {
		let driver = DepotCompactionTestDriver::new(test_ctx);
		let result = driver
			.force_compaction(manager_workflow_id, branch_id, work)
			.await?;
		if let Some(error) = &result.terminal_error {
			bail!("forced compaction failed: {error}");
		}

		Ok(result)
	}
}

pub async fn database_branch_id(
	udb: &universaldb::Database,
	bucket_id: Id,
	database_id: &str,
) -> Result<depot::types::DatabaseBranchId> {
	let database_id = database_id.to_string();
	udb.txn("test_depotcommon_branch_id", move |tx| {
		let database_id = database_id.clone();
		async move {
			depot::conveyer::branch::resolve_database_branch(
				&tx,
				depot::types::BucketId::from_gas_id(bucket_id),
				&database_id,
				universaldb::utils::IsolationLevel::Serializable,
			)
			.await?
			.context("database branch should exist")
		}
	})
	.await
}

pub async fn history(
	udb: &universaldb::Database,
	branch_id: depot::types::DatabaseBranchId,
) -> Result<BranchHistorySnapshot> {
	branch_history_snapshot(udb, branch_id).await
}

/// Asserts the exact set of txids that still own DELTA chunk rows. Always
/// exact-set equality so partial deletes and over-deletes both fail loudly.
pub fn assert_delta_txids(
	snapshot: &BranchHistorySnapshot,
	expected: impl IntoIterator<Item = u64>,
	context: &str,
) {
	let expected = expected
		.into_iter()
		.collect::<std::collections::BTreeSet<_>>();
	assert_eq!(
		snapshot.delta_txids(),
		expected,
		"[{context}] surviving DELTA txids did not match"
	);
}

pub fn assert_commit_txids(
	snapshot: &BranchHistorySnapshot,
	expected: impl IntoIterator<Item = u64>,
	context: &str,
) {
	let expected = expected
		.into_iter()
		.collect::<std::collections::BTreeSet<_>>();
	assert_eq!(
		snapshot.commit_txids(),
		expected,
		"[{context}] surviving COMMITS txids did not match"
	);
}

pub fn assert_vtx_txids(
	snapshot: &BranchHistorySnapshot,
	expected: impl IntoIterator<Item = u64>,
	context: &str,
) {
	let expected = expected
		.into_iter()
		.collect::<std::collections::BTreeSet<_>>();
	assert_eq!(
		snapshot.vtx_txids(),
		expected,
		"[{context}] surviving VTX txids did not match"
	);
}

pub fn assert_pidx(
	snapshot: &BranchHistorySnapshot,
	expected: impl IntoIterator<Item = (u32, u64)>,
	context: &str,
) {
	let expected = expected
		.into_iter()
		.collect::<std::collections::BTreeMap<_, _>>();
	assert_eq!(
		snapshot.pidx, expected,
		"[{context}] surviving PIDX rows did not match"
	);
}

pub fn assert_shard_versions(
	snapshot: &BranchHistorySnapshot,
	expected: impl IntoIterator<Item = (u32, Vec<u64>)>,
	context: &str,
) {
	let expected = expected
		.into_iter()
		.collect::<std::collections::BTreeMap<_, _>>();
	assert_eq!(
		snapshot.shard_versions, expected,
		"[{context}] surviving SHARD versions did not match"
	);
}

pub async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.txn("test_depotcommon_mod", move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}
