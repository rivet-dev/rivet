use std::{cell::RefCell, future::Future, path::Path, rc::Rc, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier, cold_tier_from_config},
	conveyer::{Db, branch, history_pin, metrics},
	debug,
	error::SqliteStorageError,
	keys::{
		PAGE_SIZE, branch_commit_key, branch_compaction_cold_shard_key,
		branch_compaction_cold_shard_prefix, branch_compaction_retired_cold_object_key,
		branch_compaction_root_key, branch_compaction_stage_hot_shard_key,
		branch_compaction_stage_hot_shard_prefix, branch_delta_chunk_key,
		branch_manifest_last_access_bucket_key, branch_meta_head_at_fork_key, branch_meta_head_key,
		branch_pidx_key, branch_pitr_interval_key, branch_shard_key, branch_shard_prefix,
		branch_vtx_key, branches_list_key, bucket_catalog_by_db_key, bucket_child_key,
		bucket_fork_pin_key, db_pin_key, sqlite_cmp_dirty_key,
	},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	policy::{set_bucket_pitr_policy, set_database_pitr_policy_override},
	types::{
		BranchState, BucketBranchId, BucketCatalogDbFact, BucketForkFact, BucketId, ColdShardRef,
		CommitRow, CompactionRoot, DBHead, DatabaseBranchId, DatabaseBranchRecord,
		DbHistoryPinKind, DirtyPage, FetchedPage, PitrIntervalCoverage, PitrPolicy,
		ResolvedVersionstamp, RestorePointId, RetiredColdObject, RetiredColdObjectDeleteState,
		SnapshotKind, SnapshotSelector, SqliteCmpDirty, decode_cold_shard_ref, decode_commit_row,
		decode_compaction_root, decode_db_head, decode_db_history_pin,
		decode_pitr_interval_coverage, decode_retired_cold_object, encode_bucket_catalog_db_fact,
		encode_bucket_fork_fact, encode_cold_shard_ref, encode_commit_row, encode_compaction_root,
		encode_database_branch_record, encode_db_head, encode_pitr_interval_coverage,
		encode_retired_cold_object, encode_sqlite_cmp_dirty,
	},
	workflows::compaction::{
		BranchStopState, ColdJobFinished, CompactionJobKind, CompactionJobStatus,
		CompanionWorkflowState, DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow,
		DbHotCompacterSignal, DbHotCompacterWorkflow, DbManagerInput, DbManagerSignal,
		DbManagerState, DbManagerWorkflow, DbReclaimerWorkflow, DeltasAvailable,
		DestroyDatabaseBranch, ForceCompaction, ForceCompactionResult, ForceCompactionWork,
		HotJobFinished, HotJobInputRange, HotShardOutputRef, ReclaimJobFinished, RunColdJob,
		RunHotJob, RunReclaimJob, TxidRange, database_branch_tag_value, test_hooks,
	},
};
use futures_util::{StreamExt, TryStreamExt};
use gas::db::{
	BumpSubSubject, Database, DatabaseKv,
	debug::{DatabaseDebug, WorkflowState},
};
use gas::prelude::{Id, Registry, SignalTrait, TestCtx, WorkflowTrait};
use rivet_pools::NodeId;
use rivet_test_deps::TestDeps;
use sha2::{Digest, Sha256};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

const TEST_DATABASE: &str = "workflow-compaction-test";
const FIVE_MINUTES_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowTierMode {
	Disabled,
	Filesystem,
}

impl WorkflowTierMode {
	fn label(self) -> &'static str {
		match self {
			WorkflowTierMode::Disabled => "cold_disabled",
			WorkflowTierMode::Filesystem => "cold_filesystem",
		}
	}
}

async fn workflow_test_matrix<F, Fut>(
	prefix: &str,
	registry: fn() -> Registry,
	mut body: F,
) -> Result<()>
where
	F: FnMut(WorkflowTierMode, TestCtx) -> Fut,
	Fut: Future<Output = Result<()>>,
{
	for tier in [WorkflowTierMode::Disabled, WorkflowTierMode::Filesystem] {
		let cold_root = if tier == WorkflowTierMode::Filesystem {
			Some(Builder::new().prefix(prefix).tempdir()?)
		} else {
			None
		};
		let test_ctx = if let Some(cold_root) = &cold_root {
			test_ctx_with_configured_cold_tier_and_registry(cold_root.path(), registry()).await?
		} else {
			TestCtx::new(registry()).await?
		};

		body(tier, test_ctx)
			.await
			.with_context(|| format!("[{}] body failed", tier.label()))?;
	}

	Ok(())
}

macro_rules! workflow_matrix {
	($prefix:expr, $registry:ident, |$tier:ident, $test_ctx:ident| $body:block) => {
		workflow_test_matrix($prefix, $registry, |$tier, mut $test_ctx| async move $body)
		.await
	};
}

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x5678), 1)
}

fn make_test_db(test_ctx: &TestCtx) -> Result<Db> {
	make_test_db_for(test_ctx, TEST_DATABASE)
}

fn make_test_db_for(test_ctx: &TestCtx, database_id: impl Into<String>) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	Ok(Db::new(
		udb,
		test_bucket(),
		database_id.into(),
		NodeId::new(),
	))
}

fn make_test_db_with_cold_tier(test_ctx: &TestCtx, cold_tier: Arc<dyn ColdTier>) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	Ok(Db::new_with_cold_tier(
		udb,
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		cold_tier,
	))
}

async fn test_ctx_with_configured_cold_tier(root: &Path) -> Result<TestCtx> {
	test_ctx_with_configured_cold_tier_and_registry(root, build_registry()).await
}

async fn test_ctx_with_configured_cold_tier_and_registry(
	root: &Path,
	registry: Registry,
) -> Result<TestCtx> {
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
	TestCtx::new_with_deps(registry, test_deps).await
}

fn configured_test_db(test_ctx: &TestCtx, cold_tier: Arc<dyn ColdTier>) -> Result<Db> {
	make_test_db_with_cold_tier(test_ctx, cold_tier)
}

fn page(fill: u8) -> Vec<u8> {
	vec![fill; PAGE_SIZE as usize]
}

fn current_time_ms() -> Result<i64> {
	let millis = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)?
		.as_millis();
	Ok(i64::try_from(millis)?)
}

fn assert_storage_error(err: &anyhow::Error, expected: SqliteStorageError) {
	assert!(
		err.chain().any(|cause| {
			cause
				.downcast_ref::<SqliteStorageError>()
				.is_some_and(|err| err == &expected)
		}),
		"expected {expected:?}, got {err:?}",
	);
}

fn dirty_page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: page(fill),
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

fn build_registry_without_hot_compacter() -> Registry {
	let mut registry = Registry::new();
	registry.register_workflow::<DbManagerWorkflow>().unwrap();
	registry
		.register_workflow::<DbColdCompacterWorkflow>()
		.unwrap();
	registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
	registry
}

fn build_registry_without_cold_compacter() -> Registry {
	let mut registry = Registry::new();
	registry.register_workflow::<DbManagerWorkflow>().unwrap();
	registry
		.register_workflow::<DbHotCompacterWorkflow>()
		.unwrap();
	registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
	registry
}

