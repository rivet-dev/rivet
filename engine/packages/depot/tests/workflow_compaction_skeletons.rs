use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Result, bail};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	conveyer::{Db, branch, history_pin},
	keys::{
		PAGE_SIZE, branch_commit_key, branch_compaction_cold_shard_key,
		branch_compaction_retired_cold_object_key, branch_compaction_root_key,
		branch_compaction_stage_hot_shard_key, branch_compaction_stage_hot_shard_prefix,
		branch_delta_chunk_key, branch_meta_head_key, branch_pidx_key, branch_shard_key,
		branch_shard_prefix, branch_vtx_key, branches_list_key, db_pin_key, ns_child_key,
		ns_fork_pin_key, nscat_by_db_key, sqlite_cmp_dirty_key,
	},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::{
		BookmarkStr, BranchState, ColdShardRef, CommitRow, CompactionRoot, DBHead,
		DatabaseBranchId, DatabaseBranchRecord, DbHistoryPinKind, DirtyPage, FetchedPage,
		NamespaceBranchId, NamespaceCatalogDbFact, NamespaceForkFact, NamespaceId,
		RetiredColdObject, RetiredColdObjectDeleteState, SqliteCmpDirty, decode_cold_shard_ref,
		decode_commit_row, decode_compaction_root, decode_db_head, decode_db_history_pin,
		decode_retired_cold_object, encode_cold_shard_ref, encode_commit_row,
		encode_compaction_root, encode_database_branch_record, encode_db_head,
		encode_namespace_catalog_db_fact, encode_retired_cold_object,
		encode_namespace_fork_fact, encode_sqlite_cmp_dirty, ResolvedVersionstamp,
	},
	workflows::compaction::{
		CompactionJobKind, CompactionJobStatus, DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow,
		DbHotCompacterWorkflow, DbManagerInput, DbManagerState, DbManagerWorkflow,
		DbReclaimerWorkflow, DeltasAvailable, DestroyDatabaseBranch, ForceCompaction,
		ForceCompactionResult, ForceCompactionWork, HotJobFinished, HotJobInputRange,
		HotShardOutputRef, ColdJobFinished, RunColdJob, RunHotJob, ReclaimJobFinished,
		RunReclaimJob, TxidRange, database_branch_tag_value, set_workflow_test_cold_tier_for_test,
	},
};
use gas::db::debug::DatabaseDebug;
use gas::db::debug::WorkflowState;
use gas::prelude::{Id, Registry, SignalTrait, TestCtx, WorkflowTrait};
use sha2::{Digest, Sha256};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};
use uuid::Uuid;

const TEST_DATABASE: &str = "workflow-compaction-test";

lazy_static::lazy_static! {
	static ref WORKFLOW_COLD_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
}

struct WorkflowColdTierTestGuard;

impl Drop for WorkflowColdTierTestGuard {
	fn drop(&mut self) {
		set_workflow_test_cold_tier_for_test(None);
	}
}

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn test_namespace() -> Id {
	Id::v1(Uuid::from_u128(0x5678), 1)
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"depot-workflow-compaction-test".to_string(),
	)))
}

fn make_test_db(test_ctx: &TestCtx) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	Ok(Db::new(
		udb,
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	))
}

fn make_test_db_with_cold_tier(test_ctx: &TestCtx, cold_tier: Arc<dyn ColdTier>) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	Ok(Db::new_with_cold_tier(
		udb,
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
		cold_tier,
	))
}

fn page(fill: u8) -> Vec<u8> {
	vec![fill; PAGE_SIZE as usize]
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
	registry.register_workflow::<DbHotCompacterWorkflow>().unwrap();
	registry
		.register_workflow::<DbColdCompacterWorkflow>()
		.unwrap();
	registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
	registry
}

