use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{
		AdminOpRecord, AuditFields, OpKind, OpStatus, encode_admin_op_record,
	},
	compactor::{cleanup_old_checkpoints, detect_refcount_leaks},
	keys::{
		checkpoint_meta_key, checkpoint_shard_key, delta_chunk_key, delta_meta_key,
		meta_admin_op_key, meta_checkpoints_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	types::{
		CheckpointEntry, CheckpointMeta, Checkpoints, DeltaMeta, DirtyPage, RetentionConfig,
		decode_checkpoint_meta, decode_checkpoints, decode_delta_meta, encode_checkpoint_meta,
		encode_checkpoints, encode_delta_meta,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

const DAY_MS: i64 = 86_400_000;
const TEST_ACTOR: &str = "checkpoint-cleanup-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-checkpoint-cleanup-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn retention_config() -> RetentionConfig {
	RetentionConfig {
		retention_ms: DAY_MS as u64,
		checkpoint_interval_ms: DAY_MS as u64,
		max_checkpoints: 25,
	}
}

fn checkpoint_entry(txid: u64, taken_at_ms: i64, refcount: u32) -> CheckpointEntry {
	CheckpointEntry {
		ckp_txid: txid,
		taken_at_ms,
		byte_count: 1,
		refcount,
	}
}

async fn seed_checkpoints(
	db: &universaldb::Database,
	entries: Vec<CheckpointEntry>,
) -> Result<()> {
	db.run(move |tx| {
		let entries = entries.clone();
		async move {
			tx.informal().set(
				&meta_checkpoints_key(TEST_ACTOR),
				&encode_checkpoints(Checkpoints {
					entries: entries.clone(),
				})?,
			);
			for entry in entries {
				tx.informal().set(
					&checkpoint_meta_key(TEST_ACTOR, entry.ckp_txid),
					&encode_checkpoint_meta(CheckpointMeta {
						taken_at_ms: entry.taken_at_ms,
						head_txid: entry.ckp_txid,
						db_size_pages: 1,
						byte_count: 1,
						refcount: entry.refcount,
						pinned_reason: None,
					})?,
				);
				tx.informal().set(
					&checkpoint_shard_key(TEST_ACTOR, entry.ckp_txid, 0),
					&[entry.ckp_txid as u8],
				);
			}
			Ok(())
		}
	})
	.await
}

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
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

async fn checkpoint_exists(db: &universaldb::Database, txid: u64) -> Result<bool> {
	Ok(read_value(db, checkpoint_meta_key(TEST_ACTOR, txid))
		.await?
		.is_some())
}

async fn checkpoint_index(db: &universaldb::Database) -> Result<Checkpoints> {
	Ok(decode_checkpoints(
		&read_value(db, meta_checkpoints_key(TEST_ACTOR))
			.await?
			.expect("checkpoint index should exist"),
	)?)
}

async fn seed_delta_meta(db: &universaldb::Database, txid: u64, taken_at_ms: i64) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal().set(
			&delta_chunk_key(TEST_ACTOR, txid, 0),
			&encode_ltx_v3(
				LtxHeader::delta(txid, 4, 1),
				&[DirtyPage {
					pgno: 1,
					bytes: vec![txid as u8; 4096],
				}],
			)?,
		);
		tx.informal().set(
			&delta_meta_key(TEST_ACTOR, txid),
			&encode_delta_meta(DeltaMeta {
				taken_at_ms,
				byte_count: 1,
				refcount: 1,
			})?,
		);
		Ok(())
	})
	.await
}

async fn seed_active_admin_op(db: &universaldb::Database) -> Result<()> {
	let op_id = Uuid::new_v4();
	db.run(move |tx| async move {
		tx.informal().set(
			&meta_admin_op_key(TEST_ACTOR, op_id),
			&encode_admin_op_record(AdminOpRecord {
				operation_id: op_id,
				op_kind: OpKind::Fork,
				actor_id: TEST_ACTOR.to_string(),
				created_at_ms: 0,
				last_progress_at_ms: 0,
				status: OpStatus::InProgress,
				holder_id: Some(NodeId::new()),
				progress: None,
				result: None,
				audit: AuditFields {
					caller_id: "test".to_string(),
					request_origin_ts_ms: 0,
					namespace_id: Uuid::new_v4(),
				},
			})?,
		);
		Ok(())
	})
	.await
}

