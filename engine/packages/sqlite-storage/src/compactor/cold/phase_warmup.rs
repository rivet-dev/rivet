use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::{
	cold_tier::ColdTier,
	pump::types::{
		ActorBranchId, ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex, LayerKind,
		SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_cold_manifest_chunk,
		decode_cold_manifest_index, encode_cold_manifest_chunk, encode_cold_manifest_index,
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColdForkWarmupOutput {
	pub copied_layers: usize,
	pub source_chunks_read: usize,
}

pub(crate) async fn run_actor(
	cold_tier: Arc<dyn ColdTier>,
	source_branch_id: ActorBranchId,
	target_branch_id: ActorBranchId,
	at_versionstamp: [u8; 16],
	cancel_token: tokio_util::sync::CancellationToken,
	now_ms: i64,
) -> Result<ColdForkWarmupOutput> {
	ensure_not_cancelled(&cancel_token)?;

	let Some(source_index_bytes) = cold_tier
		.get_object(&manifest_index_object_key(source_branch_id))
		.await
		.with_context(|| {
			format!(
				"get sqlite fork warmup source manifest for {}",
				source_branch_id.as_uuid()
			)
		})?
	else {
		return Ok(ColdForkWarmupOutput {
			copied_layers: 0,
			source_chunks_read: 0,
		});
	};
	let source_index = decode_cold_manifest_index(&source_index_bytes)
		.context("decode sqlite fork warmup source manifest")?;
	let pass_uuid = uuid::Uuid::new_v4();
	let mut warm_layers = Vec::new();
	let mut source_chunks_read = 0;

	for chunk_ref in source_index.chunks {
		ensure_not_cancelled(&cancel_token)?;
		let Some(chunk_bytes) = cold_tier
			.get_object(&chunk_ref.object_key)
			.await
			.with_context(|| format!("get sqlite fork warmup source chunk {}", chunk_ref.object_key))?
		else {
			continue;
		};
		source_chunks_read += 1;
		let chunk = decode_cold_manifest_chunk(&chunk_bytes)
			.with_context(|| format!("decode sqlite fork warmup source chunk {}", chunk_ref.object_key))?;

		for mut layer in chunk.layers {
			if layer.kind != LayerKind::Image || layer.max_versionstamp > at_versionstamp {
				continue;
			}

			ensure_not_cancelled(&cancel_token)?;
			let Some(layer_bytes) = cold_tier
				.get_object(&layer.object_key)
				.await
				.with_context(|| format!("get sqlite fork warmup source layer {}", layer.object_key))?
			else {
				continue;
			};
			let target_key =
				warm_layer_object_key(source_branch_id, target_branch_id, pass_uuid, &layer.object_key)?;
			cold_tier
				.put_object(&target_key, &layer_bytes)
				.await
				.with_context(|| format!("put sqlite fork warmup layer {target_key}"))?;
			layer.object_key = target_key;
			layer.byte_size = layer_bytes.len() as u64;
			warm_layers.push(layer);
		}
	}

	if warm_layers.is_empty() {
		return Ok(ColdForkWarmupOutput {
			copied_layers: 0,
			source_chunks_read,
		});
	}

	let pass_versionstamp = warm_layers
		.iter()
		.map(|layer| layer.max_versionstamp)
		.max()
		.unwrap_or(at_versionstamp);
	let warm_chunk = ColdManifestChunk {
		schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
		branch_id: target_branch_id,
		pass_versionstamp,
		layers: warm_layers,
		bookmarks: Vec::new(),
	};
	let warm_chunk_key = warm_manifest_chunk_object_key(target_branch_id, pass_uuid);
	let warm_chunk_bytes = encode_cold_manifest_chunk(warm_chunk.clone())?;

	ensure_not_cancelled(&cancel_token)?;
	cold_tier
		.put_object(&warm_chunk_key, &warm_chunk_bytes)
		.await
		.with_context(|| format!("put sqlite fork warmup manifest chunk {warm_chunk_key}"))?;

	let target_index_key = manifest_index_object_key(target_branch_id);
	let mut target_index =
		load_target_manifest_index(cold_tier.as_ref(), target_branch_id, &target_index_key).await?;
	target_index.chunks.push(ColdManifestChunkRef {
		object_key: warm_chunk_key.clone(),
		pass_versionstamp,
		min_versionstamp: min_layer_versionstamp(&warm_chunk),
		max_versionstamp: max_layer_versionstamp(&warm_chunk),
		byte_size: warm_chunk_bytes.len() as u64,
	});
	target_index.last_pass_at_ms = now_ms;
	target_index.last_pass_versionstamp = pass_versionstamp;

	ensure_not_cancelled(&cancel_token)?;
	cold_tier
		.put_object(&target_index_key, &encode_cold_manifest_index(target_index)?)
		.await
		.with_context(|| format!("put sqlite fork warmup manifest index {target_index_key}"))?;

	Ok(ColdForkWarmupOutput {
		copied_layers: warm_chunk.layers.len(),
		source_chunks_read,
	})
}

async fn load_target_manifest_index(
	cold_tier: &dyn ColdTier,
	target_branch_id: ActorBranchId,
	target_index_key: &str,
) -> Result<ColdManifestIndex> {
	let Some(bytes) = cold_tier.get_object(target_index_key).await? else {
		return Ok(ColdManifestIndex {
			schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
			branch_id: target_branch_id,
			chunks: Vec::new(),
			last_pass_at_ms: 0,
			last_pass_versionstamp: [0; 16],
		});
	};

	decode_cold_manifest_index(&bytes)
}

fn warm_layer_object_key(
	source_branch_id: ActorBranchId,
	target_branch_id: ActorBranchId,
	pass_uuid: uuid::Uuid,
	source_key: &str,
) -> Result<String> {
	let source_prefix = branch_object_prefix(source_branch_id);
	let target_prefix = branch_object_prefix(target_branch_id);
	if let Some(suffix) = source_key.strip_prefix(&format!("{source_prefix}/")) {
		return Ok(format!("{target_prefix}/{suffix}"));
	}

	let file_name = source_key
		.rsplit('/')
		.next()
		.filter(|name| !name.is_empty())
		.context("sqlite fork warmup source layer key was empty")?;
	Ok(format!(
		"{target_prefix}/warmup/{}/{}",
		pass_uuid.simple(),
		file_name
	))
}

fn warm_manifest_chunk_object_key(
	branch_id: ActorBranchId,
	pass_uuid: uuid::Uuid,
) -> String {
	format!(
		"{}/cold_manifest/chunks/warmup-{}.bare",
		branch_object_prefix(branch_id),
		pass_uuid.simple()
	)
}

fn manifest_index_object_key(branch_id: ActorBranchId) -> String {
	format!("{}/cold_manifest/index.bare", branch_object_prefix(branch_id))
}

fn branch_object_prefix(branch_id: ActorBranchId) -> String {
	format!("db/{}", branch_id.as_uuid().simple())
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

fn ensure_not_cancelled(cancel_token: &tokio_util::sync::CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite cold fork warmup cancelled");
	}

	Ok(())
}
