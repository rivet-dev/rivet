//! Per-shard fold logic for compaction.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Snapshot, end_of_key_range},
};

use crate::pump::{
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::{DatabaseBranchId, DirtyPage},
};

pub async fn fold_shard(
	tx: &universaldb::Transaction,
	database_id: &str,
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<i64> {
	fold_shard_inner(
		tx,
		ShardScope::Legacy { database_id },
		shard_id,
		as_of_txid,
		page_updates,
	)
	.await
}

pub async fn fold_branch_shard(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<i64> {
	fold_shard_inner(
		tx,
		ShardScope::Branch { branch_id },
		shard_id,
		as_of_txid,
		page_updates,
	)
	.await
}

async fn fold_shard_inner<'a>(
	tx: &'a universaldb::Transaction,
	scope: ShardScope<'a>,
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<i64> {
	let key = scope.shard_key(shard_id, as_of_txid);
	let previous_version_blob = tx
		.informal()
		.get(&key, Snapshot)
		.await?
		.map(Vec::<u8>::from);
	let existing_blob = scope
		.load_latest_shard_blob(tx, shard_id, as_of_txid)
		.await?;

	let mut merged_pages = BTreeMap::<u32, Vec<u8>>::new();
	let mut timestamp_ms = 0;
	if let Some(existing_blob) = existing_blob {
		let decoded = decode_ltx_v3(&existing_blob).context("decode existing shard blob")?;
		timestamp_ms = decoded.header.timestamp_ms;
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
	let header = LtxHeader::delta(as_of_txid, commit, timestamp_ms);
	let encoded = encode_ltx_v3(header, &pages).context("encode folded shard blob")?;
	let bytes_added = tracked_entry_size(&key, &encoded)?
		- previous_version_blob
			.as_ref()
			.map_or(Ok(0), |blob| tracked_entry_size(&key, blob))?;

	tx.informal().set(&key, &encoded);

	Ok(bytes_added)
}

#[derive(Clone, Copy)]
enum ShardScope<'a> {
	Branch { branch_id: DatabaseBranchId },
	Legacy { database_id: &'a str },
}

impl ShardScope<'_> {
	fn shard_key(self, shard_id: u32, as_of_txid: u64) -> Vec<u8> {
		match self {
			Self::Branch { branch_id } => keys::branch_shard_key(branch_id, shard_id, as_of_txid),
			Self::Legacy { database_id } => keys::shard_version_key(database_id, shard_id, as_of_txid),
		}
	}

	async fn load_latest_shard_blob(
		self,
		tx: &universaldb::Transaction,
		shard_id: u32,
		as_of_txid: u64,
	) -> Result<Option<Vec<u8>>> {
		match self {
			Self::Branch { branch_id } => {
				load_latest_branch_shard_blob(tx, branch_id, shard_id, as_of_txid).await
			}
			Self::Legacy { database_id } => {
				load_latest_legacy_shard_blob(tx, database_id, shard_id, as_of_txid).await
			}
		}
	}
}

async fn load_latest_legacy_shard_blob(
	tx: &universaldb::Transaction,
	database_id: &str,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<Vec<u8>>> {
	let prefix = keys::shard_version_prefix(database_id, shard_id);
	let end = end_of_key_range(&keys::shard_version_key(database_id, shard_id, as_of_txid));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(prefix.as_slice(), end.as_slice()).into()
		},
		Snapshot,
	);

	let mut latest = None;
	while let Some(entry) = stream.try_next().await? {
		latest = Some(entry.value().to_vec());
	}

	if latest.is_some() {
		return Ok(latest);
	}

	Ok(tx
		.informal()
		.get(&keys::shard_key(database_id, shard_id), Snapshot)
		.await?
		.map(Vec::<u8>::from))
}

async fn load_latest_branch_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<Vec<u8>>> {
	let prefix = keys::branch_shard_version_prefix(branch_id, shard_id);
	let end = end_of_key_range(&keys::branch_shard_key(branch_id, shard_id, as_of_txid));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(prefix.as_slice(), end.as_slice()).into()
		},
		Snapshot,
	);

	let mut latest = None;
	while let Some(entry) = stream.try_next().await? {
		latest = Some(entry.value().to_vec());
	}

	Ok(latest)
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}
