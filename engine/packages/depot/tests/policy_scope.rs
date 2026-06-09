//! Hot compaction must honor the effective PITR policy in planning as well as
//! in staging and install. Planning with a default policy while validation
//! uses the effective one livelocks every database with a non-default policy:
//! the coverage selection never matches, so every hot job is rejected and
//! deltas accumulate forever.

#![cfg(feature = "test-faults")]

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use common::compaction_harness;
use depot::{
	conveyer::Db,
	keys::PAGE_SIZE,
	types::{BucketId, DirtyPage, PitrPolicy},
	workflows::compaction::DepotCompactionTestDriver,
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0x9011c7), 1)
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

#[tokio::test]
async fn hot_compaction_succeeds_with_non_default_pitr_policy() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "policy-scope-livelock";
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

	// One-minute intervals: commits one minute apart fall into different custom
	// interval buckets but the same default five-minute bucket, so the coverage
	// selection genuinely differs between the policies.
	let custom_policy = PitrPolicy {
		interval_ms: 60_000,
		retention_ms: 24 * 60 * 60 * 1000,
	};
	depot::policy::set_bucket_pitr_policy(
		&udb,
		BucketId::from_gas_id(test_bucket()),
		custom_policy,
	)
	.await?;

	let default_interval_ms = depot::types::DEFAULT_PITR_INTERVAL_MS;
	let bucket_start =
		(now_ms()? - 2 * default_interval_ms).div_euclid(default_interval_ms) * default_interval_ms;
	db.commit(vec![page(1, 0x11)], 8, bucket_start + 10_000)
		.await?;
	db.commit(vec![page(2, 0x12)], 8, bucket_start + 70_000)
		.await?;

	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	assert_eq!(
		snapshot.hot_watermark_txid(),
		2,
		"hot compaction must install under a non-default PITR policy"
	);
	assert_eq!(
		snapshot.pitr_interval_txids(),
		[1, 2].into_iter().collect(),
		"coverage rows must bucket by the effective one-minute interval"
	);
	assert_eq!(
		snapshot.pitr_intervals.keys().copied().collect::<Vec<_>>(),
		vec![
			(bucket_start + 10_000).div_euclid(60_000) * 60_000,
			(bucket_start + 70_000).div_euclid(60_000) * 60_000,
		],
		"interval rows must use the effective policy's bucket starts"
	);

	test_ctx.shutdown().await?;
	Ok(())
}
