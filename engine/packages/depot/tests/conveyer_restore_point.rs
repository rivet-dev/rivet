mod common;

use std::time::Duration;

use anyhow::Result;
use depot::{
	conveyer::{branch, history_pin, restore_point},
	error::SqliteStorageError,
	keys::{
		branch_commit_key, branch_meta_head_at_fork_key, branch_pidx_key, branch_pitr_interval_key,
		branch_vtx_key, branches_restore_point_pin_key, bucket_branches_pin_count_key, db_pin_key,
		restore_point_key,
	},
	pitr_interval::write_pitr_interval_coverage,
	types::{
		BucketId, CommitRow, DatabaseBranchId, DbHistoryPinKind, DirtyPage, PinStatus,
		PitrIntervalCoverage, ResolvedRestoreTarget, ResolvedVersionstamp, RestorePointId,
		RestorePointRecord, RestorePointRef, SnapshotKind, SnapshotSelector, decode_commit_row,
		decode_db_head, decode_db_history_pin, decode_restore_point_record,
		encode_restore_point_record,
	},
};
use gas::prelude::Id;
use universaldb::utils::IsolationLevel::Serializable;

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
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

async fn database_branch_id(
	db: &universaldb::Database,
	bucket_id: BucketId,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	db.run(move |tx| async move {
		branch::resolve_database_branch(&tx, bucket_id, database_id, Serializable)
			.await?
			.ok_or_else(|| anyhow::anyhow!("database branch should exist"))
	})
	.await
}

async fn bucket_branch_id(
	db: &universaldb::Database,
	bucket_id: BucketId,
) -> Result<depot::types::BucketBranchId> {
	db.run(move |tx| async move {
		branch::resolve_bucket_branch(&tx, bucket_id, Serializable)
			.await?
			.ok_or_else(|| anyhow::anyhow!("bucket branch should exist"))
	})
	.await
}

