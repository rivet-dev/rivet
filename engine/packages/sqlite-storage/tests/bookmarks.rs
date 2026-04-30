use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	cold_tier::{ColdTier, FilesystemColdTier},
	compactor::{
		SqliteColdCompactSubject, compact_default_batch,
		cold::{ColdCompactorConfig, worker},
		decode_cold_compact_payload,
	},
	constants::HOT_RETENTION_FLOOR_MS,
	error::SqliteStorageError,
	keys::{
		branch_commit_key, branch_manifest_last_hot_pass_txid_key, branch_meta_compact_key,
		branch_shard_key, branch_vtx_key,
	},
	pump::{ActorDb, bookmark, branch},
	types::{
		ActorBranchId, BookmarkRef, CommitRow, DirtyPage, NamespaceId, PinStatus,
		decode_commit_row, decode_pinned_bookmark_record, encode_meta_compact,
	},
};
use tempfile::Builder;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};

const TEST_ACTOR: &str = "bookmark-source";
const OTHER_ACTOR: &str = "bookmark-other";

static COMPACTION_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn test_db() -> Result<Arc<universaldb::Database>> {
	let path = Builder::new().prefix("sqlite-storage-bookmarks-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-bookmarks-test".to_string(),
	)))
}

fn namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn other_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

fn nil_namespace() -> Id {
	Id::v1(uuid::Uuid::nil(), 1)
}

fn actor_db(db: Arc<universaldb::Database>, namespace_id: Id, actor_id: &str) -> ActorDb {
	ActorDb::new(db, test_ups(), namespace_id, actor_id.to_string(), NodeId::new())
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; sqlite_storage::keys::PAGE_SIZE as usize],
	}
}

