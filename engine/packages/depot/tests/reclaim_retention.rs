//! Reclaim retention tests measuring the exact stored history state.
//!
//! The retention contract: DELTA rows survive iff their txid is above the hot
//! watermark; COMMITS/VTX rows survive iff their txid is at or above the
//! watermark or referenced by a live retention fact (retained PITR interval
//! row or DB pin); PIDX never references txids at or below the watermark.

#![cfg(feature = "test-faults")]

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use common::compaction_harness;
use depot::{
	conveyer::Db,
	keys::{PAGE_SIZE, SHARD_SIZE},
	types::{DatabaseBranchId, DirtyPage, SnapshotSelector},
	workflows::compaction::DepotCompactionTestDriver,
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x4ec1), 1)
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

async fn setup(
	test_ctx: &TestCtx,
	database_id: &str,
) -> Result<(std::sync::Arc<universaldb::Database>, DatabaseBranchId, Id)> {
	let udb_pool = test_ctx.pools().udb()?;
	let udb = std::sync::Arc::new((*udb_pool).clone());
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;

	Ok((udb, branch_id, manager_workflow_id))
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock before epoch")?
		.as_millis();
	i64::try_from(millis).context("timestamp overflow")
}

/// Deltas folded across several install batches must be reclaimed even when
/// each batch touched a different shard. The old reclaim proof required every
/// referenced shard to have a version at exactly the current watermark, which
/// starved forever once any shard stopped being written.
#[tokio::test]
async fn reclaim_deletes_compacted_deltas_across_shards() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-multi-shard";
	let db = make_db(&test_ctx, database_id)?;
	let far_pgno = SHARD_SIZE + 3;

	// Ancient wall clocks keep PITR interval rows out of retention entirely.
	db.commit(vec![page(1, 0x11)], far_pgno, 1_000).await?;
	db.commit(vec![page(2, 0x12)], far_pgno, 2_000).await?;
	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	// The second batch touches only the second shard, so the first shard's
	// newest version stays at the old watermark.
	db.commit(vec![page(far_pgno, 0x21)], far_pgno, 3_000)
		.await?;
	db.commit(vec![page(far_pgno, 0x22)], far_pgno, 4_000)
		.await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let before = common::history(&udb, branch_id).await?;
	assert_eq!(before.hot_watermark_txid(), 4);
	common::assert_delta_txids(&before, [1, 2, 3, 4], "before reclaim");

	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;

	let after = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&after, [], "after reclaim");
	common::assert_commit_txids(&after, [4], "after reclaim");
	common::assert_vtx_txids(&after, [4], "after reclaim");
	common::assert_pidx(&after, [], "after reclaim");

	// Reads now come exclusively from SHARD coverage.
	let pages = db.get_pages(vec![1, 2, far_pgno]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0x12; PAGE_SIZE as usize]));
	assert_eq!(pages[2].bytes, Some(vec![0x22; PAGE_SIZE as usize]));

	test_ctx.shutdown().await?;
	Ok(())
}

