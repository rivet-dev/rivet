//! Accessing a database through a forked bucket must materialize a capped
//! database fork: reads are frozen at the fork point instead of tracking the
//! live source, and the first write builds on the inherited state instead of
//! shadowing it with an empty branch.

#![cfg(feature = "test-faults")]

mod common;

use anyhow::{Context, Result};
use common::compaction_harness;
use depot::{
	conveyer::{Db, branch},
	keys::PAGE_SIZE,
	types::{BucketId, DirtyPage, ResolvedVersionstamp},
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

fn gas_bucket_id(bucket: depot::types::BucketId) -> Id {
	Id::v1(bucket.as_uuid(), 1)
}

/// Reads through a forked bucket are frozen at the fork point even when the
/// source database keeps committing.
#[tokio::test]
async fn bucket_fork_reads_are_frozen_at_the_fork_point() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xacc1);
	let database_id = "bucket-fork-read-freeze";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let fork_versionstamp = common::history(&udb, branch_id)
		.await?
		.commits
		.get(&1)
		.context("commit 1 should exist")?
		.versionstamp;

	let forked_bucket = branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;

	// The source keeps writing after the fork.
	db.commit(vec![page(1, 0x22)], 8, 2_000).await?;

	let forked_db = make_db(&test_ctx, gas_bucket_id(forked_bucket), database_id)?;
	let pages = forked_db.get_pages(vec![1]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0x11; PAGE_SIZE as usize]),
		"fork reads must see the fork-point state, not live source writes"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// The first write through a forked bucket builds on the inherited state, and
/// fork writes never leak back into the source.
#[tokio::test]
async fn bucket_fork_first_write_preserves_inherited_state() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xacc2);
	let database_id = "bucket-fork-write-cow";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let fork_versionstamp = common::history(&udb, branch_id)
		.await?
		.commits
		.get(&1)
		.context("commit 1 should exist")?
		.versionstamp;

	let forked_bucket = branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;

	let forked_db = make_db(&test_ctx, gas_bucket_id(forked_bucket), database_id)?;
	forked_db.commit(vec![page(2, 0x33)], 8, 2_000).await?;

	let pages = forked_db.get_pages(vec![1, 2]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0x11; PAGE_SIZE as usize]),
		"the first fork write must not shadow the inherited state"
	);
	assert_eq!(pages[1].bytes, Some(vec![0x33; PAGE_SIZE as usize]));

	// Fork writes never leak into the source.
	let pages = db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(
		pages[1].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"fork writes must not leak into the source database"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

/// A database created in the source bucket after the fork is invisible through
/// the fork: reads see no database, and a write under that name creates a
/// fresh database instead of materializing the post-fork source state.
#[tokio::test]
async fn database_created_after_fork_is_invisible_through_the_fork() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xacc3);
	let alpha = "fork-vis-alpha";
	let beta = "fork-vis-beta";
	let alpha_db = make_db(&test_ctx, bucket, alpha)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	alpha_db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let alpha_branch = common::database_branch_id(&udb, bucket, alpha).await?;
	let fork_versionstamp = common::history(&udb, alpha_branch)
		.await?
		.commits
		.get(&1)
		.context("commit 1 should exist")?
		.versionstamp;
	let forked_bucket = branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;

	// Beta is born in the source bucket after the fork point.
	let beta_db = make_db(&test_ctx, bucket, beta)?;
	beta_db.commit(vec![page(1, 0x99)], 8, 2_000).await?;

	let forked_beta = make_db(&test_ctx, gas_bucket_id(forked_bucket), beta)?;
	let err = forked_beta
		.get_pages(vec![1])
		.await
		.expect_err("a database created after the fork must not be readable through it");
	match err.downcast_ref::<depot::error::SqliteStorageError>() {
		Some(depot::error::SqliteStorageError::DatabaseNotFound) => {}
		other => panic!("expected DatabaseNotFound, got {other:?}: {err:#}"),
	}

	// Writing under the name creates a fresh database in the fork.
	forked_beta.commit(vec![page(2, 0x33)], 8, 3_000).await?;
	let pages = forked_beta.get_pages(vec![1, 2]).await?;
	assert_eq!(
		pages[0].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"the fork's beta must not inherit the post-fork source beta"
	);
	assert_eq!(pages[1].bytes, Some(vec![0x33; PAGE_SIZE as usize]));

	// The source beta is untouched.
	let pages = beta_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x99; PAGE_SIZE as usize]));

	test_ctx.shutdown().await?;
	Ok(())
}

