use std::{
	sync::{
		Arc,
		atomic::Ordering,
	},
	time::Duration,
};

use anyhow::Result;
use gas::prelude::Id;
use namespace::types::SqliteNamespaceConfig;
use sqlite_storage::{
	compactor::{
		CheckpointOutcome, CompactorConfig, checkpoint, create_checkpoint,
		worker,
	},
	keys::{
		PAGE_SIZE, checkpoint_meta_key, checkpoint_pidx_delta_key, checkpoint_shard_key,
		delta_chunk_key, meta_checkpoints_key, meta_compact_key, meta_head_key,
		pidx_delta_key, shard_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	quota,
	types::{
		DBHead, DirtyPage, MetaCompact, decode_checkpoint_meta, decode_checkpoints,
		encode_db_head, encode_meta_compact,
	},
};
use tempfile::Builder;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;

static CHECKPOINT_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-checkpoint-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn namespace_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		default_retention_ms: 86_400_000,
		default_checkpoint_interval_ms: 1,
		default_max_checkpoints: 25,
		allow_pitr_read: true,
		allow_pitr_destructive: false,
		allow_pitr_admin: true,
		allow_fork: false,
		pitr_max_bytes_per_actor: 100_000_000,
		pitr_namespace_budget_bytes: 100_000_000,
		max_retention_ms: 86_400_000,
		admin_op_rate_per_min: 10,
		concurrent_admin_ops: 4,
		concurrent_forks_per_src: 2,
	}
}

fn pitr_config() -> CompactorConfig {
	CompactorConfig {
		pitr_enabled: true,
		max_concurrent_workers: 64,
		max_concurrent_checkpoints: 16,
		..CompactorConfig::default()
	}
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn encoded_blob(txid: u64, pages: &[(u32, u8)]) -> Result<Vec<u8>> {
	let pages = pages
		.iter()
		.map(|(pgno, fill)| page(*pgno, *fill))
		.collect::<Vec<_>>();

	encode_ltx_v3(LtxHeader::delta(txid, 512, 999), &pages)
}

async fn seed_actor(db: &universaldb::Database, actor_id: &str, head_txid: u64) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&meta_head_key(&actor_id),
				&encode_db_head(DBHead {
					head_txid,
					db_size_pages: 512,
					#[cfg(debug_assertions)]
					generation: 0,
				})?,
			);
			tx.informal().set(
				&meta_compact_key(&actor_id),
				&encode_meta_compact(MetaCompact {
					materialized_txid: head_txid,
				})?,
			);
			Ok(())
		}
	})
	.await
}

async fn seed_shard(db: &universaldb::Database, actor_id: &str, shard_id: u32) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&shard_key(&actor_id, shard_id),
				&encoded_blob(1, &[(shard_id * 64 + 1, shard_id as u8)])?,
			);
			Ok(())
		}
	})
	.await
}

async fn seed_namespace_config(
	db: &universaldb::Database,
	namespace_id: Id,
	config: SqliteNamespaceConfig,
) -> Result<()> {
	db.run(move |tx| {
		let config = config.clone();
		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			tx.write(&namespace::keys::sqlite_config_key(namespace_id), config)?;
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

async fn read_pitr(db: &universaldb::Database, actor_id: &str) -> Result<i64> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move { quota::read_pitr(&tx, &actor_id).await }
	})
	.await
}

#[tokio::test]
async fn create_checkpoint_basic() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "checkpoint-basic";
	seed_actor(&db, actor_id, 7).await?;
	seed_shard(&db, actor_id, 0).await?;

	let outcome = create_checkpoint(
		Arc::new(db.clone()),
		actor_id.to_string(),
		7,
		CancellationToken::new(),
		namespace_config(),
	)
	.await?;

	assert!(matches!(outcome, CheckpointOutcome::Created { .. }));
	assert!(
		read_value(&db, checkpoint_shard_key(actor_id, 7, 0))
			.await?
			.is_some()
	);
	let meta = decode_checkpoint_meta(
		&read_value(&db, checkpoint_meta_key(actor_id, 7))
			.await?
			.expect("checkpoint meta should exist"),
	)?;
	assert_eq!(meta.head_txid, 7);
	let checkpoints = decode_checkpoints(
		&read_value(&db, meta_checkpoints_key(actor_id))
			.await?
			.expect("checkpoint index should exist"),
	)?;
	assert_eq!(checkpoints.entries.len(), 1);
	assert_eq!(checkpoints.entries[0].ckp_txid, 7);
	assert!(read_pitr(&db, actor_id).await? > 0);

	Ok(())
}

