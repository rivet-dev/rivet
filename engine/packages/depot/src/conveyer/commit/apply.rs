use std::{collections::BTreeSet, time::Instant};

use anyhow::{Context, Result, ensure};
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};

#[cfg(feature = "test-faults")]
use crate::fault::{CommitFaultPoint, DepotFaultContext, DepotFaultController, DepotFaultPoint};
use crate::metrics;
use crate::{
	conveyer::{
		Db,
		constants::{MAX_SINGLE_SHOT_COMMIT_DIRTY_PAGES, MAX_SINGLE_SHOT_COMMIT_RAW_DIRTY_BYTES},
		db::{BranchAncestry, load_branch_ancestry},
		delta_blob,
		error::SqliteStorageError,
		keys,
		ltx::{LtxHeader, encode_ltx_v3},
		quota,
		types::{
			CommitOptions, CommitResult, DatabaseBranchId, DirtyPage, decode_commit_stage_row,
			decode_compaction_root, decode_db_head,
		},
	},
	workflows::compaction::DeltasAvailable,
};

use super::{
	branch_init::{ensure_branch_writable, resolve_or_allocate_branch},
	helpers::tx_get_value,
	publish,
	stage::clear_abandoned_commit_stage,
	test_hooks,
	truncate::collect_truncate_cleanup,
};

impl Db {
	pub async fn commit(
		&self,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
	) -> Result<()> {
		self.commit_with_options(dirty_pages, db_size_pages, now_ms, CommitOptions::default())
			.await
			.map(|_| ())
	}

