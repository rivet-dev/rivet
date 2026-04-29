mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{RestoreMode, RestoreTarget},
	compactor::{CheckpointOutcome, handle_restore, restore::test_hooks},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn restore_blocks_concurrent_commit() -> Result<()> {
	let db = support::test_db("sqlite-restore-commit-guard-").await?;
	let actor_id = "restore-commit-guard";
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), actor_id, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), actor_id, vec![(1, 0x22)], 4, 200).await?;
	let op_id = Uuid::new_v4();
	support::create_restore_record(db.clone(), actor_id, op_id).await?;
	let holder = NodeId::new();
	let (_guard, reached, release) = test_hooks::pause_after_marker_clear(actor_id);
	let restore_task = tokio::spawn(handle_restore(
		db.clone(),
		op_id,
		actor_id.to_string(),
		RestoreTarget::Txid(1),
		RestoreMode::Apply,
		holder,
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	let err = support::actor_db(db.clone(), actor_id)
		.commit(vec![support::page(2, 0x44)], 4, 400)
		.await
		.expect_err("commit should be blocked while restore marker exists");
	support::assert_actor_restore_error(&err);

	release.notify_waiters();
	restore_task.await??;
	support::actor_db(db, actor_id)
		.commit(vec![support::page(2, 0x55)], 4, 500)
		.await?;
	Ok(())
}
