//! Shrinking a database must not destroy shard history that pins and PITR
//! coverage still need, and regrowing past an old truncate must read zeros,
//! not stale pre-truncate bytes.

#![cfg(feature = "test-faults")]

mod common;

use anyhow::{Context, Result, bail};
use depot::{
	conveyer::{Db, branch},
	keys::PAGE_SIZE,
	types::{BucketId, DatabaseBranchId, DirtyPage, SnapshotSelector},
	workflows::compaction::{
		DbHotCompacterWorkflow, DbManagerWorkflow, DbReclaimerWorkflow, DepotCompactionTestDriver,
		ForceCompactionWork,
	},
};
use gas::prelude::{Id, Registry, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x720c), 1)
}

fn build_registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_workflow::<DbManagerWorkflow>().unwrap();
	registry
		.register_workflow::<DbHotCompacterWorkflow>()
		.unwrap();
	registry.register_workflow::<DbReclaimerWorkflow>().unwrap();
	registry
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn make_db(test_ctx: &TestCtx, bucket: Id, database_id: &str) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	Ok(Db::new(
		std::sync::Arc::new((*udb_pool).clone()),
		bucket,
		database_id.to_string(),
		NodeId::new(),
	))
}

async fn force_hot(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	branch_id: DatabaseBranchId,
) -> Result<()> {
	let driver = DepotCompactionTestDriver::new(test_ctx);
	let result = driver
		.force_compaction(
			manager_workflow_id,
			branch_id,
			ForceCompactionWork {
				hot: true,
				reclaim: false,
				final_settle: false,
			},
		)
		.await?;
	if let Some(error) = result.terminal_error {
		bail!("forced hot compaction failed: {error}");
	}

	Ok(())
}

/// Shrinking after compaction must keep the shard versions that a restore
/// point still resolves through, and a fork at that restore point must read
/// the pre-shrink page bytes instead of silent zeros.
#[tokio::test]
async fn truncate_preserves_shard_history_for_restore_points() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let bucket = test_bucket();
	let database_id = "truncate-history-source";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	let far_pgno = depot::keys::SHARD_SIZE + 6;
	let far_shard = far_pgno / depot::keys::SHARD_SIZE;
	db.commit(vec![page(1, 0x11), page(far_pgno, 0x22)], far_pgno, 1_000)
		.await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;

	// Shrink below the far page. This must not delete the shard version the
	// restore point reads through; it may add a pruned version at the new txid.
	db.commit(vec![page(2, 0x33)], 40, 2_000).await?;

	// Drain the compacted deltas so the fork read below must come from SHARD
	// coverage rather than the retained delta history.
	common::compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id)
		.await?;

	let snapshot = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(
		&snapshot,
		[2],
		"after reclaim, only the shrink commit delta",
	);
	let far_versions = snapshot
		.shard_versions
		.get(&far_shard)
		.context("pre-shrink shard version should survive the truncate")?;
	assert!(
		far_versions.contains(&1),
		"pre-shrink shard version should survive the truncate, got {far_versions:?}"
	);

	// Fork at the restore point and read the pre-shrink page through SHARD
	// fallback. Before the fix this read silently zero-filled.
	let forked_database_id = branch::fork_database(
		&udb,
		BucketId::from_gas_id(bucket),
		database_id.to_string(),
		SnapshotSelector::RestorePoint {
			restore_point: restore_point.clone(),
		},
		BucketId::from_gas_id(bucket),
	)
	.await?;
	let forked_db = make_db(&test_ctx, bucket, &forked_database_id)?;
	let pages = forked_db.get_pages(vec![far_pgno]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0x22; PAGE_SIZE as usize]),
		"restore point read of the pre-shrink page must return its bytes, not zeros"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// Regrowing past an old truncate boundary must read zeros for never-rewritten
/// pages. Guards the versioned-truncate design against serving stale
/// pre-truncate bytes from retained shard history.
#[tokio::test]
async fn regrow_after_truncate_reads_zeros_not_stale_bytes() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let bucket = test_bucket();
	let database_id = "truncate-history-regrow";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	let far_pgno = depot::keys::SHARD_SIZE + 6;
	db.commit(vec![page(1, 0x11), page(far_pgno, 0x22)], far_pgno, 1_000)
		.await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	// Shrink below the far page, then regrow past it without rewriting it.
	db.commit(vec![page(2, 0x33)], 40, 2_000).await?;
	db.commit(vec![page(3, 0x44)], far_pgno + 10, 3_000).await?;

	let pages = db.get_pages(vec![far_pgno]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"regrown never-rewritten page must read zeros, not stale pre-truncate bytes"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
