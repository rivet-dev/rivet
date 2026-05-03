use std::ffi::{CStr, CString};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::ptr;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use depot::{
	cold_tier::{ColdTier, FaultyColdTier, FilesystemColdTier},
	fault::{DepotFaultCheckpoint, DepotFaultController, DepotFaultReplayEvent},
	keys,
	ltx::{decode_ltx_v3, encode_ltx_v3},
	types::{
		DatabaseBranchId, DirtyPage, RestorePointId, SnapshotSelector, decode_cold_shard_ref,
		encode_cold_shard_ref,
	},
	workflows::compaction::{
		DbColdCompacterWorkflow, DbHotCompacterWorkflow, DbManagerWorkflow, DbReclaimerWorkflow,
		DepotCompactionTestDriver, ForceCompactionResult, ForceCompactionWork,
		test_hooks::{self, WorkflowColdTierGuard, WorkflowFaultControllerGuard},
	},
};
use futures_util::TryStreamExt;
use gas::prelude::{Registry, TestCtx};
use libsqlite3_sys::{
	SQLITE_BLOB, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TEXT,
	sqlite3, sqlite3_column_blob, sqlite3_column_bytes, sqlite3_column_count,
	sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type,
	sqlite3_finalize, sqlite3_prepare_v2, sqlite3_step,
};
use parking_lot::Mutex;
use rivet_pools::__rivet_util::Id;
use rivet_test_deps::TestDeps;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::runtime::Builder;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use super::super::{
	DirectStorage, DirectStorageStats, NativeDatabase, SqliteTransport, SqliteVfs, VfsConfig,
	open_database,
};
use super::oracle::{
	AmbiguousOracleOutcome, NativeSqliteOracle, OracleCommitSemantics, OracleVerification,
};
use super::verify::DepotInvariantScanner;
use super::workload::LogicalOp;

type StageFuture = Pin<Box<dyn Future<Output = Result<()>>>>;
type Stage = Box<dyn FnOnce(FaultScenarioCtx) -> StageFuture>;
type FaultSetup = Box<dyn FnOnce(&DepotFaultController) -> Result<()>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultProfile {
	Simple,
	Chaos,
}

pub(crate) struct FaultScenario {
	name: String,
	seed: u64,
	profile: FaultProfile,
	setup: Option<Stage>,
	workload: Option<Stage>,
	faults: Option<FaultSetup>,
	verify: Option<Stage>,
}

#[derive(Clone)]
pub(crate) struct FaultScenarioCtx {
	inner: Arc<FaultScenarioInner>,
}

struct FaultScenarioInner {
	name: String,
	seed: u64,
	profile: FaultProfile,
	actor_id: String,
	handle: tokio::runtime::Handle,
	storage: Arc<DirectStorage>,
	verification_cold_tier: Arc<dyn ColdTier>,
	_cold_dir: TempDir,
	database: Mutex<Option<NativeDatabase>>,
	oracle: Mutex<NativeSqliteOracle>,
	faults: DepotFaultController,
	checkpoints: Mutex<Vec<DepotFaultCheckpoint>>,
	workload: Mutex<Vec<LogicalOp>>,
	branch_head_before_faults: Mutex<Option<u64>>,
	workload_fault_event_count: Mutex<Option<usize>>,
	oracle_result: Mutex<Option<String>>,
	ambiguous_oracle_outcome: Mutex<Option<AmbiguousOracleOutcome>>,
	manager_workflow_id: tokio::sync::Mutex<Option<Id>>,
	workflow_fault_guard: Mutex<Option<WorkflowFaultControllerGuard>>,
	workflow_cold_tier_guard: Mutex<Option<WorkflowColdTierGuard>>,
	test_ctx: tokio::sync::Mutex<TestCtx>,
}

