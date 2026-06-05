//! Page read path for the stateless depot conveyer.

mod cache;
mod pidx;
mod plan;
mod shard;
mod tx;

use std::{
	collections::{BTreeMap, BTreeSet},
	time::Instant,
};

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
	types::{
		DatabaseBranchId, FetchedPage, GetPagesOptions, GetPagesResult, PageSourceCandidate,
		PageSourceCandidateResult, PageSourceKind, PageSourceProvenance, decode_commit_row,
	},
};

use self::{
	pidx::{PageRef, PageRefKind, decode_pidx_txid},
	plan::{ReadSource, StorageScope, resolve_storage_scope},
	shard::{tx_load_delta_blob, tx_load_latest_shard_blob},
	tx::{tx_get_value, tx_scan_prefix_values},
};

impl Db {
	pub async fn get_pages(&self, pgnos: Vec<u32>) -> Result<Vec<FetchedPage>> {
		self.get_pages_with_metadata(pgnos)
			.await
			.map(|result| result.pages)
	}

	pub async fn get_pages_with_metadata(&self, pgnos: Vec<u32>) -> Result<GetPagesResult> {
		self.get_pages_with_options(pgnos, GetPagesOptions::default())
			.await
	}

	pub async fn get_pages_with_options(
		&self,
		pgnos: Vec<u32>,
		options: GetPagesOptions,
	) -> Result<GetPagesResult> {
		let node_id = self.node_id.to_string();
		let labels = &[node_id.as_str()];
		let _timer = metrics::SQLITE_PUMP_GET_PAGES_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_GET_PAGES_PGNO_COUNT
			.with_label_values(labels)
			.observe(pgnos.len() as f64);
		let allow_side_effects = options.mode.allows_side_effects();
		let read_started_at = Instant::now();

		for pgno in &pgnos {
			ensure!(*pgno > 0, "get_pages does not accept page 0");
		}

		let phase_start = Instant::now();
		let cached_snapshot = if allow_side_effects {
			self.cache_snapshot.read().await.clone()
		} else {
			None
		};
		metrics::observe_get_pages_phase(&node_id, "cache_snapshot", phase_start, "ok");
		#[cfg(feature = "pidx-cache")]
		let cached_pidx = cached_snapshot
			.as_ref()
			.and_then(|snapshot| cache::snapshot_pidx_cache(&snapshot.pidx, &pgnos));
		#[cfg(not(feature = "pidx-cache"))]
		let cached_pidx = None::<BTreeMap<u32, Option<u64>>>;
		let cached_branch_id = cached_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let cached_ancestry = cached_snapshot
			.as_ref()
			.map(|snapshot| snapshot.ancestors.clone());
		let cached_access_bucket = cached_snapshot.and_then(|snapshot| snapshot.last_access_bucket);

		let database_id = self.database_id.clone();
		let database_id_for_log = database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let pgnos_for_tx = pgnos.clone();
		let now_ms = cache::now_ms()?;
		let expected_head_txid = options.expected_head_txid;
		let read_mode = options.mode;
		let diagnostic_max_txid = options.diagnostic_max_txid;
		let collect_provenance = options.collect_provenance;
		let phase_node_id = node_id.clone();
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();
		let tx_result = self
			.udb
			.txn("depot_get_pages", move |tx| {
				let phase_node_id = phase_node_id.clone();
				let database_id = database_id.clone();
				let bucket_id = bucket_id;
				let pgnos = pgnos_for_tx.clone();
				let cached_pidx = cached_pidx.clone();
				let cached_ancestry = cached_ancestry.clone();
				let cached_access_bucket = cached_access_bucket;
				let expected_head_txid = expected_head_txid;
				let read_mode = read_mode;
				let diagnostic_max_txid = diagnostic_max_txid;
				let collect_provenance = collect_provenance;
				#[cfg(feature = "test-faults")]
				let fault_controller = fault_controller.clone();

				async move {
					let mut debug = GetPagesDebug::default();
					debug.pages_requested = pgnos.len();
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
					let phase_start = Instant::now();
					let mut scope = resolve_storage_scope(
						&tx,
						bucket_id,
						&database_id,
						cached_ancestry.as_ref(),
					)
					.await?;
					metrics::observe_get_pages_phase(
						&phase_node_id,
						"resolve_scope",
						phase_start,
						"ok",
					);
					let cached_pidx = if cached_branch_id == Some(scope.branch_id()) {
						cached_pidx
					} else {
						None
					};
					let mut diagnostic_current_source_delta_fallback = false;
					if let Some(max_txid) = diagnostic_max_txid {
						ensure!(
							!read_mode.allows_side_effects(),
							"diagnostic max txid is only valid in no-side-effects mode"
						);
						let StorageScope::Branch(plan) = &mut scope;
						ensure!(
							max_txid <= plan.head.head_txid,
							"diagnostic max txid exceeded current head txid"
						);
						diagnostic_current_source_delta_fallback = max_txid < plan.head.head_txid;
						let commit_bytes = tx_get_value(
							&tx,
							&keys::branch_commit_key(plan.branch_id, max_txid),
						)
						.await?
						.context("diagnostic max txid commit row is missing")?;
						let commit = decode_commit_row(&commit_bytes)
							.context("decode diagnostic max txid commit row")?;
						plan.head.head_txid = max_txid;
						plan.head.db_size_pages = commit.db_size_pages;
						for source in &mut plan.sources {
							let ReadSource::Branch(source) = source;
							source.max_txid = source.max_txid.min(max_txid);
						}
					}
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
					if let Some(expected_head_txid) = expected_head_txid {
						if expected_head_txid != head.head_txid {
							tracing::error!(
								%database_id,
								branch_id = ?scope.branch_id(),
								expected_head_txid,
								actual_head_txid = head.head_txid,
								"sqlite head fence mismatch while reading; this indicates multiple actor instances are accessing the same sqlite database in parallel, which is incorrect actor lifecycle behavior"
							);
							return Err(SqliteStorageError::HeadFenceMismatch {
								expected_head_txid,
								actual_head_txid: head.head_txid,
							}
							.into());
						}
					}

					let pgnos_in_range = pgnos
						.into_iter()
						.filter(|pgno| *pgno <= head.db_size_pages)
						.collect::<Vec<_>>();
					debug.pages_in_range = pgnos_in_range.len();
					if pgnos_in_range.is_empty() {
						let branch_id = scope.branch_id();
						return Ok(GetPagesTxResult {
							branch_id,
							branch_ancestry: scope.branch_ancestry(),
							access_bucket: None,
							head_txid: head.head_txid,
							db_size_pages: head.db_size_pages,
							loaded_pidx_rows: None,
							page_sources: BTreeMap::new(),
							source_blobs: BTreeMap::new(),
							page_candidates: BTreeMap::new(),
							selected_candidates: BTreeMap::new(),
							shard_cache_read_outcomes: BTreeMap::new(),
							stale_pidx_pgnos: BTreeSet::new(),
							debug,
						});
					}

					let phase_start = Instant::now();
					let mut pidx_by_pgno = BTreeMap::<u32, PageRef>::new();
					let mut loaded_pidx_rows = None;
					let cache_source = cache::cache_source_for_scope(&scope);
					if let (Some(cache_source), Some(cached_pidx)) =
						(cache_source, cached_pidx.as_ref())
					{
						debug.pidx_cache_hit = true;
						debug.pidx_cache_rows_used = cached_pidx.len();
						for (pgno, txid) in cached_pidx {
							if let Some(txid) = txid.filter(|txid| *txid <= cache_source.max_txid())
							{
								pidx_by_pgno.insert(
									*pgno,
									PageRef {
										source: cache_source,
										txid,
										kind: PageRefKind::Pidx,
									},
								);
							}
						}
					} else {
						debug.pidx_cache_hit = false;
						let StorageScope::Branch(plan) = &scope;
						for source in &plan.sources {
							let rows =
								tx_scan_prefix_values(&tx, &source.pidx_prefix(&database_id))
									.await?;
							debug.pidx_sources_scanned += 1;
							debug.pidx_rows_scanned += rows.len();
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
									kind: PageRefKind::Pidx,
								});
								decoded_rows.push((pgno, txid));
							}
							if plan.sources.len() == 1 {
								loaded_pidx_rows = Some(decoded_rows);
							}
						}
						let historical_delta_sources = if diagnostic_current_source_delta_fallback
							&& !read_mode.allows_side_effects()
						{
							&plan.sources[..]
						} else {
							&plan.sources[1..]
						};
						for (pgno, page_ref) in fill_historical_delta_refs(
							&tx,
							&database_id,
							historical_delta_sources,
							&pgnos_in_range,
							&pidx_by_pgno,
							&mut debug,
						)
						.await?
						{
							pidx_by_pgno.entry(pgno).or_insert(page_ref);
						}
					}
					metrics::observe_get_pages_phase(
						&phase_node_id,
						"pidx_lookup",
						phase_start,
						"ok",
					);
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
					let mut page_candidates = BTreeMap::<u32, Vec<PageSourceCandidate>>::new();
					let mut selected_candidates = BTreeMap::<u32, PageSourceCandidate>::new();
					let mut missing_delta_prefixes = BTreeSet::new();
					let mut shard_sources = BTreeMap::<u32, Option<(Vec<u8>, Vec<u8>)>>::new();
					let mut stale_pidx_pgnos = BTreeSet::new();
					let mut shard_cache_read_outcomes =
						BTreeMap::<u32, ShardCacheReadOutcome>::new();
					let mut touched_cache_backed_page = false;

					let phase_start = Instant::now();
					for pgno in &pgnos_in_range {
						let preferred_delta = pidx_by_pgno.get(pgno).copied().map(|page_ref| {
							(
								page_ref
									.source
									.delta_chunk_prefix(&database_id, page_ref.txid),
								page_ref.source,
								page_ref.txid,
								page_ref.kind,
							)
						});

						if preferred_delta
							.as_ref()
							.is_some_and(|(prefix, _, _, _)| missing_delta_prefixes.contains(prefix))
						{
							stale_pidx_pgnos.insert(*pgno);
							if collect_provenance {
								let (_, _, txid, kind) = preferred_delta.as_ref().expect("checked above");
								page_candidates.entry(*pgno).or_default().push(PageSourceCandidate {
									kind: missing_delta_kind(*kind),
									txid: Some(*txid),
									shard_id: None,
									result: PageSourceCandidateResult::Lost,
									reason: Some("delta_blob_missing".to_string()),
								});
							}
						}

						if let Some((delta_prefix, _delta_source, delta_txid, delta_kind)) = preferred_delta
							.as_ref()
							.filter(|(prefix, _, _, _)| !missing_delta_prefixes.contains(prefix))
						{
							if !source_blobs.contains_key(delta_prefix) {
								let delta_load = tx_load_delta_blob(&tx, delta_prefix).await?;
								debug.delta_blob_loads += 1;
								debug.delta_chunk_rows_scanned += delta_load.chunk_rows_scanned;
								#[cfg(feature = "test-faults")]
								let mut blob = delta_load.blob;
								#[cfg(not(feature = "test-faults"))]
								let blob = delta_load.blob;
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
									debug.delta_blob_bytes += blob.len();
									source_blobs.insert(delta_prefix.clone(), blob);
								} else {
									debug.delta_blob_missing += 1;
									missing_delta_prefixes.insert(delta_prefix.clone());
									stale_pidx_pgnos.insert(*pgno);
									if collect_provenance {
										page_candidates.entry(*pgno).or_default().push(PageSourceCandidate {
											kind: missing_delta_kind(*delta_kind),
											txid: Some(*delta_txid),
											shard_id: None,
											result: PageSourceCandidateResult::Lost,
											reason: Some("delta_blob_missing".to_string()),
										});
									}
								}
							}

							if source_blobs.contains_key(delta_prefix) {
								if collect_provenance {
									let candidate = PageSourceCandidate {
										kind: page_ref_kind_to_source_kind(*delta_kind),
										txid: Some(*delta_txid),
										shard_id: None,
										result: PageSourceCandidateResult::Selected,
										reason: None,
									};
									page_candidates.entry(*pgno).or_default().push(candidate.clone());
									selected_candidates.insert(*pgno, candidate);
								}
								page_sources.insert(*pgno, delta_prefix.clone());
								continue;
							}

							stale_pidx_pgnos.insert(*pgno);
						}

						let shard_id = pgno / SHARD_SIZE;
						if !shard_sources.contains_key(&shard_id) {
							let shard_load = tx_load_latest_shard_blob(&tx, &scope, shard_id).await?;
							debug.hot_shard_range_scans += 1;
							debug.hot_shard_rows_scanned += shard_load.rows_scanned;
							#[cfg(feature = "test-faults")]
							let mut source = shard_load.source;
							#[cfg(not(feature = "test-faults"))]
							let source = shard_load.source;
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
							if let Some((_, blob)) = source.as_ref() {
								debug.hot_shard_hits += 1;
								debug.hot_shard_bytes += blob.len();
							} else {
								debug.hot_shard_misses += 1;
							}
							shard_sources.insert(shard_id, source);
						}

						if let Some((source_key, blob)) =
							shard_sources.get(&shard_id).cloned().flatten()
						{
							if !source_blobs.contains_key(&source_key) {
								source_blobs.insert(source_key.clone(), blob);
							}
							if collect_provenance {
								let (source_shard_id, source_as_of_txid) =
									decode_branch_shard_source_key(scope.branch_id(), &source_key)
										.unwrap_or((shard_id, 0));
								let candidate = PageSourceCandidate {
									kind: PageSourceKind::HotShard,
									txid: (source_as_of_txid != 0).then_some(source_as_of_txid),
									shard_id: Some(source_shard_id),
									result: PageSourceCandidateResult::Selected,
									reason: None,
								};
								page_candidates.entry(*pgno).or_default().push(candidate.clone());
								selected_candidates.insert(*pgno, candidate);
							}
							page_sources.insert(*pgno, source_key);
							shard_cache_read_outcomes.insert(*pgno, ShardCacheReadOutcome::FdbHit);
							touched_cache_backed_page = true;
						}
					}
					metrics::observe_get_pages_phase(
						&phase_node_id,
						"source_load",
						phase_start,
						"ok",
					);