	pub async fn commit_with_options(
		&self,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
		options: CommitOptions,
	) -> Result<CommitResult> {
		validate_dirty_pages(&dirty_pages, options.disable_size_cap)?;
		// Sorted once here rather than per transaction attempt. Cutting the delta into shard-aligned
		// segments needs ascending pages, and the LTX encoder wants them sorted anyway.
		let mut dirty_pages = dirty_pages;
		dirty_pages.sort_by_key(|page| page.pgno);
		#[cfg(feature = "test-faults")]
		maybe_fire_commit_fault(
			&self.fault_controller,
			&self.database_id,
			CommitFaultPoint::BeforeTx,
			None,
		)
		.await?;

		let node_id = self.node_id.to_string();
		let max_storage_bytes = self.max_storage_bytes();
		let labels = &[node_id.as_str(), metrics::COMMIT_PATH_SINGLE_SHOT];
		let _timer = metrics::SQLITE_PUMP_COMMIT_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT
			.with_label_values(labels)
			.observe(dirty_pages.len() as f64);
		let commit_payload_bytes = dirty_pages
			.iter()
			.map(|page| page.bytes.len())
			.fold(0_usize, usize::saturating_add);
		metrics::SQLITE_PUMP_COMMIT_PAYLOAD_BYTES
			.with_label_values(labels)
			.observe(commit_payload_bytes as f64);

		let phase_start = Instant::now();
		let cached_storage_used = *self.storage_used.read().await;
		let cached_snapshot = self.cache_snapshot.read().await.clone();
		let cached_branch_id = cached_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let cached_ancestry = cached_snapshot
			.as_ref()
			.map(|snapshot| snapshot.ancestors.clone());
		let cached_access_bucket = cached_snapshot
			.as_ref()
			.and_then(|snapshot| snapshot.last_access_bucket);
		let compaction_enabled = self.compaction_signaler.is_some();
		let last_deltas_available_at_ms = if compaction_enabled {
			*self.last_deltas_available_at_ms.read().await
		} else {
			None
		};
		#[cfg(feature = "pidx-cache")]
		let cache_was_warm = cached_snapshot
			.as_ref()
			.is_some_and(|snapshot| !snapshot.pidx.is_empty());
		#[cfg(not(feature = "pidx-cache"))]
		let cache_was_warm = false;
		metrics::observe_commit_phase(&node_id, "cache_snapshot", phase_start, "ok");
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let dirty_pages_for_tx = dirty_pages.clone();
		let expected_head_txid = options.expected_head_txid;
		let phase_node_id = node_id.clone();
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();

		// Write backpressure before the transaction opens, so the wait never holds one.
		self.await_actor_throttle(universaldb::ThrottleKind::Write)
			.await;
		let result = self
			.udb
			.txn("depot_commit", move |tx| {
				let phase_node_id = phase_node_id.clone();
				let database_id = database_id.clone();
				let bucket_id = bucket_id;
				let dirty_pages = dirty_pages_for_tx.clone();
				let expected_head_txid = expected_head_txid;
				let cached_ancestry = cached_ancestry.clone();
				let cached_access_bucket = cached_access_bucket;
				let compaction_enabled = compaction_enabled;
				let last_deltas_available_at_ms = last_deltas_available_at_ms;
				#[cfg(feature = "test-faults")]
				let fault_controller = fault_controller.clone();

				async move {
					// A commit reads (branch resolution, head, truncate scan) and writes, so both axes
					// are charged.
					tx.charge_throttle(
						rivet_config::config::DEPOT_ACTOR_THROTTLE,
						universaldb::ThrottleCharge::Both,
					)?;
					let phase_start = Instant::now();
					let branch_resolution =
						resolve_or_allocate_branch(&tx, bucket_id, &database_id).await?;
					let branch_id = branch_resolution.branch_id;
					#[cfg(feature = "test-faults")]
					maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						CommitFaultPoint::AfterBranchResolution,
						Some(branch_id),
					)
					.await?;
					ensure_branch_writable(
						&tx,
						branch_id,
						branch_resolution.database_initialized,
					)
					.await?;
					let branch_ancestry = if branch_resolution.database_initialized {
						BranchAncestry::root(branch_id)
					} else if let Some(cached_ancestry) =
						cached_ancestry.filter(|ancestry| ancestry.root_branch_id == branch_id)
					{
						cached_ancestry
					} else {
						load_branch_ancestry(&tx, branch_id).await?
					};
					metrics::observe_commit_phase(
						&phase_node_id,
						"resolve_branch",
						phase_start,
						"ok",
					);
					let head_key = keys::branch_meta_head_key(branch_id);
					let head_at_fork_key = keys::branch_meta_head_at_fork_key(branch_id);
					let branch_cache_matches = cached_branch_id == Some(branch_id);
					let phase_start = Instant::now();
					let (head_bytes, head_at_fork_bytes, storage_used) =
						if let (true, Some(storage_used)) =
							(branch_cache_matches, cached_storage_used)
						{
							(
								tx_get_value(&tx, &head_key, Serializable).await?,
								tx_get_value(&tx, &head_at_fork_key, Serializable).await?,
								storage_used,
							)
						} else {
							let quota_fut = quota::read_branch(&tx, branch_id);
							let head_fut = tx_get_value(&tx, &head_key, Serializable);
							let head_at_fork_fut =
								tx_get_value(&tx, &head_at_fork_key, Serializable);
							let (head_bytes, head_at_fork_bytes, storage_used) =
								tokio::try_join!(head_fut, head_at_fork_fut, quota_fut)?;
							(head_bytes, head_at_fork_bytes, storage_used)
						};
					metrics::observe_commit_phase(
						&phase_node_id,
						"head_read",
						phase_start,
						"ok",
					);

					let previous_head_bytes = head_bytes.as_ref().or(head_at_fork_bytes.as_ref());
					let previous_head = previous_head_bytes
						.map(|bytes| decode_db_head(bytes.as_slice()))
						.transpose()
						.context("decode current sqlite db head")?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					if let Some(expected_head_txid) = expected_head_txid {
						if expected_head_txid != actual_head_txid {
							tracing::error!(
								%database_id,
								?branch_id,
								expected_head_txid,
								actual_head_txid,
								"sqlite head fence mismatch; this indicates multiple actor instances are writing the same sqlite database in parallel, which is incorrect actor lifecycle behavior"
							);
							return Err(SqliteStorageError::HeadFenceMismatch {
								expected_head_txid,
								actual_head_txid,
							}
							.into());
						}
					}
					#[cfg(feature = "test-faults")]
					maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						CommitFaultPoint::AfterHeadRead,
						Some(branch_id),
					)
					.await?;
					let compaction_root =
						tx_get_value(&tx, &keys::branch_compaction_root_key(branch_id), Snapshot)
							.await?
							.as_deref()
							.map(decode_compaction_root)
							.transpose()
							.context("decode sqlite compaction root for dirty admission")?;
					let previous_db_size_pages = previous_head
						.as_ref()
						.map_or(db_size_pages, |head| head.db_size_pages);
					let txid = match previous_head.as_ref() {
						Some(head) => head
							.head_txid
							.checked_add(1)
							.context("sqlite head txid overflowed")?,
						None => 1,
					};
					// An abandoned staged commit leaves its rows at head + 1, which is the txid this
					// commit is about to publish, and anything still there would be published with it.
					let stage_key = keys::branch_commit_stage_key(branch_id, txid);
					let storage_used =
						if let Some(stage_bytes) =
							tx_get_value(&tx, &stage_key, Serializable).await?
						{
							let stage = decode_commit_stage_row(&stage_bytes)?;
							// The in-process quota cache is not updated while segments are staged, so
							// read the authoritative counter before refunding the abandoned bytes.
							let storage_used = quota::read_branch(&tx, branch_id)
								.await?
								.checked_sub(stage.accounted_bytes)
								.context("sqlite abandoned stage quota refund underflowed i64")?;
							clear_abandoned_commit_stage(&tx, branch_id, txid, &stage);
							tracing::info!(
								?branch_id,
								txid,
								refunded_bytes = stage.accounted_bytes,
								"cleared an abandoned staged commit before a single-shot commit reused its txid"
							);
							storage_used
						} else {
							storage_used
						};

					let phase_start = Instant::now();
					let truncate_cleanup = collect_truncate_cleanup(
						&tx,
						branch_id,
						previous_db_size_pages,
						db_size_pages,
					)
					.await?;
					metrics::observe_commit_phase(
						&phase_node_id,
						"truncate_cleanup",
						phase_start,
						"ok",
					);
					test_hooks::maybe_pause_after_truncate_cleanup(&database_id).await;
					#[cfg(feature = "test-faults")]
					maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						CommitFaultPoint::AfterTruncateCleanup,
						Some(branch_id),
					)
					.await?;

					let phase_start = Instant::now();
					// One self-contained LTX blob per shard-aligned page range, rather than one
					// blob for the whole commit. Each is independently decodable, so a reader
					// serving one page loads only the range holding it, and compaction can fold
					// part of a commit and still write complete shard images.
					let mut delta_chunks = Vec::new();
					for segment_pages in delta_blob::cut_page_segments(&dirty_pages) {
						let first_pgno = delta_blob::segment_first_pgno(segment_pages)?;
						let encoded_segment = encode_ltx_v3(
							LtxHeader::delta(txid, db_size_pages, now_ms),
							segment_pages,
						)
						.with_context(|| format!("encode commit delta segment {first_pgno}"))?;
						delta_chunks.extend(
							delta_blob::split_delta_segment_chunks(
								branch_id,
								txid,
								first_pgno,
								&encoded_segment,
							)
							.with_context(|| {
								format!("split commit delta segment {first_pgno} into chunk rows")
							})?,
						);
					}
					#[cfg(feature = "test-faults")]
					maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						CommitFaultPoint::AfterLtxEncode,
						Some(branch_id),
					)
					.await?;