/// Hot planning truncates its PIDX scan at the batch budget, so installs can
/// leave PIDX rows pointing at already-compacted txids. Reclaim must drain
/// those rows instead of treating them as permanent blockers.
#[tokio::test]
async fn reclaim_drains_stale_pidx_leftovers_from_budget_truncation() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-pidx-leftovers";
	let db = make_db(&test_ctx, database_id)?;

	// Five commits of 120 distinct pages each: 600 PIDX rows exceed the 500-key
	// batch budget, so the install's PIDX scan truncates and leaves rows
	// referencing folded txids behind.
	let pages_per_commit = 120u32;
	let total_pages = pages_per_commit * 5;
	for commit_idx in 0u32..5 {
		let fill = 0x10 + commit_idx as u8;
		let dirty = (0..pages_per_commit)
			.map(|offset| page(commit_idx * pages_per_commit + offset + 1, fill))
			.collect::<Vec<_>>();
		db.commit(dirty, total_pages, 1_000 * (commit_idx as i64 + 1))
			.await?;
	}
	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;

	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 5);
	let stale_rows = installed
		.pidx
		.values()
		.filter(|txid| **txid <= installed.hot_watermark_txid())
		.count();
	assert!(
		stale_rows > 0,
		"budget truncation should leave PIDX rows at folded txids"
	);

	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;

	let after = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&after, [], "after reclaim");
	common::assert_commit_txids(&after, [5], "after reclaim");
	common::assert_vtx_txids(&after, [5], "after reclaim");
	common::assert_pidx(&after, [], "after reclaim");

	// Spot-check shard fallback reads across the whole database.
	let pages = db.get_pages(vec![1, 121, 241, 361, 481]).await?;
	for (idx, fetched) in pages.iter().enumerate() {
		assert_eq!(
			fetched.bytes,
			Some(vec![0x10 + idx as u8; PAGE_SIZE as usize]),
			"page {} should read its committed bytes",
			fetched.pgno
		);
	}

	test_ctx.shutdown().await?;
	Ok(())
}

/// COMMITS/VTX rows at pinned txids and retained PITR interval representatives
/// survive reclaim as islands; everything else below the watermark is deleted,
/// and deleting the pin frees its island on the next pass.
#[tokio::test]
async fn reclaim_keeps_commit_islands_for_pins_and_intervals() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-keep-set";
	let db = make_db(&test_ctx, database_id)?;

	// Recent wall clocks inside the PITR retention window, aligned so commits
	// 1-2 land in one five-minute interval bucket and 3-4 in the next.
	let interval_ms = depot::types::DEFAULT_PITR_INTERVAL_MS;
	let bucket_start = (now_ms()? - 2 * interval_ms).div_euclid(interval_ms) * interval_ms;
	db.commit(vec![page(1, 0x11)], 8, bucket_start + 1_000)
		.await?;
	let restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;
	db.commit(vec![page(2, 0x12)], 8, bucket_start + 2_000)
		.await?;
	db.commit(vec![page(3, 0x13)], 8, bucket_start + interval_ms + 1_000)
		.await?;
	db.commit(vec![page(4, 0x14)], 8, bucket_start + interval_ms + 2_000)
		.await?;

	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 4);
	assert_eq!(installed.pin_txids(), [1].into_iter().collect());
	assert_eq!(
		installed.pitr_interval_txids(),
		[2, 4].into_iter().collect(),
		"each interval bucket should retain its latest commit"
	);

	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let after = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&after, [], "after first reclaim");
	common::assert_commit_txids(&after, [1, 2, 4], "after first reclaim");
	common::assert_vtx_txids(&after, [1, 2, 4], "after first reclaim");
	common::assert_pidx(&after, [], "after first reclaim");

	// Deleting the restore point frees its island; the interval islands stay.
	db.delete_restore_point(restore_point).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_commit_txids(&drained, [2, 4], "after pin delete");
	common::assert_vtx_txids(&drained, [2, 4], "after pin delete");

	test_ctx.shutdown().await?;
	Ok(())
}

