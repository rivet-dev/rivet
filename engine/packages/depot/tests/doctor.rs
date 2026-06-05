mod common;

use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use depot::{
	doctor::{
		CorruptionClass, DoctorInput, DoctorProgressPhase, DoctorSelector, DoctorVerdictKind,
		SkipOptions, doctor,
	},
	keys,
	ltx::{LtxHeader, encode_ltx_v3},
	types::{
		BucketId, DatabaseBranchId, DirtyPage, decode_commit_row, decode_database_branch_record,
		encode_database_branch_record,
	},
};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use rusqlite::{Connection, params};
use universaldb::{
	KeySelector, RangeOption, options::StreamingMode, utils::IsolationLevel::Snapshot,
};

async fn run_doctor(
	ctx: &common::TestDb,
	max_txid: Option<u64>,
) -> Result<depot::doctor::DoctorReport> {
	doctor(
		&ctx.udb,
		DoctorInput {
			selector: DoctorSelector::BucketDatabase {
				bucket_id: BucketId::from_gas_id(ctx.bucket_id).as_uuid(),
				database_id: ctx.database_id.clone(),
			},
			artifact_dir: None,
			skip: SkipOptions::default(),
			min_txid: None,
			max_txid,
			progress_hook: None,
		},
	)
	.await
}

async fn run_actor_doctor(
	ctx: &common::TestDb,
	actor_id: Id,
) -> Result<depot::doctor::DoctorReport> {
	doctor(
		&ctx.udb,
		DoctorInput {
			selector: DoctorSelector::Actor {
				namespace_id: ctx.bucket_id,
				actor_id,
			},
			artifact_dir: None,
			skip: SkipOptions::default(),
			min_txid: None,
			max_txid: None,
			progress_hook: None,
		},
	)
	.await
}

async fn branch_id(ctx: &common::TestDb) -> Result<DatabaseBranchId> {
	let bucket_id = BucketId::from_gas_id(ctx.bucket_id);
	let database_id = ctx.database_id.clone();
	ctx.udb
		.txn("test_depotdoctor", move |tx| {
			let database_id = database_id.clone();
			async move {
				depot::conveyer::branch::resolve_database_branch(
					&tx,
					bucket_id,
					&database_id,
					Snapshot,
				)
				.await?
				.context("test database branch should exist")
			}
		})
		.await
}

async fn set_value(db: &universaldb::Database, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
	db.txn("test_depotdoctor", move |tx| {
		let key = key.clone();
		let value = value.clone();
		async move {
			tx.informal().set(&key, &value);
			Ok(())
		}
	})
	.await
}

async fn clear_value(db: &universaldb::Database, key: Vec<u8>) -> Result<()> {
	db.txn("test_depotdoctor", move |tx| {
		let key = key.clone();
		async move {
			tx.informal().clear(&key);
			Ok(())
		}
	})
	.await
}

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.txn("test_depotdoctor", move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

async fn count_prefix(db: &universaldb::Database, prefix: Vec<u8>) -> Result<usize> {
	db.txn("test_depotdoctor", move |tx| {
		let prefix = prefix.clone();
		async move {
			let (range_start, range_end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					begin: KeySelector::first_greater_or_equal(range_start),
					end: KeySelector::first_greater_or_equal(range_end),
					mode: StreamingMode::WantAll,
					..RangeOption::default()
				},
				Snapshot,
			);
			let mut count = 0usize;
			while stream.try_next().await?.is_some() {
				count += 1;
			}
			Ok(count)
		}
	})
	.await
}

#[derive(Debug, PartialEq, Eq)]
struct SideEffectSnapshot {
	head: Option<Vec<u8>>,
	commit_count: usize,
	delta_count: usize,
	pidx_count: usize,
	hot_shard_count: usize,
	staged_hot_shard_count: usize,
	access_ts: Option<Vec<u8>>,
	access_bucket: Option<Vec<u8>>,
	dirty_marker: Option<Vec<u8>>,
}

