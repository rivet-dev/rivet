//! Staged commit path: a commit too large for one transaction, written as a sequence of
//! shard-aligned segments and made visible by a single finalize.
//!
//! Nothing staged is readable. A staged segment writes only its DELTA chunk rows, so there is no
//! PIDX row pointing at the txid, no COMMIT row, and head has not moved. Readers resolve pages
//! through PIDX and never see a txid above head, which makes a half-staged commit indistinguishable
//! from no commit at all.
//!
//! That holds only until something else claims the txid. An abandoned stage's rows sit at
//! `head + 1`, which is exactly the txid the next commit publishes, so every path that hands out or
//! publishes that txid has to call [`clear_abandoned_commit_stage`] first.

use std::collections::BTreeSet;

use anyhow::{Context, Result};

use crate::conveyer::{
	Db, branch,
	commit::{
		branch_init::{
			ensure_branch_writable, resolve_or_allocate_branch, write_root_branch_metadata,
		},
		helpers::{tracked_entry_size, tx_get_value},
		publish,
		truncate::collect_truncate_cleanup,
	},
	constants::{COMMIT_SEGMENT_MAX_SHARDS, MAX_COMMIT_DIRTY_PAGES, MAX_COMMIT_RAW_DIRTY_BYTES},
	delta_blob,
	error::SqliteStorageError,
	keys,
	ltx::{LtxHeader, encode_ltx_v3},
	quota,
	types::{
		CommitStageRow, DBHead, DatabaseBranchId, DirtyPage, STAGED_SEGMENT_SPAN_PAGES,
		StagedSegment, decode_commit_stage_row, decode_compaction_root, decode_db_head,
		encode_commit_stage_row,
	},
	udb,
};
use crate::{burst_mode, conveyer::types::CommitResult, metrics};

use universaldb::utils::IsolationLevel::Serializable;

/// Clears what an abandoned staged commit left at `txid` and refunds the bytes its segments were
/// charged.
///
/// A later commit at the same txid publishes every DELTA row it finds under that txid, so anything
/// the abandoned stage wrote would read back as part of it. The whole txid range is cleared rather
/// than only the rows a replacement is about to overwrite. A replacement overwrites a segment's
/// leading chunks and leaves its tail, which fails to decode. Worse, a segment covering pages the
/// replacement never touches survives whole and decodes cleanly, so the history walk and compaction
/// would serve the dead attempt's pages as that commit's.
///
/// Does not decide whether the stage is abandoned. The caller owns that: begin and single-shot
/// commit know it is, because they are the actor claiming `head + 1`, while the reclaimer has to
/// re-check head and the orphan grace window first.
pub(crate) fn clear_abandoned_commit_stage(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
	stage: &CommitStageRow,
) {
	let (delta_begin, delta_end) = keys::branch_delta_txid_range(branch_id, txid);
	tx.informal().clear_range(&delta_begin, &delta_end);
	tx.informal()
		.clear(&keys::branch_commit_stage_key(branch_id, txid));
	if stage.accounted_bytes != 0 {
		quota::atomic_add_branch(tx, branch_id, stage.accounted_bytes.saturating_neg());
	}
}

/// What a staged commit looked like, carried out of the finalize transaction so it can be metered
/// after the transaction commits rather than on every attempt.
struct StagedCommitShape {
	accounted_bytes: i64,
	segment_count: usize,
	started_at_ms: i64,
	/// Size of the finalize transaction as the database's own limit measures it, which is the bound
	/// `MAX_COMMIT_DIRTY_PAGES` exists to stay under.
	finalize_transaction_bytes: i64,
}

/// Reads the branch's current head, preferring the live head over the fork snapshot.
async fn read_head(
	tx: &universaldb::Transaction,
	branch_id: crate::conveyer::types::DatabaseBranchId,
) -> Result<(Option<DBHead>, Option<Vec<u8>>, Option<Vec<u8>>)> {
	let head_key = keys::branch_meta_head_key(branch_id);
	let head_at_fork_key = keys::branch_meta_head_at_fork_key(branch_id);
	let head_bytes = tx_get_value(tx, &head_key, Serializable).await?;
	let head_at_fork_bytes = tx_get_value(tx, &head_at_fork_key, Serializable).await?;
	let previous_head = head_bytes
		.as_ref()
		.or(head_at_fork_bytes.as_ref())
		.map(|bytes| decode_db_head(bytes))
		.transpose()
		.context("decode sqlite db head for staged commit")?;

	Ok((previous_head, head_bytes, head_at_fork_bytes))
}