/// The retention invariants hold after every write/install/reclaim cycle, not
/// just at the end of one pass.
#[tokio::test]
async fn reclaim_invariants_hold_across_repeated_cycles() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-soak";
	let db = make_db(&test_ctx, database_id)?;

	db.commit(vec![page(1, 1)], 16, 1_000).await?;
	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;
	let mut expected_fill = std::collections::BTreeMap::<u32, u8>::from([(1, 1)]);

	let mut head_txid = 1u64;
	for round in 1u64..=5 {
		for offset in 0u32..3 {
			head_txid += 1;
			let pgno = (head_txid % 13) as u32 + 1;
			let fill = (round * 10 + offset as u64) as u8;
			db.commit(vec![page(pgno, fill)], 16, 1_000 * head_txid as i64)
				.await?;
			expected_fill.insert(pgno, fill);
		}
		compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
		compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id)
			.await?;

		let snapshot = common::history(&udb, branch_id).await?;
		let watermark = snapshot.hot_watermark_txid();
		assert_eq!(watermark, head_txid, "[round {round}] watermark at head");
		common::assert_delta_txids(&snapshot, [], &format!("round {round}"));
		common::assert_commit_txids(&snapshot, [watermark], &format!("round {round}"));
		common::assert_vtx_txids(&snapshot, [watermark], &format!("round {round}"));
		common::assert_pidx(&snapshot, [], &format!("round {round}"));
		if snapshot.staged_rows != 0 {
			bail!("[round {round}] staged compaction rows leaked");
		}

		// Every page must read its latest committed bytes from shard coverage
		// alone; the row-set invariants above cannot catch a wrong fold.
		let pgnos = expected_fill.keys().copied().collect::<Vec<_>>();
		let pages = db.get_pages(pgnos.clone()).await?;
		for (pgno, fetched) in pgnos.iter().zip(&pages) {
			assert_eq!(
				fetched.bytes,
				Some(vec![expected_fill[pgno]; PAGE_SIZE as usize]),
				"[round {round}] page {pgno} should read its latest committed bytes"
			);
		}
	}

	test_ctx.shutdown().await?;
	Ok(())
}