async fn commit_row(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<CommitRow> {
	let bytes = common::read_value(db, branch_commit_key(branch_id, txid))
		.await?
		.expect("commit row should exist");
	decode_commit_row(&bytes)
}

async fn seed_pitr_interval(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
	coverage: PitrIntervalCoverage,
) -> Result<()> {
	db.run(move |tx| {
		let coverage = coverage.clone();

		async move {
			write_pitr_interval_coverage(&tx, branch_id, bucket_start_ms, coverage)?;
			Ok(())
		}
	})
	.await
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

macro_rules! restore_matrix {
	($prefix:expr, |$ctx:ident, $db:ident, $database_db:ident| $body:block) => {
		common::test_matrix($prefix, |_tier, $ctx| {
			Box::pin(async move {
				#[allow(unused_variables)]
				let $db = $ctx.udb.clone();
				let $database_db = $ctx.make_db(test_bucket(), TEST_DATABASE);
				$body
			})
		})
		.await
	};
}

#[test]
fn restore_point_format_is_fixed_width_hex() {
	let restore_point =
		RestorePointId::format(1_700_000_000_000, 42).expect("restore_point should format");

	assert_eq!(restore_point.as_str(), "0000018bcfe56800-000000000000002a");
	assert_eq!(
		restore_point.parse().expect("restore_point should parse"),
		(1_700_000_000_000, 42)
	);
}

#[test]
fn restore_point_new_rejects_malformed_wire_strings() {
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
		assert!(
			RestorePointId::new(case).is_err(),
			"{case} should be rejected"
		);
	}
}

#[test]
fn restore_point_format_rejects_negative_timestamps() {
	assert!(RestorePointId::format(-1, 0).is_err());
}

#[test]
fn restore_point_round_trip_property_for_representative_values() {
	let timestamps = [0, 1, 999, 1_700_000_000_000, i64::MAX / 2, i64::MAX];
	let txids = [0, 1, 42, u32::MAX as u64, u64::MAX - 1, u64::MAX];

	for ts_ms in timestamps {
		for txid in txids {
			let restore_point =
				RestorePointId::format(ts_ms, txid).expect("restore_point should format");
			assert_eq!(restore_point.as_str().len(), 33);
			assert_eq!(
				restore_point.parse().expect("restore_point should parse"),
				(ts_ms, txid)
			);
		}
	}
}

#[test]
fn restore_point_lex_order_matches_chronological_order_for_one_branch() {
	let mut restore_points = vec![
		RestorePointId::format(10, 5).expect("restore_point should format"),
		RestorePointId::format(9, u64::MAX).expect("restore_point should format"),
		RestorePointId::format(10, 4).expect("restore_point should format"),
		RestorePointId::format(11, 0).expect("restore_point should format"),
	];

	restore_points.sort();

	let parsed = restore_points
		.into_iter()
		.map(|restore_point| restore_point.parse().expect("restore_point should parse"))
		.collect::<Vec<_>>();

	assert_eq!(parsed, vec![(9, u64::MAX), (10, 4), (10, 5), (11, 0)]);
}

#[tokio::test]
async fn create_restore_point_returns_retained_restore_point_for_latest_commit() -> Result<()> {
	restore_matrix!(
		"create_restore_point_returns_retained_restore_point_for_latest_commit",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;

			assert_eq!(restore_point.as_str().len(), 33);
			assert_eq!(restore_point.parse()?, (1_000, 1));
			assert!(
				common::read_value(
					&db,
					restore_point_key(TEST_DATABASE, restore_point.as_str())
				)
				.await?
				.is_some()
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn create_restore_point_from_timestamp_selector_pins_selected_interval() -> Result<()> {
	restore_matrix!(
		"create_restore_point_from_timestamp_selector_pins_selected_interval",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			database_db.commit(vec![page(1, 0x22)], 2, 2_500).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let first = commit_row(&db, branch_id, 1).await?;
			seed_pitr_interval(
				&db,
				branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: first.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;

			let restore_point = database_db
				.create_restore_point(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				})
				.await?;

			assert_eq!(restore_point.parse()?, (1_000, 1));
			let record_bytes = common::read_value(
				&db,
				restore_point_key(TEST_DATABASE, restore_point.as_str()),
			)
			.await?
			.expect("restore point record should exist");
			let record = decode_restore_point_record(&record_bytes)?;
			assert_eq!(record.database_branch_id, branch_id);
			assert_eq!(record.versionstamp, first.versionstamp);
			let pin_bytes = common::read_value(
				&db,
				db_pin_key(
					branch_id,
					&history_pin::restore_point_pin_id(&restore_point),
				),
			)
			.await?
			.expect("restore point DB_PIN should exist");
			let pin = decode_db_history_pin(&pin_bytes)?;
			assert_eq!(pin.at_txid, 1);
			assert_eq!(pin.at_versionstamp, first.versionstamp);

			Ok(())
		}
	)
}

#[tokio::test]
async fn create_restore_point_revalidates_target_commit_after_resolve_race() -> Result<()> {
	restore_matrix!(
		"create_restore_point_revalidates_target_commit_after_resolve_race",
		|ctx, db, _database_db| {
			let database_id = "test-database-create-race";
			let database_db = ctx.make_db(test_bucket(), database_id);
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, database_id).await?;
			let row = commit_row(&db, branch_id, 1).await?;
			seed_pitr_interval(
				&db,
				branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: row.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;
			let restore_point = RestorePointId::format(1_000, 1)?;
			let (_guard, reached, release) =
				restore_point::test_hooks::pause_after_resolve(database_id);
			let create_task = {
				let db = db.clone();
				let database_id = database_id.to_string();
				tokio::spawn(async move {
					restore_point::create_restore_point(
						&db,
						bucket_id,
						database_id,
						SnapshotSelector::AtTimestamp {
							timestamp_ms: 1_500,
						},
					)
					.await
				})
			};

			tokio::time::timeout(Duration::from_secs(2), reached.notified())
				.await
				.expect("restore point creation should pause after resolving target");
			clear_value(&db, branch_commit_key(branch_id, 1)).await?;
			clear_value(&db, branch_vtx_key(branch_id, row.versionstamp)).await?;
			clear_value(&db, branch_pitr_interval_key(branch_id, 1_000)).await?;
			release.notify_waiters();

			let err = create_task
				.await
				.expect("restore point task should not panic")
				.expect_err("deleted target commit should abort restore point creation");
			assert_sqlite_error(err, SqliteStorageError::RestoreTargetExpired);
			assert!(
				common::read_value(&db, restore_point_key(database_id, restore_point.as_str()))
					.await?
					.is_none()
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_point_status_reads_pinned_record_or_absent() -> Result<()> {
	restore_matrix!(
		"restore_point_status_reads_pinned_record_or_absent",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());

			assert_eq!(
				database_db
					.restore_point_status(RestorePointId::format(1_001, 1)?)
					.await?,
				None
			);

			let database_branch_id = db
				.run({
					let restore_point = restore_point.clone();

					move |tx| {
						let restore_point = restore_point.clone();

						async move {
							let branch_id = branch::resolve_database_branch(
								&tx,
								bucket_id,
								TEST_DATABASE,
								Serializable,
							)
							.await?
							.expect("database branch should exist");
							let pinned_key = depot::keys::restore_point_key(
								TEST_DATABASE,
								restore_point.as_str(),
							);
							let record = RestorePointRecord {
								restore_point_id: restore_point,
								database_branch_id: branch_id,
								versionstamp: [9; 16],
								status: PinStatus::Ready,
								pin_object_key: None,
								created_at_ms: 1_000,
								updated_at_ms: 1_100,
							};
							let encoded = encode_restore_point_record(record)?;
							tx.informal().set(&pinned_key, &encoded);

							Ok(branch_id)
						}
					}
				})
				.await?;

			assert_ne!(database_branch_id.as_uuid(), uuid::Uuid::nil());
			assert_eq!(
				database_db
					.restore_point_status(restore_point.clone())
					.await?,
				Some(PinStatus::Ready)
			);
			assert_eq!(
				restore_point::restore_point_status(
					&db,
					bucket_id,
					TEST_DATABASE.to_string(),
					restore_point
				)
				.await?,
				Some(PinStatus::Ready)
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_point_ready_transition_writes_history_pin_without_object_key() -> Result<()> {
	restore_matrix!(
		"restore_point_ready_transition_writes_history_pin_without_object_key",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let bucket_branch_id = bucket_branch_id(&db, bucket_id).await?;
			let row = commit_row(&db, branch_id, 1).await?;

			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;

			assert_eq!(restore_point.parse()?, (1_000, 1));
			let pinned_bytes = common::read_value(
				&db,
				depot::keys::restore_point_key(TEST_DATABASE, restore_point.as_str()),
			)
			.await?
			.expect("restore point record should exist");
			let pinned = decode_restore_point_record(&pinned_bytes)?;
			assert_eq!(pinned.restore_point_id, restore_point);
			assert_eq!(pinned.database_branch_id, branch_id);
			assert_eq!(pinned.versionstamp, row.versionstamp);
			assert_eq!(pinned.status, PinStatus::Ready);
			assert_eq!(pinned.pin_object_key, None);
			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(branch_id))
					.await?
					.expect("branch restore_point_pin should be written"),
				row.versionstamp
			);
			let db_pin_bytes = common::read_value(
				&db,
				db_pin_key(
					branch_id,
					&history_pin::restore_point_pin_id(&restore_point),
				),
			)
			.await?
			.expect("restore_point DB_PIN should exist");
			let db_pin = decode_db_history_pin(&db_pin_bytes)?;
			assert_eq!(db_pin.kind, DbHistoryPinKind::RestorePoint);
			assert_eq!(db_pin.at_txid, 1);
			assert_eq!(db_pin.at_versionstamp, row.versionstamp);
			assert_eq!(db_pin.owner_restore_point, Some(restore_point.clone()));
			let pin_count =
				common::read_value(&db, bucket_branches_pin_count_key(bucket_branch_id))
					.await?
					.expect("bucket pin count should be incremented");
			assert_eq!(decode_i64_counter(&pin_count), 1);

			Ok(())
		}
	)
}

#[tokio::test]
async fn legacy_failed_restore_point_status_preserves_object_key() -> Result<()> {
	restore_matrix!(
		"legacy_failed_restore_point_status_preserves_object_key",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let row = commit_row(&db, branch_id, 1).await?;
			let restore_point = RestorePointId::format(1_010, 1)?;
			let pin_object_key = "db/legacy/pin/object.ltx".to_string();

			db.run({
				let restore_point = restore_point.clone();
				let pin_object_key = pin_object_key.clone();

				move |tx| {
					let restore_point = restore_point.clone();
					let pin_object_key = pin_object_key.clone();

					async move {
						tx.informal().set(
							&restore_point_key(TEST_DATABASE, restore_point.as_str()),
							&encode_restore_point_record(RestorePointRecord {
								restore_point_id: restore_point,
								database_branch_id: branch_id,
								versionstamp: row.versionstamp,
								status: PinStatus::Failed,
								pin_object_key: Some(pin_object_key),
								created_at_ms: 1_010,
								updated_at_ms: 1_020,
							})?,
						);
						Ok(())
					}
				}
			})
			.await?;

			assert_eq!(
				database_db
					.restore_point_status(restore_point.clone())
					.await?,
				Some(PinStatus::Failed)
			);
			let pinned_bytes = common::read_value(
				&db,
				restore_point_key(TEST_DATABASE, restore_point.as_str()),
			)
			.await?
			.expect("legacy failed restore point record should exist");
			let pinned = decode_restore_point_record(&pinned_bytes)?;
			assert_eq!(pinned.status, PinStatus::Failed);
			assert_eq!(pinned.pin_object_key, Some(pin_object_key));
			assert_eq!(
				common::read_value(
					&db,
					db_pin_key(
						branch_id,
						&history_pin::restore_point_pin_id(&restore_point)
					)
				)
				.await?,
				None
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn create_restore_point_enforces_bucket_pin_cap() -> Result<()> {
	restore_matrix!(
		"create_restore_point_enforces_bucket_pin_cap",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let bucket_branch_id = bucket_branch_id(&db, bucket_id).await?;
			db.run(move |tx| async move {
				tx.informal().set(
					&bucket_branches_pin_count_key(bucket_branch_id),
					&i64::from(depot::constants::MAX_RESTORE_POINTS_PER_BUCKET).to_le_bytes(),
				);
				Ok(())
			})
			.await?;

			let err = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await
				.expect_err("pin cap should reject new restore_points");

			assert_sqlite_error(err, SqliteStorageError::TooManyRestorePoints);

			Ok(())
		}
	)
}

#[tokio::test]
async fn delete_restore_point_removes_pin_and_recomputes_branch_pin() -> Result<()> {
	restore_matrix!(
		"delete_restore_point_removes_pin_and_recomputes_branch_pin",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let first = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let bucket_branch_id = bucket_branch_id(&db, bucket_id).await?;
			let first_row = commit_row(&db, branch_id, 1).await?;

			database_db.commit(vec![page(2, 0x22)], 3, 1_020).await?;
			let second = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let second_row = commit_row(&db, branch_id, 2).await?;
			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(branch_id))
					.await?
					.expect("branch restore_point_pin should be the oldest pin"),
				first_row.versionstamp
			);

			database_db.delete_restore_point(first.clone()).await?;

			assert_eq!(
				common::read_value(&db, restore_point_key(TEST_DATABASE, first.as_str())).await?,
				None
			);
			assert_eq!(
				common::read_value(
					&db,
					db_pin_key(branch_id, &history_pin::restore_point_pin_id(&first))
				)
				.await?,
				None
			);
			assert!(
				common::read_value(&db, restore_point_key(TEST_DATABASE, second.as_str()))
					.await?
					.is_some()
			);
			assert!(
				common::read_value(
					&db,
					db_pin_key(branch_id, &history_pin::restore_point_pin_id(&second))
				)
				.await?
				.is_some()
			);
			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(branch_id))
					.await?
					.expect("branch restore_point_pin should advance to the next remaining pin"),
				second_row.versionstamp
			);
			let pin_count =
				common::read_value(&db, bucket_branches_pin_count_key(bucket_branch_id))
					.await?
					.expect("bucket pin count should remain present");
			assert_eq!(decode_i64_counter(&pin_count), 1);

			Ok(())
		}
	)
}

#[tokio::test]
async fn deleting_last_restore_point_clears_branch_pin_for_later_pin() -> Result<()> {
	restore_matrix!(
		"deleting_last_restore_point_clears_branch_pin_for_later_pin",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let first = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;

			database_db.delete_restore_point(first).await?;

			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(branch_id)).await?,
				None
			);

			database_db.commit(vec![page(2, 0x22)], 3, 1_020).await?;
			let second = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let second_row = commit_row(&db, branch_id, 2).await?;

			assert!(
				common::read_value(&db, restore_point_key(TEST_DATABASE, second.as_str()))
					.await?
					.is_some()
			);
			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(branch_id))
					.await?
					.expect("branch restore_point_pin should initialize to the later pin"),
				second_row.versionstamp
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_database_from_restore_point_rolls_back_then_pins_undo_restore_point() -> Result<()>
{
	restore_matrix!(
		"restore_database_from_restore_point_rolls_back_then_pins_undo_restore_point",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let target = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			database_db
				.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let bucket_branch_id = bucket_branch_id(&db, bucket_id).await?;
			let old_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let target_row = commit_row(&db, old_branch_id, 1).await?;
			let undo_row = commit_row(&db, old_branch_id, 2).await?;

			let undo = database_db
				.restore_database(SnapshotSelector::RestorePoint {
					restore_point: target,
				})
				.await?;

			assert_eq!(undo.parse()?, (2_000, 2));
			let new_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			assert_ne!(new_branch_id, old_branch_id);
			let restored_head_bytes =
				common::read_value(&db, branch_meta_head_at_fork_key(new_branch_id))
					.await?
					.expect("rollback branch should snapshot head_at_fork");
			let restored_head = decode_db_head(&restored_head_bytes)?;
			assert_eq!(restored_head.head_txid, 1);
			assert_eq!(restored_head.db_size_pages, 2);

			let pinned_bytes =
				common::read_value(&db, restore_point_key(TEST_DATABASE, undo.as_str()))
					.await?
					.expect("undo restore point should exist");
			let pinned = decode_restore_point_record(&pinned_bytes)?;
			assert_eq!(pinned.restore_point_id, undo);
			assert_eq!(pinned.database_branch_id, old_branch_id);
			assert_eq!(pinned.versionstamp, undo_row.versionstamp);
			assert_eq!(pinned.status, PinStatus::Ready);
			assert_eq!(
				common::read_value(&db, branches_restore_point_pin_key(old_branch_id))
					.await?
					.expect("old branch restore_point_pin should keep the oldest restore point"),
				target_row.versionstamp
			);
			let pin_count =
				common::read_value(&db, bucket_branches_pin_count_key(bucket_branch_id))
					.await?
					.expect("bucket pin count should be incremented");
			assert_eq!(decode_i64_counter(&pin_count), 2);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_database_from_timestamp_rolls_back_then_pins_undo_restore_point() -> Result<()> {
	restore_matrix!(
		"restore_database_from_timestamp_rolls_back_then_pins_undo_restore_point",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			database_db
				.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let old_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let target_row = commit_row(&db, old_branch_id, 1).await?;
			let undo_row = commit_row(&db, old_branch_id, 2).await?;
			seed_pitr_interval(
				&db,
				old_branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: target_row.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;

			let undo = database_db
				.restore_database(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				})
				.await?;

			assert_eq!(undo.parse()?, (2_000, 2));
			let new_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			assert_ne!(new_branch_id, old_branch_id);
			let restored_head_bytes =
				common::read_value(&db, branch_meta_head_at_fork_key(new_branch_id))
					.await?
					.expect("rollback branch should snapshot head_at_fork");
			let restored_head = decode_db_head(&restored_head_bytes)?;
			assert_eq!(restored_head.head_txid, 1);
			assert_eq!(restored_head.db_size_pages, 2);

			let pinned_bytes =
				common::read_value(&db, restore_point_key(TEST_DATABASE, undo.as_str()))
					.await?
					.expect("undo restore point should exist");
			let pinned = decode_restore_point_record(&pinned_bytes)?;
			assert_eq!(pinned.database_branch_id, old_branch_id);
			assert_eq!(pinned.versionstamp, undo_row.versionstamp);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_database_rollback_and_undo_pin_are_atomic() -> Result<()> {
	restore_matrix!(
		"restore_database_rollback_and_undo_pin_are_atomic",
		|ctx, db, _database_db| {
			let database_id = "test-database-atomic-restore";
			let database_db = ctx.make_db(test_bucket(), database_id);
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let target = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			database_db
				.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let old_branch_id = database_branch_id(&db, bucket_id, database_id).await?;
			let undo = RestorePointId::format(2_000, 2)?;
			let _guard = restore_point::test_hooks::fail_after_restore_rollback(database_id);

			let err = database_db
				.restore_database(SnapshotSelector::RestorePoint {
					restore_point: target,
				})
				.await
				.expect_err("injected failure should abort restore");

			assert!(err.chain().any(|cause| {
				cause
					.to_string()
					.contains("injected failure after sqlite restore rollback")
			}));
			assert_eq!(
				database_branch_id(&db, bucket_id, database_id).await?,
				old_branch_id
			);
			assert!(
				common::read_value(&db, restore_point_key(database_id, undo.as_str()))
					.await?
					.is_none()
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn write_after_pitr_restore_lands_on_restored_branch() -> Result<()> {
	restore_matrix!(
		"write_after_pitr_restore_lands_on_restored_branch",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			database_db
				.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let old_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let target_row = commit_row(&db, old_branch_id, 1).await?;
			seed_pitr_interval(
				&db,
				old_branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: target_row.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;

			database_db
				.restore_database(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				})
				.await?;
			let restored_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			assert_ne!(restored_branch_id, old_branch_id);

			database_db.commit(vec![page(3, 0x44)], 3, 3_000).await?;

			let post_restore_commit = commit_row(&db, restored_branch_id, 2).await?;
			assert_eq!(post_restore_commit.db_size_pages, 3);
			assert_eq!(
				common::read_value(&db, branch_pidx_key(restored_branch_id, 1)).await?,
				None
			);
			assert!(
				common::read_value(&db, branch_pidx_key(old_branch_id, 1))
					.await?
					.is_some()
			);
			let restored_database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
			let pages = restored_database_db.get_pages(vec![1, 2, 3]).await?;
			assert_eq!(pages[0].bytes, Some(page(1, 0x11).bytes));
			assert_eq!(pages[1].bytes, Some(page(2, 0).bytes));
			assert_eq!(pages[2].bytes, Some(page(3, 0x44).bytes));

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_database_rejects_expired_timestamp_without_undo_or_pointer_swap() -> Result<()> {
	restore_matrix!(
		"restore_database_rejects_expired_timestamp_without_undo_or_pointer_swap",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let old_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let undo_id = RestorePointId::format(1_000, 1)?;

			let err = database_db
				.restore_database(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				})
				.await
				.expect_err("expired selector should reject restore");

			assert_sqlite_error(err, SqliteStorageError::RestoreTargetExpired);
			assert_eq!(
				database_branch_id(&db, bucket_id, TEST_DATABASE).await?,
				old_branch_id
			);
			assert!(
				common::read_value(&db, restore_point_key(TEST_DATABASE, undo_id.as_str()))
					.await?
					.is_none()
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_point_uses_retained_record_without_vtx() -> Result<()> {
	restore_matrix!(
		"resolve_restore_point_uses_retained_record_without_vtx",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let row = commit_row(&db, branch_id, 1).await?;

			let resolved = database_db
				.resolve_restore_point(restore_point.clone())
				.await?;

			assert_eq!(resolved.versionstamp, row.versionstamp);
			assert_eq!(
				resolved.restore_point,
				Some(RestorePointRef {
					restore_point: restore_point.clone(),
					resolved_versionstamp: Some(row.versionstamp),
				})
			);

			clear_value(&db, branch_vtx_key(branch_id, row.versionstamp)).await?;
			let resolved = database_db
				.resolve_restore_point(restore_point.clone())
				.await?;
			assert_eq!(resolved.versionstamp, row.versionstamp);

			clear_value(
				&db,
				restore_point_key(TEST_DATABASE, restore_point.as_str()),
			)
			.await?;
			let err = database_db
				.resolve_restore_point(restore_point)
				.await
				.expect_err("missing restore point record should be rejected");
			assert_sqlite_error(err, SqliteStorageError::RestorePointNotFound);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_target_latest_returns_current_head_metadata() -> Result<()> {
	restore_matrix!(
		"resolve_restore_target_latest_returns_current_head_metadata",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			database_db.commit(vec![page(1, 0x22)], 2, 2_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let head_row = commit_row(&db, branch_id, 2).await?;

			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::Latest)
				.await?;

			assert_eq!(
				resolved,
				ResolvedRestoreTarget {
					database_branch_id: branch_id,
					txid: 2,
					versionstamp: head_row.versionstamp,
					wall_clock_ms: 2_000,
					kind: SnapshotKind::Latest,
					restore_point: None,
				}
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_target_timestamp_uses_latest_unexpired_interval_before_target()
-> Result<()> {
	restore_matrix!(
		"resolve_restore_target_timestamp_uses_latest_unexpired_interval_before_target",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			database_db.commit(vec![page(1, 0x22)], 2, 2_500).await?;
			database_db.commit(vec![page(1, 0x33)], 2, 3_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let first = commit_row(&db, branch_id, 1).await?;
			let second = commit_row(&db, branch_id, 2).await?;
			let third = commit_row(&db, branch_id, 3).await?;
			seed_pitr_interval(
				&db,
				branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: first.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;
			seed_pitr_interval(
				&db,
				branch_id,
				2_000,
				PitrIntervalCoverage {
					txid: 2,
					versionstamp: second.versionstamp,
					wall_clock_ms: 2_500,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;
			seed_pitr_interval(
				&db,
				branch_id,
				3_000,
				PitrIntervalCoverage {
					txid: 3,
					versionstamp: third.versionstamp,
					wall_clock_ms: 3_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;

			let between = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: 2_750,
				})
				.await?;
			assert_eq!(between.txid, 2);
			assert_eq!(between.versionstamp, second.versionstamp);
			assert_eq!(between.wall_clock_ms, 2_500);
			assert_eq!(between.kind, SnapshotKind::AtTimestamp);
			assert_eq!(between.restore_point, None);

			let quiet_period = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: 2_999,
				})
				.await?;
			assert_eq!(quiet_period.txid, 2);

			let walked_back = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: 2_100,
				})
				.await?;
			assert_eq!(walked_back.txid, 1);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_target_timestamp_rejects_expired_interval_coverage() -> Result<()> {
	restore_matrix!(
		"resolve_restore_target_timestamp_rejects_expired_interval_coverage",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let row = commit_row(&db, branch_id, 1).await?;
			seed_pitr_interval(
				&db,
				branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: row.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: 0,
				},
			)
			.await?;

			let err = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_000,
				})
				.await
				.expect_err("expired interval coverage should not resolve");

			assert_sqlite_error(err, SqliteStorageError::RestoreTargetExpired);

			Ok(())
		}
	)
}

#[tokio::test]
async fn restore_point_survives_after_source_interval_coverage_expires() -> Result<()> {
	restore_matrix!(
		"restore_point_survives_after_source_interval_coverage_expires",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let row = commit_row(&db, branch_id, 1).await?;
			seed_pitr_interval(
				&db,
				branch_id,
				1_000,
				PitrIntervalCoverage {
					txid: 1,
					versionstamp: row.versionstamp,
					wall_clock_ms: 1_000,
					expires_at_ms: i64::MAX,
				},
			)
			.await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_000,
				})
				.await?;

			clear_value(&db, branch_pitr_interval_key(branch_id, 1_000)).await?;
			let err = database_db
				.resolve_restore_target(SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_000,
				})
				.await
				.expect_err("timestamp selector should expire with interval coverage");
			assert_sqlite_error(err, SqliteStorageError::RestoreTargetExpired);

			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::RestorePoint {
					restore_point: restore_point.clone(),
				})
				.await?;
			assert_eq!(resolved.txid, 1);
			assert_eq!(resolved.versionstamp, row.versionstamp);
			assert_eq!(
				resolved.restore_point,
				Some(RestorePointRef {
					restore_point,
					resolved_versionstamp: Some(row.versionstamp),
				})
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_target_restore_point_returns_exact_metadata() -> Result<()> {
	restore_matrix!(
		"resolve_restore_target_restore_point_returns_exact_metadata",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let row = commit_row(&db, branch_id, 1).await?;

			let resolved = database_db
				.resolve_restore_target(SnapshotSelector::RestorePoint {
					restore_point: restore_point.clone(),
				})
				.await?;

			assert_eq!(
				resolved,
				ResolvedRestoreTarget {
					database_branch_id: branch_id,
					txid: 1,
					versionstamp: row.versionstamp,
					wall_clock_ms: 1_000,
					kind: SnapshotKind::RestorePoint,
					restore_point: Some(RestorePointRef {
						restore_point,
						resolved_versionstamp: Some(row.versionstamp),
					}),
				}
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_target_restore_point_rejects_missing_record() -> Result<()> {
	restore_matrix!(
		"resolve_restore_target_restore_point_rejects_missing_record",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let missing = RestorePointId::format(1_010, 1)?;

			let err = database_db
				.resolve_restore_target(SnapshotSelector::RestorePoint {
					restore_point: missing,
				})
				.await
				.expect_err("missing restore point should not resolve");

			assert_sqlite_error(err, SqliteStorageError::RestorePointNotFound);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_point_prefers_exact_pinned_record() -> Result<()> {
	restore_matrix!(
		"resolve_restore_point_prefers_exact_pinned_record",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let pinned_versionstamp = [9; 16];
			db.run({
				let restore_point = restore_point.clone();

				move |tx| {
					let restore_point = restore_point.clone();

					async move {
						let record = RestorePointRecord {
							restore_point_id: restore_point.clone(),
							database_branch_id: branch_id,
							versionstamp: pinned_versionstamp,
							status: PinStatus::Ready,
							pin_object_key: None,
							created_at_ms: 1_000,
							updated_at_ms: 1_100,
						};
						tx.informal().set(
							&depot::keys::restore_point_key(TEST_DATABASE, restore_point.as_str()),
							&encode_restore_point_record(record)?,
						);
						Ok(())
					}
				}
			})
			.await?;

			let resolved = database_db
				.resolve_restore_point(restore_point.clone())
				.await?;

			assert_eq!(resolved.versionstamp, pinned_versionstamp);
			assert_eq!(
				resolved.restore_point,
				Some(RestorePointRef {
					restore_point,
					resolved_versionstamp: Some(pinned_versionstamp),
				})
			);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_point_rejects_missing_record_on_forked_database() -> Result<()> {
	restore_matrix!(
		"resolve_restore_point_rejects_missing_record_on_forked_database",
		|ctx, db, source_database_db| {
			source_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let restore_point = source_database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let source_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let source_commit = commit_row(&db, source_branch_id, 1).await?;
			let forked_database_id = branch::fork_database(
				&db,
				bucket_id,
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				bucket_id,
			)
			.await?;

			let err = restore_point::resolve_restore_point(
				&db,
				bucket_id,
				forked_database_id,
				restore_point,
			)
			.await
			.expect_err("restore point records are scoped to the owning database id");
			assert_sqlite_error(err, SqliteStorageError::RestorePointNotFound);

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_restore_point_honors_bucket_fork_versionstamp_cap() -> Result<()> {
	restore_matrix!(
		"resolve_restore_point_honors_bucket_fork_versionstamp_cap",
		|ctx, db, source_database_db| {
			source_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let bucket_id = BucketId::from_gas_id(test_bucket());
			let source_branch_id = database_branch_id(&db, bucket_id, TEST_DATABASE).await?;
			let fork_point = commit_row(&db, source_branch_id, 1).await?;
			let forked_bucket = branch::fork_bucket(
				&db,
				bucket_id,
				ResolvedVersionstamp {
					versionstamp: fork_point.versionstamp,
					restore_point: None,
				},
			)
			.await?;

			source_database_db
				.commit(vec![page(2, 0x22)], 3, 2_000)
				.await?;
			let post_fork_restore_point = source_database_db
				.create_restore_point(SnapshotSelector::Latest)
				.await?;

			let err = restore_point::resolve_restore_point(
				&db,
				forked_bucket,
				TEST_DATABASE.to_string(),
				post_fork_restore_point,
			)
			.await
			.expect_err("bucket fork should not resolve source commits created after the fork");
			assert_sqlite_error(err, SqliteStorageError::BranchNotReachable);

			Ok(())
		}
	)
}
