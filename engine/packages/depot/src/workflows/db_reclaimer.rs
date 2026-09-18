use std::time::Instant;

use rivet_config::config::DEPOT_COMPACTION_THROTTLE;
use universaldb::prelude::*;

use crate::{
	compaction::{
		companion::{CompanionKind, run_companion_loop},
		shared::*,
		test_hooks, *,
	},
	conveyer::{
		commit::clear_abandoned_commit_stage,
		constants::COMMIT_STAGE_ORPHAN_GRACE_MS,
		types::{ReclaimPlanOutcome, ReclaimProgress, encode_reclaim_progress},
	},
	metrics,
	workflows::db_manager::branch_record_is_live_at_generation,
};
use universaldb::prelude::{Priority, ThrottleCharge};
use universaldb::utils::CHUNK_SIZE;

#[cfg(feature = "test-faults")]
use crate::fault::ReclaimFaultPoint;

#[workflow(DbReclaimerWorkflow)]
pub async fn depot_db_reclaimer3(ctx: &mut WorkflowCtx, input: &DbReclaimerInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Reclaim).await
}

/// Replans the next reclaim slice from current FDB state. Reclaim applies its deletes immediately,
/// so a drain loop just replans after each executed slice until the whole reclaimable range has been
/// swept. This uses a focused snapshot read rather than the manager refresh so it does not clear the
/// hot dirty marker as a side effect.
///
/// The commit/delta lane is windowed by `commit_scan_cursor`, so an empty plan means only that this
/// window held nothing reclaimable. The drain decides whether to stop from `commit_scan_complete` and
/// `next_cold_scan_cursor`, never from `planned` alone.
#[activity(PlanReclaimSlice)]
pub async fn plan_reclaim_slice(
	ctx: &ActivityCtx,
	input: &PlanReclaimSliceInput,
) -> Result<PlanReclaimSliceOutput> {
	let input = input.clone();
	let _database_branch_id = input.database_branch_id;
	let now_ms = ctx.ts();
	let job_id = Id::new_v1(ctx.config().dc_label());
	// Resolved once for the whole activity rather than inside the transaction closure, so a config
	// change mid-flight cannot give two attempts of the same transaction different budgets.
	let throttle_class = throttle::CompactionThrottleClass::Reclaim.resolve(ctx.config());

	let output = ctx
		.udb()?
		.txn("depot_reclaim_plan_slice", move |tx| {
			let input = input.clone();
			async move {
				tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Both)?;
				tx.priority(Priority::Low)?;
				plan_reclaim_slice_tx(&tx, &input, job_id, now_ms, throttle_class).await
			}
		})
		.await?;

	if output.throttled {
		metrics::record_compaction_throttled(metrics::COMPACTION_KIND_RECLAIM);
	}

	Ok(output)
}

/// Output for a plan pass that cannot produce work no matter how far the cursors advance, so the drain
/// stops instead of replanning the same window.
fn plan_reclaim_slice_aborted() -> PlanReclaimSliceOutput {
	PlanReclaimSliceOutput {
		planned: None,
		next_cold_scan_cursor: None,
		next_commit_scan_cursor: 0,
		next_segment_pgno: None,
		commit_scan_complete: true,
		throttled: false,
	}
}

/// Output for a pass that read nothing because the cluster-wide compaction read budget was spent. The
/// cursors are handed back unchanged so the next pass replans the same window.
fn plan_reclaim_slice_throttled(input: &PlanReclaimSliceInput) -> PlanReclaimSliceOutput {
	PlanReclaimSliceOutput {
		planned: None,
		next_cold_scan_cursor: input.cold_scan_cursor,
		next_commit_scan_cursor: input.commit_scan_cursor,
		next_segment_pgno: input.cursor_segment_pgno,
		commit_scan_complete: false,
		throttled: true,
	}
}

/// One chunk of the v2 commit/delta history sweep: derives a window of `COMMITS`/`DELTA` reclaim
/// candidates and clears them in the same transaction.
///
/// The derive and the clear share a transaction on purpose. Split across two, the clearing side has to
/// re-derive the whole window and reject on any mismatch, which scans the history twice per unit of
/// work and turns every race into discarded reads. Together, the `Serializable` reads are themselves
/// the fence: a racing pin or commit touches `DB_PIN`/`PIDX`/`COMMITS` and aborts this transaction, and
/// every clear is a `COMPARE_AND_CLEAR` against the value this transaction read. Nothing downstream
/// consumes the commit/delta candidate set, so there is nothing a planned set would buy.
///
/// One chunk is one transaction, so a conflict costs one window rather than a whole drain, and the
/// cursor only advances on a commit.
#[activity(SweepCommitDeltaChunk)]
pub async fn sweep_commit_delta_chunk(
	ctx: &ActivityCtx,
	input: &SweepCommitDeltaChunkInput,
) -> Result<SweepCommitDeltaChunkOutput> {
	// Re-check the admission percent every window so an operator lowering it reaches jobs already in
	// flight. Unlike the hot and cold drains, reclaim ends its job instead of parking: its slot is
	// shared with the staging cleanups, and its cursors are per-job, so a later job simply replans
	// from current FDB state and nothing is lost.
	if !input.bypass_admission && !branch_admitted_now(ctx.config(), input.database_branch_id) {
		return Ok(admission_blocked_sweep_chunk(input));
	}

	let input_for_tx = input.clone();
	let now_ms = ctx.ts();
	// Resolved once for the whole activity rather than inside the transaction closure, so a config
	// change mid-flight cannot give two attempts of the same transaction different budgets.
	let throttle_class = throttle::CompactionThrottleClass::Reclaim.resolve(ctx.config());

	let start = Instant::now();
	let result = ctx
		.udb()?
		.txn("depot_reclaim_sweep_commit_delta", move |tx| {
			let input = input_for_tx.clone();
			async move {
				tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Both)?;
				tx.priority(Priority::Low)?;
				sweep_commit_delta_chunk_tx(&tx, &input, now_ms, throttle_class).await
			}
		})
		.await;
	metrics::record_reclaim_sweep_chunk(start, &result);

	let output = result?;
	if output.throttled {
		metrics::record_compaction_throttled(metrics::COMPACTION_KIND_RECLAIM);
	}

	Ok(output)
}