					let branch_id = scope.branch_id();
					let access_bucket = if touched_cache_backed_page && read_mode.allows_side_effects() {
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
						head_txid: head.head_txid,
						db_size_pages: head.db_size_pages,
						loaded_pidx_rows,
						page_sources,
						source_blobs,
						page_candidates,
						selected_candidates,
						shard_cache_read_outcomes,
						stale_pidx_pgnos,
						debug,
					})
				}
			})
			.await?;

		let mut tx_result = tx_result;

		let mut stale_pidx_pgnos = tx_result.stale_pidx_pgnos;

		let mut decoded_blobs = BTreeMap::new();
		let mut pages = Vec::with_capacity(pgnos.len());
		let mut provenance = Vec::new();
		let mut returned_bytes = 0u64;
		tx_result.debug.source_blob_count = tx_result.source_blobs.len();
		tx_result.debug.source_blob_bytes =
			tx_result.source_blobs.values().map(Vec::len).sum::<usize>();

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
				if collect_provenance {
					provenance.push(PageSourceProvenance {
						pgno,
						winner_kind: PageSourceKind::OutOfRange,
						winner_txid: None,
						winner_shard_id: None,
						candidates: Vec::new(),
					});
				}
				pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let (bytes, winner_kind, winner_txid, winner_shard_id) = if let Some(source_key) =
				tx_result.page_sources.get(&pgno)
			{
				let blob = tx_result
					.source_blobs
					.get(source_key)
					.with_context(|| format!("missing source blob for page {pgno}"))?;

				if !decoded_blobs.contains_key(source_key) {
					let decoded = decode_ltx_v3(blob).with_context(|| {
						let len = blob.len();
						let head_n = len.min(64);
						let tail_start = len.saturating_sub(64);
						format!(
							"decode source blob for page {pgno}; \
							 source_key={}; len={}; head={}; tail={}",
							crate::compaction::shared::hex_lower(source_key),
							len,
							crate::compaction::shared::hex_lower(&blob[..head_n]),
							crate::compaction::shared::hex_lower(&blob[tail_start..]),
						)
					})?;
					tx_result.debug.decoded_source_blobs += 1;
					tx_result.debug.decoded_source_bytes += blob.len();
					decoded_blobs.insert(source_key.clone(), decoded);
				}

				let mut bytes = decoded_blobs
					.get(source_key)
					.and_then(|decoded: &DecodedLtx| decoded.get_page(pgno))
					.map(ToOwned::to_owned);
				let selected = tx_result.selected_candidates.get(&pgno).cloned();
				let (winner_kind, winner_txid, winner_shard_id) = if bytes.is_some() {
					let selected = selected.unwrap_or(PageSourceCandidate {
						kind: PageSourceKind::ZeroFill,
						txid: None,
						shard_id: None,
						result: PageSourceCandidateResult::Won,
						reason: Some("source_selected_without_candidate".to_string()),
					});
					(selected.kind, selected.txid, selected.shard_id)
				} else if source_key.starts_with(&keys::branch_delta_prefix(tx_result.branch_id)) {
					stale_pidx_pgnos.insert(pgno);
					tx_result.debug.stale_delta_pages += 1;
					(
						PageSourceKind::StaleDelta,
						selected.and_then(|x| x.txid),
						None,
					)
				} else {
					(
						PageSourceKind::ZeroFill,
						None,
						selected.and_then(|x| x.shard_id),
					)
				};
				(
					bytes
						.get_or_insert_with(|| vec![0; PAGE_SIZE as usize])
						.clone(),
					winner_kind,
					winner_txid,
					winner_shard_id,
				)
			} else {
				if stale_pidx_pgnos.contains(&pgno) {
					tx_result.debug.shard_coverage_missing_pages += 1;
					return Err(SqliteStorageError::ShardCoverageMissing { pgno }.into());
				}
				tx_result
					.shard_cache_read_outcomes
					.entry(pgno)
					.or_insert(ShardCacheReadOutcome::Miss);
				(
					vec![0; PAGE_SIZE as usize],
					PageSourceKind::ZeroFill,
					None,
					None,
				)
			};
			tx_result.debug.record_winner(winner_kind);
			if collect_provenance {
				let mut candidates = tx_result.page_candidates.remove(&pgno).unwrap_or_default();
				mark_provenance_winner(&mut candidates, winner_kind, winner_txid, winner_shard_id);
				provenance.push(PageSourceProvenance {
					pgno,
					winner_kind,
					winner_txid,
					winner_shard_id,
					candidates,
				});
			}
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

		if allow_side_effects {
			self.read_bytes_since_rollup
				.fetch_add(returned_bytes, std::sync::atomic::Ordering::Relaxed);
			let mut cache_snapshot = self.cache_snapshot.write().await;
			let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
			let publish_branch_changed =
				cache::branch_cache_changed(current_branch_id, tx_result.branch_id);
			#[cfg(feature = "pidx-cache")]
			let pidx = if publish_branch_changed {
				std::sync::Arc::new(DeltaPageIndex::new())
			} else {
				cache_snapshot
					.as_ref()
					.map(|snapshot| std::sync::Arc::clone(&snapshot.pidx))
					.unwrap_or_else(|| std::sync::Arc::new(DeltaPageIndex::new()))
			};
			#[cfg(not(feature = "pidx-cache"))]
			let pidx = std::sync::Arc::new(DeltaPageIndex::new());
			#[cfg(not(feature = "pidx-cache"))]
			let _ = publish_branch_changed;
			if let Some(loaded_pidx_rows) = tx_result.loaded_pidx_rows.take() {
				metrics::SQLITE_PUMP_PIDX_FDB_LOAD_TOTAL
					.with_label_values(labels)
					.inc();

				#[cfg(feature = "pidx-cache")]
				cache::store_loaded_pidx_rows(&pidx, loaded_pidx_rows, &stale_pidx_pgnos);
				#[cfg(not(feature = "pidx-cache"))]
				let _ = loaded_pidx_rows;
			}
			#[cfg(feature = "pidx-cache")]
			if !stale_pidx_pgnos.is_empty() {
				cache::clear_stale_pidx_rows(&pidx, stale_pidx_pgnos);
			}
			#[cfg(not(feature = "pidx-cache"))]
			let _ = stale_pidx_pgnos;
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
		}

		let elapsed_ms = read_started_at.elapsed().as_millis();
		if elapsed_ms >= SQLITE_GET_PAGES_DEBUG_SLOW_MS || tx_result.debug.is_expensive() {
			tracing::info!(
				database_id = %database_id_for_log,
				bucket_id = ?self.sqlite_bucket_id(),
				branch_id = ?tx_result.branch_id,
				head_txid = tx_result.head_txid,
				db_size_pages = tx_result.db_size_pages,
				elapsed_ms,
				pages_requested = tx_result.debug.pages_requested,
				pages_in_range = tx_result.debug.pages_in_range,
				pages_from_delta = tx_result.debug.pages_from_delta,
				pages_from_historical_delta = tx_result.debug.pages_from_historical_delta,
				pages_from_hot_shard = tx_result.debug.pages_from_hot_shard,
				zero_fill_pages = tx_result.debug.zero_fill_pages,
				out_of_range_pages = tx_result.debug.out_of_range_pages,
				stale_delta_pages = tx_result.debug.stale_delta_pages,
				shard_coverage_missing_pages = tx_result.debug.shard_coverage_missing_pages,
				pidx_cache_hit = tx_result.debug.pidx_cache_hit,
				pidx_cache_rows_used = tx_result.debug.pidx_cache_rows_used,
				pidx_sources_scanned = tx_result.debug.pidx_sources_scanned,
				pidx_rows_scanned = tx_result.debug.pidx_rows_scanned,
				historical_delta_chunk_rows_scanned = tx_result.debug.historical_delta_chunk_rows_scanned,
				historical_delta_txids_decoded = tx_result.debug.historical_delta_txids_decoded,
				delta_blob_loads = tx_result.debug.delta_blob_loads,
				delta_blob_missing = tx_result.debug.delta_blob_missing,
				delta_chunk_rows_scanned = tx_result.debug.delta_chunk_rows_scanned,
				delta_blob_bytes = tx_result.debug.delta_blob_bytes,
				hot_shard_range_scans = tx_result.debug.hot_shard_range_scans,
				hot_shard_rows_scanned = tx_result.debug.hot_shard_rows_scanned,
				hot_shard_hits = tx_result.debug.hot_shard_hits,
				hot_shard_misses = tx_result.debug.hot_shard_misses,
				hot_shard_bytes = tx_result.debug.hot_shard_bytes,
				source_blob_count = tx_result.debug.source_blob_count,
				source_blob_bytes = tx_result.debug.source_blob_bytes,
				decoded_source_blobs = tx_result.debug.decoded_source_blobs,
				decoded_source_bytes = tx_result.debug.decoded_source_bytes,
				"sqlite depot get_pages debug"
			);
		}

		Ok(GetPagesResult {
			pages,
			head_txid: tx_result.head_txid,
			db_size_pages: tx_result.db_size_pages,
			provenance,
		})
	}
}

