use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use depot::{
	types::DatabaseBranchId,
	workflows::compaction::{
		DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow, DbHotCompacterWorkflow, DbManagerInput,
		DbManagerState, DbManagerWorkflow, DbReclaimerWorkflow, DeltasAvailable,
		database_branch_tag_value,
	},
};
use gas::db::debug::DatabaseDebug;
use gas::prelude::{Id, Registry, SignalTrait, TestCtx, WorkflowTrait};
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
