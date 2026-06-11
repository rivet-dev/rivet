//! Bucket-fork pin materialization must land on covered txids: exact when the
//! fork point has not been folded yet, snapped down to the newest covered
//! point otherwise, and fail-safe (retain commits, never block delta reclaim)
//! when the target predates all retained coverage.

#![cfg(feature = "test-faults")]

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use common::compaction_harness;
use depot::{
	conveyer::{Db, branch},
	keys::PAGE_SIZE,
	types::{BucketId, DbHistoryPinKind, DirtyPage, ResolvedVersionstamp},
	workflows::compaction::{DepotCompactionTestDriver, ForceCompactionWork},
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket(seed: u128) -> Id {
	Id::v1(Uuid::from_u128(seed), 1)
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

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock before epoch")?
		.as_millis();
	i64::try_from(millis).context("timestamp overflow")
}

async fn materialize_pins(
	test_ctx: &TestCtx,
	manager_workflow_id: Id,
	branch_id: depot::types::DatabaseBranchId,
) -> Result<()> {
	// Any forced refresh resolves bucket fork facts into pins.
	compaction_harness::force(
		test_ctx,
		manager_workflow_id,
		branch_id,
		ForceCompactionWork {
			hot: false,
			reclaim: false,
			final_settle: true,
		},
	)
	.await?;

	Ok(())
}

fn bucket_fork_pin_txids(snapshot: &common::BranchHistorySnapshot) -> Vec<u64> {
	snapshot
		.pins
		.iter()
		.filter(|pin| pin.kind == DbHistoryPinKind::BucketFork)
		.map(|pin| pin.at_txid)
		.collect()
}

/// A historical bucket fork (target already folded) snaps to the newest
/// covered txid at or before the fork point instead of pinning an arbitrary
/// commit whose reads die with the deltas.
#[tokio::test]
async fn bucket_fork_pin_snaps_to_newest_covered_point() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xb0f1);
	let database_id = "bucket-fork-snap";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	// Commits 1-2 in one PITR interval bucket, 3-4 in the next.
	let interval_ms = depot::types::DEFAULT_PITR_INTERVAL_MS;
	let bucket_start = (now_ms()? - 2 * interval_ms).div_euclid(interval_ms) * interval_ms;
	db.commit(vec![page(1, 0x11)], 8, bucket_start + 1_000)
		.await?;
	db.commit(vec![page(2, 0x12)], 8, bucket_start + 2_000)
		.await?;
	db.commit(vec![page(3, 0x13)], 8, bucket_start + interval_ms + 1_000)
		.await?;
	db.commit(vec![page(4, 0x14)], 8, bucket_start + interval_ms + 2_000)
		.await?;

	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 4);
	assert_eq!(
		installed.pitr_interval_txids(),
		[2, 4].into_iter().collect()
	);
	let fork_versionstamp = installed
		.commits
		.get(&3)
		.context("commit 3 should exist before reclaim")?
		.versionstamp;

	branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;
	materialize_pins(&test_ctx, manager_workflow_id, branch_id).await?;

	// Txid 3 is folded and uncovered; the newest covered point at or before the
	// fork versionstamp is the first interval representative, txid 2.
	let pinned = common::history(&udb, branch_id).await?;
	assert_eq!(
		bucket_fork_pin_txids(&pinned),
		vec![2],
		"bucket fork pin must snap to the newest covered txid at or before the fork point"
	);

	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");
	common::assert_commit_txids(&drained, [2, 4], "after reclaim");

	test_ctx.shutdown().await?;
	Ok(())
}

/// A fork point above the watermark pins the exact commit, and the pin feeds
/// shard coverage at the next install.
#[tokio::test]
async fn bucket_fork_pin_keeps_exact_unfolded_target() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xb0f2);
	let database_id = "bucket-fork-exact";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	db.commit(vec![page(2, 0x12)], 8, 2_000).await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	// Commit 3 is above the watermark when the bucket forks at it.
	db.commit(vec![page(3, 0x13)], 8, 3_000).await?;
	let head = common::history(&udb, branch_id).await?;
	assert_eq!(head.hot_watermark_txid(), 2);
	let fork_versionstamp = head
		.commits
		.get(&3)
		.context("commit 3 should exist")?
		.versionstamp;

	branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;
	materialize_pins(&test_ctx, manager_workflow_id, branch_id).await?;

	let pinned = common::history(&udb, branch_id).await?;
	assert_eq!(bucket_fork_pin_txids(&pinned), vec![3]);

	// Land one more commit so the pin sits strictly inside the next batch:
	// shard coverage at txid 3 can then only come from pin-driven staging, not
	// from 3 being the batch max.
	db.commit(vec![page(4, 0x14)], 8, 4_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 4);
	assert!(
		installed
			.shard_versions
			.get(&0)
			.is_some_and(|versions| versions.contains(&3) && versions.contains(&4)),
		"pinned txid should get shard coverage at install, got {:?}",
		installed.shard_versions
	);

	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");
	common::assert_commit_txids(&drained, [3, 4], "after reclaim");

	test_ctx.shutdown().await?;
	Ok(())
}

/// A fork point older than all retained coverage cannot be materialized. The
/// fail-safe retains commits/VTX but never blocks delta reclaim.
#[tokio::test]
async fn bucket_fork_pin_predating_coverage_fails_safe() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xb0f3);
	let database_id = "bucket-fork-expired";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	// Ancient wall clocks: no PITR interval rows survive selection, so the only
	// covered txid is the watermark, whose versionstamp is above the fork point.
	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	db.commit(vec![page(2, 0x12)], 8, 2_000).await?;
	db.commit(vec![page(3, 0x13)], 8, 3_000).await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 3);
	let fork_versionstamp = installed
		.commits
		.get(&1)
		.context("commit 1 should exist before reclaim")?
		.versionstamp;

	branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;
	materialize_pins(&test_ctx, manager_workflow_id, branch_id).await?;

	let pinned = common::history(&udb, branch_id).await?;
	assert_eq!(
		bucket_fork_pin_txids(&pinned),
		Vec::<u64>::new(),
		"an unresolvable fork target must not materialize an uncovered pin"
	);

	// Delta reclaim proceeds; commit/VTX deletes are fail-safe suppressed.
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");
	common::assert_commit_txids(
		&drained,
		[1, 2, 3],
		"after reclaim with blocked commit deletes",
	);
	common::assert_vtx_txids(
		&drained,
		[1, 2, 3],
		"after reclaim with blocked commit deletes",
	);

	// The unresolvable fork must not freeze hot compaction: a later install
	// proceeds and advances the watermark.
	db.commit(vec![page(4, 0x14)], 8, 4_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	let advanced = common::history(&udb, branch_id).await?;
	assert_eq!(
		advanced.hot_watermark_txid(),
		4,
		"an out-of-retention bucket fork must not block installs"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