#[tokio::test]
async fn create_checkpoint_multi_tx() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "checkpoint-many-shards";
	seed_actor(&db, actor_id, 9).await?;
	for shard_id in 0..200 {
		seed_shard(&db, actor_id, shard_id).await?;
	}

	let outcome = create_checkpoint(
		Arc::new(db.clone()),
		actor_id.to_string(),
		9,
		CancellationToken::new(),
		namespace_config(),
	)
	.await?;
	let CheckpointOutcome::Created { tx_count, .. } = outcome else {
		panic!("checkpoint should be created");
	};
	assert!(tx_count > 2);
	for shard_id in 0..200 {
		assert!(
			read_value(&db, checkpoint_shard_key(actor_id, 9, shard_id))
				.await?
				.is_some()
		);
	}

	Ok(())
}

#[tokio::test]
async fn create_checkpoint_skip_at_quota() -> Result<()> {
	let db = test_db().await?;
	let actor_id = "checkpoint-quota";
	seed_actor(&db, actor_id, 3).await?;
	seed_shard(&db, actor_id, 0).await?;
	let mut config = namespace_config();
	config.pitr_max_bytes_per_actor = 1;
	config.pitr_namespace_budget_bytes = 1;
	let counter = sqlite_storage::compactor::metrics::SQLITE_CHECKPOINT_SKIPPED_QUOTA_TOTAL
		.with_label_values(&["unknown"]);
	let before = counter.get();

	let outcome = create_checkpoint(
		Arc::new(db.clone()),
		actor_id.to_string(),
		3,
		CancellationToken::new(),
		config,
	)
	.await?;

	assert_eq!(outcome, CheckpointOutcome::SkippedQuota);
	assert!(
		read_value(&db, checkpoint_meta_key(actor_id, 3))
			.await?
			.is_none()
	);
	assert_eq!(counter.get(), before + 1);

	Ok(())
}