fn now_ms() -> i64 {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.expect("system clock should be after unix epoch");
	i64::try_from(elapsed.as_millis()).expect("timestamp should fit i64")
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

async fn actor_branch_id(
	db: &universaldb::Database,
	namespace_id: Id,
	actor_id: &str,
) -> Result<ActorBranchId> {
	let namespace_id = NamespaceId::from_gas_id(namespace_id);

	db.run(move |tx| async move {
		branch::resolve_actor_branch(&tx, namespace_id, actor_id, Serializable)
			.await?
			.context("actor branch should exist")
	})
	.await
}

async fn commit_row(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<CommitRow> {
	let bytes = read_value(db, branch_commit_key(branch_id, txid))
		.await?
		.context("commit row should exist")?;

	decode_commit_row(&bytes)
}

fn assert_storage_error(err: anyhow::Error, expected: SqliteStorageError) {
	assert!(
		err.chain().any(|cause| {
			cause
				.downcast_ref::<SqliteStorageError>()
				.is_some_and(|err| err == &expected)
		}),
		"expected {expected:?}, got {err:?}",
	);
}

#[tokio::test]
async fn ephemeral_bookmark_resolves_to_commit_versionstamp() -> Result<()> {
	let db = test_db().await?;
	let actor_db = actor_db(Arc::clone(&db), namespace(), TEST_ACTOR);

	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = actor_db.create_bookmark(1_010).await?;
	let branch_id = actor_branch_id(&db, namespace(), TEST_ACTOR).await?;
	let row = commit_row(&db, branch_id, 1).await?;

	let resolved = actor_db.resolve_bookmark(bookmark.clone()).await?;

	assert_eq!(resolved.versionstamp, row.versionstamp);
	assert_eq!(
		resolved.bookmark,
		Some(BookmarkRef {
			bookmark,
			resolved_versionstamp: Some(row.versionstamp),
		})
	);

	Ok(())
}

#[tokio::test]
async fn pinned_bookmark_reaches_ready_through_filesystem_cold_tier() -> Result<()> {
	let db = test_db().await?;
	let ups = test_ups();
	let mut sub = ups
		.queue_subscribe(SqliteColdCompactSubject, "cold-compactor")
		.await?;
	let actor_db = ActorDb::new(
		Arc::clone(&db),
		ups,
		namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let branch_id = actor_branch_id(&db, namespace(), TEST_ACTOR).await?;

	db.run(move |tx| async move {
		tx.informal().set(
			&branch_meta_compact_key(branch_id),
			&encode_meta_compact(sqlite_storage::types::MetaCompact {
				materialized_txid: 1,
			})?,
		);
		tx.informal()
			.set(&branch_manifest_last_hot_pass_txid_key(branch_id), &1u64.to_be_bytes());
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 1), b"image-one");
		Ok(())
	})
	.await?;

	let bookmark = actor_db.create_pinned_bookmark(1_010).await?;
	assert_eq!(actor_db.bookmark_status(bookmark.clone()).await?, Some(PinStatus::Pending));

	let msg = timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("cold trigger should publish")?;
	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_cold_compact_payload(&msg.payload)?;
	let cold_root = Builder::new().prefix("sqlite-storage-bookmark-cold-").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload,
		ColdCompactorConfig::default(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await?;

	let pinned_bytes = read_value(
		&db,
		sqlite_storage::keys::bookmark_pinned_key(TEST_ACTOR, bookmark.as_str()),
	)
	.await?
	.context("pinned bookmark record should exist")?;
	let pinned = decode_pinned_bookmark_record(&pinned_bytes)?;
	assert_eq!(pinned.status, PinStatus::Ready);
	assert_eq!(actor_db.bookmark_status(bookmark).await?, Some(PinStatus::Ready));
	assert!(pinned.pin_object_key.is_some());
	assert!(tier.get_object(pinned.pin_object_key.as_deref().unwrap()).await?.is_some());

	Ok(())
}

#[tokio::test]
async fn parent_namespace_bookmark_resolves_from_forked_namespace() -> Result<()> {
	let db = test_db().await?;
	let source = actor_db(Arc::clone(&db), namespace(), TEST_ACTOR);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = source.create_bookmark(1_010).await?;
	let source_branch = actor_branch_id(&db, namespace(), TEST_ACTOR).await?;
	let fork_point = commit_row(&db, source_branch, 1).await?;
	let forked_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		NamespaceId::from_gas_id(namespace()),
		sqlite_storage::types::ResolvedVersionstamp {
			versionstamp: fork_point.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	let resolved =
		bookmark::resolve_bookmark(&db, forked_namespace, TEST_ACTOR.to_string(), bookmark).await?;

	assert_eq!(resolved.versionstamp, fork_point.versionstamp);

	source.commit(vec![page(2, 0x22)], 3, 2_000).await?;
	let post_fork_bookmark = source.create_bookmark(2_010).await?;
	let err = bookmark::resolve_bookmark(
		&db,
		forked_namespace,
		TEST_ACTOR.to_string(),
		post_fork_bookmark,
	)
	.await
	.expect_err("post-fork source bookmark should not be visible in forked namespace");
	assert_storage_error(err, SqliteStorageError::BranchNotReachable);

	Ok(())
}

#[tokio::test]
async fn bookmark_past_hot_retention_returns_expired() -> Result<()> {
	let _compaction_test_lock = COMPACTION_TEST_LOCK.lock().await;
	let db = test_db().await?;
	let actor_db = actor_db(Arc::clone(&db), nil_namespace(), TEST_ACTOR);
	let current_ms = now_ms();
	let old_ms = current_ms - HOT_RETENTION_FLOOR_MS - 1_000;
	let recent_ms = current_ms - 1_000;

	actor_db.commit(vec![page(1, 0x11)], 2, old_ms).await?;
	let old_bookmark = actor_db.create_bookmark(old_ms).await?;
	actor_db.commit(vec![page(2, 0x22)], 3, recent_ms).await?;
	let branch_id = actor_branch_id(&db, nil_namespace(), TEST_ACTOR).await?;
	let old_row = commit_row(&db, branch_id, 1).await?;

	compact_default_batch(
		Arc::clone(&db),
		TEST_ACTOR.to_string(),
		10,
		CancellationToken::new(),
	)
	.await?;

	assert!(
		read_value(&db, branch_commit_key(branch_id, 1))
			.await?
			.is_none()
	);
	assert!(
		read_value(&db, branch_vtx_key(branch_id, old_row.versionstamp))
			.await?
			.is_none()
	);

	let err = actor_db
		.resolve_bookmark(old_bookmark)
		.await
		.expect_err("swept bookmark should be expired");
	assert_storage_error(err, SqliteStorageError::BookmarkExpired);

	Ok(())
}

#[tokio::test]
async fn unrelated_actor_bookmark_returns_branch_not_reachable() -> Result<()> {
	let db = test_db().await?;
	let source = actor_db(Arc::clone(&db), namespace(), TEST_ACTOR);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = source.create_bookmark(1_010).await?;
	let other = actor_db(Arc::clone(&db), other_namespace(), OTHER_ACTOR);
	other.commit(vec![page(1, 0x22)], 2, 1_000).await?;

	let err = bookmark::resolve_bookmark(
		&db,
		NamespaceId::from_gas_id(namespace()),
		OTHER_ACTOR.to_string(),
		bookmark,
	)
	.await
	.expect_err("actor from another namespace should not be reachable here");

	assert_storage_error(err, SqliteStorageError::BranchNotReachable);

	Ok(())
}