pub(crate) async fn sweep_commit_delta_chunk_tx(
	tx: &universaldb::Transaction,
	input: &SweepCommitDeltaChunkInput,
	now_ms: i64,
	throttle_class: ThrottleClass,
) -> Result<SweepCommitDeltaChunkOutput> {
	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for commit/delta sweep")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_sweep_chunk(
			input,
			"database branch lifecycle changed",
		));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for commit/delta sweep")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_sweep_chunk(
			input,
			"base manifest generation changed",
		));
	}

	// Gate on the read axis only. Every row this chunk clears is one it scanned, so its write volume is
	// bounded by its read volume and a second gate on the write axis would only add a way to stall
	// after the reads were already spent. The write charge below still lands, because that budget is
	// cluster-wide and shared with hot compaction.
	let read_decision = tx.check_throttle(
		DEPOT_COMPACTION_THROTTLE,
		ThrottleKind::Read,
		throttle_class,
	)?;
	if !read_decision.allowed {
		return Ok(throttled_sweep_chunk(input));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_sweep_chunk(
			input,
			"bucket fork proof is ambiguous",
		));
	}

	let mut budget = CompactionBatchBudget::fdb();
	let (pitr_interval_retention, expired_pitr_interval_rows) = read_pitr_interval_reclaim_rows(
		tx,
		input.database_branch_id,
		now_ms,
		Serializable,
		&mut budget,
	)
	.await?;

	let window = read_commit_delta_reclaim_window(
		tx,
		input.database_branch_id,
		&root,
		&db_pins,
		&pitr_interval_retention,
		input.commit_scan_cursor,
		input.cursor_segment_pgno,
		Serializable,
		&mut budget,
		// No elapsed bound. The window function's truncation contract is built for the v1 delete: it
		// rewinds the cursor and returns before the delta-materialization gate, so the sets are partial
		// and classify nothing. A sweep handed that would clear nothing, report the cursor unmoved, and
		// re-derive the same window forever. A sweep could support truncation, but only with the
		// opposite contract (keep what was classified and advance the cursor to where it stopped), so
		// do not simply pass a deadline here.
		None,
	)
	.await?;
	// Unreachable while the deadline above is `None`. Fail closed rather than silently clearing nothing
	// on a partial set, so adding a deadline cannot turn into a livelock that looks like idleness.
	if window.scan_truncated {
		return Ok(rejected_sweep_chunk(
			input,
			"commit/delta window truncated; sweep has no truncation contract",
		));
	}

	let delta_reclaim_segments = window
		.delta_reclaim_segments
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	let commit_reclaim_txids = window
		.commit_reclaim_txids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	let mut row_stats = ReclaimRowStats::default();
	// Every row the window read is charged against the read budget whether or not it is reclaimable,
	// so scanned is recorded before any classification.
	row_stats.commit.scan(window.commits.len());
	row_stats.delta.scan(window.delta_chunks.len());
	// Both halves of the interval read are scanned rows: retained rows are read in full to build the
	// coverage set the classification depends on, and only the expired half is a delete candidate.
	row_stats
		.pitr_interval
		.scan(pitr_interval_retention.len() + expired_pitr_interval_rows.len());

	// COMMITS/VTX for non-fold txids below the cold-watermark-capped bound. Not billable keys, so no
	// quota credit is taken.
	for (txid, key, value, commit) in &window.commits {
		if !commit_reclaim_txids.contains(txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.commit.clear(key.len(), value.len());

		let vtx_key = keys::branch_vtx_key(input.database_branch_id, commit.versionstamp);
		row_stats.vtx.scan(1);
		if let Some(vtx_value) = tx_get_value(tx, &vtx_key, Serializable).await? {
			if vtx_value == txid.to_be_bytes() {
				udb::compare_and_clear(tx, &vtx_key, &vtx_value);
				key_count = key_count.saturating_add(1);
				byte_count =
					byte_count.saturating_add(u64::try_from(vtx_value.len()).unwrap_or(u64::MAX));
				row_stats.vtx.clear(vtx_key.len(), vtx_value.len());
			} else {
				return Ok(rejected_sweep_chunk(
					input,
					"VTX row changed for reclaim txid",
				));
			}
		}
	}
	// Folded deltas whose pages are materialized in shards. DELTA is billable, so credit the freed
	// bytes back to quota.
	for (key, value) in &window.delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(input.database_branch_id, key)?;
		// Reclaim is classified per segment, so a chunk is only cleared when its own segment was
		// classified: a sibling segment of the same commit may still be retained.
		let first_pgno =
			match keys::decode_branch_delta_chunk_ref(input.database_branch_id, txid, key)? {
				keys::DeltaChunkRef::Legacy { .. } => None,
				keys::DeltaChunkRef::Segment { first_pgno, .. } => Some(first_pgno),
			};
		if !delta_reclaim_segments.contains(&DeltaSegmentRef { txid, first_pgno }) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		let freed = key.len().saturating_add(value.len());
		quota::atomic_add_branch(
			tx,
			input.database_branch_id,
			i64::try_from(freed).unwrap_or(i64::MAX).saturating_neg(),
		);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.delta.clear(key.len(), value.len());
	}
	// Expired `PITR_INTERVAL` rows ride along: this chunk already read them to build the retention set
	// the classification above depends on, so clearing them here costs no extra read.
	for (_, key, value, _) in &expired_pitr_interval_rows {
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.pitr_interval.clear(key.len(), value.len());
	}

	Ok(SweepCommitDeltaChunkOutput {
		status: CompactionJobStatus::Succeeded,
		throttled: false,
		next_commit_scan_cursor: window.next_commit_scan_cursor,
		next_segment_pgno: window.next_segment_pgno,
		commit_scan_complete: window.commit_scan_complete,
		key_count,
		byte_count,
		row_stats,
		cursor_advance_txids: window
			.next_commit_scan_cursor
			.saturating_sub(input.commit_scan_cursor),
		admission_blocked: false,
	})
}

fn rejected_sweep_chunk(
	input: &SweepCommitDeltaChunkInput,
	reason: impl Into<String>,
) -> SweepCommitDeltaChunkOutput {
	SweepCommitDeltaChunkOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		throttled: false,
		next_commit_scan_cursor: input.commit_scan_cursor,
		next_segment_pgno: input.cursor_segment_pgno,
		commit_scan_complete: false,
		key_count: 0,
		byte_count: 0,
		row_stats: ReclaimRowStats::default(),
		cursor_advance_txids: 0,
		admission_blocked: false,
	}
}

/// A de-admitted window read and cleared nothing, so the cursor it hands back is the one it was
/// given. The drain ends the job on this rather than parking on it.
fn admission_blocked_sweep_chunk(
	input: &SweepCommitDeltaChunkInput,
) -> SweepCommitDeltaChunkOutput {
	SweepCommitDeltaChunkOutput {
		status: CompactionJobStatus::Requested,
		throttled: false,
		next_commit_scan_cursor: input.commit_scan_cursor,
		next_segment_pgno: input.cursor_segment_pgno,
		commit_scan_complete: false,
		key_count: 0,
		byte_count: 0,
		row_stats: ReclaimRowStats::default(),
		cursor_advance_txids: 0,
		admission_blocked: true,
	}
}

fn throttled_sweep_chunk(input: &SweepCommitDeltaChunkInput) -> SweepCommitDeltaChunkOutput {
	SweepCommitDeltaChunkOutput {
		status: CompactionJobStatus::Requested,
		throttled: true,
		next_commit_scan_cursor: input.commit_scan_cursor,
		next_segment_pgno: input.cursor_segment_pgno,
		commit_scan_complete: false,
		key_count: 0,
		byte_count: 0,
		row_stats: ReclaimRowStats::default(),
		cursor_advance_txids: 0,
		admission_blocked: false,
	}
}

