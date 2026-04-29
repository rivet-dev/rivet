mod support;

use anyhow::Result;
use sqlite_storage::{
	admin::{RestoreMode, RestoreTarget},
	compactor::CheckpointOutcome,
};

#[tokio::test]
async fn restore_to_current_head() -> Result<()> {
	let db = support::test_db("sqlite-restore-current-").await?;
	let actor_id = "restore-current";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	support::commit_pages(db.clone(), actor_id, vec![(2, 0x22)], 4, 200).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 2).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(db.clone(), actor_id, RestoreTarget::Txid(3), RestoreMode::Apply)
		.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x33; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert_eq!(support::read_head(db, actor_id).await?.head_txid, 3);
	Ok(())
}

#[tokio::test]
async fn restore_to_past_txid_via_delta_replay() -> Result<()> {
	let db = support::test_db("sqlite-restore-past-").await?;
	let actor_id = "restore-past";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(2, 0x22)], 4, 200).await?;
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(db.clone(), actor_id, RestoreTarget::Txid(2), RestoreMode::Apply)
		.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert_eq!(support::read_head(db, actor_id).await?.head_txid, 2);
	Ok(())
}

#[tokio::test]
async fn restore_to_exact_checkpoint() -> Result<()> {
	let db = support::test_db("sqlite-restore-checkpoint-").await?;
	let actor_id = "restore-checkpoint";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(
		db.clone(),
		actor_id,
		RestoreTarget::CheckpointTxid(1),
		RestoreMode::Apply,
	)
	.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(support::read_head(db, actor_id).await?.head_txid, 1);
	Ok(())
}

#[tokio::test]
async fn restore_dry_run() -> Result<()> {
	let db = support::test_db("sqlite-restore-dry-run-").await?;
	let actor_id = "restore-dry-run";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x33)], 4, 300).await?;

	support::run_restore(db.clone(), actor_id, RestoreTarget::Txid(1), RestoreMode::DryRun)
		.await?;

	let pages = support::read_pages(db.clone(), actor_id, vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x33; 4096]));
	assert!(!support::marker_exists(db, actor_id).await?);
	Ok(())
}
