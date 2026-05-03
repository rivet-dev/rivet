//! Page read path for the stateless depot conveyer.

mod cache;
pub(super) mod cache_fill;
mod cold;
mod pidx;
mod plan;
mod shard;
mod tx;

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "test-faults")]
use crate::fault::{
	DepotFaultAction, DepotFaultContext, DepotFaultController, DepotFaultFired, DepotFaultPoint,
	ReadFaultPoint,
};
use anyhow::{Context, Result, ensure};

use crate::conveyer::{
	Db,
	db::{BranchAncestry, CacheSnapshot, touch_access_if_bucket_advanced},
	error::SqliteStorageError,
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{DecodedLtx, decode_ltx_v3},
	metrics,
	page_index::DeltaPageIndex,
	types::{DatabaseBranchId, FetchedPage},
};

use self::{
	cold::{ColdLayerCandidate, ColdPageCandidate, tx_load_latest_compaction_cold_ref},
	pidx::{PageRef, decode_pidx_txid},
	plan::{ReadSource, StorageScope, resolve_storage_scope},
	shard::{tx_load_delta_blob, tx_load_latest_shard_blob},
	tx::tx_scan_prefix_values,
};

impl Db {
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

		let cached_snapshot = self.cache_snapshot.read().await.clone();
		let cached_pidx = cached_snapshot
			.as_ref()
			.and_then(|snapshot| cache::snapshot_pidx_cache(&snapshot.pidx, &pgnos));
		let cached_branch_id = cached_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let cached_ancestry = cached_snapshot
			.as_ref()
			.map(|snapshot| snapshot.ancestors.clone());
		let cached_access_bucket = cached_snapshot.and_then(|snapshot| snapshot.last_access_bucket);

		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let pgnos_for_tx = pgnos.clone();
		let now_ms = cache::now_ms()?;
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();
		let tx_result = self
			.udb
			.run(move |tx| {
				let database_id = database_id.clone();
				let bucket_id = bucket_id;
				let pgnos = pgnos_for_tx.clone();
				let cached_pidx = cached_pidx.clone();
				let cached_ancestry = cached_ancestry.clone();
				let cached_access_bucket = cached_access_bucket;
				#[cfg(feature = "test-faults")]
				let fault_controller = fault_controller.clone();

				async move {
					#[cfg(feature = "test-faults")]
					maybe_fire_read_fault(
						&fault_controller,
						ReadFaultPoint::BeforeScopeResolve,
						&database_id,
						None,
						None,
						None,
					)
					.await?;
					let scope = resolve_storage_scope(
						&tx,
						bucket_id,
						&database_id,
						cached_ancestry.as_ref(),
					)
					.await?;
					let cached_pidx = if cached_branch_id == Some(scope.branch_id()) {
						cached_pidx
					} else {
						None
					};
					let StorageScope::Branch(plan) = &scope;
					let head = plan.head.clone();
					#[cfg(feature = "test-faults")]
					maybe_fire_read_fault(
						&fault_controller,
						ReadFaultPoint::AfterScopeResolve,
						&database_id,
						Some(scope.branch_id()),
						None,
						None,
					)
					.await?;

					let pgnos_in_range = pgnos
						.into_iter()
						.filter(|pgno| *pgno <= head.db_size_pages)
						.collect::<Vec<_>>();
					if pgnos_in_range.is_empty() {
						let branch_id = scope.branch_id();
						return Ok(GetPagesTxResult {
							branch_id,
							branch_ancestry: scope.branch_ancestry(),
							access_bucket: None,
							db_size_pages: head.db_size_pages,
							loaded_pidx_rows: None,
							page_sources: BTreeMap::new(),
							source_blobs: BTreeMap::new(),
							shard_cache_read_outcomes: BTreeMap::new(),
							cold_page_candidates: BTreeMap::new(),
							stale_pidx_pgnos: BTreeSet::new(),
						});
					}

					let mut pidx_by_pgno = BTreeMap::<u32, PageRef>::new();
					let mut loaded_pidx_rows = None;
					let cache_source = cache::cache_source_for_scope(&scope);
					if let (Some(cache_source), Some(cached_pidx)) =
						(cache_source, cached_pidx.as_ref())
					{
						for (pgno, txid) in cached_pidx {
							if let Some(txid) = txid.filter(|txid| *txid <= cache_source.max_txid())
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
						let StorageScope::Branch(plan) = &scope;
						for source in &plan.sources {
							let rows =
								tx_scan_prefix_values(&tx, &source.pidx_prefix(&database_id))
									.await?;
							let mut decoded_rows = Vec::new();
							for (key, value) in rows {
								let pgno = source.decode_pidx_pgno(&database_id, &key)?;
								let txid = decode_pidx_txid(&value)?;
								if txid > source.max_txid() {
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
						for (pgno, page_ref) in fill_historical_delta_refs(
							&tx,
							&database_id,
							&plan.sources[1..],
							&pgnos_in_range,
							&pidx_by_pgno,
						)
						.await?
						{
							pidx_by_pgno.entry(pgno).or_insert(page_ref);
						}
					}
					#[cfg(feature = "test-faults")]
					for pgno in &pgnos_in_range {
						maybe_fire_read_fault(
							&fault_controller,
							ReadFaultPoint::AfterPidxScan,
							&database_id,
							Some(scope.branch_id()),
							Some(*pgno),
							Some(*pgno / SHARD_SIZE),
						)
						.await?;
					}

					let mut page_sources = BTreeMap::new();
					let mut source_blobs = BTreeMap::new();
					let mut missing_delta_prefixes = BTreeSet::new();
					let mut shard_sources = BTreeMap::<u32, Option<(Vec<u8>, Vec<u8>)>>::new();
					let mut stale_pidx_pgnos = BTreeSet::new();
					let mut cold_page_candidates = BTreeMap::<u32, Vec<ColdPageCandidate>>::new();
					let mut shard_cache_read_outcomes =
						BTreeMap::<u32, ShardCacheReadOutcome>::new();
					let mut touched_cache_backed_page = false;

					for pgno in &pgnos_in_range {
						let mut cold_candidates = Vec::new();
						let preferred_delta = pidx_by_pgno.get(pgno).copied().map(|page_ref| {
							(
								page_ref
									.source
									.delta_chunk_prefix(&database_id, page_ref.txid),
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
								#[cfg(feature = "test-faults")]
								let mut blob = blob;
								#[cfg(feature = "test-faults")]
								if matches!(
									maybe_fire_read_fault(
										&fault_controller,
										if blob.is_some() {
											ReadFaultPoint::AfterDeltaBlobLoad
										} else {
											ReadFaultPoint::DeltaBlobMissing
										},
										&database_id,
										Some(scope.branch_id()),
										Some(*pgno),
										Some(*pgno / SHARD_SIZE),
									)
									.await?,
									Some(DepotFaultFired {
										action: DepotFaultAction::DropArtifact,
										..
									})
								) {
									blob = None;
								}
								if let Some(blob) = blob {
									source_blobs.insert(delta_prefix.clone(), blob);
								} else {
									missing_delta_prefixes.insert(delta_prefix.clone());
									stale_pidx_pgnos.insert(*pgno);
									let ReadSource::Branch(source) = *delta_source;
									cold_candidates.push(
										ColdLayerCandidate {
											branch_id: source.branch_id,
											owner_txid: *delta_txid,
											shard_id: pgno / SHARD_SIZE,
										}
										.into(),
									);
								}
							}

							if source_blobs.contains_key(delta_prefix) {
								page_sources.insert(*pgno, delta_prefix.clone());
								continue;
							}

							stale_pidx_pgnos.insert(*pgno);
						}

						let shard_id = pgno / SHARD_SIZE;
						if !shard_sources.contains_key(&shard_id) {
							let source = tx_load_latest_shard_blob(&tx, &scope, shard_id).await?;
							#[cfg(feature = "test-faults")]
							let mut source = source;
							#[cfg(feature = "test-faults")]
							if matches!(
								maybe_fire_read_fault(
									&fault_controller,
									ReadFaultPoint::AfterShardBlobLoad,
									&database_id,
									Some(scope.branch_id()),
									Some(*pgno),
									Some(shard_id),
								)
								.await?,
								Some(DepotFaultFired {
									action: DepotFaultAction::DropArtifact,
									..
								})
							) {
								source = None;
							}
							shard_sources.insert(shard_id, source);
						}

						if let Some((source_key, blob)) =
							shard_sources.get(&shard_id).cloned().flatten()
						{
							if !source_blobs.contains_key(&source_key) {
								source_blobs.insert(source_key.clone(), blob);
							}
							page_sources.insert(*pgno, source_key);
							shard_cache_read_outcomes.insert(*pgno, ShardCacheReadOutcome::FdbHit);
							touched_cache_backed_page = true;
						} else {
							if let Some(reference) =
								tx_load_latest_compaction_cold_ref(&tx, &scope, shard_id).await?
							{
								#[cfg(feature = "test-faults")]
								let drop_ref = matches!(
									maybe_fire_read_fault(
										&fault_controller,
										ReadFaultPoint::ColdRefSelected,
										&database_id,
										Some(scope.branch_id()),
										Some(*pgno),
										Some(shard_id),
									)
									.await?,
									Some(DepotFaultFired {
										action: DepotFaultAction::DropArtifact,
										..
									})
								);
								#[cfg(not(feature = "test-faults"))]
								let drop_ref = false;
								if !drop_ref {
									cold_candidates.push(reference.into());
								}
								touched_cache_backed_page = true;
							}
							cold_candidates.extend(
								scope
									.cold_layer_candidates(*pgno)
									.into_iter()
									.map(ColdPageCandidate::from),
							);
							if !cold_candidates.is_empty() {
								cold_page_candidates.insert(*pgno, cold_candidates);
							}
						}
					}

					let branch_id = scope.branch_id();
					let access_bucket = if touched_cache_backed_page {
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
						branch_id,
						branch_ancestry: scope.branch_ancestry(),
						access_bucket,
						db_size_pages: head.db_size_pages,
						loaded_pidx_rows,
						page_sources,
						source_blobs,
						shard_cache_read_outcomes,
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
		let shard_cache_fill_jobs = cold_pages.shard_cache_fills;
		for (pgno, (source_key, blob)) in cold_pages.pages {
			tx_result.page_sources.insert(pgno, source_key.clone());
			tx_result.source_blobs.entry(source_key).or_insert(blob);
			tx_result
				.shard_cache_read_outcomes
				.insert(pgno, ShardCacheReadOutcome::ColdHit);
		}

		let mut stale_pidx_pgnos = tx_result.stale_pidx_pgnos;

		let mut decoded_blobs = BTreeMap::new();
		let mut pages = Vec::with_capacity(pgnos.len());
		let mut returned_bytes = 0u64;

		for pgno in pgnos {
			#[cfg(feature = "test-faults")]
			maybe_fire_read_fault(
				&self.fault_controller,
				ReadFaultPoint::BeforeReturnPages,
				&self.database_id,
				Some(tx_result.branch_id),
				Some(pgno),
				Some(pgno / SHARD_SIZE),
			)
			.await?;
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
				if bytes.is_none()
					&& source_key.starts_with(&keys::branch_delta_prefix(tx_result.branch_id))
				{
					stale_pidx_pgnos.insert(pgno);
				}
				bytes
					.get_or_insert_with(|| vec![0; PAGE_SIZE as usize])
					.clone()
			} else {
				if stale_pidx_pgnos.contains(&pgno) {
					return Err(SqliteStorageError::ShardCoverageMissing { pgno }.into());
				}
				tx_result
					.shard_cache_read_outcomes
					.entry(pgno)
					.or_insert(ShardCacheReadOutcome::Miss);
				vec![0; PAGE_SIZE as usize]
			};
			if let Some(outcome) = tx_result.shard_cache_read_outcomes.get(&pgno) {
				metrics::SQLITE_SHARD_CACHE_READ_TOTAL
					.with_label_values(&[outcome.as_label()])
					.inc();
			}

			returned_bytes += bytes.len() as u64;
			pages.push(FetchedPage {
				pgno,
				bytes: Some(bytes),
			});
		}

		self.read_bytes_since_rollup
			.fetch_add(returned_bytes, std::sync::atomic::Ordering::Relaxed);
		let mut cache_snapshot = self.cache_snapshot.write().await;
		let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let publish_branch_changed =
			cache::branch_cache_changed(current_branch_id, tx_result.branch_id);
		let pidx = if publish_branch_changed {
			std::sync::Arc::new(DeltaPageIndex::new())
		} else {
			cache_snapshot
				.as_ref()
				.map(|snapshot| std::sync::Arc::clone(&snapshot.pidx))
				.unwrap_or_else(|| std::sync::Arc::new(DeltaPageIndex::new()))
		};
		if let Some(loaded_pidx_rows) = tx_result.loaded_pidx_rows.take() {
			metrics::SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL
				.with_label_values(labels)
				.inc();

			cache::store_loaded_pidx_rows(&pidx, loaded_pidx_rows, &stale_pidx_pgnos);
		}
		if !stale_pidx_pgnos.is_empty() {
			cache::clear_stale_pidx_rows(&pidx, stale_pidx_pgnos);
		}
		let last_access_bucket = tx_result.access_bucket.or_else(|| {
			cache_snapshot
				.as_ref()
				.filter(|snapshot| snapshot.branch_id == tx_result.branch_id)
				.and_then(|snapshot| snapshot.last_access_bucket)
		});
		*cache_snapshot = Some(CacheSnapshot {
			branch_id: tx_result.branch_id,
			ancestors: tx_result.branch_ancestry,
			last_access_bucket,
			pidx,
		});

		#[cfg(feature = "test-faults")]
		let mut shard_cache_fill_jobs = shard_cache_fill_jobs;
		#[cfg(feature = "test-faults")]
		{
			let mut filtered_jobs = Vec::with_capacity(shard_cache_fill_jobs.len());
			for job in shard_cache_fill_jobs {
				let key = job.key();
				let fired = maybe_fire_read_fault(
					&self.fault_controller,
					ReadFaultPoint::ShardCacheFillEnqueue,
					&self.database_id,
					Some(key.branch_id),
					None,
					Some(key.shard_id),
				)
				.await?;
				if !matches!(
					fired,
					Some(DepotFaultFired {
						action: DepotFaultAction::DropArtifact,
						..
					})
				) {
					filtered_jobs.push(job);
				}
			}
			shard_cache_fill_jobs = filtered_jobs;
		}

		self.shard_cache_fill.enqueue_many(shard_cache_fill_jobs);

		Ok(pages)
	}
}

struct GetPagesTxResult {
	branch_id: DatabaseBranchId,
	branch_ancestry: BranchAncestry,
	access_bucket: Option<i64>,
	db_size_pages: u32,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	shard_cache_read_outcomes: BTreeMap<u32, ShardCacheReadOutcome>,
	cold_page_candidates: BTreeMap<u32, Vec<ColdPageCandidate>>,
	stale_pidx_pgnos: BTreeSet<u32>,
}

#[derive(Clone, Copy)]
enum ShardCacheReadOutcome {
	FdbHit,
	ColdHit,
	Miss,
}

impl ShardCacheReadOutcome {
	fn as_label(self) -> &'static str {
		match self {
			ShardCacheReadOutcome::FdbHit => metrics::SHARD_CACHE_READ_FDB_HIT,
			ShardCacheReadOutcome::ColdHit => metrics::SHARD_CACHE_READ_COLD_HIT,
			ShardCacheReadOutcome::Miss => metrics::SHARD_CACHE_READ_MISS,
		}
	}
}

async fn fill_historical_delta_refs(
	tx: &universaldb::Transaction,
	database_id: &str,
	capped_sources: &[ReadSource],
	pgnos: &[u32],
	pidx_by_pgno: &BTreeMap<u32, PageRef>,
) -> Result<BTreeMap<u32, PageRef>> {
	let mut missing_pgnos = pgnos
		.iter()
		.copied()
		.filter(|pgno| !pidx_by_pgno.contains_key(pgno))
		.collect::<BTreeSet<_>>();
	let mut refs = BTreeMap::new();

	for source in capped_sources {
		if missing_pgnos.is_empty() {
			break;
		}

		let mut chunks_by_txid = BTreeMap::<u64, Vec<(u32, Vec<u8>)>>::new();
		for (key, chunk) in tx_scan_prefix_values(tx, &source.delta_prefix(database_id)).await? {
			let txid = source.decode_delta_chunk_txid(database_id, &key)?;
			if txid > source.max_txid() {
				continue;
			}
			let chunk_idx = source.decode_delta_chunk_idx(database_id, txid, &key)?;
			chunks_by_txid
				.entry(txid)
				.or_default()
				.push((chunk_idx, chunk));
		}

		for (txid, mut chunks) in chunks_by_txid.into_iter().rev() {
			if missing_pgnos.is_empty() {
				break;
			}

			chunks.sort_by_key(|(chunk_idx, _)| *chunk_idx);
			let mut blob = Vec::new();
			for (_, chunk) in chunks {
				blob.extend_from_slice(&chunk);
			}
			let decoded = decode_ltx_v3(&blob)
				.with_context(|| format!("decode historical sqlite delta {txid}"))?;
			let found_pgnos = missing_pgnos
				.iter()
				.copied()
				.filter(|pgno| decoded.get_page(*pgno).is_some())
				.collect::<Vec<_>>();
			for pgno in found_pgnos {
				refs.insert(
					pgno,
					PageRef {
						source: *source,
						txid,
					},
				);
				missing_pgnos.remove(&pgno);
			}
		}
	}

	Ok(refs)
}

#[cfg(feature = "test-faults")]
pub(super) async fn maybe_fire_read_fault(
	fault_controller: &Option<DepotFaultController>,
	point: ReadFaultPoint,
	database_id: &str,
	database_branch_id: Option<DatabaseBranchId>,
	page_number: Option<u32>,
	shard_id: Option<u32>,
) -> Result<Option<DepotFaultFired>> {
	let Some(controller) = fault_controller else {
		return Ok(None);
	};
	let mut context = DepotFaultContext::new().database_id(database_id);
	if let Some(database_branch_id) = database_branch_id {
		context = context.database_branch_id(database_branch_id);
	}
	if let Some(page_number) = page_number {
		context = context.page_number(page_number);
	}
	if let Some(shard_id) = shard_id {
		context = context.shard_id(shard_id);
	}

	controller
		.maybe_fire(DepotFaultPoint::Read(point), context)
		.await
}