pub(crate) async fn plan_reclaim_slice_tx(
	tx: &universaldb::Transaction,
	input: &PlanReclaimSliceInput,
	job_id: Id,
	now_ms: i64,
	throttle_class: ThrottleClass,
) -> Result<PlanReclaimSliceOutput> {
	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for reclaim planning")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(plan_reclaim_slice_aborted());
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for reclaim planning")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(plan_reclaim_slice_aborted());
	}

	// Back off before the scan if the cluster-wide compaction read budget for this window is spent.
	// Checked after the cheap metadata reads have confirmed a live branch at the base generation, so a
	// throttled pass touches none of the COMMITS/DELTA history it would otherwise walk. Reclaim charges
	// every row it scans, not just the rows it plans to delete, so a pass over mostly-retained history
	// costs real read volume while planning nothing. Neither cursor advances, so the drain replans this
	// same window after backing off.
	let read_decision = tx.check_throttle(
		DEPOT_COMPACTION_THROTTLE,
		ThrottleKind::Read,
		throttle_class,
	)?;
	if !read_decision.allowed {
		return Ok(plan_reclaim_slice_throttled(input));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	let bucket_proof_blocked_reclaim =
		resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await?;
	let shard_cache_policy =
		read_effective_shard_cache_policy_for_branch(tx, branch_record.as_ref()).await?;
	// One budget bounds the whole slice: every lane's delete rides in a single FDB transaction, so
	// the plan and the delete consume the same budget in the same order to stay under the FDB
	// transaction size limit while deriving identical sets.
	let mut budget = CompactionBatchBudget::fdb();
	let reclaim_inputs = read_reclaim_input_snapshot(
		tx,
		input.database_branch_id,
		&root,
		&db_pins,
		branch_record.as_ref(),
		shard_cache_policy,
		input.cold_scan_cursor,
		input.commit_scan_cursor,
		input.cursor_segment_pgno,
		Serializable,
		now_ms,
		&mut budget,
		// The plan's window is what the v1 delete re-derives, so it has to come out of the deterministic
		// batch budget alone. A wall-clock bound here would move the boundary from one pass to the
		// next and the delete could never reproduce it.
		None,
		!input.skip_commit_delta,
	)
	.await?;
	let next_cold_scan_cursor = reclaim_inputs.next_cold_scan_cursor;
	let next_commit_scan_cursor = reclaim_inputs.next_commit_scan_cursor;
	let next_segment_pgno = reclaim_inputs.next_segment_pgno;
	let commit_scan_complete = reclaim_inputs.commit_scan_complete;
	// The dead-shard version sweep is a separate `SweepDeadShardVersions` activity, not a lane of the
	// replan loop, so this planner never triggers on it. `dead_shard_sweep_needed` stays false here.

	let manifest_generation = root.manifest_generation;
	let snapshot = ManagerFdbSnapshot {
		branch_record,
		head: None,
		root,
		dirty: None,
		db_pins,
		hot_inputs: HotInputSnapshot::default(),
		reclaim_inputs,
		bucket_proof_blocked_reclaim,
		cleared_dirty: false,
	};
	let planned = plan_reclaim_job(
		input.database_branch_id,
		&snapshot,
		job_id,
		now_ms,
		input.skip_commit_delta,
	)
	.map(|planned| PlannedReclaimSlice {
		input_range: planned.input_range,
		input_fingerprint: planned.input_fingerprint,
	});

	// Diagnostic only. The branch is known live and generation-matched here, so this cannot
	// resurrect a key in a swept branch's subspace.
	tx.informal().set(
		&keys::branch_compaction_reclaim_progress_key(input.database_branch_id),
		&encode_reclaim_progress(ReclaimProgress {
			schema_version: 1,
			manifest_generation,
			commit_scan_cursor: next_commit_scan_cursor,
			commit_scan_complete,
			cold_scan_cursor: next_cold_scan_cursor,
			last_outcome: if planned.is_some() {
				ReclaimPlanOutcome::Planned
			} else {
				ReclaimPlanOutcome::NothingReclaimable
			},
			updated_at_ms: now_ms,
		})
		.context("encode sqlite reclaim progress for reclaim planning")?,
	);

	Ok(PlanReclaimSliceOutput {
		planned,
		next_cold_scan_cursor,
		next_commit_scan_cursor,
		next_segment_pgno,
		commit_scan_complete,
		throttled: false,
	})
}

#[activity(ReclaimFdbJob)]
#[max_retries = 256]
pub async fn reclaim_fdb_job(
	ctx: &ActivityCtx,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	// Re-check the admission percent every call so an operator lowering it reaches jobs already in
	// flight, on the same terms as `sweep_commit_delta_chunk`. This gate covers the staging cleanup
	// too, which is the only reclaim work that reaches this activity without passing through the
	// drain's gate first, and which is therefore the only thing that kept writing at zero percent.
	if !input.bypass_admission && !branch_admitted_now(ctx.config(), input.database_branch_id) {
		return Ok(admission_blocked_reclaim_job());
	}

	let input = input.clone();
	let input_for_tx = input.clone();
	let now_ms = ctx.ts();
	// Resolved once for the whole activity rather than inside the transaction closure, so a config
	// change mid-flight cannot give two attempts of the same transaction different budgets.
	let throttle_class = throttle::CompactionThrottleClass::Reclaim.resolve(ctx.config());

	// Bounds the re-derive scan for the whole call. Captured before the transaction so the deadline is
	// the same one every attempt measures against.
	let scan_deadline = Instant::now() + crate::CMP_RECLAIM_EARLY_TXN_TIMEOUT;
	let start = Instant::now();
	let result = ctx
		.udb()?
		.txn("depot_reclaim_fdb", move |tx| {
			let input = input_for_tx.clone();
			async move {
				tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Both)?;
				tx.priority(Priority::Low)?;
				reclaim_fdb_job_tx(&tx, &input, now_ms, scan_deadline, throttle_class).await
			}
		})
		.await;
	metrics::record_reclaim_fdb(start, &result);

	let output = result?;
	if output.throttled {
		metrics::record_compaction_throttled(metrics::COMPACTION_KIND_RECLAIM);
	}
	record_shard_cache_eviction_metrics(&input, &output);
	Ok(output)
}

fn record_shard_cache_eviction_metrics(input: &ReclaimFdbJobInput, output: &ReclaimFdbJobOutput) {
	if output.status != CompactionJobStatus::Succeeded
		|| input.input_range.shard_cache_evictions.is_empty()
	{
		return;
	}

	let evicted_bytes = input
		.input_range
		.shard_cache_evictions
		.iter()
		.map(|eviction| eviction.size_bytes)
		.fold(0_u64, u64::saturating_add);
	metrics::SQLITE_SHARD_CACHE_EVICTION_TOTAL
		.with_label_values(&[metrics::SHARD_CACHE_EVICTION_CLEARED])
		.inc_by(input.input_range.shard_cache_evictions.len() as u64);
	if evicted_bytes > 0 {
		let evicted_bytes = i64::try_from(evicted_bytes).unwrap_or(i64::MAX);
		let resident_bytes = metrics::SQLITE_SHARD_CACHE_RESIDENT_BYTES.get();
		metrics::SQLITE_SHARD_CACHE_RESIDENT_BYTES
			.set(resident_bytes.saturating_sub(evicted_bytes).max(0));
	}
}

