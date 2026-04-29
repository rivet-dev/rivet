mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::{self, CheckpointOutcome, fork::test_hooks},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

static FORK_HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn fork_pins_deltas() -> Result<()> {
	let _lock = FORK_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-fork-pins-deltas-").await?;
	let src = "fork-pins-src";
	let dst = "fork-pins-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;
	support::commit_pages(db.clone(), src, vec![(3, 0x33)], 4, 300).await?;
	let op_id = Uuid::new_v4();
	support::create_fork_record(db.clone(), src, op_id).await?;
	let (_guard, reached, release) = test_hooks::pause_after_source_refs_pinned(src);
	let task = tokio::spawn(compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-pins-deltas"),
		op_id,
		src.to_string(),
		RestoreTarget::Txid(3),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
		NodeId::new(),
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	assert_eq!(support::read_checkpoint_meta(db.clone(), src, 1).await?.refcount, 1);
	assert_eq!(support::read_delta_meta(db.clone(), src, 2).await?.refcount, 1);
	assert_eq!(support::read_delta_meta(db.clone(), src, 3).await?.refcount, 1);
	release.notify_waiters();
	task.await??;
	assert_eq!(support::read_checkpoint_meta(db.clone(), src, 1).await?.refcount, 0);
	assert_eq!(support::read_delta_meta(db.clone(), src, 2).await?.refcount, 0);
	assert_eq!(support::read_delta_meta(db, src, 3).await?.refcount, 0);
	Ok(())
}
