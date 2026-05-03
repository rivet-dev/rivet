use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Snapshot, end_of_key_range},
};

use crate::conveyer::keys;

use super::plan::{ReadSource, StorageScope};

pub(super) async fn tx_load_delta_blob(
	tx: &universaldb::Transaction,
	delta_prefix: &[u8],
) -> Result<Option<Vec<u8>>> {
	let mut delta_chunks = super::tx::tx_scan_prefix_values(tx, delta_prefix).await?;
	if delta_chunks.is_empty() {
		return Ok(None);
	}
	delta_chunks.sort_by_key(|(key, _)| key.clone());

	let mut delta_blob = Vec::new();
	for (expected_idx, (key, chunk)) in delta_chunks.into_iter().enumerate() {
		let chunk_idx = decode_delta_chunk_idx(delta_prefix, &key)?;
		ensure!(
			chunk_idx == u32::try_from(expected_idx).unwrap_or(u32::MAX),
			"sqlite delta chunks must be contiguous from chunk 0"
		);
		delta_blob.extend_from_slice(&chunk);
	}

	Ok(Some(delta_blob))
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
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	let sources = match scope {
		StorageScope::Branch(plan) => plan.sources.clone(),
	};

	for source in sources {
		let as_of_txid = match source {
			ReadSource::Branch(source) => source.max_txid,
		};
		let prefix = match source {
			ReadSource::Branch(source) => {
				keys::branch_shard_version_prefix(source.branch_id, shard_id)
			}
		};
		let end_key = match source {
			ReadSource::Branch(source) => {
				keys::branch_shard_key(source.branch_id, shard_id, as_of_txid)
			}
		};
		let end = end_of_key_range(&end_key);
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
			latest = Some((entry.key().to_vec(), entry.value().to_vec()));
		}

		if latest.is_some() {
			return Ok(latest);
		}
	}

	Ok(None)
}