/// Charges the read axis for what a reclaim pass pulled out of FDB, in a transaction of its own.
///
/// Deliberately not the pass's own transaction. The pass owes the throttle for every read it issued,
/// including the ones an attempt made before aborting, and a charge riding on an aborted transaction
/// is discarded along with it. This charge is a single conflict-free atomic add, so it commits under
/// the contention the pass itself could not survive.
///
/// A failure here is logged rather than propagated: the reads are already spent, and failing the

async fn reclaim_fdb_job_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
	now_ms: i64,
	scan_deadline: Instant,
	throttle_class: ThrottleClass,
) -> Result<ReclaimFdbJobOutput> {
	if input.job_kind != CompactionJobKind::Reclaim {
		return Ok(rejected_reclaim_job("reclaimer received a non-reclaim job"));
	}
	// Back off before doing any deletes if the cluster-wide compaction write budget for this window
	// is spent. Reclaim deletes use `COMPARE_AND_CLEAR`, which carries the cleared value, so they are
	// real FDB write volume. `now_ms` is the activity's real wall-clock time, correct for the window.
	let decision = tx.check_throttle(
		DEPOT_COMPACTION_THROTTLE,
		ThrottleKind::Write,
		throttle_class,
	)?;
	if !decision.allowed {
		return Ok(throttled_reclaim_job());
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = reclaim_fdb_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::PlanBeforeSnapshot,
	)
	.await?
	{
		return Ok(output);
	}
	if input.input_range.delta_reclaim_segments.is_empty()
		&& input.input_range.commit_reclaim_txids.is_empty()
		&& input.input_range.cold_objects.is_empty()
		&& input.input_range.shard_cache_evictions.is_empty()
		&& (!input.input_range.stale_hot_job_ids.is_empty()
			|| !input.input_range.stale_cold_job_ids.is_empty()
			|| !input.input_range.stale_commit_stage_txids.is_empty())
	{
		return cleanup_repair_fdb_outputs_tx(tx, input, util::timestamp::now()).await;
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for FDB reclaim")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_reclaim_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for FDB reclaim")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_reclaim_job("base manifest generation changed"));
	}

	// Back off before re-deriving the slice if the cluster-wide compaction read budget for this window
	// is spent. The delete side re-runs the planner's whole scan to rebuild the candidate set under OCC,
	// so it costs the same read volume the plan pass did. Checked after the cheap metadata guards and
	// before any of that scan runs, so a throttled call reads none of the history.
	let read_decision = tx.check_throttle(
		DEPOT_COMPACTION_THROTTLE,
		ThrottleKind::Read,
		throttle_class,
	)?;
	if !read_decision.allowed {
		return Ok(throttled_reclaim_job());
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_reclaim_job("bucket fork proof is ambiguous"));
	}
	let snapshot = read_reclaim_input_snapshot(
		tx,
		input.database_branch_id,
		&root,
		&db_pins,
		branch_record.as_ref(),
		read_effective_shard_cache_policy_for_branch(tx, branch_record.as_ref()).await?,
		// Re-derive the cold-object reclaim set from the exact window this slice was planned against, so
		// the comparison below fences a racing cold-ref change via OCC.
		input.input_range.cold_scan_cursor,
		// Same for the commit window: re-deriving from a different cursor would classify a different
		// set of txids and reject every slice past the first. The segment cursor is part of that
		// window, so a slice that started mid-commit re-derives from the same segment.
		input.input_range.commit_scan_cursor,
		input.input_range.cursor_segment_pgno,
		Serializable,
		now_ms,
		// A fresh budget matching the plan's makes the budget-capped lanes re-derive the same
		// deterministic prefixes; the dead-shard lane revalidates the planned refs instead of
		// re-walking, so nothing after the snapshot consumes it.
		&mut CompactionBatchBudget::fdb(),
		Some(scan_deadline),
		// A v2 slice's commit/delta lane belongs to the sweep. Deriving it here would rebuild a set the
		// plan deliberately left empty and reject on the mismatch.
		!input.input_range.skip_commit_delta,
	)
	.await?;
	// A truncated scan derives a strictly smaller set than the plan did, so every comparison below
	// would report a change that never happened and reject the slice. Hand the pass back instead: the
	// companion re-dispatches the same input, and what this attempt read is charged either way.
	if snapshot.scan_truncated {
		return Ok(incomplete_reclaim_job());
	}
	// Re-derive the classification under Serializable and reject on any change. A racing pin or
	// commit touches `DB_PIN`/`PIDX`/`COMMITS`, which the snapshot reads above conflict on, so it both
	// aborts via OCC and shifts the coverage-derived classification here.
	// A job planned before reclaim became per-segment carries no segments, so it fails here and is
	// replanned rather than acted on with a set this code can no longer interpret.
	if snapshot.delta_reclaim_segments != input.input_range.delta_reclaim_segments {
		return Ok(rejected_reclaim_job("folded delta reclaim set changed"));
	}
	if snapshot.commit_reclaim_txids != input.input_range.commit_reclaim_txids {
		return Ok(rejected_reclaim_job("commit reclaim set changed"));
	}
	if snapshot.cold_object_refs != input.input_range.cold_objects {
		return Ok(rejected_reclaim_job("cold object reclaim set changed"));
	}
	if snapshot
		.shard_cache_evictions
		.iter()
		.map(|candidate| candidate.reference.clone())
		.collect::<Vec<_>>()
		!= input.input_range.shard_cache_evictions
	{
		return Ok(rejected_reclaim_job("shard cache eviction set changed"));
	}

	let input_fingerprint = fingerprint_reclaim_inputs_scoped(
		input.database_branch_id,
		&root,
		&snapshot,
		!input.input_range.skip_commit_delta,
	);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job("reclaim input fingerprint changed"));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = reclaim_fdb_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::PlanAfterSnapshot,
	)
	.await?
	{
		return Ok(output);
	}

	let delta_reclaim_segments = input
		.input_range
		.delta_reclaim_segments
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	let commit_reclaim_txids = input
		.input_range
		.commit_reclaim_txids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	let mut row_stats = ReclaimRowStats::default();
	// Every row the snapshot read is charged against the read budget whether or not it is
	// reclaimable, so scanned is recorded before any classification.
	row_stats.commit.scan(snapshot.commits.len());
	row_stats.delta.scan(snapshot.delta_chunks.len());
	// Both halves of the interval read are scanned rows: retained rows are read in full to build the
	// coverage set the classification depends on, and only the expired half is a delete candidate.
	row_stats
		.pitr_interval
		.scan(snapshot.pitr_interval_retention.len() + snapshot.expired_pitr_interval_rows.len());
	#[cfg(feature = "test-faults")]
	if let Some(output) =
		reclaim_fdb_fault_output(input.database_branch_id, ReclaimFaultPoint::BeforeHotDelete)
			.await?
	{
		return Ok(output);
	}
	// COMMITS/VTX for non-fold txids below the cold-watermark-capped bound. These are not billable
	// keys, so no quota credit is taken (#10).
	for (txid, key, value, commit) in &snapshot.commits {
		if !commit_reclaim_txids.contains(txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.commit.clear(key.len(), value.len());

		let vtx_key = keys::branch_vtx_key(input.database_branch_id, commit.versionstamp);
		row_stats.vtx.scan(1);
		if let Some(vtx_value) = tx_get_value(tx, &vtx_key, Serializable).await? {
			if vtx_value == txid.to_be_bytes() {
				udb::compare_and_clear(tx, &vtx_key, &vtx_value);
				key_count = key_count.saturating_add(1);
				byte_count =
					byte_count.saturating_add(u64::try_from(vtx_value.len()).unwrap_or(u64::MAX));
				row_stats.vtx.clear(vtx_key.len(), vtx_value.len());
			} else {
				return Ok(rejected_reclaim_job("VTX row changed for reclaim txid"));
			}
		}
	}
	// Folded deltas whose pages are materialized in shards (C6). DELTA is a billable key, so credit the
	// freed bytes back to quota (#10).
	for (key, value) in &snapshot.delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(input.database_branch_id, key)?;
		// Classified per segment, so a chunk is only cleared when its own segment was classified: a
		// sibling segment of the same commit may still be retained.
		let first_pgno =
			match keys::decode_branch_delta_chunk_ref(input.database_branch_id, txid, key)? {
				keys::DeltaChunkRef::Legacy { .. } => None,
				keys::DeltaChunkRef::Segment { first_pgno, .. } => Some(first_pgno),
			};
		if !delta_reclaim_segments.contains(&DeltaSegmentRef { txid, first_pgno }) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		let freed = key.len().saturating_add(value.len());
		quota::atomic_add_branch(
			tx,
			input.database_branch_id,
			i64::try_from(freed).unwrap_or(i64::MAX).saturating_neg(),
		);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.delta.clear(key.len(), value.len());
	}
	for (_, key, value, _) in &snapshot.expired_pitr_interval_rows {
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
		row_stats.pitr_interval.clear(key.len(), value.len());
	}
	for candidate in &snapshot.shard_cache_evictions {
		if !input
			.input_range
			.shard_cache_evictions
			.contains(&candidate.reference)
		{
			continue;
		}
		row_stats.shard_evict.scan(candidate.shard_rows.len());
		for (key, value) in &candidate.shard_rows {
			udb::compare_and_clear(tx, key, value);
			key_count = key_count.saturating_add(1);
			byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
			row_stats.shard_evict.clear(key.len(), value.len());
		}
	}
	// The dead-shard version-retention sweep (C4) runs in its own `SweepDeadShardVersions` activity, not
	// as a lane here.
	// Best-effort cleanup of expired `SHARD_LRU` recency rows (stale duplicates plus the rows of the
	// idle shards we just demoted). These are an index, not billable data, so they are not fenced
	// against the planned eviction set; compare_and_clear no-ops if a row was already cleared.
	for lru_key in &snapshot.shard_lru_cleanup_keys {
		udb::compare_and_clear(tx, lru_key, &[]);
		key_count = key_count.saturating_add(1);
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) =
		reclaim_fdb_fault_output(input.database_branch_id, ReclaimFaultPoint::AfterHotDelete)
			.await?
	{
		return Ok(output);
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
			row_stats,
		}],
		throttled: false,
		// Every other lane derives its candidate set under the shared slice budget, so a slice never
		// leaves a partially handled candidate behind; the reclaim companion replans instead.
		has_more: false,
		admission_blocked: false,
	})
}

