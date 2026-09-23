//! Page read path for the stateless depot conveyer.

mod cache;
mod pidx;
mod plan;
mod shard;
mod sqlite_page;
mod tx;

use std::{
	collections::{BTreeMap, BTreeSet},
	time::{Duration, Instant},
};

#[cfg(feature = "test-faults")]
use crate::fault::{
	DepotFaultAction, DepotFaultContext, DepotFaultController, DepotFaultFired, DepotFaultPoint,
	ReadFaultPoint,
};
use anyhow::{Context, Result, ensure};
use futures_util::{StreamExt, TryStreamExt, stream};

use crate::conveyer::{
	Db,
	db::{BranchAncestry, CacheSnapshot, touch_access_if_bucket_advanced},
	delta_blob,
	error::SqliteStorageError,
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{LtxBlob, decode_ltx_v3},
	page_index::{DeltaPageIndex, PageOwner},
	types::{
		DatabaseBranchId, FetchedPage, GetPagesOptions, GetPagesReadStats, GetPagesResult,
		PageSourceCandidate, PageSourceCandidateResult, PageSourceKind, PageSourceProvenance,
		decode_commit_row,
	},
};
use crate::metrics;

use self::{
	pidx::{PageRef, PageRefKind, decode_pidx_txid},
	plan::{ReadSource, StorageScope, resolve_storage_scope},
	shard::{
		DeltaBlobLoad, ShardBlobLoad, tx_load_delta_blob, tx_load_latest_shard_blob,
		tx_load_source_shard_fold_floors,
	},
	tx::{tx_get_value, tx_scan_range_reverse_limited},
};

/// Maximum number of concurrent FDB reads issued while prefetching page sources.
const PAGE_SOURCE_FETCH_CONCURRENCY: usize = 32;