struct GetPagesTxResult {
	branch_id: DatabaseBranchId,
	branch_ancestry: BranchAncestry,
	access_bucket: Option<i64>,
	head_txid: u64,
	db_size_pages: u32,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	page_candidates: BTreeMap<u32, Vec<PageSourceCandidate>>,
	selected_candidates: BTreeMap<u32, PageSourceCandidate>,
	shard_cache_read_outcomes: BTreeMap<u32, ShardCacheReadOutcome>,
	stale_pidx_pgnos: BTreeSet<u32>,
	debug: GetPagesDebug,
}

const SQLITE_GET_PAGES_DEBUG_SLOW_MS: u128 = 1_000;
const SQLITE_GET_PAGES_DEBUG_EXPENSIVE_ROWS: usize = 256;

#[derive(Default)]
struct GetPagesDebug {
	pages_requested: usize,
	pages_in_range: usize,
	pages_from_delta: usize,
	pages_from_historical_delta: usize,
	pages_from_hot_shard: usize,
	zero_fill_pages: usize,
	out_of_range_pages: usize,
	stale_delta_pages: usize,
	shard_coverage_missing_pages: usize,
	pidx_cache_hit: bool,
	pidx_cache_rows_used: usize,
	pidx_sources_scanned: usize,
	pidx_rows_scanned: usize,
	historical_delta_chunk_rows_scanned: usize,
	historical_delta_txids_decoded: usize,
	delta_blob_loads: usize,
	delta_blob_missing: usize,
	delta_chunk_rows_scanned: usize,
	delta_blob_bytes: usize,
	hot_shard_range_scans: usize,
	hot_shard_rows_scanned: usize,
	hot_shard_hits: usize,
	hot_shard_misses: usize,
	hot_shard_bytes: usize,
	source_blob_count: usize,
	source_blob_bytes: usize,
	decoded_source_blobs: usize,
	decoded_source_bytes: usize,
}

