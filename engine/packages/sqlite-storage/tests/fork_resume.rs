mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::{self, CheckpointOutcome, fork::test_hooks},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

static FORK_RESUME_HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn fork_resume_after_pod_failure() -> Result<()> {
	let _lock = FORK_RESUME_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-fork-resume-").await?;
	let src = "fork-resume-src";
	let dst = "fork-resume-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;
	let op_id = Uuid::new_v4();
	support::create_fork_record(db.clone(), src, op_id).await?;
	let holder = NodeId::new();
	let (guard, reached, _release) = test_hooks::pause_after_marker_write(dst);
	let task = tokio::spawn(compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-resume"),
		op_id,
		src.to_string(),
		RestoreTarget::Txid(2),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
		holder,
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	assert!(support::fork_marker_exists(db.clone(), dst).await?);
	task.abort();
	let _ = task.await;
	drop(guard);

	compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-resume-2"),
		op_id,
		src.to_string(),
		RestoreTarget::Txid(2),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
		holder,
		CancellationToken::new(),
	)
	.await?;

	let pages = support::read_pages(db.clone(), dst, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert!(!support::fork_marker_exists(db, dst).await?);
	Ok(())
}
