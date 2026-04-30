use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};

use anyhow::{Context, Result, bail};

use crate::{
	cold_tier::ColdTier,
	pump::{
		constants::STALE_MARKER_AGE_MS,
		ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
		types::{
			BookmarkIndexEntry, ColdManifestChunk, ColdManifestChunkRef, ColdManifestIndex,
			DirtyPage, LayerEntry, LayerKind, PinStatus, PointerSnapshot,
			SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_cold_manifest_index,
			encode_actor_branch_record, encode_cold_manifest_chunk, encode_cold_manifest_index,
			encode_pointer_snapshot,
		},
	},
};

use super::phase_a::{
	ColdCommitRow, ColdPendingMarker, ColdPhaseAPlan, ColdPinUpload, decode_pending_marker,
	encode_pending_marker,
};

pub(crate) async fn run(
	cold_tier: Arc<dyn ColdTier>,
	plan: &ColdPhaseAPlan,
	cancel_token: tokio_util::sync::CancellationToken,
	now_ms: i64,
) -> Result<ColdPhaseBOutput> {
	ensure_not_cancelled(&cancel_token)?;

	let object_keys = planned_object_keys(plan);
	let marker = ColdPendingMarker {
		planned_object_keys: object_keys.iter().cloned().collect(),
		..plan.marker.clone()
	};
	let mut bytes_uploaded = 0u64;
	let marker_bytes = encode_pending_marker(marker)?;
	bytes_uploaded += marker_bytes.len() as u64;
	cold_tier
		.put_object(&plan.pending_marker_key, &marker_bytes)
		.await
		.with_context(|| format!("update sqlite cold pending marker {}", plan.pending_marker_key))?;

	let mut layers = Vec::new();

	ensure_not_cancelled(&cancel_token)?;
	for shard in &plan.shard_versions {
		let object_key = image_object_key(plan, shard.shard_id, shard.as_of_txid);
		cold_tier
			.put_object(&object_key, &shard.bytes)
			.await
			.with_context(|| format!("put sqlite cold image layer {object_key}"))?;
		bytes_uploaded += shard.bytes.len() as u64;
		layers.push(LayerEntry {
			kind: LayerKind::Image,
			shard_id: Some(shard.shard_id),
			min_txid: shard.as_of_txid,
			max_txid: shard.as_of_txid,
			min_versionstamp: versionstamp_for_txid(plan, shard.as_of_txid),
			max_versionstamp: versionstamp_for_txid(plan, shard.as_of_txid),
			byte_size: shard.bytes.len() as u64,
			checksum: checksum_bytes(&shard.bytes),
			object_key,
		});
	}

	ensure_not_cancelled(&cancel_token)?;
	if let Some((object_key, bytes, min_txid, max_txid)) = delta_layer(plan) {
		cold_tier
			.put_object(&object_key, &bytes)
			.await
			.with_context(|| format!("put sqlite cold delta layer {object_key}"))?;
		bytes_uploaded += bytes.len() as u64;
		layers.push(LayerEntry {
			kind: LayerKind::Delta,
			shard_id: None,
			min_txid,
			max_txid,
			min_versionstamp: versionstamp_for_txid(plan, min_txid),
			max_versionstamp: versionstamp_for_txid(plan, max_txid),
			byte_size: bytes.len() as u64,
			checksum: checksum_bytes(&bytes),
			object_key,
		});
	}

	let mut bookmarks = Vec::new();
	let mut uploaded_pins = Vec::new();
	ensure_not_cancelled(&cancel_token)?;
	for pin in &plan.pin_uploads {
		let object_key = pin_object_key(plan, pin);
		let bytes = build_pin_image(plan, pin)
			.with_context(|| format!("build sqlite cold pin layer {object_key}"))?;
		cold_tier
			.put_object(&object_key, &bytes)
			.await
			.with_context(|| format!("put sqlite cold pin layer {object_key}"))?;
		bytes_uploaded += bytes.len() as u64;
		let txid = txid_for_versionstamp(plan, pin.versionstamp).unwrap_or(plan.materialized_txid);
		layers.push(LayerEntry {
			kind: LayerKind::Pin,
			shard_id: None,
			min_txid: txid,
			max_txid: txid,
			min_versionstamp: pin.versionstamp,
			max_versionstamp: pin.versionstamp,
			byte_size: bytes.len() as u64,
			checksum: checksum_bytes(&bytes),
			object_key: object_key.clone(),
		});
		bookmarks.push(BookmarkIndexEntry {
			schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
			bookmark_str: pin.bookmark.clone(),
			pinned: true,
			pin_object_key: Some(object_key.clone()),
			pin_status: PinStatus::Pending,
			created_at_ms: plan.marker.created_at_ms,
		});
		uploaded_pins.push(ColdUploadedPin {
			actor_id: pin.actor_id.clone(),
			actor_branch_id: plan.branch_id,
			bookmark: pin.bookmark.clone(),
			versionstamp: pin.versionstamp,
			object_key,
		});
	}

	ensure_not_cancelled(&cancel_token)?;
	if let Some(record) = plan.branch_record.clone() {
		let object_key = branch_record_object_key(plan);
		let record_bytes = encode_actor_branch_record(record)?;
		bytes_uploaded += record_bytes.len() as u64;
		cold_tier
			.put_object(&object_key, &record_bytes)
			.await
			.with_context(|| format!("put sqlite cold branch record {object_key}"))?;
	}

	let pass_versionstamp = pass_versionstamp(plan);
	let chunk_key = manifest_chunk_object_key(plan);
	let chunk = ColdManifestChunk {
		schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
		branch_id: plan.branch_id,
		pass_versionstamp,
		layers,
		bookmarks,
	};
	let chunk_bytes = encode_cold_manifest_chunk(chunk.clone())?;
	bytes_uploaded += chunk_bytes.len() as u64;

	ensure_not_cancelled(&cancel_token)?;
	cold_tier
		.put_object(&chunk_key, &chunk_bytes)
		.await
		.with_context(|| format!("put sqlite cold manifest chunk {chunk_key}"))?;

	let index_key = manifest_index_object_key(plan);
	let mut index = load_manifest_index(cold_tier.as_ref(), plan, &index_key).await?;
	index.chunks.push(ColdManifestChunkRef {
		object_key: chunk_key.clone(),
		pass_versionstamp,
		min_versionstamp: min_layer_versionstamp(&chunk),
		max_versionstamp: max_layer_versionstamp(&chunk),
		byte_size: chunk_bytes.len() as u64,
	});
	index.last_pass_at_ms = now_ms;
	index.last_pass_versionstamp = pass_versionstamp;
	let index_bytes = encode_cold_manifest_index(index)?;
	bytes_uploaded += index_bytes.len() as u64;
	cold_tier
		.put_object(&index_key, &index_bytes)
		.await
		.with_context(|| format!("put sqlite cold manifest index {index_key}"))?;

	let snapshot_key = pointer_snapshot_object_key(plan);
	let snapshot_bytes = encode_pointer_snapshot(pointer_snapshot(plan, pass_versionstamp))?;
	bytes_uploaded += snapshot_bytes.len() as u64;
	cold_tier
		.put_object(&snapshot_key, &snapshot_bytes)
		.await
		.with_context(|| format!("put sqlite cold pointer snapshot {snapshot_key}"))?;

	let stale_markers_cleaned = clean_stale_pending_markers(cold_tier.as_ref(), plan, now_ms).await?;

	Ok(ColdPhaseBOutput {
		layer_count: chunk.layers.len(),
		bookmark_count: chunk.bookmarks.len(),
		stale_markers_cleaned,
		bytes_uploaded,
		uploaded_pins,
	})
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdPhaseBOutput {
	pub layer_count: usize,
	pub bookmark_count: usize,
	pub stale_markers_cleaned: usize,
	pub bytes_uploaded: u64,
	pub uploaded_pins: Vec<ColdUploadedPin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdUploadedPin {
	pub actor_id: String,
	pub actor_branch_id: crate::pump::types::ActorBranchId,
	pub bookmark: crate::pump::types::BookmarkStr,
	pub versionstamp: [u8; 16],
	pub object_key: String,
}

fn planned_object_keys(plan: &ColdPhaseAPlan) -> BTreeSet<String> {
	let mut keys = BTreeSet::from([
		branch_record_object_key(plan),
		manifest_index_object_key(plan),
		manifest_chunk_object_key(plan),
		pointer_snapshot_object_key(plan),
	]);

	for shard in &plan.shard_versions {
		keys.insert(image_object_key(plan, shard.shard_id, shard.as_of_txid));
	}
	if !plan.delta_chunks.is_empty() {
		keys.insert(delta_object_key(plan, min_delta_txid(plan), max_delta_txid(plan)));
	}
	for pin in &plan.pin_uploads {
		keys.insert(pin_object_key(plan, pin));
	}

	keys
}

async fn load_manifest_index(
	cold_tier: &dyn ColdTier,
	plan: &ColdPhaseAPlan,
	index_key: &str,
) -> Result<ColdManifestIndex> {
	let Some(bytes) = cold_tier.get_object(index_key).await? else {
		return Ok(ColdManifestIndex {
			schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
			branch_id: plan.branch_id,
			chunks: Vec::new(),
			last_pass_at_ms: 0,
			last_pass_versionstamp: [0; 16],
		});
	};

	decode_cold_manifest_index(&bytes)
}

fn delta_layer(plan: &ColdPhaseAPlan) -> Option<(String, Vec<u8>, u64, u64)> {
	if plan.delta_chunks.is_empty() {
		return None;
	}

	let min_txid = min_delta_txid(plan);
	let max_txid = max_delta_txid(plan);
	let mut chunks = plan.delta_chunks.clone();
	chunks.sort_by_key(|chunk| (chunk.txid, chunk.chunk_idx));
	let bytes = chunks
		.into_iter()
		.flat_map(|chunk| chunk.bytes)
		.collect::<Vec<_>>();

	Some((delta_object_key(plan, min_txid, max_txid), bytes, min_txid, max_txid))
}

fn build_pin_image(plan: &ColdPhaseAPlan, pin: &ColdPinUpload) -> Result<Vec<u8>> {
	let pin_txid = txid_for_versionstamp(plan, pin.versionstamp).unwrap_or(plan.materialized_txid);
	let mut latest_by_shard = BTreeMap::new();

	for shard in plan
		.shard_versions
		.iter()
		.filter(|shard| shard.as_of_txid <= pin_txid)
	{
		latest_by_shard
			.entry(shard.shard_id)
			.and_modify(|current: &mut &super::phase_a::ColdShardVersion| {
				if shard.as_of_txid > current.as_of_txid {
					*current = shard;
				}
			})
			.or_insert(shard);
	}

	let mut pages = Vec::new();
	for shard in latest_by_shard.values() {
		match decode_ltx_v3(&shard.bytes) {
			Ok(decoded) => pages.extend(decoded.pages),
			Err(_) => return Ok(concat_shard_bytes(plan, pin_txid)),
		}
	}
	if pages.is_empty() {
		return Ok(Vec::new());
	}

	pages.sort_by_key(|page| page.pgno);
	let db_size_pages = commit_for_txid(plan, pin_txid).map_or(1, |commit| commit.row.db_size_pages);
	encode_ltx_v3(
		LtxHeader {
			min_txid: pin_txid,
			max_txid: pin_txid,
			commit: db_size_pages,
			timestamp_ms: commit_for_txid(plan, pin_txid).map_or(0, |commit| commit.row.wall_clock_ms),
			..LtxHeader::delta(pin_txid.max(1), db_size_pages, 0)
		},
		&pages
			.into_iter()
			.filter(|page: &DirtyPage| page.pgno <= db_size_pages)
			.collect::<Vec<_>>(),
	)
}

fn concat_shard_bytes(plan: &ColdPhaseAPlan, pin_txid: u64) -> Vec<u8> {
	let mut shards = plan
		.shard_versions
		.iter()
		.filter(|shard| shard.as_of_txid <= pin_txid)
		.collect::<Vec<_>>();
	shards.sort_by_key(|shard| (shard.shard_id, shard.as_of_txid));

	shards
		.into_iter()
		.flat_map(|shard| shard.bytes.clone())
		.collect::<Vec<_>>()
}

fn pointer_snapshot(plan: &ColdPhaseAPlan, pass_versionstamp: [u8; 16]) -> PointerSnapshot {
	let actors = plan
		.actor_id
		.as_ref()
		.map(|actor_id| {
			vec![(
				actor_id.clone(),
				plan.branch_record
					.as_ref()
					.map_or_else(crate::types::NamespaceBranchId::nil, |record| {
						record.namespace_branch
					}),
				plan.branch_id,
			)]
		})
		.unwrap_or_default();

	PointerSnapshot {
		schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
		pass_versionstamp,
		actors,
		namespaces: Vec::new(),
	}
}

async fn clean_stale_pending_markers(
	cold_tier: &dyn ColdTier,
	plan: &ColdPhaseAPlan,
	now_ms: i64,
) -> Result<usize> {
	let prefix = format!("{}/pending", branch_object_prefix(plan));
	let objects = cold_tier.list_prefix(&prefix).await?;
	let mut cleaned = 0;

	for object in objects {
		if object.key == plan.pending_marker_key {
			continue;
		}
		let Some(bytes) = cold_tier.get_object(&object.key).await? else {
			continue;
		};
		let marker = match decode_pending_marker(&bytes) {
			Ok(marker) => marker,
			Err(err) => {
				tracing::warn!(?err, key = %object.key, "failed to decode sqlite cold pending marker");
				continue;
			}
		};
		if now_ms.saturating_sub(marker.created_at_ms) < i64::from(STALE_MARKER_AGE_MS) {
			continue;
		}

		cold_tier.delete_objects(&marker.planned_object_keys).await?;
		cold_tier.delete_objects(&[object.key]).await?;
		cleaned += 1;
	}

	Ok(cleaned)
}

fn min_delta_txid(plan: &ColdPhaseAPlan) -> u64 {
	plan.delta_chunks
		.iter()
		.map(|chunk| chunk.txid)
		.min()
		.unwrap_or_default()
}

fn max_delta_txid(plan: &ColdPhaseAPlan) -> u64 {
	plan.delta_chunks
		.iter()
		.map(|chunk| chunk.txid)
		.max()
		.unwrap_or_default()
}

fn commit_for_txid(plan: &ColdPhaseAPlan, txid: u64) -> Option<&ColdCommitRow> {
	plan.commit_rows.iter().find(|commit| commit.txid == txid)
}

fn versionstamp_for_txid(plan: &ColdPhaseAPlan, txid: u64) -> [u8; 16] {
	commit_for_txid(plan, txid).map_or([0; 16], |commit| commit.row.versionstamp)
}

fn txid_for_versionstamp(plan: &ColdPhaseAPlan, versionstamp: [u8; 16]) -> Option<u64> {
	plan.vtx_rows
		.iter()
		.find(|row| row.versionstamp == versionstamp)
		.map(|row| row.txid)
}

fn pass_versionstamp(plan: &ColdPhaseAPlan) -> [u8; 16] {
	plan.vtx_rows
		.iter()
		.max_by_key(|row| row.txid)
		.map_or([0; 16], |row| row.versionstamp)
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

fn branch_record_object_key(plan: &ColdPhaseAPlan) -> String {
	format!("{}/branch_record.bare", branch_object_prefix(plan))
}

fn manifest_index_object_key(plan: &ColdPhaseAPlan) -> String {
	format!("{}/cold_manifest/index.bare", branch_object_prefix(plan))
}

fn manifest_chunk_object_key(plan: &ColdPhaseAPlan) -> String {
	format!(
		"{}/cold_manifest/chunks/{}.bare",
		branch_object_prefix(plan),
		plan.pass_uuid.simple()
	)
}

fn pointer_snapshot_object_key(plan: &ColdPhaseAPlan) -> String {
	format!(
		"{}/pointer_snapshot/{}.bare",
		branch_object_prefix(plan),
		plan.pass_uuid.simple()
	)
}

fn image_object_key(plan: &ColdPhaseAPlan, shard_id: u32, as_of_txid: u64) -> String {
	format!(
		"{}/image/{:08x}/{shard_id:08x}-{as_of_txid:016x}.ltx",
		branch_object_prefix(plan),
		(as_of_txid >> 32) as u32,
	)
}

fn delta_object_key(plan: &ColdPhaseAPlan, min_txid: u64, max_txid: u64) -> String {
	format!(
		"{}/delta/{min_txid:016x}-{max_txid:016x}.ltx",
		branch_object_prefix(plan)
	)
}

fn pin_object_key(plan: &ColdPhaseAPlan, pin: &ColdPinUpload) -> String {
	format!(
		"{}/pin/{}.ltx",
		branch_object_prefix(plan),
		hex_bytes(&pin.versionstamp)
	)
}

fn branch_object_prefix(plan: &ColdPhaseAPlan) -> String {
	format!("db/{}", plan.branch_id.as_uuid().simple())
}

fn checksum_bytes(bytes: &[u8]) -> u64 {
	bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
		(hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
	})
}

fn hex_bytes(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn ensure_not_cancelled(cancel_token: &tokio_util::sync::CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite cold compaction cancelled");
	}

	Ok(())
}
