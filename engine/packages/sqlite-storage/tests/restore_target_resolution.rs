mod support;

use anyhow::Result;
use sqlite_storage::{
	admin::{RestoreMode, RestoreTarget},
	compactor::CheckpointOutcome,
};

#[tokio::test]
async fn restore_timestamp_resolves_to_matching_delta() -> Result<()> {
	let db = support::test_db("sqlite-restore-timestamp-").await?;
	let actor_id = "restore-timestamp";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(2, 0x22)], 4, 200).await?;
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(
		db.clone(),
		actor_id,
		RestoreTarget::TimestampMs(250),
		RestoreMode::Apply,
	)
	.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert_eq!(support::read_head(db, actor_id).await?.head_txid, 2);
	Ok(())
}

#[tokio::test]
async fn restore_latest_checkpoint_resolves_to_checkpoint_txid() -> Result<()> {
	let db = support::test_db("sqlite-restore-latest-checkpoint-").await?;
	let actor_id = "restore-latest-checkpoint";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x22)], 4, 200).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 2).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(
		db.clone(),
		actor_id,
		RestoreTarget::LatestCheckpoint,
		RestoreMode::Apply,
	)
	.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x22; 4096]));
	assert_eq!(support::read_head(db, actor_id).await?.head_txid, 2);
	Ok(())
}
