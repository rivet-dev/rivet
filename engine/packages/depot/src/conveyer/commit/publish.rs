//! The commit publish sequence: the transaction work that makes a commit visible.
//!
//! Shared so the single-shot commit path and the staged (segmented) commit path cannot drift. Both
//! must write the same rows in the same order, and two copies of this sequence is how a corruption
//! bug gets written. Everything before it (branch resolution, the head fence, truncate collection,
//! delta encoding) differs between the two paths and stays with the caller.

use std::{collections::BTreeSet, sync::Arc, time::Instant};

use anyhow::{Context, Result};
use universaldb::options::MutationType;

#[cfg(feature = "test-faults")]
use crate::fault::{CommitFaultPoint, DepotFaultController};
use crate::metrics;
use crate::{
	burst_mode,
	conveyer::{
		Db, branch,
		db::{CacheSnapshot, touch_access_if_bucket_advanced},
		keys,
		page_index::DeltaPageIndex,
		quota,
		types::{CommitRow, DBHead, DatabaseBranchId, encode_commit_row, encode_db_head},
		udb,
	},
};

use super::{
	apply::CommitTxResult,
	branch_init::{BranchResolution, write_root_branch_metadata},
	dirty::admit_deltas_available,
	helpers::tracked_entry_size,
	truncate::{TruncateCleanup, fence_truncate_cleanup_row},
};

#[cfg(feature = "test-faults")]
use super::apply::maybe_fire_commit_fault;

/// Everything the publish sequence needs that its callers derive differently.
pub(super) struct PublishCommitInput<'a> {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) branch_resolution: &'a BranchResolution,
	pub(super) branch_ancestry: crate::conveyer::db::BranchAncestry,
	pub(super) bucket_id: crate::conveyer::types::BucketId,
	pub(super) database_id: &'a str,
	pub(super) txid: u64,
	pub(super) db_size_pages: u32,
	pub(super) now_ms: i64,
	pub(super) previous_head: Option<DBHead>,
	pub(super) head_key: Vec<u8>,
	pub(super) head_at_fork_key: Vec<u8>,
	pub(super) head_bytes: Option<Vec<u8>>,
	pub(super) head_at_fork_bytes: Option<Vec<u8>>,
	/// The pages this commit wrote. The single-shot path takes them from the request; the staged path
	/// derives them from the staged segments' LTX page indexes.
	pub(super) dirty_pgnos: BTreeSet<u32>,
	pub(super) delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	pub(super) truncate_cleanup: TruncateCleanup,
	pub(super) storage_used: i64,
	/// Bytes of storage this database may occupy, before burst mode adjusts it.
	pub(super) max_storage_bytes: i64,
	pub(super) compaction_root: Option<crate::conveyer::types::CompactionRoot>,
	pub(super) compaction_enabled: bool,
	pub(super) last_deltas_available_at_ms: Option<i64>,
	pub(super) cached_access_bucket: Option<i64>,
	pub(super) phase_node_id: String,
	#[cfg(feature = "test-faults")]
	pub(super) fault_controller: Option<DepotFaultController>,
}

