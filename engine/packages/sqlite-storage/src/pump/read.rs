//! Page read path for the stateless sqlite-storage pump.

use std::{
	collections::{BTreeMap, BTreeSet},
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{IsolationLevel::Snapshot, end_of_key_range},
};

use crate::pump::{
	ActorDb,
	actor_db::{
		BranchAncestry, CachedColdManifest, load_branch_ancestry,
		touch_access_if_bucket_advanced,
	},
	branch,
	error::SqliteStorageError,
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{DecodedLtx, decode_ltx_v3},
	metrics,
	page_index::DeltaPageIndex,
	types::{
		ActorBranchId, DBHead, FetchedPage, LayerEntry, LayerKind,
		decode_cold_manifest_chunk, decode_cold_manifest_index, decode_db_head,
	},
};

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

impl ActorDb {
	pub async fn get_pages(&self, pgnos: Vec<u32>) -> Result<Vec<FetchedPage>> {
		let node_id = self.node_id.to_string();
		let labels = &[node_id.as_str()];
		let _timer = metrics::SQLITE_PUMP_GET_PAGES_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_GET_PAGES_PGNO_COUNT
			.with_label_values(labels)
			.observe(pgnos.len() as f64);

		for pgno in &pgnos {
			ensure!(*pgno > 0, "get_pages does not accept page 0");
		}

		let cached_pidx = {
			let cache = self.cache.lock();
			let cached_rows = cache.range(0, u32::MAX);
			if cached_rows.is_empty() {
				None
			} else {
				Some(
					pgnos
						.iter()
						.map(|pgno| (*pgno, cache.get(*pgno)))
						.collect::<BTreeMap<_, _>>(),
				)
			}
		};

		let actor_id = self.actor_id.clone();
		let namespace_id = self.sqlite_namespace_id();
		let cached_branch_id = *self.branch_id.lock();
		let cached_ancestry = self.ancestors.lock().clone();
		let cached_access_bucket = *self.last_access_bucket.lock();
		let pgnos_for_tx = pgnos.clone();
		let now_ms = now_ms()?;
		let tx_result = self
			.udb
			.run(move |tx| {
				let actor_id = actor_id.clone();
				let namespace_id = namespace_id;
				let pgnos = pgnos_for_tx.clone();
				let cached_pidx = cached_pidx.clone();
				let cached_ancestry = cached_ancestry.clone();
				let cached_access_bucket = cached_access_bucket;

				async move {
					let scope = resolve_storage_scope(
						&tx,
						namespace_id,
						&actor_id,
						cached_ancestry.as_ref(),
					)
					.await?;
					let cached_pidx = if scope.branch_id() == cached_branch_id {
						cached_pidx
					} else {
						None
					};
					let head = match &scope {
						StorageScope::Branch(plan) => plan.head.clone(),
						StorageScope::Legacy => {
							let head_bytes = tx_get_value(&tx, &keys::meta_head_key(&actor_id)).await?;
							let Some(head_bytes) = head_bytes else {
								return Err(SqliteStorageError::MetaMissing {
									operation: "get_pages",
								}
								.into());
							};
							decode_db_head(&head_bytes)?
						}
					};

					let pgnos_in_range = pgnos
						.into_iter()
						.filter(|pgno| *pgno <= head.db_size_pages)
						.collect::<Vec<_>>();
					if pgnos_in_range.is_empty() {
						let access_bucket = if let Some(branch_id) = scope.branch_id() {
							touch_access_if_bucket_advanced(
								&tx,
								branch_id,
								cached_access_bucket,
								now_ms,
							)
							.await?
						} else {
							None
						};
						return Ok(GetPagesTxResult {
							branch_id: scope.branch_id(),
							branch_ancestry: scope.branch_ancestry(),
							access_bucket,
							db_size_pages: head.db_size_pages,
							loaded_pidx_rows: None,
							page_sources: BTreeMap::new(),
							source_blobs: BTreeMap::new(),
							cold_page_candidates: BTreeMap::new(),
							stale_pidx_pgnos: BTreeSet::new(),
						});
					}

					let mut pidx_by_pgno = BTreeMap::<u32, PageRef>::new();
					let mut loaded_pidx_rows = None;
					let cache_source = match &scope {
						StorageScope::Branch(plan) if plan.sources.len() == 1 => Some(plan.sources[0]),
						StorageScope::Legacy => Some(ReadSource::Legacy),
						StorageScope::Branch(_) => None,
					};
					if let (Some(cache_source), Some(cached_pidx)) =
						(cache_source, cached_pidx.as_ref())
					{
						for (pgno, txid) in cached_pidx {
							if let Some(txid) =
								txid.filter(|txid| *txid <= cache_source.max_txid(head.head_txid))
							{
								pidx_by_pgno.insert(
									*pgno,
									PageRef {
										source: cache_source,
										txid,
									},
								);
							}
						}
					} else {
						match &scope {
							StorageScope::Branch(plan) => {
								for source in &plan.sources {
									let rows =
										tx_scan_prefix_values(&tx, &source.pidx_prefix(&actor_id)).await?;
									let mut decoded_rows = Vec::new();
									for (key, value) in rows {
										let pgno = source.decode_pidx_pgno(&actor_id, &key)?;
										let txid = decode_pidx_txid(&value)?;
										if txid > source.max_txid(head.head_txid) {
											continue;
										}
										pidx_by_pgno.entry(pgno).or_insert(PageRef {
											source: *source,
											txid,
										});
										decoded_rows.push((pgno, txid));
									}
									if plan.sources.len() == 1 {
										loaded_pidx_rows = Some(decoded_rows);
									}
								}
							}
							StorageScope::Legacy => {
								let legacy_prefix = ReadSource::Legacy.pidx_prefix(&actor_id);
								let rows = tx_scan_prefix_values(&tx, &legacy_prefix).await?;
								let decoded_rows = rows
									.into_iter()
									.map(|(key, value)| {
										Ok((
											ReadSource::Legacy.decode_pidx_pgno(&actor_id, &key)?,
											decode_pidx_txid(&value)?,
										))
									})
									.collect::<Result<Vec<_>>>()?;
								let decoded_rows = decoded_rows
									.into_iter()
									.filter(|(_, txid)| *txid <= ReadSource::Legacy.max_txid(head.head_txid))
									.collect::<Vec<_>>();
								for (pgno, txid) in &decoded_rows {
									pidx_by_pgno.insert(
										*pgno,
										PageRef {
											source: ReadSource::Legacy,
											txid: *txid,
										},
									);
								}
								loaded_pidx_rows = Some(decoded_rows);
							}
						}
					}

					let mut page_sources = BTreeMap::new();
					let mut source_blobs = BTreeMap::new();
					let mut missing_delta_prefixes = BTreeSet::new();
					let mut shard_sources = BTreeMap::<u32, Option<(Vec<u8>, Vec<u8>)>>::new();
					let mut stale_pidx_pgnos = BTreeSet::new();
					let mut cold_page_candidates = BTreeMap::<u32, Vec<ColdLayerCandidate>>::new();

					for pgno in &pgnos_in_range {
						let mut cold_candidates = Vec::new();
						let preferred_delta = pidx_by_pgno
							.get(pgno)
							.copied()
							.map(|page_ref| {
								(
									page_ref.source.delta_chunk_prefix(&actor_id, page_ref.txid),
									page_ref.source,
									page_ref.txid,
								)
							});

						if preferred_delta
							.as_ref()
							.is_some_and(|(prefix, _, _)| missing_delta_prefixes.contains(prefix))
						{
							stale_pidx_pgnos.insert(*pgno);
						}

						if let Some((delta_prefix, delta_source, delta_txid)) = preferred_delta
							.as_ref()
							.filter(|(prefix, _, _)| !missing_delta_prefixes.contains(prefix))
						{
							if !source_blobs.contains_key(delta_prefix) {
								let blob = tx_load_delta_blob(&tx, delta_prefix).await?;
								if let Some(blob) = blob {
									source_blobs.insert(delta_prefix.clone(), blob);
								} else {
									missing_delta_prefixes.insert(delta_prefix.clone());
									stale_pidx_pgnos.insert(*pgno);
									if let ReadSource::Branch(source) = *delta_source {
										cold_candidates.push(ColdLayerCandidate {
											branch_id: source.branch_id,
											owner_txid: *delta_txid,
											shard_id: pgno / SHARD_SIZE,
										});
									}
								}
							}

							if source_blobs.contains_key(delta_prefix) {
								page_sources.insert(*pgno, delta_prefix.clone());
								continue;
							}

							if matches!(delta_source, ReadSource::Legacy | ReadSource::Branch(_)) {
								stale_pidx_pgnos.insert(*pgno);
							}
						}

						let shard_id = pgno / SHARD_SIZE;
						if !shard_sources.contains_key(&shard_id) {
							let source =
								tx_load_latest_shard_blob(&tx, &scope, &actor_id, shard_id, head.head_txid)
									.await?;
							shard_sources.insert(shard_id, source);
						}

						if let Some((source_key, blob)) = shard_sources
							.get(&shard_id)
							.cloned()
							.flatten()
						{
							if !source_blobs.contains_key(&source_key) {
								source_blobs.insert(source_key.clone(), blob);
							}
							page_sources.insert(*pgno, source_key);
						} else {
							cold_candidates.extend(scope.cold_layer_candidates(*pgno));
							if !cold_candidates.is_empty() {
								cold_page_candidates.insert(*pgno, cold_candidates);
							}
						}
					}

					let access_bucket = if let Some(branch_id) = scope.branch_id() {
						touch_access_if_bucket_advanced(
							&tx,
							branch_id,
							cached_access_bucket,
							now_ms,
						)
						.await?
					} else {
						None
					};

					Ok(GetPagesTxResult {
						branch_id: scope.branch_id(),
						branch_ancestry: scope.branch_ancestry(),
						access_bucket,
						db_size_pages: head.db_size_pages,
						loaded_pidx_rows,
						page_sources,
						source_blobs,
						cold_page_candidates,
						stale_pidx_pgnos,
					})
				}
			})
			.await?;

		let mut tx_result = tx_result;
		let cold_pages = self
			.load_cold_page_blobs(&tx_result.cold_page_candidates)
			.await?;
		for (pgno, (source_key, blob)) in cold_pages {
			tx_result.page_sources.insert(pgno, source_key.clone());
			tx_result.source_blobs.entry(source_key).or_insert(blob);
		}

		let mut stale_pidx_pgnos = tx_result.stale_pidx_pgnos;
		if let Some(loaded_pidx_rows) = tx_result.loaded_pidx_rows {
			metrics::SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL
				.with_label_values(labels)
				.inc();

			let loaded_index = DeltaPageIndex::new();
			for (pgno, txid) in loaded_pidx_rows {
				if !stale_pidx_pgnos.contains(&pgno) {
					loaded_index.insert(pgno, txid);
				}
			}

			let cache = self.cache.lock();
			for (pgno, txid) in loaded_index.range(0, u32::MAX) {
				cache.insert(pgno, txid);
			}
		}

		let mut decoded_blobs = BTreeMap::new();
		let mut pages = Vec::with_capacity(pgnos.len());
		let mut returned_bytes = 0u64;

		for pgno in pgnos {
			if pgno > tx_result.db_size_pages {
				pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let bytes = if let Some(source_key) = tx_result.page_sources.get(&pgno) {
				let blob = tx_result
					.source_blobs
					.get(source_key)
					.with_context(|| format!("missing source blob for page {pgno}"))?;

				if !decoded_blobs.contains_key(source_key) {
					let decoded = decode_ltx_v3(blob)
						.with_context(|| format!("decode source blob for page {pgno}"))?;
					decoded_blobs.insert(source_key.clone(), decoded);
				}

				let mut bytes = decoded_blobs
					.get(source_key)
					.and_then(|decoded: &DecodedLtx| decoded.get_page(pgno))
					.map(ToOwned::to_owned);
				if bytes.is_none() && source_key.starts_with(&keys::delta_prefix(&self.actor_id)) {
					stale_pidx_pgnos.insert(pgno);
				}
				if let Some(branch_id) = tx_result.branch_id {
					if bytes.is_none() && source_key.starts_with(&keys::branch_delta_prefix(branch_id)) {
						stale_pidx_pgnos.insert(pgno);
					}
				}
				bytes.get_or_insert_with(|| vec![0; PAGE_SIZE as usize]).clone()
			} else {
				vec![0; PAGE_SIZE as usize]
			};

			returned_bytes += bytes.len() as u64;
			pages.push(FetchedPage {
				pgno,
				bytes: Some(bytes),
			});
		}

		if !stale_pidx_pgnos.is_empty() {
			let cache = self.cache.lock();
			for pgno in stale_pidx_pgnos {
				cache.remove(pgno);
			}
		}

		*self.read_bytes_since_rollup.lock() += returned_bytes;
		if let Some(branch_id) = tx_result.branch_id {
			if cached_branch_id.is_some_and(|cached_branch_id| cached_branch_id != branch_id) {
				self.cache.lock().clear();
			}
			*self.branch_id.lock() = Some(branch_id);
			*self.ancestors.lock() = tx_result.branch_ancestry;
			if let Some(access_bucket) = tx_result.access_bucket {
				*self.last_access_bucket.lock() = Some(access_bucket);
			}
		} else {
			*self.ancestors.lock() = None;
		}

		Ok(pages)
	}
}

struct GetPagesTxResult {
	branch_id: Option<ActorBranchId>,
	branch_ancestry: Option<BranchAncestry>,
	access_bucket: Option<i64>,
	db_size_pages: u32,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	cold_page_candidates: BTreeMap<u32, Vec<ColdLayerCandidate>>,
	stale_pidx_pgnos: BTreeSet<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColdLayerCandidate {
	branch_id: ActorBranchId,
	owner_txid: u64,
	shard_id: u32,
}

impl ActorDb {
	async fn load_cold_page_blobs(
		&self,
		page_candidates: &BTreeMap<u32, Vec<ColdLayerCandidate>>,
	) -> Result<BTreeMap<u32, (Vec<u8>, Vec<u8>)>> {
		if page_candidates.is_empty() || self.cold_tier.is_none() {
			return Ok(BTreeMap::new());
		}

		let mut out = BTreeMap::new();
		let mut loaded_objects = BTreeMap::<String, Vec<u8>>::new();
		let mut decoded_objects = BTreeMap::<String, DecodedLtx>::new();

		for (pgno, candidates) in page_candidates {
			for candidate in candidates {
				for layer in self.find_cold_layers(*candidate).await? {
					let object_key = layer.object_key.clone();
					if !loaded_objects.contains_key(&object_key) {
						let Some(cold_tier) = &self.cold_tier else {
							continue;
						};
						let Some(bytes) = cold_tier
							.get_object(&object_key)
							.await
							.with_context(|| format!("get sqlite cold layer {object_key}"))?
						else {
							continue;
						};
						loaded_objects.insert(object_key.clone(), bytes);
					}

					if !decoded_objects.contains_key(&object_key) {
						let bytes = loaded_objects
							.get(&object_key)
							.expect("cold object should be loaded before decode");
						let decoded = decode_ltx_v3(bytes)
							.with_context(|| format!("decode sqlite cold layer {object_key}"))?;
						decoded_objects.insert(object_key.clone(), decoded);
					}

					if decoded_objects
						.get(&object_key)
						.and_then(|decoded| decoded.get_page(*pgno))
						.is_none()
					{
						continue;
					}

					let bytes = loaded_objects
						.get(&object_key)
						.expect("cold object should be loaded before return")
						.clone();
					out.insert(*pgno, (cold_source_key(&object_key), bytes));
					break;
				}
				if out.contains_key(pgno) {
					break;
				}
			}
		}

		Ok(out)
	}

