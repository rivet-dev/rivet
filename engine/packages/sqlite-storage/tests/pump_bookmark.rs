use std::{sync::Arc, time::Duration};

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{SqliteColdCompactPayload, SqliteColdCompactSubject, decode_cold_compact_payload},
	error::SqliteStorageError,
	keys::{
		bookmark_key, bookmark_pinned_key, branch_commit_key, branch_vtx_key, branches_bk_pin_key,
		namespace_branches_pin_count_key,
	},
	pump::{ActorDb, bookmark, branch},
	types::{
		ActorBranchId, BookmarkRef, BookmarkStr, CommitRow, DirtyPage, NamespaceId, PinStatus,
		PinnedBookmarkRecord, ResolvedVersionstamp, decode_commit_row,
		decode_pinned_bookmark_record, encode_pinned_bookmark_record,
	},
};
use tempfile::Builder;
use tokio::time::timeout;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};

const TEST_ACTOR: &str = "test-actor";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-bookmark-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-pump-bookmark-test".to_string(),
	)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; sqlite_storage::keys::PAGE_SIZE as usize],
	}
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

async fn clear_value(db: &universaldb::Database, key: Vec<u8>) -> Result<()> {
	db.run(move |tx| {
		let key = key.clone();

		async move {
			tx.informal().clear(&key);
			Ok(())
		}
	})
	.await
}

async fn actor_branch_id(
	db: &universaldb::Database,
	namespace_id: NamespaceId,
	actor_id: &str,
) -> Result<ActorBranchId> {
	db.run(move |tx| async move {
		branch::resolve_actor_branch(&tx, namespace_id, actor_id, Serializable)
			.await?
			.ok_or_else(|| anyhow::anyhow!("actor branch should exist"))
	})
	.await
}

async fn namespace_branch_id(
	db: &universaldb::Database,
	namespace_id: NamespaceId,
) -> Result<sqlite_storage::types::NamespaceBranchId> {
	db.run(move |tx| async move {
		branch::resolve_namespace_branch(&tx, namespace_id, Serializable)
			.await?
			.ok_or_else(|| anyhow::anyhow!("namespace branch should exist"))
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
		.expect("commit row should exist");
	decode_commit_row(&bytes)
}

fn assert_sqlite_error(err: anyhow::Error, expected: SqliteStorageError) {
	let actual = err
		.downcast_ref::<SqliteStorageError>()
		.expect("error should be a SqliteStorageError");
	assert_eq!(actual, &expected);
}

fn decode_i64_counter(bytes: &[u8]) -> i64 {
	i64::from_le_bytes(bytes.try_into().expect("counter should be 8 bytes"))
}

#[test]
fn bookmark_format_is_fixed_width_hex() {
	let bookmark = BookmarkStr::format(1_700_000_000_000, 42).expect("bookmark should format");

	assert_eq!(bookmark.as_str(), "0000018bcfe56800-000000000000002a");
	assert_eq!(
		bookmark.parse().expect("bookmark should parse"),
		(1_700_000_000_000, 42)
	);
}

#[test]
fn bookmark_new_rejects_malformed_wire_strings() {
	let cases = [
		"",
		"0000018bcfe56800",
		"0000018bcfe56800_000000000000002a",
		"0000018bcfe5680-000000000000002a",
		"0000018bcfe56800-00000000000002ag",
		"0000018bcfe56800-000000000000002a00",
		"0000018bcfe56800-00000000000002🙂",
	];

	for case in cases {
		assert!(BookmarkStr::new(case).is_err(), "{case} should be rejected");
	}
}

#[test]
fn bookmark_format_rejects_negative_timestamps() {
	assert!(BookmarkStr::format(-1, 0).is_err());
}

#[test]
fn bookmark_round_trip_property_for_representative_values() {
	let timestamps = [
		0,
		1,
		999,
		1_700_000_000_000,
		i64::MAX / 2,
		i64::MAX,
	];
	let txids = [0, 1, 42, u32::MAX as u64, u64::MAX - 1, u64::MAX];

	for ts_ms in timestamps {
		for txid in txids {
			let bookmark = BookmarkStr::format(ts_ms, txid).expect("bookmark should format");
			assert_eq!(bookmark.as_str().len(), 33);
			assert_eq!(
				bookmark.parse().expect("bookmark should parse"),
				(ts_ms, txid)
			);
		}
	}
}

#[test]
fn bookmark_lex_order_matches_chronological_order_for_one_branch() {
	let mut bookmarks = vec![
		BookmarkStr::format(10, 5).expect("bookmark should format"),
		BookmarkStr::format(9, u64::MAX).expect("bookmark should format"),
		BookmarkStr::format(10, 4).expect("bookmark should format"),
		BookmarkStr::format(11, 0).expect("bookmark should format"),
	];

	bookmarks.sort();

	let parsed = bookmarks
		.into_iter()
		.map(|bookmark| bookmark.parse().expect("bookmark should parse"))
		.collect::<Vec<_>>();

	assert_eq!(parsed, vec![(9, u64::MAX), (10, 4), (10, 5), (11, 0)]);
}

#[tokio::test]
async fn create_bookmark_returns_ephemeral_bookmark_for_latest_commit() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);

	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = actor_db.create_bookmark(1_000).await?;

	assert_eq!(bookmark.as_str().len(), 33);
	assert_eq!(bookmark.parse()?, (1_000, 1));
	assert_eq!(
		read_value(&db, bookmark_key(TEST_ACTOR, bookmark.as_str())).await?,
		None
	);

	Ok(())
}

