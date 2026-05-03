#![cfg(feature = "test-faults")]

use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	conveyer::{Db, branch},
	fault::{
		ColdCompactionFaultPoint, DepotFaultController, DepotFaultPoint, HotCompactionFaultPoint,
		ReclaimFaultPoint,
	},
	keys::{
		PAGE_SIZE, branch_compaction_cold_shard_key, branch_compaction_retired_cold_object_key,
		branch_shard_key,
	},
	types::{
		BucketId, DatabaseBranchId, DirtyPage, RetiredColdObjectDeleteState, SnapshotSelector,
		decode_retired_cold_object,
	},
	workflows::compaction::{
		CompactionJobKind, DbColdCompacterWorkflow, DbHotCompacterWorkflow, DbManagerWorkflow,
		DbReclaimerWorkflow, DepotCompactionTestDriver, ForceCompactionWork, test_hooks,
	},
};
use gas::prelude::{Id, Registry, TestCtx};
use rivet_pools::NodeId;
use rivet_test_deps::TestDeps;
use sha2::{Digest, Sha256};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

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

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x99aa), 1)
}

fn dirty_page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut output = [0_u8; 32];
	output.copy_from_slice(&digest);
	output
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

fn make_db_with_cold_tier(
	test_ctx: &TestCtx,
	database_id: impl Into<String>,
	cold_tier: Arc<dyn ColdTier>,
) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	Ok(Db::new_with_cold_tier(
		Arc::new((*udb_pool).clone()),
		test_bucket(),
		database_id.into(),
		NodeId::new(),
		cold_tier,
	))
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

async fn read_value(test_ctx: &TestCtx, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	test_ctx
		.pools()
		.udb()?
		.run(move |tx| {
			let key = key.clone();
			async move {
				Ok(tx
					.informal()
					.get(&key, Snapshot)
					.await?
					.map(|bytes| bytes.to_vec()))
			}
		})
		.await
}

async fn start_timer_disabled_manager(
	test_ctx: &TestCtx,
	database_branch_id: DatabaseBranchId,
) -> Result<Id> {
	DepotCompactionTestDriver::new(test_ctx)
		.start_manager(
			database_branch_id,
			Some("compaction-fault-test".to_string()),
			true,
		)
		.await
}

