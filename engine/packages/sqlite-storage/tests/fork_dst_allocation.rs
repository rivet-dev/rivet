mod support;

use anyhow::Result;
use rivet_error::RivetError;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::{self, CheckpointOutcome},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn fork_dst_spec_existing_empty() -> Result<()> {
	let db = support::test_db("sqlite-fork-existing-empty-").await?;
	let src = "fork-existing-empty-src";
	let dst = "fork-existing-empty-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));

	support::run_fork(
		db.clone(),
		src,
		RestoreTarget::Txid(1),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
	)
	.await?;

	assert_eq!(support::read_head(db, dst).await?.head_txid, 1);
	Ok(())
}

#[tokio::test]
async fn fork_dst_spec_existing_nonempty() -> Result<()> {
	let db = support::test_db("sqlite-fork-existing-nonempty-").await?;
	let src = "fork-existing-nonempty-src";
	let dst = "fork-existing-nonempty-dst";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), dst, vec![(1, 0x55)], 4, 150).await?;
	let op_id = Uuid::new_v4();
	support::create_fork_record(db.clone(), src, op_id).await?;

	let err = compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-existing-nonempty"),
		op_id,
		src.to_string(),
		RestoreTarget::Txid(1),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst.to_string(),
		},
		NodeId::new(),
		CancellationToken::new(),
	)
	.await
	.expect_err("nonempty destination should fail");
	let extracted = RivetError::extract(&err);
	assert_eq!(extracted.group(), "sqlite_admin");
	assert_eq!(extracted.code(), "fork_destination_exists");
	assert_eq!(support::read_checkpoint_meta(db.clone(), src, 1).await?.refcount, 0);
	Ok(())
}

#[tokio::test]
async fn fork_dst_spec_allocate() -> Result<()> {
	let db = support::test_db("sqlite-fork-allocate-").await?;
	let src = "fork-allocate-src";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));

	support::run_fork(
		db,
		src,
		RestoreTarget::Txid(1),
		ForkMode::Apply,
		ForkDstSpec::Allocate {
			dst_namespace_id: Uuid::new_v4(),
		},
	)
	.await?;
	Ok(())
}