#[tokio::test]
async fn bookmark_status_reads_pinned_record_or_absent() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = actor_db.create_bookmark(1_000).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());

	assert_eq!(actor_db.bookmark_status(bookmark.clone()).await?, None);

	let actor_branch_id = db
		.run({
			let bookmark = bookmark.clone();

			move |tx| {
				let bookmark = bookmark.clone();

				async move {
					let branch_id =
						branch::resolve_actor_branch(&tx, namespace_id, TEST_ACTOR, Serializable)
							.await?
							.expect("actor branch should exist");
					let pinned_key =
						sqlite_storage::keys::bookmark_pinned_key(TEST_ACTOR, bookmark.as_str());
					let record = PinnedBookmarkRecord {
						bookmark,
						actor_branch_id: branch_id,
						versionstamp: [9; 16],
						status: PinStatus::Ready,
						pin_object_key: Some("pin/ready.ltx".to_string()),
						created_at_ms: 1_000,
						updated_at_ms: 1_100,
					};
					let encoded = encode_pinned_bookmark_record(record)?;
					tx.informal().set(&pinned_key, &encoded);

					Ok(branch_id)
				}
			}
		})
		.await?;

	assert_ne!(actor_branch_id.as_uuid(), uuid::Uuid::nil());
	assert_eq!(actor_db.bookmark_status(bookmark.clone()).await?, Some(PinStatus::Ready));
	assert_eq!(
		bookmark::bookmark_status(&db, namespace_id, TEST_ACTOR.to_string(), bookmark).await?,
		Some(PinStatus::Ready)
	);

	Ok(())
}

#[tokio::test]
async fn create_pinned_bookmark_writes_pending_pin_and_cold_trigger() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let ups = test_ups();
	let mut sub = ups
		.queue_subscribe(SqliteColdCompactSubject, "cold-compactor")
		.await?;
	let actor_db = ActorDb::new(
		db.clone(),
		ups,
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let namespace_branch_id = namespace_branch_id(&db, namespace_id).await?;
	let row = commit_row(&db, branch_id, 1).await?;

	let bookmark = actor_db.create_pinned_bookmark(1_010).await?;

	assert_eq!(bookmark.parse()?, (1_010, 1));
	let pinned_bytes = read_value(
		&db,
		sqlite_storage::keys::bookmark_pinned_key(TEST_ACTOR, bookmark.as_str()),
	)
	.await?
	.expect("pinned bookmark record should exist");
	let pinned = decode_pinned_bookmark_record(&pinned_bytes)?;
	assert_eq!(pinned.bookmark, bookmark);
	assert_eq!(pinned.actor_branch_id, branch_id);
	assert_eq!(pinned.versionstamp, row.versionstamp);
	assert_eq!(pinned.status, PinStatus::Pending);
	assert_eq!(pinned.pin_object_key, None);
	assert_eq!(
		read_value(&db, branches_bk_pin_key(branch_id))
			.await?
			.expect("branch bk_pin should be written"),
		row.versionstamp
	);
	let pin_count = read_value(&db, namespace_branches_pin_count_key(namespace_branch_id))
		.await?
		.expect("namespace pin count should be incremented");
	assert_eq!(decode_i64_counter(&pin_count), 1);

	let msg = timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("cold trigger should publish")?;
	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_cold_compact_payload(&msg.payload)?;
	assert_eq!(
		payload,
		SqliteColdCompactPayload::CreatePinnedBookmark {
			actor_id: TEST_ACTOR.to_string(),
			actor_branch_id: branch_id,
			bookmark,
			versionstamp: row.versionstamp,
		}
	);

	Ok(())
}

