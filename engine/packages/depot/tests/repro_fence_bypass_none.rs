// Repro: when an actor instance commits or reads with `expected_head_txid: None`,
// the depot head fence at `engine/packages/depot/src/conveyer/commit/apply.rs:179`
// and `engine/packages/depot/src/conveyer/read.rs:134` is silently skipped.
//
// The depot-client VFS sends `expected_head_txid: state.head_txid` (vfs.rs:1346,
// vfs.rs:1496). On a fresh actor instance (cold start, takeover, or post-crash
// restart) `state.head_txid` is `None` until the first response from depot
// populates it (vfs.rs:1357). When envoy breaks and a stale instance reconnects
// before its replacement has populated `state.head_txid`, both can issue
// commits with `expected_head_txid: None` and both pass the fence.
//
// This reproduction goes directly through the `Db` API (the same surface
// `pegboard-envoy` calls), no VFS plumbing required.

mod common;

use anyhow::Result;
use depot::{
	keys::PAGE_SIZE,
	types::{CommitOptions, DirtyPage, FetchedPage, GetPagesOptions},
};
use gas::prelude::Id;

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0xfb01), 1)
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
async fn commit_fence_bypassed_when_expected_head_txid_is_none() -> Result<()> {
	common::test_matrix("depot-fence-bypass-commit-none", |_tier, ctx| {
		Box::pin(async move {
			// Two Db handles on the same database. In production these would be
			// two different actor instances writing through their own
			// pegboard-envoy WS. Here they share UDB, mirroring the depot side
			// of two concurrent envoys.
			let db_a = ctx.make_db(test_bucket(), TEST_DATABASE);
			let db_b = ctx.make_db(test_bucket(), TEST_DATABASE);

			// Owner instance writes page 1 = 0xAA. expected=Some(0) is the
			// well-behaved bootstrap fence.
			let first = db_a
				.commit_with_options(
					vec![page(1, 0xAA)],
					1,
					1_000,
					CommitOptions {
						expected_head_txid: Some(0),
					},
				)
				.await?;
			assert_eq!(first.head_txid, 1);
			assert_eq!(
				db_a.get_pages(vec![1]).await?,
				vec![fetched_page(1, 0xAA)]
			);

			// Stale instance comes back online. Its in-memory head_txid is
			// None because it never finished a get_pages round-trip with the
			// previous head. It commits page 1 = 0xBB with `None`.
			//
			// Apply path at apply.rs:179-194 only fences when
			// `expected_head_txid` is `Some(_)`. With `None` the commit is
			// accepted regardless of the current head.
			let second = db_b
				.commit_with_options(
					vec![page(1, 0xBB)],
					1,
					1_001,
					CommitOptions {
						expected_head_txid: None,
					},
				)
				.await
				.expect(
					"BUG: expected_head_txid=None bypasses the fence even though \
					 db_a is the live writer and head is already at 1",
				);
			assert_eq!(
				second.head_txid, 2,
				"the second commit advanced head, proving the fence was bypassed"
			);

			// Corruption: db_a's content was silently overwritten. Verify via
			// a fresh Db handle to avoid db_a's stale read-cache. (db_a's
			// cache_snapshot still maps page 1 -> txid=1 from its own commit;
			// it would also return stale 0xAA pages without a fence error.)
			let db_verify = ctx.make_db(test_bucket(), TEST_DATABASE);
			assert_eq!(
				db_verify.get_pages(vec![1]).await?,
				vec![fetched_page(1, 0xBB)],
				"db_a's 0xAA was silently overwritten by db_b's 0xBB"
			);

			// And db_a still believes it owns the database at head=1, with
			// page 1 = 0xAA, until its next properly-fenced operation.
			assert_eq!(
				db_a.get_pages(vec![1]).await?,
				vec![fetched_page(1, 0xAA)],
				"db_a's local read-cache still serves the stale (pre-overwrite) state"
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn read_fence_bypassed_when_expected_head_txid_is_none() -> Result<()> {
	common::test_matrix("depot-fence-bypass-read-none", |_tier, ctx| {
		Box::pin(async move {
			let db_owner = ctx.make_db(test_bucket(), TEST_DATABASE);
			let db_stale = ctx.make_db(test_bucket(), TEST_DATABASE);

			// Owner advances the database to head_txid=2.
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

			// Stale instance whose cached state.head_txid was 1 from before
			// the owner's second commit. With a proper fence, it would learn
			// of the takeover via HeadFenceMismatch.
			let proper_fence = db_stale
				.get_pages_with_options(
					vec![1],
					GetPagesOptions {
						expected_head_txid: Some(1),
					},
				)
				.await
				.expect_err("Some(stale) must fence");
			assert!(
				proper_fence
					.chain()
					.any(|cause| cause.to_string().contains("head fence mismatch")),
				"unexpected error: {proper_fence:#}"
			);

			// Same stale instance, but now its state.head_txid was reset to
			// None by a process restart, an envoy reconnect, or any code path
			// that constructs a fresh `Db` (which initializes
			// cache_snapshot=None per db.rs:291).
			//
			// read.rs:134-149 only fences when expected is Some. With None the
			// stale instance silently receives the latest content (0xBB at
			// txid=2) without learning that another instance is the live
			// writer. Any commit it builds on top of these pages will diverge
			// from its own conceptual generation.
			let bypassed = db_stale
				.get_pages_with_options(
					vec![1],
					GetPagesOptions {
						expected_head_txid: None,
					},
				)
				.await?;
			assert_eq!(bypassed.head_txid, 2);
			assert_eq!(bypassed.pages, vec![fetched_page(1, 0xBB)]);

			Ok(())
		})
	})
	.await
}