/// Standalone dead-shard version-retention sweep (C4). Walks the entire `CMP/fold` index in bounded
/// FDB transactions inside this one activity, holding the cross-chunk `prev` supersession context in
/// local memory instead of persisting it through workflow state, and deletes dead versions as it
/// goes. A single forward walk detects every current supersession, so the activity completes the
/// whole sweep in one pass unless it crosses `CMP_BULK_ACTIVITY_EARLY_TIMEOUT`, in which case it
/// returns `Requested` and the companion re-dispatches. Already-deleted versions are gone from the
/// fold index, so a re-dispatched walk from the start simply continues.
#[activity(SweepDeadShardVersions)]
#[timeout = crate::CMP_BULK_ACTIVITY_TIMEOUT_SECS]
pub async fn sweep_dead_shard_versions(
	ctx: &ActivityCtx,
	input: &SweepDeadShardVersionsInput,
) -> Result<SweepDeadShardVersionsOutput> {
	let start = Instant::now();
	// Accumulated outside the walk so a dispatch that errors partway still reports what its committed
	// chunks freed.
	let mut row_volume = ReclaimRowVolume::default();
	let result = sweep_dead_shard_versions_inner(ctx, input, &mut row_volume)
		.await
		.map(|status| SweepDeadShardVersionsOutput { status, row_volume });
	metrics::record_reclaim_dead_shard_sweep(start, &result, &row_volume);
	result
}

/// Walks the fold index and deletes the dead versions it finds, returning the dispatch status and
/// accumulating the freed volume into `row_volume` as each chunk commits.
async fn sweep_dead_shard_versions_inner(
	ctx: &ActivityCtx,
	input: &SweepDeadShardVersionsInput,
	row_volume: &mut ReclaimRowVolume,
) -> Result<CompactionJobStatus> {
	let early_timeout = test_hooks::bulk_activity_early_timeout(input.database_branch_id);
	let start = Instant::now();
	// Cross-chunk supersession context, held in local memory across this activity's internal FDB
	// transactions. Never serialized into workflow state.
	//
	// This restarts at the beginning of the fold index on every dispatch, and that is deliberate.
	// `SweepDeadShardVersionsInput` carries no resume cursor on purpose: see the note on
	// `DeadShardScanState` for why one cannot be added without changing how a dead version is
	// identified. Progress across dispatches comes from deletions shrinking the fold index, not from
	// a cursor. Re-walking is cheap because a covered fold is skipped on its index row alone, before
	// any shard blob is read.
	let mut scan = DeadShardScanState::default();
	loop {
		// Real wall-clock time, re-read per chunk so a sweep that spans several windows resolves its
		// retention boundary against the live one. Capturing outside the transaction closure keeps the
		// value stable across that chunk's FDB txn retries.
		let throttle_now_ms = util::timestamp::now();
		let input = input.clone();
		let scan_in = scan.clone();
		let outcome = ctx
			.udb()?
			.txn("depot_reclaim_sweep_dead_shard_chunk", move |tx| {
				let input = input.clone();
				let scan_in = scan_in.clone();
				async move {
					tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Both)?;
					tx.priority(Priority::Low)?;
					sweep_dead_shard_chunk_tx(&tx, &input, &scan_in, throttle_now_ms).await
				}
			})
			.await?;
		match outcome {
			DeadShardSweepOutcome::Terminal(status) => {
				return Ok(status);
			}
			DeadShardSweepOutcome::Continue {
				next_scan,
				has_more,
				row_volume: chunk_volume,
			} => {
				scan = next_scan;
				row_volume.merge(&chunk_volume);
				if !has_more {
					return Ok(CompactionJobStatus::Succeeded);
				}
				// The chunk committed, so a re-dispatch loses no work. The companion re-dispatches with a
				// fresh timeout budget; the re-walk from the start skips the versions this call deleted.
				if start.elapsed() > early_timeout {
					return Ok(CompactionJobStatus::Requested);
				}
			}
		}
	}
}