#[tokio::test]
async fn create_pinned_bookmark_enforces_namespace_pin_cap() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let namespace_branch_id = namespace_branch_id(&db, namespace_id).await?;
	db.run(move |tx| async move {
		tx.informal().set(
			&namespace_branches_pin_count_key(namespace_branch_id),
			&i64::from(sqlite_storage::constants::MAX_PINS_PER_NAMESPACE).to_le_bytes(),
		);
		Ok(())
	})
	.await?;

	let err = actor_db
		.create_pinned_bookmark(1_010)
		.await
		.expect_err("pin cap should reject new pinned bookmarks");

	assert_sqlite_error(err, SqliteStorageError::TooManyPins);

	Ok(())
}

#[tokio::test]
async fn delete_pinned_bookmark_removes_pin_and_schedules_recompute() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let ups = test_ups();
	let mut sub = ups
		.queue_subscribe(SqliteColdCompactSubject, "cold-compactor")
		.await?;
	let actor_db = ActorDb::new(
		db.clone(),
		ups,
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let first = actor_db.create_pinned_bookmark(1_010).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let namespace_branch_id = namespace_branch_id(&db, namespace_id).await?;
	let first_row = commit_row(&db, branch_id, 1).await?;
	let NextOutput::Message(_) = timeout(Duration::from_secs(1), sub.next()).await?? else {
		panic!("subscriber unexpectedly unsubscribed");
	};

	actor_db.commit(vec![page(2, 0x22)], 3, 1_020).await?;
	let second = actor_db.create_pinned_bookmark(1_030).await?;
	let second_row = commit_row(&db, branch_id, 2).await?;
	let NextOutput::Message(_) = timeout(Duration::from_secs(1), sub.next()).await?? else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	assert_eq!(
		read_value(&db, branches_bk_pin_key(branch_id))
			.await?
			.expect("branch bk_pin should be the oldest pin"),
		first_row.versionstamp
	);

	actor_db.delete_pinned_bookmark(first.clone()).await?;

	assert_eq!(
		read_value(&db, bookmark_key(TEST_ACTOR, first.as_str())).await?,
		None
	);
	assert_eq!(
		read_value(&db, bookmark_pinned_key(TEST_ACTOR, first.as_str())).await?,
		None
	);
	assert!(
		read_value(&db, bookmark_pinned_key(TEST_ACTOR, second.as_str()))
			.await?
			.is_some()
	);
	assert_eq!(
		read_value(&db, branches_bk_pin_key(branch_id))
			.await?
			.expect("branch bk_pin should advance to the next remaining pin"),
		second_row.versionstamp
	);
	let pin_count = read_value(&db, namespace_branches_pin_count_key(namespace_branch_id))
		.await?
		.expect("namespace pin count should remain present");
	assert_eq!(decode_i64_counter(&pin_count), 1);

	let msg = timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("cold trigger should publish")?;
	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_cold_compact_payload(&msg.payload)?;
	assert_eq!(
		payload,
		SqliteColdCompactPayload::DeletePinnedBookmark {
			actor_id: TEST_ACTOR.to_string(),
			actor_branch_id: branch_id,
			bookmark: first,
			versionstamp: first_row.versionstamp,
			pin_object_key: None,
		}
	);

	Ok(())
}

