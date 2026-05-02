//! In-memory page index support for delta lookups.

use anyhow::{Context, Result, ensure};
use scc::HashMap;
use std::sync::atomic::AtomicUsize;
use universaldb::Subspace;

use crate::udb;

const PGNO_BYTES: usize = std::mem::size_of::<u32>();
const TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Default)]
pub struct DeltaPageIndex {
	entries: HashMap<u32, u64>,
}

impl DeltaPageIndex {
	pub fn new() -> Self {
		Self {
			entries: HashMap::default(),
		}
	}

	pub async fn load_from_store(
		db: &universaldb::Database,
		subspace: &Subspace,
		op_counter: &AtomicUsize,
		prefix: Vec<u8>,
	) -> Result<Self> {
		let rows = udb::scan_prefix_values(db, subspace, op_counter, prefix.clone()).await?;
		let index = Self::new();

		for (key, value) in rows {
			let pgno = decode_pgno(&key, &prefix)?;
			let txid = decode_txid(&value)?;
			let _ = index.entries.upsert_sync(pgno, txid);
		}

		Ok(index)
	}

	pub fn get(&self, pgno: u32) -> Option<u64> {
		self.entries.read_sync(&pgno, |_, txid| *txid)
	}

	pub fn insert(&self, pgno: u32, txid: u64) {
		let _ = self.entries.upsert_sync(pgno, txid);
	}

	pub fn remove(&self, pgno: u32) -> Option<u64> {
		self.entries.remove_sync(&pgno).map(|(_, txid)| txid)
	}

	pub fn range(&self, start: u32, end: u32) -> Vec<(u32, u64)> {
		if start > end {
			return Vec::new();
		}

		let mut pages = Vec::new();
		self.entries.iter_sync(|pgno, txid| {
			if *pgno >= start && *pgno <= end {
				pages.push((*pgno, *txid));
			}
			true
		});
		pages.sort_unstable_by_key(|(pgno, _)| *pgno);
		pages
	}
}

fn decode_pgno(key: &[u8], prefix: &[u8]) -> Result<u32> {
	ensure!(
		key.starts_with(prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::DeltaPageIndex;
	use crate::keys::{pidx_delta_key, pidx_delta_prefix};
	use crate::test_utils::test_db;
	use crate::udb::{WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

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
	async fn load_from_store_reads_sorted_scan_prefix_entries() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		apply_write_ops(
			&db,
			&subspace,
			counter.as_ref(),
			vec![
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 8), 81_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 21_u64.to_be_bytes().to_vec()),
				WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 17),
					171_u64.to_be_bytes().to_vec(),
				),
			],
		)
		.await?;

		let prefix = pidx_delta_prefix(TEST_ACTOR);
		counter.store(0, std::sync::atomic::Ordering::SeqCst);
		let index =
			DeltaPageIndex::load_from_store(&db, &subspace, counter.as_ref(), prefix.clone())
				.await?;

		assert_eq!(index.get(2), Some(21));
		assert_eq!(index.get(8), Some(81));
		assert_eq!(index.get(17), Some(171));
		assert_eq!(index.range(1, 20), vec![(2, 21), (8, 81), (17, 171)]);
		assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

		Ok(())
	}
}