async fn side_effect_snapshot(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<SideEffectSnapshot> {
	Ok(SideEffectSnapshot {
		head: read_value(db, keys::branch_meta_head_key(branch_id)).await?,
		commit_count: count_prefix(db, keys::branch_commit_prefix(branch_id)).await?,
		delta_count: count_prefix(db, keys::branch_delta_prefix(branch_id)).await?,
		pidx_count: count_prefix(db, keys::branch_pidx_prefix(branch_id)).await?,
		hot_shard_count: count_prefix(db, keys::branch_shard_prefix(branch_id)).await?,
		staged_hot_shard_count: count_prefix(db, keys::branch_compaction_stage_prefix(branch_id))
			.await?,
		access_ts: read_value(db, keys::branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		access_bucket: read_value(db, keys::branch_manifest_last_access_bucket_key(branch_id))
			.await?,
		dirty_marker: read_value(db, keys::sqlite_cmp_dirty_key(branch_id)).await?,
	})
}

// These fixtures use rusqlite to create known-good SQLite page bytes. That keeps
// doctor tests focused on Depot storage and reports instead of also exercising
// depot-client VFS registration, native worker lifecycle, fencing, flush timing,
// and transport behavior.
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
		bytes.len() % keys::PAGE_SIZE as usize == 0,
		"SQLite fixture should be page aligned"
	);
	Ok(bytes
		.chunks(keys::PAGE_SIZE as usize)
		.enumerate()
		.map(|(idx, bytes)| DirtyPage {
			pgno: u32::try_from(idx + 1).expect("fixture page number should fit in u32"),
			bytes: bytes.to_vec(),
		})
		.collect())
}

async fn commit_sqlite(ctx: &common::TestDb, entries: &[(&str, &str)], now_ms: i64) -> Result<()> {
	let pages = sqlite_dirty_pages(entries)?;
	let db_size_pages =
		u32::try_from(pages.len()).context("fixture page count should fit in u32")?;
	ctx.db.commit(pages, db_size_pages, now_ms).await
}

async fn overwrite_delta(
	ctx: &common::TestDb,
	branch_id: DatabaseBranchId,
	txid: u64,
	db_size_pages: u32,
	pages: Vec<DirtyPage>,
) -> Result<()> {
	let blob = encode_ltx_v3(
		LtxHeader::delta(txid, db_size_pages, 10_000 + txid as i64),
		&pages,
	)?;
	set_value(
		&ctx.udb,
		keys::branch_delta_chunk_key(branch_id, txid, 0),
		blob,
	)
	.await
}

fn assert_contract(report: &depot::doctor::DoctorReport) -> Result<()> {
	let value = serde_json::to_value(report)?;
	for key in [
		"verdict",
		"generated_at_ms",
		"duration_ms",
		"selected_head_txid",
		"selected_db_size_pages",
		"first_bad_txid",
		"previous_good_txid",
		"identity",
		"analysis_scope",
		"facts",
		"analyses",
		"artifacts",
	] {
		assert!(value.get(key).is_some(), "missing top-level key {key}");
	}
	for key in [
		"commit_chain",
		"versionstamp_index",
		"delta_integrity",
		"pidx_integrity",
		"sqlite_reconstruction",
		"first_bad_txid",
	] {
		if report.analyses.get("commit_chain").is_some() {
			assert!(
				report.analyses.get(key).is_some(),
				"missing analysis key {key}"
			);
		}
	}
	Ok(())
}

#[tokio::test]
async fn doctor_reports_healthy_root_database() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-healthy", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2"), ("b", "3")], 2_000).await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Healthy,
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.selected_head_txid, Some(2));
	assert_eq!(
		report.analyses["delta_integrity"]["ok"].as_bool(),
		Some(true)
	);
	Ok(())
}

#[tokio::test]
async fn doctor_reports_actor_without_depot_database_as_inconclusive() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-missing-actor-db", common::TierMode::Disabled).await?;
	let actor_id = Id::new_v1(42);

	let report = run_actor_doctor(&ctx, actor_id).await?;

	assert_contract(&report)?;
	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Inconclusive);
	assert_eq!(
		report.verdict.reason,
		Some("depot_actor_database_not_found")
	);
	assert_eq!(report.facts["depot_branch"]["found"].as_bool(), Some(false));
	assert_eq!(
		report.identity["detected_storage_kind"].as_str(),
		Some("actor_without_depot_sqlite_database")
	);
	Ok(())
}

