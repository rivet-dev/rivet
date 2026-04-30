mod fault_common;

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlite_storage::{
	error::SqliteStorageError,
	keys::{
		branch_commit_key, branch_meta_head_at_fork_key, branch_vtx_key, branches_bk_pin_key,
		branches_desc_pin_key,
	},
	pump::branch,
	types::{ActorBranchId, NamespaceBranchId, decode_commit_row},
};

fn has_storage_error(err: &anyhow::Error, expected: SqliteStorageError) -> bool {
	err.chain().any(|cause| {
		cause
			.downcast_ref::<SqliteStorageError>()
			.is_some_and(|err| err == &expected)
	})
}

#[tokio::test]
async fn gc_pin_recompute_under_bookmark_delete_race() -> Result<()> {
	let db = Arc::new(fault_common::test_db("sqlite-storage-gc-bookmark-race-").await?);
	let actor_db = fault_common::actor_db(Arc::clone(&db), fault_common::TEST_ACTOR);
	actor_db.commit(vec![fault_common::page(1, 0x11)], 2, 1_000).await?;
	let branch_id = fault_common::actor_branch_id_for(&db, fault_common::TEST_ACTOR).await?;
	let commit_bytes = fault_common::read_value(&db, branch_commit_key(branch_id, 1))
		.await?
		.context("commit row should exist")?;
	let commit = decode_commit_row(&commit_bytes)?;
	let bookmark = actor_db.create_pinned_bookmark(1_010).await?;

	actor_db.delete_pinned_bookmark(bookmark).await?;
	let fork_before_gc = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x1111));
	db.run(move |tx| async move {
		branch::derive_branch_at(
			&tx,
			branch_id,
			commit.versionstamp,
			fork_before_gc,
			NamespaceBranchId::nil(),
			None,
		)
		.await
	})
	.await?;
	assert!(
		fault_common::read_value(&db, branch_meta_head_at_fork_key(fork_before_gc))
			.await?
			.is_some(),
		"fork should still succeed while the hot rows have not been GC'd"
	);

	db.run(move |tx| async move {
		tx.informal().clear(&branch_commit_key(branch_id, 1));
		tx.informal().clear(&branch_vtx_key(branch_id, commit.versionstamp));
		tx.informal().clear(&branches_desc_pin_key(branch_id));
		tx.informal().clear(&branches_bk_pin_key(branch_id));
		Ok(())
	})
	.await?;
	let fork_after_gc = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x2222));
	let err = db
		.run(move |tx| async move {
			branch::derive_branch_at(
				&tx,
				branch_id,
				commit.versionstamp,
				fork_after_gc,
				NamespaceBranchId::nil(),
				None,
			)
			.await
		})
		.await
		.expect_err("fork should fail once GC has removed the VTX row");
	assert!(
		has_storage_error(&err, SqliteStorageError::BookmarkExpired),
		"unexpected error: {err:?}"
	);
	assert!(
		fault_common::read_value(&db, branch_meta_head_at_fork_key(fork_after_gc))
			.await?
			.is_none()
	);

	Ok(())
}