#[derive(Clone, Debug)]
pub(crate) struct FaultScenarioReplayRecord {
	pub(crate) scenario: String,
	pub(crate) seed: u64,
	pub(crate) profile: FaultProfile,
	pub(crate) checkpoints: Vec<String>,
	pub(crate) workload: Vec<LogicalOp>,
	pub(crate) branch_head_before_faults: Option<u64>,
	pub(crate) branch_head_after_workload: Option<u64>,
	pub(crate) oracle_result: Option<String>,
	pub(crate) ambiguous_oracle_outcome: Option<AmbiguousOracleOutcome>,
	pub(crate) fault_events: Vec<FaultScenarioReplayEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultReplayPhase {
	Workload,
	Verification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FaultScenarioReplayEvent {
	pub(crate) event: DepotFaultReplayEvent,
	pub(crate) phase: FaultReplayPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectStorageCounterSnapshot {
	stats: DirectStorageStats,
}

impl FaultScenario {
	pub(crate) fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			seed: 0,
			profile: FaultProfile::Simple,
			setup: None,
			workload: None,
			faults: None,
			verify: None,
		}
	}

	pub(crate) fn seed(mut self, seed: u64) -> Self {
		self.seed = seed;
		self
	}

	pub(crate) fn profile(mut self, profile: FaultProfile) -> Self {
		self.profile = profile;
		self
	}

	pub(crate) fn setup<F, Fut>(mut self, setup: F) -> Self
	where
		F: FnOnce(FaultScenarioCtx) -> Fut + 'static,
		Fut: Future<Output = Result<()>> + 'static,
	{
		self.setup = Some(Box::new(move |ctx| Box::pin(setup(ctx))));
		self
	}

	pub(crate) fn workload<F, Fut>(mut self, workload: F) -> Self
	where
		F: FnOnce(FaultScenarioCtx) -> Fut + 'static,
		Fut: Future<Output = Result<()>> + 'static,
	{
		self.workload = Some(Box::new(move |ctx| Box::pin(workload(ctx))));
		self
	}

	pub(crate) fn faults<F>(mut self, faults: F) -> Self
	where
		F: FnOnce(&DepotFaultController) -> Result<()> + 'static,
	{
		self.faults = Some(Box::new(faults));
		self
	}

	pub(crate) fn verify<F, Fut>(mut self, verify: F) -> Self
	where
		F: FnOnce(FaultScenarioCtx) -> Fut + 'static,
		Fut: Future<Output = Result<()>> + 'static,
	{
		self.verify = Some(Box::new(move |ctx| Box::pin(verify(ctx))));
		self
	}

	pub(crate) fn run(self) -> Result<()> {
		let runtime = Builder::new_multi_thread()
			.worker_threads(2)
			.enable_all()
			.build()
			.context("fault scenario runtime should build")?;
		let ctx = runtime.block_on(FaultScenarioCtx::new(&self))?;
		ctx.open_database()?;

		let mut result = Ok(());
		if let Some(setup) = self.setup {
			result = runtime.block_on(setup(ctx.clone()));
		}
		if result.is_ok() {
			result = runtime.block_on(ctx.enter_strict_workload_mode());
		}
		let strict_workload_counters = if result.is_ok() {
			Some(ctx.direct_storage_counter_snapshot())
		} else {
			None
		};
		if result.is_ok() {
			result = runtime.block_on(ctx.capture_branch_head_before_faults());
		}
		if result.is_ok() {
			if let Some(faults) = self.faults {
				result = faults(ctx.fault_controller());
			}
		}
		if result.is_ok() {
			if let Some(workload) = self.workload {
				result = runtime.block_on(workload(ctx.clone()));
			}
		}
		if result.is_ok() {
			if let Some(strict_workload_counters) = &strict_workload_counters {
				result = ctx.assert_strict_mirror_counters_unchanged(strict_workload_counters);
			}
		}
		if result.is_ok() {
			result = ctx.fault_controller().assert_expected_fired();
		}
		if result.is_ok() {
			ctx.mark_workload_faults_complete();
		}
		if result.is_ok() {
			if let Some(verify) = self.verify {
				result = runtime.block_on(verify(ctx.clone()));
			}
		}

		let shutdown_result = runtime.block_on(ctx.shutdown());
		result?;
		shutdown_result?;
		Ok(())
	}
}

impl FaultScenarioCtx {
	async fn new(scenario: &FaultScenario) -> Result<Self> {
		let cold_dir = tempfile::tempdir().context("cold tier temp dir should build")?;
		let test_ctx = test_ctx_with_cold_tier(cold_dir.path()).await?;
		let udb = test_ctx.pools().udb()?;
		let handle = tokio::runtime::Handle::current();
		let actor_id = super::super::next_test_name("sqlite-fault-actor");
		let faults = DepotFaultController::new();
		let cold_tier = Arc::new(FaultyColdTier::new_with_fault_controller_for_test(
			FilesystemColdTier::new(cold_dir.path()),
			"sqlite-fault-cold-tier",
			faults.clone(),
		)) as Arc<dyn ColdTier>;
		let verification_cold_tier =
			Arc::new(FilesystemColdTier::new(cold_dir.path())) as Arc<dyn ColdTier>;
		let storage = Arc::new(DirectStorage::new_with_cold_tier_and_fault_controller(
			(*udb).clone(),
			cold_tier,
			faults.clone(),
		));

		Ok(Self {
			inner: Arc::new(FaultScenarioInner {
				name: scenario.name.clone(),
				seed: scenario.seed,
				profile: scenario.profile,
				actor_id,
				handle,
				storage,
				verification_cold_tier,
				_cold_dir: cold_dir,
				database: Mutex::new(None),
				oracle: Mutex::new(NativeSqliteOracle::open()?),
				faults,
				checkpoints: Mutex::new(Vec::new()),
				workload: Mutex::new(Vec::new()),
				branch_head_before_faults: Mutex::new(None),
				workload_fault_event_count: Mutex::new(None),
				oracle_result: Mutex::new(None),
				ambiguous_oracle_outcome: Mutex::new(None),
				manager_workflow_id: tokio::sync::Mutex::new(None),
				workflow_fault_guard: Mutex::new(None),
				workflow_cold_tier_guard: Mutex::new(None),
				test_ctx: tokio::sync::Mutex::new(test_ctx),
			}),
		})
	}