#[tokio::test]
async fn doctor_reports_legacy_actor_storage_as_unsupported() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-legacy-actor-db", common::TierMode::Disabled).await?;
	let actor_id = Id::new_v1(43);

	set_value(
		&ctx.udb,
		keys::meta_head_key(&actor_id.to_string()),
		b"legacy-head".to_vec(),
	)
	.await?;

	let report = run_actor_doctor(&ctx, actor_id).await?;

	assert_contract(&report)?;
	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Unsupported);
	assert_eq!(
		report.verdict.unsupported_reason,
		Some("unsupported_legacy_depot_database_scoped_storage")
	);
	assert_eq!(
		report.identity["detected_storage_kind"].as_str(),
		Some("legacy_depot_database_scoped")
	);
	assert_eq!(
		report.facts["legacy_depot_database_scoped"]["head_present"].as_bool(),
		Some(true)
	);
	Ok(())
}

#[tokio::test]
async fn doctor_reports_malformed_delta_chunk_as_json() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-bad-delta", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;

	set_value(
		&ctx.udb,
		keys::branch_delta_chunk_key(branch_id, 1, 0),
		b"not an ltx blob".to_vec(),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("sequential_replay_invalid"));
	assert_eq!(
		report.analyses["delta_integrity"]["ok"].as_bool(),
		Some(false)
	);
	assert_eq!(
		report.analyses["sqlite_reconstruction"]["skip_reason"].as_str(),
		Some("delta_integrity_failed")
	);
	Ok(())
}

#[tokio::test]
async fn doctor_reports_non_contiguous_delta_chunks_as_json() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-missing-delta-chunk",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let chunk = read_value(&ctx.udb, keys::branch_delta_chunk_key(branch_id, 1, 0))
		.await?
		.context("delta chunk 0 should exist")?;

	set_value(
		&ctx.udb,
		keys::branch_delta_chunk_key(branch_id, 1, 1),
		chunk,
	)
	.await?;
	clear_value(&ctx.udb, keys::branch_delta_chunk_key(branch_id, 1, 0)).await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(
		report.analyses["delta_integrity"]["ok"].as_bool(),
		Some(false)
	);
	assert_eq!(
		report.facts["deltas"]["rows"][0]["decode_status"].as_str(),
		Some("decode_error")
	);
	Ok(())
}

#[tokio::test]
async fn doctor_verdict_uses_commit_chain_gap_analysis() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-commit-gap", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	let branch_id = branch_id(&ctx).await?;

	clear_value(&ctx.udb, keys::branch_commit_key(branch_id, 1)).await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("commit_chain_failed"));
	assert_eq!(report.analyses["commit_chain"]["ok"].as_bool(), Some(false));
	Ok(())
}

#[tokio::test]
async fn doctor_verdict_uses_pidx_mismatch_analysis() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-pidx", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	let branch_id = branch_id(&ctx).await?;

	set_value(
		&ctx.udb,
		keys::branch_pidx_key(branch_id, 1),
		1u64.to_be_bytes().to_vec(),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("pidx_integrity_failed"));
	assert_eq!(
		report.analyses["pidx_integrity"]["ok"].as_bool(),
		Some(false)
	);
	Ok(())
}

#[tokio::test]
async fn doctor_reports_hot_shard_mismatch() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-hot-mismatch", common::TierMode::Disabled).await?;
	let old_pages = sqlite_dirty_pages(&[("a", "1")])?;
	let old_db_size_pages =
		u32::try_from(old_pages.len()).context("fixture page count should fit in u32")?;
	ctx.db
		.commit(old_pages.clone(), old_db_size_pages, 1_000)
		.await?;
	let branch_id = branch_id(&ctx).await?;
	let old_page = old_pages
		.iter()
		.find(|page| page.pgno == 2)
		.context("fixture should include page 2")?
		.clone();
	let hot_blob = encode_ltx_v3(LtxHeader::delta(1, old_db_size_pages, 1_000), &[old_page])?;
	set_value(
		&ctx.udb,
		keys::branch_shard_key(branch_id, 2 / keys::SHARD_SIZE, 1),
		hot_blob,
	)
	.await?;

	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	clear_value(&ctx.udb, keys::branch_pidx_key(branch_id, 2)).await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Corrupt,
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.verdict.reason, Some("depot_resolved_image_differs"));
	assert_eq!(
		report.verdict.corruption_class,
		depot::doctor::CorruptionClass::HotCompactionState
	);
	assert_eq!(report.first_bad_txid, Some(2));
	assert!(
		report.analyses["hot_compaction_mismatches"]["mismatch_count"]
			.as_u64()
			.is_some_and(|count| count > 0),
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	Ok(())
}

