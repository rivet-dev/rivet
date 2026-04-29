//! Per-shard fold logic for compaction.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use universaldb::utils::IsolationLevel::Snapshot;

use crate::pump::{
	keys::{PAGE_SIZE, SHARD_SIZE, shard_key},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::DirtyPage,
};

pub async fn fold_shard(
	tx: &universaldb::Transaction,
	actor_id: &str,
	shard_id: u32,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<()> {
	let key = shard_key(actor_id, shard_id);
	let existing_blob = tx
		.informal()
		.get(&key, Snapshot)
		.await?
		.map(Vec::<u8>::from);

	let mut merged_pages = BTreeMap::<u32, Vec<u8>>::new();
	let mut header = None;
	if let Some(existing_blob) = existing_blob {
		let decoded = decode_ltx_v3(&existing_blob).context("decode existing shard blob")?;
		header = Some(decoded.header);
		for page in decoded.pages {
			if page.pgno / SHARD_SIZE == shard_id {
				ensure!(
					page.bytes.len() == PAGE_SIZE as usize,
					"page {} had {} bytes, expected {}",
					page.pgno,
					page.bytes.len(),
					PAGE_SIZE
				);
				merged_pages.insert(page.pgno, page.bytes);
			}
		}
	}

	for (pgno, bytes) in page_updates {
		ensure!(pgno > 0, "page number must be greater than zero");
		ensure!(
			pgno / SHARD_SIZE == shard_id,
			"page {} does not belong to shard {}",
			pgno,
			shard_id
		);
		ensure!(
			bytes.len() == PAGE_SIZE as usize,
			"page {} had {} bytes, expected {}",
			pgno,
			bytes.len(),
			PAGE_SIZE
		);
		merged_pages.insert(pgno, bytes);
	}

	let pages = merged_pages
		.into_iter()
		.map(|(pgno, bytes)| DirtyPage { pgno, bytes })
		.collect::<Vec<_>>();
	let commit = pages.iter().map(|page| page.pgno).max().unwrap_or(1);
	let header = header
		.map(|header| LtxHeader::delta(header.max_txid.max(1), commit, header.timestamp_ms))
		.unwrap_or_else(|| LtxHeader::delta(1, commit, 0));
	let encoded = encode_ltx_v3(header, &pages).context("encode folded shard blob")?;

	tx.informal().set(&key, &encoded);

	Ok(())
}