fn fence_head(expected_head_txid: Option<u64>, actual_head_txid: u64) -> Result<()> {
	if let Some(expected_head_txid) = expected_head_txid {
		if expected_head_txid != actual_head_txid {
			return Err(SqliteStorageError::HeadFenceMismatch {
				expected_head_txid,
				actual_head_txid,
			}
			.into());
		}
	}

	Ok(())
}

impl Db {
	/// Opens a staged commit and returns the txid it will publish as.
	///
	/// The engine allocates the txid rather than the client: it is always `head + 1`, and Pegboard's
	/// single-writer invariant means no other actor can be allocating against the same branch, so no
	/// allocator is needed.
	///
	/// This is also the primary cleanup for an abandoned stage. An orphan sits at `head + 1`, and
	/// since an abandoned stage never moved head, the next begin is handed that same txid. Clearing
	/// the range here is a metadata operation rather than a per-row delete, and the refund is exact
	/// because the staged row recorded what it charged. Collision is the ordinary case, not an edge
	/// case, so the orphan sweep in the manager is only a backstop for a branch whose actor never
	/// returns.
	pub async fn commit_stage_begin(
		&self,
		generation: u64,
		expected_head_txid: Option<u64>,
	) -> Result<u64> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		// Engine wall clock: this stamps the orphan grace window, so it must not come from the actor.
		let now_ms = rivet_util::timestamp::now();