/// Keep-set islands are not just retained rows: forks and restores must still
/// resolve and read through them after the surrounding history is gone.
#[tokio::test]
async fn reclaim_islands_remain_resolvable() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-island-reads";
	let db = make_db(&test_ctx, database_id)?;

	let interval_ms = depot::types::DEFAULT_PITR_INTERVAL_MS;
	let bucket_start = (now_ms()? - 2 * interval_ms).div_euclid(interval_ms) * interval_ms;
	db.commit(vec![page(1, 0x11)], 8, bucket_start + 1_000)
		.await?;
	let restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;
	db.commit(vec![page(2, 0x12)], 8, bucket_start + 2_000)
		.await?;
	db.commit(vec![page(3, 0x13)], 8, bucket_start + interval_ms + 1_000)
		.await?;

	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");

	// Fork from the restore point pinned at txid 1.
	let bucket = depot::types::BucketId::from_gas_id(test_bucket());
	let rp_fork = depot::conveyer::branch::fork_database(
		&udb,
		bucket,
		database_id.to_string(),
		SnapshotSelector::RestorePoint {
			restore_point: restore_point.clone(),
		},
		bucket,
	)
	.await?;
	let rp_db = make_db(&test_ctx, &rp_fork)?;
	let pages = rp_db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(
		pages[1].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"page 2 did not exist at the pinned txid"
	);

	// Fork from a timestamp resolving to the retained interval representative.
	let ts_fork = depot::conveyer::branch::fork_database(
		&udb,
		bucket,
		database_id.to_string(),
		SnapshotSelector::AtTimestamp {
			timestamp_ms: bucket_start + interval_ms - 1,
		},
		bucket,
	)
	.await?;
	let ts_db = make_db(&test_ctx, &ts_fork)?;
	let pages = ts_db.get_pages(vec![1, 2, 3]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0x12; PAGE_SIZE as usize]));
	assert_eq!(
		pages[2].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"page 3 was committed after the interval representative"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// Superseded shard versions are deleted once nothing can read through them:
/// reclaim keeps the newest version at or below each covered txid (pins,
/// retained interval representatives, the watermark) plus everything above the
/// watermark, and deletes the rest.
#[tokio::test]
async fn reclaim_deletes_superseded_shard_versions() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-shard-versions";
	let db = make_db(&test_ctx, database_id)?;

	// Four rounds of write + install on the same shard, with a restore point
	// pinned at txid 2. Ancient wall clocks keep PITR rows out of retention.
	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let (udb, branch_id, manager_workflow_id) = setup(&test_ctx, database_id).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	db.commit(vec![page(1, 0x22)], 8, 2_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	let restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;

	db.commit(vec![page(1, 0x33)], 8, 3_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	db.commit(vec![page(1, 0x44)], 8, 4_000).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let installed = common::history(&udb, branch_id).await?;
	assert_eq!(installed.hot_watermark_txid(), 4);
	common::assert_shard_versions(&installed, [(0, vec![1, 2, 3, 4])], "before reclaim");

	// The pin at txid 2 and the watermark at txid 4 each keep their newest
	// at-or-below version; versions 1 and 3 are unreadable and deleted.
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_shard_versions(&drained, [(0, vec![2, 4])], "after reclaim");

	// Reads through both keepers stay correct.
	let pages = db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x44; PAGE_SIZE as usize]));
	let bucket = depot::types::BucketId::from_gas_id(test_bucket());
	let rp_fork = depot::conveyer::branch::fork_database(
		&udb,
		bucket,
		database_id.to_string(),
		SnapshotSelector::RestorePoint {
			restore_point: restore_point.clone(),
		},
		bucket,
	)
	.await?;
	let rp_db = make_db(&test_ctx, &rp_fork)?;
	let pages = rp_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x22; PAGE_SIZE as usize]));

	// Deleting the restore point frees its keeper on the next pass. The fork
	// pin written by the fork above keeps txid 2 alive, so drop expectations
	// account for it: delete the restore point only after verifying the fork
	// read, then expect the fork pin to keep version 2.
	db.delete_restore_point(restore_point).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let final_state = common::history(&udb, branch_id).await?;
	common::assert_shard_versions(
		&final_state,
		[(0, vec![2, 4])],
		"the fork pin still keeps txid 2 after the restore point is deleted",
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// With planning timers enabled, one reclaim pass re-arms the next immediately
/// on success, so a backlog wider than one batch drains without further force
/// signals. Backlog: 1200 PIDX rows, far more than one 500-key pass clears.
#[tokio::test]
async fn reclaim_rearm_drains_backlog_without_repeated_force() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "reclaim-rearm-drain";
	let db = make_db(&test_ctx, database_id)?;

	let pages_per_commit = 120u32;
	let total_pages = pages_per_commit * 10;
	for commit_idx in 0u32..10 {
		let dirty = (0..pages_per_commit)
			.map(|offset| page(commit_idx * pages_per_commit + offset + 1, 0x10))
			.collect::<Vec<_>>();
		db.commit(dirty, total_pages, 1_000 * (commit_idx as i64 + 1))
			.await?;
	}

	let udb_pool = test_ctx.pools().udb()?;
	let udb = std::sync::Arc::new((*udb_pool).clone());
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	// Planning timers stay enabled so the re-arm path is the only driver of the
	// second and later reclaim passes.
	let manager_workflow_id = driver.start_manager(branch_id, None, false).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	compaction_harness::force_reclaim(&test_ctx, manager_workflow_id, branch_id).await?;

	// The backlog cannot fit one pass; only the success re-arm can finish it.
	let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
	loop {
		let snapshot = common::history(&udb, branch_id).await?;
		if snapshot.delta_txids().is_empty()
			&& snapshot.pidx.is_empty()
			&& snapshot.commit_txids() == [10].into_iter().collect()
		{
			break;
		}
		if tokio::time::Instant::now() > deadline {
			bail!(
				"re-arm did not drain the backlog: deltas={:?} pidx_rows={} commits={:?}",
				snapshot.delta_txids(),
				snapshot.pidx.len(),
				snapshot.commit_txids()
			);
		}
		// The drain is driven by workflow signals with no test-visible waiter;
		// polling the stored state is the only observation point.
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}

	test_ctx.shutdown().await?;
	Ok(())
}