	pub(crate) async fn sql(&self, sql: &str) -> Result<()> {
		self.with_database_blocking(|db| {
			super::super::sqlite_exec(db.as_ptr(), sql).map_err(anyhow::Error::msg)
		})?;
		self.inner.oracle.lock().apply_sql(sql)
	}

	pub(crate) async fn query(&self, sql: &str) -> Result<Vec<Vec<String>>> {
		self.with_database_blocking(|db| query_rows(db.as_ptr(), sql))
	}

	pub(crate) async fn exec(&self, op: LogicalOp) -> Result<()> {
		self.exec_with_oracle_semantics(op, OracleCommitSemantics::Success)
			.await
	}

	pub(crate) async fn exec_with_durable_error(&self, op: LogicalOp) -> Result<()> {
		let result = self.with_database_blocking(|db| op.apply(db.as_ptr()));
		if result.is_ok() {
			bail!("operation unexpectedly succeeded after durable fault");
		}

		self.inner.workload.lock().push(op.clone());
		self.inner
			.oracle
			.lock()
			.apply_logical_op(op, OracleCommitSemantics::Success)
	}

	#[allow(dead_code)]
	pub(crate) async fn exec_with_oracle_semantics(
		&self,
		op: LogicalOp,
		semantics: OracleCommitSemantics,
	) -> Result<()> {
		match semantics {
			OracleCommitSemantics::PreCommitFailure => {
				let result = self.with_database_blocking(|db| op.apply(db.as_ptr()));
				if result.is_ok() {
					bail!("operation unexpectedly succeeded before pre-commit failure");
				}
			}
			OracleCommitSemantics::Success => {
				self.with_database_blocking(|db| op.apply(db.as_ptr()))?;
			}
			OracleCommitSemantics::AmbiguousPostCommit => {
				let _ = self.with_database_blocking(|db| op.apply(db.as_ptr()));
			}
		}

		self.inner.workload.lock().push(op.clone());
		self.inner.oracle.lock().apply_logical_op(op, semantics)
	}

