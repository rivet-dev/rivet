mod support;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{ForkDstSpec, ForkMode, RestoreTarget},
	compactor::{self, CheckpointOutcome},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn fork_concurrent_two_dsts() -> Result<()> {
	let db = support::test_db("sqlite-fork-concurrent-").await?;
	let src = "fork-concurrent-src";
	let dst_b = "fork-concurrent-b";
	let dst_c = "fork-concurrent-c";
	support::commit_pages(db.clone(), src, vec![(1, 0x11)], 4, 100).await?;
	assert!(matches!(
		support::checkpoint(db.clone(), src, 1).await?,
		CheckpointOutcome::Created { .. }
	));
	support::commit_pages(db.clone(), src, vec![(2, 0x22)], 4, 200).await?;
	let op_b = Uuid::new_v4();
	let op_c = Uuid::new_v4();
	support::create_fork_record(db.clone(), src, op_b).await?;
	support::create_fork_record(db.clone(), src, op_c).await?;

	let fork_b = compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-concurrent-b"),
		op_b,
		src.to_string(),
		RestoreTarget::Txid(2),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst_b.to_string(),
		},
		NodeId::new(),
		CancellationToken::new(),
	);
	let fork_c = compactor::handle_fork(
		db.clone(),
		support::test_ups("fork-concurrent-c"),
		op_c,
		src.to_string(),
		RestoreTarget::Txid(2),
		ForkMode::Apply,
		ForkDstSpec::Existing {
			dst_actor_id: dst_c.to_string(),
		},
		NodeId::new(),
		CancellationToken::new(),
	);
	tokio::try_join!(fork_b, fork_c)?;

	let src_pages = support::read_pages(db.clone(), src, vec![1, 2]).await?;
	assert_eq!(support::read_pages(db.clone(), dst_b, vec![1, 2]).await?, src_pages);
	assert_eq!(support::read_pages(db, dst_c, vec![1, 2]).await?, src_pages);
	Ok(())
}