	async fn find_cold_layers(
		&self,
		candidate: ColdLayerCandidate,
	) -> Result<Vec<LayerEntry>> {
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

	async fn load_cold_manifest(
		&self,
		branch_id: ActorBranchId,
	) -> Result<CachedColdManifest> {
		if let Some(manifest) = self.cold_manifest_cache.lock().get(branch_id) {
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
			chunks.push(
				decode_cold_manifest_chunk(&chunk_bytes).with_context(|| {
					format!("decode sqlite cold manifest chunk {}", chunk_ref.object_key)
				})?,
			);
		}

		let manifest = CachedColdManifest { chunks };
		self.cold_manifest_cache
			.lock()
			.insert(branch_id, manifest.clone());

		Ok(manifest)
	}
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

fn cold_manifest_index_object_key(branch_id: ActorBranchId) -> String {
	format!("db/{}/cold_manifest/index.bare", branch_id.as_uuid().simple())
}

fn now_ms() -> Result<i64> {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system time is before unix epoch")?;
	i64::try_from(duration.as_millis()).context("current time exceeded i64 milliseconds")
}

#[derive(Debug, Clone)]
enum StorageScope {
	Branch(BranchReadPlan),
	Legacy,
}

impl StorageScope {
	fn branch_id(&self) -> Option<ActorBranchId> {
		match self {
			Self::Branch(plan) => Some(plan.branch_id),
			Self::Legacy => None,
		}
	}

	fn branch_ancestry(&self) -> Option<BranchAncestry> {
		match self {
			Self::Branch(plan) => Some(plan.ancestry.clone()),
			Self::Legacy => None,
		}
	}

	fn cold_layer_candidates(
		&self,
		pgno: u32,
	) -> Vec<ColdLayerCandidate> {
		match self {
			Self::Branch(plan) => plan
				.sources
				.iter()
				.filter_map(|source| match source {
					ReadSource::Branch(source) => Some(ColdLayerCandidate {
						branch_id: source.branch_id,
						owner_txid: source.max_txid,
						shard_id: pgno / SHARD_SIZE,
					}),
					ReadSource::Legacy => None,
				})
				.collect(),
			Self::Legacy => {
				Vec::new()
			}
		}
	}
}

#[derive(Debug, Clone)]
struct BranchReadPlan {
	branch_id: ActorBranchId,
	head: DBHead,
	ancestry: BranchAncestry,
	sources: Vec<ReadSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ReadSource {
	Branch(BranchSource),
	Legacy,
}

impl ReadSource {
	fn pidx_prefix(self, actor_id: &str) -> Vec<u8> {
		match self {
			Self::Branch(source) => keys::branch_pidx_prefix(source.branch_id),
			Self::Legacy => keys::pidx_delta_prefix(actor_id),
		}
	}

	fn decode_pidx_pgno(self, actor_id: &str, key: &[u8]) -> Result<u32> {
		match self {
			Self::Branch(source) => decode_branch_pidx_pgno(source.branch_id, key),
			Self::Legacy => decode_pidx_pgno(actor_id, key),
		}
	}

	fn delta_chunk_prefix(self, actor_id: &str, txid: u64) -> Vec<u8> {
		match self {
			Self::Branch(source) => keys::branch_delta_chunk_prefix(source.branch_id, txid),
			Self::Legacy => keys::delta_chunk_prefix(actor_id, txid),
		}
	}

	fn max_txid(self, legacy_as_of_txid: u64) -> u64 {
		match self {
			Self::Branch(source) => source.max_txid,
			Self::Legacy => legacy_as_of_txid,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BranchSource {
	branch_id: ActorBranchId,
	max_txid: u64,
}

#[derive(Debug, Clone, Copy)]
struct PageRef {
	source: ReadSource,
	txid: u64,
}

async fn resolve_storage_scope(
	tx: &universaldb::Transaction,
	namespace_id: crate::pump::types::NamespaceId,
	actor_id: &str,
	cached_ancestry: Option<&BranchAncestry>,
) -> Result<StorageScope> {
	Ok(
		match branch::resolve_actor_branch(tx, namespace_id, actor_id, Snapshot).await? {
			Some(branch_id) => {
				StorageScope::Branch(load_branch_read_plan(tx, branch_id, cached_ancestry).await?)
			}
			None => StorageScope::Legacy,
		},
	)
}

async fn load_branch_read_plan(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	cached_ancestry: Option<&BranchAncestry>,
) -> Result<BranchReadPlan> {
	let head_bytes = tx_get_value(tx, &keys::branch_meta_head_key(branch_id)).await?;
	let head = if let Some(head_bytes) = head_bytes {
		decode_db_head(&head_bytes)?
	} else {
		let head_at_fork_bytes = tx_get_value(tx, &keys::branch_meta_head_at_fork_key(branch_id))
			.await?
			.ok_or(SqliteStorageError::MetaMissing {
				operation: "get_pages",
			})?;
		decode_db_head(&head_at_fork_bytes)?
	};

	let ancestry = if let Some(cached_ancestry) =
		cached_ancestry.filter(|ancestry| ancestry.root_branch_id == branch_id)
	{
		cached_ancestry.clone()
	} else {
		load_branch_ancestry(tx, branch_id).await?
	};

	let mut sources = Vec::new();
	for ancestor in &ancestry.ancestors {
		let max_txid = match ancestor.parent_versionstamp_cap {
			Some(parent_versionstamp) => {
				lookup_txid_for_read(tx, ancestor.branch_id, parent_versionstamp).await?
			}
			None => head.head_txid,
		};
		sources.push(ReadSource::Branch(BranchSource {
			branch_id: ancestor.branch_id,
			max_txid,
		}));
	}

	Ok(BranchReadPlan {
		branch_id,
		head,
		ancestry,
		sources,
	})
}

async fn lookup_txid_for_read(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = tx_get_value(tx, &keys::branch_vtx_key(branch_id, versionstamp))
		.await?
		.ok_or(SqliteStorageError::BookmarkExpired)?;
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
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

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		universaldb::RangeOption {
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

async fn tx_load_delta_blob(
	tx: &universaldb::Transaction,
	delta_prefix: &[u8],
) -> Result<Option<Vec<u8>>> {
	let delta_chunks = tx_scan_prefix_values(tx, delta_prefix).await?;
	if delta_chunks.is_empty() {
		return Ok(None);
	}

	let mut delta_blob = Vec::new();
	for (_, chunk) in delta_chunks {
		delta_blob.extend_from_slice(&chunk);
	}

	Ok(Some(delta_blob))
}

async fn tx_load_latest_shard_blob(
	tx: &universaldb::Transaction,
	scope: &StorageScope,
	actor_id: &str,
	shard_id: u32,
	legacy_as_of_txid: u64,
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	let sources = match scope {
		StorageScope::Branch(plan) => plan.sources.clone(),
		StorageScope::Legacy => vec![ReadSource::Legacy],
	};

	for source in sources {
		let as_of_txid = match source {
			ReadSource::Branch(source) => source.max_txid,
			ReadSource::Legacy => legacy_as_of_txid,
		};
		let prefix = match source {
			ReadSource::Branch(source) => {
				keys::branch_shard_version_prefix(source.branch_id, shard_id)
			}
			ReadSource::Legacy => keys::shard_version_prefix(actor_id, shard_id),
		};
		let end_key = match source {
			ReadSource::Branch(source) => {
				keys::branch_shard_key(source.branch_id, shard_id, as_of_txid)
			}
			ReadSource::Legacy => keys::shard_version_key(actor_id, shard_id, as_of_txid),
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

		if matches!(source, ReadSource::Legacy) {
			let legacy_key = keys::shard_key(actor_id, shard_id);
			if let Some(value) = tx
				.informal()
				.get(&legacy_key, Snapshot)
				.await?
			{
				return Ok(Some((legacy_key, value.to_vec())));
			}
		}
	}

	Ok(None)
}

fn decode_branch_pidx_pgno(branch_id: ActorBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	ensure!(
		key.starts_with(&prefix),
		"branch pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"branch pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("branch pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == PIDX_TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		PIDX_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}
