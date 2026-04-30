use std::{collections::BTreeSet, sync::Arc};

use anyhow::{Context, Result, bail};

use crate::{
	cold_tier::ColdTier,
	gc::{VERSIONSTAMP_ZERO, read_branch_gc_pin_tx},
	pump::{
		types::{
			ActorBranchId, ColdManifestChunk, ColdManifestChunkRef, decode_cold_manifest_chunk,
			decode_cold_manifest_index, encode_cold_manifest_chunk, encode_cold_manifest_index,
		},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColdSweepOutput {
	pub removed_chunks: usize,
	pub removed_layers: usize,
	pub deleted_objects: usize,
}

pub(crate) async fn run(
	db: &universaldb::Database,
	cold_tier: Arc<dyn ColdTier>,
	branch_id: ActorBranchId,
	cancel_token: tokio_util::sync::CancellationToken,
) -> Result<ColdSweepOutput> {
	ensure_not_cancelled(&cancel_token)?;

	let Some(gc_pin) = read_gc_pin(db, branch_id).await? else {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	};
	if gc_pin == VERSIONSTAMP_ZERO {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	}

	let index_key = manifest_index_object_key(branch_id);
	let Some(index_bytes) = cold_tier.get_object(&index_key).await? else {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	};
	let mut index = decode_cold_manifest_index(&index_bytes)
		.with_context(|| format!("decode sqlite cold manifest index {index_key}"))?;

	let mut retained_refs = Vec::with_capacity(index.chunks.len());
	let mut chunk_rewrites = Vec::new();
	let mut delete_keys = BTreeSet::new();
	let mut removed_chunks = 0;
	let mut removed_layers = 0;

	for chunk_ref in &index.chunks {
		ensure_not_cancelled(&cancel_token)?;

		let Some(chunk_bytes) = cold_tier.get_object(&chunk_ref.object_key).await? else {
			continue;
		};
		let chunk = decode_cold_manifest_chunk(&chunk_bytes)
			.with_context(|| format!("decode sqlite cold manifest chunk {}", chunk_ref.object_key))?;
		let obsolete_layer_keys = chunk
			.layers
			.iter()
			.filter(|layer| {
				layer.max_versionstamp != VERSIONSTAMP_ZERO && layer.max_versionstamp < gc_pin
			})
			.map(|layer| layer.object_key.clone())
			.collect::<BTreeSet<_>>();

		if obsolete_layer_keys.is_empty() {
			retained_refs.push(chunk_ref.clone());
			continue;
		}

		removed_layers += obsolete_layer_keys.len();
		delete_keys.extend(obsolete_layer_keys.iter().cloned());

		if chunk.layers.iter().all(|layer| {
			layer.max_versionstamp != VERSIONSTAMP_ZERO && layer.max_versionstamp < gc_pin
		}) {
			removed_chunks += 1;
			delete_keys.insert(chunk_ref.object_key.clone());
			continue;
		}

		let retained_chunk = retain_manifest_chunk(chunk, &obsolete_layer_keys);
		if retained_chunk.layers.is_empty() {
			removed_chunks += 1;
			delete_keys.insert(chunk_ref.object_key.clone());
			continue;
		}
		let retained_bytes = encode_cold_manifest_chunk(retained_chunk.clone())?;
		retained_refs.push(ColdManifestChunkRef {
			object_key: chunk_ref.object_key.clone(),
			pass_versionstamp: chunk_ref.pass_versionstamp,
			min_versionstamp: min_layer_versionstamp(&retained_chunk),
			max_versionstamp: max_layer_versionstamp(&retained_chunk),
			byte_size: retained_bytes.len() as u64,
		});
		chunk_rewrites.push((chunk_ref.object_key.clone(), retained_bytes));
	}

	if delete_keys.is_empty() && retained_refs.len() == index.chunks.len() {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	}

	let Some(current_pin) = read_gc_pin(db, branch_id).await? else {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	};
	if current_pin < gc_pin {
		return Ok(ColdSweepOutput {
			removed_chunks: 0,
			removed_layers: 0,
			deleted_objects: 0,
		});
	}

	for (chunk_key, chunk_bytes) in chunk_rewrites {
		cold_tier
			.put_object(&chunk_key, &chunk_bytes)
			.await
			.with_context(|| format!("rewrite sqlite cold manifest chunk {chunk_key}"))?;
	}

	index.chunks = retained_refs;
	cold_tier
		.put_object(&index_key, &encode_cold_manifest_index(index)?)
		.await
		.with_context(|| format!("rewrite sqlite cold manifest index {index_key}"))?;

	let delete_keys = delete_keys.into_iter().collect::<Vec<_>>();
	cold_tier.delete_objects(&delete_keys).await?;

	Ok(ColdSweepOutput {
		removed_chunks,
		removed_layers,
		deleted_objects: delete_keys.len(),
	})
}

async fn read_gc_pin(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
) -> Result<Option<[u8; 16]>> {
	db.run(move |tx| async move {
		Ok(read_branch_gc_pin_tx(&tx, branch_id)
			.await?
			.map(|pin| pin.gc_pin))
	})
	.await
}

fn retain_manifest_chunk(
	mut chunk: ColdManifestChunk,
	obsolete_layer_keys: &BTreeSet<String>,
) -> ColdManifestChunk {
	chunk
		.layers
		.retain(|layer| !obsolete_layer_keys.contains(&layer.object_key));
	chunk.bookmarks.retain(|bookmark| {
		bookmark
			.pin_object_key
			.as_ref()
			.is_none_or(|key| !obsolete_layer_keys.contains(key))
	});
	chunk
}

fn min_layer_versionstamp(chunk: &ColdManifestChunk) -> [u8; 16] {
	chunk
		.layers
		.iter()
		.map(|layer| layer.min_versionstamp)
		.min()
		.unwrap_or(chunk.pass_versionstamp)
}

fn max_layer_versionstamp(chunk: &ColdManifestChunk) -> [u8; 16] {
	chunk
		.layers
		.iter()
		.map(|layer| layer.max_versionstamp)
		.max()
		.unwrap_or(chunk.pass_versionstamp)
}

fn manifest_index_object_key(branch_id: ActorBranchId) -> String {
	format!("{}/cold_manifest/index.bare", branch_object_prefix(branch_id))
}

fn branch_object_prefix(branch_id: ActorBranchId) -> String {
	format!("db/{}", branch_id.as_uuid().simple())
}

fn ensure_not_cancelled(cancel_token: &tokio_util::sync::CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite cold compaction cancelled");
	}

	Ok(())
}
