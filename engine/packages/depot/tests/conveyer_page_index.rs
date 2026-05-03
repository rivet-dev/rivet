mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use depot::{
	keys::{pidx_delta_key, pidx_delta_prefix},
	page_index::DeltaPageIndex,
};
use universaldb::Subspace;
use uuid::Uuid;

const TEST_DATABASE: &str = "test-database";

#[test]
fn insert_get_and_remove_round_trip() {
	let index = DeltaPageIndex::new();

	assert_eq!(index.get(7), None);

	index.insert(7, 11);
	index.insert(9, 15);

	assert_eq!(index.get(7), Some(11));
	assert_eq!(index.get(9), Some(15));
	assert_eq!(index.remove(7), Some(11));
	assert_eq!(index.get(7), None);
	assert_eq!(index.remove(99), None);
}

#[test]
fn insert_overwrites_existing_txid() {
	let index = DeltaPageIndex::new();

	index.insert(4, 20);
	index.insert(4, 21);

	assert_eq!(index.get(4), Some(21));
}

#[test]
fn range_returns_sorted_pages_within_bounds() {
	let index = DeltaPageIndex::new();
	index.insert(12, 1200);
	index.insert(3, 300);
	index.insert(7, 700);
	index.insert(15, 1500);

	assert_eq!(index.range(4, 12), vec![(7, 700), (12, 1200)]);
	assert_eq!(index.range(20, 10), Vec::<(u32, u64)>::new());
}

#[tokio::test]
async fn load_from_store_reads_scan_prefix_entries() -> Result<()> {
	common::test_matrix("depot-page-index-load", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let subspace = Subspace::new(&("depot-page-index", Uuid::new_v4().to_string()));
			db.run({
				let subspace = subspace.clone();
				move |tx| {
					let subspace = subspace.clone();
					async move {
						let key = |logical_key: Vec<u8>| {
							[subspace.bytes(), logical_key.as_slice()].concat()
						};
						tx.informal().set(
							&key(pidx_delta_key(TEST_DATABASE, 8)),
							&81_u64.to_be_bytes(),
						);
						tx.informal().set(
							&key(pidx_delta_key(TEST_DATABASE, 2)),
							&21_u64.to_be_bytes(),
						);
						tx.informal().set(
							&key(pidx_delta_key(TEST_DATABASE, 17)),
							&171_u64.to_be_bytes(),
						);
						tx.informal().set(
							&key(pidx_delta_key("other-database", 2)),
							&999_u64.to_be_bytes(),
						);
						Ok(())
					}
				}
			})
			.await?;

			let counter = AtomicUsize::new(0);
			let index = DeltaPageIndex::load_from_store(
				&db,
				&subspace,
				&counter,
				pidx_delta_prefix(TEST_DATABASE),
			)
			.await?;

			assert_eq!(index.get(2), Some(21));
			assert_eq!(index.get(8), Some(81));
			assert_eq!(index.get(17), Some(171));
			assert_eq!(index.range(1, 20), vec![(2, 21), (8, 81), (17, 171)]);
			assert_eq!(counter.load(Ordering::SeqCst), 1);

			Ok(())
		})
	})
	.await
}