					let dirty_pgnos = dirty_pages
						.iter()
						.map(|page| page.pgno)
						.collect::<BTreeSet<_>>();
					metrics::observe_commit_phase(
						&phase_node_id,
						"encode_delta",
						phase_start,
						"ok",
					);

					let published = publish::publish_commit(
						&tx,
						publish::PublishCommitInput {
							branch_id,
							branch_resolution: &branch_resolution,
							branch_ancestry,
							bucket_id,
							database_id: &database_id,
							txid,
							db_size_pages,
							now_ms,
							previous_head,
							head_key,
							head_at_fork_key,
							head_bytes,
							head_at_fork_bytes,
							dirty_pgnos,
							delta_chunks,
							truncate_cleanup,
							storage_used,
							max_storage_bytes,
							compaction_root,
							compaction_enabled,
							last_deltas_available_at_ms,
							cached_access_bucket,
							phase_node_id: phase_node_id.clone(),
							#[cfg(feature = "test-faults")]
							fault_controller: fault_controller.clone(),
						},
					)
					.await?;

					// Read last, so it covers every write the transaction makes. A small commit is
					// nowhere near the limit, but truncate cleanup runs on this path too and is
					// bounded by the size of the shrink rather than by the commit cap, so this is
					// where a shrink of a large database shows up against the limit.
					let transaction_bytes = tx.approximate_size().await?;

