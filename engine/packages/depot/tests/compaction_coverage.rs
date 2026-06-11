//! Coverage-invariant tests for workflow hot compaction: every coverage txid
//! published by an install must be fully reconstructable from SHARD rows alone.

#![cfg(feature = "test-faults")]

mod common;

use anyhow::{Context, Result, bail};
use depot::{
	conveyer::Db,
	keys::PAGE_SIZE,
	types::{DatabaseBranchId, DirtyPage, SnapshotSelector},
	workflows::compaction::{
		DATABASE_BRANCH_ID_TAG, DbHotCompacterWorkflow, DbManagerWorkflow, DbReclaimerWorkflow,
		DepotCompactionTestDriver, ForceCompactionWork,
	},
};
use gas::prelude::{Id, Registry, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0xc0de), 1)
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

fn make_db(test_ctx: &TestCtx, database_id: &str) -> Result<Db> {
	let udb_pool = test_ctx.pools().udb()?;
	Ok(Db::new(
		std::sync::Arc::new((*udb_pool).clone()),
		test_bucket(),
		database_id.to_string(),
		NodeId::new(),
	))
}

async fn read_branch_id(test_ctx: &TestCtx, database_id: &str) -> Result<DatabaseBranchId> {
	let udb = test_ctx.pools().udb()?;
	common::database_branch_id(&udb, test_bucket(), database_id).await
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

async fn shard_pages(
	test_ctx: &TestCtx,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Vec<(u32, Vec<u8>)>> {
	let udb = test_ctx.pools().udb()?;
	let blob = common::read_value(
		&udb,
		depot::keys::branch_shard_key(branch_id, shard_id, as_of_txid),
	)
	.await?
	.with_context(|| format!("shard {shard_id} as_of {as_of_txid} should exist"))?;
	let decoded = depot::ltx::decode_ltx_v3(&blob)?;

	Ok(decoded
		.pages
		.into_iter()
		.map(|page| (page.pgno, page.bytes))
		.collect())
}

/// A pin inside a hot batch must get full shard coverage at its own txid even
/// when the database shrinks later in the same batch. Folding intermediate
/// coverage txids with the batch-max commit's db_size drops pages that were
/// live at the pin.
#[tokio::test]
async fn hot_install_covers_pinned_txid_with_its_own_db_size() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_id = "coverage-pin-shrink";
	let db = make_db(&test_ctx, database_id)?;

	// Ancient wall clocks keep PITR interval selection out of the picture so the
	// pin is the only intermediate coverage txid.
	let far_pgno = depot::keys::SHARD_SIZE + 6;
	db.commit(vec![page(1, 0x11), page(far_pgno, 0x22)], far_pgno, 1_000)
		.await?;
	// Pin txid 1 while page `far_pgno` is still inside the database.
	let _restore_point = db.create_restore_point(SnapshotSelector::Latest).await?;
	// Shrink below `far_pgno`, then keep writing, all in the same hot batch.
	db.commit(vec![page(2, 0x33)], 40, 2_000).await?;
	db.commit(vec![page(3, 0x44)], 40, 3_000).await?;

	let branch_id = read_branch_id(&test_ctx, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = {
		let udb = test_ctx.pools().udb()?;
		common::history(&udb, branch_id).await?
	};
	assert_eq!(snapshot.hot_watermark_txid(), 3);
	assert_eq!(snapshot.pin_txids(), [1].into_iter().collect());

	// Shard 1 holds `far_pgno`. Coverage at the pinned txid must include it
	// because db_size at txid 1 was above it; coverage at the batch max must
	// not, because the database had shrunk to 40 pages by then.
	let far_shard = far_pgno / depot::keys::SHARD_SIZE;
	assert_eq!(
		snapshot.shard_versions.get(&far_shard),
		Some(&vec![1]),
		"pinned txid should publish a shard version for the pre-shrink page"
	);
	let pages = shard_pages(&test_ctx, branch_id, far_shard, 1).await?;
	assert_eq!(
		pages.iter().map(|(pgno, _)| *pgno).collect::<Vec<_>>(),
		vec![far_pgno],
		"shard coverage at the pinned txid should contain the pre-shrink page"
	);
	assert_eq!(pages[0].1, vec![0x22; PAGE_SIZE as usize]);

	// The head coverage txid folds with the post-shrink size: shard 0 only.
	let head_pages = shard_pages(&test_ctx, branch_id, 0, 3).await?;
	assert_eq!(
		head_pages.iter().map(|(pgno, _)| *pgno).collect::<Vec<_>>(),
		vec![1, 2, 3]
	);
	assert_eq!(
		snapshot.shard_versions.get(&0),
		Some(&vec![1, 3]),
		"shard 0 should publish versions at both coverage txids"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// A page truncated mid-batch and never rewritten must stay dead at coverage
/// txids after the database regrows past it. Folding deltas without truncate
/// awareness resurrects pre-truncate bytes into the regrown range.
#[tokio::test]
async fn shrink_then_regrow_in_batch_zero_fills_at_coverage() -> Result<()> {
	let mut test_ctx = TestCtx::new(build_registry()).await?;
	let database_id = "coverage-shrink-regrow";
	let db = make_db(&test_ctx, database_id)?;

	let far_pgno = depot::keys::SHARD_SIZE + 6;
	// Write the far page, shrink below it, then regrow past it without
	// rewriting it, all inside one hot batch.
	db.commit(vec![page(1, 0x11), page(far_pgno, 0x22)], far_pgno, 1_000)
		.await?;
	db.commit(vec![page(2, 0x33)], 40, 2_000).await?;
	db.commit(vec![page(3, 0x44)], far_pgno + 10, 3_000).await?;

	let branch_id = read_branch_id(&test_ctx, database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let pages = db.get_pages(vec![far_pgno]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"truncated never-rewritten page must read zeros after install, not resurrected bytes"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
