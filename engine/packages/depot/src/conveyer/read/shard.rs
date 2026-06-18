use anyhow::{Context, Result, bail, ensure};
use futures_util::TryStreamExt;
use sha2::{Digest, Sha256};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Snapshot, end_of_key_range},
};

use crate::conveyer::{
	constants::DELTA_OBJECT_CHUNK_BYTES,
	keys,
	ltx::decode_ltx_page_frame,
	shard_blob::resolve_branch_shard_value,
	types::{decode_delta_manifest, decode_delta_page_index_entry},
};

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

pub(super) async fn tx_load_large_delta_page(
	tx: &universaldb::Transaction,
	source: ReadSource,
	txid: u64,
	pgno: u32,
) -> Result<Option<Vec<u8>>> {
	let ReadSource::Branch(source) = source;
	let manifest_key = keys::branch_delta_manifest_key(source.branch_id, txid);
	let Some(manifest_bytes) = super::tx::tx_get_value(tx, &manifest_key).await? else {
		if super::tx::tx_get_value(
			tx,
			&keys::branch_delta_pageidx_key(source.branch_id, txid, pgno),
		)
		.await?
		.is_some()
		{
			bail!("sqlite large delta manifest missing for txid {txid} page {pgno}");
		}
		return Ok(None);
	};
	let manifest = decode_delta_manifest(&manifest_bytes)
		.with_context(|| format!("decode sqlite large delta manifest {txid}"))?;
	ensure!(
		manifest.txid == txid,
		"sqlite large delta manifest txid {} did not match key txid {}",
		manifest.txid,
		txid
	);

	let page_index_bytes = super::tx::tx_get_value(
		tx,
		&keys::branch_delta_pageidx_key(source.branch_id, txid, pgno),
	)
	.await?
	.with_context(|| format!("sqlite large delta page index missing for txid {txid} page {pgno}"))?;
	let page_index = decode_delta_page_index_entry(&page_index_bytes)
		.with_context(|| format!("decode sqlite large delta page index for txid {txid} page {pgno}"))?;
	ensure!(
		page_index.txid == txid,
		"sqlite large delta page index txid {} did not match key txid {}",
		page_index.txid,
		txid
	);
	ensure!(
		page_index.object_id == manifest.object_id,
		"sqlite large delta page index object did not match manifest object"
	);

	let encoded_size = page_index.encoded_size as u64;
	let encoded_end = page_index
		.encoded_offset
		.checked_add(encoded_size)
		.context("sqlite large delta page byte range overflowed")?;
	ensure!(
		encoded_end <= manifest.encoded_len,
		"sqlite large delta page byte range exceeded object length"
	);

	let first_chunk = page_index.encoded_offset / DELTA_OBJECT_CHUNK_BYTES as u64;
	let last_chunk = (encoded_end.saturating_sub(1)) / DELTA_OBJECT_CHUNK_BYTES as u64;
	ensure!(
		last_chunk < u64::from(manifest.chunk_count),
		"sqlite large delta page byte range exceeded object chunk count"
	);

	let mut object_bytes = Vec::new();
	for chunk_idx in first_chunk..=last_chunk {
		let chunk_idx = u32::try_from(chunk_idx).context("sqlite large delta chunk index exceeded u32")?;
		let chunk = super::tx::tx_get_value(
			tx,
			&keys::branch_delta_object_chunk_key(
				source.branch_id,
				manifest.object_id,
				chunk_idx,
			),
		)
		.await?
		.with_context(|| {
			format!("sqlite large delta object chunk missing for txid {txid} chunk {chunk_idx}")
		})?;
		object_bytes.extend_from_slice(&chunk);
	}

	let chunk_base_offset = first_chunk * DELTA_OBJECT_CHUNK_BYTES as u64;
	let slice_start = usize::try_from(page_index.encoded_offset - chunk_base_offset)
		.context("sqlite large delta slice start exceeded usize")?;
	let slice_len =
		usize::try_from(encoded_size).context("sqlite large delta slice length exceeded usize")?;
	let slice_end = slice_start
		.checked_add(slice_len)
		.context("sqlite large delta slice end overflowed")?;
	ensure!(
		slice_end <= object_bytes.len(),
		"sqlite large delta page byte range exceeded fetched chunks"
	);

	let page = decode_ltx_page_frame(
		&object_bytes[slice_start..slice_end],
		pgno,
		keys::PAGE_SIZE,
	)
	.with_context(|| format!("decode sqlite large delta page frame for txid {txid} page {pgno}"))?;
	let digest = Sha256::digest(&page.bytes);
	ensure!(
		digest.as_slice() == page_index.page_hash,
		"sqlite large delta page hash mismatch for txid {} page {}",
		txid,
		pgno
	);

	Ok(Some(page.bytes))
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
		let ReadSource::Branch(source) = source;
		let as_of_txid = source.max_txid;
		let prefix = keys::branch_shard_version_prefix(source.branch_id, shard_id);
		let end_key = keys::branch_shard_key(source.branch_id, shard_id, as_of_txid);
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

		if let Some((key, value)) = latest {
			let latest_as_of_txid = decode_branch_shard_as_of_txid(source.branch_id, shard_id, &key)?;
			let blob =
				resolve_branch_shard_value(
					tx,
					source.branch_id,
					shard_id,
					latest_as_of_txid,
					&value,
					Snapshot,
				)
				.await?;
			return Ok(Some((key, blob)));
		}
	}

	Ok(None)
}

fn decode_branch_shard_as_of_txid(
	branch_id: crate::conveyer::types::DatabaseBranchId,
	shard_id: u32,
	key: &[u8],
) -> Result<u64> {
	let prefix = keys::branch_shard_version_prefix(branch_id, shard_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("sqlite branch shard key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix.try_into().with_context(|| {
		format!(
			"sqlite branch shard key suffix had {} bytes, expected {}",
			suffix.len(),
			std::mem::size_of::<u64>()
		)
	})?;

	Ok(u64::from_be_bytes(bytes))
}
