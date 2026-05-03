mod common;

use anyhow::{Context, Result};
use depot::{
	conveyer::branch,
	error::SqliteStorageError,
	keys::{
		branch_commit_key, branch_meta_head_at_fork_key, branch_vtx_key, branches_desc_pin_key,
		branches_restore_point_pin_key,
	},
	types::{BucketBranchId, BucketId, DatabaseBranchId, DirtyPage, decode_commit_row},
};
use gas::prelude::Id;
use universaldb::utils::IsolationLevel::Serializable;

const TEST_DATABASE: &str = "database-a";

fn bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
}

async fn database_branch_id_for(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	db.run(move |tx| async move {
		branch::resolve_database_branch(
			&tx,
			BucketId::from_gas_id(bucket()),
			TEST_DATABASE,
			Serializable,
		)
		.await?
		.context("database branch should exist")
	})
	.await
}

fn has_storage_error(err: &anyhow::Error, expected: SqliteStorageError) -> bool {
	err.chain().any(|cause| {
		cause
			.downcast_ref::<SqliteStorageError>()
			.is_some_and(|err| err == &expected)
	})
}

#[tokio::test]
async fn gc_pin_recompute_under_restore_point_delete_race() -> Result<()> {
	common::test_matrix("depot-gc-restore-point-race", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.make_db(bucket(), TEST_DATABASE);
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = database_branch_id_for(&db).await?;
			let commit_bytes = common::read_value(&db, branch_commit_key(branch_id, 1))
				.await?
				.context("commit row should exist")?;
			let commit = decode_commit_row(&commit_bytes)?;
			let restore_point = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;

			database_db.delete_restore_point(restore_point).await?;
			let fork_before_gc = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x1111));
			db.run(move |tx| async move {
				branch::derive_branch_at(
					&tx,
					branch_id,
					commit.versionstamp,
					fork_before_gc,
					BucketBranchId::nil(),
					None,
				)
				.await
			})
			.await?;
			assert!(
				common::read_value(&db, branch_meta_head_at_fork_key(fork_before_gc))
					.await?
					.is_some(),
				"fork should still succeed while the hot rows have not been GC'd"
			);

			db.run(move |tx| async move {
				tx.informal().clear(&branch_commit_key(branch_id, 1));
				tx.informal()
					.clear(&branch_vtx_key(branch_id, commit.versionstamp));
				tx.informal().clear(&branches_desc_pin_key(branch_id));
				tx.informal()
					.clear(&branches_restore_point_pin_key(branch_id));
				Ok(())
			})
			.await?;
			let fork_after_gc = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x2222));
			let err = db
				.run(move |tx| async move {
					branch::derive_branch_at(
						&tx,
						branch_id,
						commit.versionstamp,
						fork_after_gc,
						BucketBranchId::nil(),
						None,
					)
					.await
				})
				.await
				.expect_err("fork should fail once GC has removed the VTX row");
			assert!(
				has_storage_error(&err, SqliteStorageError::RestoreTargetExpired),
				"unexpected error: {err:?}"
			);
			assert!(
				common::read_value(&db, branch_meta_head_at_fork_key(fork_after_gc))
					.await?
					.is_none()
			);

			Ok(())
		})
	})
	.await
}
