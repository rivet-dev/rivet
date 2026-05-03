#![cfg(feature = "test-faults")]

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use depot::{
	conveyer::{Db, branch},
	keys::PAGE_SIZE,
	types::{BucketId, DatabaseBranchId, DirtyPage},
	workflows::compaction::{
		CompactionJobKind, DATABASE_BRANCH_ID_TAG, DbColdCompacterWorkflow, DbHotCompacterWorkflow,
		DbManagerState, DbManagerWorkflow, DbReclaimerWorkflow, DepotCompactionTestDriver,
		ForceCompactionWork,
	},
};
use gas::{
	db::debug::{DatabaseDebug, EventData},
	prelude::{Id, Registry, TestCtx},
};
use rivet_pools::NodeId;
use uuid::Uuid;

const TEST_DATABASE: &str = "forced-compaction-test-driver";

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x9abc), 1)
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

fn dirty_page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn make_db(test_ctx: &TestCtx, database_id: impl Into<String>) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	Ok(Db::new(
		Arc::new((*udb_pool).clone()),
		test_bucket(),
		database_id.into(),
		NodeId::new(),
	))
}

async fn read_database_branch_id(
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
				BucketId::from_gas_id(test_bucket()),
				&database_id,
				universaldb::utils::IsolationLevel::Serializable,
			)
			.await?
			.context("database branch should exist")
		}
	})
	.await
}

async fn latest_manager_state(test_ctx: &TestCtx, workflow_id: Id) -> Result<DbManagerState> {
	let history = DatabaseDebug::get_workflow_history(test_ctx.debug_db(), workflow_id, true)
		.await?
		.context("manager workflow history not found")?;

	for event in history.events.into_iter().rev() {
		if let EventData::Loop(loop_event) = event.data {
			return Ok(serde_json::from_value::<DbManagerState>(loop_event.state)?);
		}
	}

	bail!("manager workflow has no loop state")
}

#[tokio::test]
async fn test_driver_forces_noop_without_planning_timers() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let db = make_db(&test_ctx, TEST_DATABASE)?;
	db.commit(vec![dirty_page(1, 0x11)], 2, 1_000).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, TEST_DATABASE).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(database_branch_id, None, true).await?;

	let requested_work = ForceCompactionWork {
		hot: false,
		cold: true,
		reclaim: false,
		final_settle: true,
	};
	let result = driver
		.force_compaction(manager_workflow_id, database_branch_id, requested_work)
		.await?;
	let manager_state = latest_manager_state(&test_ctx, manager_workflow_id).await?;

	assert_eq!(result.requested_work, requested_work);
	assert!(result.attempted_job_kinds.is_empty());
	assert!(result.completed_job_ids.is_empty());
	assert!(result.terminal_error.is_none());
	assert!(
		result
			.skipped_noop_reasons
			.contains(&"cold:no-actionable-lag".to_string())
	);
	assert!(
		result
			.skipped_noop_reasons
			.contains(&"final-settle:refreshed".to_string())
	);
	assert_eq!(manager_state.planning_deadlines, Default::default());

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn test_driver_forces_hot_compaction_and_exposes_result_fields() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_id = "forced-compaction-test-driver-hot";
	let db = make_db(&test_ctx, database_id)?;
	db.commit(vec![dirty_page(1, 0x22)], 2, 1_000).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver
		.start_manager(database_branch_id, Some("actor-for-test".to_string()), true)
		.await?;

	let requested_work = ForceCompactionWork {
		hot: true,
		cold: false,
		reclaim: false,
		final_settle: false,
	};
	let result = driver
		.force_compaction(manager_workflow_id, database_branch_id, requested_work)
		.await?;
	let manager_state = latest_manager_state(&test_ctx, manager_workflow_id).await?;

	assert_eq!(result.requested_work, requested_work);
	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
	assert_eq!(result.completed_job_ids.len(), 1);
	assert!(result.skipped_noop_reasons.is_empty());
	assert!(result.terminal_error.is_none());
	assert_eq!(manager_state.planning_deadlines, Default::default());

	let tag_value = depot::workflows::compaction::database_branch_tag_value(database_branch_id);
	assert_eq!(
		test_ctx
			.find_workflow::<DbManagerWorkflow>((DATABASE_BRANCH_ID_TAG, &tag_value))
			.await?,
		Some(manager_workflow_id)
	);

	test_ctx.shutdown().await?;
	Ok(())
}
