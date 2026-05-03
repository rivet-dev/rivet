use std::collections::BTreeMap;

#[cfg(feature = "test-faults")]
use crate::fault::{DepotFaultAction, DepotFaultFired, ReadFaultPoint};
use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{
		IsolationLevel::{Serializable, Snapshot},
		end_of_key_range,
	},
};

use crate::conveyer::{
	Db,
	db::CachedColdManifest,
	error::SqliteStorageError,
	keys,
	ltx::{DecodedLtx, decode_ltx_v3},
	metrics,
	types::{
		ColdShardRef, CompactionRoot, DatabaseBranchId, LayerEntry, LayerKind,
		decode_cold_manifest_chunk, decode_cold_manifest_index, decode_cold_shard_ref,
		decode_compaction_root,
	},
};

#[cfg(feature = "test-faults")]
use super::maybe_fire_read_fault;
use super::{
	cache_fill::{ShardCacheFillJob, ShardCacheFillKey},
	plan::{ReadSource, StorageScope},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ColdLayerCandidate {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) owner_txid: u64,
	pub(super) shard_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ColdPageCandidate {
	ColdManifestLayer(ColdLayerCandidate),
	CompactionColdShard(CompactionColdShardCandidate),
}

impl From<ColdLayerCandidate> for ColdPageCandidate {
	fn from(candidate: ColdLayerCandidate) -> Self {
		Self::ColdManifestLayer(candidate)
	}
}

impl From<CompactionColdShardCandidate> for ColdPageCandidate {
	fn from(candidate: CompactionColdShardCandidate) -> Self {
		Self::CompactionColdShard(candidate)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompactionColdShardCandidate {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) reference: ColdShardRef,
}

pub(super) struct LoadedColdPages {
	pub(super) pages: BTreeMap<u32, (Vec<u8>, Vec<u8>)>,
	pub(super) shard_cache_fills: Vec<ShardCacheFillJob>,
}

enum ColdObjectPageLoad {
	ObjectMissing,
	PageMissing,
	PagePresent(Vec<u8>),
}

impl Db {
	pub(super) async fn load_cold_page_blobs(
		&self,
		page_candidates: &BTreeMap<u32, Vec<ColdPageCandidate>>,
	) -> Result<LoadedColdPages> {
		if page_candidates.is_empty() {
			return Ok(LoadedColdPages {
				pages: BTreeMap::new(),
				shard_cache_fills: Vec::new(),
			});
		}
		if self.cold_tier.is_none() {
			if let Some(pgno) = page_candidates.iter().find_map(|(pgno, candidates)| {
				candidates
					.iter()
					.any(|candidate| matches!(candidate, ColdPageCandidate::CompactionColdShard(_)))
					.then_some(*pgno)
			}) {
				return Err(SqliteStorageError::ShardCoverageMissing { pgno }.into());
			}

			return Ok(LoadedColdPages {
				pages: BTreeMap::new(),
				shard_cache_fills: Vec::new(),
			});
		}

		let mut out = BTreeMap::new();
		let mut shard_cache_fills = BTreeMap::<ShardCacheFillKey, ShardCacheFillJob>::new();
		let mut loaded_objects = BTreeMap::<String, Vec<u8>>::new();
		let mut decoded_objects = BTreeMap::<String, DecodedLtx>::new();

		for (pgno, candidates) in page_candidates {
			for candidate in candidates {
				match candidate {
					ColdPageCandidate::ColdManifestLayer(candidate) => {
						for layer in self.find_cold_layers(*candidate).await? {
							if let ColdObjectPageLoad::PagePresent(bytes) = self
								.load_cold_object_for_page(
									*pgno,
									&layer.object_key,
									&mut loaded_objects,
									&mut decoded_objects,
								)
								.await?
							{
								out.insert(*pgno, (cold_source_key(&layer.object_key), bytes));
								break;
							}
						}
					}
					ColdPageCandidate::CompactionColdShard(candidate) => {
						match self
							.load_cold_object_for_page(
								*pgno,
								&candidate.reference.object_key,
								&mut loaded_objects,
								&mut decoded_objects,
							)
							.await?
						{
							ColdObjectPageLoad::ObjectMissing => {
								return Err(SqliteStorageError::ShardCoverageMissing {
									pgno: *pgno,
								}
								.into());
							}
							ColdObjectPageLoad::PageMissing => {}
							ColdObjectPageLoad::PagePresent(bytes) => {
								self.ensure_compaction_cold_ref_still_current(candidate, *pgno)
									.await?;
								let job = ShardCacheFillJob::new(
									candidate.branch_id,
									candidate.reference.clone(),
									bytes.clone(),
								)?;
								shard_cache_fills.entry(job.key()).or_insert(job);
								out.insert(
									*pgno,
									(cold_source_key(&candidate.reference.object_key), bytes),
								);
							}
						}
					}
				}
				if out.contains_key(pgno) {
					break;
				}
			}
		}

		Ok(LoadedColdPages {
			pages: out,
			shard_cache_fills: shard_cache_fills.into_values().collect(),
		})
	}

	async fn load_cold_object_for_page(
		&self,
		pgno: u32,
		object_key: &str,
		loaded_objects: &mut BTreeMap<String, Vec<u8>>,
		decoded_objects: &mut BTreeMap<String, DecodedLtx>,
	) -> Result<ColdObjectPageLoad> {
		if !loaded_objects.contains_key(object_key) {
			let Some(cold_tier) = &self.cold_tier else {
				return Ok(ColdObjectPageLoad::ObjectMissing);
			};
			let _timer = metrics::SQLITE_SHARD_CACHE_COLD_READ_DURATION.start_timer();
			let bytes = cold_tier
				.get_object(object_key)
				.await
				.with_context(|| format!("get sqlite cold layer {object_key}"))?;
			#[cfg(feature = "test-faults")]
			let mut bytes = bytes;
			#[cfg(feature = "test-faults")]
			if matches!(
				maybe_fire_read_fault(
					&self.fault_controller,
					ReadFaultPoint::ColdObjectMissing,
					&self.database_id,
					None,
					Some(pgno),
					Some(pgno / keys::SHARD_SIZE),
				)
				.await?,
				Some(DepotFaultFired {
					action: DepotFaultAction::DropArtifact,
					..
				})
			) {
				bytes = None;
			}
			let Some(bytes) = bytes else {
				return Ok(ColdObjectPageLoad::ObjectMissing);
			};
			loaded_objects.insert(object_key.to_string(), bytes);
		}

		if !decoded_objects.contains_key(object_key) {
			let bytes = loaded_objects
				.get(object_key)
				.expect("cold object should be loaded before decode");
			let decoded = decode_ltx_v3(bytes)
				.with_context(|| format!("decode sqlite cold layer {object_key}"))?;
			decoded_objects.insert(object_key.to_string(), decoded);
		}

		if decoded_objects
			.get(object_key)
			.and_then(|decoded| decoded.get_page(pgno))
			.is_none()
		{
			return Ok(ColdObjectPageLoad::PageMissing);
		}

		Ok(ColdObjectPageLoad::PagePresent(
			loaded_objects
				.get(object_key)
				.expect("cold object should be loaded before page return")
				.clone(),
		))
	}

	async fn ensure_compaction_cold_ref_still_current(
		&self,
		candidate: &CompactionColdShardCandidate,
		pgno: u32,
	) -> Result<()> {
		let branch_id = candidate.branch_id;
		let reference = candidate.reference.clone();
		let key = keys::branch_compaction_cold_shard_key(
			branch_id,
			reference.shard_id,
			reference.as_of_txid,
		);

		self.udb
			.run(move |tx| {
				let key = key.clone();
				let reference = reference.clone();
				async move {
					let Some(value) = tx
						.informal()
						.get(&key, Serializable)
						.await?
						.map(Vec::<u8>::from)
					else {
						return Err(SqliteStorageError::ShardCoverageMissing { pgno }.into());
					};
					let current = decode_cold_shard_ref(&value)?;
					if current != reference {
						return Err(SqliteStorageError::ShardCoverageMissing { pgno }.into());
					}

					Ok(())
				}
			})
			.await
	}

	async fn find_cold_layers(&self, candidate: ColdLayerCandidate) -> Result<Vec<LayerEntry>> {
		let manifest = self.load_cold_manifest(candidate.branch_id).await?;
		let mut layers = Vec::new();

		for chunk in &manifest.chunks {
			for layer in &chunk.layers {
				if !layer_covers_candidate(layer, candidate) {
					continue;
				}

				layers.push(layer.clone());
			}
		}

		layers.sort_by(|a, b| {
			b.max_txid
				.cmp(&a.max_txid)
				.then_with(|| layer_kind_rank(b.kind).cmp(&layer_kind_rank(a.kind)))
		});

		Ok(layers)
	}

	async fn load_cold_manifest(&self, branch_id: DatabaseBranchId) -> Result<CachedColdManifest> {
		let cached_manifest = { self.cold_manifest_cache.write().await.get(branch_id) };
		if let Some(manifest) = cached_manifest {
			return Ok(manifest);
		}

		let Some(cold_tier) = &self.cold_tier else {
			return Ok(CachedColdManifest { chunks: Vec::new() });
		};
		let index_key = cold_manifest_index_object_key(branch_id);
		let Some(index_bytes) = cold_tier
			.get_object(&index_key)
			.await
			.with_context(|| format!("get sqlite cold manifest index {index_key}"))?
		else {
			return Ok(CachedColdManifest { chunks: Vec::new() });
		};
		let index = decode_cold_manifest_index(&index_bytes)
			.with_context(|| format!("decode sqlite cold manifest index {index_key}"))?;
		let mut chunks = Vec::new();

		for chunk_ref in index.chunks {
			let Some(chunk_bytes) = cold_tier
				.get_object(&chunk_ref.object_key)
				.await
				.with_context(|| {
					format!("get sqlite cold manifest chunk {}", chunk_ref.object_key)
				})?
			else {
				continue;
			};
			chunks.push(decode_cold_manifest_chunk(&chunk_bytes).with_context(|| {
				format!("decode sqlite cold manifest chunk {}", chunk_ref.object_key)
			})?);
		}

		let manifest = CachedColdManifest { chunks };
		self.cold_manifest_cache
			.write()
			.await
			.insert(branch_id, manifest.clone());

		Ok(manifest)
	}
}

pub(super) async fn tx_load_latest_compaction_cold_ref(
	tx: &universaldb::Transaction,
	scope: &StorageScope,
	shard_id: u32,
) -> Result<Option<CompactionColdShardCandidate>> {
	let sources = match scope {
		StorageScope::Branch(plan) => plan.sources.clone(),
	};

	for source in sources {
		let ReadSource::Branch(source) = source;
		let Some(root) = tx_load_compaction_root(tx, source.branch_id).await? else {
			continue;
		};
		let as_of_txid = source.max_txid;
		let prefix = keys::branch_compaction_cold_shard_version_prefix(source.branch_id, shard_id);
		let end_key =
			keys::branch_compaction_cold_shard_key(source.branch_id, shard_id, as_of_txid);
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
			latest = Some(entry.value().to_vec());
		}

		let Some(value) = latest else {
			continue;
		};
		let reference = decode_cold_shard_ref(&value)?;
		if reference.shard_id == shard_id
			&& reference.as_of_txid <= as_of_txid
			&& reference.publish_generation <= root.manifest_generation
		{
			return Ok(Some(CompactionColdShardCandidate {
				branch_id: source.branch_id,
				reference,
			}));
		}
	}

	Ok(None)
}

async fn tx_load_compaction_root(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Option<CompactionRoot>> {
	let Some(bytes) =
		super::tx::tx_get_value(tx, &keys::branch_compaction_root_key(branch_id)).await?
	else {
		return Ok(None);
	};
	if bytes.is_empty() {
		return Ok(None);
	}

	Ok(Some(decode_compaction_root(&bytes)?))
}

fn layer_covers_candidate(layer: &LayerEntry, candidate: ColdLayerCandidate) -> bool {
	if candidate.owner_txid < layer.min_txid || candidate.owner_txid > layer.max_txid {
		return false;
	}

	match layer.kind {
		LayerKind::Image => layer.shard_id == Some(candidate.shard_id),
		LayerKind::Delta | LayerKind::Pin => true,
	}
}

fn layer_kind_rank(kind: LayerKind) -> u8 {
	match kind {
		LayerKind::Pin => 3,
		LayerKind::Delta => 2,
		LayerKind::Image => 1,
	}
}

fn cold_source_key(object_key: &str) -> Vec<u8> {
	let mut key = b"cold:".to_vec();
	key.extend_from_slice(object_key.as_bytes());
	key
}

fn cold_manifest_index_object_key(branch_id: DatabaseBranchId) -> String {
	format!(
		"db/{}/cold_manifest/index.bare",
		branch_id.as_uuid().simple()
	)
}
