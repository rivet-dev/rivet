mod support;

use anyhow::Result;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::CheckpointOutcome,
};

#[tokio::test]
async fn fork_at_head() -> Result<()> {
	let db = support::test_db("sqlite-fork-head-").await?;
	let src = "fork-head-src";
	let dst = "fork-head-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 2).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(1, 0x33)], 4, 300).await?;

	support::run_fork(
		db.clone(),
		src,
		RestoreTarget::Txid(3),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
	)
	.await?;

	let src_pages = support::read_pages(db.clone(), src, vec![1, 2]).await?;
	let dst_pages = support::read_pages(db.clone(), dst, vec![1, 2]).await?;
	assert_eq!(dst_pages, src_pages);
	assert_eq!(support::read_head(db.clone(), dst).await?.head_txid, 3);
	assert_eq!(support::read_head(db, src).await?.head_txid, 3);
	Ok(())
}

#[tokio::test]
async fn fork_at_past_txid() -> Result<()> {
	let db = support::test_db("sqlite-fork-past-").await?;
	let src = "fork-past-src";
	let dst = "fork-past-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;
	support::commit_pages(db.clone(), src, vec![(1, 0x33)], 4, 300).await?;

	support::run_fork(
		db.clone(),
		src,
		RestoreTarget::Txid(2),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
	)
	.await?;

	let pages = support::read_pages(db.clone(), dst, vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; 4096]));
	assert_eq!(pages[1].bytes, Some(vec![0x22; 4096]));
	assert_eq!(support::read_head(db, dst).await?.head_txid, 2);
	Ok(())
}