#[tokio::test]
async fn create_checkpoint_disabled_by_flag() -> Result<()> {
	let _lock = CHECKPOINT_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let actor_id = "checkpoint-disabled";
	let namespace_id = Id::new_v1(80);
	seed_actor(&db, actor_id, 1).await?;
	seed_shard(&db, actor_id, 0).await?;
	seed_namespace_config(&db, namespace_id, namespace_config()).await?;

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		sqlite_storage::compactor::SqliteCompactPayload {
			actor_id: actor_id.to_string(),
			namespace_id: Some(namespace_id),
			actor_name: Some(actor_id.to_string()),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		CompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	assert!(
		read_value(&db, checkpoint_meta_key(actor_id, 1))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn create_checkpoint_uses_plan_time_txid() -> Result<()> {
	let _lock = CHECKPOINT_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let actor_id = "checkpoint-plan-time";
	let namespace_id = Id::new_v1(81);
	seed_actor(&db, actor_id, 1).await?;
	seed_shard(&db, actor_id, 0).await?;
	seed_namespace_config(&db, namespace_id, namespace_config()).await?;
	let (_guard, reached, release) = sqlite_storage::compactor::compact::test_hooks::pause_after_plan(actor_id);
	let task = tokio::spawn(worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		sqlite_storage::compactor::SqliteCompactPayload {
			actor_id: actor_id.to_string(),
			namespace_id: Some(namespace_id),
			actor_name: Some(actor_id.to_string()),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		pitr_config(),
		CancellationToken::new(),
	));

	reached.notified().await;
	db.run({
		let actor_id = actor_id.to_string();
		move |tx| {
			let actor_id = actor_id.clone();
			async move {
				tx.informal().set(
					&delta_chunk_key(&actor_id, 2, 0),
					&encoded_blob(2, &[(2, 0x22)])?,
				);
				tx.informal()
					.set(&pidx_delta_key(&actor_id, 2), &2_u64.to_be_bytes());
				tx.informal().set(
					&meta_head_key(&actor_id),
					&encode_db_head(DBHead {
						head_txid: 2,
						db_size_pages: 512,
						#[cfg(debug_assertions)]
						generation: 0,
					})?,
				);
				Ok(())
			}
		}
	})
	.await?;
	release.notify_waiters();
	task.await??;

	let meta = decode_checkpoint_meta(
		&read_value(&db, checkpoint_meta_key(actor_id, 1))
			.await?
			.expect("plan-time checkpoint should exist"),
	)?;
	assert_eq!(meta.head_txid, 1);
	assert!(
		read_value(&db, checkpoint_meta_key(actor_id, 2))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, checkpoint_pidx_delta_key(actor_id, 1, 2))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn checkpoint_creation_cancellable() -> Result<()> {
	let _lock = CHECKPOINT_TEST_LOCK.lock().await;
	let db = test_db().await?;
	let actor_id = "checkpoint-cancel";
	seed_actor(&db, actor_id, 5).await?;
	seed_shard(&db, actor_id, 0).await?;
	seed_shard(&db, actor_id, 1).await?;
	let cancel_token = CancellationToken::new();
	let (_guard, reached, release) = checkpoint::test_hooks::pause_after_copy_tx(actor_id);
	let task = tokio::spawn(create_checkpoint(
		Arc::new(db.clone()),
		actor_id.to_string(),
		5,
		cancel_token.clone(),
		namespace_config(),
	));

	reached.notified().await;
	cancel_token.cancel();
	release.notify_waiters();
	let err = task.await?.expect_err("checkpoint should cancel");
	assert!(err.to_string().contains("cancelled"));
	assert!(
		read_value(&db, checkpoint_meta_key(actor_id, 5))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, checkpoint_shard_key(actor_id, 5, 0))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn create_checkpoint_concurrent_semaphore() -> Result<()> {
	let _lock = CHECKPOINT_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let namespace_id = Id::new_v1(82);
	seed_namespace_config(&db, namespace_id, namespace_config()).await?;
	for idx in 0..32 {
		let actor_id = format!("checkpoint-concurrent-{idx}");
		seed_actor(&db, &actor_id, 1).await?;
		seed_shard(&db, &actor_id, 0).await?;
	}
	let semaphore = Arc::new(Semaphore::new(16));
	let (_guard, max_seen, reached, release) = checkpoint::test_hooks::pause_inside_checkpoint();
	let mut tasks = Vec::new();
	for idx in 0..32 {
		let actor_id = format!("checkpoint-concurrent-{idx}");
		tasks.push(tokio::spawn(worker::test_hooks::handle_payload_once_with_checkpoint_semaphore(
			Arc::clone(&db),
			sqlite_storage::compactor::SqliteCompactPayload {
				actor_id,
				namespace_id: Some(namespace_id),
				actor_name: None,
				commit_bytes_since_rollup: 0,
				read_bytes_since_rollup: 0,
			},
			pitr_config(),
			CancellationToken::new(),
			Arc::clone(&semaphore),
		)));
	}

	tokio::time::timeout(Duration::from_secs(2), async {
		loop {
			if max_seen.load(Ordering::SeqCst) >= 16 {
				break;
			}
			reached.notified().await;
		}
	})
	.await
	.expect("sixteen checkpoints should enter");
	assert_eq!(max_seen.load(Ordering::SeqCst), 16);
	assert_eq!(semaphore.available_permits(), 0);
	release.notify_waiters();
	for task in tasks {
		task.await??;
	}

	Ok(())
}
