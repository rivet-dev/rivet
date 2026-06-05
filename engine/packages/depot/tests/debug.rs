mod common;

use anyhow::Result;
use depot::{
	debug,
	keys::{PAGE_SIZE, branch_commit_key},
	types::{DatabaseBranchId, DirtyPage, decode_commit_row},
};
use universaldb::utils::IsolationLevel::Snapshot;

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

async fn commit_row(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<depot::types::CommitRow> {
	db.txn("test_depotdebug", move |tx| async move {
		let bytes = tx
			.informal()
			.get(&branch_commit_key(branch_id, txid), Snapshot)
			.await?
			.expect("commit row should exist");

		decode_commit_row(&bytes)
	})
	.await
}

#[tokio::test]
async fn debug_dumps_ancestry_pins_restore_points_and_gc_pin() -> Result<()> {
	common::test_matrix("depot-debug-ancestry", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.db;

			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
			let first_commit = commit_row(&db, branch_id, 1).await?;
			let pinned = database_db
				.create_restore_point(depot::types::SnapshotSelector::Latest)
				.await?;
			database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;

			let ancestry = debug::dump_database_ancestry(&database_db).await?;
			assert_eq!(ancestry, vec![(branch_id, None)]);

			let pins = debug::dump_branch_pins(&database_db).await?;
			assert_eq!(pins.branch_id, branch_id);
			assert_eq!(pins.refcount, 1);
			assert_eq!(pins.restore_point_pin, first_commit.versionstamp);

			let restore_points = debug::list_restore_points(&database_db).await?;
			assert!(restore_points.iter().any(|entry| {
				entry.restore_point_id == pinned
					&& entry.pin_status == depot::types::PinStatus::Ready
			}));

			assert_eq!(
				debug::estimate_gc_pin(&database_db).await?,
				first_commit.versionstamp
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn debug_read_at_returns_page_state_for_versionstamp() -> Result<()> {
	common::test_matrix("depot-debug-read-at", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_db = ctx.db;

			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let branch_id = debug::dump_database_ancestry(&database_db).await?[0].0;
			let first_commit = commit_row(&db, branch_id, 1).await?;
			database_db
				.commit(vec![page(1, 0x22), page(2, 0x33)], 3, 2_000)
				.await?;
			let second_commit = commit_row(&db, branch_id, 2).await?;

			let first_state = debug::read_at(&database_db, first_commit.versionstamp).await?;
			assert_eq!(first_state.txid, 1);
			assert_eq!(first_state.db_size_pages, 2);
			assert_eq!(
				first_state.pages[0].bytes.as_deref(),
				Some(&vec![0x11; PAGE_SIZE as usize][..])
			);
			assert_eq!(
				first_state.pages[1].bytes.as_deref(),
				Some(&vec![0; PAGE_SIZE as usize][..])
			);

			let second_state = debug::read_at(&database_db, second_commit.versionstamp).await?;
			assert_eq!(second_state.txid, 2);
			assert_eq!(second_state.db_size_pages, 3);
			assert_eq!(
				second_state.pages[0].bytes.as_deref(),
				Some(&vec![0x22; PAGE_SIZE as usize][..])
			);
			assert_eq!(
				second_state.pages[1].bytes.as_deref(),
				Some(&vec![0x33; PAGE_SIZE as usize][..])
			);

			Ok(())
		})
	})
	.await
}
