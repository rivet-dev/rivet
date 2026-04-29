mod support;

use anyhow::Result;
use sqlite_storage::{
	admin::{self, OpStatus, RestoreMode, RestoreTarget},
	compactor::CheckpointOutcome,
	keys,
};

#[tokio::test]
async fn restore_invalid_target_after_head_marks_failed() -> Result<()> {
	let db = support::test_db("sqlite-restore-invalid-head-").await?;
	let actor_id = "restore-invalid-head";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));

	let op_id = support::run_restore(
		db.clone(),
		actor_id,
		RestoreTarget::Txid(99),
		RestoreMode::Apply,
	)
	.await?;
	let record = admin::read(db, op_id).await?.expect("record should exist");
	assert_eq!(record.status, OpStatus::Failed);
	Ok(())
}

#[tokio::test]
async fn restore_invalid_target_with_missing_intermediate_delta_marks_failed() -> Result<()> {
	let db = support::test_db("sqlite-restore-missing-delta-").await?;
	let actor_id = "restore-missing-delta";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(2, 0x22)], 4, 200).await?;
	db.run({
		let actor_id = actor_id.to_string();
		move |tx| {
			let actor_id = actor_id.clone();
			async move {
				let prefix = keys::delta_chunk_prefix(&actor_id, 2);
				let (begin, end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
				tx.informal().clear_range(&begin, &end);
				Ok(())
			}
		}
	})
	.await?;

	let op_id = support::run_restore(
		db.clone(),
		actor_id,
		RestoreTarget::Txid(2),
		RestoreMode::Apply,
	)
	.await?;
	let record = admin::read(db, op_id).await?.expect("record should exist");
	assert_eq!(record.status, OpStatus::Failed);
	Ok(())
}
