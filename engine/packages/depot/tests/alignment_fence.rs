//! Snapshot-target alignment fence: pins and forks may only land on txids that
//! are above the hot watermark or already covered (the watermark, a retained
//! PITR interval representative, or an existing pin). Anything else would
//! become unreadable once reclaim deletes the compacted deltas.

#![cfg(feature = "test-faults")]

mod common;

use anyhow::{Context, Result};
use common::compaction_harness;
use depot::{
	conveyer::{Db, branch, restore_point::test_hooks as restore_point_test_hooks},
	error::SqliteStorageError,
	keys::PAGE_SIZE,
	types::{BucketId, DirtyPage, ResolvedVersionstamp, SnapshotSelector},
	workflows::compaction::DepotCompactionTestDriver,
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0xfe2ce), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn make_db(test_ctx: &TestCtx, database_id: &str) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	Ok(Db::new(
		std::sync::Arc::new((*udb_pool).clone()),
		test_bucket(),
		database_id.to_string(),
		NodeId::new(),
	))
}

fn assert_fork_out_of_retention(err: &anyhow::Error) {
	match err.downcast_ref::<SqliteStorageError>() {
		Some(SqliteStorageError::ForkOutOfRetention) => {}
		other => panic!("expected ForkOutOfRetention, got {other:?}: {err:#}"),
	}
}

/// Forking at an uncovered historical versionstamp must fail instead of
/// creating a branch whose reads break once reclaim deletes the deltas.
/// Forking at a covered txid keeps working after reclaim drains everything.
#[tokio::test]
async fn fork_requires_covered_or_unfolded_target() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "fence-fork-target";
	let db = make_db(&test_ctx, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	db.commit(vec![page(2, 0x22)], 8, 2_000).await?;
	db.commit(vec![page(3, 0x33)], 8, 3_000).await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(snapshot.hot_watermark_txid(), 3);
	let uncovered_versionstamp = snapshot
		.commits
		.get(&1)
		.context("commit 1 should still exist before reclaim")?
		.versionstamp;
	let covered_versionstamp = snapshot
		.commits
		.get(&3)
		.context("commit 3 should exist")?
		.versionstamp;

	// Txid 1 is below the watermark with no pin or interval row covering it.
	let err = branch::fork_database(
		&udb,
		BucketId::from_gas_id(test_bucket()),
		database_id.to_string(),
		ResolvedVersionstamp {
			versionstamp: uncovered_versionstamp,
			restore_point: None,
		},
		BucketId::from_gas_id(test_bucket()),
	)
	.await
	.expect_err("fork at an uncovered folded txid must be rejected");
	assert_fork_out_of_retention(&err);

	// The watermark itself is always covered.
	let forked_database_id = branch::fork_database(
		&udb,
		BucketId::from_gas_id(test_bucket()),
		database_id.to_string(),
		ResolvedVersionstamp {
			versionstamp: covered_versionstamp,
			restore_point: None,
		},
		BucketId::from_gas_id(test_bucket()),
	)
	.await?;

	// After reclaim drains the deltas, the fork still reads its frozen state.
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");
	common::assert_commit_txids(&drained, [3], "after reclaim");
	common::assert_vtx_txids(&drained, [3], "after reclaim");

	let forked_db = make_db(&test_ctx, &forked_database_id)?;
	let pages = forked_db.get_pages(vec![1, 2, 3]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; PAGE_SIZE as usize]));
	assert_eq!(pages[2].bytes, Some(vec![0x33; PAGE_SIZE as usize]));

	test_ctx.shutdown().await?;
	Ok(())
}

/// Interval-covered txids strictly below the watermark are valid fork targets:
/// the fence's coverage scan, not just its watermark comparison, must pass
/// them.
#[tokio::test]
async fn fork_at_interval_covered_txid_below_watermark_succeeds() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "fence-fork-covered";
	let db = make_db(&test_ctx, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	let interval_ms = depot::types::DEFAULT_PITR_INTERVAL_MS;
	let bucket_start = {
		use std::time::{SystemTime, UNIX_EPOCH};
		let now = i64::try_from(
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.context("system clock before epoch")?
				.as_millis(),
		)
		.context("timestamp overflow")?;
		(now - 2 * interval_ms).div_euclid(interval_ms) * interval_ms
	};
	db.commit(vec![page(1, 0x11)], 8, bucket_start + 1_000)
		.await?;
	db.commit(vec![page(2, 0x22)], 8, bucket_start + 2_000)
		.await?;
	db.commit(vec![page(3, 0x33)], 8, bucket_start + interval_ms + 1_000)
		.await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(snapshot.hot_watermark_txid(), 3);
	assert!(snapshot.pitr_interval_txids().contains(&2));
	let covered_versionstamp = snapshot
		.commits
		.get(&2)
		.context("interval island commit should survive")?
		.versionstamp;

	let forked_database_id = branch::fork_database(
		&udb,
		BucketId::from_gas_id(test_bucket()),
		database_id.to_string(),
		ResolvedVersionstamp {
			versionstamp: covered_versionstamp,
			restore_point: None,
		},
		BucketId::from_gas_id(test_bucket()),
	)
	.await?;
	let forked_db = make_db(&test_ctx, &forked_database_id)?;
	let pages = forked_db.get_pages(vec![1, 2, 3]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; PAGE_SIZE as usize]));
	assert_eq!(
		pages[2].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"page 3 was committed after the covered fork point"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// A restore point resolved at the head must not be pinned at a stale txid
/// after a concurrent install advances the watermark past it. The fence makes
/// creation fail so the caller re-resolves, instead of silently creating a pin
/// whose reads die with the deltas.
#[tokio::test]
async fn restore_point_creation_rejects_target_folded_during_race() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "fence-restore-race";
	let db = make_db(&test_ctx, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	db.commit(vec![page(2, 0x22)], 8, 2_000).await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;

	// Pause restore point creation between resolution (head txid 2) and the
	// pin write, then land another commit and install through txid 3.
	let (guard, reached, release) = restore_point_test_hooks::pause_after_resolve(database_id);
	let create_task = tokio::spawn({
		let db = make_db(&test_ctx, database_id)?;
		async move { db.create_restore_point(SnapshotSelector::Latest).await }
	});
	reached.notified().await;

	db.commit(vec![page(3, 0x33)], 8, 3_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 3);

	release.notify_waiters();
	drop(guard);
	let create_result = create_task.await?;
	let err = create_result
		.expect_err("restore point creation must fail when its resolved txid was folded uncovered");
	match err.downcast_ref::<SqliteStorageError>() {
		Some(SqliteStorageError::RestoreTargetExpired) => {}
		other => panic!("expected RestoreTargetExpired, got {other:?}: {err:#}"),
	}

	// No pin may exist at the stale txid.
	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(
		snapshot.pin_txids(),
		std::collections::BTreeSet::new(),
		"the raced restore point must not leave a pin behind"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