async fn read_delta_refcount(db: &universaldb::Database, txid: u64) -> Result<u32> {
	let bytes = read_value(db, delta_meta_key(TEST_ACTOR, txid))
		.await?
		.expect("delta meta should exist");
	Ok(decode_delta_meta(&bytes)?.refcount)
}

#[tokio::test]
async fn cleanup_old_checkpoints_basic() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_checkpoints(
		&db,
		(1..=5)
			.map(|txid| checkpoint_entry(txid, 0, 0))
			.collect(),
	)
	.await?;

	let outcome = cleanup_old_checkpoints(
		Arc::clone(&db),
		TEST_ACTOR.to_string(),
		retention_config(),
		DAY_MS + 1,
	)
	.await?;

	assert_eq!(outcome.checkpoints_deleted, 4);
	for txid in 1..=4 {
		assert!(!checkpoint_exists(&db, txid).await?);
	}
	assert!(checkpoint_exists(&db, 5).await?);
	assert_eq!(checkpoint_index(&db).await?.entries.len(), 1);

	Ok(())
}

#[tokio::test]
async fn cleanup_old_checkpoints_skips_pinned() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_checkpoints(
		&db,
		vec![
			checkpoint_entry(1, 0, 1),
			checkpoint_entry(2, 0, 0),
		],
	)
	.await?;

	cleanup_old_checkpoints(
		Arc::clone(&db),
		TEST_ACTOR.to_string(),
		retention_config(),
		DAY_MS + 1,
	)
	.await?;

	assert!(checkpoint_exists(&db, 1).await?);
	assert!(checkpoint_exists(&db, 2).await?);

	Ok(())
}

#[tokio::test]
async fn cleanup_old_checkpoints_keeps_latest() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_checkpoints(&db, vec![checkpoint_entry(1, 0, 0)]).await?;

	cleanup_old_checkpoints(
		Arc::clone(&db),
		TEST_ACTOR.to_string(),
		retention_config(),
		DAY_MS + 1,
	)
	.await?;

	assert!(checkpoint_exists(&db, 1).await?);

	Ok(())
}

#[tokio::test]
async fn detect_refcount_leak_resets_after_window() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_checkpoints(&db, vec![checkpoint_entry(1, 0, 1)]).await?;
	seed_delta_meta(&db, 1, 0).await?;

	let outcome = detect_refcount_leaks(Arc::clone(&db), TEST_ACTOR.to_string(), 101, 10).await?;

	assert_eq!(outcome.checkpoint_refs_reset, 1);
	assert_eq!(outcome.delta_refs_reset, 1);
	let checkpoint_meta = decode_checkpoint_meta(
		&read_value(&db, checkpoint_meta_key(TEST_ACTOR, 1))
			.await?
			.expect("checkpoint meta should exist"),
	)?;
	assert_eq!(checkpoint_meta.refcount, 0);
	assert_eq!(checkpoint_index(&db).await?.entries[0].refcount, 0);
	assert_eq!(read_delta_refcount(&db, 1).await?, 0);

	Ok(())
}

#[tokio::test]
async fn detect_refcount_leak_skips_active_op() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_checkpoints(&db, vec![checkpoint_entry(1, 0, 1)]).await?;
	seed_delta_meta(&db, 1, 0).await?;
	seed_active_admin_op(&db).await?;

	let outcome = detect_refcount_leaks(Arc::clone(&db), TEST_ACTOR.to_string(), 101, 10).await?;

	assert_eq!(outcome.checkpoint_refs_reset, 0);
	assert_eq!(outcome.delta_refs_reset, 0);
	let checkpoint_meta = decode_checkpoint_meta(
		&read_value(&db, checkpoint_meta_key(TEST_ACTOR, 1))
			.await?
			.expect("checkpoint meta should exist"),
	)?;
	assert_eq!(checkpoint_meta.refcount, 1);
	assert_eq!(read_delta_refcount(&db, 1).await?, 1);

	Ok(())
}