async fn wait_until<T, F, Fut>(description: impl Into<String>, mut check: F) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<Option<T>>>,
{
	let description = description.into();
	let started_at = tokio::time::Instant::now();

	loop {
		if let Some(value) = check().await? {
			return Ok(value);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for {description}");
		}

		// Signal debug rows and UDB test-observation rows do not expose a change notification API here.
		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_workflow<W: WorkflowTrait>(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<Id> {
	let tag_value = database_branch_tag_value(database_branch_id);
	let db = DatabaseKv::new(test_ctx.config().clone(), test_ctx.pools().clone()).await?;
	let mut workflow_created = db
		.bump_sub(BumpSubSubject::WorkflowCreated {
			tag: tag_value.clone(),
		})
		.await?;
	let timeout = tokio::time::sleep(Duration::from_secs(5));
	tokio::pin!(timeout);

	loop {
		if let Some(workflow_id) = test_ctx
			.find_workflow::<W>((DATABASE_BRANCH_ID_TAG, &tag_value))
			.await?
		{
			return Ok(workflow_id);
		}

		tokio::select! {
			bump = workflow_created.next() => {
				bump.context("workflow-created bump stream closed")?;
			}
			_ = &mut timeout => {
				bail!("timed out waiting for workflow {}", W::NAME);
			}
		}
	}
}

async fn wait_for_signal_ack(test_ctx: &TestCtx, signal_id: Id) -> Result<()> {
	wait_until("signal ack", || async {
		let signal = DatabaseDebug::get_signals(test_ctx.debug_db(), vec![signal_id])
			.await?
			.into_iter()
			.next();

		if let Some(signal) = signal {
			if signal.state == gas::db::debug::SignalState::Acked {
				return Ok(Some(()));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_run_hot_job(test_ctx: &TestCtx, hot_workflow_id: Id) -> Result<RunHotJob> {
	wait_until("RunHotJob signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(hot_workflow_id),
			Some(<RunHotJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(Some(serde_json::from_value(signal.body)?));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_run_cold_job(test_ctx: &TestCtx, cold_workflow_id: Id) -> Result<RunColdJob> {
	wait_until("RunColdJob signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(cold_workflow_id),
			Some(<RunColdJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(Some(serde_json::from_value(signal.body)?));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_run_reclaim_job(
	test_ctx: &TestCtx,
	reclaimer_workflow_id: Id,
) -> Result<RunReclaimJob> {
	wait_until("RunReclaimJob signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(reclaimer_workflow_id),
			Some(<RunReclaimJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(Some(serde_json::from_value(signal.body)?));
		}

		Ok(None)
	})
	.await
}

async fn single_destroy_signal_for_workflow(
	test_ctx: &TestCtx,
	workflow_id: Id,
) -> Result<DestroyDatabaseBranch> {
	let signals = DatabaseDebug::find_signals(
		test_ctx.debug_db(),
		&[],
		Some(workflow_id),
		Some(<DestroyDatabaseBranch as SignalTrait>::NAME),
		None,
	)
	.await?;
	assert_eq!(signals.len(), 1);

	Ok(serde_json::from_value(
		signals.into_iter().next().unwrap().body,
	)?)
}

async fn wait_for_reclaim_job_finished_signal(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
) -> Result<()> {
	wait_until("ReclaimJobFinished signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(manager_workflow_id),
			Some(<ReclaimJobFinished as SignalTrait>::NAME),
			None,
		)
		.await?;
		if !signals.is_empty() {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_hot_job_finished_signal(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	job_id: Id,
) -> Result<HotJobFinished> {
	wait_until("HotJobFinished signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(manager_workflow_id),
			Some(<HotJobFinished as SignalTrait>::NAME),
			None,
		)
		.await?;
		for signal in signals {
			let signal = serde_json::from_value::<HotJobFinished>(signal.body)?;
			if signal.job_id == job_id {
				return Ok(Some(signal));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_cold_job_finished_signal(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	job_id: Id,
) -> Result<ColdJobFinished> {
	wait_until("ColdJobFinished signal", || async {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(manager_workflow_id),
			Some(<ColdJobFinished as SignalTrait>::NAME),
			None,
		)
		.await?;
		for signal in signals {
			let signal = serde_json::from_value::<ColdJobFinished>(signal.body)?;
			if signal.job_id == job_id {
				return Ok(Some(signal));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_manager_cursor(
	test_ctx: &TestCtx,
	workflow_id: Id,
	observed_head_txid: u64,
) -> Result<DbManagerState> {
	wait_until("manager dirty cursor", || async {
		let history = DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
			.await?
			.ok_or_else(|| anyhow::anyhow!("manager workflow history not found"))?;

		for event in history.events.into_iter().rev() {
			if let gas::db::debug::EventData::Loop(loop_event) = event.data {
				let state = serde_json::from_value::<DbManagerState>(loop_event.state)?;
				if state
					.last_dirty_cursor
					.as_ref()
					.is_some_and(|cursor| cursor.observed_head_txid == observed_head_txid)
				{
					return Ok(Some(state));
				}
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_manager_state(
	test_ctx: &TestCtx,
	workflow_id: Id,
	predicate: impl FnMut(&DbManagerState) -> bool,
) -> Result<DbManagerState> {
	let predicate = Rc::new(RefCell::new(predicate));
	wait_until("manager state", || {
		let predicate = predicate.clone();
		async move {
			let history =
				DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
					.await?
					.ok_or_else(|| anyhow::anyhow!("manager workflow history not found"))?;

			for event in history.events.into_iter().rev() {
				if let gas::db::debug::EventData::Loop(loop_event) = event.data {
					let state = serde_json::from_value::<DbManagerState>(loop_event.state)?;
					if (predicate.borrow_mut())(&state) {
						return Ok(Some(state));
					}
				}
			}

			Ok(None)
		}
	})
	.await
}

async fn latest_manager_state(test_ctx: &TestCtx, workflow_id: Id) -> Result<DbManagerState> {
	let history = DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
		.await?
		.ok_or_else(|| anyhow::anyhow!("manager workflow history not found"))?;

	for event in history.events.into_iter().rev() {
		if let gas::db::debug::EventData::Loop(loop_event) = event.data {
			return Ok(serde_json::from_value::<DbManagerState>(loop_event.state)?);
		}
	}

	bail!("manager workflow has no loop state")
}

fn manager_has_distinct_companions(state: &DbManagerState) -> bool {
	state.companion_workflow_ids.hot_compacter_workflow_id
		!= state.companion_workflow_ids.cold_compacter_workflow_id
		&& state.companion_workflow_ids.hot_compacter_workflow_id
			!= state.companion_workflow_ids.reclaimer_workflow_id
		&& state.companion_workflow_ids.cold_compacter_workflow_id
			!= state.companion_workflow_ids.reclaimer_workflow_id
}

async fn latest_companion_state(
	test_ctx: &TestCtx,
	workflow_id: Id,
) -> Result<CompanionWorkflowState> {
	let history = DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
		.await?
		.ok_or_else(|| anyhow::anyhow!("companion workflow history not found"))?;

	for event in history.events.into_iter().rev() {
		if let gas::db::debug::EventData::Loop(loop_event) = event.data {
			return Ok(serde_json::from_value::<CompanionWorkflowState>(
				loop_event.state,
			)?);
		}
	}

	bail!("companion workflow has no loop state")
}

async fn force_compaction_and_wait_idle(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	database_branch_id: DatabaseBranchId,
	request_id: Id,
	requested_work: ForceCompactionWork,
) -> Result<ForceCompactionResult> {
	wait_for_manager_state(
		test_ctx,
		manager_workflow_id,
		manager_has_distinct_companions,
	)
	.await?;

	let signal_id = test_ctx
		.signal(ForceCompaction {
			database_branch_id,
			request_id,
			requested_work,
		})
		.to_workflow_id(manager_workflow_id)
		.send()
		.await?
		.expect("signal should target manager workflow");
	wait_for_signal_ack(test_ctx, signal_id).await?;

	let manager_state = wait_for_manager_state(test_ctx, manager_workflow_id, |state| {
		state
			.force_compactions
			.recent_results
			.iter()
			.any(|result| result.request_id == request_id)
	})
	.await?;

	manager_state
		.force_compactions
		.recent_results
		.into_iter()
		.find(|result| result.request_id == request_id)
		.ok_or_else(|| anyhow::anyhow!("force compaction result should be recorded"))
}

async fn wait_for_workflow_state(
	test_ctx: &TestCtx,
	workflow_id: Id,
	expected_state: WorkflowState,
) -> Result<()> {
	wait_until(format!("workflow state {expected_state:?}"), || async {
		let workflow = DatabaseDebug::get_workflows(test_ctx.debug_db(), vec![workflow_id])
			.await?
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("workflow not found"))?;
		if workflow.state == expected_state {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_dirty_marker_cleared(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<()> {
	wait_until("dirty marker clear", || async {
		let dirty = read_value(test_ctx, sqlite_cmp_dirty_key(database_branch_id)).await?;
		if dirty.is_none() {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_staged_hot_rows(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let prefix = branch_compaction_stage_hot_shard_prefix(database_branch_id, job_id);

	wait_until("staged hot shard rows", || async {
		let rows = read_prefix_values(test_ctx, prefix.clone()).await?;
		if !rows.is_empty() {
			return Ok(Some(rows));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_hot_install(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	as_of_txid: u64,
) -> Result<CompactionRoot> {
	wait_until("hot install", || async {
		let root = read_value(test_ctx, branch_compaction_root_key(database_branch_id))
			.await?
			.as_deref()
			.map(decode_compaction_root)
			.transpose()?;
		let pidx = read_value(test_ctx, branch_pidx_key(database_branch_id, 1)).await?;
		let shard = read_value(
			test_ctx,
			branch_shard_key(database_branch_id, 0, as_of_txid),
		)
		.await?;

		if let Some(root) = root {
			if root.manifest_generation == 1
				&& root.hot_watermark_txid == as_of_txid
				&& pidx.is_none()
				&& shard.is_some()
			{
				return Ok(Some(root));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_reclaim_delete(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<()> {
	wait_until("reclaim delete", || async {
		let delta = read_value(
			test_ctx,
			branch_delta_chunk_key(database_branch_id, txid, 0),
		)
		.await?;
		let commit = read_value(test_ctx, branch_commit_key(database_branch_id, txid)).await?;
		if delta.is_none() && commit.is_none() {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_cold_publish(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	as_of_txid: u64,
) -> Result<depot::types::ColdShardRef> {
	wait_until("cold publish", || async {
		let root = read_value(test_ctx, branch_compaction_root_key(database_branch_id))
			.await?
			.as_deref()
			.map(decode_compaction_root)
			.transpose()?;
		let cold_ref = read_value(
			test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, as_of_txid),
		)
		.await?
		.as_deref()
		.map(decode_cold_shard_ref)
		.transpose()?;

		if let (Some(root), Some(cold_ref)) = (&root, &cold_ref) {
			if root.cold_watermark_txid == as_of_txid && cold_ref.as_of_txid == as_of_txid {
				return Ok(Some(cold_ref.clone()));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_retired_cold_object_state(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	object_key: &str,
	state: RetiredColdObjectDeleteState,
) -> Result<depot::types::RetiredColdObject> {
	let retired_key =
		branch_compaction_retired_cold_object_key(database_branch_id, object_key_hash(object_key));

	wait_until(format!("retired cold object state {state:?}"), || async {
		let retired = read_value(test_ctx, retired_key.clone())
			.await?
			.as_deref()
			.map(decode_retired_cold_object)
			.transpose()?;
		if let Some(retired) = retired {
			if retired.delete_state == state {
				return Ok(Some(retired));
			}
		}

		Ok(None)
	})
	.await
}

async fn wait_for_cold_object_deleted(tier: &dyn ColdTier, object_key: &str) -> Result<()> {
	wait_until("cold object delete", || async {
		if tier.get_object(object_key).await?.is_none() {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

async fn wait_for_stage_row_cleared(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
) -> Result<()> {
	wait_until("staged hot shard cleanup", || async {
		let rows = read_prefix_values(
			test_ctx,
			branch_compaction_stage_hot_shard_prefix(database_branch_id, job_id),
		)
		.await?;
		if rows.is_empty() {
			return Ok(Some(()));
		}

		Ok(None)
	})
	.await
}

fn object_key_hash(object_key: &str) -> [u8; 32] {
	sha256(object_key.as_bytes())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}

async fn read_value(test_ctx: &TestCtx, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| {
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

async fn read_database_branch_id(test_ctx: &TestCtx) -> Result<DatabaseBranchId> {
	read_named_database_branch_id(test_ctx, TEST_DATABASE).await
}

async fn read_named_database_branch_id(
	test_ctx: &TestCtx,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let db = test_ctx.pools().udb()?;
	let database_id = database_id.to_string();
	db.run(move |tx| {
		let database_id = database_id.clone();
		async move {
			branch::resolve_database_branch(
				&tx,
				depot::types::BucketId::from_gas_id(test_bucket()),
				&database_id,
				universaldb::utils::IsolationLevel::Serializable,
			)
			.await?
			.ok_or_else(|| anyhow::anyhow!("database branch should exist"))
		}
	})
	.await
}

async fn read_pitr_interval_coverage(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
) -> Result<Option<PitrIntervalCoverage>> {
	read_value(
		test_ctx,
		branch_pitr_interval_key(database_branch_id, bucket_start_ms),
	)
	.await?
	.as_deref()
	.map(decode_pitr_interval_coverage)
	.transpose()
}

async fn read_pitr_interval_txid(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
) -> Result<Option<u64>> {
	Ok(
		read_pitr_interval_coverage(test_ctx, database_branch_id, bucket_start_ms)
			.await?
			.map(|coverage| coverage.txid),
	)
}

async fn read_bucket_branch_id(test_ctx: &TestCtx) -> Result<BucketBranchId> {
	let db = test_ctx.pools().udb()?;
	db.run(|tx| async move {
		branch::resolve_bucket_branch(
			&tx,
			BucketId::from_gas_id(test_bucket()),
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
		.ok_or_else(|| anyhow::anyhow!("bucket branch should exist"))
	})
	.await
}

async fn read_prefix_values(
	test_ctx: &TestCtx,
	prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| {
		let prefix = prefix.clone();
		async move {
			let prefix_subspace =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
			let rows = tx
				.informal()
				.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: universaldb::options::StreamingMode::WantAll,
						..universaldb::RangeOption::from(&prefix_subspace)
					},
					Snapshot,
				)
				.try_collect::<Vec<_>>()
				.await?;

			Ok(rows
				.into_iter()
				.map(|entry| (entry.key().to_vec(), entry.value().to_vec()))
				.collect())
		}
	})
	.await
}

async fn seed_manager_branch(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	head_txid: u64,
	root: Option<CompactionRoot>,
	dirty: Option<SqliteCmpDirty>,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	let bucket_branch =
		BucketBranchId::from_uuid(Uuid::from_u128(0x9999_8888_7777_6666_5555_4444_3333_2222));
	db.run(move |tx| {
		let root = root.clone();
		let dirty = dirty.clone();
		async move {
			let branch_record = DatabaseBranchRecord {
				branch_id: database_branch_id,
				bucket_branch,
				parent: None,
				parent_versionstamp: None,
				root_versionstamp: [0; 16],
				fork_depth: 0,
				created_at_ms: 1_000,
				created_from_restore_point: None,
				state: BranchState::Live,
				lifecycle_generation: 0,
			};
			tx.informal().set(
				&branches_list_key(database_branch_id),
				&encode_database_branch_record(branch_record)?,
			);
			tx.informal().set(
				&branch_meta_head_key(database_branch_id),
				&encode_db_head(DBHead {
					head_txid,
					db_size_pages: 2,
					post_apply_checksum: 0,
					branch_id: database_branch_id,
					#[cfg(debug_assertions)]
					generation: 0,
				})?,
			);
			for txid in 1..=head_txid {
				let mut versionstamp = [0; 16];
				versionstamp[8..16].copy_from_slice(&txid.to_be_bytes());
				tx.informal().set(
					&branch_commit_key(database_branch_id, txid),
					&encode_commit_row(CommitRow {
						wall_clock_ms: 1_000 + i64::try_from(txid).unwrap_or(i64::MAX),
						versionstamp,
						db_size_pages: 2,
						post_apply_checksum: txid,
					})?,
				);
				tx.informal().set(
					&branch_vtx_key(database_branch_id, versionstamp),
					&txid.to_be_bytes(),
				);
				let delta_blob = encode_ltx_v3(
					LtxHeader::delta(txid, 1, 1_000 + i64::try_from(txid).unwrap_or(i64::MAX)),
					&[DirtyPage {
						pgno: 1,
						bytes: vec![txid as u8; PAGE_SIZE as usize],
					}],
				)?;
				tx.informal().set(
					&branch_delta_chunk_key(database_branch_id, txid, 0),
					&delta_blob,
				);
			}
			tx.informal().set(
				&branch_pidx_key(database_branch_id, 1),
				&head_txid.to_be_bytes(),
			);
			if let Some(root) = root {
				tx.informal().set(
					&branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root)?,
				);
			}
			if let Some(dirty) = dirty {
				tx.informal().set(
					&sqlite_cmp_dirty_key(database_branch_id),
					&encode_sqlite_cmp_dirty(dirty)?,
				);
			}
			Ok(())
		}
	})
	.await
}

async fn update_branch_lifecycle(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	state: BranchState,
	lifecycle_generation: u64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		let key = branches_list_key(database_branch_id);
		let record_bytes = tx
			.informal()
			.get(&key, Snapshot)
			.await?
			.expect("database branch record should exist");
		let mut record = depot::types::decode_database_branch_record(&record_bytes)?;
		record.state = state;
		record.lifecycle_generation = lifecycle_generation;
		tx.informal()
			.set(&key, &encode_database_branch_record(record)?);
		Ok(())
	})
	.await
}

async fn clear_branch_record(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		tx.informal().clear(&branches_list_key(database_branch_id));
		Ok(())
	})
	.await
}

async fn seed_restore_point_db_pin(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	at_txid: u64,
) -> Result<RestorePointId> {
	let restore_point =
		RestorePointId::format(1_000 + i64::try_from(at_txid).unwrap_or(i64::MAX), at_txid)?;
	let db = test_ctx.pools().udb()?;
	db.run({
		let restore_point = restore_point.clone();
		move |tx| {
			let restore_point = restore_point.clone();
			async move {
				let commit_bytes = tx
					.informal()
					.get(&branch_commit_key(database_branch_id, at_txid), Snapshot)
					.await?
					.expect("pinned commit row should exist");
				let commit = decode_commit_row(&commit_bytes)?;
				history_pin::write_restore_point_pin(
					&tx,
					database_branch_id,
					restore_point,
					commit.versionstamp,
					at_txid,
					commit.wall_clock_ms,
				)
			}
		}
	})
	.await?;

	Ok(restore_point)
}

async fn seed_pitr_interval_coverage(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
	txid: u64,
	expires_at_ms: i64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		let commit_bytes = tx
			.informal()
			.get(&branch_commit_key(database_branch_id, txid), Snapshot)
			.await?
			.expect("PITR interval commit row should exist");
		let commit = decode_commit_row(&commit_bytes)?;
		tx.informal().set(
			&branch_pitr_interval_key(database_branch_id, bucket_start_ms),
			&encode_pitr_interval_coverage(PitrIntervalCoverage {
				txid,
				versionstamp: commit.versionstamp,
				wall_clock_ms: commit.wall_clock_ms,
				expires_at_ms,
			})?,
		);
		Ok(())
	})
	.await
}

async fn publish_test_shard_and_clear_pidx(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	as_of_txid: u64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		let shard_blob = encode_ltx_v3(
			LtxHeader::delta(as_of_txid, 1, 1_000),
			&[DirtyPage {
				pgno: 1,
				bytes: vec![as_of_txid as u8; PAGE_SIZE as usize],
			}],
		)?;
		tx.informal().set(
			&branch_shard_key(database_branch_id, 0, as_of_txid),
			&shard_blob,
		);
		tx.informal().clear(&branch_pidx_key(database_branch_id, 1));
		Ok(())
	})
	.await
}

async fn set_test_pidx(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_pidx_key(database_branch_id, 1), &txid.to_be_bytes());
		Ok(())
	})
	.await
}

async fn clear_hot_rows_for_cold_read(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		tx.informal()
			.clear(&branch_shard_key(database_branch_id, 0, txid));
		tx.informal()
			.clear(&branch_delta_chunk_key(database_branch_id, txid, 0));
		tx.informal().clear(&branch_pidx_key(database_branch_id, 1));
		Ok(())
	})
	.await
}

async fn set_branch_access_bucket(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	bucket: i64,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		tx.informal().set(
			&branch_manifest_last_access_bucket_key(database_branch_id),
			&bucket.to_le_bytes(),
		);
		Ok(())
	})
	.await
}

async fn run_reclaim_force(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	request_id: Id,
) -> Result<ForceCompactionResult> {
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		test_ctx,
		manager_workflow_id,
		database_branch_id,
		request_id,
		ForceCompactionWork {
			hot: false,
			cold: false,
			reclaim: true,
			final_settle: false,
		},
	)
	.await
}

async fn seed_workflow_cold_ref(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	publish_generation: u64,
	object_key: String,
	bytes: Vec<u8>,
) -> Result<ColdShardRef> {
	let content_hash = sha256(&bytes);
	let mut versionstamp = [0; 16];
	versionstamp[8..16].copy_from_slice(&as_of_txid.to_be_bytes());
	let cold_ref = ColdShardRef {
		object_key,
		object_generation_id: Id::new_v1(u16::try_from(as_of_txid).unwrap_or(u16::MAX)),
		shard_id,
		as_of_txid,
		min_txid: as_of_txid,
		max_txid: as_of_txid,
		min_versionstamp: versionstamp,
		max_versionstamp: versionstamp,
		size_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
		content_hash,
		publish_generation,
	};

	let db = test_ctx.pools().udb()?;
	db.run({
		let cold_ref = cold_ref.clone();
		move |tx| {
			let cold_ref = cold_ref.clone();
			let bytes = bytes.clone();
			async move {
				tx.informal().set(
					&branch_compaction_cold_shard_key(database_branch_id, shard_id, as_of_txid),
					&encode_cold_shard_ref(cold_ref)?,
				);
				tx.informal().set(
					&branch_shard_key(database_branch_id, shard_id, as_of_txid),
					&bytes,
				);
				Ok(())
			}
		}
	})
	.await?;

	Ok(cold_ref)
}

async fn seed_bucket_fork_proof(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	source_bucket_branch_id: BucketBranchId,
	target_bucket_branch_id: BucketBranchId,
	fork_txid: u64,
	write_fork_pin_fact: bool,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		let mut fork_versionstamp = [0; 16];
		fork_versionstamp[8..16].copy_from_slice(&fork_txid.to_be_bytes());
		tx.informal().set(
			&bucket_catalog_by_db_key(database_branch_id, source_bucket_branch_id),
			&encode_bucket_catalog_db_fact(BucketCatalogDbFact {
				database_branch_id,
				bucket_branch_id: source_bucket_branch_id,
				catalog_versionstamp: [0; 16],
				tombstone_versionstamp: None,
			})?,
		);
		let fact = BucketForkFact {
			source_bucket_branch_id,
			target_bucket_branch_id,
			fork_versionstamp,
			parent_cap_versionstamp: fork_versionstamp,
		};
		let encoded_fact = encode_bucket_fork_fact(fact)?;
		tx.informal().set(
			&bucket_child_key(
				source_bucket_branch_id,
				fork_versionstamp,
				target_bucket_branch_id,
			),
			&encoded_fact,
		);
		if write_fork_pin_fact {
			tx.informal().set(
				&bucket_fork_pin_key(
					source_bucket_branch_id,
					fork_versionstamp,
					target_bucket_branch_id,
				),
				&encoded_fact,
			);
		}
		Ok(())
	})
	.await
}

#[test]
fn compaction_workflow_names_are_stable() {
	assert_eq!(<DbManagerWorkflow as WorkflowTrait>::NAME, "db_manager");
	assert_eq!(
		<DbHotCompacterWorkflow as WorkflowTrait>::NAME,
		"db_hot_compacter"
	);
	assert_eq!(
		<DbColdCompacterWorkflow as WorkflowTrait>::NAME,
		"db_cold_compacter"
	);
	assert_eq!(<DbReclaimerWorkflow as WorkflowTrait>::NAME, "db_reclaimer");
}

#[tokio::test]
async fn manager_spawns_companions_and_records_deltas_available() -> Result<()> {
	let database_branch_id = database_branch_id(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-spawns-companions-and-records-deltas-available",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let cold_workflow_id =
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;

			let signal_id = test_ctx
				.signal(DeltasAvailable {
					database_branch_id,
					observed_head_txid: 123,
					dirty_updated_at_ms: 1_714_000_000_000,
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");

			wait_for_signal_ack(&test_ctx, signal_id).await?;
			let manager_state =
				wait_for_manager_cursor(&test_ctx, manager_workflow_id, 123).await?;

			assert_eq!(
				manager_state
					.companion_workflow_ids
					.hot_compacter_workflow_id,
				hot_workflow_id
			);
			assert_eq!(
				manager_state
					.companion_workflow_ids
					.cold_compacter_workflow_id,
				cold_workflow_id
			);
			assert_eq!(
				manager_state.companion_workflow_ids.reclaimer_workflow_id,
				reclaimer_workflow_id
			);
			assert_ne!(hot_workflow_id, cold_workflow_id);
			assert_ne!(hot_workflow_id, reclaimer_workflow_id);
			assert_ne!(cold_workflow_id, reclaimer_workflow_id);
			assert!(manager_state.active_jobs.hot.is_none());
			assert!(manager_state.active_jobs.cold.is_none());
			assert!(manager_state.active_jobs.reclaim.is_none());

			let manager_workflow =
				DatabaseDebug::get_workflows(test_ctx.debug_db(), vec![manager_workflow_id])
					.await?
					.into_iter()
					.next()
					.expect("manager workflow should exist");
			assert_eq!(
				manager_workflow.tags,
				serde_json::json!({ DATABASE_BRANCH_ID_TAG: tag_value })
			);

			assert_eq!(
				<DeltasAvailable as SignalTrait>::NAME,
				"depot_sqlite_cmp_deltas_available"
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_ignores_unrelated_branch_signals_without_mutating_state() -> Result<()> {
	let primary_branch_id = database_branch_id(0x0012_2233_4455_6677_8899_aabb_ccdd_eeff);
	let unrelated_branch_id = database_branch_id(0x0013_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-ignores-unrelated-branch-signals-without-mutating-state",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(primary_branch_id);
			seed_manager_branch(&test_ctx, primary_branch_id, 0, None, None).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(primary_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			wait_for_manager_state(
				&test_ctx,
				manager_workflow_id,
				manager_has_distinct_companions,
			)
			.await?;

			let signal = DeltasAvailable {
				database_branch_id: unrelated_branch_id,
				observed_head_txid: 999,
				dirty_updated_at_ms: 1_714_000_000_000,
			};
			assert_eq!(
				DbManagerSignal::DeltasAvailable(signal.clone()).database_branch_id(),
				unrelated_branch_id
			);
			let signal_id = test_ctx
				.signal(signal)
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let manager_state = latest_manager_state(&test_ctx, manager_workflow_id).await?;
			assert!(manager_state.last_dirty_cursor.is_none());
			assert!(manager_state.force_compactions.pending_requests.is_empty());
			assert!(manager_state.force_compactions.recent_results.is_empty());
			assert_eq!(manager_state.branch_stop_state, BranchStopState::Running);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn companion_ignores_unrelated_branch_signals_without_mutating_state() -> Result<()> {
	let primary_branch_id = database_branch_id(0x0014_2233_4455_6677_8899_aabb_ccdd_eeff);
	let unrelated_branch_id = database_branch_id(0x0015_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-companion-ignores-unrelated-branch-signals-without-mutating-state",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(primary_branch_id);
			seed_manager_branch(&test_ctx, primary_branch_id, 0, None, None).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(primary_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, primary_branch_id).await?;

			let signal = RunHotJob {
				database_branch_id: unrelated_branch_id,
				job_id: Id::new_v1(70),
				job_kind: CompactionJobKind::Hot,
				base_lifecycle_generation: 0,
				base_manifest_generation: 0,
				input_fingerprint: [0; 32],
				status: CompactionJobStatus::Requested,
				input_range: HotJobInputRange {
					txids: TxidRange {
						min_txid: 1,
						max_txid: 1,
					},
					coverage_txids: vec![1],
					max_pages: 1,
					max_bytes: 1,
				},
			};
			assert_eq!(
				DbHotCompacterSignal::RunHotJob(signal.clone()).database_branch_id(),
				unrelated_branch_id
			);
			let signal_id = test_ctx
				.signal(signal)
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot companion workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let companion_state = latest_companion_state(&test_ctx, hot_workflow_id).await?;
			assert_eq!(companion_state, CompanionWorkflowState::Idle);
			assert!(
				read_prefix_values(
					&test_ctx,
					branch_compaction_stage_hot_shard_prefix(unrelated_branch_id, Id::new_v1(70)),
				)
				.await?
				.is_empty()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn companion_destroy_signal_stops_idle_hot_cold_and_reclaim() -> Result<()> {
	let database_branch_id = database_branch_id(0x0016_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-companion-destroy-signal-stops-idle-hot-cold-and-reclaim",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let workflow_ids = [
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?,
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?,
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?,
			];

			for (index, workflow_id) in workflow_ids.into_iter().enumerate() {
				let signal_id = test_ctx
					.signal(DestroyDatabaseBranch {
						database_branch_id,
						lifecycle_generation: 7,
						requested_at_ms: 1_714_000_000_000 + index as i64,
						reason: format!("direct idle companion destroy {index}"),
					})
					.to_workflow_id(workflow_id)
					.send()
					.await?
					.expect("signal should target companion workflow");
				wait_for_signal_ack(&test_ctx, signal_id).await?;
				wait_for_workflow_state(&test_ctx, workflow_id, WorkflowState::Complete).await?;

				let companion_state = latest_companion_state(&test_ctx, workflow_id).await?;
				assert_eq!(
					companion_state,
					CompanionWorkflowState::Stopping {
						active_job: None,
						lifecycle_generation: 7,
						requested_at_ms: 1_714_000_000_000 + index as i64,
						reason: format!("direct idle companion destroy {index}"),
					}
				);
			}

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_destroy_stops_idle_companions() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d10_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-destroy-stops-idle-companions",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let cold_workflow_id =
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;

			let signal_id = test_ctx
				.signal(DestroyDatabaseBranch {
					database_branch_id,
					lifecycle_generation: 0,
					requested_at_ms: 1_714_000_000_000,
					reason: "test destroy".into(),
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete)
				.await?;
			wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, cold_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete)
				.await?;

			let manager_state = latest_manager_state(&test_ctx, manager_workflow_id).await?;
			assert!(manager_state.active_jobs.hot.is_none());
			assert!(manager_state.active_jobs.cold.is_none());
			assert!(manager_state.active_jobs.reclaim.is_none());
			assert!(matches!(
				manager_state.branch_stop_state,
				BranchStopState::Stopped { .. }
			));

			for companion_workflow_id in [hot_workflow_id, cold_workflow_id, reclaimer_workflow_id]
			{
				let destroy =
					single_destroy_signal_for_workflow(&test_ctx, companion_workflow_id).await?;
				assert_eq!(destroy.database_branch_id, database_branch_id);
				assert_eq!(destroy.lifecycle_generation, 0);
				assert_eq!(destroy.requested_at_ms, 1_714_000_000_000);
				assert_eq!(destroy.reason, "test destroy");
			}

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_recreated_for_deleted_branch_stops_without_scheduling() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d11_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-recreated-for-deleted-branch-stops-without-scheduling",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;
			clear_branch_record(&test_ctx, database_branch_id).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let cold_workflow_id =
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;

			wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete)
				.await?;
			wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, cold_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete)
				.await?;
			let run_hot_signals = DatabaseDebug::find_signals(
				test_ctx.debug_db(),
				&[],
				Some(hot_workflow_id),
				Some(<RunHotJob as SignalTrait>::NAME),
				None,
			)
			.await?;
			assert!(run_hot_signals.is_empty());
			let hot_destroy =
				single_destroy_signal_for_workflow(&test_ctx, hot_workflow_id).await?;
			let cold_destroy =
				single_destroy_signal_for_workflow(&test_ctx, cold_workflow_id).await?;
			let reclaimer_destroy =
				single_destroy_signal_for_workflow(&test_ctx, reclaimer_workflow_id).await?;
			for destroy in [&hot_destroy, &cold_destroy, &reclaimer_destroy] {
				assert_eq!(destroy.database_branch_id, database_branch_id);
				assert_eq!(destroy.lifecycle_generation, 0);
				assert_eq!(destroy.reason, "database branch is not live");
			}
			assert_eq!(hot_destroy.requested_at_ms, cold_destroy.requested_at_ms);
			assert_eq!(
				hot_destroy.requested_at_ms,
				reclaimer_destroy.requested_at_ms
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_branch_not_live_stop_clears_active_jobs() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d13_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-branch-not-live-stop-clears-active-jobs",
		build_registry_without_hot_compacter,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let cold_workflow_id =
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
			let run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;
			assert_eq!(run_hot_job.database_branch_id, database_branch_id);
			wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_some()
			})
			.await?;

			clear_branch_record(&test_ctx, database_branch_id).await?;

			wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete)
				.await?;
			wait_for_workflow_state(&test_ctx, cold_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete)
				.await?;

			let manager_state = latest_manager_state(&test_ctx, manager_workflow_id).await?;
			assert!(manager_state.active_jobs.hot.is_none());
			assert!(manager_state.active_jobs.cold.is_none());
			assert!(manager_state.active_jobs.reclaim.is_none());
			assert!(matches!(
				manager_state.branch_stop_state,
				BranchStopState::Stopped { .. }
			));
			let destroy = single_destroy_signal_for_workflow(&test_ctx, hot_workflow_id).await?;
			assert_eq!(destroy.database_branch_id, database_branch_id);
			assert_eq!(destroy.lifecycle_generation, 0);
			assert_eq!(destroy.reason, "database branch is not live");

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_destroy_during_active_hot_job_completes() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d12_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-destroy-during-active-hot-job-completes",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;
			assert_eq!(run_hot_job.job_kind, CompactionJobKind::Hot);
			assert_eq!(run_hot_job.database_branch_id, database_branch_id);
			assert_eq!(run_hot_job.input_range.txids.min_txid, 1);
			assert_eq!(
				run_hot_job.input_range.txids.max_txid,
				quota_threshold_head()
			);

			let signal_id = test_ctx
				.signal(DestroyDatabaseBranch {
					database_branch_id,
					lifecycle_generation: 0,
					requested_at_ms: 1_714_000_000_001,
					reason: "test destroy during hot".into(),
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete)
				.await?;
			wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_rejects_hot_publish_after_lifecycle_generation_bump() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d14_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-rejects-hot-publish-after-lifecycle-generation-bump",
		build_registry_without_hot_compacter,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_some()
			})
			.await?;
			let active_hot_job = manager_state
				.active_jobs
				.hot
				.expect("manager should hold planned hot job active");
			assert_eq!(active_hot_job.job_id, run_hot_job.job_id);
			assert_eq!(active_hot_job.base_lifecycle_generation, 0);

			let staged_blob = encode_ltx_v3(
				LtxHeader::delta(active_hot_job.input_range.txids.min_txid, 1, 1_002),
				&[DirtyPage {
					pgno: 1,
					bytes: page(0x14),
				}],
			)?;
			let output_ref = HotShardOutputRef {
				shard_id: 0,
				as_of_txid: active_hot_job.input_range.txids.max_txid,
				min_txid: active_hot_job.input_range.txids.min_txid,
				max_txid: active_hot_job.input_range.txids.max_txid,
				size_bytes: u64::try_from(staged_blob.len()).unwrap_or(u64::MAX),
				content_hash: sha256(&staged_blob),
			};
			test_ctx
				.pools()
				.udb()?
				.run({
					let staged_blob = staged_blob.clone();
					let active_hot_job = active_hot_job.clone();
					let output_ref = output_ref.clone();
					move |tx| {
						let staged_blob = staged_blob.clone();
						async move {
							tx.informal().set(
								&branch_compaction_stage_hot_shard_key(
									database_branch_id,
									active_hot_job.job_id,
									output_ref.shard_id,
									output_ref.as_of_txid,
									0,
								),
								&staged_blob,
							);
							Ok(())
						}
					}
				})
				.await?;
			update_branch_lifecycle(&test_ctx, database_branch_id, BranchState::Live, 1).await?;

			let signal_id = test_ctx
				.signal(HotJobFinished {
					database_branch_id,
					job_id: active_hot_job.job_id,
					job_kind: CompactionJobKind::Hot,
					base_manifest_generation: active_hot_job.base_manifest_generation,
					input_fingerprint: active_hot_job.input_fingerprint,
					status: CompactionJobStatus::Succeeded,
					output_refs: vec![output_ref],
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state
					.active_jobs
					.hot
					.as_ref()
					.is_some_and(|job| job.base_lifecycle_generation == 1)
			})
			.await?;
			let rescheduled_hot_job = manager_state
				.active_jobs
				.hot
				.expect("manager should reschedule hot work at the new generation");
			assert_eq!(rescheduled_hot_job.base_lifecycle_generation, 1);
			assert!(
				read_value(
					&test_ctx,
					branch_shard_key(database_branch_id, 0, quota_threshold_head()),
				)
				.await?
				.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_refresh_clears_idle_dirty_marker_without_planning_hot_job() -> Result<()> {
	let database_branch_id = database_branch_id(0x1010_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-refresh-clears-idle-dirty-marker-without-planning-hot-job",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: 1,
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_dirty_marker_cleared(&test_ctx, database_branch_id).await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.planning_deadlines.next_hot_check_at_ms.is_some()
					&& state.planning_deadlines.next_cold_check_at_ms.is_some()
					&& state.planning_deadlines.next_reclaim_check_at_ms.is_some()
					&& state.planning_deadlines.final_settle_check_at_ms.is_some()
			})
			.await?;

			assert!(manager_state.active_jobs.hot.is_none());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_refresh_plans_first_hot_job_from_fdb_state() -> Result<()> {
	let database_branch_id = database_branch_id(0x2020_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-refresh-plans-first-hot-job-from-fdb-state",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_none()
			})
			.await?;

			assert!(manager_state.active_jobs.hot.is_none());
			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1))
					.await?
					.is_none()
			);
			assert!(
				read_value(
					&test_ctx,
					branch_shard_key(database_branch_id, 0, quota_threshold_head()),
				)
				.await?
				.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn duplicate_deltas_available_does_not_create_duplicate_hot_job() -> Result<()> {
	let database_branch_id = database_branch_id(0x3030_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-duplicate-deltas-available-does-not-create-duplicate-hot-job",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;

			let signal_id = test_ctx
				.signal(DeltasAvailable {
					database_branch_id,
					observed_head_txid: 99,
					dirty_updated_at_ms: 1_714_000_000_500,
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;
			let second_state = wait_for_manager_cursor(&test_ctx, manager_workflow_id, 99).await?;
			let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
				.await?
				.as_deref()
				.map(decode_compaction_root)
				.transpose()?
				.expect("hot install should publish compaction root");
			let shard_rows =
				read_prefix_values(&test_ctx, branch_shard_prefix(database_branch_id)).await?;

			assert!(second_state.active_jobs.hot.is_none());
			assert_eq!(root.manifest_generation, 1);
			assert_eq!(root.hot_watermark_txid, quota_threshold_head());
			assert_eq!(shard_rows.len(), 1);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_compaction_noop_records_completion_result() -> Result<()> {
	let database_branch_id = database_branch_id(0x3131_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-force-compaction-noop-records-completion-result",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let request_id = Id::new_v1(42);
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				request_id,
				ForceCompactionWork {
					hot: true,
					cold: true,
					reclaim: true,
					final_settle: true,
				},
			)
			.await?;

			assert_eq!(result.request_id, request_id);
			assert!(result.attempted_job_kinds.is_empty());
			assert!(result.completed_job_ids.is_empty());
			assert!(
				result
					.skipped_noop_reasons
					.contains(&"hot:no-actionable-lag".to_string())
			);
			assert!(
				result
					.skipped_noop_reasons
					.contains(&"reclaim:no-actionable-work".to_string())
			);
			assert!(
				result
					.skipped_noop_reasons
					.contains(&"final-settle:refreshed".to_string())
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_hot_compaction_publishes_planned_work_below_threshold() -> Result<()> {
	let database_branch_id = database_branch_id(0x3232_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-force-hot-compaction-publishes-planned-work-below-threshold",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(&test_ctx, database_branch_id, 1, None, None).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let request_id = Id::new_v1(43);
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				request_id,
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert_eq!(result.completed_job_ids.len(), 1);
			assert!(result.skipped_noop_reasons.is_empty());
			assert!(result.terminal_error.is_none());
			let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
				.await?
				.as_deref()
				.map(decode_compaction_root)
				.transpose()?
				.expect("force hot compaction should publish root");
			assert_eq!(root.hot_watermark_txid, 1);
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1))
					.await?
					.is_none()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_hot_compaction_writes_pitr_interval_coverage() -> Result<()> {
	workflow_matrix!(
		"workflow-force-hot-pitr",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: 10,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x01)], 2, 1_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x02)], 2, 1_004)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x03)], 2, 1_012)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x04)], 2, 1_018)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x05)], 2, 1_029)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(83),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(result.terminal_error.is_none());
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 1_000).await?,
				Some(2)
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 1_010).await?,
				Some(4)
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 1_020).await?,
				Some(5)
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 1_030).await?,
				None
			);
			for txid in [2, 4, 5] {
				assert!(
					read_value(&test_ctx, branch_shard_key(database_branch_id, 0, txid))
						.await?
						.is_some()
				);
			}

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_pitr_timestamp_resolution_uses_force_compacted_interval_coverage() -> Result<()> {
	workflow_matrix!(
		"workflow-pitr-timestamp-resolution",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: FIVE_MINUTES_MS,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let base_ms = 1_700_000_000_000_i64.div_euclid(FIVE_MINUTES_MS) * FIVE_MINUTES_MS;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x11)], 2, base_ms + 60_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x22)], 2, base_ms + 240_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x33)], 2, base_ms + 360_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x44)], 2, base_ms + 660_000)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(database_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(92),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(result.terminal_error.is_none());
			for (bucket_start_ms, expected_txid, requested_ms, expected_fill) in [
				(base_ms, 2, base_ms + 300_000, 0x22),
				(base_ms + FIVE_MINUTES_MS, 3, base_ms + 600_000, 0x33),
				(base_ms + 2 * FIVE_MINUTES_MS, 4, base_ms + 900_000, 0x44),
			] {
				assert_eq!(
					read_pitr_interval_txid(&test_ctx, database_branch_id, bucket_start_ms).await?,
					Some(expected_txid)
				);
				assert!(
					read_value(
						&test_ctx,
						branch_shard_key(database_branch_id, 0, expected_txid)
					)
					.await?
					.is_some()
				);

				let resolved = database_db
					.resolve_restore_target(SnapshotSelector::AtTimestamp {
						timestamp_ms: requested_ms,
					})
					.await?;
				assert_eq!(resolved.kind, SnapshotKind::AtTimestamp);
				assert_eq!(resolved.txid, expected_txid);
				let state = debug::read_at(&database_db, resolved.versionstamp).await?;
				assert_eq!(state.txid, expected_txid);
				assert_eq!(
					state.pages[0].bytes.as_deref(),
					Some(page(expected_fill).as_slice())
				);
			}

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_pitr_timestamp_resolution_uses_previous_commit_through_quiet_period() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-pitr-timestamp-resolution-uses-previous-commit-through-quiet-period",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: FIVE_MINUTES_MS,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let base_ms = 1_700_000_000_000_i64.div_euclid(FIVE_MINUTES_MS) * FIVE_MINUTES_MS;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x51)], 2, base_ms + 60_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x52)], 2, base_ms + 17 * 60_000)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(database_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(93),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, base_ms).await?,
				Some(1)
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, base_ms + FIVE_MINUTES_MS)
					.await?,
				None
			);
			assert_eq!(
				read_pitr_interval_txid(
					&test_ctx,
					database_branch_id,
					base_ms + 2 * FIVE_MINUTES_MS
				)
				.await?,
				None
			);
			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: base_ms + 12 * 60_000,
				})
				.await?;
			assert_eq!(resolved.txid, 1);
			let state = debug::read_at(&database_db, resolved.versionstamp).await?;
			assert_eq!(state.pages[0].bytes.as_deref(), Some(page(0x51).as_slice()));

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_pitr_timestamp_resolution_expires_after_configured_retention() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-pitr-timestamp-resolution-expires-after-configured-retention",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: 100,
					retention_ms: 2_500,
				},
			)
			.await?;
			let committed_at_ms = current_time_ms()?;
			let bucket_start_ms = committed_at_ms.div_euclid(100) * 100;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x61)], 2, committed_at_ms)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(database_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(94),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			let coverage =
				read_pitr_interval_coverage(&test_ctx, database_branch_id, bucket_start_ms)
					.await?
					.expect("force hot compaction should publish PITR coverage");
			assert_eq!(coverage.txid, 1);
			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: committed_at_ms,
				})
				.await?;
			assert_eq!(resolved.txid, 1);
			let state = debug::read_at(&database_db, resolved.versionstamp).await?;
			assert_eq!(state.pages[0].bytes.as_deref(), Some(page(0x61).as_slice()));

			wait_until("PITR interval expiry", || async {
				if current_time_ms()? > coverage.expires_at_ms {
					return Ok(Some(()));
				}

				Ok(None)
			})
			.await?;
			let err = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: committed_at_ms,
				})
				.await
				.expect_err("expired PITR interval should reject timestamp resolution");
			assert_storage_error(&err, SqliteStorageError::RestoreTargetExpired);
			assert!(
				read_pitr_interval_coverage(&test_ctx, database_branch_id, bucket_start_ms)
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_restore_point_remains_readable_after_interval_coverage_expires() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-restore-point-remains-readable-after-interval-coverage-expires",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: 100,
					retention_ms: 2_500,
				},
			)
			.await?;
			let committed_at_ms = current_time_ms()?;
			let bucket_start_ms = committed_at_ms.div_euclid(100) * 100;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x62)], 2, committed_at_ms)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(database_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(95),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::AtTimestamp {
					timestamp_ms: committed_at_ms,
				})
				.await?;
			let coverage =
				read_pitr_interval_coverage(&test_ctx, database_branch_id, bucket_start_ms)
					.await?
					.expect("force hot compaction should publish PITR coverage");

			wait_until("PITR interval expiry", || async {
				if current_time_ms()? > coverage.expires_at_ms {
					return Ok(Some(()));
				}

				Ok(None)
			})
			.await?;
			let err = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: committed_at_ms,
				})
				.await
				.expect_err("timestamp selector should expire without interval coverage");
			assert_storage_error(&err, SqliteStorageError::RestoreTargetExpired);
			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::RestorePoint {
					restore_point: restore_point.clone(),
				})
				.await?;
			assert_eq!(resolved.txid, 1);
			let state = debug::read_at(&database_db, resolved.versionstamp).await?;
			assert_eq!(state.pages[0].bytes.as_deref(), Some(page(0x62).as_slice()));
			assert!(
				read_value(
					&test_ctx,
					db_pin_key(
						database_branch_id,
						&history_pin::restore_point_pin_id(&restore_point)
					),
				)
				.await?
				.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_fork_and_restore_from_timestamp_selector_read_resolved_commit() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-fork-and-restore-from-timestamp-selector-read-resolved-commit",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: FIVE_MINUTES_MS,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let base_ms = 1_700_000_000_000_i64.div_euclid(FIVE_MINUTES_MS) * FIVE_MINUTES_MS;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x71)], 2, base_ms + 60_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x72)], 2, base_ms + 360_000)
				.await?;
			let old_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(old_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(old_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				old_branch_id,
				Id::new_v1(96),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			let selector = SnapshotSelector::AtTimestamp {
				timestamp_ms: base_ms + 300_000,
			};
			let resolved = database_db.resolve_restore_target(selector.clone()).await?;
			assert_eq!(resolved.txid, 1);
			assert_eq!(
				debug::read_at(&database_db, resolved.versionstamp)
					.await?
					.pages[0]
					.bytes
					.as_deref(),
				Some(page(0x71).as_slice())
			);

			let forked_database_id = branch::fork_database(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				selector.clone(),
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let forked_db = make_test_db_for(&test_ctx, forked_database_id.clone())?;
			assert_eq!(
				forked_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x71)),
				}]
			);
			let forked_branch_id =
				read_named_database_branch_id(&test_ctx, &forked_database_id).await?;
			let forked_head_at_fork =
				read_value(&test_ctx, branch_meta_head_at_fork_key(forked_branch_id))
					.await?
					.expect("forked branch should store head_at_fork");
			assert_eq!(decode_db_head(&forked_head_at_fork)?.head_txid, 1);

			let undo_restore_point = database_db.restore_database(selector).await?;
			let restored_db = make_test_db(&test_ctx)?;
			assert_eq!(
				restored_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x71)),
				}]
			);
			let restored_branch_id = read_database_branch_id(&test_ctx).await?;
			assert_ne!(restored_branch_id, old_branch_id);
			let restored_head_at_fork =
				read_value(&test_ctx, branch_meta_head_at_fork_key(restored_branch_id))
					.await?
					.expect("restored branch should store head_at_fork");
			assert_eq!(decode_db_head(&restored_head_at_fork)?.head_txid, 1);
			assert!(
				read_value(
					&test_ctx,
					db_pin_key(
						old_branch_id,
						&history_pin::restore_point_pin_id(&undo_restore_point)
					),
				)
				.await?
				.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn hot_compacter_rejects_stale_pitr_interval_selection() -> Result<()> {
	workflow_matrix!(
		"workflow-hot-compacter-rejects-stale-pitr-interval-selection",
		build_registry,
		|_tier, test_ctx| {
			let udb = test_ctx.pools().udb()?;
			set_bucket_pitr_policy(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				PitrPolicy {
					interval_ms: 5,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x01)], 2, 1_000)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x02)], 2, 1_004)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x03)], 2, 1_012)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x04)], 2, 1_018)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			set_database_pitr_policy_override(
				&*udb,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE,
				PitrPolicy {
					interval_ms: 10,
					retention_ms: 9_000_000_000_000,
				},
			)
			.await?;
			let stale_job_id = Id::new_v1(84);

			let signal_id = test_ctx
				.signal(RunHotJob {
					database_branch_id,
					job_id: stale_job_id,
					job_kind: CompactionJobKind::Hot,
					base_lifecycle_generation: 0,
					base_manifest_generation: 0,
					input_fingerprint: [9; 32],
					status: CompactionJobStatus::Requested,
					input_range: HotJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 4,
						},
						coverage_txids: vec![2, 3, 4],
						max_pages: 1,
						max_bytes: 1,
					},
				})
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot compacter workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;
			let finished =
				wait_for_hot_job_finished_signal(&test_ctx, manager_workflow_id, stale_job_id)
					.await?;

			assert_eq!(finished.job_id, stale_job_id);
			assert!(matches!(
				finished.status,
				CompactionJobStatus::Rejected { ref reason }
					if reason == "hot compaction coverage targets changed"
			));
			assert!(finished.output_refs.is_empty());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn hot_install_rejects_staged_output_after_concurrent_commit() -> Result<()> {
	workflow_matrix!(
		"workflow-hot-install-rejects-staged-output-after-concurrent-commit",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0xa1)], 2, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			wait_for_manager_state(
				&test_ctx,
				manager_workflow_id,
				manager_has_distinct_companions,
			)
			.await?;

			let (_pause_guard, reached_hot_stage, release_hot_stage) =
				test_hooks::pause_after_hot_stage(database_branch_id);
			let request_id = Id::new_v1(64);
			let signal_id = test_ctx
				.signal(ForceCompaction {
					database_branch_id,
					request_id,
					requested_work: ForceCompactionWork {
						hot: true,
						cold: false,
						reclaim: false,
						final_settle: false,
					},
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			tokio::time::timeout(Duration::from_secs(5), reached_hot_stage.notified()).await?;
			let active_hot_job = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_some()
			})
			.await?
			.active_jobs
			.hot
			.expect("manager should hold the staged hot job active");
			assert_eq!(active_hot_job.input_range.txids.max_txid, 1);
			let staged_rows =
				wait_for_staged_hot_rows(&test_ctx, database_branch_id, active_hot_job.job_id)
					.await?;
			assert!(!staged_rows.is_empty());

			database_db
				.commit(vec![dirty_page(1, 0xa2)], 2, 1_002)
				.await?;
			release_hot_stage.notify_one();
			drop(_pause_guard);

			wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_none()
			})
			.await?;
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_none()
			);
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0xa2)),
				}]
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_reclaim_waits_for_reclaim_completion() -> Result<()> {
	let database_branch_id = database_branch_id(0x3333_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-force-reclaim-waits-for-reclaim-completion",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 1,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 1).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let request_id = Id::new_v1(44);
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				request_id,
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Reclaim]);
			assert_eq!(result.completed_job_ids.len(), 1);
			assert!(result.terminal_error.is_none());
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_none()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_reclaim_reports_pidx_safety_gate() -> Result<()> {
	let database_branch_id = database_branch_id(0x3334_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-force-reclaim-reports-pidx-safety-gate",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 1,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(46),
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert!(result.attempted_job_kinds.is_empty());
			assert!(result.completed_job_ids.is_empty());
			assert_eq!(
				result.skipped_noop_reasons,
				vec!["reclaim:pidx-dependencies".to_string()]
			);
			assert!(result.terminal_error.is_none());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_force_hot_compaction_preserves_reads_after_pidx_clear() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-force-hot-compaction-preserves-reads-after-pidx-clear",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x11), dirty_page(2, 0x22)], 3, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(45),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(result.terminal_error.is_none());
			assert_eq!(
				database_db.get_pages(vec![1, 2]).await?,
				vec![
					FetchedPage {
						pgno: 1,
						bytes: Some(page(0x11)),
					},
					FetchedPage {
						pgno: 2,
						bytes: Some(page(0x22)),
					},
				]
			);
			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1))
					.await?
					.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 2))
					.await?
					.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_some()
			);
			let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
				.await?
				.as_deref()
				.map(decode_compaction_root)
				.transpose()?
				.expect("hot force compaction should publish a root");
			assert_eq!(root.hot_watermark_txid, 1);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_force_reclaim_removes_hot_rows_and_keeps_reads() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-force-reclaim-removes-hot-rows-and-keeps-reads",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x33)], 2, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let commit = read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
				.await?
				.as_deref()
				.map(decode_commit_row)
				.transpose()?
				.expect("commit should exist before reclaim");
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(46),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(47),
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert!(
				result.attempted_job_kinds.is_empty()
					|| result.attempted_job_kinds == vec![CompactionJobKind::Reclaim]
			);
			assert!(result.terminal_error.is_none());
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x33)),
				}]
			);
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_none()
			);
			assert!(
				read_value(
					&test_ctx,
					branch_vtx_key(database_branch_id, commit.versionstamp)
				)
				.await?
				.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_force_compaction_preserves_exact_restore_point_txid() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-force-compaction-preserves-exact-restore-point-txid",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x41)], 2, 1_001)
				.await?;
			let restore_point = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			database_db
				.commit(vec![dirty_page(1, 0x42)], 2, 1_002)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(48),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(49),
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x42)),
				}]
			);
			let pinned_shard = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
				.await?
				.expect("pinned txid shard should be published exactly");
			let latest_shard = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 2))
				.await?
				.expect("latest txid shard should be published");
			assert_eq!(
				decode_ltx_v3(&pinned_shard)?.get_page(1),
				Some(page(0x41).as_slice())
			);
			assert_eq!(
				decode_ltx_v3(&latest_shard)?.get_page(1),
				Some(page(0x42).as_slice())
			);
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_some()
			);
			let pin_bytes = read_value(
				&test_ctx,
				db_pin_key(
					database_branch_id,
					&history_pin::restore_point_pin_id(&restore_point),
				),
			)
			.await?
			.expect("restore_point DB_PIN should exist");
			let pin = decode_db_history_pin(&pin_bytes)?;
			assert_eq!(pin.kind, DbHistoryPinKind::RestorePoint);
			assert_eq!(pin.at_txid, 1);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_force_reclaim_materializes_bucket_fork_pin() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-force-reclaim-materializes-bucket-fork-pin",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x51)], 2, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let source_bucket_branch_id = read_bucket_branch_id(&test_ctx).await?;
			let fork_commit = read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
				.await?
				.as_deref()
				.map(decode_commit_row)
				.transpose()?
				.expect("fork-point commit should exist");
			let udb_pool = test_ctx.pools().udb()?;
			let udb = Arc::new((*udb_pool).clone());
			let forked_bucket = branch::fork_bucket(
				udb.as_ref(),
				BucketId::from_gas_id(test_bucket()),
				ResolvedVersionstamp {
					versionstamp: fork_commit.versionstamp,
					restore_point: None,
				},
			)
			.await?;
			database_db
				.commit(vec![dirty_page(1, 0x52)], 2, 1_002)
				.await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(50),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			let result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(51),
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert!(
				result.attempted_job_kinds.is_empty()
					|| result.attempted_job_kinds == vec![CompactionJobKind::Reclaim]
			);
			assert!(result.terminal_error.is_none());
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x52)),
				}]
			);
			let forked_bucket_branch_id = udb
				.run(move |tx| async move {
					branch::resolve_bucket_branch(
						&tx,
						forked_bucket,
						universaldb::utils::IsolationLevel::Serializable,
					)
					.await?
					.ok_or_else(|| anyhow::anyhow!("forked bucket branch should exist"))
				})
				.await?;
			assert!(
				read_value(
					&test_ctx,
					bucket_fork_pin_key(
						source_bucket_branch_id,
						fork_commit.versionstamp,
						forked_bucket_branch_id,
					),
				)
				.await?
				.is_some()
			);
			let pin_bytes = read_value(
				&test_ctx,
				db_pin_key(
					database_branch_id,
					&history_pin::bucket_fork_pin_id(forked_bucket_branch_id),
				),
			)
			.await?
			.expect("bucket-derived DB_PIN should be materialized");
			let pin = decode_db_history_pin(&pin_bytes)?;
			assert_eq!(pin.kind, DbHistoryPinKind::BucketFork);
			assert_eq!(pin.at_txid, 1);
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn force_cold_compaction_is_noop_when_cold_storage_disabled() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db
		.commit(vec![dirty_page(1, 0x41)], 2, 1_001)
		.await?;
	let _restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(151),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let result = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(152),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;

	assert!(result.attempted_job_kinds.is_empty());
	assert!(result.terminal_error.is_none());
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn cold_disabled_read_missing_fdb_shard_returns_error() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db
		.commit(vec![dirty_page(1, 0x43)], 2, 1_001)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(155),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let shard_bytes = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
		.await?
		.expect("hot compaction should publish FDB shard coverage");
	let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("hot compaction should publish root");
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		root.manifest_generation,
		"db/cold-disabled/unreachable-shard.ltx".to_string(),
		shard_bytes,
	)
	.await?;
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		tx.informal()
			.clear(&branch_shard_key(database_branch_id, 0, 1));
		Ok(())
	})
	.await?;

	let missing = database_db.get_pages(vec![1]).await;
	assert_storage_error(
		&missing.expect_err(
			"cold-disabled reads must fail when the authoritative FDB shard is missing",
		),
		SqliteStorageError::ShardCoverageMissing { pgno: 1 },
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn configured_cold_storage_publishes_and_reads_workflow_cold_refs() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-configured-cold-")
		.tempdir()?;
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let tier = cold_tier_from_config(test_ctx.config())
		.await?
		.expect("configured cold tier should be enabled");
	let database_db = configured_test_db(&test_ctx, tier.clone())?;
	database_db
		.commit(vec![dirty_page(1, 0x42)], 2, 1_001)
		.await?;
	let _restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(153),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let result = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(154),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;

	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Cold]);
	assert!(result.terminal_error.is_none());
	let cold_ref = wait_for_cold_publish(&test_ctx, database_branch_id, 1).await?;
	assert!(tier.get_object(&cold_ref.object_key).await?.is_some());
	clear_hot_rows_for_cold_read(&test_ctx, database_branch_id, 1).await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0x42)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_clear_old_cold_backed_shard_cache_rows() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-evict-")
		.tempdir()?;
	let evicted = metrics::SQLITE_SHARD_CACHE_EVICTION_TOTAL
		.with_label_values(&[metrics::SHARD_CACHE_EVICTION_CLEARED]);
	let evicted_before = evicted.get();
	let database_branch_id = database_branch_id(0xe001_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(1),
		}],
	)?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		format!(
			"db/{}/shard/00000000/0000000000000001-cache.ltx",
			database_branch_id.as_uuid().simple()
		),
		shard_bytes,
	)
	.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(89)).await?;

	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Reclaim]);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_none()
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_some()
	);
	assert!(evicted.get() >= evicted_before + 1);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_eviction_preserves_future_pin_reads_via_cold_ref() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-future-pin-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let database_db = make_test_db_with_cold_tier(&test_ctx, tier.clone())?;
	database_db
		.commit(vec![dirty_page(1, 0x71)], 2, 1_001)
		.await?;
	database_db
		.commit(vec![dirty_page(2, 0x72)], 2, 1_002)
		.await?;
	let restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(0x71),
		}],
	)?;
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-future-pin.ltx",
		database_branch_id.as_uuid().simple()
	);
	tier.put_object(&object_key, &shard_bytes).await?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		object_key,
		shard_bytes,
	)
	.await?;
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| async move {
			tx.informal().set(
				&branch_compaction_root_key(database_branch_id),
				&encode_compaction_root(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 0,
					cold_watermark_txid: 1,
					cold_watermark_versionstamp: [0; 16],
				})?,
			);
			Ok(())
		})
		.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(95)).await?;

	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Reclaim]);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_none()
	);
	assert!(
		read_value(
			&test_ctx,
			db_pin_key(
				database_branch_id,
				&history_pin::restore_point_pin_id(&restore_point)
			),
		)
		.await?
		.as_deref()
		.map(decode_db_history_pin)
		.transpose()?
		.is_some_and(|pin| pin.at_txid == 2)
	);
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| async move {
			tx.informal().clear(&branch_pidx_key(database_branch_id, 1));
			Ok(())
		})
		.await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0x71)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_keep_recently_accessed_shard_cache_rows() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-recent-")
		.tempdir()?;
	let database_branch_id = database_branch_id(0xe002_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	set_branch_access_bucket(&test_ctx, database_branch_id, i64::MAX).await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(2),
		}],
	)?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		format!(
			"db/{}/shard/00000000/0000000000000001-recent.ltx",
			database_branch_id.as_uuid().simple()
		),
		shard_bytes,
	)
	.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(90)).await?;

	assert!(
		result
			.skipped_noop_reasons
			.contains(&"reclaim:no-actionable-work".to_string())
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_require_matching_cold_ref() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-no-ref-")
		.tempdir()?;
	let database_branch_id = database_branch_id(0xe003_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(3),
		}],
	)?;
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| {
			let shard_bytes = shard_bytes.clone();
			async move {
				tx.informal()
					.set(&branch_shard_key(database_branch_id, 0, 1), &shard_bytes);
				Ok(())
			}
		})
		.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(91)).await?;

	assert!(
		result
			.skipped_noop_reasons
			.contains(&"reclaim:no-actionable-work".to_string())
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_reject_hash_mismatched_cold_ref() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-hash-")
		.tempdir()?;
	let database_branch_id = database_branch_id(0xe004_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(4),
		}],
	)?;
	let mut cold_ref = seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		format!(
			"db/{}/shard/00000000/0000000000000001-hash.ltx",
			database_branch_id.as_uuid().simple()
		),
		shard_bytes,
	)
	.await?;
	cold_ref.content_hash = [9; 32];
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| {
			let cold_ref = cold_ref.clone();
			async move {
				tx.informal().set(
					&branch_compaction_cold_shard_key(database_branch_id, 0, 1),
					&encode_cold_shard_ref(cold_ref)?,
				);
				Ok(())
			}
		})
		.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(92)).await?;

	assert!(
		result
			.skipped_noop_reasons
			.contains(&"reclaim:no-actionable-work".to_string())
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_keep_unexpired_interval_pinned_shard() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-shard-cache-interval-")
		.tempdir()?;
	let database_branch_id = database_branch_id(0xe005_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(5),
		}],
	)?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		format!(
			"db/{}/shard/00000000/0000000000000001-interval.ltx",
			database_branch_id.as_uuid().simple()
		),
		shard_bytes,
	)
	.await?;
	let mut versionstamp = [0; 16];
	versionstamp[8..16].copy_from_slice(&1_u64.to_be_bytes());
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| async move {
			tx.informal().set(
				&branch_pitr_interval_key(database_branch_id, 1_000),
				&encode_pitr_interval_coverage(PitrIntervalCoverage {
					txid: 1,
					versionstamp,
					wall_clock_ms: 1_001,
					expires_at_ms: i64::MAX,
				})?,
			);
			Ok(())
		})
		.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(93)).await?;

	assert!(
		result
			.skipped_noop_reasons
			.contains(&"reclaim:no-actionable-work".to_string())
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_evictions_skip_when_cold_storage_is_disabled() -> Result<()> {
	let database_branch_id = database_branch_id(0xe006_2233_4455_6677_8899_aabb_ccdd_eeff);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 1,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	let shard_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(6),
		}],
	)?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		format!(
			"db/{}/shard/00000000/0000000000000001-disabled.ltx",
			database_branch_id.as_uuid().simple()
		),
		shard_bytes,
	)
	.await?;

	let result = run_reclaim_force(&test_ctx, database_branch_id, Id::new_v1(94)).await?;

	assert!(
		result
			.skipped_noop_reasons
			.contains(&"reclaim:no-actionable-work".to_string())
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_force_cold_publish_reads_after_hot_rows_removed() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-force-cold-e2e-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let database_db = make_test_db_with_cold_tier(&test_ctx, tier.clone())?;
	database_db
		.commit(vec![dirty_page(1, 0x61)], 2, 1_001)
		.await?;
	let _restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(52),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let result = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(53),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;

	assert!(
		result.attempted_job_kinds.is_empty()
			|| result.attempted_job_kinds == vec![CompactionJobKind::Cold]
	);
	assert!(result.terminal_error.is_none());
	let cold_ref = wait_for_cold_publish(&test_ctx, database_branch_id, 1).await?;
	assert!(tier.get_object(&cold_ref.object_key).await?.is_some());
	clear_hot_rows_for_cold_read(&test_ctx, database_branch_id, 1).await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0x61)),
		}]
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_some()
	);
	database_db.wait_for_shard_cache_fill_idle_for_test().await;
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_dual_purpose_shard_cache_eviction_reads_and_refills() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-dual-purpose-shard-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let database_db = make_test_db_with_cold_tier(&test_ctx, tier.clone())?;
	database_db
		.commit(vec![dirty_page(1, 0xa3)], 2, 1_001)
		.await?;
	let restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(
			DATABASE_BRANCH_ID_TAG,
			&database_branch_tag_value(database_branch_id),
		)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(193),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let cold = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(194),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	assert_eq!(
		cold.attempted_job_kinds,
		vec![CompactionJobKind::Cold],
		"{cold:?}"
	);
	assert!(cold.terminal_error.is_none(), "{cold:?}");

	let cold_ref = wait_for_cold_publish(&test_ctx, database_branch_id, 1).await?;
	let cold_object = tier
		.get_object(&cold_ref.object_key)
		.await?
		.expect("cold publish should upload shard bytes");
	assert_eq!(sha256(&cold_object), cold_ref.content_hash);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);
	database_db.delete_restore_point(restore_point).await?;

	let reclaim = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(195),
		ForceCompactionWork {
			hot: false,
			cold: false,
			reclaim: true,
			final_settle: false,
		},
	)
	.await?;

	assert_eq!(
		reclaim.attempted_job_kinds,
		vec![CompactionJobKind::Reclaim]
	);
	assert!(reclaim.terminal_error.is_none());
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_none()
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_some()
	);
	assert!(
		read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
			.await?
			.is_none()
	);

	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0xa3)),
		}]
	);
	database_db.wait_for_shard_cache_fill_idle_for_test().await;
	let refilled_shard = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
		.await?
		.expect("cold read should refill the FDB shard cache");
	assert_eq!(
		decode_ltx_v3(&refilled_shard)?.get_page(1),
		Some(page(0xa3).as_slice())
	);
	assert!(
		read_value(
			&test_ctx,
			branch_manifest_last_access_bucket_key(database_branch_id),
		)
		.await?
		.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_cold_disabled_keeps_fdb_shard_and_skips_cold_work() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db
		.commit(vec![dirty_page(1, 0xa4)], 2, 1_001)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(
			DATABASE_BRANCH_ID_TAG,
			&database_branch_tag_value(database_branch_id),
		)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(196),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let cold = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(197),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let reclaim = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(198),
		ForceCompactionWork {
			hot: false,
			cold: false,
			reclaim: true,
			final_settle: false,
		},
	)
	.await?;

	let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("hot compaction should publish root");
	assert!(cold.attempted_job_kinds.is_empty());
	assert!(cold.terminal_error.is_none());
	assert!(reclaim.terminal_error.is_none());
	assert_eq!(root.hot_watermark_txid, 1);
	assert_eq!(root.cold_watermark_txid, 0);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0xa4)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_cold_upload_failure_keeps_fdb_shard_readable() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-upload-failure-")
		.tempdir()?;
	let invalid_root = cold_root.path().join("not-a-directory");
	tokio::fs::write(&invalid_root, b"not a directory").await?;
	let mut test_ctx = test_ctx_with_configured_cold_tier(&invalid_root).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db
		.commit(vec![dirty_page(1, 0xa5)], 2, 1_001)
		.await?;
	let _restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(
			DATABASE_BRANCH_ID_TAG,
			&database_branch_tag_value(database_branch_id),
		)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(199),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let cold_workflow_id =
		wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
	let request_id = Id::new_v1(200);
	let signal_id = test_ctx
		.signal(ForceCompaction {
			database_branch_id,
			request_id,
			requested_work: ForceCompactionWork {
				hot: false,
				cold: true,
				reclaim: false,
				final_settle: false,
			},
		})
		.to_workflow_id(manager_workflow_id)
		.send()
		.await?
		.expect("signal should target manager workflow");
	wait_for_signal_ack(&test_ctx, signal_id).await?;
	let run_cold_job = wait_for_run_cold_job(&test_ctx, cold_workflow_id).await?;
	let cold =
		wait_for_cold_job_finished_signal(&test_ctx, manager_workflow_id, run_cold_job.job_id)
			.await?;

	let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("hot compaction should publish root");
	assert!(matches!(
		cold.status,
		CompactionJobStatus::Rejected { ref reason }
			if reason == "cold shard upload failed"
	));
	assert!(cold.output_refs.is_empty());
	assert_eq!(root.hot_watermark_txid, 1);
	assert_eq!(root.cold_watermark_txid, 0);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0xa5)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn stale_pidx_missing_delta_falls_back_to_fdb_shard() -> Result<()> {
	workflow_matrix!(
		"workflow-stale-pidx-missing-delta-falls-back-to-fdb-shard",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0xa6)], 2, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(
					DATABASE_BRANCH_ID_TAG,
					&database_branch_tag_value(database_branch_id),
				)
				.unique()
				.dispatch()
				.await?;

			force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(201),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			set_test_pidx(&test_ctx, database_branch_id, 1).await?;
			test_ctx
				.pools()
				.udb()?
				.run(move |tx| async move {
					tx.informal()
						.clear(&branch_delta_chunk_key(database_branch_id, 1, 0));
					Ok(())
				})
				.await?;

			assert!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_none()
			);
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_some()
			);
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0xa6)),
				}]
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_workflow_compacts_reclaims_multiple_deltas_and_keeps_reads() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-workflow-compacts-reclaims-multiple-deltas-and-keeps-reads",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			let mut commits = Vec::new();
			for txid in 1..=3 {
				database_db
					.commit(
						vec![dirty_page(1, 0x70 + u8::try_from(txid).unwrap_or(u8::MAX))],
						2,
						1_000 + i64::try_from(txid).unwrap_or(i64::MAX),
					)
					.await?;
			}
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			for txid in 1..=3 {
				let commit = read_value(&test_ctx, branch_commit_key(database_branch_id, txid))
					.await?
					.as_deref()
					.map(decode_commit_row)
					.transpose()?
					.expect("commit row should exist before reclaim");
				commits.push((txid, commit));
			}
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			let hot_result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(60),
				ForceCompactionWork {
					hot: true,
					cold: false,
					reclaim: false,
					final_settle: false,
				},
			)
			.await?;
			let reclaim_result = force_compaction_and_wait_idle(
				&test_ctx,
				manager_workflow_id,
				database_branch_id,
				Id::new_v1(61),
				ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: false,
				},
			)
			.await?;

			assert_eq!(hot_result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(hot_result.terminal_error.is_none());
			assert!(
				reclaim_result.attempted_job_kinds.is_empty()
					|| reclaim_result.attempted_job_kinds == vec![CompactionJobKind::Reclaim]
			);
			assert!(reclaim_result.terminal_error.is_none());
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(0x73)),
				}]
			);
			for (txid, commit) in commits {
				assert!(
					read_value(
						&test_ctx,
						branch_delta_chunk_key(database_branch_id, txid, 0)
					)
					.await?
					.is_none()
				);
				assert!(
					read_value(&test_ctx, branch_commit_key(database_branch_id, txid))
						.await?
						.is_none()
				);
				assert!(
					read_value(
						&test_ctx,
						branch_vtx_key(database_branch_id, commit.versionstamp)
					)
					.await?
					.is_none()
				);
			}
			let root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
				.await?
				.as_deref()
				.map(decode_compaction_root)
				.transpose()?
				.expect("hot compaction should publish a root");
			assert_eq!(root.hot_watermark_txid, 3);
			assert_eq!(root.cold_watermark_txid, 0);
			assert_eq!(
				read_prefix_values(
					&test_ctx,
					branch_compaction_cold_shard_prefix(database_branch_id),
				)
				.await?
				.len(),
				0
			);
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 3))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn e2e_workflow_cold_publish_reclaim_retires_obsolete_object() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-retire-e2e-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let database_db = make_test_db_with_cold_tier(&test_ctx, tier.clone())?;
	database_db
		.commit(vec![dirty_page(1, 0x81)], 2, 1_001)
		.await?;
	let old_restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(62),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(63),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let old_ref = wait_for_cold_publish(&test_ctx, database_branch_id, 1).await?;
	assert!(tier.get_object(&old_ref.object_key).await?.is_some());
	database_db.delete_restore_point(old_restore_point).await?;

	database_db
		.commit(vec![dirty_page(1, 0x82)], 2, 1_002)
		.await?;
	let current_restore_point = database_db
		.create_restore_point(depot::types::SnapshotSelector::Latest)
		.await?;
	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(64),
		ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(65),
		ForceCompactionWork {
			hot: false,
			cold: true,
			reclaim: false,
			final_settle: false,
		},
	)
	.await?;
	let current_ref = wait_for_cold_publish(&test_ctx, database_branch_id, 2).await?;
	assert_ne!(old_ref.object_key, current_ref.object_key);
	assert!(tier.get_object(&current_ref.object_key).await?.is_some());
	database_db
		.delete_restore_point(current_restore_point)
		.await?;

	let reclaim_result = force_compaction_and_wait_idle(
		&test_ctx,
		manager_workflow_id,
		database_branch_id,
		Id::new_v1(66),
		ForceCompactionWork {
			hot: false,
			cold: false,
			reclaim: true,
			final_settle: false,
		},
	)
	.await?;

	assert!(reclaim_result.terminal_error.is_none());
	wait_for_retired_cold_object_state(
		&test_ctx,
		database_branch_id,
		&old_ref.object_key,
		RetiredColdObjectDeleteState::Deleted,
	)
	.await?;
	assert!(tier.get_object(&old_ref.object_key).await?.is_none());
	assert!(tier.get_object(&current_ref.object_key).await?.is_some());
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 2),
		)
		.await?
		.is_some()
	);
	clear_hot_rows_for_cold_read(&test_ctx, database_branch_id, 1).await?;
	clear_hot_rows_for_cold_read(&test_ctx, database_branch_id, 2).await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(0x82)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_workflow_rejects_stale_hot_work_then_stops_on_branch_deletion() -> Result<()> {
	workflow_matrix!(
		"workflow-e2e-workflow-rejects-stale-hot-work-then-stops-on-branch-deletion",
		build_registry,
		|_tier, test_ctx| {
			let database_db = make_test_db(&test_ctx)?;
			database_db
				.commit(vec![dirty_page(1, 0x91)], 2, 1_001)
				.await?;
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			update_branch_lifecycle(&test_ctx, database_branch_id, BranchState::Live, 1).await?;
			let tag_value = database_branch_tag_value(database_branch_id);
			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let cold_workflow_id =
				wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
			let stale_job_id = Id::new_v1(67);

			let signal_id = test_ctx
				.signal(RunHotJob {
					database_branch_id,
					job_id: stale_job_id,
					job_kind: CompactionJobKind::Hot,
					base_lifecycle_generation: 0,
					base_manifest_generation: 0,
					input_fingerprint: [0x67; 32],
					status: CompactionJobStatus::Requested,
					input_range: HotJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 1,
						},
						coverage_txids: vec![1],
						max_pages: 1,
						max_bytes: 1,
					},
				})
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot compacter workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let staged_rows = read_prefix_values(
				&test_ctx,
				branch_compaction_stage_hot_shard_prefix(database_branch_id, stale_job_id),
			)
			.await?;
			assert!(staged_rows.is_empty());

			clear_branch_record(&test_ctx, database_branch_id).await?;
			let signal_id = test_ctx
				.signal(DestroyDatabaseBranch {
					database_branch_id,
					lifecycle_generation: 1,
					requested_at_ms: 1_714_000_000_002,
					reason: "test e2e branch deletion".into(),
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete)
				.await?;
			wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, cold_workflow_id, WorkflowState::Complete).await?;
			wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete)
				.await?;
			assert!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
					.await?
					.is_none()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn hot_compacter_writes_idempotent_staged_shard_output() -> Result<()> {
	let database_branch_id = database_branch_id(0x4040_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-hot-compacter-writes-idempotent-staged-shard-output",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;
			let first_staged_rows =
				wait_for_staged_hot_rows(&test_ctx, database_branch_id, run_hot_job.job_id).await?;

			assert_eq!(first_staged_rows.len(), 1);
			wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
			assert_eq!(
				read_value(&test_ctx, branch_pidx_key(database_branch_id, 1)).await?,
				None
			);

			let signal_id = test_ctx
				.signal(run_hot_job.clone())
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot compacter workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let second_staged_rows = read_prefix_values(
				&test_ctx,
				branch_compaction_stage_hot_shard_prefix(database_branch_id, run_hot_job.job_id),
			)
			.await?;
			assert_eq!(second_staged_rows, first_staged_rows);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn hot_compacter_rejects_stale_base_generation_without_staging() -> Result<()> {
	let database_branch_id = database_branch_id(0x5050_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-hot-compacter-rejects-stale-base-generation-without-staging",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 2,
					hot_watermark_txid: 0,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let stale_job_id = Id::new_v1(42);
			let signal_id = test_ctx
				.signal(RunHotJob {
					database_branch_id,
					job_id: stale_job_id,
					job_kind: CompactionJobKind::Hot,
					base_lifecycle_generation: 0,
					base_manifest_generation: 1,
					input_fingerprint: [7; 32],
					status: CompactionJobStatus::Requested,
					input_range: HotJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 1,
						},
						coverage_txids: vec![1],
						max_pages: 1,
						max_bytes: 1,
					},
				})
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot compacter workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let staged_rows = read_prefix_values(
				&test_ctx,
				branch_compaction_stage_hot_shard_prefix(database_branch_id, stale_job_id),
			)
			.await?;
			assert!(staged_rows.is_empty());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn hot_compacter_rejects_stale_lifecycle_generation_without_staging() -> Result<()> {
	let database_branch_id = database_branch_id(0x5051_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-hot-compacter-rejects-stale-lifecycle-generation-without-staging",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 0,
					hot_watermark_txid: 0,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			update_branch_lifecycle(&test_ctx, database_branch_id, BranchState::Live, 1).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let stale_job_id = Id::new_v1(43);
			let signal_id = test_ctx
				.signal(RunHotJob {
					database_branch_id,
					job_id: stale_job_id,
					job_kind: CompactionJobKind::Hot,
					base_lifecycle_generation: 0,
					base_manifest_generation: 0,
					input_fingerprint: [8; 32],
					status: CompactionJobStatus::Requested,
					input_range: HotJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 1,
						},
						coverage_txids: vec![1],
						max_pages: 1,
						max_bytes: 1,
					},
				})
				.to_workflow_id(hot_workflow_id)
				.send()
				.await?
				.expect("signal should target hot compacter workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			let staged_rows = read_prefix_values(
				&test_ctx,
				branch_compaction_stage_hot_shard_prefix(database_branch_id, stale_job_id),
			)
			.await?;
			assert!(staged_rows.is_empty());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_schedules_cleanup_for_stale_hot_output() -> Result<()> {
	let database_branch_id = database_branch_id(0x5052_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-schedules-cleanup-for-stale-hot-output",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				0,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 0,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			update_branch_lifecycle(&test_ctx, database_branch_id, BranchState::Live, 7).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
			wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.last_observed_branch_lifecycle_generation == Some(7)
			})
			.await?;
			let stale_job_id = Id::new_v1(52);
			let staged_blob = encode_ltx_v3(
				LtxHeader::delta(1, 1, 1_001),
				&[DirtyPage {
					pgno: 1,
					bytes: page(9),
				}],
			)?;
			let output_ref = HotShardOutputRef {
				shard_id: 0,
				as_of_txid: 1,
				min_txid: 1,
				max_txid: 1,
				size_bytes: u64::try_from(staged_blob.len()).unwrap_or(u64::MAX),
				content_hash: sha256(&staged_blob),
			};
			test_ctx
				.pools()
				.udb()?
				.run({
					let staged_blob = staged_blob.clone();
					move |tx| {
						let staged_blob = staged_blob.clone();
						async move {
							tx.informal().set(
								&branch_compaction_stage_hot_shard_key(
									database_branch_id,
									stale_job_id,
									0,
									1,
									0,
								),
								&staged_blob,
							);
							Ok(())
						}
					}
				})
				.await?;

			let signal_id = test_ctx
				.signal(HotJobFinished {
					database_branch_id,
					job_id: stale_job_id,
					job_kind: CompactionJobKind::Hot,
					base_manifest_generation: 1,
					input_fingerprint: [5; 32],
					status: CompactionJobStatus::Succeeded,
					output_refs: vec![output_ref],
				})
				.to_workflow_id(manager_workflow_id)
				.send()
				.await?
				.expect("signal should target manager workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;
			let repair_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
			assert_eq!(repair_job.base_lifecycle_generation, 7);
			assert_eq!(repair_job.input_range.staged_hot_shards.len(), 1);

			wait_for_stage_row_cleared(&test_ctx, database_branch_id, stale_job_id).await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.reclaim.is_none()
			})
			.await?;
			assert!(manager_state.active_jobs.reclaim.is_none());

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_schedules_cleanup_for_stale_cold_output() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-orphan-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let database_branch_id = database_branch_id(0x5053_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		0,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
	wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.last_observed_branch_lifecycle_generation == Some(0)
	})
	.await?;
	let stale_job_id = Id::new_v1(53);
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-orphan.ltx",
		database_branch_id.as_uuid().simple()
	);
	let object_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(10),
		}],
	)?;
	tier.put_object(&object_key, &object_bytes).await?;
	let cold_ref = ColdShardRef {
		object_key: object_key.clone(),
		object_generation_id: stale_job_id,
		shard_id: 0,
		as_of_txid: 1,
		min_txid: 1,
		max_txid: 1,
		min_versionstamp: [1; 16],
		max_versionstamp: [1; 16],
		size_bytes: u64::try_from(object_bytes.len()).unwrap_or(u64::MAX),
		content_hash: sha256(&object_bytes),
		publish_generation: 2,
	};

	let signal_id = test_ctx
		.signal(ColdJobFinished {
			database_branch_id,
			job_id: stale_job_id,
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: 1,
			input_fingerprint: [6; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: vec![cold_ref],
		})
		.to_workflow_id(manager_workflow_id)
		.send()
		.await?
		.expect("signal should target manager workflow");
	wait_for_signal_ack(&test_ctx, signal_id).await?;
	let repair_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
	assert_eq!(repair_job.input_range.orphan_cold_objects.len(), 1);

	wait_for_cold_object_deleted(tier.as_ref(), &object_key).await?;
	let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.active_jobs.reclaim.is_none()
	})
	.await?;
	assert!(manager_state.active_jobs.reclaim.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_schedules_cold_cleanup_intent_when_cold_storage_disabled() -> Result<()> {
	let database_branch_id = database_branch_id(0x5053_3233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		0,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: 0,
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
	wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.last_observed_branch_lifecycle_generation == Some(0)
	})
	.await?;
	let stale_job_id = Id::new_v1(54);
	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-disabled-orphan.ltx",
		database_branch_id.as_uuid().simple()
	);
	let object_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(10),
		}],
	)?;
	let cold_ref = ColdShardRef {
		object_key: object_key.clone(),
		object_generation_id: stale_job_id,
		shard_id: 0,
		as_of_txid: 1,
		min_txid: 1,
		max_txid: 1,
		min_versionstamp: [1; 16],
		max_versionstamp: [1; 16],
		size_bytes: u64::try_from(object_bytes.len()).unwrap_or(u64::MAX),
		content_hash: sha256(&object_bytes),
		publish_generation: 2,
	};

	let signal_id = test_ctx
		.signal(ColdJobFinished {
			database_branch_id,
			job_id: stale_job_id,
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: 1,
			input_fingerprint: [6; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: vec![cold_ref],
		})
		.to_workflow_id(manager_workflow_id)
		.send()
		.await?
		.expect("signal should target manager workflow");
	wait_for_signal_ack(&test_ctx, signal_id).await?;
	let repair_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
	assert_eq!(repair_job.input_range.orphan_cold_objects.len(), 1);
	assert_eq!(
		repair_job.input_range.orphan_cold_objects[0].object_key,
		object_key
	);

	let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.active_jobs.reclaim.is_none()
	})
	.await?;
	assert!(manager_state.active_jobs.reclaim.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_cleans_uploaded_cold_output_when_active_publish_rejects() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-active-reject-")
		.tempdir()?;
	let mut test_ctx = test_ctx_with_configured_cold_tier_and_registry(
		cold_root.path(),
		build_registry_without_cold_compacter(),
	)
	.await?;
	let tier = cold_tier_from_config(test_ctx.config())
		.await?
		.expect("configured cold tier should be enabled");

	let database_branch_id = database_branch_id(0x5054_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		cold_threshold_head(),
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: cold_threshold_head(),
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, cold_threshold_head()).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
	let cold_workflow_id =
		wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
	let run_cold_job = wait_for_run_cold_job(&test_ctx, cold_workflow_id).await?;

	let object_key = format!(
		"db/{}/shard/00000000/{:016x}-{}-manual-reject.ltx",
		database_branch_id.as_uuid().simple(),
		cold_threshold_head(),
		run_cold_job.job_id,
	);
	let object_bytes = encode_ltx_v3(
		LtxHeader::delta(cold_threshold_head(), 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(11),
		}],
	)?;
	tier.put_object(&object_key, &object_bytes).await?;
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| async move {
			let root_key = branch_compaction_root_key(database_branch_id);
			let root_bytes = tx
				.informal()
				.get(&root_key, Snapshot)
				.await?
				.expect("compaction root should exist");
			let mut root = decode_compaction_root(&root_bytes)?;
			root.manifest_generation = root.manifest_generation.saturating_add(1);
			tx.informal().set(&root_key, &encode_compaction_root(root)?);
			Ok(())
		})
		.await?;
	let cold_ref = ColdShardRef {
		object_key: object_key.clone(),
		object_generation_id: run_cold_job.job_id,
		shard_id: 0,
		as_of_txid: cold_threshold_head(),
		min_txid: run_cold_job.input_range.txids.min_txid,
		max_txid: cold_threshold_head(),
		min_versionstamp: run_cold_job.input_range.min_versionstamp,
		max_versionstamp: run_cold_job.input_range.max_versionstamp,
		size_bytes: u64::try_from(object_bytes.len()).unwrap_or(u64::MAX),
		content_hash: sha256(&object_bytes),
		publish_generation: run_cold_job.base_manifest_generation.saturating_add(1),
	};

	let signal_id = test_ctx
		.signal(ColdJobFinished {
			database_branch_id,
			job_id: run_cold_job.job_id,
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: run_cold_job.base_manifest_generation,
			input_fingerprint: run_cold_job.input_fingerprint,
			status: CompactionJobStatus::Succeeded,
			output_refs: vec![cold_ref],
		})
		.to_workflow_id(manager_workflow_id)
		.send()
		.await?
		.expect("signal should target manager workflow");
	wait_for_signal_ack(&test_ctx, signal_id).await?;

	let repair_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
	assert_eq!(repair_job.input_range.orphan_cold_objects.len(), 1);
	assert_eq!(
		repair_job.input_range.orphan_cold_objects[0].object_key,
		object_key
	);
	wait_for_cold_object_deleted(tier.as_ref(), &object_key).await?;
	let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.active_jobs.reclaim.is_none()
	})
	.await?;
	assert!(manager_state.active_jobs.reclaim.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_publishes_hot_output_and_reads_through_shard_after_pidx_clear() -> Result<()> {
	workflow_matrix!(
		"workflow-manager-publishes-hot-output-and-reads-through-shard-after-pidx-clear",
		build_registry,
		|_tier, test_ctx| {
			let udb_pool = test_ctx.pools().udb()?;
			let udb = Arc::new((*udb_pool).clone());
			let database_db = Db::new(udb, test_bucket(), TEST_DATABASE.to_string(), NodeId::new());

			for txid in 1..=quota_threshold_head() {
				database_db
					.commit(
						vec![dirty_page(1, u8::try_from(txid).unwrap_or(u8::MAX))],
						1,
						1_000 + i64::try_from(txid).unwrap_or(i64::MAX),
					)
					.await?;
			}
			let database_branch_id = read_database_branch_id(&test_ctx).await?;
			let tag_value = database_branch_tag_value(database_branch_id);

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.active_jobs.hot.is_none()
			})
			.await?;

			assert!(manager_state.active_jobs.hot.is_none());
			assert_eq!(
				database_db.get_pages(vec![1]).await?,
				vec![FetchedPage {
					pgno: 1,
					bytes: Some(page(
						u8::try_from(quota_threshold_head()).unwrap_or(u8::MAX)
					)),
				}]
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn manager_publishes_cold_output_and_reads_through_cold_ref() -> Result<()> {
	let cold_root = Builder::new().prefix("depot-workflow-cold-").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	let database_db = Db::new_with_cold_tier(
		udb,
		test_bucket(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		tier.clone(),
	);

	database_db.commit(vec![dirty_page(1, 1)], 1, 1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		cold_threshold_head(),
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 1,
			hot_watermark_txid: cold_threshold_head(),
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, cold_threshold_head()).await?;
	let seeded_root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("seeded compaction root should exist");
	assert_eq!(seeded_root.hot_watermark_txid, cold_threshold_head());
	let seeded_head = read_value(&test_ctx, branch_meta_head_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()?
		.expect("seeded head should exist");
	assert_eq!(seeded_head.head_txid, cold_threshold_head());

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let cold_workflow_id =
		wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;

	let run_cold_job = wait_for_run_cold_job(&test_ctx, cold_workflow_id).await?;
	assert_eq!(run_cold_job.job_kind, CompactionJobKind::Cold);
	assert_eq!(run_cold_job.database_branch_id, database_branch_id);
	assert_eq!(run_cold_job.input_range.txids.min_txid, 1);
	assert_eq!(
		run_cold_job.input_range.txids.max_txid,
		cold_threshold_head()
	);
	let cold_ref =
		wait_for_cold_publish(&test_ctx, database_branch_id, cold_threshold_head()).await?;
	let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.active_jobs.cold.is_none()
	})
	.await?;

	assert!(manager_state.active_jobs.cold.is_none());
	assert_eq!(run_cold_job.job_id, cold_ref.object_generation_id);
	assert_eq!(cold_ref.shard_id, 0);
	assert_eq!(cold_ref.as_of_txid, cold_threshold_head());
	assert_eq!(cold_ref.publish_generation, 2);
	assert!(cold_ref.object_key.starts_with(&format!(
		"db/{}/shard/00000000/{:016x}-{}-",
		database_branch_id.as_uuid().simple(),
		cold_threshold_head(),
		run_cold_job.job_id
	)));
	assert!(cold_ref.object_key.ends_with(".ltx"));
	assert!(
		tier.get_object(&cold_ref.object_key)
			.await?
			.expect("cold object should be uploaded")
			.len() > 0
	);

	clear_hot_rows_for_cold_read(&test_ctx, database_branch_id, cold_threshold_head()).await?;
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(cold_threshold_head() as u8)),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_retires_cold_object_before_grace_delete_and_cleanup() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-retire-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let database_branch_id = database_branch_id(0xd1d1_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		2,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 2,
			hot_watermark_txid: 2,
			cold_watermark_txid: 2,
			cold_watermark_versionstamp: {
				let mut versionstamp = [0; 16];
				versionstamp[8..16].copy_from_slice(&2_u64.to_be_bytes());
				versionstamp
			},
		}),
		None,
	)
	.await?;

	let old_key = format!(
		"db/{}/shard/00000000/0000000000000001-old.ltx",
		database_branch_id.as_uuid().simple()
	);
	let current_key = format!(
		"db/{}/shard/00000000/0000000000000002-current.ltx",
		database_branch_id.as_uuid().simple()
	);
	let old_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(1),
		}],
	)?;
	let current_bytes = encode_ltx_v3(
		LtxHeader::delta(2, 1, 1_002),
		&[DirtyPage {
			pgno: 1,
			bytes: page(2),
		}],
	)?;
	tier.put_object(&old_key, &old_bytes).await?;
	tier.put_object(&current_key, &current_bytes).await?;
	let old_ref = seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		old_key.clone(),
		old_bytes,
	)
	.await?;
	seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		2,
		2,
		current_key.clone(),
		current_bytes,
	)
	.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;

	let retired = wait_for_retired_cold_object_state(
		&test_ctx,
		database_branch_id,
		&old_ref.object_key,
		RetiredColdObjectDeleteState::Retired,
	)
	.await?;
	assert_eq!(retired.object_key, old_ref.object_key);
	assert!(tier.get_object(&old_ref.object_key).await?.is_some());
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 2),
		)
		.await?
		.is_some()
	);
	let destroy_signal_id = test_ctx
		.signal(DestroyDatabaseBranch {
			database_branch_id,
			lifecycle_generation: 0,
			requested_at_ms: 1_714_000_000_003,
			reason: "stop reclaimer during cold-object grace window".into(),
		})
		.to_workflow_id(reclaimer_workflow_id)
		.send()
		.await?
		.expect("signal should target reclaimer workflow");

	let retired_root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("retired compaction root should exist");
	assert_eq!(retired_root.manifest_generation, 3);

	wait_for_cold_object_deleted(tier.as_ref(), &old_ref.object_key).await?;
	wait_for_signal_ack(&test_ctx, destroy_signal_id).await?;
	wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete).await?;
	let deleted = wait_for_retired_cold_object_state(
		&test_ctx,
		database_branch_id,
		&old_ref.object_key,
		RetiredColdObjectDeleteState::Deleted,
	)
	.await?;
	assert_eq!(deleted.object_key, old_ref.object_key);
	assert!(tier.get_object(&current_key).await?.is_some());
	let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
		state.active_jobs.reclaim.is_none()
	})
	.await?;
	assert!(manager_state.active_jobs.reclaim.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_logs_and_retains_live_cold_ref_when_s3_object_is_missing() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-missing-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let database_branch_id = database_branch_id(0xd1d2_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		2,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 2,
			hot_watermark_txid: 2,
			cold_watermark_txid: 2,
			cold_watermark_versionstamp: {
				let mut versionstamp = [0; 16];
				versionstamp[8..16].copy_from_slice(&2_u64.to_be_bytes());
				versionstamp
			},
		}),
		None,
	)
	.await?;

	let missing_key = format!(
		"db/{}/shard/00000000/0000000000000001-missing.ltx",
		database_branch_id.as_uuid().simple()
	);
	let missing_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(1),
		}],
	)?;
	let missing_ref = seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		missing_key.clone(),
		missing_bytes,
	)
	.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
	let _run_reclaim_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
	wait_for_reclaim_job_finished_signal(&test_ctx, manager_workflow_id).await?;

	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_some()
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_retired_cold_object_key(
				database_branch_id,
				object_key_hash(&missing_ref.object_key),
			),
		)
		.await?
		.is_none()
	);
	assert!(tier.get_object(&missing_key).await?.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_logs_and_retains_live_cold_ref_for_delete_issued_object() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-delete-issued-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let database_branch_id = database_branch_id(0xd1d3_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		2,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 2,
			hot_watermark_txid: 2,
			cold_watermark_txid: 2,
			cold_watermark_versionstamp: {
				let mut versionstamp = [0; 16];
				versionstamp[8..16].copy_from_slice(&2_u64.to_be_bytes());
				versionstamp
			},
		}),
		None,
	)
	.await?;

	let object_key = format!(
		"db/{}/shard/00000000/0000000000000001-delete-issued.ltx",
		database_branch_id.as_uuid().simple()
	);
	let object_bytes = encode_ltx_v3(
		LtxHeader::delta(1, 1, 1_001),
		&[DirtyPage {
			pgno: 1,
			bytes: page(1),
		}],
	)?;
	tier.put_object(&object_key, &object_bytes).await?;
	let cold_ref = seed_workflow_cold_ref(
		&test_ctx,
		database_branch_id,
		0,
		1,
		1,
		object_key.clone(),
		object_bytes,
	)
	.await?;
	test_ctx
		.pools()
		.udb()?
		.run({
			let object_key = object_key.clone();
			move |tx| {
				let object_key = object_key.clone();
				async move {
					tx.informal().set(
						&branch_compaction_retired_cold_object_key(
							database_branch_id,
							object_key_hash(&object_key),
						),
						&encode_retired_cold_object(RetiredColdObject {
							object_key,
							object_generation_id: cold_ref.object_generation_id,
							content_hash: cold_ref.content_hash,
							retired_manifest_generation: 3,
							retired_at_ms: 1_001,
							delete_after_ms: 1_002,
							delete_state: RetiredColdObjectDeleteState::DeleteIssued,
						})?,
					);
					Ok(())
				}
			}
		})
		.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
	let _run_reclaim_job = wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
	wait_for_reclaim_job_finished_signal(&test_ctx, manager_workflow_id).await?;

	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_some()
	);
	let retired = wait_for_retired_cold_object_state(
		&test_ctx,
		database_branch_id,
		&object_key,
		RetiredColdObjectDeleteState::DeleteIssued,
	)
	.await?;
	assert_eq!(retired.object_key, object_key);
	assert!(tier.get_object(&retired.object_key).await?.is_some());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn cold_compacter_rejects_stale_base_generation_without_publish() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-stale-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	let database_branch_id = database_branch_id(0xd0d0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = test_ctx_with_configured_cold_tier(cold_root.path()).await?;
	seed_manager_branch(
		&test_ctx,
		database_branch_id,
		1,
		Some(CompactionRoot {
			schema_version: 1,
			manifest_generation: 2,
			hot_watermark_txid: 1,
			cold_watermark_txid: 0,
			cold_watermark_versionstamp: [0; 16],
		}),
		None,
	)
	.await?;
	publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 1).await?;

	let _manager_workflow_id = test_ctx
		.workflow(DbManagerInput::new(database_branch_id, None))
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let cold_workflow_id =
		wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;
	let mut versionstamp = [0; 16];
	versionstamp[8..16].copy_from_slice(&1_u64.to_be_bytes());
	let signal_id = test_ctx
		.signal(RunColdJob {
			database_branch_id,
			job_id: Id::new_v1(42),
			job_kind: CompactionJobKind::Cold,
			base_lifecycle_generation: 0,
			base_manifest_generation: 1,
			input_fingerprint: [9; 32],
			status: CompactionJobStatus::Requested,
			input_range: depot::workflows::compaction::ColdJobInputRange {
				txids: TxidRange {
					min_txid: 1,
					max_txid: 1,
				},
				min_versionstamp: versionstamp,
				max_versionstamp: versionstamp,
				max_bytes: 1,
			},
		})
		.to_workflow_id(cold_workflow_id)
		.send()
		.await?
		.expect("signal should target cold compacter workflow");
	wait_for_signal_ack(&test_ctx, signal_id).await?;

	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1),
		)
		.await?
		.is_none()
	);
	assert!(tier.list_prefix("").await?.is_empty());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_hot_planning_materializes_exact_pinned_txid() -> Result<()> {
	let database_branch_id = database_branch_id(0x6060_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-manager-hot-planning-materializes-exact-pinned-txid",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: 100,
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;
			let _restore_point =
				seed_restore_point_db_pin(&test_ctx, database_branch_id, 50).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let hot_workflow_id =
				wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
			let run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;

			assert_eq!(run_hot_job.input_range.txids.max_txid, 100);
			assert_eq!(run_hot_job.input_range.coverage_txids, vec![50, 100]);

			wait_for_hot_install(&test_ctx, database_branch_id, 100).await?;
			let pinned_shard = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 50))
				.await?
				.expect("pinned txid shard should be published");
			let latest_shard = read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 100))
				.await?
				.expect("latest head shard should be published");

			let pinned_decoded = decode_ltx_v3(&pinned_shard)?;
			let latest_decoded = decode_ltx_v3(&latest_shard)?;
			assert_eq!(pinned_decoded.header.max_txid, 50);
			assert_eq!(latest_decoded.header.max_txid, 100);
			assert_eq!(pinned_decoded.get_page(1), Some(page(50).as_slice()));
			assert_eq!(latest_decoded.get_page(1), Some(page(100).as_slice()));
			assert_eq!(
				read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 99)).await?,
				None
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_deletes_obsolete_fdb_rows_after_hot_coverage() -> Result<()> {
	let database_branch_id = database_branch_id(0x7070_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-deletes-obsolete-fdb-rows-after-hot-coverage",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				None,
				Some(SqliteCmpDirty {
					observed_head_txid: quota_threshold_head(),
					updated_at_ms: 1_714_000_000_000,
				}),
			)
			.await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;

			wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
			let run_reclaim_job =
				wait_for_run_reclaim_job(&test_ctx, reclaimer_workflow_id).await?;
			assert_eq!(run_reclaim_job.job_kind, CompactionJobKind::Reclaim);
			assert_eq!(run_reclaim_job.database_branch_id, database_branch_id);
			assert_eq!(run_reclaim_job.input_range.txids.min_txid, 1);
			assert_eq!(
				run_reclaim_job.input_range.txids.max_txid,
				quota_threshold_head()
			);
			assert!(!run_reclaim_job.input_range.txid_refs.is_empty());
			wait_for_reclaim_delete(&test_ctx, database_branch_id, quota_threshold_head()).await?;

			let mut versionstamp = [0; 16];
			versionstamp[8..16].copy_from_slice(&quota_threshold_head().to_be_bytes());
			assert!(
				read_value(&test_ctx, branch_vtx_key(database_branch_id, versionstamp))
					.await?
					.is_none()
			);
			assert!(
				read_value(
					&test_ctx,
					branch_shard_key(database_branch_id, 0, quota_threshold_head()),
				)
				.await?
				.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_retains_rows_when_pidx_still_references_deleted_txid() -> Result<()> {
	let database_branch_id = database_branch_id(0x8080_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-retains-rows-when-pidx-still-references-deleted-txid",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: quota_threshold_head(),
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(
				&test_ctx,
				database_branch_id,
				quota_threshold_head(),
			)
			.await?;
			set_test_pidx(&test_ctx, database_branch_id, 1).await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.planning_deadlines.next_reclaim_check_at_ms.is_some()
			})
			.await?;

			assert!(manager_state.active_jobs.reclaim.is_none());
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_rejects_stale_manifest_generation() -> Result<()> {
	let database_branch_id = database_branch_id(0x9090_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-rejects-stale-manifest-generation",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				1,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 1,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 1).await?;
			set_test_pidx(&test_ctx, database_branch_id, 1).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let reclaimer_workflow_id =
				wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
			let mut versionstamp = [0; 16];
			versionstamp[8..16].copy_from_slice(&1_u64.to_be_bytes());
			let signal_id = test_ctx
				.signal(RunReclaimJob {
					database_branch_id,
					job_id: Id::new_v1(42),
					job_kind: CompactionJobKind::Reclaim,
					base_lifecycle_generation: 0,
					base_manifest_generation: 0,
					input_fingerprint: [3; 32],
					status: CompactionJobStatus::Requested,
					input_range: depot::workflows::compaction::ReclaimJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 1,
						},
						txid_refs: vec![depot::workflows::compaction::ReclaimTxidRef {
							txid: 1,
							versionstamp,
						}],
						cold_objects: Vec::new(),
						shard_cache_evictions: Vec::new(),
						staged_hot_shards: Vec::new(),
						orphan_cold_objects: Vec::new(),
						max_keys: 500,
						max_bytes: 2 * 1024 * 1024,
					},
				})
				.to_workflow_id(reclaimer_workflow_id)
				.send()
				.await?
				.expect("signal should target reclaimer workflow");
			wait_for_signal_ack(&test_ctx, signal_id).await?;

			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_retains_pinned_txid_history() -> Result<()> {
	let database_branch_id = database_branch_id(0xa0a0_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-retains-pinned-txid-history",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			let _restore_point =
				seed_restore_point_db_pin(&test_ctx, database_branch_id, 50).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_reclaim_delete(&test_ctx, database_branch_id, 49).await?;
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 50, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 50))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_retains_unexpired_pitr_interval_history() -> Result<()> {
	let database_branch_id = database_branch_id(0xa1a1_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-retains-unexpired-pitr-interval-history",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			seed_pitr_interval_coverage(&test_ctx, database_branch_id, 5_000, 50, i64::MAX).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_reclaim_delete(&test_ctx, database_branch_id, 49).await?;
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 50, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 50))
					.await?
					.is_some()
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 5_000).await?,
				Some(50)
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_deletes_expired_pitr_interval_and_reclaims_history() -> Result<()> {
	let database_branch_id = database_branch_id(0xa2a2_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-deletes-expired-pitr-interval-and-reclaims-history",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			seed_pitr_interval_coverage(&test_ctx, database_branch_id, 5_000, 50, 0).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_reclaim_delete(&test_ctx, database_branch_id, 100).await?;
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 5_000).await?,
				None
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_keeps_restore_point_after_pitr_interval_expires() -> Result<()> {
	let database_branch_id = database_branch_id(0xa3a3_2233_4455_6677_8899_aabb_ccdd_eeff);
	workflow_matrix!(
		"workflow-reclaimer-keeps-restore-point-after-pitr-interval-expires",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			let restore_point =
				seed_restore_point_db_pin(&test_ctx, database_branch_id, 50).await?;
			seed_pitr_interval_coverage(&test_ctx, database_branch_id, 5_000, 50, 0).await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_reclaim_delete(&test_ctx, database_branch_id, 49).await?;
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 50, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 50))
					.await?
					.is_some()
			);
			assert_eq!(
				read_pitr_interval_txid(&test_ctx, database_branch_id, 5_000).await?,
				None
			);
			assert!(
				read_value(
					&test_ctx,
					db_pin_key(
						database_branch_id,
						&history_pin::restore_point_pin_id(&restore_point)
					),
				)
				.await?
				.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_materializes_bucket_fork_pin_before_delete() -> Result<()> {
	let database_branch_id = database_branch_id(0xb0b0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let source_bucket_branch_id =
		BucketBranchId::from_uuid(Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888));
	let target_bucket_branch_id =
		BucketBranchId::from_uuid(Uuid::from_u128(0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0001));
	workflow_matrix!(
		"workflow-reclaimer-materializes-bucket-fork-pin-before-delete",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			seed_bucket_fork_proof(
				&test_ctx,
				database_branch_id,
				source_bucket_branch_id,
				target_bucket_branch_id,
				50,
				true,
			)
			.await?;

			let _manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;

			wait_for_reclaim_delete(&test_ctx, database_branch_id, 49).await?;
			let pin_bytes = read_value(
				&test_ctx,
				db_pin_key(
					database_branch_id,
					&history_pin::bucket_fork_pin_id(target_bucket_branch_id),
				),
			)
			.await?
			.expect("bucket-derived DB_PIN should be materialized");
			let pin = decode_db_history_pin(&pin_bytes)?;
			assert_eq!(pin.kind, DbHistoryPinKind::BucketFork);
			assert_eq!(pin.owner_bucket_branch_id, Some(target_bucket_branch_id));
			assert_eq!(pin.at_txid, 50);
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 50, 0))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

#[tokio::test]
async fn reclaimer_retains_history_when_bucket_proof_is_ambiguous() -> Result<()> {
	let database_branch_id = database_branch_id(0xc0c0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let source_bucket_branch_id =
		BucketBranchId::from_uuid(Uuid::from_u128(0x2222_3333_4444_5555_6666_7777_8888_9999));
	let target_bucket_branch_id =
		BucketBranchId::from_uuid(Uuid::from_u128(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0001_0002));
	workflow_matrix!(
		"workflow-reclaimer-retains-history-when-bucket-proof-is-ambiguous",
		build_registry,
		|_tier, test_ctx| {
			let tag_value = database_branch_tag_value(database_branch_id);
			seed_manager_branch(
				&test_ctx,
				database_branch_id,
				100,
				Some(CompactionRoot {
					schema_version: 1,
					manifest_generation: 1,
					hot_watermark_txid: 100,
					cold_watermark_txid: 0,
					cold_watermark_versionstamp: [0; 16],
				}),
				None,
			)
			.await?;
			publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, 100).await?;
			seed_bucket_fork_proof(
				&test_ctx,
				database_branch_id,
				source_bucket_branch_id,
				target_bucket_branch_id,
				50,
				false,
			)
			.await?;

			let manager_workflow_id = test_ctx
				.workflow(DbManagerInput::new(database_branch_id, None))
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			let manager_state = wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
				state.planning_deadlines.next_reclaim_check_at_ms.is_some()
			})
			.await?;

			assert!(manager_state.active_jobs.reclaim.is_none());
			assert!(
				read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0))
					.await?
					.is_some()
			);
			assert!(
				read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
					.await?
					.is_some()
			);

			test_ctx.shutdown().await?;
			Ok(())
		}
	)
}

fn quota_threshold_head() -> u64 {
	depot::quota::COMPACTION_DELTA_THRESHOLD
}

fn cold_threshold_head() -> u64 {
	depot::HOT_BURST_COLD_LAG_THRESHOLD_TXIDS
}