	pub(crate) async fn checkpoint(&self, name: impl Into<String>) -> Result<()> {
		self.inner
			.checkpoints
			.lock()
			.push(DepotFaultCheckpoint::new(name));
		Ok(())
	}

	pub(crate) async fn reload_database(&self) -> Result<()> {
		self.close_database();
		self.inner.storage.enable_strict_mode();
		self.inner
			.storage
			.evict_actor_db(&self.inner.actor_id)
			.await;
		self.open_database_blocking()?;
		Ok(())
	}

	pub(crate) fn direct_storage_counter_snapshot(&self) -> DirectStorageCounterSnapshot {
		DirectStorageCounterSnapshot {
			stats: self.inner.storage.stats(),
		}
	}

	pub(crate) fn assert_strict_mirror_counters_unchanged(
		&self,
		before: &DirectStorageCounterSnapshot,
	) -> Result<()> {
		let after = self.direct_storage_counter_snapshot();
		if after.stats.mirror_reads != before.stats.mirror_reads {
			bail!(
				"strict workload used mirror reads: before={}, after={}",
				before.stats.mirror_reads,
				after.stats.mirror_reads
			);
		}
		if after.stats.mirror_fills != before.stats.mirror_fills {
			bail!(
				"strict workload used mirror fills: before={}, after={}",
				before.stats.mirror_fills,
				after.stats.mirror_fills
			);
		}
		if after.stats.mirror_seeds != before.stats.mirror_seeds {
			bail!(
				"strict workload used mirror seeds: before={}, after={}",
				before.stats.mirror_seeds,
				after.stats.mirror_seeds
			);
		}
		Ok(())
	}

	#[allow(dead_code)]
	pub(crate) async fn force_hot_compaction(&self) -> Result<ForceCompactionResult> {
		self.force_compaction(ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		})
		.await
	}

