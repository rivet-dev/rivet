// Repro: a stale actor instance whose cached `state.head_txid` happens to
// match the txid of a freshly-restored branch's `head_at_fork` can pass the
// depot head fence and land writes on top of a rolled-back branch, silently
// undoing the restore.
//
// The depot fence at `engine/packages/depot/src/conveyer/commit/apply.rs:154-194`
// reads `actual_head_txid` from `previous_head_bytes = head_bytes.or(head_at_fork_bytes)`.
// `engine/packages/depot/src/conveyer/branch/fork.rs:238-251` writes
// `head_at_fork.head_txid = txid_at_versionstamp` when a branch is derived
// for a restore/rollback. So a fresh branch B2 forked at txid=K reports
// `actual_head_txid = K` to the next commit's fence check. Any stale writer
// whose cached `state.head_txid == K` (e.g. because it went stale right
// after the restore-point pin commit) passes the fence and overwrites the
// restored state.
//
// When envoy breaks and a stale instance reconnects after an operator
// restore, the fence does not protect the restored state from the stale
// instance's pending dirty pages.

mod common;

use anyhow::Result;
use depot::{
	keys::PAGE_SIZE,
	types::{CommitOptions, DirtyPage, FetchedPage, SnapshotSelector},
};
use gas::prelude::Id;

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0xfb02), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn fetched_page(pgno: u32, fill: u8) -> FetchedPage {
	FetchedPage {
		pgno,
		bytes: Some(vec![fill; PAGE_SIZE as usize]),
	}
}

#[tokio::test]
async fn stale_writer_lands_on_rolled_back_branch_via_head_at_fork() -> Result<()> {
	common::test_matrix("depot-stale-writer-post-restore", |_tier, ctx| {
		Box::pin(async move {
			let db_owner = ctx.make_db(test_bucket(), TEST_DATABASE);

			// Initial state: page 1 = 0xAA at head_txid=1.
			db_owner
				.commit_with_options(
					vec![page(1, 0xAA)],
					1,
					1_000,
					CommitOptions {
						expected_head_txid: Some(0),
					},
				)
				.await?;

			// Operator pins this state with a restore point.
			let restore_point = db_owner.create_restore_point(SnapshotSelector::Latest).await?;

			// A second actor instance ("stale") was alive at this moment with
			// state.head_txid cached as Some(1) and a pending dirty page
			// (0xCC). It then loses its envoy connection.
			let db_stale = ctx.make_db(test_bucket(), TEST_DATABASE);

			// While stale is offline, the owner advances the database with
			// 0xBB at txid=2.
			db_owner
				.commit_with_options(
					vec![page(1, 0xBB)],
					1,
					1_001,
					CommitOptions {
						expected_head_txid: Some(1),
					},
				)
				.await?;

			// Operator decides the 0xBB write was a mistake and restores the
			// database to the pinned restore point. This forks a new branch
			// B2 with `head_at_fork.head_txid = 1` and swaps DBPTR onto it.
			db_owner
				.restore_database(SnapshotSelector::RestorePoint {
					restore_point: restore_point.clone(),
				})
				.await?;

			// Sanity: post-restore reads should reflect 0xAA.
			assert_eq!(
				db_owner.get_pages(vec![1]).await?,
				vec![fetched_page(1, 0xAA)],
				"post-restore baseline should be 0xAA"
			);

			// Stale instance reconnects through a working envoy. Its cached
			// state.head_txid is still Some(1) (its last successful response
			// from before going stale). It flushes its pending 0xCC page with
			// `expected_head_txid=Some(1)`.
			//
			// On B2 there is no /META/head yet, so apply.rs reads
			// /META/head_at_fork and finds head_txid=1. The fence check at
			// apply.rs:179-194 sees expected==actual==1 and accepts the write.
			let stale_commit = db_stale
				.commit_with_options(
					vec![page(1, 0xCC)],
					1,
					1_010,
					CommitOptions {
						expected_head_txid: Some(1),
					},
				)
				.await
				.expect(
					"BUG: stale writer's cached head_txid matches the rolled-back \
					 branch's head_at_fork, so the fence accepts a commit that \
					 overwrites the restored state",
				);
			assert_eq!(stale_commit.head_txid, 2);

			// The restore is silently undone. Reads now return 0xCC, not the
			// 0xAA the operator restored to.
			assert_eq!(
				db_owner.get_pages(vec![1]).await?,
				vec![fetched_page(1, 0xCC)],
				"restored state was silently overwritten by a stale writer"
			);

			Ok(())
		})
	})
	.await
}