#[tokio::test]
async fn hot_stage_success_waits_for_delayed_install_fault() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_id = "compaction-fault-hot-delay";
	let db = make_db(&test_ctx, database_id)?;
	db.commit(vec![dirty_page(1, 0x21)], 2, 1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, database_id).await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::HotCompaction(
			HotCompactionFaultPoint::InstallBeforeStagedRead,
		))
		.database_branch_id(database_branch_id)
		.once()
		.delay(Duration::from_millis(25))?;
	let _guard =
		test_hooks::register_workflow_fault_controller(database_branch_id, controller.clone());
	let manager_workflow_id = start_timer_disabled_manager(&test_ctx, database_branch_id).await?;

	let started = tokio::time::Instant::now();
	let result = DepotCompactionTestDriver::new(&test_ctx)
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;

	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
	assert!(!result.completed_job_ids.is_empty());
	assert!(result.terminal_error.is_none());
	assert!(started.elapsed() >= Duration::from_millis(25));
	controller.assert_expected_fired()?;

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn hot_install_failure_after_shard_publish_leaves_shard_without_root() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_id = "compaction-fault-hot-publish-fail";
	let db = make_db(&test_ctx, database_id)?;
	db.commit(vec![dirty_page(1, 0x31)], 2, 1_001).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, database_id).await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::HotCompaction(
			HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear,
		))
		.database_branch_id(database_branch_id)
		.once()
		.fail("hot install failed after shard publish")?;
	let _guard =
		test_hooks::register_workflow_fault_controller(database_branch_id, controller.clone());
	let manager_workflow_id = start_timer_disabled_manager(&test_ctx, database_branch_id).await?;

	let result = DepotCompactionTestDriver::new(&test_ctx)
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;

	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
	assert!(!result.completed_job_ids.is_empty());
	assert!(
		result
			.terminal_error
			.as_deref()
			.is_some_and(|err| err.contains("hot install failed after shard publish"))
	);
	assert!(
		read_value(&test_ctx, branch_shard_key(database_branch_id, 0, 1))
			.await?
			.is_some()
	);
	controller.assert_expected_fired()?;

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn cold_upload_succeeds_before_publish_fault() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-compaction-fault-cold-publish-")
		.tempdir()?;
	let mut test_ctx = test_ctx_with_cold_tier(cold_root.path()).await?;
	let database_id = "compaction-fault-cold-publish";
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let db = make_db_with_cold_tier(&test_ctx, database_id, tier)?;
	db.commit(vec![dirty_page(1, 0x41)], 2, 1_001).await?;
	let restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, database_id).await?;
	let manager_workflow_id = start_timer_disabled_manager(&test_ctx, database_branch_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::ColdCompaction(
			ColdCompactionFaultPoint::PublishBeforeColdRefWrite,
		))
		.database_branch_id(database_branch_id)
		.once()
		.fail("cold publish failed after upload")?;
	let _guard =
		test_hooks::register_workflow_fault_controller(database_branch_id, controller.clone());

	let result = driver
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: true,
				cold: true,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;

	assert!(
		result
			.attempted_job_kinds
			.contains(&CompactionJobKind::Cold)
	);
	assert!(!result.completed_job_ids.is_empty());
	assert!(
		result
			.terminal_error
			.as_deref()
			.is_some_and(|err| err.contains("cold publish failed after upload"))
	);
	assert!(
		read_value(
			&test_ctx,
			branch_compaction_cold_shard_key(database_branch_id, 0, 1)
		)
		.await?
		.is_some()
	);
	controller.assert_expected_fired()?;
	db.delete_restore_point(restore_point).await?;

	test_ctx.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn forced_reclaim_deletes_retired_cold_object_with_short_grace() -> Result<()> {
	let cold_root = Builder::new()
		.prefix("depot-compaction-fault-reclaim-")
		.tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let mut test_ctx = test_ctx_with_cold_tier(cold_root.path()).await?;
	let database_id = "compaction-fault-reclaim";
	let db = make_db_with_cold_tier(&test_ctx, database_id, tier.clone())?;
	db.commit(vec![dirty_page(1, 0x51)], 2, 1_001).await?;
	let old_restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;
	let database_branch_id = read_database_branch_id(&test_ctx, database_id).await?;
	let manager_workflow_id = start_timer_disabled_manager(&test_ctx, database_branch_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let _grace_guard =
		test_hooks::override_cold_object_delete_grace_for_test(database_branch_id, 0);

	driver
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;
	driver
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: false,
				cold: true,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;
	let old_ref = read_value(
		&test_ctx,
		branch_compaction_cold_shard_key(database_branch_id, 0, 1),
	)
	.await?
	.as_deref()
	.map(depot::types::decode_cold_shard_ref)
	.transpose()?
	.context("old cold ref should exist")?;
	db.delete_restore_point(old_restore_point).await?;

	db.commit(vec![dirty_page(1, 0x52)], 2, 1_002).await?;
	let controller = DepotFaultController::new();
	controller
		.at(DepotFaultPoint::Reclaim(ReclaimFaultPoint::AfterColdDelete))
		.database_branch_id(database_branch_id)
		.once()
		.delay(Duration::from_millis(1))?;
	let _guard =
		test_hooks::register_workflow_fault_controller(database_branch_id, controller.clone());

	let result = driver
		.force_compaction(
			manager_workflow_id,
			database_branch_id,
			ForceCompactionWork {
				hot: true,
				cold: true,
				reclaim: true,
				final_settle: false,
			},
		)
		.await?;

	assert!(
		result
			.attempted_job_kinds
			.contains(&CompactionJobKind::Cold)
	);
	assert!(
		result
			.attempted_job_kinds
			.contains(&CompactionJobKind::Reclaim)
	);
	assert!(result.terminal_error.is_none());
	assert!(tier.get_object(&old_ref.object_key).await?.is_none());
	let retired = read_value(
		&test_ctx,
		branch_compaction_retired_cold_object_key(
			database_branch_id,
			content_hash(old_ref.object_key.as_bytes()),
		),
	)
	.await?
	.as_deref()
	.map(decode_retired_cold_object)
	.transpose()?
	.context("retired cold object should exist")?;
	assert_eq!(retired.object_key, old_ref.object_key);
	assert_eq!(retired.delete_state, RetiredColdObjectDeleteState::Deleted);
	controller.assert_expected_fired()?;

	test_ctx.shutdown().await?;
	Ok(())
}