	#[allow(dead_code)]
	pub(crate) async fn force_cold_compaction(&self) -> Result<ForceCompactionResult> {
		self.force_compaction(ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		})
		.await
	}

	#[allow(dead_code)]
	pub(crate) async fn force_reclaim(&self) -> Result<ForceCompactionResult> {
		self.force_compaction(ForceCompactionWork {
			hot: false,
			cold: false,
			reclaim: true,
			final_settle: false,
		})
		.await
	}

	pub(crate) async fn force_compaction(
		&self,
		work: ForceCompactionWork,
	) -> Result<ForceCompactionResult> {
		let database_branch_id = self.database_branch_id().await?;
		let manager_workflow_id = self.manager_workflow_id(database_branch_id).await?;
		let test_ctx = self.inner.test_ctx.lock().await;
		DepotCompactionTestDriver::new(&test_ctx)
			.force_compaction(manager_workflow_id, database_branch_id, work)
			.await
	}

	pub(crate) async fn verify_sqlite_integrity(&self) -> Result<()> {
		self.with_database_blocking(|db| NativeSqliteOracle::verify_integrity(db.as_ptr()))?;
		self.inner.oracle.lock().verify_oracle_integrity()?;
		Ok(())
	}

	#[allow(dead_code)]
	pub(crate) async fn verify_sqlite_integrity_rows(&self) -> Result<()> {
		let quick = self.query("PRAGMA quick_check;").await?;
		if quick != vec![vec!["ok".to_string()]] {
			bail!("sqlite quick_check failed: {quick:?}");
		}

		let integrity = self.query("PRAGMA integrity_check;").await?;
		if integrity != vec![vec!["ok".to_string()]] {
			bail!("sqlite integrity_check failed: {integrity:?}");
		}

		let foreign_keys = self.query("PRAGMA foreign_key_check;").await?;
		if !foreign_keys.is_empty() {
			bail!("sqlite foreign_key_check failed: {foreign_keys:?}");
		}

		Ok(())
	}

	pub(crate) async fn verify_against_native_oracle(&self) -> Result<()> {
		let result =
			self.with_database_blocking(|db| self.inner.oracle.lock().verify_matches(db.as_ptr()));
		let mut ambiguous_outcome = self.inner.ambiguous_oracle_outcome.lock();
		*ambiguous_outcome = match &result {
			Ok(OracleVerification::Ambiguous(outcome)) => Some(*outcome),
			Err(err) if format!("{err:#}").contains("ambiguous oracle mismatch") => {
				Some(AmbiguousOracleOutcome::Invalid)
			}
			Ok(OracleVerification::Matched) | Err(_) => None,
		};
		*self.inner.oracle_result.lock() = Some(match &result {
			Ok(OracleVerification::Matched) => "matched".to_string(),
			Ok(OracleVerification::Ambiguous(outcome)) => {
				format!("ambiguous:{}", outcome.as_str())
			}
			Err(err) => format!("{err:#}"),
		});
		result.map(|_| ())
	}

	pub(crate) async fn verify_depot_invariants(&self) -> Result<()> {
		DepotInvariantScanner::new(
			self.inner.storage.depot_database(),
			Some(Arc::clone(&self.inner.verification_cold_tier)),
			self.inner.actor_id.clone(),
		)
		.verify()
		.await
	}

	pub(crate) async fn replay_record(&self) -> FaultScenarioReplayRecord {
		let branch_head_after_workload = self
			.inner
			.storage
			.read_branch_head(&self.inner.actor_id)
			.await
			.ok()
			.map(|(_, head_txid)| head_txid);
		let workload_fault_event_count = (*self.inner.workload_fault_event_count.lock())
			.unwrap_or_else(|| self.inner.faults.replay_log().len());
		let fault_events = self
			.inner
			.faults
			.replay_log_with_unfired()
			.into_iter()
			.enumerate()
			.map(|(index, event)| FaultScenarioReplayEvent {
				event,
				phase: if index < workload_fault_event_count {
					FaultReplayPhase::Workload
				} else {
					FaultReplayPhase::Verification
				},
			})
			.collect();

		FaultScenarioReplayRecord {
			scenario: self.inner.name.clone(),
			seed: self.inner.seed,
			profile: self.inner.profile,
			checkpoints: self
				.inner
				.checkpoints
				.lock()
				.iter()
				.map(|checkpoint| checkpoint.name().to_string())
				.collect(),
			workload: self.inner.workload.lock().clone(),
			branch_head_before_faults: *self.inner.branch_head_before_faults.lock(),
			branch_head_after_workload,
			oracle_result: self.inner.oracle_result.lock().clone(),
			ambiguous_oracle_outcome: *self.inner.ambiguous_oracle_outcome.lock(),
			fault_events,
		}
	}

	pub(crate) fn fault_controller(&self) -> &DepotFaultController {
		&self.inner.faults
	}

	fn open_database(&self) -> Result<()> {
		let database = open_fault_database(
			&self.inner.handle,
			Arc::clone(&self.inner.storage),
			&self.inner.actor_id,
		)?;
		*self.inner.database.lock() = Some(database);
		Ok(())
	}

	fn open_database_blocking(&self) -> Result<()> {
		tokio::task::block_in_place(|| self.open_database())
	}

	async fn enter_strict_workload_mode(&self) -> Result<()> {
		self.close_database();
		self.inner.storage.enable_strict_mode();
		self.inner
			.storage
			.evict_actor_db(&self.inner.actor_id)
			.await;
		self.open_database_blocking()
	}

	async fn shutdown(&self) -> Result<()> {
		self.close_database();
		self.inner.workflow_fault_guard.lock().take();
		self.inner.workflow_cold_tier_guard.lock().take();
		let mut test_ctx = self.inner.test_ctx.lock().await;
		test_ctx.shutdown().await
	}

	pub(crate) async fn database_branch_id(&self) -> Result<DatabaseBranchId> {
		self.inner
			.storage
			.read_branch_head(&self.inner.actor_id)
			.await
			.map(|(branch_id, _)| branch_id)
	}

	#[allow(dead_code)]
	pub(crate) fn depot_database(&self) -> Arc<universaldb::Database> {
		self.inner.storage.depot_database()
	}

	pub(crate) async fn create_restore_point(&self) -> Result<RestorePointId> {
		self.inner
			.storage
			.actor_db(self.inner.actor_id.clone())
			.await
			.create_restore_point(SnapshotSelector::Latest)
			.await
	}

	pub(crate) async fn delete_restore_point(&self, restore_point: RestorePointId) -> Result<()> {
		self.inner
			.storage
			.actor_db(self.inner.actor_id.clone())
			.await
			.delete_restore_point(restore_point)
			.await
	}

	pub(crate) async fn override_cold_object_delete_grace(
		&self,
		grace_ms: i64,
	) -> Result<test_hooks::ColdObjectDeleteGraceGuard> {
		let database_branch_id = self.database_branch_id().await?;
		Ok(test_hooks::override_cold_object_delete_grace_for_test(
			database_branch_id,
			grace_ms,
		))
	}

	pub(crate) async fn seed_page_as_cold_ref_for_harness_test(&self, pgno: u32) -> Result<()> {
		let dirty_pages = self.with_database_blocking(|db| {
			let state = db._vfs.ctx().state.read();
			(1..=state.db_size_pages)
				.filter(|candidate_pgno| {
					*candidate_pgno / depot::keys::SHARD_SIZE == pgno / depot::keys::SHARD_SIZE
				})
				.map(|candidate_pgno| {
					let bytes = state.page_cache.get(&candidate_pgno).with_context(|| {
						format!(
							"page {candidate_pgno} should be present in strict VFS cache before cold-ref seed"
						)
					})?;
					Ok(DirtyPage {
						pgno: candidate_pgno,
						bytes,
					})
				})
				.collect::<Result<Vec<_>>>()
		})?;
		self.inner
			.storage
			.seed_pages_as_cold_ref(&self.inner.actor_id, pgno, dirty_pages)
			.await
	}

	pub(crate) async fn remove_page_from_seeded_cold_ref_for_harness_test(
		&self,
		pgno: u32,
	) -> Result<()> {
		let cold_tier = self
			.inner
			.storage
			.cold_tier()
			.context("fault scenario cold tier should be configured")?;
		let (branch_id, head_txid) = self
			.inner
			.storage
			.read_branch_head(&self.inner.actor_id)
			.await?;
		let shard_id = pgno / keys::SHARD_SIZE;
		let cold_ref_key = keys::branch_compaction_cold_shard_key(branch_id, shard_id, head_txid);
		let mut reference = self
			.inner
			.storage
			.depot_database()
			.run(move |tx| {
				let cold_ref_key = cold_ref_key.clone();
				async move {
					let value = tx
						.informal()
						.get(&cold_ref_key, Serializable)
						.await?
						.context("seeded cold ref should exist")?;
					decode_cold_shard_ref(&value)
				}
			})
			.await?;

		let object_bytes = cold_tier
			.get_object(&reference.object_key)
			.await?
			.context("seeded cold object should exist")?;
		let decoded = decode_ltx_v3(&object_bytes)?;
		let mut removed = false;
		let pages = decoded
			.pages
			.into_iter()
			.filter(|page| {
				if page.pgno == pgno {
					removed = true;
					false
				} else {
					true
				}
			})
			.collect::<Vec<_>>();
		ensure!(removed, "seeded cold object did not contain page {pgno}");

		let rewritten_bytes = encode_ltx_v3(decoded.header, &pages)?;
		let digest = Sha256::digest(&rewritten_bytes);
		reference.size_bytes = rewritten_bytes.len() as u64;
		reference.content_hash.copy_from_slice(&digest);
		cold_tier
			.put_object(&reference.object_key, &rewritten_bytes)
			.await?;

		self.inner
			.storage
			.depot_database()
			.run(move |tx| {
				let reference = reference.clone();
				async move {
					tx.informal().set(
						&keys::branch_compaction_cold_shard_key(branch_id, shard_id, head_txid),
						&encode_cold_shard_ref(reference)?,
					);
					tx.informal().set(
						&keys::branch_pidx_key(branch_id, pgno),
						&head_txid.to_be_bytes(),
					);
					Ok(())
				}
			})
			.await
	}

	pub(crate) async fn read_page_from_depot(&self, pgno: u32) -> Result<()> {
		self.inner.storage.enable_strict_mode();
		self.inner
			.storage
			.evict_actor_db(&self.inner.actor_id)
			.await;
		self.inner
			.storage
			.get_pages(&self.inner.actor_id, &[pgno])
			.await?;
		Ok(())
	}

	pub(crate) async fn latest_delta_chunk_count(&self) -> Result<usize> {
		let (branch_id, head_txid) = self
			.inner
			.storage
			.read_branch_head(&self.inner.actor_id)
			.await?;
		let db = self.inner.storage.depot_database();
		db.run(move |tx| async move {
			let prefix = keys::branch_delta_chunk_prefix(branch_id, head_txid);
			let prefix_subspace =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..RangeOption::from(&prefix_subspace)
				},
				Snapshot,
			);
			let mut count = 0;
			while stream.try_next().await?.is_some() {
				count += 1;
			}
			Ok(count)
		})
		.await
	}

	pub(crate) fn cold_gets(&self) -> u64 {
		self.inner.storage.stats().cold_gets
	}

	async fn capture_branch_head_before_faults(&self) -> Result<()> {
		let (_, head_txid) = self
			.inner
			.storage
			.read_branch_head(&self.inner.actor_id)
			.await?;
		*self.inner.branch_head_before_faults.lock() = Some(head_txid);
		Ok(())
	}

	fn mark_workload_faults_complete(&self) {
		*self.inner.workload_fault_event_count.lock() = Some(self.inner.faults.replay_log().len());
	}

	async fn manager_workflow_id(&self, database_branch_id: DatabaseBranchId) -> Result<Id> {
		if let Some(manager_workflow_id) = *self.inner.manager_workflow_id.lock().await {
			return Ok(manager_workflow_id);
		}

		let test_ctx = self.inner.test_ctx.lock().await;
		*self.inner.workflow_fault_guard.lock() =
			Some(test_hooks::register_workflow_fault_controller(
				database_branch_id,
				self.inner.faults.clone(),
			));
		if self.inner.workflow_cold_tier_guard.lock().is_none() {
			let cold_tier = self
				.inner
				.storage
				.cold_tier()
				.context("fault scenario cold tier should be configured")?;
			*self.inner.workflow_cold_tier_guard.lock() = Some(
				test_hooks::install_workflow_cold_tier_for_test(database_branch_id, cold_tier),
			);
		}
		let manager_workflow_id = DepotCompactionTestDriver::new(&test_ctx)
			.start_manager(database_branch_id, Some(self.inner.actor_id.clone()), true)
			.await?;
		*self.inner.manager_workflow_id.lock().await = Some(manager_workflow_id);
		Ok(manager_workflow_id)
	}

	fn with_database_blocking<T>(&self, f: impl FnOnce(&NativeDatabase) -> Result<T>) -> Result<T> {
		tokio::task::block_in_place(|| self.with_database(f))
	}

	fn with_database<T>(&self, f: impl FnOnce(&NativeDatabase) -> Result<T>) -> Result<T> {
		let database = self.inner.database.lock();
		let database = database
			.as_ref()
			.context("fault scenario database is closed")?;
		f(database)
	}

	fn close_database(&self) {
		let _ = self.inner.database.lock().take();
	}
}

