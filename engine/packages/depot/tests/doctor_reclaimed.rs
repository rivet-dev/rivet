//! Doctor must report a healthy verdict for databases whose history has been
//! reclaimed: commits below the watermark are keep-set islands and deltas are
//! folded into shard coverage, so diagnosis seeds its replay from shards
//! instead of expecting contiguous delta history from txid 1.

#![cfg(feature = "test-faults")]

mod common;

use anyhow::{Context, Result, ensure};
use common::compaction_harness;
use depot::{
	conveyer::Db,
	doctor::{DoctorInput, DoctorSelector, DoctorVerdictKind, SkipOptions, doctor},
	types::{BucketId, DirtyPage},
	workflows::compaction::DepotCompactionTestDriver,
};
use gas::prelude::{Id, TestCtx};
use rivet_pools::NodeId;
use rusqlite::{Connection, params};
use uuid::Uuid;

fn test_bucket() -> Id {
	Id::v1(Uuid::from_u128(0xd0c7), 1)
}

fn sqlite_dirty_pages(entries: &[(&str, &str)]) -> Result<Vec<DirtyPage>> {
	let dir = tempfile::tempdir()?;
	let path = dir.path().join("fixture.sqlite");
	let conn = Connection::open(&path)?;
	conn.execute_batch(
		"PRAGMA page_size=4096;
		 PRAGMA journal_mode=DELETE;
		 CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT NOT NULL);",
	)?;
	for (key, value) in entries {
		conn.execute(
			"INSERT OR REPLACE INTO kv (k, v) VALUES (?1, ?2)",
			params![key, value],
		)?;
	}
	conn.execute_batch("PRAGMA optimize;")?;
	drop(conn);

	let bytes = std::fs::read(&path)?;
	ensure!(
		bytes.len() % depot::keys::PAGE_SIZE as usize == 0,
		"SQLite fixture should be page aligned"
	);
	Ok(bytes
		.chunks(depot::keys::PAGE_SIZE as usize)
		.enumerate()
		.map(|(idx, bytes)| DirtyPage {
			pgno: u32::try_from(idx + 1).expect("fixture page number should fit in u32"),
			bytes: bytes.to_vec(),
		})
		.collect())
}

async fn commit_sqlite(db: &Db, entries: &[(&str, &str)], now_ms: i64) -> Result<()> {
	let pages = sqlite_dirty_pages(entries)?;
	let db_size_pages =
		u32::try_from(pages.len()).context("fixture page count should fit in u32")?;
	db.commit(pages, db_size_pages, now_ms).await
}

#[tokio::test]
async fn doctor_reports_reclaimed_database_as_healthy() -> Result<()> {
	let mut test_ctx = TestCtx::new(compaction_harness::build_registry()).await?;
	let database_id = "doctor-reclaimed";
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

	// Ancient wall clocks keep PITR interval rows out of retention so reclaim
	// punches maximal holes.
	commit_sqlite(&db, &[("alpha", "one")], 1_000).await?;
	commit_sqlite(&db, &[("alpha", "one"), ("beta", "two")], 2_000).await?;
	let branch_id = common::database_branch_id(&udb, test_bucket(), database_id).await?;
	let driver = DepotCompactionTestDriver::new(&test_ctx);
	let manager_workflow_id = driver.start_manager(branch_id, None, true).await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;

	commit_sqlite(
		&db,
		&[("alpha", "one"), ("beta", "two"), ("gamma", "three")],
		3_000,
	)
	.await?;
	compaction_harness::force_hot(&test_ctx, manager_workflow_id, branch_id).await?;
	compaction_harness::force_reclaim_until_idle(&test_ctx, manager_workflow_id, branch_id).await?;

	let snapshot = common::history(&udb, branch_id).await?;
	common::assert_delta_txids(&snapshot, [], "after reclaim");
	common::assert_commit_txids(&snapshot, [3], "after reclaim");

	let report = doctor(
		&udb,
		DoctorInput {
			selector: DoctorSelector::BucketDatabase {
				bucket_id: BucketId::from_gas_id(test_bucket()).as_uuid(),
				database_id: database_id.to_string(),
			},
			artifact_dir: None,
			skip: SkipOptions::default(),
			min_txid: None,
			max_txid: None,
			progress_hook: None,
		},
	)
	.await?;
	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Healthy,
		"reclaimed history is the steady state, not corruption: {:?}",
		report.verdict
	);

	test_ctx.shutdown().await?;
	Ok(())
}
