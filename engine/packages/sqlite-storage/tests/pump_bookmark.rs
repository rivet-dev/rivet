use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	keys::bookmark_key,
	pump::{ActorDb, bookmark, branch},
	types::{
		BookmarkStr, DirtyPage, NamespaceId, PinStatus, PinnedBookmarkRecord,
		encode_pinned_bookmark_record,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

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