/// Writes the rows that make `txid` visible, in the order the readers depend on.
pub(super) async fn publish_commit(
	tx: &universaldb::Transaction,
	input: PublishCommitInput<'_>,
) -> Result<CommitTxResult> {
	let PublishCommitInput {
		branch_id,
		branch_resolution,
		branch_ancestry,
		bucket_id,
		database_id,
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
		phase_node_id,
		#[cfg(feature = "test-faults")]
		fault_controller,
	} = input;
	let phase_start = Instant::now();

	let new_head = DBHead {
		head_txid: txid,
		db_size_pages,
		post_apply_checksum: previous_head
			.as_ref()
			.map_or(0, |head| head.post_apply_checksum),
		branch_id,
	};
	let encoded_head = encode_db_head(new_head.clone()).context("encode new sqlite db head")?;
	let txid_bytes = txid.to_be_bytes();
	let commit_row = CommitRow {
		wall_clock_ms: now_ms,
		versionstamp: udb::INCOMPLETE_VERSIONSTAMP,
		db_size_pages,
		post_apply_checksum: new_head.post_apply_checksum,
	};
	let encoded_commit_row = encode_commit_row(commit_row).context("encode sqlite commit row")?;
	let versionstamped_commit_row =
		udb::append_versionstamp_offset(encoded_commit_row.clone(), &udb::INCOMPLETE_VERSIONSTAMP)
			.context("prepare versionstamped sqlite commit row")?;
	let commit_key = keys::branch_commit_key(branch_id, txid);
	let vtx_storage_key = keys::branch_vtx_key(branch_id, udb::INCOMPLETE_VERSIONSTAMP);
	let versionstamped_vtx_key =
		udb::append_versionstamp_offset(vtx_storage_key.clone(), &udb::INCOMPLETE_VERSIONSTAMP)
			.context("prepare versionstamped sqlite vtx key")?;
	// Named for what it measures. The delta blobs are encoded by the caller, which records
	// `encode_delta` itself; reusing that label here double-counted one commit under two spans that
	// time different work.
	metrics::observe_commit_phase(&phase_node_id, "encode_head", phase_start, "ok");

	let added_bytes = tracked_entry_size(&head_key, &encoded_head)?
		+ tracked_entry_size(&commit_key, &encoded_commit_row)?
		+ tracked_entry_size(&vtx_storage_key, &txid_bytes)?
		+ delta_chunks
			.iter()
			.map(|(key, value)| tracked_entry_size(key, value))
			.sum::<Result<i64>>()?
		+ truncate_cleanup.added_bytes
		+ dirty_pgnos
			.iter()
			.map(|pgno| tracked_entry_size(&keys::branch_pidx_key(branch_id, *pgno), &txid_bytes))
			.sum::<Result<i64>>()?;
	let removed_bytes = head_bytes
		.as_ref()
		.map_or(Ok(0), |bytes| tracked_entry_size(&head_key, bytes))?
		+ truncate_cleanup.deleted_bytes;
	let quota_delta = added_bytes
		.checked_sub(removed_bytes)
		.context("sqlite commit quota delta overflowed i64")?;
	let would_be = storage_used
		.checked_add(quota_delta)
		.context("sqlite commit quota check overflowed i64")?;
	let burst_signal = burst_mode::read_branch_signal_for_head(txid, compaction_root.as_ref());
	let deltas_available = if compaction_enabled {
		admit_deltas_available(
			tx,
			branch_id,
			txid,
			compaction_root.as_ref(),
			burst_signal.cold_watermark_txid,
			now_ms,
			last_deltas_available_at_ms,
		)
		.await?
	} else {
		None
	};
	let hot_quota_cap = burst_mode::adjusted_hot_quota_cap(max_storage_bytes, burst_signal)?;
	quota::cap_check_with_cap(would_be, hot_quota_cap)?;

	#[cfg(feature = "test-faults")]
	maybe_fire_commit_fault(
		&fault_controller,
		&database_id,
		CommitFaultPoint::BeforeDeltaWrites,
		Some(branch_id),
	)
	.await?;
	let phase_start = Instant::now();
	for (key, value) in &delta_chunks {
		tx.informal().set(key, value);
	}
	metrics::observe_commit_phase(&phase_node_id, "write_delta_chunks", phase_start, "ok");
	#[cfg(feature = "test-faults")]
	maybe_fire_commit_fault(
		&fault_controller,
		&database_id,
		CommitFaultPoint::BeforePidxWrites,
		Some(branch_id),
	)
	.await?;
	let phase_start = Instant::now();
	for pgno in &dirty_pgnos {
		tx.informal()
			.set(&keys::branch_pidx_key(branch_id, *pgno), &txid_bytes);
	}
	for row in &truncate_cleanup.pidx_clears {
		fence_truncate_cleanup_row(tx, row).await?;
		tx.informal().clear(&row.key);
	}
	for row in &truncate_cleanup.shard_clears {
		fence_truncate_cleanup_row(tx, row).await?;
		tx.informal().clear(&row.key);
	}
	for (key, value) in &truncate_cleanup.shard_writes {
		tx.informal().set(key, value);
	}
	metrics::observe_commit_phase(&phase_node_id, "write_page_index", phase_start, "ok");
	#[cfg(feature = "test-faults")]
	maybe_fire_commit_fault(
		&fault_controller,
		&database_id,
		CommitFaultPoint::BeforeHeadWrite,
		Some(branch_id),
	)
	.await?;
	let phase_start = Instant::now();
	tx.informal().set(&head_key, &encoded_head);
	if head_at_fork_bytes.is_some() {
		tx.informal().clear(&head_at_fork_key);
	}
	if branch_resolution.bucket_initialized {
		branch::write_root_bucket_metadata(
			tx,
			bucket_id,
			branch_resolution.bucket_branch_id,
			now_ms,
			&udb::INCOMPLETE_VERSIONSTAMP,
		)?;
	}
	if branch_resolution.database_initialized {
		write_root_branch_metadata(
			tx,
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
	#[cfg(feature = "test-faults")]
	maybe_fire_commit_fault(
		&fault_controller,
		&database_id,
		CommitFaultPoint::BeforeCommitRows,
		Some(branch_id),
	)
	.await?;
	tx.informal().atomic_op(
		&commit_key,
		&versionstamped_commit_row,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&versionstamped_vtx_key,
		&txid_bytes,
		MutationType::SetVersionstampedKey,
	);
	#[cfg(feature = "test-faults")]
	maybe_fire_commit_fault(
		&fault_controller,
		&database_id,
		CommitFaultPoint::BeforeQuotaMutation,
		Some(branch_id),
	)
	.await?;
	if quota_delta != 0 {
		quota::atomic_add_branch(tx, branch_id, quota_delta);
	}
	let touched_shards = dirty_pgnos
		.iter()
		.map(|pgno| pgno / keys::SHARD_SIZE)
		.collect::<BTreeSet<_>>();
	let access_bucket = touch_access_if_bucket_advanced(
		tx,
		branch_id,
		cached_access_bucket,
		&touched_shards,
		now_ms,
	)
	.await?;
	metrics::observe_commit_phase(&phase_node_id, "write_manifest", phase_start, "ok");

	Ok(CommitTxResult {
		branch_id,
		branch_ancestry,
		access_bucket,
		txid,
		deltas_available,
		dirty_pgnos,
		truncated_pgnos: truncate_cleanup.truncated_pgnos,
		added_bytes,
		storage_used: would_be,
	})
}

/// Records a published commit in the in-process caches every commit path shares.
///
/// Split out of the single-shot path so the staged path cannot skip it. Skipping it is not merely a
/// missed optimization: the per-page index is only valid at one head, so a commit that advances the
/// head without touching the cache leaves a snapshot that the next commit would adopt and stamp with
/// its own txid, hiding the pages in between from every read that trusts the cache.
pub(super) async fn record_published_commit(
	db: &Db,
	result: &CommitTxResult,
	cache_was_warm: bool,
	node_id: &str,
) -> Result<()> {
	let phase_start = Instant::now();
	*db.storage_used.write().await = Some(result.storage_used);
	db.commit_bytes_since_rollup.fetch_add(
		u64::try_from(result.added_bytes).context("commit added bytes should be non-negative")?,
		std::sync::atomic::Ordering::Relaxed,
	);
	#[cfg(not(feature = "pidx-cache"))]
	let _ = (cache_was_warm, &result.dirty_pgnos, &result.truncated_pgnos);

	let mut cache_snapshot = db.cache_snapshot.write().await;
	let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
	let publish_branch_changed =
		current_branch_id.is_some_and(|branch_id| branch_id != result.branch_id);
	// A snapshot may only be carried forward when this commit is the one that advances the head it
	// was built at. Any other txid means something else published in between, and its pages are
	// missing from the index.
	#[cfg(feature = "pidx-cache")]
	let cache_head_stale = cache_snapshot
		.as_ref()
		.is_some_and(|snapshot| snapshot.cache_head_txid != result.txid.saturating_sub(1));
	#[cfg(feature = "pidx-cache")]
	let pidx = if publish_branch_changed || cache_head_stale {
		Arc::new(DeltaPageIndex::new())
	} else {
		cache_snapshot
			.as_ref()
			.map(|snapshot| Arc::clone(&snapshot.pidx))
			.unwrap_or_else(|| Arc::new(DeltaPageIndex::new()))
	};
	#[cfg(not(feature = "pidx-cache"))]
	let pidx = Arc::new(DeltaPageIndex::new());
	#[cfg(not(feature = "pidx-cache"))]
	let _ = publish_branch_changed;
	#[cfg(feature = "pidx-cache")]
	let pidx_was_warm = !pidx.is_empty();
	// Maintain the cache only when it was already warm (or was just reset, giving a fresh index to
	// seed): refresh every dirty page to this commit's txid and drop truncated pages. A cold cache is
	// left untouched so a commit never makes it claim ownership of a page the store could later
	// evict. Seeding a reset index is safe for the opposite reason: every entry it then holds was
	// written by this commit, so all of them are correct at the head it is stamped with.
	#[cfg(feature = "pidx-cache")]
	if cache_was_warm || pidx_was_warm || publish_branch_changed || cache_head_stale {
		for pgno in &result.truncated_pgnos {
			pidx.remove(*pgno);
		}
		for pgno in &result.dirty_pgnos {
			pidx.insert_owner(*pgno, result.txid);
		}
	}
	let last_access_bucket = result.access_bucket.or_else(|| {
		cache_snapshot
			.as_ref()
			.filter(|snapshot| snapshot.branch_id == result.branch_id)
			.and_then(|snapshot| snapshot.last_access_bucket)
	});
	*cache_snapshot = Some(CacheSnapshot {
		branch_id: result.branch_id,
		ancestors: result.branch_ancestry.clone(),
		last_access_bucket,
		pidx,
		cache_head_txid: result.txid,
	});
	metrics::observe_commit_phase(node_id, "cache_update", phase_start, "ok");

	Ok(())
}