impl GetPagesDebug {
	fn is_expensive(&self) -> bool {
		self.pidx_rows_scanned >= SQLITE_GET_PAGES_DEBUG_EXPENSIVE_ROWS
			|| self.delta_chunk_rows_scanned >= SQLITE_GET_PAGES_DEBUG_EXPENSIVE_ROWS
			|| self.historical_delta_chunk_rows_scanned >= SQLITE_GET_PAGES_DEBUG_EXPENSIVE_ROWS
			|| self.hot_shard_rows_scanned >= SQLITE_GET_PAGES_DEBUG_EXPENSIVE_ROWS
	}

	fn record_winner(&mut self, kind: PageSourceKind) {
		match kind {
			PageSourceKind::PidxDelta => self.pages_from_delta += 1,
			PageSourceKind::HistoricalDelta => self.pages_from_historical_delta += 1,
			PageSourceKind::MissingDelta => self.stale_delta_pages += 1,
			PageSourceKind::StaleDelta => self.stale_delta_pages += 1,
			PageSourceKind::HotShard => self.pages_from_hot_shard += 1,
			PageSourceKind::ZeroFill => self.zero_fill_pages += 1,
			PageSourceKind::OutOfRange => self.out_of_range_pages += 1,
		}
	}
}

#[derive(Clone, Copy)]
enum ShardCacheReadOutcome {
	FdbHit,
	Miss,
}