fn build_registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_workflow::<DbManagerWorkflow>().unwrap();
	registry
		.register_workflow::<DbHotCompacterWorkflow>()
		.unwrap();
	registry
		.register_workflow::<DbColdCompacterWorkflow>()
		.unwrap();
	registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
	registry
}

async fn test_ctx_with_cold_tier(root: &Path) -> Result<TestCtx> {
	let mut test_deps = TestDeps::new().await?;
	let mut config_root = (**test_deps.config()).clone();
	config_root.sqlite = Some(rivet_config::config::Sqlite {
		workflow_cold_storage: Some(rivet_config::config::SqliteWorkflowColdStorage::FileSystem(
			rivet_config::config::SqliteWorkflowColdStorageFileSystem {
				root: root.display().to_string(),
			},
		)),
	});
	test_deps.config = rivet_config::Config::from_root(config_root);
	TestCtx::new_with_deps(build_registry(), test_deps).await
}

fn open_fault_database(
	handle: &tokio::runtime::Handle,
	storage: Arc<DirectStorage>,
	actor_id: &str,
) -> Result<NativeDatabase> {
	let mut config = VfsConfig::default();
	config.assert_batch_atomic = false;
	let vfs = SqliteVfs::register_with_transport(
		&super::super::next_test_name("sqlite-fault-vfs"),
		SqliteTransport::from_direct(storage),
		actor_id.to_string(),
		handle.clone(),
		config,
		None,
	)
	.map_err(anyhow::Error::msg)?;

	open_database(vfs, actor_id).map_err(anyhow::Error::msg)
}