					Ok((published, transaction_bytes))
				}
			})
			.await?;
		let (result, transaction_bytes) = result;
		metrics::SQLITE_COMMIT_TRANSACTION_BYTES
			.with_label_values(labels)
			.observe(transaction_bytes.max(0) as f64);
		#[cfg(feature = "test-faults")]
		maybe_fire_commit_fault(
			&self.fault_controller,
			&self.database_id,
			CommitFaultPoint::AfterUdbCommit,
			Some(result.branch_id),
		)
		.await?;

		publish::record_published_commit(self, &result, cache_was_warm, &node_id).await?;

		self.publish_deltas_available_if_needed(result.deltas_available, result.branch_id)
			.await?;

		Ok(CommitResult {
			head_txid: result.txid,
			db_size_pages,
		})
	}

	pub(super) async fn publish_deltas_available_if_needed(
		&self,
		signal: Option<DeltasAvailable>,
		branch_id: DatabaseBranchId,
	) -> Result<()> {
		#[cfg(not(feature = "test-faults"))]
		let _ = branch_id;

		let Some(signal) = signal else {
			return Ok(());
		};
		let Some(signaler) = &self.compaction_signaler else {
			return Ok(());
		};

		#[cfg(feature = "test-faults")]
		maybe_fire_commit_fault(
			&self.fault_controller,
			&self.database_id,
			CommitFaultPoint::BeforeCompactionSignal,
			Some(branch_id),
		)
		.await?;
		let signal_at_ms = signal.dirty_updated_at_ms;
		if let Err(err) = signaler(signal).await {
			tracing::warn!(?err, "failed to send sqlite workflow compaction wakeup");
			return Ok(());
		}
		#[cfg(feature = "test-faults")]
		maybe_fire_commit_fault(
			&self.fault_controller,
			&self.database_id,
			CommitFaultPoint::AfterCompactionSignal,
			Some(branch_id),
		)
		.await?;

		*self.last_deltas_available_at_ms.write().await = Some(signal_at_ms);
		Ok(())
	}
}

#[cfg(feature = "test-faults")]
pub(super) async fn maybe_fire_commit_fault(
	controller: &Option<DepotFaultController>,
	database_id: &str,
	point: CommitFaultPoint,
	branch_id: Option<DatabaseBranchId>,
) -> Result<()> {
	let Some(controller) = controller else {
		return Ok(());
	};

	let mut context = DepotFaultContext::new().database_id(database_id.to_string());
	if let Some(branch_id) = branch_id {
		context = context.database_branch_id(branch_id);
	}
	controller
		.maybe_fire(DepotFaultPoint::Commit(point), context)
		.await?;
	Ok(())
}

fn validate_dirty_pages(dirty_pages: &[DirtyPage], disable_size_cap: bool) -> Result<()> {
	let mut seen = BTreeSet::new();
	let mut actual_size_bytes = 0_u64;
	for page in dirty_pages {
		ensure!(page.pgno > 0, "sqlite commit does not accept page 0");
		ensure!(
			page.bytes.len() == keys::PAGE_SIZE as usize,
			"sqlite commit page {} had {} bytes, expected {}",
			page.pgno,
			page.bytes.len(),
			keys::PAGE_SIZE
		);
		ensure!(
			seen.insert(page.pgno),
			"sqlite commit duplicated page {} in a single request",
			page.pgno
		);
		actual_size_bytes =
			actual_size_bytes.saturating_add(u64::try_from(page.bytes.len()).unwrap_or(u64::MAX));
	}

	if dirty_pages.len() > MAX_SINGLE_SHOT_COMMIT_DIRTY_PAGES
		|| actual_size_bytes > MAX_SINGLE_SHOT_COMMIT_RAW_DIRTY_BYTES as u64
	{
		tracing::warn!(
			dirty_pages = dirty_pages.len(),
			actual_size_bytes,
			max_dirty_pages = MAX_SINGLE_SHOT_COMMIT_DIRTY_PAGES,
			max_size_bytes = MAX_SINGLE_SHOT_COMMIT_RAW_DIRTY_BYTES,
			"sqlite commit exceeds the engine-side single-shot size cap"
		);
		if !disable_size_cap {
			return Err(SqliteStorageError::CommitTooLarge {
				actual_size_bytes,
				max_size_bytes: MAX_SINGLE_SHOT_COMMIT_RAW_DIRTY_BYTES as u64,
			}
			.into());
		}
	}

	Ok(())
}

pub(super) struct CommitTxResult {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) branch_ancestry: BranchAncestry,
	pub(super) access_bucket: Option<i64>,
	pub(super) txid: u64,
	pub(super) deltas_available: Option<DeltasAvailable>,
	pub(super) dirty_pgnos: BTreeSet<u32>,
	pub(super) truncated_pgnos: Vec<u32>,
	pub(super) added_bytes: i64,
	pub(super) storage_used: i64,
}