impl ShardCacheReadOutcome {
	fn as_label(self) -> &'static str {
		match self {
			ShardCacheReadOutcome::FdbHit => metrics::SHARD_CACHE_READ_FDB_HIT,
			ShardCacheReadOutcome::Miss => metrics::SHARD_CACHE_READ_MISS,
		}
	}
}

fn page_ref_kind_to_source_kind(kind: PageRefKind) -> PageSourceKind {
	match kind {
		PageRefKind::Pidx => PageSourceKind::PidxDelta,
		PageRefKind::HistoricalDelta => PageSourceKind::HistoricalDelta,
	}
}

fn missing_delta_kind(kind: PageRefKind) -> PageSourceKind {
	match kind {
		PageRefKind::Pidx => PageSourceKind::MissingDelta,
		PageRefKind::HistoricalDelta => PageSourceKind::MissingDelta,
	}
}

fn decode_branch_shard_source_key(branch_id: DatabaseBranchId, key: &[u8]) -> Option<(u32, u64)> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key.strip_prefix(prefix.as_slice())?;
	if suffix.len() != std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
		|| suffix[std::mem::size_of::<u32>()] != b'/'
	{
		return None;
	}
	let shard_id = u32::from_be_bytes(suffix[..std::mem::size_of::<u32>()].try_into().ok()?);
	let txid = u64::from_be_bytes(suffix[std::mem::size_of::<u32>() + 1..].try_into().ok()?);
	Some((shard_id, txid))
}