/// Walks a branch's whole `PIDX` prefix once, clearing rows stranded by hot slices that folded a page
/// without clearing its PIDX row. Those rows pin their delta and commit against reclaim permanently,
/// because the owner-window filter in `read_hot_input_snapshot` never revisits an owner below the
/// watermark, so nothing else in the system can ever free them.
///
/// The walk runs across bounded FDB transactions inside this activity, holding only a page cursor in
/// local memory and clearing each window's rows in the transaction that found them, so no plan/delete
/// OCC fence is needed. On an early timeout it returns the cursor and the companion re-dispatches from
/// it; restarting from the beginning instead would re-read every live row before reaching the next
/// stale one. Completing the walk writes the repair marker, which retires the sweep for the branch.
#[activity(SweepStalePidx)]
#[timeout = crate::CMP_BULK_ACTIVITY_TIMEOUT_SECS]
pub async fn sweep_stale_pidx(
	ctx: &ActivityCtx,
	input: &SweepStalePidxInput,
) -> Result<SweepStalePidxOutput> {
	let early_timeout = test_hooks::bulk_activity_early_timeout(input.database_branch_id);
	let start = Instant::now();
	let mut pgno_cursor = input.pgno_cursor;
	let mut cleared_count = 0_u64;
	let mut retained_unconfirmed = input.retained_unconfirmed;
	loop {
		let mut input = input.clone();
		input.retained_unconfirmed = retained_unconfirmed;
		let outcome = ctx
			.udb()?
			.txn("depot_reclaim_sweep_stale_pidx_chunk", move |tx| {
				let input = input.clone();
				async move {
					tx.charge_throttle(DEPOT_COMPACTION_THROTTLE, ThrottleCharge::Both)?;
					tx.priority(Priority::Low)?;
					sweep_stale_pidx_chunk_tx(&tx, &input, pgno_cursor).await
				}
			})
			.await?;
		match outcome {
			StalePidxSweepOutcome::Terminal(status) => {
				return Ok(SweepStalePidxOutput {
					status,
					next_pgno_cursor: pgno_cursor,
					cleared_count,
					retained_unconfirmed,
				});
			}
			StalePidxSweepOutcome::Continue {
				next_pgno_cursor,
				has_more,
				cleared,
				retained_unconfirmed: window_retained,
			} => {
				pgno_cursor = next_pgno_cursor;
				cleared_count = cleared_count.saturating_add(cleared);
				retained_unconfirmed = window_retained;
				if !has_more {
					return Ok(SweepStalePidxOutput {
						status: CompactionJobStatus::Succeeded,
						next_pgno_cursor: pgno_cursor,
						cleared_count,
						retained_unconfirmed,
					});
				}
				// The chunk committed, so a re-dispatch loses no work: it resumes at this cursor.
				if start.elapsed() > early_timeout {
					return Ok(SweepStalePidxOutput {
						status: CompactionJobStatus::Requested,
						next_pgno_cursor: pgno_cursor,
						cleared_count,
						retained_unconfirmed,
					});
				}
			}
		}
	}
}

pub(super) enum StalePidxSweepOutcome {
	/// One PIDX window walked and its stale rows cleared; `has_more` is true while rows remain.
	Continue {
		next_pgno_cursor: Option<u32>,
		has_more: bool,
		cleared: u64,
		/// Whether the walk, including this window, has retained a row it could not confirm.
		retained_unconfirmed: bool,
	},
	/// The branch lifecycle or manifest generation changed, or the branch is already repaired; return
	/// this status unchanged.
	Terminal(CompactionJobStatus),
}

pub(super) async fn sweep_stale_pidx_chunk_tx(
	tx: &universaldb::Transaction,
	input: &SweepStalePidxInput,
	pgno_cursor: Option<u32>,
) -> Result<StalePidxSweepOutcome> {
	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for stale pidx sweep")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(StalePidxSweepOutcome::Terminal(
			rejected_reclaim_job("database branch lifecycle changed").status,
		));
	}

	let root = read_compaction_root_or_default(tx, input.database_branch_id).await?;
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(StalePidxSweepOutcome::Terminal(
			rejected_reclaim_job("base manifest generation changed").status,
		));
	}

	// A branch that already completed the walk is done for good: the prevention fix in hot planning
	// means no new stale rows appear, and re-walking would re-read every live PIDX row of the branch on
	// every reclaim job. This point read is the whole steady-state cost of the sweep.
	if tx_get_value(
		tx,
		&keys::branch_compaction_pidx_repair_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.is_some()
	{
		return Ok(StalePidxSweepOutcome::Terminal(
			CompactionJobStatus::Succeeded,
		));
	}

	// A branch that has never installed a fold cannot hold a stale row: staleness means a fold left a
	// page PIDX-owned, and the watermark only ever advances to a folded txid. Every row would classify
	// live here, so the walk can only scan the whole prefix for nothing and then retire the branch on
	// a pass that inspected no foldable row. Leave the marker unwritten so the sweep still gets its
	// one real walk once folding starts.
	if root.hot_watermark_txid == 0 {
		return Ok(StalePidxSweepOutcome::Terminal(
			CompactionJobStatus::Succeeded,
		));
	}

	// One budget bounds this window's PIDX reads, the shard images it reads to confirm page coverage,
	// and the clears it drives, so the walk-and-clear stays under the FDB transaction size limit.
	let mut budget = CompactionBatchBudget::fdb();
	let chunk = read_stale_pidx_chunk(
		tx,
		input.database_branch_id,
		&root,
		pgno_cursor,
		Serializable,
		&mut budget,
	)
	.await?;
	// The walk is not gated on the read budget: it runs in bounded windows behind a persisted cursor
	// and clears as it goes, so charging (in the wrapper, which measures the whole transaction) is
	// enough to make other background work yield to it without adding a stall path mid-repair.

	let mut byte_count = 0_u64;
	for (key, value) in &chunk.candidates {
		udb::compare_and_clear(tx, key, value);
		// PIDX is a billable key, so credit the freed bytes back to quota.
		let freed = key.len().saturating_add(value.len());
		quota::atomic_add_branch(
			tx,
			input.database_branch_id,
			i64::try_from(freed).unwrap_or(i64::MAX).saturating_neg(),
		);
		byte_count = byte_count.saturating_add(u64::try_from(freed).unwrap_or(u64::MAX));
	}

	// Mark the branch repaired in the same transaction that finished the walk, so the marker can never
	// claim a walk that did not commit. A walk that retained a row it could not confirm has not
	// repaired the branch: the marker is one-shot, so writing it there retires the sweep with those
	// rows still owning their pages, and nothing revisits them or the deltas they pin. Leave it
	// unwritten so a later job walks again once the missing shard coverage exists.
	let retained_unconfirmed = input.retained_unconfirmed || chunk.retained_unconfirmed;
	if !chunk.has_more {
		if retained_unconfirmed {
			tracing::warn!(
				database_branch_id = ?input.database_branch_id,
				hot_watermark_txid = root.hot_watermark_txid,
				"stale pidx walk finished with rows it could not confirm; leaving the branch unrepaired"
			);
		} else {
			tx.informal().set(
				&keys::branch_compaction_pidx_repair_key(input.database_branch_id),
				&root.hot_watermark_txid.to_be_bytes(),
			);
		}
	}

	Ok(StalePidxSweepOutcome::Continue {
		next_pgno_cursor: chunk.next_pgno_cursor,
		has_more: chunk.has_more,
		cleared: u64::try_from(chunk.candidates.len()).unwrap_or(u64::MAX),
		retained_unconfirmed,
	})
}