fn query_rows(db: *mut sqlite3, sql: &str) -> Result<Vec<Vec<String>>> {
	let c_sql = CString::new(sql)?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		bail!(
			"{sql} prepare failed with code {rc}: {}",
			sqlite_error_message(db)
		);
	}

	let mut rows = Vec::new();
	loop {
		match unsafe { sqlite3_step(stmt) } {
			SQLITE_ROW => rows.push(read_row(stmt)),
			libsqlite3_sys::SQLITE_DONE => break,
			step_rc => {
				unsafe {
					sqlite3_finalize(stmt);
				}
				bail!(
					"{sql} step failed with code {step_rc}: {}",
					sqlite_error_message(db)
				);
			}
		}
	}

	unsafe {
		sqlite3_finalize(stmt);
	}
	Ok(rows)
}

fn read_row(stmt: *mut libsqlite3_sys::sqlite3_stmt) -> Vec<String> {
	let column_count = unsafe { sqlite3_column_count(stmt) };
	(0..column_count)
		.map(|index| match unsafe { sqlite3_column_type(stmt, index) } {
			SQLITE_INTEGER => unsafe { sqlite3_column_int64(stmt, index) }.to_string(),
			SQLITE_FLOAT => unsafe { sqlite3_column_double(stmt, index) }.to_string(),
			SQLITE_TEXT => {
				let text = unsafe { sqlite3_column_text(stmt, index) };
				if text.is_null() {
					String::new()
				} else {
					unsafe { CStr::from_ptr(text.cast()) }
						.to_string_lossy()
						.into_owned()
				}
			}
			SQLITE_BLOB => {
				let blob = unsafe { sqlite3_column_blob(stmt, index) };
				let len = unsafe { sqlite3_column_bytes(stmt, index) };
				if blob.is_null() || len == 0 {
					String::new()
				} else {
					let bytes =
						unsafe { std::slice::from_raw_parts(blob.cast::<u8>(), len as usize) };
					hex_upper(bytes)
				}
			}
			SQLITE_NULL => "NULL".to_string(),
			other => format!("UNKNOWN({other})"),
		})
		.collect()
}

fn hex_upper(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789ABCDEF";
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		out.push(HEX[(byte >> 4) as usize] as char);
		out.push(HEX[(byte & 0x0f) as usize] as char);
	}
	out
}

fn sqlite_error_message(db: *mut sqlite3) -> String {
	let err = unsafe { libsqlite3_sys::sqlite3_errmsg(db) };
	if err.is_null() {
		return "unknown sqlite error".to_string();
	}
	unsafe { CStr::from_ptr(err) }
		.to_string_lossy()
		.into_owned()
}
