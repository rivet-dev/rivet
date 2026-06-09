use anyhow::{Context, Result, ensure};
use futures_util::{TryStreamExt, future::try_join_all};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Serializable, end_of_key_range},
};

use crate::conveyer::{db::LtxBlobCache, keys};

use super::plan::{ReadSource, StorageScope};

pub(super) async fn tx_load_delta_blob(
	tx: &universaldb::Transaction,
	delta_prefix: &[u8],
	cache: &LtxBlobCache,
) -> Result<DeltaBlobLoad> {
	// Serve immutable delta blobs from the per-database cache without re-fetching
	// from FDB. The delta prefix includes the owning txid, so the key uniquely
	// identifies immutable content.
	if let Some(cached) = cache.get(delta_prefix).await {
		return Ok(DeltaBlobLoad {
			blob: Some(cached.bytes().to_vec()),
			chunk_rows_scanned: 0,
		});
	}

	let delta_chunks = super::tx::tx_scan_prefix_values(tx, delta_prefix).await?;
	if delta_chunks.is_empty() {
		return Ok(DeltaBlobLoad {
			blob: None,
			chunk_rows_scanned: 0,
		});
	}
	let chunk_rows_scanned = delta_chunks.len();

	// FDB returns rows in key order, and the chunk index is the big-endian u32
	// key suffix, so the natural scan order already matches chunk order. The
	// contiguity check below relies on that order without re-sorting.
	let mut delta_blob = Vec::new();
	for (expected_idx, (key, chunk)) in delta_chunks.into_iter().enumerate() {
		let chunk_idx = decode_delta_chunk_idx(delta_prefix, &key)?;
		ensure!(
			chunk_idx == u32::try_from(expected_idx).unwrap_or(u32::MAX),
			"sqlite delta chunks must be contiguous from chunk 0"
		);
		delta_blob.extend_from_slice(&chunk);
	}

	Ok(DeltaBlobLoad {
		blob: Some(delta_blob),
		chunk_rows_scanned,
	})
}

pub(super) struct DeltaBlobLoad {
	pub(super) blob: Option<Vec<u8>>,
	pub(super) chunk_rows_scanned: usize,
}

fn decode_delta_chunk_idx(delta_prefix: &[u8], key: &[u8]) -> Result<u32> {
	let suffix = key
		.strip_prefix(delta_prefix)
		.context("sqlite delta chunk key did not start with expected prefix")?;
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"sqlite delta chunk key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(suffix.try_into().context(
		"sqlite delta chunk suffix should decode as u32",
	)?))
}

pub(super) async fn tx_load_latest_shard_blob(
	tx: &universaldb::Transaction,
	scope: &StorageScope,
	shard_id: u32,
) -> Result<ShardBlobLoad> {
	let StorageScope::Branch(plan) = scope;

	// Scan every source's latest shard version concurrently. Each scan is a
	// reverse range limited to one row, so it reads only the newest version at or
	// below the source's cap instead of streaming every historical version.
	let per_source = try_join_all(
		plan.sources
			.iter()
			.map(|source| tx_load_source_shard_blob(tx, *source, shard_id)),
	)
	.await?;

	// Sources are ordered most specific first, so the first source with a hit
	// wins, matching the sequential fallback order.
	let mut rows_scanned = 0usize;
	let mut latest = None;
	for (source, found) in per_source {
		rows_scanned += found;
		if latest.is_none() && source.is_some() {
			latest = source;
		}
	}

	Ok(ShardBlobLoad {
		source: latest,
		rows_scanned,
	})
}

async fn tx_load_source_shard_blob(
	tx: &universaldb::Transaction,
	source: ReadSource,
	shard_id: u32,
) -> Result<(Option<(Vec<u8>, Vec<u8>)>, usize)> {
	let ReadSource::Branch(source) = source;
	let prefix = keys::branch_shard_version_prefix(source.branch_id, shard_id);
	let end_key = keys::branch_shard_key(source.branch_id, shard_id, source.max_txid);
	let end = end_of_key_range(&end_key);

	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			reverse: true,
			limit: Some(1),
			..(prefix.as_slice(), end.as_slice()).into()
		},
		// TODO: This can probably be made Snapshot again to reduce contention if
		// read side freshness is not worth the cost.
		Serializable,
	);

	let mut rows_scanned = 0usize;
	let mut latest = None;
	while let Some(entry) = stream.try_next().await? {
		rows_scanned += 1;
		latest = Some((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok((latest, rows_scanned))
}

pub(super) struct ShardBlobLoad {
	pub(super) source: Option<(Vec<u8>, Vec<u8>)>,
	pub(super) rows_scanned: usize,
}