/// Rolling back a database through a forked bucket operates on the fork's
/// materialized branch; the source branch stays live and readable.
#[tokio::test]
async fn rollback_through_fork_never_freezes_the_source() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xacc4);
	let database_id = "fork-rollback-cow";
	let db = make_db(&test_ctx, bucket, database_id)?;
	let udb = {
		let udb_pool = test_ctx.pools().udb()?;
		std::sync::Arc::new((*udb_pool).clone())
	};

	db.commit(vec![page(1, 0x11)], 8, 1_000).await?;
	let source_branch = common::database_branch_id(&udb, bucket, database_id).await?;
	let fork_versionstamp = common::history(&udb, source_branch)
		.await?
		.commits
		.get(&1)
		.context("commit 1 should exist")?
		.versionstamp;
	let forked_bucket = branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;

	// Write through the fork so it has local history, then roll the fork back
	// at its own commit.
	let forked_db = make_db(&test_ctx, gas_bucket_id(forked_bucket), database_id)?;
	forked_db.commit(vec![page(2, 0x33)], 8, 2_000).await?;
	let fork_branch =
		common::database_branch_id(&udb, gas_bucket_id(forked_bucket), database_id).await?;
	let fork_commit_versionstamp = common::history(&udb, fork_branch)
		.await?
		.commits
		.values()
		.next()
		.context("fork branch should have a local commit")?
		.versionstamp;
	branch::rollback_database(
		&udb,
		forked_bucket,
		database_id.to_string(),
		ResolvedVersionstamp {
			versionstamp: fork_commit_versionstamp,
			restore_point: None,
		},
	)
	.await?;

	// The source branch is untouched: still live, still readable.
	let record_bytes = common::read_value(&udb, depot::keys::branches_list_key(source_branch))
		.await?
		.context("source branch record should exist")?;
	let record = depot::types::decode_database_branch_record(&record_bytes)?;
	assert_eq!(
		record.state,
		depot::types::BranchState::Live,
		"rollback through a fork must not freeze the source branch"
	);
	let pages = db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0u8; PAGE_SIZE as usize]));

	test_ctx.shutdown().await?;
	Ok(())
}

/// Materializing through a fork whose target is already folded snaps to the
/// newest covered point, and stays readable after reclaim drains the deltas.
#[tokio::test]
async fn bucket_fork_access_snaps_to_covered_point_after_reclaim() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let bucket = test_bucket(0xacc5);
	let database_id = "fork-covered-snap";
	let db = make_db(&test_ctx, bucket, database_id)?;
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

	let branch_id = common::database_branch_id(&udb, bucket, database_id).await?;
	let fork_versionstamp = common::history(&udb, branch_id)
		.await?
		.commits
		.get(&2)
		.context("commit 2 should exist")?
		.versionstamp;

	let driver = depot::workflows::compaction::DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;
	let drained = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&drained, [], "after reclaim");
	assert!(drained.pitr_interval_txids().contains(&2));

	// Txid 2 is below the watermark; the fork snaps to its interval island.
	let forked_bucket = branch::fork_bucket(
		&udb,
		BucketId::from_gas_id(bucket),
		ResolvedVersionstamp {
			versionstamp: fork_versionstamp,
			restore_point: None,
		},
	)
	.await?;
	let forked_db = make_db(&test_ctx, gas_bucket_id(forked_bucket), database_id)?;
	let pages = forked_db.get_pages(vec![1, 2, 3]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; PAGE_SIZE as usize]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; PAGE_SIZE as usize]));
	assert_eq!(
		pages[2].bytes,
		Some(vec![0u8; PAGE_SIZE as usize]),
		"page 3 was committed after the fork point"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
