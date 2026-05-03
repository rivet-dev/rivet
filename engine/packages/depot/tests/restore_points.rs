mod common;

use anyhow::{Context, Result};
use depot::{
	constants::HOT_RETENTION_FLOOR_MS,
	conveyer::{branch, history_pin, restore_point},
	error::SqliteStorageError,
	keys::{
		branch_commit_key, branch_manifest_last_hot_pass_txid_key, branch_meta_compact_key,
		branch_shard_key, branch_vtx_key, db_pin_key,
	},
	types::{
		BucketId, CommitRow, DatabaseBranchId, DbHistoryPinKind, DirtyPage, PinStatus,
		RestorePointRef, decode_commit_row, decode_db_history_pin, decode_restore_point_record,
		encode_meta_compact,
	},
};
use gas::prelude::Id;
use universaldb::utils::IsolationLevel::Serializable;

const OTHER_ACTOR: &str = "restore_point-other";

fn other_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

fn nil_bucket() -> Id {
	Id::v1(uuid::Uuid::nil(), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
}

fn now_ms() -> i64 {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.expect("system clock should be after unix epoch");
	i64::try_from(elapsed.as_millis()).expect("timestamp should fit i64")
}

async fn database_branch_id(
	db: &universaldb::Database,
	bucket_id: Id,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let bucket_id = BucketId::from_gas_id(bucket_id);

	db.run(move |tx| async move {
		branch::resolve_database_branch(&tx, bucket_id, database_id, Serializable)
			.await?
			.context("database branch should exist")
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
async fn restore_point_resolves_to_retained_record_versionstamp() -> Result<()> {
	common::test_matrix("depot-restore-point-resolves", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let database_id = ctx.database_id.clone();
			let database_db = ctx.make_db(source_bucket, database_id.clone());

			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			let branch_id = database_branch_id(&db, source_bucket, &database_id).await?;
			let row = commit_row(&db, branch_id, 1).await?;

			let resolved = database_db
				.resolve_restore_point(restore_point.clone())
				.await?;

			assert_eq!(resolved.versionstamp, row.versionstamp);
			assert_eq!(
				resolved.restore_point,
				Some(RestorePointRef {
					restore_point,
					resolved_versionstamp: Some(row.versionstamp),
				})
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn restore_point_is_ready_without_legacy_cold_handoff_or_crash_window() -> Result<()> {
	common::test_matrix("depot-restore-point-ready", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let database_id = ctx.database_id.clone();
			let database_db = ctx.make_db(source_bucket, database_id.clone());
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = database_branch_id(&db, source_bucket, &database_id).await?;
			let row = commit_row(&db, branch_id, 1).await?;

			db.run(move |tx| async move {
				tx.informal().set(
					&branch_meta_compact_key(branch_id),
					&encode_meta_compact(depot::types::MetaCompact {
						materialized_txid: 1,
					})?,
				);
				tx.informal().set(
					&branch_manifest_last_hot_pass_txid_key(branch_id),
					&1u64.to_be_bytes(),
				);
				tx.informal()
					.set(&branch_shard_key(branch_id, 0, 1), b"image-one");
				Ok(())
			})
			.await?;

			let restore_point = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			assert_eq!(
				database_db
					.restore_point_status(restore_point.clone())
					.await?,
				Some(PinStatus::Ready)
			);

			let pinned_bytes = common::read_value(
				&db,
				depot::keys::restore_point_key(&database_id, restore_point.as_str()),
			)
			.await?
			.context("restore point record should exist")?;
			let pinned = decode_restore_point_record(&pinned_bytes)?;
			assert_eq!(pinned.status, PinStatus::Ready);
			assert_eq!(
				database_db.restore_point_status(restore_point).await?,
				Some(PinStatus::Ready)
			);
			assert!(pinned.pin_object_key.is_none());
			let db_pin_bytes = common::read_value(
				&db,
				db_pin_key(
					branch_id,
					&history_pin::restore_point_pin_id(&pinned.restore_point_id),
				),
			)
			.await?
			.context("restore point DB_PIN should exist")?;
			let db_pin = decode_db_history_pin(&db_pin_bytes)?;
			assert_eq!(db_pin.kind, DbHistoryPinKind::RestorePoint);
			assert_eq!(db_pin.at_txid, 1);
			assert_eq!(db_pin.at_versionstamp, row.versionstamp);
			assert_eq!(db_pin.owner_restore_point, Some(pinned.restore_point_id));

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn parent_bucket_restore_point_resolves_from_forked_bucket() -> Result<()> {
	common::test_matrix("depot-restore-point-forked-bucket", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = source
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			let source_branch = database_branch_id(&db, source_bucket, &database_id).await?;
			let fork_point = commit_row(&db, source_branch, 1).await?;
			let forked_bucket = branch::fork_bucket(
				&db,
				BucketId::from_gas_id(source_bucket),
				depot::types::ResolvedVersionstamp {
					versionstamp: fork_point.versionstamp,
					restore_point: None,
				},
			)
			.await?;

			let resolved = restore_point::resolve_restore_point(
				&db,
				forked_bucket,
				database_id.clone(),
				restore_point,
			)
			.await?;

			assert_eq!(resolved.versionstamp, fork_point.versionstamp);

			source.commit(vec![page(2, 0x22)], 3, 2_000).await?;
			let post_fork_restore_point = source
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			let err = restore_point::resolve_restore_point(
				&db,
				forked_bucket,
				database_id,
				post_fork_restore_point,
			)
			.await
			.expect_err("post-fork source restore_point should not be visible in forked bucket");
			assert_storage_error(err, SqliteStorageError::BranchNotReachable);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn restore_point_survives_hot_commit_and_vtx_reclaim() -> Result<()> {
	common::test_matrix("depot-restore-point-vtx-reclaim", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			let database_db = ctx.make_db(nil_bucket(), database_id.clone());
			let current_ms = now_ms();
			let old_ms = current_ms - HOT_RETENTION_FLOOR_MS - 1_000;
			let recent_ms = current_ms - 1_000;

			database_db.commit(vec![page(1, 0x11)], 2, old_ms).await?;
			let old_restore_point = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			database_db
				.commit(vec![page(2, 0x22)], 3, recent_ms)
				.await?;
			let branch_id = database_branch_id(&db, nil_bucket(), &database_id).await?;
			let old_row = commit_row(&db, branch_id, 1).await?;

			db.run(move |tx| async move {
				tx.informal().clear(&branch_commit_key(branch_id, 1));
				tx.informal()
					.clear(&branch_vtx_key(branch_id, old_row.versionstamp));
				Ok(())
			})
			.await?;

			assert!(
				common::read_value(&db, branch_commit_key(branch_id, 1))
					.await?
					.is_none()
			);
			assert!(
				common::read_value(&db, branch_vtx_key(branch_id, old_row.versionstamp))
					.await?
					.is_none()
			);

			let resolved = database_db.resolve_restore_point(old_restore_point).await?;
			assert_eq!(resolved.versionstamp, old_row.versionstamp);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn unrelated_database_restore_point_returns_branch_not_reachable() -> Result<()> {
	common::test_matrix("depot-restore-point-unrelated", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, database_id);
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = source
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			let other = ctx.make_db(other_bucket(), OTHER_ACTOR);
			other.commit(vec![page(1, 0x22)], 2, 1_000).await?;

			let err = restore_point::resolve_restore_point(
				&db,
				BucketId::from_gas_id(source_bucket),
				OTHER_ACTOR.to_string(),
				restore_point,
			)
			.await
			.expect_err("database from another bucket should not be reachable here");

			assert_storage_error(err, SqliteStorageError::BranchNotReachable);

			Ok(())
		})
	})
	.await
}