		self.udb
			.txn("depot_commit_stage_begin", move |tx| {
				let database_id = database_id.clone();
				async move {
					let branch_resolution =
						resolve_or_allocate_branch(&tx, bucket_id, &database_id).await?;
					let branch_id = branch_resolution.branch_id;
					// Allocating a branch is not durable until its metadata is written. The
					// single-shot path does that as part of publishing, but a staged commit spans
					// several transactions, so without it each one would resolve a different branch
					// and the staged rows would be written under an id nothing else can find.
					if branch_resolution.bucket_initialized {
						branch::write_root_bucket_metadata(
							&tx,
							bucket_id,
							branch_resolution.bucket_branch_id,
							now_ms,
							&udb::INCOMPLETE_VERSIONSTAMP,
						)?;
					}
					if branch_resolution.database_initialized {
						write_root_branch_metadata(
							&tx,
							branch_id,
							bucket_id,
							branch_resolution.bucket_branch_id,
							&database_id,
							now_ms,
							&udb::INCOMPLETE_VERSIONSTAMP,
							branch_resolution.bucket_initialized,
						)
						.await?;
					}
					ensure_branch_writable(&tx, branch_id, branch_resolution.database_initialized)
						.await?;
					let (previous_head, _, _) = read_head(&tx, branch_id).await?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					fence_head(expected_head_txid, actual_head_txid)?;
					let txid = actual_head_txid.saturating_add(1);

					// Reclaim whatever an abandoned stage left at this txid before writing over it.
					let stage_key = keys::branch_commit_stage_key(branch_id, txid);
					if let Some(existing) = tx_get_value(&tx, &stage_key, Serializable).await? {
						let existing = decode_commit_stage_row(&existing)?;
						clear_abandoned_commit_stage(&tx, branch_id, txid, &existing);
						tracing::info!(
							?branch_id,
							txid,
							refunded_bytes = existing.accounted_bytes,
							"cleared an abandoned staged commit before reusing its txid"
						);
					}

					tx.informal().set(
						&stage_key,
						&encode_commit_stage_row(CommitStageRow {
							accounted_bytes: 0,
							segments: Vec::new(),
							generation,
							started_at_ms: now_ms,
						})?,
					);

					Ok(txid)
				}
			})
			.await
	}

	/// Stages one shard-aligned segment of an open staged commit.
	pub async fn commit_stage_segment(
		&self,
		generation: u64,
		txid: u64,
		first_pgno: u32,
		dirty_pages: Vec<DirtyPage>,
	) -> Result<u64> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let max_storage_bytes = self.max_storage_bytes();
		let mut dirty_pages = dirty_pages;
		dirty_pages.sort_by_key(|page| page.pgno);

		// Charged per segment rather than once per commit: charging at finalize would be too late,
		// since the bytes have already landed by then and there would be nothing left to slow down.
		self.await_actor_throttle(universaldb::ThrottleKind::Write)
			.await;
		let staged = self
			.udb
			.txn("depot_commit_stage_segment", move |tx| {
				let database_id = database_id.clone();
				let dirty_pages = dirty_pages.clone();
				async move {
					tx.charge_throttle(
						rivet_config::config::DEPOT_ACTOR_THROTTLE,
						universaldb::ThrottleCharge::Both,
					)?;
					let branch_resolution =
						resolve_or_allocate_branch(&tx, bucket_id, &database_id).await?;
					let branch_id = branch_resolution.branch_id;
					let (previous_head, _, _) = read_head(&tx, branch_id).await?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					// Re-fence on every segment. A commit that lost the branch mid-stage must not keep
					// paying to stage bytes that can never be finalized.
					if txid != actual_head_txid.saturating_add(1) {
						return Err(SqliteStorageError::HeadFenceMismatch {
							expected_head_txid: txid.saturating_sub(1),
							actual_head_txid,
						}
						.into());
					}

					let stage_key = keys::branch_commit_stage_key(branch_id, txid);
					let Some(stage_bytes) = tx_get_value(&tx, &stage_key, Serializable).await?
					else {
						return Err(SqliteStorageError::StageNotFound { txid, first_pgno }.into());
					};
					let mut stage = decode_commit_stage_row(&stage_bytes)?;
					if stage.generation != generation {
						return Err(SqliteStorageError::StageSegmentInvalid {
							reason: format!(
								"staged commit belongs to generation {}, not {generation}",
								stage.generation
							),
						}
						.into());
					}
					validate_segment(
						&dirty_pages,
						first_pgno,
						stage.segments.last().map(|segment| segment.first_pgno),
					)?;
					let segment =
						StagedSegment::new(first_pgno, dirty_pages.iter().map(|page| page.pgno))?;

					// Refused as the commit grows past the cap rather than at finalize. Finalize is
					// where the cap actually binds, but by then the whole payload has been written
					// and charged, so an oversized commit would cost its full storage before
					// anything told the client it could never land.
					let staged_pages = stage
						.segments
						.iter()
						.chain(std::iter::once(&segment))
						.map(|segment| u64::from(segment.page_count()))
						.sum::<u64>();
					if staged_pages > MAX_COMMIT_DIRTY_PAGES as u64 {
						return Err(SqliteStorageError::CommitTooLarge {
							actual_size_bytes: staged_pages * u64::from(keys::PAGE_SIZE),
							max_size_bytes: MAX_COMMIT_RAW_DIRTY_BYTES as u64,
						}
						.into());
					}

					let encoded = encode_ltx_v3(
						LtxHeader::delta(
							txid,
							// The database size is only known at finalize, so a staged segment
							// records the size it was written against. Finalize writes the real one.
							previous_head.as_ref().map_or(0, |head| head.db_size_pages),
							stage.started_at_ms,
						),
						&dirty_pages,
					)
					.with_context(|| format!("encode staged commit segment {first_pgno}"))?;
					let chunks = delta_blob::split_delta_segment_chunks(
						branch_id, txid, first_pgno, &encoded,
					)
					.with_context(|| {
						format!("split staged commit segment {first_pgno} into chunk rows")
					})?;

					let segment_bytes = chunks
						.iter()
						.map(|(key, value)| tracked_entry_size(key, value))
						.sum::<Result<i64>>()?;
					// Charged as the segment lands, not at finalize: charging afterwards would mean
					// the bytes are already on disk before anything could refuse them, so a runaway
					// write would be caught only once it had already cost the space.
					let storage_used = quota::read_branch(&tx, branch_id).await?;
					let would_be = storage_used
						.checked_add(segment_bytes)
						.context("staged commit quota check overflowed i64")?;
					let compaction_root = tx_get_value(
						&tx,
						&keys::branch_compaction_root_key(branch_id),
						Serializable,
					)
					.await?
					.as_deref()
					.map(decode_compaction_root)
					.transpose()
					.context("decode sqlite compaction root for staged commit")?;
					let hot_quota_cap = burst_mode::adjusted_hot_quota_cap(
						max_storage_bytes,
						burst_mode::read_branch_signal_for_head(txid, compaction_root.as_ref()),
					)?;
					quota::cap_check_with_cap(would_be, hot_quota_cap)?;

					for (key, value) in &chunks {
						tx.informal().set(key, value);
					}
					quota::atomic_add_branch(&tx, branch_id, segment_bytes);

					stage.accounted_bytes = stage.accounted_bytes.saturating_add(segment_bytes);
					stage.segments.push(segment);
					tx.informal()
						.set(&stage_key, &encode_commit_stage_row(stage)?);

					Ok(u64::try_from(segment_bytes).unwrap_or(0))
				}
			})
			.await;

		// Counted once per call rather than per transaction attempt, so a retried segment does not
		// inflate either number.
		let node_id = self.node_id.to_string();
		match &staged {
			Ok(segment_bytes) => metrics::SQLITE_COMMIT_STAGE_SEGMENT_BYTES_TOTAL
				.with_label_values(&[node_id.as_str()])
				.inc_by(*segment_bytes),
			Err(err) => {
				if err.chain().any(|source| {
					matches!(
						source.downcast_ref::<SqliteStorageError>(),
						Some(SqliteStorageError::CommitTooLarge { .. })
					)
				}) {
					metrics::SQLITE_COMMIT_REJECTED_TOTAL
						.with_label_values(&[node_id.as_str(), "too_large"])
						.inc();
				}
			}
		}

		staged
	}

	/// Makes a staged commit visible in one transaction.
	///
	/// This is the atomic flip: until it commits, the staged segments are unreachable; after it,
	/// every page they carry resolves through PIDX exactly as a single-shot commit's would. It runs
	/// the same publish sequence as the single-shot path rather than its own copy, so the two cannot
	/// drift in what they write or in what order.
	/// Publishes an open staged commit.
	///
	/// `txid` is the fence. Begin allocated it as `head + 1` and finalize requires it to still be
	/// `head + 1`, so a separate `expected_head_txid` could only ever restate `txid - 1` and a
	/// caller could pass the two disagreeing.
	pub async fn commit_finalize(
		&self,
		generation: u64,
		txid: u64,
		new_db_size_pages: u32,
		now_ms: i64,
		segment_first_pgnos: Vec<u32>,
	) -> Result<CommitResult> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let node_id = self.node_id.to_string();
		let tx_node_id = node_id.clone();
		let compaction_enabled = self.compaction_signaler.is_some();
		let max_storage_bytes = self.max_storage_bytes();
		let cached_snapshot = self.cache_snapshot.read().await.clone();
		#[cfg(feature = "pidx-cache")]
		let cache_was_warm = cached_snapshot
			.as_ref()
			.is_some_and(|snapshot| !snapshot.pidx.is_empty());
		#[cfg(not(feature = "pidx-cache"))]
		let cache_was_warm = false;
		let cached_access_bucket = cached_snapshot
			.as_ref()
			.and_then(|snapshot| snapshot.last_access_bucket);
		let last_deltas_available_at_ms = if compaction_enabled {
			*self.last_deltas_available_at_ms.read().await
		} else {
			None
		};

		let (result, staged_shape) = self
			.udb
			.txn("depot_commit_finalize", move |tx| {
				let database_id = database_id.clone();
				let segment_first_pgnos = segment_first_pgnos.clone();
				let node_id = tx_node_id.clone();
				async move {
					// The staged bytes were charged as they landed, but finalize writes one PIDX row
					// per page of the commit, which at the cap is megabytes of its own. Leaving it
					// uncharged would let the largest single transaction an actor can cause be the
					// one lane nothing measures.
					tx.charge_throttle(
						rivet_config::config::DEPOT_ACTOR_THROTTLE,
						universaldb::ThrottleCharge::Both,
					)?;
					let branch_resolution =
						resolve_or_allocate_branch(&tx, bucket_id, &database_id).await?;
					let branch_id = branch_resolution.branch_id;
					// Checked again at finalize, not only at begin: a branch can be frozen while a
					// staged commit is mid-flight, and finalize is the transaction that publishes.
					ensure_branch_writable(&tx, branch_id, branch_resolution.database_initialized)
						.await?;
					let (previous_head, head_bytes, head_at_fork_bytes) =
						read_head(&tx, branch_id).await?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					if txid != actual_head_txid.saturating_add(1) {
						return Err(SqliteStorageError::HeadFenceMismatch {
							expected_head_txid: txid.saturating_sub(1),
							actual_head_txid,
						}
						.into());
					}

					let stage_key = keys::branch_commit_stage_key(branch_id, txid);
					let Some(stage_bytes) = tx_get_value(&tx, &stage_key, Serializable).await?
					else {
						return Err(SqliteStorageError::StageSegmentInvalid {
							reason: format!("no staged commit is open at txid {txid}"),
						}
						.into());
					};
					let stage = decode_commit_stage_row(&stage_bytes)?;
					if stage.generation != generation {
						return Err(SqliteStorageError::StageSegmentInvalid {
							reason: format!(
								"staged commit belongs to generation {}, not {generation}",
								stage.generation
							),
						}
						.into());
					}

					// The commit's page set comes from the stage row, never from reading the staged
					// blobs back. Each segment's row entry was written in the same transaction as
					// its chunks, so listing it is proof its bytes are present, and rebuilding the
					// page set from bitmaps keeps this transaction's size independent of how large
					// the commit is. Reading the blobs would instead pull the whole payload into
					// the one transaction that has to stay small.
					//
					// The client's claimed segment list is still checked against it, so a client
					// that lost a stage reply and finalized anyway is rejected here rather than
					// publishing a commit with a hole in it.
					let staged_first_pgnos = stage
						.segments
						.iter()
						.map(|segment| segment.first_pgno)
						.collect::<Vec<_>>();
					if staged_first_pgnos != segment_first_pgnos {
						if let Some(first_pgno) = segment_first_pgnos
							.iter()
							.find(|first_pgno| !staged_first_pgnos.contains(first_pgno))
						{
							return Err(SqliteStorageError::StageNotFound {
								txid,
								first_pgno: *first_pgno,
							}
							.into());
						}
						return Err(SqliteStorageError::StageSegmentInvalid {
							reason: format!(
								"finalize claimed {} segments but {} were staged",
								segment_first_pgnos.len(),
								staged_first_pgnos.len()
							),
						}
						.into());
					}

					let dirty_pgnos = stage
						.segments
						.iter()
						.flat_map(|segment| segment.pages())
						.collect::<BTreeSet<_>>();

					let previous_db_size_pages =
						previous_head.as_ref().map_or(0, |head| head.db_size_pages);
					let truncate_cleanup = collect_truncate_cleanup(
						&tx,
						branch_id,
						previous_db_size_pages,
						new_db_size_pages,
					)
					.await?;
					let compaction_root = tx_get_value(
						&tx,
						&keys::branch_compaction_root_key(branch_id),
						Serializable,
					)
					.await?
					.as_deref()
					.map(decode_compaction_root)
					.transpose()
					.context("decode sqlite compaction root for staged commit finalize")?;
					let branch_ancestry =
						crate::conveyer::db::load_branch_ancestry(&tx, branch_id).await?;
					let storage_used = quota::read_branch(&tx, branch_id).await?;

					// The staged chunks are already written and already charged, so publish is handed
					// an empty chunk list: writing them again would duplicate the rows and charging
					// them again would double-count the branch's usage.
					let published = publish::publish_commit(
						&tx,
						publish::PublishCommitInput {
							branch_id,
							branch_resolution: &branch_resolution,
							branch_ancestry,
							bucket_id,
							database_id: &database_id,
							txid,
							db_size_pages: new_db_size_pages,
							now_ms,
							previous_head,
							head_key: keys::branch_meta_head_key(branch_id),
							head_at_fork_key: keys::branch_meta_head_at_fork_key(branch_id),
							head_bytes,
							head_at_fork_bytes,
							dirty_pgnos,
							delta_chunks: Vec::new(),
							truncate_cleanup,
							storage_used,
							max_storage_bytes,
							compaction_root,
							compaction_enabled,
							last_deltas_available_at_ms,
							cached_access_bucket,
							phase_node_id: node_id.clone(),
							#[cfg(feature = "test-faults")]
							fault_controller: None,
						},
					)
					.await?;

					// The stage is spent. Leaving the row behind would make the next begin at this
					// txid think it found an orphan and refund bytes that are now live commit data.
					tx.informal().clear(&stage_key);

					// Read last, so it covers every write the transaction makes. This is the number
					// the commit cap is chosen against, and until it was measured the cap could
					// only be calibrated backwards from a cluster that refused a commit.
					let finalize_transaction_bytes = tx.approximate_size().await?;

					Ok((
						published,
						StagedCommitShape {
							accounted_bytes: stage.accounted_bytes,
							segment_count: stage.segments.len(),
							started_at_ms: stage.started_at_ms,
							finalize_transaction_bytes,
						},
					))
				}
			})
			.await?;

		// Metered here rather than inside the transaction so a retried attempt does not count twice,
		// and labelled `staged` so the size distribution of these commits is separable from the
		// single-shot ones they exist to replace.
		let labels = &[node_id.as_str(), metrics::COMMIT_PATH_STAGED];
		metrics::SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT
			.with_label_values(labels)
			.observe(result.dirty_pgnos.len() as f64);
		metrics::SQLITE_PUMP_COMMIT_PAYLOAD_BYTES
			.with_label_values(labels)
			.observe(staged_shape.accounted_bytes.max(0) as f64);
		metrics::SQLITE_COMMIT_STAGE_SEGMENTS
			.with_label_values(&[node_id.as_str()])
			.observe(staged_shape.segment_count as f64);
		// Wall time across the whole begin-to-finalize sequence, which is what a staged commit
		// actually costs an actor. The finalize transaction on its own says nothing about that: the
		// segments are where the round trips and the bytes are.
		metrics::SQLITE_PUMP_COMMIT_DURATION
			.with_label_values(labels)
			.observe(now_ms.saturating_sub(staged_shape.started_at_ms).max(0) as f64 / 1000.0);
		metrics::SQLITE_COMMIT_TRANSACTION_BYTES
			.with_label_values(labels)
			.observe(staged_shape.finalize_transaction_bytes.max(0) as f64);

		publish::record_published_commit(self, &result, cache_was_warm, &node_id).await?;
		self.publish_deltas_available_if_needed(result.deltas_available, result.branch_id)
			.await?;

		Ok(CommitResult {
			head_txid: result.txid,
			db_size_pages: new_db_size_pages,
		})
	}
}

