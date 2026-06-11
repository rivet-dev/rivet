//! Hot planning must not let one malformed row or one skewed clock stall
//! compaction: undecodable PIDX rows are skipped fail-closed, and commits with
//! future wall clocks still receive PITR interval coverage.

#![cfg(feature = "test-faults")]

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use common::compaction_harness;
use depot::{
	conveyer::Db, keys::PAGE_SIZE, types::DirtyPage,
	workflows::compaction::DepotCompactionTestDriver,
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x4071), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock before epoch")?
		.as_millis();
	i64::try_from(millis).context("timestamp overflow")
}

/// A PIDX row with an undecodable value must be skipped by planning instead of
/// poisoning the install activity into a permanent failure loop.
#[tokio::test]
async fn hot_compaction_skips_undecodable_pidx_rows() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "hot-corrupt-pidx";
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};
	let db = Db::new(
		udb.clone(),
		test_bucket(),
		database_id.to_string(),
		NodeId::new(),
	);

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;

	// Plant a malformed PIDX value directly.
	let corrupt_key = depot::keys::branch_pidx_key(branch_id, 7);
	{
		let corrupt_key = corrupt_key.clone();
		udb.txn("test_depothot_corrupt_pidx", move |tx| {
			let corrupt_key = corrupt_key.clone();
			async move {
				tx.informal().set(&corrupt_key, b"bad");
				Ok(())
			}
		})
		.await?;
	}

	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(
		snapshot.hot_watermark_txid(),
		1,
		"a corrupt PIDX row must not block hot compaction"
	);

	// The corrupt row is left in place for operators; it is not silently
	// deleted or reinterpreted.
	let raw = common::read_value(&udb, corrupt_key.clone()).await?;
	assert_eq!(raw.as_deref(), Some(b"bad".as_slice()));
	assert_eq!(snapshot.undecodable_pidx_keys, vec![corrupt_key]);

	test_ctx.shutdown().await?;
	Ok(())
}

/// Commits stamped slightly in the future by clock skew still get interval
/// coverage; dropping them makes those txids permanently unreachable for
/// timestamp forks once the batch is folded.
#[tokio::test]
async fn future_wall_clock_commits_receive_interval_coverage() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "hot-future-clock";
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};
	let db = Db::new(
		udb.clone(),
		test_bucket(),
		database_id.to_string(),
		NodeId::new(),
	);

	// Two minutes of skew, well within one default interval of now.
	db.commit(vec![page(1, 0x11)], 8, now_ms()? + 2 * 60 * 1000)
		.await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(snapshot.hot_watermark_txid(), 1);
	assert_eq!(
		snapshot.pitr_interval_txids(),
		[1].into_iter().collect(),
		"a future-stamped commit must still receive interval coverage"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