async fn wait_for_workflow<W: WorkflowTrait>(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<Id> {
	let started_at = Instant::now();
	let tag_value = database_branch_tag_value(database_branch_id);

	loop {
		if let Some(workflow_id) = test_ctx
			.find_workflow::<W>((DATABASE_BRANCH_ID_TAG, &tag_value))
			.await?
		{
			return Ok(workflow_id);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for workflow {}", W::NAME);
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_signal_ack(test_ctx: &TestCtx, signal_id: Id) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let signal = DatabaseDebug::get_signals(test_ctx.debug_db(), vec![signal_id])
			.await?
			.into_iter()
			.next();

		if let Some(signal) = signal {
			if signal.state == gas::db::debug::SignalState::Acked {
				return Ok(());
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for signal ack");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_run_hot_job(test_ctx: &TestCtx, hot_workflow_id: Id) -> Result<RunHotJob> {
	let started_at = Instant::now();

	loop {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(hot_workflow_id),
			Some(<RunHotJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(serde_json::from_value(signal.body)?);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for RunHotJob signal");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_run_cold_job(test_ctx: &TestCtx, cold_workflow_id: Id) -> Result<RunColdJob> {
	let started_at = Instant::now();

	loop {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(cold_workflow_id),
			Some(<RunColdJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(serde_json::from_value(signal.body)?);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for RunColdJob signal");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_run_reclaim_job(
	test_ctx: &TestCtx,
	reclaimer_workflow_id: Id,
) -> Result<RunReclaimJob> {
	let started_at = Instant::now();

	loop {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(reclaimer_workflow_id),
			Some(<RunReclaimJob as SignalTrait>::NAME),
			None,
		)
		.await?;
		if let Some(signal) = signals.into_iter().next() {
			return Ok(serde_json::from_value(signal.body)?);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for RunReclaimJob signal");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_reclaim_job_finished_signal(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let signals = DatabaseDebug::find_signals(
			test_ctx.debug_db(),
			&[],
			Some(manager_workflow_id),
			Some(<ReclaimJobFinished as SignalTrait>::NAME),
			None,
		)
		.await?;
		if !signals.is_empty() {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for ReclaimJobFinished signal");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_manager_cursor(
	test_ctx: &TestCtx,
	workflow_id: Id,
	observed_head_txid: u64,
) -> Result<DbManagerState> {
	let started_at = Instant::now();

	loop {
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
					return Ok(state);
				}
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for manager dirty cursor");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_manager_state(
	test_ctx: &TestCtx,
	workflow_id: Id,
	mut predicate: impl FnMut(&DbManagerState) -> bool,
) -> Result<DbManagerState> {
	let started_at = Instant::now();

	loop {
		let history = DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
			.await?
			.ok_or_else(|| anyhow::anyhow!("manager workflow history not found"))?;

		for event in history.events.into_iter().rev() {
			if let gas::db::debug::EventData::Loop(loop_event) = event.data {
				let state = serde_json::from_value::<DbManagerState>(loop_event.state)?;
				if predicate(&state) {
					return Ok(state);
				}
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for manager state");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn force_compaction_and_wait_idle(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	database_branch_id: DatabaseBranchId,
	request_id: Id,
	requested_work: ForceCompactionWork,
) -> Result<ForceCompactionResult> {
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
			.force_compaction_results
			.iter()
			.any(|result| result.request_id == request_id)
	})
	.await?;

	manager_state
		.force_compaction_results
		.into_iter()
		.find(|result| result.request_id == request_id)
		.ok_or_else(|| anyhow::anyhow!("force compaction result should be recorded"))
}

async fn wait_for_workflow_state(
	test_ctx: &TestCtx,
	workflow_id: Id,
	expected_state: WorkflowState,
) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let workflow = DatabaseDebug::get_workflows(test_ctx.debug_db(), vec![workflow_id])
			.await?
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("workflow not found"))?;
		if workflow.state == expected_state {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for workflow state {:?}", expected_state);
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_dirty_marker_cleared(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let dirty = read_value(test_ctx, sqlite_cmp_dirty_key(database_branch_id)).await?;
		if dirty.is_none() {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for dirty marker clear");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_staged_hot_rows(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let started_at = Instant::now();
	let prefix = branch_compaction_stage_hot_shard_prefix(database_branch_id, job_id);

	loop {
		let rows = read_prefix_values(test_ctx, prefix.clone()).await?;
		if !rows.is_empty() {
			return Ok(rows);
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for staged hot shard rows");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_hot_install(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	as_of_txid: u64,
) -> Result<CompactionRoot> {
	let started_at = Instant::now();

	loop {
		let root = read_value(test_ctx, branch_compaction_root_key(database_branch_id))
			.await?
			.as_deref()
			.map(decode_compaction_root)
			.transpose()?;
		let pidx = read_value(test_ctx, branch_pidx_key(database_branch_id, 1)).await?;
		let shard = read_value(test_ctx, branch_shard_key(database_branch_id, 0, as_of_txid)).await?;

		if let Some(root) = root {
			if root.manifest_generation == 1
				&& root.hot_watermark_txid == as_of_txid
				&& pidx.is_none()
				&& shard.is_some()
			{
				return Ok(root);
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for hot install");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_reclaim_delete(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let delta = read_value(test_ctx, branch_delta_chunk_key(database_branch_id, txid, 0)).await?;
		let commit = read_value(test_ctx, branch_commit_key(database_branch_id, txid)).await?;
		if delta.is_none() && commit.is_none() {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for reclaim delete");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_cold_publish(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	as_of_txid: u64,
) -> Result<depot::types::ColdShardRef> {
	let started_at = Instant::now();

	loop {
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
				return Ok(cold_ref.clone());
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			let cold_rows = read_prefix_values(
				test_ctx,
				depot::keys::branch_compaction_cold_shard_prefix(database_branch_id),
			)
			.await?;
			bail!(
				"timed out waiting for cold publish: root={root:?} cold_ref={cold_ref:?} cold_rows={}",
				cold_rows.len()
			);
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_retired_cold_object_state(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	object_key: &str,
	state: RetiredColdObjectDeleteState,
) -> Result<depot::types::RetiredColdObject> {
	let started_at = Instant::now();
	let retired_key =
		branch_compaction_retired_cold_object_key(database_branch_id, object_key_hash(object_key));

	loop {
		let retired = read_value(test_ctx, retired_key.clone())
			.await?
			.as_deref()
			.map(decode_retired_cold_object)
			.transpose()?;
		if let Some(retired) = retired {
			if retired.delete_state == state {
				return Ok(retired);
			}
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for retired cold object state {state:?}");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_cold_object_deleted(tier: &dyn ColdTier, object_key: &str) -> Result<()> {
	let started_at = Instant::now();

	loop {
		if tier.get_object(object_key).await?.is_none() {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for cold object delete");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
}

async fn wait_for_stage_row_cleared(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
) -> Result<()> {
	let started_at = Instant::now();

	loop {
		let rows = read_prefix_values(
			test_ctx,
			branch_compaction_stage_hot_shard_prefix(database_branch_id, job_id),
		)
		.await?;
		if rows.is_empty() {
			return Ok(());
		}

		if started_at.elapsed() > Duration::from_secs(5) {
			bail!("timed out waiting for staged hot shard cleanup");
		}

		tokio::time::sleep(Duration::from_millis(25)).await;
	}
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
	let db = test_ctx.pools().udb()?;
	db.run(|tx| async move {
		branch::resolve_database_branch(
			&tx,
			depot::types::NamespaceId::from_gas_id(test_namespace()),
			TEST_DATABASE,
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
		.ok_or_else(|| anyhow::anyhow!("database branch should exist"))
	})
	.await
}

async fn read_namespace_branch_id(test_ctx: &TestCtx) -> Result<NamespaceBranchId> {
	let db = test_ctx.pools().udb()?;
	db.run(|tx| async move {
		branch::resolve_namespace_branch(
			&tx,
			NamespaceId::from_gas_id(test_namespace()),
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
		.ok_or_else(|| anyhow::anyhow!("namespace branch should exist"))
	})
	.await
}

async fn read_prefix_values(test_ctx: &TestCtx, prefix: Vec<u8>) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| {
		let prefix = prefix.clone();
		async move {
			let prefix_subspace = universaldb::Subspace::from(
				universaldb::tuple::Subspace::from_bytes(prefix),
			);
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
	let namespace_branch = NamespaceBranchId::from_uuid(Uuid::from_u128(
		0x9999_8888_7777_6666_5555_4444_3333_2222,
	));
	db.run(move |tx| {
		let root = root.clone();
		let dirty = dirty.clone();
		async move {
			let branch_record = DatabaseBranchRecord {
				branch_id: database_branch_id,
				namespace_branch,
				parent: None,
				parent_versionstamp: None,
				root_versionstamp: [0; 16],
				fork_depth: 0,
				created_at_ms: 1_000,
				created_from_bookmark: None,
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
				tx.informal()
					.set(&branch_vtx_key(database_branch_id, versionstamp), &txid.to_be_bytes());
				let delta_blob = encode_ltx_v3(
					LtxHeader::delta(txid, 1, 1_000 + i64::try_from(txid).unwrap_or(i64::MAX)),
					&[DirtyPage {
						pgno: 1,
						bytes: vec![txid as u8; PAGE_SIZE as usize],
					}],
				)?;
				tx.informal()
					.set(&branch_delta_chunk_key(database_branch_id, txid, 0), &delta_blob);
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

async fn seed_bookmark_db_pin(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	at_txid: u64,
) -> Result<BookmarkStr> {
	let bookmark = BookmarkStr::format(1_000 + i64::try_from(at_txid).unwrap_or(i64::MAX), at_txid)?;
	let db = test_ctx.pools().udb()?;
	db.run({
		let bookmark = bookmark.clone();
		move |tx| {
			let bookmark = bookmark.clone();
			async move {
				let commit_bytes = tx
					.informal()
					.get(&branch_commit_key(database_branch_id, at_txid), Snapshot)
					.await?
					.expect("pinned commit row should exist");
				let commit = decode_commit_row(&commit_bytes)?;
				history_pin::write_bookmark_pin(
					&tx,
					database_branch_id,
					bookmark,
					commit.versionstamp,
					at_txid,
					commit.wall_clock_ms,
				)
			}
		}
	})
	.await?;

	Ok(bookmark)
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
		tx.informal()
			.set(&branch_shard_key(database_branch_id, 0, as_of_txid), &shard_blob);
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
				tx.informal()
					.set(&branch_shard_key(database_branch_id, shard_id, as_of_txid), &bytes);
				Ok(())
			}
		}
	})
	.await?;

	Ok(cold_ref)
}

async fn seed_namespace_fork_proof(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
	source_namespace_branch_id: NamespaceBranchId,
	target_namespace_branch_id: NamespaceBranchId,
	fork_txid: u64,
	write_fork_pin_fact: bool,
) -> Result<()> {
	let db = test_ctx.pools().udb()?;
	db.run(move |tx| async move {
		let mut fork_versionstamp = [0; 16];
		fork_versionstamp[8..16].copy_from_slice(&fork_txid.to_be_bytes());
		tx.informal().set(
			&nscat_by_db_key(database_branch_id, source_namespace_branch_id),
			&encode_namespace_catalog_db_fact(NamespaceCatalogDbFact {
				database_branch_id,
				namespace_branch_id: source_namespace_branch_id,
				catalog_versionstamp: [0; 16],
				tombstone_versionstamp: None,
			})?,
		);
		let fact = NamespaceForkFact {
			source_namespace_branch_id,
			target_namespace_branch_id,
			fork_versionstamp,
			parent_cap_versionstamp: fork_versionstamp,
		};
		let encoded_fact = encode_namespace_fork_fact(fact)?;
		tx.informal().set(
			&ns_child_key(
				source_namespace_branch_id,
				fork_versionstamp,
				target_namespace_branch_id,
			),
			&encoded_fact,
		);
		if write_fork_pin_fact {
			tx.informal().set(
				&ns_fork_pin_key(
					source_namespace_branch_id,
					fork_versionstamp,
					target_namespace_branch_id,
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
	assert_eq!(
		<DbReclaimerWorkflow as WorkflowTrait>::NAME,
		"db_reclaimer"
	);
}

#[tokio::test]
async fn manager_spawns_companions_and_records_deltas_available() -> Result<()> {
	let database_branch_id = database_branch_id(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	let manager_state = wait_for_manager_cursor(&test_ctx, manager_workflow_id, 123).await?;

	assert_eq!(
		manager_state.companion_workflow_ids.hot_compacter_workflow_id,
		Some(hot_workflow_id)
	);
	assert_eq!(
		manager_state
			.companion_workflow_ids
			.cold_compacter_workflow_id,
		Some(cold_workflow_id)
	);
	assert_eq!(
		manager_state.companion_workflow_ids.reclaimer_workflow_id,
		Some(reclaimer_workflow_id)
	);
	assert!(manager_state.active_hot_job.is_none());
	assert!(manager_state.active_cold_job.is_none());
	assert!(manager_state.active_reclaim_job.is_none());

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

#[tokio::test]
async fn manager_destroy_stops_idle_companions() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d10_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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

	wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete).await?;
	wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;
	wait_for_workflow_state(&test_ctx, cold_workflow_id, WorkflowState::Complete).await?;
	wait_for_workflow_state(&test_ctx, reclaimer_workflow_id, WorkflowState::Complete).await?;

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_recreated_for_deleted_branch_stops_without_scheduling() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d11_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	update_branch_lifecycle(&test_ctx, database_branch_id, BranchState::Deleted, 1).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let hot_workflow_id =
		wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;

	wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete).await?;
	wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;
	let run_hot_signals = DatabaseDebug::find_signals(
		test_ctx.debug_db(),
		&[],
		Some(hot_workflow_id),
		Some(<RunHotJob as SignalTrait>::NAME),
		None,
	)
	.await?;
	assert!(run_hot_signals.is_empty());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_destroy_during_active_hot_job_completes() -> Result<()> {
	let database_branch_id = database_branch_id(0x0d12_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let hot_workflow_id =
		wait_for_workflow::<DbHotCompacterWorkflow>(&test_ctx, database_branch_id).await?;
	let _run_hot_job = wait_for_run_hot_job(&test_ctx, hot_workflow_id).await?;

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

	wait_for_workflow_state(&test_ctx, manager_workflow_id, WorkflowState::Complete).await?;
	wait_for_workflow_state(&test_ctx, hot_workflow_id, WorkflowState::Complete).await?;

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_refresh_clears_idle_dirty_marker_without_planning_hot_job() -> Result<()> {
	let database_branch_id = database_branch_id(0x1010_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	wait_for_dirty_marker_cleared(&test_ctx, database_branch_id).await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.planning_deadlines.next_hot_check_at_ms.is_some()
				&& state.planning_deadlines.next_cold_check_at_ms.is_some()
				&& state.planning_deadlines.next_reclaim_check_at_ms.is_some()
				&& state.planning_deadlines.final_settle_check_at_ms.is_some()
		})
		.await?;

	assert!(manager_state.active_hot_job.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_refresh_plans_first_hot_job_from_fdb_state() -> Result<()> {
	let database_branch_id = database_branch_id(0x2020_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| state.active_hot_job.is_none())
			.await?;

	assert!(manager_state.active_hot_job.is_none());
	assert!(read_value(&test_ctx, branch_pidx_key(database_branch_id, 1)).await?.is_none());
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

#[tokio::test]
async fn duplicate_deltas_available_does_not_create_duplicate_hot_job() -> Result<()> {
	let database_branch_id = database_branch_id(0x3030_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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
	let shard_rows = read_prefix_values(&test_ctx, branch_shard_prefix(database_branch_id)).await?;

	assert!(second_state.active_hot_job.is_none());
	assert_eq!(root.manifest_generation, 1);
	assert_eq!(root.hot_watermark_txid, quota_threshold_head());
	assert_eq!(shard_rows.len(), 1);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn force_compaction_noop_records_completion_result() -> Result<()> {
	let database_branch_id = database_branch_id(0x3131_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(&test_ctx, database_branch_id, 0, None, None).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
			.contains(&"final-settle:refreshed".to_string())
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn force_hot_compaction_publishes_planned_work_below_threshold() -> Result<()> {
	let database_branch_id = database_branch_id(0x3232_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	seed_manager_branch(&test_ctx, database_branch_id, 1, None, None).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	assert!(read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1)).await?.is_some());
	assert!(read_value(&test_ctx, branch_pidx_key(database_branch_id, 1)).await?.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn force_reclaim_waits_for_reclaim_completion() -> Result<()> {
	let database_branch_id = database_branch_id(0x3333_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn e2e_force_hot_compaction_preserves_reads_after_pidx_clear() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db
		.commit(vec![dirty_page(1, 0x11), dirty_page(2, 0x22)], 3, 1_001)
		.await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	assert!(read_value(&test_ctx, branch_pidx_key(database_branch_id, 1)).await?.is_none());
	assert!(read_value(&test_ctx, branch_pidx_key(database_branch_id, 2)).await?.is_none());
	assert!(read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1)).await?.is_some());
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

#[tokio::test]
async fn e2e_force_reclaim_removes_hot_rows_and_keeps_reads() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db.commit(vec![dirty_page(1, 0x33)], 2, 1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let commit = read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
		.await?
		.as_deref()
		.map(decode_commit_row)
		.transpose()?
		.expect("commit should exist before reclaim");
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	assert!(read_value(&test_ctx, branch_commit_key(database_branch_id, 1)).await?.is_none());
	assert!(
		read_value(&test_ctx, branch_vtx_key(database_branch_id, commit.versionstamp))
			.await?
			.is_none()
	);
	assert!(read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1)).await?.is_some());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_force_compaction_preserves_exact_pinned_bookmark_txid() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db.commit(vec![dirty_page(1, 0x41)], 2, 1_001).await?;
	let bookmark = database_db.create_pinned_bookmark(1_001).await?;
	database_db.commit(vec![dirty_page(1, 0x42)], 2, 1_002).await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	assert_eq!(decode_ltx_v3(&pinned_shard)?.get_page(1), Some(page(0x41).as_slice()));
	assert_eq!(decode_ltx_v3(&latest_shard)?.get_page(1), Some(page(0x42).as_slice()));
	assert!(read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0)).await?.is_some());
	assert!(read_value(&test_ctx, branch_commit_key(database_branch_id, 1)).await?.is_some());
	let pin_bytes = read_value(
		&test_ctx,
		db_pin_key(database_branch_id, &history_pin::bookmark_pin_id(&bookmark)),
	)
	.await?
	.expect("bookmark DB_PIN should exist");
	let pin = decode_db_history_pin(&pin_bytes)?;
	assert_eq!(pin.kind, DbHistoryPinKind::Bookmark);
	assert_eq!(pin.at_txid, 1);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_force_reclaim_materializes_namespace_fork_pin() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db(&test_ctx)?;
	database_db.commit(vec![dirty_page(1, 0x51)], 2, 1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let source_namespace_branch_id = read_namespace_branch_id(&test_ctx).await?;
	let fork_commit = read_value(&test_ctx, branch_commit_key(database_branch_id, 1))
		.await?
		.as_deref()
		.map(decode_commit_row)
		.transpose()?
		.expect("fork-point commit should exist");
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	let forked_namespace = branch::fork_namespace(
		udb.as_ref(),
		&test_ups(),
		NamespaceId::from_gas_id(test_namespace()),
		ResolvedVersionstamp {
			versionstamp: fork_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;
	database_db.commit(vec![dirty_page(1, 0x52)], 2, 1_002).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	let forked_namespace_branch_id = udb
		.run(move |tx| async move {
			branch::resolve_namespace_branch(
				&tx,
				forked_namespace,
				universaldb::utils::IsolationLevel::Serializable,
			)
			.await?
			.ok_or_else(|| anyhow::anyhow!("forked namespace branch should exist"))
		})
		.await?;
	assert!(
		read_value(
			&test_ctx,
			ns_fork_pin_key(
				source_namespace_branch_id,
				fork_commit.versionstamp,
				forked_namespace_branch_id,
			),
		)
		.await?
		.is_some()
	);
	let pin_bytes = read_value(
		&test_ctx,
		db_pin_key(
			database_branch_id,
			&history_pin::namespace_fork_pin_id(forked_namespace_branch_id),
		),
	)
	.await?
	.expect("namespace-derived DB_PIN should be materialized");
	let pin = decode_db_history_pin(&pin_bytes)?;
	assert_eq!(pin.kind, DbHistoryPinKind::NamespaceFork);
	assert_eq!(pin.at_txid, 1);
	assert!(read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 1, 0)).await?.is_some());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn e2e_force_cold_publish_reads_after_hot_rows_removed() -> Result<()> {
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-force-cold-e2e-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_db = make_test_db_with_cold_tier(&test_ctx, tier.clone())?;
	database_db.commit(vec![dirty_page(1, 0x61)], 2, 1_001).await?;
	let _bookmark = database_db.create_pinned_bookmark(1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx).await?;
	let tag_value = database_branch_tag_value(database_branch_id);
	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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
	assert!(read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1)).await?.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn hot_compacter_writes_idempotent_staged_shard_output() -> Result<()> {
	let database_branch_id = database_branch_id(0x4040_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn hot_compacter_rejects_stale_base_generation_without_staging() -> Result<()> {
	let database_branch_id = database_branch_id(0x5050_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn hot_compacter_rejects_stale_lifecycle_generation_without_staging() -> Result<()> {
	let database_branch_id = database_branch_id(0x5051_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn manager_schedules_cleanup_for_stale_hot_output() -> Result<()> {
	let database_branch_id = database_branch_id(0x5052_2233_4455_6677_8899_aabb_ccdd_eeff);
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
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
	assert_eq!(repair_job.input_range.staged_hot_shards.len(), 1);

	wait_for_stage_row_cleared(&test_ctx, database_branch_id, stale_job_id).await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.active_reclaim_job.is_none()
		})
		.await?;
	assert!(manager_state.active_reclaim_job.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_schedules_cleanup_for_stale_cold_output() -> Result<()> {
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-orphan-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let database_branch_id = database_branch_id(0x5053_2233_4455_6677_8899_aabb_ccdd_eeff);
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let reclaimer_workflow_id =
		wait_for_workflow::<DbReclaimerWorkflow>(&test_ctx, database_branch_id).await?;
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
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.active_reclaim_job.is_none()
		})
		.await?;
	assert!(manager_state.active_reclaim_job.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_publishes_hot_output_and_reads_through_shard_after_pidx_clear() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	let database_db = Db::new(
		udb,
		test_ups(),
		test_namespace(),
		TEST_DATABASE.to_string(),
		NodeId::new(),
	);

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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| state.active_hot_job.is_none())
			.await?;

	assert!(manager_state.active_hot_job.is_none());
	assert_eq!(
		database_db.get_pages(vec![1]).await?,
		vec![FetchedPage {
			pgno: 1,
			bytes: Some(page(u8::try_from(quota_threshold_head()).unwrap_or(u8::MAX))),
		}]
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn manager_publishes_cold_output_and_reads_through_cold_ref() -> Result<()> {
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let udb_pool = test_ctx.pools().udb()?;
	let udb = Arc::new((*udb_pool).clone());
	let database_db = Db::new_with_cold_tier(
		udb,
		test_ups(),
		test_namespace(),
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let cold_workflow_id =
		wait_for_workflow::<DbColdCompacterWorkflow>(&test_ctx, database_branch_id).await?;

	let run_cold_job = wait_for_run_cold_job(&test_ctx, cold_workflow_id).await?;
	let cold_ref = wait_for_cold_publish(&test_ctx, database_branch_id, cold_threshold_head()).await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| state.active_cold_job.is_none())
			.await?;

	assert!(manager_state.active_cold_job.is_none());
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
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-retire-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let database_branch_id = database_branch_id(0xd1d1_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

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
	let retired_root = read_value(&test_ctx, branch_compaction_root_key(database_branch_id))
		.await?
		.as_deref()
		.map(decode_compaction_root)
		.transpose()?
		.expect("retired compaction root should exist");
	assert_eq!(retired_root.manifest_generation, 3);

	wait_for_cold_object_deleted(tier.as_ref(), &old_ref.object_key).await?;
	let deleted = wait_for_retired_cold_object_state(
		&test_ctx,
		database_branch_id,
		&old_ref.object_key,
		RetiredColdObjectDeleteState::Deleted,
	)
	.await?;
	assert_eq!(deleted.object_key, old_ref.object_key);
	assert!(tier.get_object(&current_key).await?.is_some());
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.active_reclaim_job.is_none()
		})
		.await?;
	assert!(manager_state.active_reclaim_job.is_none());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_logs_and_retains_live_cold_ref_when_s3_object_is_missing() -> Result<()> {
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-missing-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let database_branch_id = database_branch_id(0xd1d2_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-delete-issued-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let database_branch_id = database_branch_id(0xd1d3_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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
	let _cold_test_lock = WORKFLOW_COLD_TEST_LOCK.lock().await;
	let cold_root = Builder::new()
		.prefix("depot-workflow-cold-stale-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	set_workflow_test_cold_tier_for_test(Some(tier.clone()));
	let _cold_tier_guard = WorkflowColdTierTestGuard;

	let database_branch_id = database_branch_id(0xd0d0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	let _bookmark = seed_bookmark_db_pin(&test_ctx, database_branch_id, 50).await?;

	let _manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn reclaimer_deletes_obsolete_fdb_rows_after_hot_coverage() -> Result<()> {
	let database_branch_id = database_branch_id(0x7070_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	wait_for_hot_install(&test_ctx, database_branch_id, quota_threshold_head()).await?;
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

#[tokio::test]
async fn reclaimer_retains_rows_when_pidx_still_references_deleted_txid() -> Result<()> {
	let database_branch_id = database_branch_id(0x8080_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	publish_test_shard_and_clear_pidx(&test_ctx, database_branch_id, quota_threshold_head()).await?;
	set_test_pidx(&test_ctx, database_branch_id, 1).await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.planning_deadlines.next_reclaim_check_at_ms.is_some()
		})
		.await?;

	assert!(manager_state.active_reclaim_job.is_none());
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

#[tokio::test]
async fn reclaimer_rejects_stale_manifest_generation() -> Result<()> {
	let database_branch_id = database_branch_id(0x9090_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn reclaimer_retains_pinned_txid_history() -> Result<()> {
	let database_branch_id = database_branch_id(0xa0a0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	let _bookmark = seed_bookmark_db_pin(&test_ctx, database_branch_id, 50).await?;

	let _manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
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

#[tokio::test]
async fn reclaimer_materializes_namespace_fork_pin_before_delete() -> Result<()> {
	let database_branch_id = database_branch_id(0xb0b0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let source_namespace_branch_id = NamespaceBranchId::from_uuid(Uuid::from_u128(
		0x1111_2222_3333_4444_5555_6666_7777_8888,
	));
	let target_namespace_branch_id = NamespaceBranchId::from_uuid(Uuid::from_u128(
		0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0001,
	));
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	seed_namespace_fork_proof(
		&test_ctx,
		database_branch_id,
		source_namespace_branch_id,
		target_namespace_branch_id,
		50,
		true,
	)
	.await?;

	let _manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;

	wait_for_reclaim_delete(&test_ctx, database_branch_id, 49).await?;
	let pin_bytes = read_value(
		&test_ctx,
		db_pin_key(
			database_branch_id,
			&history_pin::namespace_fork_pin_id(target_namespace_branch_id),
		),
	)
	.await?
	.expect("namespace-derived DB_PIN should be materialized");
	let pin = decode_db_history_pin(&pin_bytes)?;
	assert_eq!(pin.kind, DbHistoryPinKind::NamespaceFork);
	assert_eq!(pin.owner_namespace_branch_id, Some(target_namespace_branch_id));
	assert_eq!(pin.at_txid, 50);
	assert!(
		read_value(&test_ctx, branch_delta_chunk_key(database_branch_id, 50, 0))
			.await?
			.is_some()
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn reclaimer_retains_history_when_namespace_proof_is_ambiguous() -> Result<()> {
	let database_branch_id = database_branch_id(0xc0c0_2233_4455_6677_8899_aabb_ccdd_eeff);
	let tag_value = database_branch_tag_value(database_branch_id);
	let source_namespace_branch_id = NamespaceBranchId::from_uuid(Uuid::from_u128(
		0x2222_3333_4444_5555_6666_7777_8888_9999,
	));
	let target_namespace_branch_id = NamespaceBranchId::from_uuid(Uuid::from_u128(
		0xaaaa_bbbb_cccc_dddd_eeee_ffff_0001_0002,
	));
	let mut test_ctx = TestCtx::new(build_registry()).await?;
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
	seed_namespace_fork_proof(
		&test_ctx,
		database_branch_id,
		source_namespace_branch_id,
		target_namespace_branch_id,
		50,
		false,
	)
	.await?;

	let manager_workflow_id = test_ctx
		.workflow(DbManagerInput { database_branch_id })
		.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
		.unique()
		.dispatch()
		.await?;
	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| {
			state.planning_deadlines.next_reclaim_check_at_ms.is_some()
		})
		.await?;

	assert!(manager_state.active_reclaim_job.is_none());
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

fn quota_threshold_head() -> u64 {
	depot::quota::COMPACTION_DELTA_THRESHOLD
}

fn cold_threshold_head() -> u64 {
	depot::HOT_BURST_COLD_LAG_THRESHOLD_TXIDS
}