/// Rejects a segment the engine cannot safely fold later.
///
/// Alignment is enforced here rather than trusted from the client: a shard split across two segments
/// could be folded from one of them and written as a shard image missing the other's newer pages,
/// which is silent corruption rather than a failed request.
fn validate_segment(
	dirty_pages: &[DirtyPage],
	first_pgno: u32,
	last_first_pgno: Option<u32>,
) -> Result<()> {
	if dirty_pages.is_empty() {
		return Err(SqliteStorageError::StageSegmentInvalid {
			reason: "segment has no pages".to_string(),
		}
		.into());
	}
	if first_pgno % keys::SHARD_SIZE != 0 {
		return Err(SqliteStorageError::StageSegmentInvalid {
			reason: format!("first page {first_pgno} is not shard-aligned"),
		}
		.into());
	}
	if let Some(last_first_pgno) = last_first_pgno {
		// Segments must not merely ascend, they must not overlap spans. Ascending alone still lets
		// two segments a single shard apart both claim shards in the middle, and a shard split
		// across two segments is the one thing alignment exists to prevent: compaction folds a
		// segment into a shard image, so it could fold one of them and write an image missing the
		// other's newer pages. That is silent corruption rather than a failed request.
		if first_pgno < last_first_pgno.saturating_add(STAGED_SEGMENT_SPAN_PAGES) {
			return Err(SqliteStorageError::StageSegmentInvalid {
				reason: format!(
					"segment {first_pgno} overlaps the span of the segment at {last_first_pgno}"
				),
			}
			.into());
		}
	}

	let shard_limit = (first_pgno / keys::SHARD_SIZE).saturating_add(COMMIT_SEGMENT_MAX_SHARDS);
	for page in dirty_pages {
		if page.pgno < first_pgno {
			return Err(SqliteStorageError::StageSegmentInvalid {
				reason: format!(
					"page {} sits below the segment start {first_pgno}",
					page.pgno
				),
			}
			.into());
		}
		if page.pgno / keys::SHARD_SIZE >= shard_limit {
			return Err(SqliteStorageError::StageSegmentInvalid {
				reason: format!(
					"page {} sits past the segment's {COMMIT_SEGMENT_MAX_SHARDS}-shard span",
					page.pgno
				),
			}
			.into());
		}
	}

	Ok(())
}
