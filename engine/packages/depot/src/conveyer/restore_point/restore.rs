use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Serializable;

use crate::conveyer::{
	branch,
	error::SqliteStorageError,
	keys,
	restore_point::{
		pinned::create_restore_point_for_resolved_tx,
		resolve::resolve_restore_target,
		shared::{ResolvedRestorePointPin, RestorePointCreateResult},
		test_hooks,
	},
	types::{
		BucketId, DatabaseBranchId, RestorePointId, SnapshotSelector, decode_commit_row,
		decode_db_head,
	},
};

pub async fn restore_database(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	selector: SnapshotSelector,
) -> Result<RestorePointId> {
	let target = resolve_restore_target(udb, bucket_id, database_id.clone(), selector).await?;
	let undo =
		capture_current_restore_point_for_restore(udb, bucket_id, database_id.clone()).await?;

	let result =
		restore_database_to_target_and_pin_undo(udb, bucket_id, database_id, target, undo).await?;

	Ok(result.restore_point)
}

async fn restore_database_to_target_and_pin_undo(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	target: crate::conveyer::types::ResolvedRestoreTarget,
	undo: ResolvedRestorePointPin,
) -> Result<RestorePointCreateResult> {
	let rolled_branch_id = DatabaseBranchId::new_v4();

	udb.run(move |tx| {
		let database_id = database_id.clone();
		let target = target.clone();
		let undo = undo.clone();

		async move {
			branch::rollback_database_to_target_tx(
				&tx,
				bucket_id,
				&database_id,
				&target,
				rolled_branch_id,
			)
			.await?;
			test_hooks::maybe_fail_after_restore_rollback(&database_id)?;
			create_restore_point_for_resolved_tx(&tx, bucket_id, &database_id, &undo).await
		}
	})
	.await
}

async fn capture_current_restore_point_for_restore(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
) -> Result<ResolvedRestorePointPin> {
	udb.run(move |tx| {
		let database_id = database_id.clone();

		async move {
			let branch_id =
				branch::resolve_database_branch(&tx, bucket_id, &database_id, Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;
			let head_bytes = tx
				.informal()
				.get(&keys::branch_meta_head_key(branch_id), Serializable)
				.await?
				.context("sqlite database branch head is missing")?;
			let head = decode_db_head(&head_bytes).context("decode sqlite database branch head")?;
			let commit_bytes = tx
				.informal()
				.get(
					&keys::branch_commit_key(branch_id, head.head_txid),
					Serializable,
				)
				.await?
				.context("sqlite database branch head commit row is missing")?;
			let commit = decode_commit_row(&commit_bytes)
				.context("decode sqlite database branch commit row")?;

			Ok(ResolvedRestorePointPin {
				restore_point: RestorePointId::format(commit.wall_clock_ms, head.head_txid)?,
				database_branch_id: branch_id,
				versionstamp: commit.versionstamp,
				created_at_ms: commit.wall_clock_ms,
			})
		}
	})
	.await
}