enum DeadShardSweepOutcome {
	/// One fold chunk walked and its dead versions deleted; `has_more` is true while folds remain.
	Continue {
		next_scan: DeadShardScanState,
		has_more: bool,
		/// Rows this chunk freed. Returned from the transaction rather than counted into a metric
		/// inside it, so a retried attempt does not count its discarded work.
		row_volume: ReclaimRowVolume,
	},
	/// The branch lifecycle or manifest generation changed; return this status unchanged.
	Terminal(CompactionJobStatus),
}

async fn sweep_dead_shard_chunk_tx(
	tx: &universaldb::Transaction,
	input: &SweepDeadShardVersionsInput,
	scan: &DeadShardScanState,
	throttle_now_ms: i64,
) -> Result<DeadShardSweepOutcome> {
	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for dead shard sweep")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(DeadShardSweepOutcome::Terminal(
			rejected_reclaim_job("database branch lifecycle changed").status,
		));
	}

	let root = read_compaction_root_or_default(tx, input.database_branch_id).await?;
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(DeadShardSweepOutcome::Terminal(
			rejected_reclaim_job("base manifest generation changed").status,
		));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(DeadShardSweepOutcome::Terminal(
			rejected_reclaim_job("bucket fork proof is ambiguous").status,
		));
	}

	// Coverage inputs for the walk: unexpired PITR interval reps plus pins and head. A fresh budget
	// is used only to read the retention set; the expired-interval reclaim rows are handled by the
	// separate reclaim lane, so they are discarded here.
	let (pitr_interval_retention, _expired) = read_pitr_interval_reclaim_rows(
		tx,
		input.database_branch_id,
		throttle_now_ms,
		Serializable,
		&mut CompactionBatchBudget::fdb(),
	)
	.await?;

	// One budget bounds this chunk's fold reads and candidate deletes so the walk-and-delete stays
	// under the FDB transaction size limit.
	let mut budget = CompactionBatchBudget::fdb();
	let chunk = read_dead_shard_versions_chunk(
		tx,
		input.database_branch_id,
		&db_pins,
		&pitr_interval_retention,
		scan,
		Serializable,
		&mut budget,
	)
	.await?;
	// Like the stale-PIDX sweep this walk is not gated on the read budget: it deletes as it goes in
	// bounded chunks, so charging the transaction is enough to make other background work yield to it.
	// Delete this chunk's dead versions in the same transaction that found them, so no plan/delete OCC
	// fence is needed; the Serializable reads conflict on any racing pin/fold/commit change.
	let row_volume =
		delete_dead_shard_versions_tx(tx, input.database_branch_id, &chunk.candidates).await?;

	Ok(DeadShardSweepOutcome::Continue {
		next_scan: chunk.next_scan,
		has_more: chunk.has_more,
		row_volume,
	})
}

