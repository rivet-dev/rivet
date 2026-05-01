use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Result, bail};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use depot::{
	conveyer::{Db, branch, history_pin},
	keys::{
		PAGE_SIZE, branch_commit_key, branch_compaction_root_key,
		branch_compaction_stage_hot_shard_prefix, branch_delta_chunk_key, branch_meta_head_key,
		branch_pidx_key, branch_shard_key, branch_shard_prefix, branches_list_key,
		sqlite_cmp_dirty_key,
	},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::{
		BookmarkStr, BranchState, CommitRow, CompactionRoot, DBHead, DatabaseBranchId, DatabaseBranchRecord,
		DirtyPage, FetchedPage, NamespaceBranchId, SqliteCmpDirty, decode_commit_row,
		decode_compaction_root, encode_commit_row, encode_compaction_root,
		encode_database_branch_record, encode_db_head, encode_sqlite_cmp_dirty,
	},
	workflows::compaction::{
		CompactionJobKind, CompactionJobStatus, DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow,
		DbHotCompacterWorkflow, DbManagerInput, DbManagerState, DbManagerWorkflow,
		DbReclaimerWorkflow, DeltasAvailable, HotJobInputRange, RunHotJob,
		TxidRange,
		database_branch_tag_value,
	},
};
use gas::db::debug::DatabaseDebug;
use gas::prelude::{Id, Registry, SignalTrait, TestCtx, WorkflowTrait};
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};
use uuid::Uuid;

const TEST_DATABASE: &str = "workflow-compaction-test";

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
	assert!(
		read_value(
			&test_ctx,
			branch_delta_chunk_key(database_branch_id, quota_threshold_head(), 0),
		)
		.await?
		.is_some()
	);
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
	assert!(
		read_value(
			&test_ctx,
			branch_delta_chunk_key(database_branch_id, quota_threshold_head(), 0),
		)
		.await?
		.is_some()
	);
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

fn quota_threshold_head() -> u64 {
	depot::quota::COMPACTION_DELTA_THRESHOLD
}