#[tokio::test]
async fn doctor_verdict_uses_missing_vtx_analysis() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-vtx", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let commit_bytes = read_value(&ctx.udb, keys::branch_commit_key(branch_id, 1))
		.await?
		.context("commit row should exist")?;
	let commit = decode_commit_row(&commit_bytes)?;

	clear_value(
		&ctx.udb,
		keys::branch_vtx_key(branch_id, commit.versionstamp),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("versionstamp_index_failed"));
	assert_eq!(
		report.analyses["versionstamp_index"]["ok"].as_bool(),
		Some(false)
	);
	Ok(())
}

#[tokio::test]
async fn doctor_rejects_zero_max_txid() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-max-zero", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;

	let report = run_doctor(&ctx, Some(0)).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Inconclusive);
	assert_eq!(report.verdict.reason, Some("invalid_max_txid"));
	Ok(())
}

#[tokio::test]
async fn doctor_max_txid_selects_existing_commit() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-max-existing", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;

	let report = run_doctor(&ctx, Some(1)).await?;

	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Healthy,
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.selected_head_txid, Some(1));
	assert_eq!(report.analysis_scope["selected_txid"].as_u64(), Some(1));
	assert_eq!(
		report.analysis_scope["selected_txid_is_live_head"].as_bool(),
		Some(false)
	);
	Ok(())
}

#[tokio::test]
async fn doctor_max_txid_selects_latest_existing_commit_below_gap() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-max-gap", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	commit_sqlite(&ctx, &[("a", "3")], 3_000).await?;
	let branch_id = branch_id(&ctx).await?;

	clear_value(&ctx.udb, keys::branch_commit_key(branch_id, 2)).await?;

	let report = run_doctor(&ctx, Some(2)).await?;

	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Healthy,
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.selected_head_txid, Some(1));
	assert_eq!(report.analysis_scope["max_txid"].as_u64(), Some(2));
	assert_eq!(report.analysis_scope["selected_txid"].as_u64(), Some(1));
	Ok(())
}

#[tokio::test]
async fn doctor_stops_on_unsupported_pitr_state() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-pitr", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;

	set_value(
		&ctx.udb,
		keys::branch_pitr_interval_key(branch_id, 0),
		b"unsupported".to_vec(),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Unsupported);
	assert_eq!(
		report.verdict.unsupported_reason,
		Some("unsupported_unexpected_storage_shape")
	);
	Ok(())
}

#[tokio::test]
async fn doctor_stops_on_unsupported_fork_state() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-fork", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let record_bytes = read_value(&ctx.udb, keys::branches_list_key(branch_id))
		.await?
		.context("branch record should exist")?;
	let mut record = decode_database_branch_record(&record_bytes)?;
	record.fork_depth = 1;

	set_value(
		&ctx.udb,
		keys::branches_list_key(branch_id),
		encode_database_branch_record(record)?,
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Unsupported);
	assert_eq!(
		report.verdict.unsupported_reason,
		Some("unsupported_unexpected_storage_shape")
	);
	Ok(())
}

#[tokio::test]
async fn doctor_diagnostic_resolver_does_not_touch_access_metadata() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-no-side-effects", common::TierMode::Disabled).await?;
	let pages = sqlite_dirty_pages(&[("a", "1")])?;
	let db_size_pages =
		u32::try_from(pages.len()).context("fixture page count should fit in u32")?;
	ctx.db.commit(pages.clone(), db_size_pages, 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let page = pages
		.iter()
		.find(|page| page.pgno == 2)
		.context("fixture should include page 2")?
		.clone();
	let hot_blob = encode_ltx_v3(LtxHeader::delta(1, db_size_pages, 1_000), &[page])?;
	set_value(
		&ctx.udb,
		keys::branch_shard_key(branch_id, 2 / keys::SHARD_SIZE, 1),
		hot_blob,
	)
	.await?;
	clear_value(&ctx.udb, keys::branch_pidx_key(branch_id, 2)).await?;
	let before = side_effect_snapshot(&ctx.udb, branch_id).await?;

	let report = run_doctor(&ctx, None).await?;
	let after = side_effect_snapshot(&ctx.udb, branch_id).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Healthy);
	assert_eq!(after, before);
	Ok(())
}

