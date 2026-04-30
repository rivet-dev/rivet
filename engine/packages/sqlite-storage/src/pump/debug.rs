//! Debug-only diagnostics for PITR and branch state.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Snapshot, end_of_key_range},
};

use crate::{
	gc,
	pump::{
		ActorDb, actor_db::load_branch_ancestry, branch, keys,
		ltx::{DecodedLtx, decode_ltx_v3},
		types::{
			ActorBranchId, BookmarkIndexEntry, BookmarkStr, ColdManifestChunk,
			ColdManifestIndex, CommitRow, FetchedPage, LayerEntry,
			LayerKind, PinStatus, SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_cold_manifest_chunk,
			decode_cold_manifest_index, decode_commit_row, decode_db_head,
			decode_pinned_bookmark_record,
		},
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPins {
	pub branch_id: ActorBranchId,
	pub refcount: i64,
	pub desc_pin: [u8; 16],
	pub bk_pin: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdManifest {
	pub branch_id: ActorBranchId,
	pub index: Option<ColdManifestIndex>,
	pub chunks: Vec<ColdManifestChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageState {
	pub branch_id: ActorBranchId,
	pub txid: u64,
	pub versionstamp: [u8; 16],
	pub db_size_pages: u32,
	pub pages: Vec<FetchedPage>,
}

#[derive(Debug, Clone, Copy)]
struct DebugReadSource {
	branch_id: ActorBranchId,
	max_txid: u64,
}

pub async fn dump_actor_ancestry(
	actor_db: &ActorDb,
) -> Result<Vec<(ActorBranchId, Option<[u8; 16]>)>> {
	let branch_id = resolve_current_branch(actor_db).await?;
	let ancestry = actor_db
		.udb
		.run(move |tx| async move { load_branch_ancestry(&tx, branch_id).await })
		.await?;

	Ok(ancestry
		.ancestors
		.into_iter()
		.map(|ancestor| (ancestor.branch_id, ancestor.parent_versionstamp_cap))
		.collect())
}

pub async fn dump_branch_pins(actor_db: &ActorDb) -> Result<BranchPins> {
	let branch_id = resolve_current_branch(actor_db).await?;
	let pin = gc::estimate_branch_gc_pin(&actor_db.udb, branch_id)
		.await?
		.context("sqlite branch was missing while dumping pins")?;

	Ok(BranchPins {
		branch_id,
		refcount: pin.refcount,
		desc_pin: pin.desc_pin,
		bk_pin: pin.bk_pin,
	})
}

pub async fn list_bookmarks(actor_db: &ActorDb) -> Result<Vec<BookmarkIndexEntry>> {
	let actor_id = actor_db.actor_id.clone();
	let branch_id = resolve_current_branch(actor_db).await?;

	actor_db
		.udb
		.run(move |tx| {
			let actor_id = actor_id.clone();

			async move {
				let mut entries = Vec::new();
				if let Some(head_bytes) = tx_get_value(&tx, &keys::branch_meta_head_key(branch_id)).await? {
					let head = decode_db_head(&head_bytes).context("decode sqlite branch head")?;
					if let Some(commit_bytes) =
						tx_get_value(&tx, &keys::branch_commit_key(branch_id, head.head_txid)).await?
					{
						let commit = decode_commit_row(&commit_bytes)
							.context("decode sqlite head commit row for bookmark listing")?;
						entries.push(BookmarkIndexEntry {
							schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
							bookmark_str: BookmarkStr::format(commit.wall_clock_ms, head.head_txid)?,
							pinned: false,
							pin_object_key: None,
							pin_status: PinStatus::Ready,
							created_at_ms: commit.wall_clock_ms,
						});
					}
				}

				for (key, value) in scan_prefix(&tx, &keys::bookmark_key(&actor_id, "")).await? {
					if !key.ends_with(b"/pinned") {
						continue;
					}
					let record = decode_pinned_bookmark_record(&value)
						.context("decode sqlite pinned bookmark record for debug listing")?;
					entries.push(BookmarkIndexEntry {
						schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
						bookmark_str: record.bookmark,
						pinned: true,
						pin_object_key: record.pin_object_key,
						pin_status: record.status,
						created_at_ms: record.created_at_ms,
					});
				}
				entries.sort_by(|a, b| {
					a.bookmark_str
						.cmp(&b.bookmark_str)
						.then_with(|| a.pinned.cmp(&b.pinned))
				});

				Ok(entries)
			}
		})
		.await
}

pub async fn dump_cold_manifest(actor_db: &ActorDb) -> Result<ColdManifest> {
	let branch_id = resolve_current_branch(actor_db).await?;
	let Some(cold_tier) = &actor_db.cold_tier else {
		return Ok(ColdManifest {
			branch_id,
			index: None,
			chunks: Vec::new(),
		});
	};

	let index_key = cold_manifest_index_object_key(branch_id);
	let Some(index_bytes) = cold_tier
		.get_object(&index_key)
		.await
		.with_context(|| format!("get sqlite cold manifest index {index_key}"))?
	else {
		return Ok(ColdManifest {
			branch_id,
			index: None,
			chunks: Vec::new(),
		});
	};
	let index = decode_cold_manifest_index(&index_bytes)
		.with_context(|| format!("decode sqlite cold manifest index {index_key}"))?;
	let mut chunks = Vec::new();
	for chunk_ref in &index.chunks {
		let Some(chunk_bytes) = cold_tier
			.get_object(&chunk_ref.object_key)
			.await
			.with_context(|| {
				format!("get sqlite cold manifest chunk {}", chunk_ref.object_key)
			})?
		else {
			continue;
		};
		chunks.push(
			decode_cold_manifest_chunk(&chunk_bytes).with_context(|| {
				format!("decode sqlite cold manifest chunk {}", chunk_ref.object_key)
			})?,
		);
	}

	Ok(ColdManifest {
		branch_id,
		index: Some(index),
		chunks,
	})
}

pub async fn estimate_gc_pin(actor_db: &ActorDb) -> Result<[u8; 16]> {
	let branch_id = resolve_current_branch(actor_db).await?;
	Ok(gc::estimate_branch_gc_pin(&actor_db.udb, branch_id)
		.await?
		.context("sqlite branch was missing while estimating GC pin")?
		.gc_pin)
}

pub async fn read_at(actor_db: &ActorDb, versionstamp: [u8; 16]) -> Result<PageState> {
	let root_branch_id = resolve_current_branch(actor_db).await?;
	let cold_tier = actor_db.cold_tier.clone();
	let branch_id_for_tx = root_branch_id;
	let read_plan = actor_db
		.udb
		.run(move |tx| async move {
			let ancestry = load_branch_ancestry(&tx, branch_id_for_tx).await?;
			let mut target = None;
			for (idx, ancestor) in ancestry.ancestors.iter().enumerate() {
				if ancestor
					.parent_versionstamp_cap
					.is_some_and(|cap| versionstamp > cap)
				{
					continue;
				}
				let Some(txid) = lookup_vtx_txid(&tx, ancestor.branch_id, versionstamp).await? else {
					continue;
				};
				target = Some((idx, *ancestor, txid));
				break;
			}
			let (target_idx, target_ancestor, target_txid) =
				target.context("sqlite versionstamp was not reachable from actor branch")?;
			let commit = read_commit_row(&tx, target_ancestor.branch_id, target_txid)
				.await?
				.context("sqlite commit row was missing for debug read_at")?;

			let mut sources = Vec::new();
			for (idx, ancestor) in ancestry.ancestors[target_idx..].iter().enumerate() {
				let max_txid = if idx == 0 {
					target_txid
				} else {
					let cap = ancestor
						.parent_versionstamp_cap
						.context("sqlite ancestor cap is missing")?;
					lookup_vtx_txid(&tx, ancestor.branch_id, cap)
						.await?
						.context("sqlite ancestor cap VTX row is missing")?
				};
				sources.push(DebugReadSource {
					branch_id: ancestor.branch_id,
					max_txid,
				});
			}

			let pages = load_pages_from_hot_tier(&tx, commit.db_size_pages, &sources).await?;

			Ok(DebugReadPlan {
				branch_id: target_ancestor.branch_id,
				txid: target_txid,
				versionstamp,
				db_size_pages: commit.db_size_pages,
				pages,
				sources,
			})
		})
		.await?;

	let pages = fill_cold_pages(
		cold_tier.as_ref(),
		read_plan.db_size_pages,
		read_plan.pages,
		&read_plan.sources,
	)
	.await?;

	Ok(PageState {
		branch_id: read_plan.branch_id,
		txid: read_plan.txid,
		versionstamp: read_plan.versionstamp,
		db_size_pages: read_plan.db_size_pages,
		pages,
	})
}

struct DebugReadPlan {
	branch_id: ActorBranchId,
	txid: u64,
	versionstamp: [u8; 16],
	db_size_pages: u32,
	pages: BTreeMap<u32, Option<Vec<u8>>>,
	sources: Vec<DebugReadSource>,
}

async fn resolve_current_branch(actor_db: &ActorDb) -> Result<ActorBranchId> {
	let namespace_id = actor_db.sqlite_namespace_id();
	let actor_id = actor_db.actor_id.clone();
	actor_db
		.udb
		.run(move |tx| {
			let actor_id = actor_id.clone();

			async move {
				branch::resolve_actor_branch(&tx, namespace_id, &actor_id, Snapshot)
					.await?
					.context("sqlite actor branch is missing")
			}
		})
		.await
}

async fn load_pages_from_hot_tier(
	tx: &universaldb::Transaction,
	db_size_pages: u32,
	sources: &[DebugReadSource],
) -> Result<BTreeMap<u32, Option<Vec<u8>>>> {
	let mut pages = (1..=db_size_pages)
		.map(|pgno| (pgno, None))
		.collect::<BTreeMap<_, _>>();

	for source in sources {
		for (_txid, blob) in tx_load_delta_blobs(tx, *source).await? {
			let decoded = decode_ltx_v3(&blob)
				.with_context(|| format!("decode sqlite debug delta for branch {:?}", source.branch_id))?;
			for pgno in 1..=db_size_pages {
				if pages.get(&pgno).is_some_and(Option::is_some) {
					continue;
				}
				if let Some(bytes) = decoded.get_page(pgno) {
					pages.insert(pgno, Some(bytes.to_vec()));
				}
			}
		}
	}

	let mut decoded_blobs = BTreeMap::<Vec<u8>, DecodedLtx>::new();
	for pgno in 1..=db_size_pages {
		if pages.get(&pgno).is_some_and(Option::is_some) {
			continue;
		}

		let shard_id = pgno / keys::SHARD_SIZE;
		if let Some((source_key, blob)) = tx_load_latest_shard_blob(tx, sources, shard_id).await? {
			if !decoded_blobs.contains_key(&source_key) {
				decoded_blobs.insert(
					source_key.clone(),
					decode_ltx_v3(&blob)
						.with_context(|| format!("decode sqlite debug shard for page {pgno}"))?,
				);
			}
			if let Some(bytes) = decoded_blobs
				.get(&source_key)
				.and_then(|decoded| decoded.get_page(pgno))
			{
				pages.insert(pgno, Some(bytes.to_vec()));
				continue;
			}
		}

		pages.insert(pgno, None);
	}

	Ok(pages)
}

async fn fill_cold_pages(
	cold_tier: Option<&std::sync::Arc<dyn crate::cold_tier::ColdTier>>,
	db_size_pages: u32,
	mut pages: BTreeMap<u32, Option<Vec<u8>>>,
	sources: &[DebugReadSource],
) -> Result<Vec<FetchedPage>> {
	let mut loaded_objects = BTreeMap::<String, Vec<u8>>::new();
	let mut decoded_objects = BTreeMap::<String, DecodedLtx>::new();

	for pgno in 1..=db_size_pages {
		if pages.get(&pgno).is_some_and(Option::is_some) {
			continue;
		}

		let Some(cold_tier) = cold_tier else {
			pages.insert(pgno, Some(vec![0; keys::PAGE_SIZE as usize]));
			continue;
		};
		let shard_id = pgno / keys::SHARD_SIZE;
		for source in sources {
			let mut layers = load_manifest_layers(cold_tier.as_ref(), *source, shard_id).await?;
			layers.sort_by(|a, b| {
				b.max_txid
					.cmp(&a.max_txid)
					.then_with(|| layer_kind_rank(b.kind).cmp(&layer_kind_rank(a.kind)))
			});
			for layer in layers {
				let object_key = layer.object_key.clone();
				if !loaded_objects.contains_key(&object_key) {
					let Some(bytes) = cold_tier
						.get_object(&object_key)
						.await
						.with_context(|| format!("get sqlite debug cold layer {object_key}"))?
					else {
						continue;
					};
					loaded_objects.insert(object_key.clone(), bytes);
				}
				if !decoded_objects.contains_key(&object_key) {
					let bytes = loaded_objects
						.get(&object_key)
						.context("sqlite debug cold layer should be loaded before decode")?;
					decoded_objects.insert(
						object_key.clone(),
						decode_ltx_v3(bytes)
							.with_context(|| format!("decode sqlite debug cold layer {object_key}"))?,
					);
				}
				if let Some(bytes) = decoded_objects
					.get(&object_key)
					.and_then(|decoded| decoded.get_page(pgno))
				{
					pages.insert(pgno, Some(bytes.to_vec()));
					break;
				}
			}
			if pages.get(&pgno).is_some_and(Option::is_some) {
				break;
			}
		}
		pages
			.entry(pgno)
			.or_insert_with(|| Some(vec![0; keys::PAGE_SIZE as usize]));
	}

	Ok((1..=db_size_pages)
		.map(|pgno| FetchedPage {
			pgno,
			bytes: pages
				.remove(&pgno)
				.unwrap_or_else(|| Some(vec![0; keys::PAGE_SIZE as usize])),
		})
		.collect())
}

async fn load_manifest_layers(
	cold_tier: &dyn crate::cold_tier::ColdTier,
	source: DebugReadSource,
	shard_id: u32,
) -> Result<Vec<LayerEntry>> {
	let Some(index_bytes) = cold_tier
		.get_object(&cold_manifest_index_object_key(source.branch_id))
		.await?
	else {
		return Ok(Vec::new());
	};
	let index = decode_cold_manifest_index(&index_bytes)
		.context("decode sqlite debug cold manifest index")?;
	let mut layers = Vec::new();

	for chunk_ref in index.chunks {
		let Some(chunk_bytes) = cold_tier.get_object(&chunk_ref.object_key).await? else {
			continue;
		};
		let chunk = decode_cold_manifest_chunk(&chunk_bytes)
			.context("decode sqlite debug cold manifest chunk")?;
		for layer in chunk.layers {
			if source.max_txid < layer.min_txid || source.max_txid > layer.max_txid {
				continue;
			}
			match layer.kind {
				LayerKind::Image if layer.shard_id == Some(shard_id) => layers.push(layer),
				LayerKind::Delta | LayerKind::Pin => layers.push(layer),
				LayerKind::Image => {}
			}
		}
	}

	Ok(layers)
}

async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<Option<CommitRow>> {
	let Some(bytes) = tx_get_value(tx, &keys::branch_commit_key(branch_id, txid)).await? else {
		return Ok(None);
	};

	Ok(Some(
		decode_commit_row(&bytes).context("decode sqlite commit row for debug read")?,
	))
}

async fn lookup_vtx_txid(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	versionstamp: [u8; 16],
) -> Result<Option<u64>> {
	let Some(bytes) = tx_get_value(tx, &keys::branch_vtx_key(branch_id, versionstamp)).await? else {
		return Ok(None);
	};
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, Snapshot)
		.await?
		.map(Vec::<u8>::from))
}

async fn scan_prefix(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

async fn tx_load_delta_blobs(
	tx: &universaldb::Transaction,
	source: DebugReadSource,
) -> Result<Vec<(u64, Vec<u8>)>> {
	let mut chunks_by_txid = BTreeMap::<u64, Vec<(u32, Vec<u8>)>>::new();
	for (key, chunk) in scan_prefix(tx, &keys::branch_delta_prefix(source.branch_id)).await? {
		let txid = keys::decode_branch_delta_chunk_txid(source.branch_id, &key)?;
		if txid > source.max_txid {
			continue;
		}
		let chunk_idx = keys::decode_branch_delta_chunk_idx(source.branch_id, txid, &key)?;
		chunks_by_txid
			.entry(txid)
			.or_default()
			.push((chunk_idx, chunk));
	}

	let mut blobs = Vec::new();
	for (txid, mut chunks) in chunks_by_txid.into_iter().rev() {
		chunks.sort_by_key(|(chunk_idx, _)| *chunk_idx);
		let mut blob = Vec::new();
		for (_, chunk) in chunks {
			blob.extend_from_slice(&chunk);
		}
		blobs.push((txid, blob));
	}

	Ok(blobs)
}

async fn tx_load_latest_shard_blob(
	tx: &universaldb::Transaction,
	sources: &[DebugReadSource],
	shard_id: u32,
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	for source in sources {
		let prefix = keys::branch_shard_version_prefix(source.branch_id, shard_id);
		let end_key = keys::branch_shard_key(source.branch_id, shard_id, source.max_txid);
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

fn cold_manifest_index_object_key(branch_id: ActorBranchId) -> String {
	format!("db/{}/cold_manifest/index.bare", branch_id.as_uuid().simple())
}

fn layer_kind_rank(kind: LayerKind) -> u8 {
	match kind {
		LayerKind::Pin => 3,
		LayerKind::Delta => 2,
		LayerKind::Image => 1,
	}
}
