mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::{self, CheckpointOutcome, fork::test_hooks},
	keys::meta_compactor_lease_key,
};
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

static REFCOUNT_HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn fork_refcount_sequencing() -> Result<()> {
	let _lock = REFCOUNT_HOOK_LOCK.lock().await;
	let db = support::test_db("sqlite-fork-refcount-sequencing-").await?;
	let src = "fork-refcount-src";
	let dst = "fork-refcount-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	let op_id = Uuid::new_v4();
	support::create_fork_record(db.clone(), src, op_id).await?;
	let (_guard, reached, release) = test_hooks::pause_after_source_refs_pinned(src);
	let holder = NodeId::new();
	let task = tokio::spawn(compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-refcount-sequencing"),
		op_id,
		src.to_string(),
		RestoreTarget::Txid(1),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
		holder,
		CancellationToken::new(),
	));

	tokio::time::timeout(std::time::Duration::from_secs(1), reached.notified()).await?;
	assert_eq!(support::read_checkpoint_meta(db.clone(), src, 1).await?.refcount, 1);
	let lease_exists = db
		.run({
			let src = src.to_string();
			move |tx| {
				let src = src.clone();
				async move {
					Ok(tx
						.informal()
						.get(&meta_compactor_lease_key(&src), Snapshot)
						.await?
						.is_some())
				}
			}
		})
		.await?;
	assert!(lease_exists);
	release.notify_waiters();
	task.await??;
	assert_eq!(support::read_checkpoint_meta(db, src, 1).await?.refcount, 0);
	Ok(())
}