#[tokio::test]
async fn doctor_finds_first_bad_sequential_txid() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-first-bad-sequential",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let commit_bytes = read_value(&ctx.udb, keys::branch_commit_key(branch_id, 2))
		.await?
		.context("commit row should exist")?;
	let commit = decode_commit_row(&commit_bytes)?;
	let corrupt_header_page = DirtyPage {
		pgno: 1,
		bytes: vec![0; keys::PAGE_SIZE as usize],
	};
	overwrite_delta(
		&ctx,
		branch_id,
		2,
		commit.db_size_pages,
		vec![corrupt_header_page],
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(
		report.verdict.corruption_class,
		CorruptionClass::DeltaHistory
	);
	assert_eq!(report.verdict.reason, Some("sequential_replay_invalid"));
	assert_eq!(report.first_bad_txid, Some(2));
	assert_eq!(report.previous_good_txid, Some(1));
	assert_eq!(
		report.analyses["first_bad_txid"]["pages_changed_by_first_bad_txid"]["rows"][0].as_u64(),
		Some(1)
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_reports_pidx_pointing_at_missing_delta() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-pidx-missing-delta",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;

	set_value(
		&ctx.udb,
		keys::branch_pidx_key(branch_id, 1),
		99u64.to_be_bytes().to_vec(),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("pidx_integrity_failed"));
	let pidx = report.facts["pidx"]["rows"]
		.as_array()
		.context("pidx rows should be an array")?;
	let row = pidx
		.iter()
		.find(|row| row["pgno"].as_u64() == Some(1))
		.context("page 1 pidx fact should exist")?;
	assert_eq!(row["owner_txid"].as_u64(), Some(99));
	assert_eq!(row["owner_txid_exists"].as_bool(), Some(false));
	assert_eq!(row["owner_delta_contains_page"].as_bool(), Some(false));
	assert_eq!(
		row["stored_owner_matches_computed_owner"].as_bool(),
		Some(false)
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_min_txid_only_moves_first_bad_search_start() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-min-search-start", common::TierMode::Disabled).await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	commit_sqlite(&ctx, &[("a", "3")], 3_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let corrupt_header_page = DirtyPage {
		pgno: 1,
		bytes: vec![0; keys::PAGE_SIZE as usize],
	};
	for txid in [2, 3] {
		let commit_bytes = read_value(&ctx.udb, keys::branch_commit_key(branch_id, txid))
			.await?
			.context("commit row should exist")?;
		let commit = decode_commit_row(&commit_bytes)?;
		overwrite_delta(
			&ctx,
			branch_id,
			txid,
			commit.db_size_pages,
			vec![corrupt_header_page.clone()],
		)
		.await?;
	}

	let full_report = run_doctor(&ctx, None).await?;
	let min_report = doctor(
		&ctx.udb,
		DoctorInput {
			selector: DoctorSelector::BucketDatabase {
				bucket_id: BucketId::from_gas_id(ctx.bucket_id).as_uuid(),
				database_id: ctx.database_id.clone(),
			},
			artifact_dir: None,
			skip: SkipOptions::default(),
			min_txid: Some(3),
			max_txid: None,
			progress_hook: None,
		},
	)
	.await?;

	assert_eq!(full_report.first_bad_txid, Some(2));
	assert_eq!(min_report.first_bad_txid, Some(3));
	assert_eq!(min_report.facts["commits"]["total_count"].as_u64(), Some(3));
	assert_eq!(min_report.analysis_scope["min_txid"].as_u64(), Some(3));
	assert_contract(&min_report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_ignores_hot_shard_above_selected_max_txid() -> Result<()> {
	let ctx =
		common::build_test_db("depot-doctor-hot-above-max", common::TierMode::Disabled).await?;
	let old_pages = sqlite_dirty_pages(&[("a", "1")])?;
	let old_db_size_pages =
		u32::try_from(old_pages.len()).context("fixture page count should fit in u32")?;
	ctx.db
		.commit(old_pages.clone(), old_db_size_pages, 1_000)
		.await?;
	commit_sqlite(&ctx, &[("a", "2")], 2_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let old_page = old_pages
		.iter()
		.find(|page| page.pgno == 2)
		.context("fixture should include page 2")?
		.clone();
	let hot_blob = encode_ltx_v3(LtxHeader::delta(2, old_db_size_pages, 2_000), &[old_page])?;
	set_value(
		&ctx.udb,
		keys::branch_shard_key(branch_id, 2 / keys::SHARD_SIZE, 2),
		hot_blob,
	)
	.await?;

	let report = run_doctor(&ctx, Some(1)).await?;

	assert_eq!(
		report.verdict.verdict,
		DoctorVerdictKind::Healthy,
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.selected_head_txid, Some(1));
	assert_eq!(
		report.facts["hot_shards"]["rows"][0]["eligible_at_selected_txid"].as_bool(),
		Some(false)
	);
	assert_eq!(
		report.facts["page_provenance"]["rows"]
			.as_array()
			.map(|rows| rows
				.iter()
				.any(|row| row["depot_winner_kind"].as_str() == Some("hot_shard")))
			.unwrap_or(false),
		false
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_marks_changed_during_diagnosis_inconclusive() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-changed-during-diagnosis",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let udb = ctx.udb.clone();
	let hook: depot::doctor::DoctorProgressHook = Arc::new(move |phase| {
		let udb = udb.clone();
		Box::pin(async move {
			match phase {
				DoctorProgressPhase::AfterStartSnapshot => {
					set_value(
						&udb,
						keys::branch_compaction_stage_hot_shard_key(
							branch_id,
							Id::new_v1(42),
							0,
							1,
							0,
						),
						b"changed".to_vec(),
					)
					.await?;
				}
			}
			Ok(())
		})
	});

	let report = doctor(
		&ctx.udb,
		DoctorInput {
			selector: DoctorSelector::BucketDatabase {
				bucket_id: BucketId::from_gas_id(ctx.bucket_id).as_uuid(),
				database_id: ctx.database_id.clone(),
			},
			artifact_dir: None,
			skip: SkipOptions::default(),
			min_txid: None,
			max_txid: None,
			progress_hook: Some(hook),
		},
	)
	.await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Inconclusive);
	assert_eq!(
		report.verdict.reason,
		Some("database_changed_during_diagnosis")
	);
	assert_eq!(
		report.analysis_scope["changed_during_diagnosis"].as_bool(),
		Some(true)
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_large_scan_reports_chunk_counts_and_limits() -> Result<()> {
	let ctx = common::build_test_db("depot-doctor-large-scan", common::TierMode::Disabled).await?;
	let pages = sqlite_dirty_pages(&[("a", "1")])?;
	let db_size_pages =
		u32::try_from(pages.len()).context("fixture page count should fit in u32")?;
	ctx.db.commit(pages.clone(), db_size_pages, 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	let hot_page = pages
		.iter()
		.find(|page| page.pgno == 2)
		.context("fixture should include page 2")?
		.clone();
	let hot_blob = encode_ltx_v3(LtxHeader::delta(2, db_size_pages, 2_000), &[hot_page])?;

	for idx in 0..1_100u64 {
		let mut versionstamp = [0u8; 16];
		versionstamp[0] = 0x80;
		versionstamp[8..16].copy_from_slice(&idx.to_be_bytes());
		set_value(
			&ctx.udb,
			keys::branch_vtx_key(branch_id, versionstamp),
			(idx + 2).to_be_bytes().to_vec(),
		)
		.await?;
		let pgno = 10_000 + u32::try_from(idx).context("test index should fit in u32")?;
		set_value(
			&ctx.udb,
			keys::branch_pidx_key(branch_id, pgno),
			1u64.to_be_bytes().to_vec(),
		)
		.await?;
		set_value(
			&ctx.udb,
			keys::branch_shard_key(branch_id, pgno, 2),
			hot_blob.clone(),
		)
		.await?;
	}

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Healthy);
	assert!(
		report.facts["storage_inventory"]["scan_chunks"]["vtx"]
			.as_u64()
			.is_some_and(|chunks| chunks > 1),
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert!(
		report.facts["storage_inventory"]["scan_chunks"]["pidx"]
			.as_u64()
			.is_some_and(|chunks| chunks > 1),
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert!(
		report.facts["storage_inventory"]["scan_chunks"]["hot_shards"]
			.as_u64()
			.is_some_and(|chunks| chunks > 1),
		"{}",
		serde_json::to_string_pretty(&report)?
	);
	assert_eq!(report.facts["vtx"]["total_count"].as_u64(), Some(1_101));
	assert_eq!(report.facts["pidx"]["total_count"].as_u64(), Some(1_105));
	assert_eq!(
		report.facts["hot_shards"]["total_count"].as_u64(),
		Some(1_100)
	);
	assert_eq!(report.facts["vtx"]["omitted_count"].as_u64(), Some(0));
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_classifies_resolver_zero_fill_divergence_as_depot_reconstruction() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-resolver-zero-fill",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;

	clear_value(&ctx.udb, keys::branch_pidx_key(branch_id, 1)).await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(
		report.verdict.corruption_class,
		CorruptionClass::DepotReconstruction
	);
	assert_eq!(report.verdict.reason, Some("depot_resolved_image_differs"));
	assert_eq!(
		report.analyses["sqlite_reconstruction"]["mismatch_buckets"]["depot_zero_filled_but_replay_nonzero"]["rows"][0]
			.as_u64(),
		Some(1)
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_returns_structured_json_for_hot_resolver_error() -> Result<()> {
	let ctx = common::build_test_db(
		"depot-doctor-hot-resolver-error",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&ctx, &[("a", "1")], 1_000).await?;
	let branch_id = branch_id(&ctx).await?;
	clear_value(&ctx.udb, keys::branch_pidx_key(branch_id, 2)).await?;
	set_value(
		&ctx.udb,
		keys::branch_shard_key(branch_id, 2 / keys::SHARD_SIZE, 1),
		b"not an ltx blob".to_vec(),
	)
	.await?;

	let report = run_doctor(&ctx, None).await?;

	assert_eq!(report.verdict.verdict, DoctorVerdictKind::Corrupt);
	assert_eq!(report.verdict.reason, Some("depot_resolver_failed"));
	assert_eq!(
		report.analyses["resolver_errors"]["kind"].as_str(),
		Some("malformed_source_blob")
	);
	assert_eq!(
		report.facts["hot_shards"]["rows"][0]["decode_status"].as_str(),
		Some("error")
	);
	assert_contract(&report)?;
	Ok(())
}

#[tokio::test]
async fn doctor_output_contract_covers_report_shapes() -> Result<()> {
	let healthy =
		common::build_test_db("depot-doctor-contract-healthy", common::TierMode::Disabled).await?;
	commit_sqlite(&healthy, &[("a", "1")], 1_000).await?;
	assert_contract(&run_doctor(&healthy, None).await?)?;

	let delta =
		common::build_test_db("depot-doctor-contract-delta", common::TierMode::Disabled).await?;
	commit_sqlite(&delta, &[("a", "1")], 1_000).await?;
	let delta_branch = branch_id(&delta).await?;
	set_value(
		&delta.udb,
		keys::branch_delta_chunk_key(delta_branch, 1, 0),
		b"not an ltx blob".to_vec(),
	)
	.await?;
	assert_contract(&run_doctor(&delta, None).await?)?;

	let unsupported = common::build_test_db(
		"depot-doctor-contract-unsupported",
		common::TierMode::Disabled,
	)
	.await?;
	commit_sqlite(&unsupported, &[("a", "1")], 1_000).await?;
	let unsupported_branch = branch_id(&unsupported).await?;
	set_value(
		&unsupported.udb,
		keys::branch_pitr_interval_key(unsupported_branch, 0),
		b"unsupported".to_vec(),
	)
	.await?;
	assert_contract(&run_doctor(&unsupported, None).await?)?;

	Ok(())
}
