use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use universaldb::utils::IsolationLevel;

use super::{
	constants::DELTA_OBJECT_CHUNK_BYTES,
	keys,
	types::{
		DatabaseBranchId, HotShardManifest, decode_hot_shard_manifest,
		encode_hot_shard_manifest,
	},
};

pub fn write_branch_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	blob: &[u8],
) -> Result<()> {
	let shard_key = keys::branch_shard_key(branch_id, shard_id, as_of_txid);
	if blob.len() <= DELTA_OBJECT_CHUNK_BYTES {
		tx.informal().set(&shard_key, blob);
		return Ok(());
	}

	let content_hash = content_hash(blob);
	let chunks = blob
		.chunks(DELTA_OBJECT_CHUNK_BYTES)
		.enumerate()
		.map(|(chunk_idx, chunk)| {
			Ok((
				u32::try_from(chunk_idx).context("sqlite hot shard chunk index exceeded u32")?,
				chunk,
			))
		})
		.collect::<Result<Vec<_>>>()?;
	let manifest = HotShardManifest {
		shard_id,
		as_of_txid,
		chunk_count: u32::try_from(chunks.len())
			.context("sqlite hot shard chunk count exceeded u32")?,
		encoded_len: u64::try_from(blob.len()).unwrap_or(u64::MAX),
		content_hash,
	};
	let manifest_bytes = encode_hot_shard_manifest(manifest)?;

	tx.informal().set(&shard_key, &manifest_bytes);
	for (chunk_idx, chunk) in chunks {
		tx.informal().set(
			&keys::branch_shard_chunk_key(branch_id, shard_id, as_of_txid, chunk_idx),
			chunk,
		);
	}

	Ok(())
}

pub async fn resolve_branch_shard_value(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	value: &[u8],
	isolation_level: IsolationLevel,
) -> Result<Vec<u8>> {
	let Ok(manifest) = decode_hot_shard_manifest(value) else {
		return Ok(value.to_vec());
	};
	ensure!(
		manifest.shard_id == shard_id && manifest.as_of_txid == as_of_txid,
		"sqlite hot shard manifest key did not match row key"
	);

	let mut blob = Vec::new();
	for chunk_idx in 0..manifest.chunk_count {
		let chunk_key = keys::branch_shard_chunk_key(branch_id, shard_id, as_of_txid, chunk_idx);
		let chunk = tx
			.informal()
			.get(&chunk_key, isolation_level)
			.await?
			.with_context(|| {
				format!(
					"sqlite hot shard chunk missing for shard {} txid {} chunk {}",
					shard_id, as_of_txid, chunk_idx
				)
			})?;
		blob.extend_from_slice(&chunk);
	}

	ensure!(
		manifest.encoded_len == u64::try_from(blob.len()).unwrap_or(u64::MAX),
		"sqlite hot shard chunk length mismatch"
	);
	ensure!(
		manifest.content_hash == content_hash(&blob),
		"sqlite hot shard chunk hash mismatch"
	);

	Ok(blob)
}

pub fn clear_branch_shard_chunks(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
	row_value: &[u8],
) -> Result<()> {
	let Ok(manifest) = decode_hot_shard_manifest(row_value) else {
		return Ok(());
	};
	ensure!(
		manifest.shard_id == shard_id && manifest.as_of_txid == as_of_txid,
		"sqlite hot shard manifest key did not match row key"
	);

	for chunk_idx in 0..manifest.chunk_count {
		tx.informal()
			.clear(&keys::branch_shard_chunk_key(
				branch_id, shard_id, as_of_txid, chunk_idx,
			));
	}

	Ok(())
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}