/// Maximum number of DELTA chunk rows read per FDB range call while walking a
/// source's delta history backward. This caps one range call, not the walk; the
/// walk's own bounds are resolving every missing page and the shard-version floor
/// below which a page's state already lives in a shard image.
const DELTA_HISTORY_SCAN_BATCH_KEYS: usize = 500;

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
			.map(|snapshot| cache::snapshot_pidx_cache(&snapshot.pidx, &pgnos));
		#[cfg(not(feature = "pidx-cache"))]
		let cached_pidx = None::<BTreeMap<u32, PageOwner>>;
		let cached_branch_id = cached_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		#[cfg(feature = "pidx-cache")]
		let cached_head_txid = cached_snapshot
			.as_ref()
			.map(|snapshot| snapshot.cache_head_txid);
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
		let ltx_blob_cache = self.ltx_blob_cache.clone();
		let delta_layout_cache = self.delta_segment_layout_cache.clone();
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();
		// Read backpressure. A large staged commit makes its own pages take the slow read path, so
		// read volume peaks alongside write volume.
		self.await_actor_throttle(universaldb::ThrottleKind::Read)
			.await;
		let tx_result = self
			.udb
			.txn("depot_get_pages", move |tx| {
				let phase_node_id = phase_node_id.clone();
				let ltx_blob_cache = ltx_blob_cache.clone();
				let delta_layout_cache = delta_layout_cache.clone();
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
					// Opt this transaction's reads into the actor throttle's accounting. Charging is
					// what makes the next caller's check meaningful; the check itself already
					// happened before the transaction opened.
					tx.charge_throttle(
						rivet_config::config::DEPOT_ACTOR_THROTTLE,
						universaldb::ThrottleCharge::Read,
					)?;
					let mut debug = GetPagesDebug::default();
					debug.pages_requested = pgnos.len();
					debug.requested_pgno_min = pgnos.iter().copied().min().unwrap_or(0);
					debug.requested_pgno_max = pgnos.iter().copied().max().unwrap_or(0);
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
					let cache_branch_matches = cached_branch_id == Some(scope.branch_id());
					// A page's content is a pure function of (branch_id, head_txid): content
					// changes only via a commit, which advances the head. Trust the cached PIDX
					// only when the head it was built at still matches the head resolved in this
					// transaction, so a foreign writer advancing the head forces a fresh scan.
					#[cfg(feature = "pidx-cache")]
					let cache_head_matches = {
						let StorageScope::Branch(plan) = &scope;
						cached_head_txid == Some(plan.head.head_txid)
					};
					#[cfg(not(feature = "pidx-cache"))]
					let cache_head_matches = true;
					let cached_pidx = if cache_branch_matches && cache_head_matches {
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
					{
						let StorageScope::Branch(plan_for_debug) = &scope;
						debug.sources_len = plan_for_debug.sources.len();
						debug.head_source_max_txid = plan_for_debug
							.sources
							.first()
							.map(|source| source.max_txid())
							.unwrap_or(0);
					}
					let cache_source = cache::cache_source_for_scope(&scope);
					let StorageScope::Branch(plan) = &scope;

					// Resolve owners already known to the lazy per-page cache, and collect
					// the pages that still need a point read. The cache is only available
					// for a single-source (unforked) read; a forked read has no cache and
					// point-reads every requested page.
					let mut pages_to_read: Vec<u32> = Vec::new();
					match (cache_source, cached_pidx.as_ref()) {
						(Some(cache_source), Some(cached_known)) => {
							debug.pidx_cache_hit = !cached_known.is_empty();
							debug.pidx_cache_rows_used = cached_known.len();
							for pgno in &pgnos_in_range {
								match cached_known.get(pgno) {
									Some(PageOwner::Owner(txid))
										if *txid <= cache_source.max_txid() =>
									{
										pidx_by_pgno.insert(
											*pgno,
											PageRef {
												source: cache_source,
												txid: *txid,
												kind: PageRefKind::Pidx,
											},
										);
									}
									// A cached owner above the cap (diagnostic reads) and a cached
									// proven-absent owner both mean no usable PIDX owner; fall
									// through without a point read.
									Some(PageOwner::Owner(_)) | Some(PageOwner::NoOwner) => {}
									None => pages_to_read.push(*pgno),
								}
							}
						}
						_ => {
							debug.pidx_cache_hit = false;
							pages_to_read = pgnos_in_range.clone();
						}
					}

					// Point-read PIDX owners for exactly the unresolved pages across
					// sources in priority order, instead of scanning every source's whole
					// PIDX prefix.
					let point_read_owners = point_read_pidx_owners(
						&tx,
						&database_id,
						&plan.sources,
						&pages_to_read,
						&mut debug,
					)
					.await?;
					for (pgno, page_ref) in &point_read_owners {
						pidx_by_pgno.entry(*pgno).or_insert(*page_ref);
					}

					// Teach the single-source cache the owners found and the pages proven
					// to have no PIDX owner, so a later read of the same pages is a cache
					// hit. A single-source point read only hits that source, so a missing
					// owner means the page has no PIDX owner.
					if cache_source.is_some() {
						loaded_pidx_rows = Some(
							pages_to_read
								.iter()
								.map(|pgno| {
									(
										*pgno,
										point_read_owners.get(pgno).map(|page_ref| page_ref.txid),
									)
								})
								.collect::<Vec<_>>(),
						);
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
					// Commits whose scan cost has already been charged to the debug counters.
					let mut counted_delta_txids = BTreeSet::new();
					let mut shard_sources =
						BTreeMap::<u32, Option<(DatabaseBranchId, Vec<u8>, Vec<u8>)>>::new();
					let mut stale_pidx_pgnos = BTreeSet::new();
					let mut shard_cache_read_outcomes =
						BTreeMap::<u32, ShardCacheReadOutcome>::new();
					let mut touched_cache_backed_page = false;
					let mut touched_shards = BTreeSet::<u32>::new();

					let phase_start = Instant::now();

					// Phase 1: prefetch every unique delta blob, shard blob, and cold ref
					// concurrently. The assembly loop below dedups loads per delta prefix and
					// per shard, so we resolve the unique keys here, fetch each exactly once,
					// and let the loop read from these maps instead of awaiting inline.
					let tx_ref = &tx;
					let scope_ref = &scope;
					let ltx_blob_cache_ref = &ltx_blob_cache;
					let delta_layout_cache_ref = &delta_layout_cache;
					#[cfg(feature = "test-faults")]
					let database_id_ref = &database_id;
					#[cfg(feature = "test-faults")]
					let fault_controller_ref = &fault_controller;

					// Unique delta prefixes referenced by PIDX, paired with the first page that
					// references them so fault injection stays attributed to the same page as
					// the sequential path.
					// One load per commit, carrying every page that resolves through it. A commit can
					// store its pages across several blobs, so the load needs the whole page set to
					// know which of them to materialize; loading them all would copy blobs no page
					// in this read asks for.
					let mut delta_prefix_triggers: Vec<(Vec<u8>, ReadSource, u64, Vec<u32>)> =
						Vec::new();
					{
						let mut by_prefix = BTreeMap::<Vec<u8>, usize>::new();
						for pgno in &pgnos_in_range {
							if let Some(page_ref) = pidx_by_pgno.get(pgno) {
								let prefix = page_ref
									.source
									.delta_chunk_prefix(&database_id, page_ref.txid);
								match by_prefix.get(&prefix) {
									Some(idx) => delta_prefix_triggers[*idx].3.push(*pgno),
									None => {
										by_prefix.insert(prefix.clone(), delta_prefix_triggers.len());
										delta_prefix_triggers.push((
											prefix,
											page_ref.source,
											page_ref.txid,
											vec![*pgno],
										));
									}
								}
							}
						}
					}

					let delta_blobs: BTreeMap<Vec<u8>, DeltaBlobLoad> =
						stream::iter(delta_prefix_triggers)
							.map(move |(prefix, source, txid, trigger_pgnos)| async move {
								let trigger_pgno = trigger_pgnos[0];
								let load = tx_load_delta_blob(
									tx_ref,
									source,
									txid,
									&trigger_pgnos,
									ltx_blob_cache_ref,
									delta_layout_cache_ref,
								)
								.await?;
								#[cfg(feature = "test-faults")]
								let load = {
									let mut load = load;
									if matches!(
										maybe_fire_read_fault(
											fault_controller_ref,
											if load.segments.is_empty() {
												ReadFaultPoint::DeltaBlobMissing
											} else {
												ReadFaultPoint::AfterDeltaBlobLoad
											},
											database_id_ref,
											Some(scope_ref.branch_id()),
											Some(trigger_pgno),
											Some(trigger_pgno / SHARD_SIZE),
										)
										.await?,
										Some(DepotFaultFired {
											action: DepotFaultAction::DropArtifact,
											..
										})
									) {
										// Dropping every blob of the commit is what "the delta
										// artifact is gone" means now that a commit can hold
										// several: any survivor would still resolve pages.
										load.segments.clear();
									}
									load
								};
								#[cfg(not(feature = "test-faults"))]
								let _ = trigger_pgno;
								Result::<(Vec<u8>, DeltaBlobLoad)>::Ok((prefix, load))
							})
							.buffer_unordered(PAGE_SOURCE_FETCH_CONCURRENCY)
							.try_collect()
							.await?;

					// Shards are only consulted for pages whose preferred delta is absent or
					// whose delta blob is missing. Collect the unique shards for those pages,
					// keeping the first referencing page for fault attribution.
					let mut shard_triggers: Vec<(u32, u32)> = Vec::new();
					{
						let mut seen = BTreeSet::new();
						for pgno in &pgnos_in_range {
							let has_present_delta =
								pidx_by_pgno.get(pgno).is_some_and(|page_ref| {
									let prefix = page_ref
										.source
										.delta_chunk_prefix(&database_id, page_ref.txid);
									delta_blobs
										.get(&prefix)
										.and_then(|load| {
											delta_blob::segment_for_page(&load.segments, *pgno)
										})
										.is_some()
								});
							if has_present_delta {
								continue;
							}
							let shard_id = pgno / SHARD_SIZE;
							if seen.insert(shard_id) {
								shard_triggers.push((shard_id, *pgno));
							}
						}
					}

					let shard_blobs: BTreeMap<u32, ShardBlobLoad> = stream::iter(shard_triggers)
						.map(move |(shard_id, trigger_pgno)| async move {
							let load =
								tx_load_latest_shard_blob(tx_ref, scope_ref, shard_id).await?;
							#[cfg(feature = "test-faults")]
							let load = {
								let mut load = load;
								if matches!(
									maybe_fire_read_fault(
										fault_controller_ref,
										ReadFaultPoint::AfterShardBlobLoad,
										database_id_ref,
										Some(scope_ref.branch_id()),
										Some(trigger_pgno),
										Some(shard_id),
									)
									.await?,
									Some(DepotFaultFired {
										action: DepotFaultAction::DropArtifact,
										..
									})
								) {
									load.source = None;
								}
								load
							};
							#[cfg(not(feature = "test-faults"))]
							let _ = trigger_pgno;
							Result::<(u32, ShardBlobLoad)>::Ok((shard_id, load))
						})
						.buffer_unordered(PAGE_SOURCE_FETCH_CONCURRENCY)
						.try_collect()
						.await?;

					for pgno in &pgnos_in_range {
						// A page resolves through the key of the blob that can hold it, not through its
						// commit's prefix: a segmented commit stores its pages across several blobs, so
						// two pages of one commit can resolve to different ones. When the commit has no
						// blob at all the prefix stands in, so the missing-delta bookkeeping below still
						// has a stable identity to record.
						let preferred_delta = pidx_by_pgno.get(pgno).copied().map(|page_ref| {
							let txid_prefix = page_ref
								.source
								.delta_chunk_prefix(&database_id, page_ref.txid);
							let blob_key = delta_blobs
								.get(&txid_prefix)
								.and_then(|load| {
									delta_blob::segment_for_page(&load.segments, *pgno)
								})
								.map(|segment| segment.key.clone())
								.unwrap_or(txid_prefix);
							(blob_key, page_ref.source, page_ref.txid, page_ref.kind)
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

						if let Some((delta_prefix, delta_source, delta_txid, delta_kind)) = preferred_delta
							.as_ref()
							.filter(|(prefix, _, _, _)| !missing_delta_prefixes.contains(prefix))
						{
							if !source_blobs.contains_key(delta_prefix) {
								let txid_prefix = delta_source
									.delta_chunk_prefix(&database_id, *delta_txid);
								let delta_load = delta_blobs
									.get(&txid_prefix)
									.expect("delta blob prefetched for every PIDX prefix");
								debug.delta_blob_loads += 1;
								// One scan covers the whole commit, so its rows are counted once even
								// when several of its blobs are consulted.
								if counted_delta_txids.insert(txid_prefix) {
									debug.delta_chunk_rows_scanned += delta_load.chunk_rows_scanned;
								}
								if let Some(segment) =
									delta_blob::segment_for_page(&delta_load.segments, *pgno)
								{
									debug.delta_blob_bytes += segment.blob.len();
									source_blobs.insert(delta_prefix.clone(), segment.blob.clone());
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
							let shard_load = shard_blobs
								.get(&shard_id)
								.expect("shard blob prefetched for every consulted shard");
							debug.hot_shard_range_scans += 1;
							debug.hot_shard_rows_scanned += shard_load.rows_scanned;
							let source = shard_load.source.clone();
							if let Some((_, _, blob)) = source.as_ref() {
								debug.hot_shard_hits += 1;
								debug.hot_shard_bytes += blob.len();
							} else {
								debug.hot_shard_misses += 1;
							}
							shard_sources.insert(shard_id, source);
						}

						if let Some((source_branch_id, source_key, blob)) =
							shard_sources.get(&shard_id).cloned().flatten()
						{
							if !source_blobs.contains_key(&source_key) {
								source_blobs.insert(source_key.clone(), blob);
							}
							if collect_provenance {
								let (source_shard_id, source_as_of_txid) =
									decode_branch_shard_source_key(source_branch_id, &source_key)
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
							touched_shards.insert(shard_id);
						}
					}
					metrics::observe_get_pages_phase(
						&phase_node_id,
						"source_load",
						phase_start,
						"ok",
					);

					debug.pidx_by_pgno_len = pidx_by_pgno.len();
					debug.page_sources_len = page_sources.len();
					let branch_id = scope.branch_id();
					let access_bucket = if touched_cache_backed_page && read_mode.allows_side_effects() {
						touch_access_if_bucket_advanced(
							&tx,
							branch_id,
							cached_access_bucket,
							&touched_shards,
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
		let stale_pidx_pgnos = tx_result.stale_pidx_pgnos;

		let mut decoded_blobs = BTreeMap::<Vec<u8>, std::sync::Arc<LtxBlob>>::new();
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
				if !decoded_blobs.contains_key(source_key) {
					// Reuse the immutable blob across reads. The cache holds the parsed
					// page index plus raw bytes; on a miss we parse the index once (no
					// page decompression) and cache it.
					let decoded = if let Some(cached) = self.ltx_blob_cache.get(source_key).await {
						cached
					} else {
						let blob = tx_result
							.source_blobs
							.get(source_key)
							.with_context(|| format!("missing source blob for page {pgno}"))?;
						let decoded = std::sync::Arc::new(
							LtxBlob::decode_index(blob.clone()).with_context(|| {
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
							})?,
						);
						tx_result.debug.decoded_source_blobs += 1;
						tx_result.debug.decoded_source_bytes += decoded.bytes().len();
						self.ltx_blob_cache
							.insert(source_key.clone(), decoded.clone())
							.await;
						decoded
					};
					decoded_blobs.insert(source_key.clone(), decoded);
				}

				let mut bytes = decoded_blobs
					.get(source_key)
					.map(|decoded| decoded.get_page(pgno))
					.transpose()?
					.flatten();
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
					// A PIDX row names this delta as the page's owner, so the delta must carry the
					// page. Zero-filling here would hand SQLite a blank page in place of committed
					// data, which is exactly how a compaction defect becomes a malformed database.
					// Fail the read instead so the defect is visible where it happened.
					let txid =
						keys::decode_branch_delta_chunk_txid(tx_result.branch_id, source_key)
							.ok()
							.or_else(|| selected.and_then(|x| x.txid))
							.unwrap_or_default();
					tracing::error!(
						database_id = %database_id_for_log,
						pgno,
						txid,
						"pidx-owned delta does not carry its page"
					);
					return Err(SqliteStorageError::DeltaPageMissing { pgno, txid }.into());
				} else {
					// The read resolved to a shard image, and every shard image is required to be a
					// complete image of its shard, so this is a compaction defect. The page is still
					// served as zeros rather than failing the whole batch, because the open-time
					// preload fetches many pages at once and a hard error here would make an already
					// damaged database unopenable rather than merely wrong. The metric and log are
					// what make the state visible.
					let shard_id = selected.and_then(|x| x.shard_id);
					tracing::warn!(
						database_id = %database_id_for_log,
						pgno,
						shard_id,
						"shard image does not carry a page it should cover; serving zeros"
					);
					metrics::SQLITE_READ_SHARD_PAGE_MISSING_TOTAL
						.with_label_values(&[node_id.as_str()])
						.inc();
					(PageSourceKind::ZeroFill, None, shard_id)
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

		// Return overflow pages referenced by the requested leaf pages up front.
		// This runs before taking the cache-snapshot write lock because it issues
		// nested get_pages calls that take the lock themselves. Expansion is a
		// best-effort prefetch: an error here must not fail the base read, whose
		// requested pages are already materialized.
		if options.expand_overflow {
			if let Err(err) = self
				.expand_overflow_pages(&mut pages, tx_result.db_size_pages)
				.await
			{
				tracing::warn!(
					database_id = %self.database_id,
					?err,
					"sqlite overflow prefetch expansion failed; returning base pages",
				);
			}
		}

		if allow_side_effects {
			self.read_bytes_since_rollup
				.fetch_add(returned_bytes, std::sync::atomic::Ordering::Relaxed);

			// Return overflow pages referenced by the requested leaf pages up front.
			// This runs before taking the cache-snapshot write lock because it issues
			// nested get_pages calls that take the lock themselves. Expansion is a
			// best-effort prefetch: an error here must not fail the base read, whose
			// requested pages are already materialized.
			if options.expand_overflow {
				if let Err(err) = self
					.expand_overflow_pages(&mut pages, tx_result.db_size_pages)
					.await
				{
					tracing::warn!(
						database_id = %self.database_id,
						?err,
						"sqlite overflow prefetch expansion failed; returning base pages",
					);
				}
			}

			let mut cache_snapshot = self.cache_snapshot.write().await;
			let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
			let publish_branch_changed =
				cache::branch_cache_changed(current_branch_id, tx_result.branch_id);
			// The lazy per-page cache is only valid at a single head. If the head
			// advanced since the cached snapshot, its entries may now be stale, so
			// reset to a fresh index and repopulate from this read instead of reusing
			// the old Arc (which would serve stale ownership at the new head).
			#[cfg(feature = "pidx-cache")]
			let cache_head_changed = cache_snapshot
				.as_ref()
				.is_some_and(|snapshot| snapshot.cache_head_txid != tx_result.head_txid);
			#[cfg(feature = "pidx-cache")]
			let pidx = if publish_branch_changed || cache_head_changed {
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
				metrics::SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL
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
				cache_head_txid: tx_result.head_txid,
			});
		}

		let elapsed_ms = read_started_at.elapsed().as_millis();
		if elapsed_ms >= SQLITE_GET_PAGES_DEBUG_SLOW_MS || tx_result.debug.is_expensive() {
			tracing::debug!(
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
				pages_from_cold = tx_result.debug.pages_from_cold,
				zero_fill_pages = tx_result.debug.zero_fill_pages,
				out_of_range_pages = tx_result.debug.out_of_range_pages,
				stale_delta_pages = tx_result.debug.stale_delta_pages,
				shard_coverage_missing_pages = tx_result.debug.shard_coverage_missing_pages,
				pidx_cache_hit = tx_result.debug.pidx_cache_hit,
				pidx_cache_rows_used = tx_result.debug.pidx_cache_rows_used,
				pidx_sources_scanned = tx_result.debug.pidx_sources_scanned,
				pidx_rows_scanned = tx_result.debug.pidx_rows_scanned,
				sources_len = tx_result.debug.sources_len,
				head_source_max_txid = tx_result.debug.head_source_max_txid,
				requested_pgno_min = tx_result.debug.requested_pgno_min,
				requested_pgno_max = tx_result.debug.requested_pgno_max,
				scanned_txid_min = tx_result.debug.scanned_txid_min,
				scanned_txid_max = tx_result.debug.scanned_txid_max,
				pidx_rows_filtered_above_max_txid = tx_result.debug.pidx_rows_filtered_above_max_txid,
				pidx_by_pgno_len = tx_result.debug.pidx_by_pgno_len,
				page_sources_len = tx_result.debug.page_sources_len,
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
				cold_page_loads = tx_result.debug.cold_page_loads,
				cold_page_bytes = tx_result.debug.cold_page_bytes,
				source_blob_count = tx_result.debug.source_blob_count,
				source_blob_bytes = tx_result.debug.source_blob_bytes,
				decoded_source_blobs = tx_result.debug.decoded_source_blobs,
				decoded_source_bytes = tx_result.debug.decoded_source_bytes,
				"sqlite depot get_pages debug"
			);
		}

		let returned_bytes = pages
			.iter()
			.filter_map(|page| page.bytes.as_ref().map(Vec::len))
			.fold(0_usize, usize::saturating_add);
		metrics::SQLITE_PUMP_GET_PAGES_RETURNED_BYTES
			.with_label_values(labels)
			.observe(returned_bytes as f64);

		// Page one's header carries its own copy of the database size, and the commit row for the
		// txid this read resolved to carries the authoritative one. They are written together, so a
		// disagreement means the read served a page one from a different point in history than the
		// size it was paired with. That is what makes the failure destructive rather than merely
		// wrong: the VFS latches the header's size at open and never revisits it, so a short page
		// one silently truncates every page above it on the next commit.
		if let Some(page_db_size_pages) = pages
			.iter()
			.find(|page| page.pgno == 1)
			.and_then(|page| page.bytes.as_deref())
			.and_then(sqlite_page::header_db_size_pages)
		{
			if page_db_size_pages != tx_result.db_size_pages {
				tracing::error!(
					database_id = %database_id_for_log,
					txid = tx_result.head_txid,
					page_db_size_pages,
					head_db_size_pages = tx_result.db_size_pages,
					"sqlite page one disagrees with the database size at its txid"
				);
				metrics::SQLITE_READ_STALE_MAIN_PAGE_TOTAL
					.with_label_values(&[node_id.as_str()])
					.inc();
				return Err(SqliteStorageError::StaleMainPage {
					txid: tx_result.head_txid,
					page_db_size_pages,
					head_db_size_pages: tx_result.db_size_pages,
				}
				.into());
			}
		}

		crate::startup::remember(
			self.sqlite_bucket_id(),
			&self.database_id,
			tx_result.branch_id,
			tx_result.head_txid,
			tx_result.db_size_pages,
			None,
			pages
				.iter()
				.filter_map(|p| p.bytes.as_ref().map(|b| (p.pgno, b.clone()))),
		)
		.await;
		Ok(GetPagesResult {
			pages,
			head_txid: tx_result.head_txid,
			db_size_pages: tx_result.db_size_pages,
			provenance,
			read_stats: tx_result.debug.read_stats(),
		})
	}

	/// Append the overflow pages referenced by the already-fetched leaf pages to
	/// `pages`, walking each overflow chain to its end. Overflow pages are loaded
	/// through nested get_pages calls (without further expansion), so the actor
	/// reads them from its local cache instead of round tripping per row.
	async fn expand_overflow_pages(
		&self,
		pages: &mut Vec<FetchedPage>,
		db_size_pages: u32,
	) -> Result<()> {
		let page_size = PAGE_SIZE as usize;
		// The reserved-byte count lives in the database header (page 1). It is
		// zero for Rivet databases; fall back to zero when page 1 is absent.
		let reserved = pages
			.iter()
			.find(|page| page.pgno == 1)
			.and_then(|page| page.bytes.as_deref())
			.and_then(sqlite_page::header_reserved_bytes)
			.unwrap_or(0);

		let mut seen: BTreeSet<u32> = pages.iter().map(|page| page.pgno).collect();
		let mut frontier: Vec<u32> = Vec::new();
		for page in pages.iter() {
			let Some(bytes) = page.bytes.as_deref() else {
				continue;
			};
			for head in sqlite_page::overflow_head_pages(
				page.pgno,
				bytes,
				page_size,
				reserved,
				db_size_pages,
			) {
				if seen.insert(head) {
					frontier.push(head);
				}
			}
		}

		// Bound the walk in wall-clock time. Each level is its own UDB read
		// transaction, so a deep overflow chain (a multi-MB row discovered one
		// page deeper per level) would otherwise run many sequential transactions.
		// Stopping early just leaves the remaining overflow pages to be fetched on
		// demand, the pre-existing behavior.
		let deadline = Instant::now() + OVERFLOW_EXPANSION_TIME_BUDGET;
		let mut budget = MAX_OVERFLOW_EXPANSION_PAGES;
		while !frontier.is_empty() && budget > 0 {
			if Instant::now() >= deadline {
				break;
			}
			if frontier.len() > budget {
				frontier.truncate(budget);
			}
			budget -= frontier.len();

			// Box the recursive call to keep the async future a finite size.
			let fetched = Box::pin(self.get_pages_with_options(
				frontier.clone(),
				GetPagesOptions {
					expected_head_txid: None,
					expand_overflow: false,
					..Default::default()
				},
			))
			.await?
			.pages;

			let mut next_frontier = Vec::new();
			for page in fetched {
				if let Some(bytes) = page.bytes.as_deref() {
					if let Some(next) = sqlite_page::overflow_next_page(bytes, db_size_pages) {
						if seen.insert(next) {
							next_frontier.push(next);
						}
					}
				}
				pages.push(page);
			}
			frontier = next_frontier;
		}

		Ok(())
	}
}

/// Cap on the number of overflow pages a single get_pages call may eagerly
/// return, bounding response size when leaf pages reference long overflow chains.
const MAX_OVERFLOW_EXPANSION_PAGES: usize = 2048;

/// Wall-clock ceiling on the overflow-chain walk. Overflow prefetch is
/// best-effort, so once this elapses the remaining chain is left for on-demand
/// fetching rather than extending the read latency further.
const OVERFLOW_EXPANSION_TIME_BUDGET: Duration = Duration::from_millis(2500);

struct GetPagesTxResult {
	branch_id: DatabaseBranchId,
	branch_ancestry: BranchAncestry,
	access_bucket: Option<i64>,
	head_txid: u64,
	db_size_pages: u32,
	loaded_pidx_rows: Option<Vec<(u32, Option<u64>)>>,
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
	pages_from_cold: usize,
	zero_fill_pages: usize,
	out_of_range_pages: usize,
	stale_delta_pages: usize,
	shard_coverage_missing_pages: usize,
	pidx_cache_hit: bool,
	pidx_cache_rows_used: usize,
	pidx_sources_scanned: usize,
	pidx_rows_scanned: usize,
	sources_len: usize,
	head_source_max_txid: u64,
	requested_pgno_min: u32,
	requested_pgno_max: u32,
	scanned_txid_min: u64,
	scanned_txid_max: u64,
	pidx_rows_filtered_above_max_txid: usize,
	pidx_by_pgno_len: usize,
	page_sources_len: usize,
	historical_delta_chunk_rows_scanned: usize,
	historical_delta_txids_decoded: usize,
	historical_delta_scan_floor_txid: u64,
	historical_delta_pages_shard_superseded: usize,
	delta_blob_loads: usize,
	delta_blob_missing: usize,
	delta_chunk_rows_scanned: usize,
	delta_blob_bytes: usize,
	hot_shard_range_scans: usize,
	hot_shard_rows_scanned: usize,
	hot_shard_hits: usize,
	hot_shard_misses: usize,
	hot_shard_bytes: usize,
	cold_page_loads: usize,
	cold_page_bytes: usize,
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

	fn read_stats(&self) -> GetPagesReadStats {
		GetPagesReadStats {
			historical_delta_chunk_rows_scanned: self.historical_delta_chunk_rows_scanned,
			historical_delta_txids_decoded: self.historical_delta_txids_decoded,
			historical_delta_scan_floor_txid: self.historical_delta_scan_floor_txid,
			historical_delta_pages_shard_superseded: self.historical_delta_pages_shard_superseded,
		}
	}

	fn record_winner(&mut self, kind: PageSourceKind) {
		match kind {
			PageSourceKind::PidxDelta => self.pages_from_delta += 1,
			PageSourceKind::HistoricalDelta => self.pages_from_historical_delta += 1,
			PageSourceKind::MissingDelta => self.stale_delta_pages += 1,
			PageSourceKind::HotShard => self.pages_from_hot_shard += 1,
			PageSourceKind::Cold => self.pages_from_cold += 1,
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
						PageSourceKind::ZeroFill => "selected_source_did_not_contain_page",
						PageSourceKind::PidxDelta
						| PageSourceKind::HistoricalDelta
						| PageSourceKind::MissingDelta
						| PageSourceKind::HotShard
						| PageSourceKind::Cold
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

/// Point-read the PIDX owner of each requested page across sources in priority
/// order, reading only the requested pages instead of every source's whole PIDX
/// prefix. The first source that owns a page (with an owner txid at or below its
/// cap) wins, matching the prior full-scan `or_insert` priority. Pages still
/// unowned after every source are left for the DELTA-history / SHARD fallbacks.
async fn point_read_pidx_owners(
	tx: &universaldb::Transaction,
	database_id: &str,
	sources: &[ReadSource],
	pages: &[u32],
	debug: &mut GetPagesDebug,
) -> Result<BTreeMap<u32, PageRef>> {
	let mut owners = BTreeMap::new();
	let mut remaining = pages.to_vec();

	for source in sources.iter().copied() {
		if remaining.is_empty() {
			break;
		}
		debug.pidx_sources_scanned += 1;

		// Point-read this source's PIDX for each still-unowned page concurrently;
		// these are independent reads on the same transaction.
		let reads: Vec<(u32, Option<u64>)> = stream::iter(remaining.iter().copied())
			.map(|pgno| {
				let key = source.pidx_key(database_id, pgno);
				async move {
					let txid = match tx_get_value(tx, &key).await? {
						Some(value) => Some(decode_pidx_txid(&value)?),
						None => None,
					};
					Result::<(u32, Option<u64>)>::Ok((pgno, txid))
				}
			})
			.buffer_unordered(PAGE_SOURCE_FETCH_CONCURRENCY)
			.try_collect()
			.await?;

		let mut next_remaining = Vec::new();
		for (pgno, txid) in reads {
			match txid {
				Some(txid) => {
					debug.pidx_rows_scanned += 1;
					if debug.scanned_txid_min == 0 || txid < debug.scanned_txid_min {
						debug.scanned_txid_min = txid;
					}
					if txid > debug.scanned_txid_max {
						debug.scanned_txid_max = txid;
					}
					if txid <= source.max_txid() {
						owners.entry(pgno).or_insert(PageRef {
							source,
							txid,
							kind: PageRefKind::Pidx,
						});
					} else {
						// An ancestor PIDX owner newer than the fork cap is ignored;
						// the page resolves through that source's capped DELTA history
						// or an older source.
						debug.pidx_rows_filtered_above_max_txid += 1;
						next_remaining.push(pgno);
					}
				}
				None => next_remaining.push(pgno),
			}
		}
		remaining = next_remaining;
	}

	Ok(owners)
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

	// Walk each capped source's DELTA history newest-first, stopping as soon as
	// every missing page is resolved. Sources are processed sequentially to
	// preserve source priority; a source is not read at all once nothing is
	// missing.
	for source in capped_sources.iter().copied() {
		if missing_pgnos.is_empty() {
			break;
		}

		// Bound this source's walk by its own folded shard versions. A page whose last
		// write at or below the cap was already folded appears in no retained delta, so
		// without this bound the walk runs the source's whole retained history to
		// exhaustion on every read before the SHARD fallback gets a turn.
		let shard_ids = missing_pgnos
			.iter()
			.map(|pgno| pgno / SHARD_SIZE)
			.collect::<BTreeSet<_>>();
		let shard_fold_floors = tx_load_source_shard_fold_floors(tx, source, &shard_ids).await?;

		scan_source_delta_history(
			tx,
			database_id,
			source,
			&missing_pgnos,
			&shard_fold_floors,
			&mut refs,
			debug,
		)
		.await?;

		// Pages this source resolved are done; the rest fall through to the next
		// source, which walks its own history under its own floors.
		missing_pgnos.retain(|pgno| !refs.contains_key(pgno));
	}

	Ok(refs)
}

/// Txid below which `pgno` is already covered by one of the source's folded shard versions, if it
/// has one.
fn page_shard_fold_floor(shard_fold_floors: &BTreeMap<u32, u64>, pgno: u32) -> Option<u64> {
	shard_fold_floors.get(&(pgno / SHARD_SIZE)).copied()
}

/// Lowest txid the DELTA-history walk still has to read for `pages`. Each page is superseded below
/// its shard's floor, so the walk can stop at the lowest floor among them; a page whose shard has no
/// floor keeps the walk unbounded below.
fn delta_history_scan_floor(pages: &BTreeSet<u32>, shard_fold_floors: &BTreeMap<u32, u64>) -> u64 {
	pages
		.iter()
		.map(|pgno| page_shard_fold_floor(shard_fold_floors, *pgno).unwrap_or(0))
		.min()
		.unwrap_or(0)
}

/// Drops the pages whose shard floor sits at or above `txid`, so the walk stops searching for them as
/// it descends past their fold. Returns the number dropped.
fn drop_shard_superseded_pages(
	pages: &mut BTreeSet<u32>,
	shard_fold_floors: &BTreeMap<u32, u64>,
	txid: u64,
) -> usize {
	let superseded = pages
		.iter()
		.copied()
		.filter(|pgno| {
			page_shard_fold_floor(shard_fold_floors, *pgno).is_some_and(|floor| floor >= txid)
		})
		.collect::<Vec<_>>();
	for pgno in &superseded {
		pages.remove(pgno);
	}

	superseded.len()
}

/// Walk a single source's DELTA history from `source.max_txid()` downward in
/// bounded reverse range reads, decoding each fully-gathered txid and resolving
/// any missing pages it covers. Returns once every missing page is resolved, every
/// remaining page is covered by one of this source's shard versions, or the history
/// above those versions is exhausted, so a recent page never reads more than the
/// first batch and a folded page reads no history at all.
async fn scan_source_delta_history(
	tx: &universaldb::Transaction,
	database_id: &str,
	source: ReadSource,
	missing_pgnos: &BTreeSet<u32>,
	shard_fold_floors: &BTreeMap<u32, u64>,
	refs: &mut BTreeMap<u32, PageRef>,
	debug: &mut GetPagesDebug,
) -> Result<()> {
	// Pages this source's history can still resolve. Shrinks as deltas resolve pages and
	// as the walk descends past a page's shard version, and is local to this source so a
	// page a shard version covers here still reaches the next source.
	let mut searching = missing_pgnos.clone();
	debug.historical_delta_pages_shard_superseded +=
		drop_shard_superseded_pages(&mut searching, shard_fold_floors, source.max_txid());

	// Start the reverse walk just above the source cap so chunks newer than
	// max_txid are never read, even on an ancestor source whose parent received
	// many commits after this source's fork point. The cap txid holds multiple
	// chunk rows, so the exclusive bound must sit past its largest possible chunk
	// suffix to keep every chunk of the cap txid in range; `delta_txid_scan_end`
	// owns that, so the bound does not have to assume how wide a suffix is.
	let mut batch_end = source.delta_txid_scan_end(database_id, source.max_txid());
	// Chunks for the txid currently being gathered. Reverse key order yields a
	// txid's chunks contiguously (high chunk index to low), so the gathered set is
	// complete the moment a lower txid appears or the history ends.
	let mut pending: Option<(u64, Vec<(Vec<u8>, Vec<u8>)>)> = None;

	loop {
		if searching.is_empty() {
			break;
		}

		// Read no deeper than the lowest shard version still covering a searched page.
		// Recomputed per batch because resolving a page can raise the floor.
		let scan_floor = delta_history_scan_floor(&searching, shard_fold_floors);
		if scan_floor > debug.historical_delta_scan_floor_txid {
			debug.historical_delta_scan_floor_txid = scan_floor;
		}
		let begin = if scan_floor == 0 {
			source.delta_prefix(database_id)
		} else {
			source.delta_chunk_prefix(database_id, scan_floor.saturating_add(1))
		};
		if begin.as_slice() >= batch_end.as_slice() {
			break;
		}

		let batch =
			tx_scan_range_reverse_limited(tx, &begin, &batch_end, DELTA_HISTORY_SCAN_BATCH_KEYS)
				.await?;
		if batch.is_empty() {
			break;
		}
		let batch_len = batch.len();
		let mut last_key = None;

		for (key, chunk) in batch {
			debug.historical_delta_chunk_rows_scanned += 1;
			let txid = source.decode_delta_chunk_txid(database_id, &key)?;

			let same_txid = matches!(&pending, Some((pending_txid, _)) if *pending_txid == txid);
			if !same_txid {
				if let Some((pending_txid, pending_chunks)) = pending.take() {
					process_delta_txid(
						source,
						pending_txid,
						pending_chunks,
						&mut searching,
						refs,
						debug,
					)?;
				}
				// Descending past a page's shard version means no lower delta can
				// improve on it, so it stops being searched and falls to that version.
				debug.historical_delta_pages_shard_superseded +=
					drop_shard_superseded_pages(&mut searching, shard_fold_floors, txid);
				if searching.is_empty() {
					pending = None;
					last_key = None;
					break;
				}
				pending = Some((txid, Vec::new()));
			}

			pending
				.as_mut()
				.expect("pending initialized above")
				.1
				.push((key.clone(), chunk));
			last_key = Some(key);
		}

		if searching.is_empty() {
			break;
		}
		// A short batch means the range is exhausted.
		if batch_len < DELTA_HISTORY_SCAN_BATCH_KEYS {
			break;
		}
		match last_key {
			// Continue strictly below the smallest key read this batch.
			Some(key) => batch_end = key,
			None => break,
		}
	}

	if !searching.is_empty() {
		if let Some((pending_txid, pending_chunks)) = pending.take() {
			process_delta_txid(
				source,
				pending_txid,
				pending_chunks,
				&mut searching,
				refs,
				debug,
			)?;
		}
	}

	Ok(())
}

/// Decode one fully-gathered delta txid and record a `HistoricalDelta` ref for
/// every page still being searched that it covers.
///
/// A commit's rows are reassembled into its blobs rather than concatenated. A segmented commit
/// stores one self-contained LTX blob per shard-aligned page range and every one of them restarts
/// its chunk index at zero, so sorting the rows by chunk index and joining them yields interleaved
/// bytes rather than a decodable file.
///
/// Only the blobs that can hold a page still being searched are decoded. Segments cover disjoint
/// page ranges, so a walk looking for one page does not pay to decode the rest of a large commit.
fn process_delta_txid(
	source: ReadSource,
	txid: u64,
	rows: Vec<(Vec<u8>, Vec<u8>)>,
	searching: &mut BTreeSet<u32>,
	refs: &mut BTreeMap<u32, PageRef>,
	debug: &mut GetPagesDebug,
) -> Result<()> {
	if searching.is_empty() {
		return Ok(());
	}

	let segments = source.reassemble_delta_segments(txid, rows)?;
	debug.historical_delta_txids_decoded += 1;

	let mut by_segment = BTreeMap::<usize, Vec<u32>>::new();
	for pgno in searching.iter().copied() {
		if let Some(idx) = delta_blob::segment_index_for_page(&segments, pgno) {
			by_segment.entry(idx).or_default().push(pgno);
		}
	}

	let mut found_pgnos = Vec::new();
	for (idx, pgnos) in by_segment {
		let segment = &segments[idx];
		let decoded = decode_ltx_v3(&segment.blob).with_context(|| {
			format!(
				"decode historical sqlite delta {txid} segment {:?}",
				segment.first_pgno
			)
		})?;
		found_pgnos.extend(
			pgnos
				.into_iter()
				.filter(|pgno| decoded.get_page(*pgno).is_some()),
		);
	}

	for pgno in found_pgnos {
		refs.insert(
			pgno,
			PageRef {
				source,
				txid,
				kind: PageRefKind::HistoricalDelta,
			},
		);
		searching.remove(&pgno);
	}

	Ok(())
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