#[tokio::test]
async fn resolve_bookmark_returns_ephemeral_commit_versionstamp_via_vtx() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = actor_db.create_bookmark(1_010).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let row = commit_row(&db, branch_id, 1).await?;

	let resolved = actor_db.resolve_bookmark(bookmark.clone()).await?;

	assert_eq!(resolved.versionstamp, row.versionstamp);
	assert_eq!(
		resolved.bookmark,
		Some(BookmarkRef {
			bookmark: bookmark.clone(),
			resolved_versionstamp: Some(row.versionstamp),
		})
	);

	clear_value(&db, branch_vtx_key(branch_id, row.versionstamp)).await?;
	let err = actor_db
		.resolve_bookmark(bookmark)
		.await
		.expect_err("missing VTX should make the bookmark expired");
	assert_sqlite_error(err, SqliteStorageError::BookmarkExpired);

	Ok(())
}

#[tokio::test]
async fn resolve_bookmark_prefers_exact_pinned_record() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let bookmark = actor_db.create_bookmark(1_010).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let pinned_versionstamp = [9; 16];
	db.run({
		let bookmark = bookmark.clone();

		move |tx| {
			let bookmark = bookmark.clone();

			async move {
				let record = PinnedBookmarkRecord {
					bookmark: bookmark.clone(),
					actor_branch_id: branch_id,
					versionstamp: pinned_versionstamp,
					status: PinStatus::Ready,
					pin_object_key: Some("pin/exact.ltx".to_string()),
					created_at_ms: 1_000,
					updated_at_ms: 1_100,
				};
				tx.informal().set(
					&sqlite_storage::keys::bookmark_pinned_key(TEST_ACTOR, bookmark.as_str()),
					&encode_pinned_bookmark_record(record)?,
				);
				Ok(())
			}
		}
	})
	.await?;

	let resolved = actor_db.resolve_bookmark(bookmark.clone()).await?;

	assert_eq!(resolved.versionstamp, pinned_versionstamp);
	assert_eq!(
		resolved.bookmark,
		Some(BookmarkRef {
			bookmark,
			resolved_versionstamp: Some(pinned_versionstamp),
		})
	);

	Ok(())
}

#[tokio::test]
async fn resolve_bookmark_walks_actor_parent_chain() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let source_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	source_actor_db
		.commit(vec![page(1, 0x11)], 2, 1_000)
		.await?;
	let bookmark = source_actor_db.create_bookmark(1_010).await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let source_branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let source_commit = commit_row(&db, source_branch_id, 1).await?;
	let forked_actor_id = branch::fork_actor(
		&db,
		namespace_id,
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
		namespace_id,
	)
	.await?;

	let resolved =
		bookmark::resolve_bookmark(&db, namespace_id, forked_actor_id, bookmark).await?;

	assert_eq!(resolved.versionstamp, source_commit.versionstamp);

	Ok(())
}

#[tokio::test]
async fn resolve_bookmark_honors_namespace_fork_versionstamp_cap() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let source_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	source_actor_db
		.commit(vec![page(1, 0x11)], 2, 1_000)
		.await?;
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let source_branch_id = actor_branch_id(&db, namespace_id, TEST_ACTOR).await?;
	let fork_point = commit_row(&db, source_branch_id, 1).await?;
	let forked_namespace = branch::fork_namespace(
		&db,
		namespace_id,
		ResolvedVersionstamp {
			versionstamp: fork_point.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	source_actor_db
		.commit(vec![page(2, 0x22)], 3, 2_000)
		.await?;
	let post_fork_bookmark = source_actor_db.create_bookmark(2_010).await?;

	let err = bookmark::resolve_bookmark(
		&db,
		forked_namespace,
		TEST_ACTOR.to_string(),
		post_fork_bookmark,
	)
	.await
	.expect_err("namespace fork should not resolve source commits created after the fork");
	assert_sqlite_error(err, SqliteStorageError::BranchNotReachable);

	Ok(())
}
