use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use depot::{
	keys::{
		branch_commit_key, branch_compaction_root_key, branch_delta_chunk_key, branch_meta_head_key,
		branch_pidx_key, branches_list_key, sqlite_cmp_dirty_key,
	},
	types::{
		BranchState, CommitRow, CompactionRoot, DBHead, DatabaseBranchId, DatabaseBranchRecord,
		NamespaceBranchId, SqliteCmpDirty, encode_commit_row, encode_compaction_root,
		encode_database_branch_record, encode_db_head, encode_sqlite_cmp_dirty,
	},
	workflows::compaction::{
		DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow, DbHotCompacterWorkflow, DbManagerInput,
		DbManagerState, DbManagerWorkflow, DbReclaimerWorkflow, DeltasAvailable,
		PlannedInputRange, database_branch_tag_value,
	},
};
use gas::db::debug::DatabaseDebug;
use gas::prelude::{Id, Registry, SignalTrait, TestCtx, WorkflowTrait};
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
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
				tx.informal()
					.set(&branch_delta_chunk_key(database_branch_id, txid, 0), &[txid as u8]);
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

	let manager_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| state.active_hot_job.is_some())
			.await?;
	let active_hot_job = manager_state
		.active_hot_job
		.expect("manager should record active hot job");

	assert_eq!(active_hot_job.database_branch_id, database_branch_id);
	assert_eq!(active_hot_job.base_manifest_generation, 0);
	assert_ne!(active_hot_job.input_fingerprint, [0; 32]);
	match active_hot_job.input_range {
		PlannedInputRange::Hot(range) => {
			assert_eq!(range.txids.min_txid, 1);
			assert_eq!(range.txids.max_txid, quota_threshold_head());
			assert!(range.max_bytes > 0);
		}
		PlannedInputRange::Cold(_) | PlannedInputRange::Reclaim(_) => {
			bail!("expected hot input range")
		}
	}

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
	let first_state =
		wait_for_manager_state(&test_ctx, manager_workflow_id, |state| state.active_hot_job.is_some())
			.await?;
	let first_job_id = first_state
		.active_hot_job
		.as_ref()
		.expect("manager should have active hot job")
		.job_id;

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
	let second_job_id = second_state
		.active_hot_job
		.as_ref()
		.expect("manager should keep active hot job")
		.job_id;

	assert_eq!(second_job_id, first_job_id);

	test_ctx.shutdown().await?;
	Ok(())
}

fn quota_threshold_head() -> u64 {
	depot::quota::COMPACTION_DELTA_THRESHOLD
}