fn mark_provenance_winner(
	candidates: &mut Vec<PageSourceCandidate>,
	winner_kind: PageSourceKind,
	winner_txid: Option<u64>,
	winner_shard_id: Option<u32>,
) {
	let mut found_winner = false;
	for candidate in candidates.iter_mut() {
		if candidate.kind == winner_kind
			&& candidate.txid == winner_txid
			&& candidate.shard_id == winner_shard_id
		{
			candidate.result = PageSourceCandidateResult::Won;
			candidate.reason = None;
			found_winner = true;
		} else if candidate.result == PageSourceCandidateResult::Selected {
			candidate.result = PageSourceCandidateResult::Lost;
			if candidate.reason.is_none() {
				candidate.reason = Some(
					match winner_kind {
						PageSourceKind::StaleDelta => "delta_does_not_contain_page",
						PageSourceKind::ZeroFill => "selected_source_did_not_contain_page",
						PageSourceKind::PidxDelta
						| PageSourceKind::HistoricalDelta
						| PageSourceKind::MissingDelta
						| PageSourceKind::HotShard
						| PageSourceKind::OutOfRange => "superseded",
					}
					.to_string(),
				);
			}
		}
	}
	if !found_winner {
		candidates.push(PageSourceCandidate {
			kind: winner_kind,
			txid: winner_txid,
			shard_id: winner_shard_id,
			result: PageSourceCandidateResult::Won,
			reason: None,
		});
	}
}

async fn fill_historical_delta_refs(
	tx: &universaldb::Transaction,
	database_id: &str,
	capped_sources: &[ReadSource],
	pgnos: &[u32],
	pidx_by_pgno: &BTreeMap<u32, PageRef>,
	debug: &mut GetPagesDebug,
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
			debug.historical_delta_chunk_rows_scanned += 1;
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
			debug.historical_delta_txids_decoded += 1;
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
						kind: PageRefKind::HistoricalDelta,
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
