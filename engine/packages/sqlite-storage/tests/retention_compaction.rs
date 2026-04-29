use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use namespace::types::SqliteNamespaceConfig;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{SqliteCompactPayload, worker},
	keys::{PAGE_SIZE, delta_chunk_key, delta_meta_key, meta_checkpoints_key},
	pump::ActorDb,
	types::{Checkpoints, DirtyPage, decode_delta_meta, encode_checkpoints},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const DAY_MS: i64 = 86_400_000;
const TEST_ACTOR: &str = "retention-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-retention-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-retention-test".to_string(),
	)))
}

fn namespace_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		default_retention_ms: DAY_MS as u64,
		default_checkpoint_interval_ms: DAY_MS as u64,
		default_max_checkpoints: 25,
		allow_pitr_read: true,
		allow_pitr_destructive: false,
		allow_pitr_admin: true,
		allow_fork: false,
		pitr_max_bytes_per_actor: 100_000_000,
		pitr_namespace_budget_bytes: 100_000_000,
		max_retention_ms: DAY_MS as u64,
		admin_op_rate_per_min: 10,
		concurrent_admin_ops: 4,
		concurrent_forks_per_src: 2,
	}
}

fn pitr_config() -> sqlite_storage::compactor::CompactorConfig {
	sqlite_storage::compactor::CompactorConfig {
		pitr_enabled: true,
		batch_size_deltas: 32,
		#[cfg(debug_assertions)]
		quota_validate_every: 0,
		..sqlite_storage::compactor::CompactorConfig::default()
	}
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn now_ms() -> Result<i64> {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)?;
	Ok(i64::try_from(elapsed.as_millis())?)
}

async fn seed_namespace_config(db: &universaldb::Database, namespace_id: Id) -> Result<()> {
	db.run(move |tx| async move {
		let tx = tx.with_subspace(namespace::keys::subspace());
		tx.write(&namespace::keys::sqlite_config_key(namespace_id), namespace_config())?;
		Ok(())
	})
	.await
}

async fn write_deltas(db: Arc<universaldb::Database>, now_ms: i64) -> Result<()> {
	let actor_db = ActorDb::new(db, test_ups(), TEST_ACTOR.to_string(), NodeId::new());
	for txid in 1..=5 {
		actor_db
			.commit(vec![page(txid, txid as u8)], 8, now_ms)
			.await?;
	}

	Ok(())
}

async fn seed_checkpoint_index(db: &universaldb::Database) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal().set(
			&meta_checkpoints_key(TEST_ACTOR),
			&encode_checkpoints(Checkpoints {
				entries: vec![sqlite_storage::types::CheckpointEntry {
					ckp_txid: 5,
					taken_at_ms: 0,
					byte_count: 0,
					refcount: 0,
				}],
			})?,
		);
		Ok(())
	})
	.await
}

async fn run_compactor(db: Arc<universaldb::Database>, namespace_id: Id) -> Result<()> {
	worker::test_hooks::handle_payload_once(
		db,
		SqliteCompactPayload {
			actor_id: TEST_ACTOR.to_string(),
			namespace_id: Some(namespace_id),
			actor_name: Some("actor".to_string()),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		pitr_config(),
		CancellationToken::new(),
	)
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

async fn set_delta_refcount(db: &universaldb::Database, txid: u64, refcount: u32) -> Result<()> {
	db.run(move |tx| async move {
		let key = delta_meta_key(TEST_ACTOR, txid);
		let mut meta = decode_delta_meta(
			&tx.informal()
				.get(&key, Snapshot)
				.await?
				.expect("delta meta should exist"),
		)?;
		meta.refcount = refcount;
		tx.informal()
			.set(&key, &sqlite_storage::types::encode_delta_meta(meta)?);
		Ok(())
	})
	.await
}

async fn delta_exists(db: &universaldb::Database, txid: u64) -> Result<bool> {
	Ok(read_value(db, delta_chunk_key(TEST_ACTOR, txid, 0))
		.await?
		.is_some())
}

#[tokio::test]
async fn delta_preserved_within_retention() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace_id = Id::new_v1(3101);
	seed_namespace_config(&db, namespace_id).await?;
	write_deltas(Arc::clone(&db), now_ms()?).await?;
	seed_checkpoint_index(&db).await?;

	run_compactor(Arc::clone(&db), namespace_id).await?;

	for txid in 1..=5 {
		assert!(delta_exists(&db, txid).await?);
	}

	Ok(())
}

#[tokio::test]
async fn delta_deleted_past_retention() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace_id = Id::new_v1(3102);
	seed_namespace_config(&db, namespace_id).await?;
	write_deltas(Arc::clone(&db), 0).await?;
	seed_checkpoint_index(&db).await?;

	run_compactor(Arc::clone(&db), namespace_id).await?;

	for txid in 1..=5 {
		assert!(!delta_exists(&db, txid).await?);
	}

	Ok(())
}

#[tokio::test]
async fn delta_pinned_by_refcount() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace_id = Id::new_v1(3103);
	seed_namespace_config(&db, namespace_id).await?;
	write_deltas(Arc::clone(&db), 0).await?;
	set_delta_refcount(&db, 3, 1).await?;
	seed_checkpoint_index(&db).await?;

	run_compactor(Arc::clone(&db), namespace_id).await?;

	assert!(delta_exists(&db, 3).await?);
	assert!(!delta_exists(&db, 2).await?);

	Ok(())
}
