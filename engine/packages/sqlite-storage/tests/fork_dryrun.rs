mod support;

use anyhow::Result;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::CheckpointOutcome,
	keys::meta_head_key,
};
use universaldb::utils::IsolationLevel::Snapshot;

#[tokio::test]
async fn fork_dryrun() -> Result<()> {
	let db = support::test_db("sqlite-fork-dryrun-").await?;
	let src = "fork-dryrun-src";
	let dst = "fork-dryrun-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;

	support::run_fork(
		db.clone(),
		src,
		RestoreTarget::Txid(2),
		ForkMode::DryRun,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
	)
	.await?;

	let dst_has_head = db
		.run({
			let dst = dst.to_string();
			move |tx| {
				let dst = dst.clone();
				async move {
					Ok(tx
						.informal()
						.get(&meta_head_key(&dst), Snapshot)
						.await?
						.is_some())
				}
			}
		})
		.await?;
	assert!(!dst_has_head);
	assert!(!support::fork_marker_exists(db, dst).await?);
	Ok(())
}