pub(super) async fn cleanup_repair_fdb_outputs_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
	now_ms: i64,
) -> Result<ReclaimFdbJobOutput> {
	let input_fingerprint =
		fingerprint_repair_reclaim_range(input.database_branch_id, &input.input_range);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job(
			"repair cleanup input fingerprint changed",
		));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for repair cleanup")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_reclaim_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Snapshot,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for repair cleanup")?;
	let manifest_generation = root
		.as_ref()
		.map(|root| root.manifest_generation)
		.unwrap_or(input.base_manifest_generation);

	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	let mut row_stats = ReclaimRowStats::default();
	// One budget bounds every read and clear this transaction performs. A whole job's staging area is
	// as large as the shard images the drain staged, which does not fit one FDB transaction, so the
	// cleanup drains across several bounded calls: each clears the ref row of every shard it handled,
	// so re-running the same input rescans only what is left.
	let mut budget = CompactionBatchBudget::fdb();
	let mut has_more = false;
	let mut cleared_any = false;
	// Re-derive the orphan staged shards from the FDB staging area under each stale job id instead of
	// receiving the ref list in the signal. Each ref row identifies a staged shard blob; validate the
	// blob against the ref, then clear both the blob chunks and the ref row.
	// Abandoned staged commits. Cheap next to the staged-shard cleanup below: one range clear, one
	// quota refund and one row per orphan, with no blobs to validate, so they are handled first and
	// do not need the batch budget's page-at-a-time treatment.
	for txid in &input.input_range.stale_commit_stage_txids {
		let stage_key = keys::branch_commit_stage_key(input.database_branch_id, *txid);
		let Some(stage_bytes) = tx_get_value(tx, &stage_key, Serializable).await? else {
			continue;
		};
		let stage = crate::conveyer::types::decode_commit_stage_row(&stage_bytes)?;
		// Re-check against the live head inside the delete transaction. The scan that found this ran
		// against an earlier snapshot, and a stage that finalized in between is now live commit data
		// whose bytes must not be refunded.
		let head_txid = tx_get_value(
			tx,
			&keys::branch_meta_head_key(input.database_branch_id),
			Serializable,
		)
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()?
		.map_or(0, |head| head.head_txid);
		if *txid <= head_txid {
			continue;
		}
		// Re-check the grace window too, for the same reason. Minutes can pass between the scan and
		// this transaction (manager refresh, the cleanup queue, admission parking), and an actor
		// that came back in that time clears the orphan and opens a fresh stage at the same txid.
		// That stage is still above head, so the head check alone would let this clear a live
		// commit in progress and refund its bytes, failing the actor's next segment or finalize
		// with `StageNotFound`. The row's own timestamp is what distinguishes the two.
		if now_ms.saturating_sub(stage.started_at_ms) < COMMIT_STAGE_ORPHAN_GRACE_MS {
			continue;
		}

		clear_abandoned_commit_stage(tx, input.database_branch_id, *txid, &stage);
		cleared_any = true;
		tracing::info!(
			database_branch_id = ?input.database_branch_id,
			txid,
			refunded_bytes = stage.accounted_bytes,
			repair_action = "cleanup_orphan_commit_stage",
			"cleared an abandoned staged commit"
		);
	}
	'jobs: for job_id in &input.input_range.stale_hot_job_ids {
		let staged_refs = read_staged_hot_shard_refs_limited(
			tx,
			input.database_branch_id,
			*job_id,
			Serializable,
			&mut budget,
		)
		.await?;
		if staged_refs.has_more {
			has_more = true;
		}
		if staged_refs.refs.is_empty() && !staged_refs.has_more {
			// Blobs and their ref row are staged in one transaction, so a job with no ref rows left has
			// nothing readable under it: an earlier cleanup pass cleared the refs, or the job never
			// staged anything. Clear the whole subspace so any residue cannot outlive its refs and be
			// rediscovered by the staging orphan scan forever.
			let job_prefix =
				keys::branch_compaction_stage_job_prefix(input.database_branch_id, *job_id);
			let (begin, end) =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(job_prefix))
					.range();
			tx.informal().clear_range(&begin, &end);
			continue;
		}
		for staged in &staged_refs.refs {
			// Reserve against the ref's declared image size before reading anything, so a blob this
			// transaction has no room to clear is never materialized. The refs already handled stay
			// cleared, so the re-dispatch resumes here without a persisted cursor. Always clear at
			// least one shard per call: a shard image is far smaller than the batch budget, and
			// bailing on an untouched budget would re-dispatch forever without progress.
			//
			// A shard image is stored as `CHUNK_SIZE` chunk rows plus its ref row, so this is the
			// rows the clear would issue if the blob is there.
			let reserved_rows = usize::try_from(staged.size_bytes.div_ceil(CHUNK_SIZE as u64))
				.unwrap_or(usize::MAX)
				.saturating_add(1);
			if cleared_any && !budget.can_add(reserved_rows, staged.size_bytes) {
				has_more = true;
				break 'jobs;
			}
			cleared_any = true;

			// This lane clears a finished job's staging area. It never deletes rows from `SHARD`,
			// and that restraint is the safety property, not an omission.
			//
			// It runs on every finished job, successful ones included, so under direct-to-shard folds
			// the images a job's refs name may be live data some page already resolves through.
			// Nothing reachable from here distinguishes that from an abandoned job's scratch: the
			// images share `SHARD/{shard}/{as_of_txid}` with any other job that folded the same
			// coverage txid, the fold is deterministic so their bytes are identical, and `CMP/fold`
			// is rewritten by later installs and garbage-collected by cold publish. Every fence built
			// on those signals answers "unpublished" for data that is live, and deleting on it loses
			// pages silently.
			//
			// Nothing else deletes them either: `read_dead_shard_versions_chunk` seeds and advances
			// `prev` only from `CMP/fold` entries, and a job that never installed wrote none, so an
			// abandoned image's txid is never a candidate at any watermark.
			//
			// They are reclaimed by being overwritten instead. A drain's boundaries are reproducible
			// from the watermark (`plan_drain_head_txid`), so an abandoned job's successor folds the
			// same `(shard_id, as_of_txid)` keys and replaces its images. The exception is a forced
			// drain, which takes the live head by design and can therefore leave a boundary image no
			// later drain revisits.
			//
			// What is left is a leak, not a correctness bug: those images are complete and
			// read-consistent, they are simply never freed. Closing it is deliberately not this
			// lane's job, because every signal reachable from here is one of the unsound ones above.

			let stage_chunk_prefix = keys::branch_compaction_stage_hot_shard_txid_prefix(
				input.database_branch_id,
				*job_id,
				staged.shard_id,
				staged.as_of_txid,
			);
			let stage_rows = tx_scan_prefix_values(tx, &stage_chunk_prefix, Serializable).await?;
			// Charge what is actually here, not what the ref reserved. A direct fold left no blob
			// under this job, so the transaction clears one ref row; charging it a whole image would
			// spill the cleanup across many transactions to do almost nothing.
			if stage_rows.is_empty() {
				budget.add(1, 0);
			} else {
				budget.add(reserved_rows, staged.size_bytes);
			}
			if !stage_rows.is_empty() {
				let stage_value =
					shard_blob::assemble_chunked_rows(&stage_chunk_prefix, &stage_rows)?;
				if staged.size_bytes != u64::try_from(stage_value.len()).unwrap_or(u64::MAX)
					|| staged.content_hash != content_hash(&stage_value)
				{
					// Clear it anyway, loudly. Staging is scratch no reader ever consults and this
					// job is finished, so mismatched bytes have no recovery value, while retaining
					// them leaks the blob forever and leaves the ref row that makes the staging scan
					// re-report this job on every refresh. The clears below are
					// `compare_and_clear` against the values just read, so a row some writer changed
					// underneath is skipped rather than clobbered.
					tracing::error!(
						?input.database_branch_id,
						manifest_generation,
						?job_id,
						shard_id = staged.shard_id,
						as_of_txid = staged.as_of_txid,
						repair_action = "clear_mismatched_staged_hot_output",
						"staged hot shard cleanup found mismatched bytes"
					);
				}

				tracing::debug!(
					?input.database_branch_id,
					manifest_generation,
					?job_id,
					shard_id = staged.shard_id,
					as_of_txid = staged.as_of_txid,
					repair_action = "clear_staged_hot_output",
					"clearing orphan staged hot shard output"
				);
				row_stats.staging.scan(stage_rows.len());
				for (key, value) in &stage_rows {
					udb::compare_and_clear(tx, key, value);
					key_count = key_count.saturating_add(1);
					byte_count =
						byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
					row_stats.staging.clear(key.len(), value.len());
				}
				// Credit the ref's recorded image size rather than the chunk rows just cleared, so this
				// stays directly comparable with the image bytes the staging write path reports.
				row_stats.staging_blob_bytes_cleared = row_stats
					.staging_blob_bytes_cleared
					.saturating_add(staged.size_bytes);
			}

			// Clear the ref row itself so a retry does not re-scan a shard whose blob is already gone.
			let ref_key = keys::branch_compaction_stage_hot_ref_key(
				input.database_branch_id,
				*job_id,
				staged.min_txid,
				staged.shard_id,
				staged.as_of_txid,
			);
			tx.informal().clear(&ref_key);
			key_count = key_count.saturating_add(1);
			row_stats.staging.clear(ref_key.len(), 0);
		}
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
			row_stats,
		}],
		throttled: false,
		has_more,
		admission_blocked: false,
	})
}

fn rejected_reclaim_job(reason: impl Into<String>) -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
		throttled: false,
		has_more: false,
		admission_blocked: false,
	}
}

/// The branch fell outside the admission percent. Nothing was read or deleted, and `has_more` is not
/// set, so the companion ends the job instead of re-dispatching it. The staging cleanup ids the job
/// was carrying are re-derived from FDB by the next refresh's staging orphan scan, so ending here
/// defers the work rather than dropping it.
fn admission_blocked_reclaim_job() -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Requested,
		output_refs: Vec::new(),
		throttled: false,
		has_more: false,
		admission_blocked: true,
	}
}

fn throttled_reclaim_job() -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Requested,
		output_refs: Vec::new(),
		throttled: true,
		has_more: false,
		admission_blocked: false,
	}
}

/// The pass gave up on its elapsed bound before it could re-derive the slice. Nothing was deleted and
/// no cursor moved, so `has_more` sends the companion back around to the same input. It is not
/// `throttled`: no budget was spent, and the companion's throttle backoff would only delay a pass
/// that is already losing to how long the scan takes. What it read is charged by the activity, so a
/// branch that keeps landing here drives the read estimate up and starts being throttled on its own.
fn incomplete_reclaim_job() -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Requested,
		output_refs: Vec::new(),
		throttled: false,
		has_more: true,
		admission_blocked: false,
	}
}

#[cfg(feature = "test-faults")]
async fn reclaim_fdb_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
) -> Result<Option<ReclaimFdbJobOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(ReclaimFdbJobOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			output_refs: Vec::new(),
			throttled: false,
			has_more: false,
			admission_blocked: false,
		})),
	}
}
