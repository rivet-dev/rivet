use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use gas::prelude::Id;
use rivet_pools::NodeId;
use sha2::{Digest, Sha256};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

use super::{
	ActiveHotCompactionJob, ActiveReclaimCompactionJob, BranchStopState, CompactionJobKind,
	CompactionJobStatus, CompactionRoot, CompanionWorkflowIds, DatabaseBranchId,
	DatabaseBranchRecord, DbManagerInput, DbManagerState, DeadShardScanState, ForceCompaction,
	ForceCompactionTracker, ForceCompactionWork, HotInputSnapshot, HotInstallResume,
	HotInstallShardCursor, HotJobFinished, HotJobInputRange, ManagerActiveJobs, ManagerEffect,
	ManagerFdbSnapshot, ManagerStopReason, ManagerWakeIntervals, PendingCleanupQueue,
	PlannedHotCompactionJob, PlannedReclaimCompactionJob, ReclaimFdbJobInput, ReclaimInputSnapshot,
	ReclaimJobFinished, ReclaimJobInputRange, RefreshManagerOutput, ShardCachePolicy,
	StagedHotShardRef, StalePidxSweepOutcome, SweepStalePidxInput, TxidRange, WakeTriggers,
	cleanup_repair_fdb_outputs_tx, encode_staged_hot_shard_ref, fingerprint_repair_reclaim_range,
	manager_effect_for_requested_stop, manager_effects_after_refresh,
	manager_effects_for_hot_job_finished, manager_effects_for_reclaim_job_finished, plan_hot_job,
	read_reclaim_input_snapshot, repair_reclaim_input_range, schedule_next_wake,
	sweep_stale_pidx_chunk_tx,
};
use crate::compaction::shared::{
	CompactionBatchBudget, branch_admitted_now, build_staged_hot_shard_blob,
	decode_branch_pidx_pgno, read_dead_shard_versions_chunk, read_hot_input_snapshot,
	read_pitr_interval_reclaim_rows, read_stale_pidx_chunk, select_pitr_interval_coverage,
};
use crate::compaction::shared::{
	StagedHotShardBlob, collect_hot_pages_by_shard, content_hash, db_size_pages_at_txid,
	plan_drain_head_txid, shard_version_is_retained, snapped_drain_head_txid,
	tx_scan_range_values_limited,
};
use crate::compaction::throttle::CompactionThrottleClass;
use crate::conveyer::{
	Db, branch,
	constants::{
		CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES, CMP_MAX_PENDING_CLEANUP_JOB_IDS,
		HOT_DRAIN_HEAD_GRAIN_TXIDS, MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS,
	},
	keys,
	ltx::{DecodedLtx, LtxHeader, decode_ltx_v3, encode_ltx_v3},
	shard_blob,
	types::{
		BranchState, BucketBranchId, BucketId, CommitRow, DBHead, DbHistoryPin, DbHistoryPinKind,
		DirtyPage, FoldIndexEntry, PitrIntervalCoverage, PitrPolicy, encode_commit_row,
		encode_compaction_root, encode_database_branch_record, encode_db_head,
		encode_fold_index_entry, encode_pitr_interval_coverage,
	},
};
use crate::workflows::SweepCommitDeltaChunkInput;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("depot-workflow-compaction-inline-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn read_raw_key(db: &universaldb::Database, key: &[u8]) -> Result<Option<Vec<u8>>> {
	let key = key.to_vec();
	db.txn("test_depotinline_workflows_compaction", move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(|value| value.to_vec()))
		}
	})
	.await
	.map_err(Into::into)
}

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn bucket_branch_id() -> BucketBranchId {
	BucketBranchId::from_uuid(Uuid::from_u128(0x9abc))
}

fn branch_record(
	database_branch_id: DatabaseBranchId,
	lifecycle_generation: u64,
) -> DatabaseBranchRecord {
	DatabaseBranchRecord {
		branch_id: database_branch_id,
		bucket_branch: bucket_branch_id(),
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [0; 16],
		fork_depth: 0,
		created_at_ms: 1_000,
		created_from_restore_point: None,
		state: BranchState::Live,
		lifecycle_generation,
	}
}

fn root(manifest_generation: u64) -> CompactionRoot {
	root_with_watermarks(manifest_generation, 0, 0)
}

fn root_with_watermarks(
	manifest_generation: u64,
	hot_watermark_txid: u64,
	cold_watermark_txid: u64,
) -> CompactionRoot {
	CompactionRoot {
		schema_version: 1,
		manifest_generation,
		hot_watermark_txid,
		cold_watermark_txid,
		cold_watermark_versionstamp: [0; 16],
	}
}

fn head(database_branch_id: DatabaseBranchId, head_txid: u64) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages: 4,
		post_apply_checksum: 55,
		branch_id: database_branch_id,
	}
}

fn commit(versionstamp_byte: u8) -> CommitRow {
	CommitRow {
		wall_clock_ms: 1_234,
		versionstamp: [versionstamp_byte; 16],
		db_size_pages: 4,
		post_apply_checksum: 5_678,
	}
}

/// The hot install loop state was once a bare cursor integer, and a workflow that stopped early
/// before it became a struct still has that integer persisted. Rejecting it wedges the branch's
/// manager permanently, so both encodings must resume.
#[test]
fn hot_install_resume_accepts_legacy_cursor_state() {
	let legacy = serde_json::from_str::<Option<HotInstallResume>>("5008")
		.expect("legacy scalar loop state should deserialize");
	let legacy = legacy.expect("legacy scalar is a present resume position");
	assert_eq!(legacy.cursor, 5008);
	assert_eq!(legacy.shard_cursor, None);
	assert_eq!(legacy.installed_shard_count, 0);
	assert_eq!(legacy.installed_shard_bytes, 0);

	let current = serde_json::to_string(&HotInstallResume {
		cursor: 5008,
		cursor_segment_pgno: None,
		shard_cursor: Some(HotInstallShardCursor {
			shard_id: 3,
			as_of_txid: 4096,
		}),
		installed_shard_count: 12,
		installed_shard_bytes: 4096,
	})
	.expect("current loop state should serialize");
	let current = serde_json::from_str::<HotInstallResume>(&current)
		.expect("current loop state should round-trip");
	assert_eq!(current.cursor, 5008);
	assert_eq!(
		current.shard_cursor,
		Some(HotInstallShardCursor {
			shard_id: 3,
			as_of_txid: 4096,
		})
	);
	assert_eq!(current.installed_shard_count, 12);
	assert_eq!(current.installed_shard_bytes, 4096);

	assert!(
		serde_json::from_str::<Option<HotInstallResume>>("null")
			.expect("absent loop state should deserialize")
			.is_none()
	);
}

/// Every retained PITR coverage position makes hot staging fold a complete image of every shard it
/// covers, so a disabled cluster must select none. This is what keeps the retained shard versions at
/// one per drain instead of `retention_ms / interval_ms` per shard.
#[test]
fn disabled_pitr_selects_no_interval_coverage() {
	let now_ms = 10 * 60 * 1000;
	let commits = (1..=4_u64)
		.map(|txid| {
			(
				txid,
				CommitRow {
					// Spread across four distinct 1-minute buckets.
					wall_clock_ms: now_ms - (txid as i64 * 60 * 1000),
					versionstamp: [txid as u8; 16],
					db_size_pages: 4,
					post_apply_checksum: 5_678,
				},
			)
		})
		.collect::<Vec<_>>();
	let enabled = PitrPolicy {
		interval_ms: 60 * 1000,
		retention_ms: 60 * 60 * 1000,
	};

	assert_eq!(
		select_pitr_interval_coverage(Some(enabled), &commits, now_ms)
			.expect("enabled selection should succeed")
			.len(),
		4
	);
	assert!(
		select_pitr_interval_coverage(None, &commits, now_ms)
			.expect("disabled selection should succeed")
			.is_empty()
	);
}

fn encoded_delta(txid: u64) -> Result<Vec<u8>> {
	encode_ltx_v3(
		LtxHeader::delta(txid, 1, txid as i64),
		&[DirtyPage {
			pgno: 1,
			bytes: vec![txid as u8; keys::PAGE_SIZE as usize],
		}],
	)
}

fn encoded_delta_on_page(txid: u64, pgno: u32) -> Result<Vec<u8>> {
	encode_ltx_v3(
		LtxHeader::delta(txid, 1, txid as i64),
		&[DirtyPage {
			pgno,
			bytes: vec![txid as u8; keys::PAGE_SIZE as usize],
		}],
	)
}

fn finish_expected_fingerprint(fingerprint: Sha256) -> [u8; 32] {
	let digest = fingerprint.finalize();
	let mut output = [0_u8; 32];
	output.copy_from_slice(&digest);
	output
}

fn update_expected_fingerprint(fingerprint: &mut Sha256, bytes: &[u8]) {
	fingerprint.update((bytes.len() as u64).to_be_bytes());
	fingerprint.update(bytes);
}

fn planned_hot_job(
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	input_range: HotJobInputRange,
) -> PlannedHotCompactionJob {
	let drain_head_txid = input_range.txids.max_txid;
	PlannedHotCompactionJob {
		database_branch_id,
		job_id,
		base_lifecycle_generation: 7,
		base_manifest_generation: 11,
		input_fingerprint: [3; 32],
		input_range,
		drain_head_txid,
		drain_now_ms: 1_234,
		planned_at_ms: 1_234,
		attempt: 2,
	}
}

fn planned_reclaim_job(
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	input_range: ReclaimJobInputRange,
) -> PlannedReclaimCompactionJob {
	PlannedReclaimCompactionJob {
		database_branch_id,
		job_id,
		base_lifecycle_generation: 7,
		base_manifest_generation: 11,
		input_fingerprint: [3; 32],
		input_range,
		planned_at_ms: 1_234,
		attempt: 2,
	}
}

fn companion_workflow_ids() -> CompanionWorkflowIds {
	CompanionWorkflowIds::new(Id::new_v1(4100), Id::new_v1(4102))
}

fn manager_input(database_branch_id: DatabaseBranchId) -> DbManagerInput {
	DbManagerInput::new(database_branch_id, Some("actor-for-test".to_string()))
}

fn reclaim_range() -> ReclaimJobInputRange {
	ReclaimJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		delta_reclaim_segments: Vec::new(),
		cursor_segment_pgno: None,
		commit_reclaim_txids: Vec::new(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		stale_hot_job_ids: Vec::new(),
		stale_commit_stage_txids: Vec::new(),
		stale_cold_job_ids: Vec::new(),
		skip_commit_delta: false,
		cold_scan_cursor: None,
		commit_scan_cursor: 0,
		max_keys: 10,
		max_bytes: 4096,
	}
}

fn refresh_without_planned_work() -> RefreshManagerOutput {
	RefreshManagerOutput {
		refreshed_at_ms: 1_000,
		planned_hot_job: None,
		planned_cold_job: None,
		planned_reclaim_job: None,
		observed_dirty: None,
		head_txid: Some(4),
		branch_is_live: true,
		branch_lifecycle_generation: Some(1),
		db_pin_count: 0,
		reclaim_noop_reason: Some("reclaim:no-actionable-work".to_string()),
		compaction_admitted: true,
		orphan_stage_hot_job_ids: Vec::new(),
		orphan_stage_cold_job_ids: Vec::new(),
		orphan_commit_stage_txids: Vec::new(),
	}
}

#[test]
fn force_compaction_tracker_deduplicates_requests_and_records_noop_results() {
	let database_branch_id = database_branch_id(0x4200);
	let request_id = Id::new_v1(4200);
	let request = ForceCompaction {
		database_branch_id,
		request_id,
		requested_work: ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	};
	let active_jobs = ManagerActiveJobs::default();
	let refresh = refresh_without_planned_work();
	let mut tracker = ForceCompactionTracker::default();

	tracker.record_request(request.clone(), 100, &active_jobs);
	tracker.record_request(request.clone(), 101, &active_jobs);
	assert_eq!(tracker.pending_requests.len(), 1);
	tracker.complete_ready_requests(&active_jobs, &refresh, 102);
	assert!(tracker.pending_requests.is_empty());
	assert_eq!(tracker.recent_results.len(), 1);
	assert_eq!(tracker.recent_results[0].request_id, request_id);
	assert_eq!(
		tracker.recent_results[0].skipped_noop_reasons,
		vec!["hot:no-actionable-lag".to_string()]
	);

	tracker.record_request(request, 103, &active_jobs);
	assert!(tracker.pending_requests.is_empty());
	assert_eq!(tracker.recent_results.len(), 1);
}

#[test]
fn force_compaction_tracker_adopts_active_jobs_and_records_success() {
	let database_branch_id = database_branch_id(0x4201);
	let job_id = Id::new_v1(4201);
	let active_jobs = ManagerActiveJobs {
		hot: Some(ActiveHotCompactionJob::from_planned(planned_hot_job(
			database_branch_id,
			job_id,
			HotJobInputRange {
				txids: TxidRange {
					min_txid: 1,
					max_txid: 4,
				},
				max_pgno_exclusive: None,
				coverage_txids: vec![4],
				max_pages: 8,
				max_bytes: 1024,
			},
		))),
		..Default::default()
	};
	let mut tracker = ForceCompactionTracker::default();

	tracker.record_request(
		ForceCompaction {
			database_branch_id,
			request_id: Id::new_v1(4202),
			requested_work: ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		},
		100,
		&active_jobs,
	);
	assert_eq!(
		tracker.pending_requests[0].attempted_job_kinds,
		vec![CompactionJobKind::Hot]
	);
	tracker.complete_ready_requests(&active_jobs, &refresh_without_planned_work(), 101);
	assert_eq!(tracker.pending_requests.len(), 1);

	tracker.record_job_finished(
		CompactionJobKind::Hot,
		job_id,
		&CompactionJobStatus::Succeeded,
	);
	tracker.complete_ready_requests(
		&ManagerActiveJobs::default(),
		&refresh_without_planned_work(),
		102,
	);
	assert!(tracker.pending_requests.is_empty());
	assert_eq!(tracker.recent_results[0].completed_job_ids, vec![job_id]);
	assert!(tracker.recent_results[0].terminal_error.is_none());
}

#[test]
fn force_compaction_tracker_records_attempted_failed_jobs() {
	let database_branch_id = database_branch_id(0x4203);
	let job_id = Id::new_v1(4203);
	let mut tracker = ForceCompactionTracker::default();

	tracker.record_request(
		ForceCompaction {
			database_branch_id,
			request_id: Id::new_v1(4204),
			requested_work: ForceCompactionWork {
				hot: false,
				cold: false,
				reclaim: true,
				final_settle: false,
			},
		},
		100,
		&ManagerActiveJobs::default(),
	);
	tracker.record_job_attempted(CompactionJobKind::Reclaim);
	tracker.record_job_finished(
		CompactionJobKind::Reclaim,
		job_id,
		&CompactionJobStatus::Failed {
			error: "reclaim delete failed".to_string(),
		},
	);
	tracker.complete_ready_requests(
		&ManagerActiveJobs::default(),
		&refresh_without_planned_work(),
		101,
	);

	let result = &tracker.recent_results[0];
	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Reclaim]);
	assert_eq!(result.completed_job_ids, vec![job_id]);
	assert_eq!(
		result.terminal_error,
		Some("reclaim delete failed".to_string())
	);
}

#[test]
fn manager_effects_map_job_completion_signals_to_workflow_actions() {
	let database_branch_id = database_branch_id(0x4100);
	let input = manager_input(database_branch_id);
	let hot_job_id = Id::new_v1(4103);
	let reclaim_job_id = Id::new_v1(4105);
	let hot_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		max_pgno_exclusive: None,
		coverage_txids: vec![4],
		max_pages: 8,
		max_bytes: 1024,
	};
	let reclaim_range = reclaim_range();
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.active_jobs.hot = Some(ActiveHotCompactionJob::from_planned(planned_hot_job(
		database_branch_id,
		hot_job_id,
		hot_range.clone(),
	)));
	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(
		planned_reclaim_job(database_branch_id, reclaim_job_id, reclaim_range.clone()),
	));

	let hot_effects = manager_effects_for_hot_job_finished(
		&mut state,
		&input,
		HotJobFinished {
			database_branch_id,
			job_id: hot_job_id,
			job_kind: CompactionJobKind::Hot,
			base_manifest_generation: 11,
			status: CompactionJobStatus::Succeeded,
		},
	);
	assert!(matches!(
		hot_effects.as_slice(),
		[ManagerEffect::InstallHotOutput { .. }]
	));

	let reclaim_effects = manager_effects_for_reclaim_job_finished(
		&mut state,
		ReclaimJobFinished {
			database_branch_id,
			job_id: reclaim_job_id,
			job_kind: CompactionJobKind::Reclaim,
			base_manifest_generation: 11,
			input_fingerprint: [3; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: Vec::new(),
		},
		1_000,
	);
	assert!(matches!(
		reclaim_effects.as_slice(),
		[ManagerEffect::FinishReclaimJob { .. }]
	));
}

#[test]
fn manager_effects_cover_branch_stop() {
	let database_branch_id = database_branch_id(0x4101);
	let input = manager_input(database_branch_id);
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.last_observed_branch_lifecycle_generation = Some(5);

	state.branch_stop_state = BranchStopState::StopRequested {
		lifecycle_generation: 6,
		requested_at_ms: 12_345,
		reason: ManagerStopReason::ExplicitDestroy {
			reason: "test destroy".to_string(),
		},
	};
	let Some(ManagerEffect::StopCompanions { request }) =
		manager_effect_for_requested_stop(&state, &input)
	else {
		panic!("expected stop companion effect");
	};
	assert_eq!(request.database_branch_id, database_branch_id);
	assert_eq!(request.lifecycle_generation, 6);
	assert_eq!(request.requested_at_ms, 12_345);
	assert_eq!(
		request.reason,
		ManagerStopReason::ExplicitDestroy {
			reason: "test destroy".to_string(),
		}
	);
}

#[test]
fn manager_dispatches_planned_reclaim_without_a_reclaim_timer_wake() {
	// Hot dispatch keys off `signal_received` while reclaim used to additionally require its own
	// timer to have elapsed. A refresh that found reclaim work on a signal wake therefore threw the
	// planned job away: not dispatched, not queued, and without shortening the next wake, so the
	// branch parked on the idle poll still holding everything it had just planned to free.
	let database_branch_id = database_branch_id(0x4180);
	let input = manager_input(database_branch_id);
	let state = DbManagerState::new(companion_workflow_ids());
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		Id::new_v1(4180),
		reclaim_range(),
	));
	refresh.reclaim_noop_reason = None;

	let triggers = WakeTriggers {
		hot: true,
		// The reclaim timer has NOT elapsed on this wake.
		reclaim: false,
	};

	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"planned reclaim work must dispatch on any wake that finds it, not only a reclaim-timer wake"
	);
}

#[test]
fn manager_dispatches_queued_cleanup_before_planned_reclaim() {
	let database_branch_id = database_branch_id(0x4107);
	let input = manager_input(database_branch_id);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		Id::new_v1(4170),
		reclaim_range(),
	));
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};

	let mut state = DbManagerState::new(companion_workflow_ids());
	assert!(state.pending_cleanups.push_hot(Id::new_v1(4171)));

	// Queued cleanup takes the free slot ahead of planned reclaim work: it is bounded one-shot work
	// and its staging stays resident for as long as it waits.
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::DispatchPendingCleanups)),
		"queued cleanup must dispatch when the reclaim slot is free"
	);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"only one reclaim job may hold the slot"
	);

	// With nothing queued the planned reclaim job runs as before.
	state.pending_cleanups = Default::default();
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"planned reclaim must still run when no cleanup is queued"
	);
}

#[test]
fn manager_cleanup_yields_the_reclaim_slot_every_other_cycle() {
	let database_branch_id = database_branch_id(0x410a);
	let input = manager_input(database_branch_id);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		Id::new_v1(4200),
		reclaim_range(),
	));
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};

	// A cleanup that keeps failing is re-reported by the staging scan forever, so unconditional
	// priority would stop commit, delta, and cold-object reclaim on this branch for good.
	let mut state = DbManagerState::new(companion_workflow_ids());
	assert!(state.pending_cleanups.push_hot(Id::new_v1(4201)));
	state.last_reclaim_slot_was_cleanup = true;
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"reclaim must get the slot after a cleanup took the previous one"
	);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::DispatchPendingCleanups)),
	);

	// Yielding only applies while reclaim work is actually ready, so cleanup keeps every slot no
	// other lane wants.
	refresh.planned_reclaim_job = None;
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::DispatchPendingCleanups)),
		"cleanup must not yield to a lane with nothing planned"
	);
}

#[test]
fn manager_does_not_dispatch_queued_cleanup_while_reclaimer_is_busy() {
	let database_branch_id = database_branch_id(0x4108);
	let input = manager_input(database_branch_id);
	let refresh = refresh_without_planned_work();
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(
		planned_reclaim_job(database_branch_id, Id::new_v1(4180), reclaim_range()),
	));
	assert!(state.pending_cleanups.push_cold(Id::new_v1(4181)));

	let effects = manager_effects_after_refresh(
		&state,
		&input,
		&refresh,
		1_500,
		WakeTriggers {
			hot: false,
			reclaim: true,
		},
	);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::DispatchPendingCleanups)),
		"cleanup must wait for the reclaim slot instead of racing the running job"
	);

	// The queue survives the iteration that could not dispatch it.
	assert!(!state.pending_cleanups.is_empty());
}

#[test]
fn manager_holds_off_a_reclaim_input_that_keeps_rejecting() {
	let database_branch_id = database_branch_id(0x410a);
	let input = manager_input(database_branch_id);
	let job_id = Id::new_v1(4200);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		job_id,
		reclaim_range(),
	));
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};

	let mut state = DbManagerState::new(companion_workflow_ids());
	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(
		planned_reclaim_job(database_branch_id, job_id, reclaim_range()),
	));
	manager_effects_for_reclaim_job_finished(
		&mut state,
		ReclaimJobFinished {
			database_branch_id,
			job_id,
			job_kind: CompactionJobKind::Reclaim,
			base_manifest_generation: 11,
			input_fingerprint: [3; 32],
			status: CompactionJobStatus::Rejected {
				reason: "live cold ref points at missing S3 object".to_string(),
			},
			output_refs: Vec::new(),
		},
		1_000,
	);
	// What the executor does for the `FinishReclaimJob` effect, so the slot is free the way it is on
	// the wake that follows a finished job.
	state.active_jobs.reclaim = None;

	// The rejection is reproduced by the input, and a finished job wakes the manager, so
	// re-dispatching here spins at the job's round-trip latency.
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_100, triggers);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"a just-rejected input must not re-dispatch immediately"
	);

	// The delay only covers the input that produced it, so work that appears meanwhile still runs.
	let mut changed_refresh = refresh.clone();
	let mut changed_job =
		planned_reclaim_job(database_branch_id, Id::new_v1(4201), reclaim_range());
	changed_job.input_fingerprint = [4; 32];
	changed_refresh.planned_reclaim_job = Some(changed_job);
	let effects = manager_effects_after_refresh(&state, &input, &changed_refresh, 1_100, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"a different reclaim input must dispatch while another is held off"
	);

	// The manager arms its own retry rather than waiting for the branch to be written again.
	schedule_next_wake(
		&mut state,
		&input,
		1_100,
		true,
		triggers,
		wake_intervals(60_000),
	);
	let retry_at_ms = state
		.reclaim_backoff
		.as_ref()
		.expect("rejection must record a backoff")
		.retry_at_ms;
	assert_eq!(state.next_reclaim_check_at_ms, Some(retry_at_ms));

	// Once the delay expires the same input is planned and dispatched again.
	let effects = manager_effects_after_refresh(&state, &input, &refresh, retry_at_ms, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. })),
		"the held-off input must run again once its delay expires"
	);
}

#[test]
fn manager_reclaim_backoff_grows_and_clears_on_success() {
	let database_branch_id = database_branch_id(0x410b);
	let mut state = DbManagerState::new(companion_workflow_ids());

	let rejected = |job_id: Id| ReclaimJobFinished {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Reclaim,
		base_manifest_generation: 11,
		input_fingerprint: [3; 32],
		status: CompactionJobStatus::Rejected {
			reason: "live cold ref points at missing S3 object".to_string(),
		},
		output_refs: Vec::new(),
	};

	let mut delays = Vec::new();
	for attempt in 0..3 {
		let job_id = Id::new_v1(4210 + attempt);
		state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(
			planned_reclaim_job(database_branch_id, job_id, reclaim_range()),
		));
		manager_effects_for_reclaim_job_finished(&mut state, rejected(job_id), 1_000);
		delays.push(
			state
				.reclaim_backoff
				.as_ref()
				.expect("rejection must record a backoff")
				.retry_at_ms - 1_000,
		);
	}
	// Consecutive rejections of one input double the wait, so a permanently rejecting input settles
	// at the ceiling instead of running at the round-trip rate forever.
	assert_eq!(
		delays,
		vec![
			MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS,
			MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS * 2,
			MANAGER_RECLAIM_REJECTION_BACKOFF_BASE_MS * 4,
		]
	);

	let job_id = Id::new_v1(4220);
	state.active_jobs.reclaim = Some(ActiveReclaimCompactionJob::from_planned(
		planned_reclaim_job(database_branch_id, job_id, reclaim_range()),
	));
	manager_effects_for_reclaim_job_finished(
		&mut state,
		ReclaimJobFinished {
			status: CompactionJobStatus::Succeeded,
			..rejected(job_id)
		},
		1_000,
	);
	// The input the delay covered is gone, so nothing is left to hold off.
	assert!(state.reclaim_backoff.is_none());
}

#[test]
fn manager_arms_a_wake_while_cleanup_is_queued() {
	let database_branch_id = database_branch_id(0x4109);
	let input = manager_input(database_branch_id);
	let mut state = DbManagerState::new(companion_workflow_ids());
	assert!(state.pending_cleanups.push_hot(Id::new_v1(4190)));

	// No signal arrived and no lane is armed. Without a deadline the manager parks until the next
	// commit, holding the queued job's staging resident for as long as the branch stays quiet.
	schedule_next_wake(
		&mut state,
		&input,
		5_000,
		false,
		WakeTriggers::default(),
		wake_intervals(60_000),
	);
	assert!(state.next_reclaim_check_at_ms.is_some());
}

#[test]
fn pending_cleanup_queue_refuses_ids_past_its_cap() {
	let mut queue = PendingCleanupQueue::default();
	let first_job_id = Id::new_v1(1);
	assert!(queue.push_hot(first_job_id));
	for _ in 1..CMP_MAX_PENDING_CLEANUP_JOB_IDS {
		assert!(queue.push_hot(Id::new_v1(1)));
	}
	// Refusal is not silent loss: the staging orphan scan rediscovers a refused id.
	assert!(!queue.push_hot(Id::new_v1(9_000)));
	// A duplicate is accepted without growing the queue, so a re-reported job cannot fill it.
	assert!(queue.push_hot(first_job_id));
	assert_eq!(queue.hot_job_ids.len(), CMP_MAX_PENDING_CLEANUP_JOB_IDS);
	// The lanes are capped independently.
	assert!(queue.push_cold(Id::new_v1(9_001)));
}

#[test]
fn manager_refresh_effects_skip_unadmitted_branch_unless_forced() {
	let database_branch_id = database_branch_id(0x4104);
	let input = manager_input(database_branch_id);
	let hot_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		max_pgno_exclusive: None,
		coverage_txids: Vec::new(),
		max_pages: 1,
		max_bytes: 512,
	};
	let mut refresh = refresh_without_planned_work();
	refresh.planned_hot_job = Some(planned_hot_job(
		database_branch_id,
		Id::new_v1(4140),
		hot_range,
	));
	// The branch fell outside the percentage-based admission fraction this refresh.
	refresh.compaction_admitted = false;
	let triggers = WakeTriggers {
		hot: true,
		reclaim: false,
	};

	// Not admitted and not forced: no hot job starts even though one is planned and triggered.
	let state = DbManagerState::new(companion_workflow_ids());
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunHotJob { .. })),
		"unadmitted branch must not start a hot job"
	);

	// An explicit force-compaction request bypasses the admission gate for that lane.
	let mut forced_state = DbManagerState::new(companion_workflow_ids());
	forced_state.force_compactions.record_request(
		ForceCompaction {
			database_branch_id,
			request_id: Id::new_v1(4141),
			requested_work: ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		},
		1_400,
		&forced_state.active_jobs,
	);
	let effects = manager_effects_after_refresh(&forced_state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunHotJob { .. })),
		"forced compaction must bypass the admission gate"
	);
}

#[test]
fn admission_helpers_follow_a_runtime_percent_change() {
	use rivet_config::{
		Config, DynamicConfigUpdate,
		config::{Root, Sqlite},
	};

	// Buckets are `uuid % 10_000 / 10_000`, so these two branches sit at 0.05 and 0.95.
	let low_bucket_branch_id = database_branch_id(500);
	let high_bucket_branch_id = database_branch_id(9_500);
	let config = Config::from_root(Root {
		sqlite: Some(Sqlite {
			compaction_admission_percent: Some(100.0),
			..Sqlite::default()
		}),
		..Root::default()
	});

	assert!(branch_admitted_now(&config, low_bucket_branch_id));
	assert!(branch_admitted_now(&config, high_bucket_branch_id));

	// Lowering the percent at runtime is what an in-flight drain re-reads each slice, so the helper
	// has to see the change without a restart.
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(10.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	assert!(branch_admitted_now(&config, low_bucket_branch_id));
	assert!(
		!branch_admitted_now(&config, high_bucket_branch_id),
		"a branch above the new percent must stop being admitted"
	);

	// Raising it again re-admits the branch, so a parked drain resumes where it stopped.
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(100.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	assert!(branch_admitted_now(&config, high_bucket_branch_id));

	// Dropping the percent to zero admits nothing, including the branch in the lowest bucket.
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(0.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	assert!(!branch_admitted_now(&config, low_bucket_branch_id));
}

#[test]
fn manager_marks_only_forced_jobs_as_bypassing_admission() {
	let database_branch_id = database_branch_id(0x4108);
	let input = manager_input(database_branch_id);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_hot_job = Some(planned_hot_job(
		database_branch_id,
		Id::new_v1(4148),
		HotJobInputRange {
			txids: TxidRange {
				min_txid: 1,
				max_txid: 4,
			},
			max_pgno_exclusive: None,
			coverage_txids: Vec::new(),
			max_pages: 1,
			max_bytes: 512,
		},
	));
	let triggers = WakeTriggers {
		hot: true,
		reclaim: false,
	};

	// An ordinary admitted job stays subject to the percent for the rest of its drain, so a later
	// drop can still evict it.
	let state = DbManagerState::new(companion_workflow_ids());
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects.iter().any(|effect| matches!(
			effect,
			ManagerEffect::RunHotJob {
				bypass_admission: false,
				..
			}
		)),
		"an admitted job must stay subject to the admission percent while it drains"
	);

	// A forced job carries the bypass, so lowering the percent mid-drain does not evict work the
	// operator explicitly asked for.
	let mut forced_state = DbManagerState::new(companion_workflow_ids());
	forced_state.force_compactions.record_request(
		ForceCompaction {
			database_branch_id,
			request_id: Id::new_v1(4149),
			requested_work: ForceCompactionWork {
				hot: true,
				cold: false,
				reclaim: false,
				final_settle: false,
			},
		},
		1_400,
		&forced_state.active_jobs,
	);
	let effects = manager_effects_after_refresh(&forced_state, &input, &refresh, 1_500, triggers);
	assert!(
		effects.iter().any(|effect| matches!(
			effect,
			ManagerEffect::RunHotJob {
				bypass_admission: true,
				..
			}
		)),
		"a forced job must bypass the admission percent for its whole drain"
	);
}

/// Zero percent has to mean no compaction work at all. Staging cleanup used to be exempt, which left
/// a de-admitted branch deleting staged shard images at the full compaction budget, and reading every
/// one of them back to validate it first.
#[test]
fn manager_holds_staging_cleanup_on_an_unadmitted_branch() {
	let database_branch_id = database_branch_id(0x4109);
	let input = manager_input(database_branch_id);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		Id::new_v1(4150),
		reclaim_range(),
	));
	// The branch fell outside the percentage-based admission fraction this refresh.
	refresh.compaction_admitted = false;
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};

	let mut state = DbManagerState::new(companion_workflow_ids());
	assert!(state.pending_cleanups.push_hot(Id::new_v1(4151)));
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);

	assert!(
		!effects.iter().any(|effect| matches!(
			effect,
			ManagerEffect::DispatchPendingCleanups | ManagerEffect::RunReclaimJob { .. }
		)),
		"an unadmitted branch must start no reclaim work, staging cleanup included"
	);

	// Holding the queue is what makes that safe: the ids are still there for the next admitted wake,
	// so a percent that goes down and comes back up costs a delay rather than the staging area.
	assert!(
		!state.pending_cleanups.is_empty(),
		"holding cleanup must leave the queued job ids in place"
	);
	refresh.compaction_admitted = true;
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::DispatchPendingCleanups)),
		"a re-admitted branch must resume the staging cleanup it was holding"
	);
}

#[test]
fn manager_holds_every_reclaim_lane_while_unadmitted() {
	let database_branch_id = database_branch_id(0x410a);
	let input = manager_input(database_branch_id);
	let mut refresh = refresh_without_planned_work();
	refresh.planned_reclaim_job = Some(planned_reclaim_job(
		database_branch_id,
		Id::new_v1(4152),
		reclaim_range(),
	));
	refresh.compaction_admitted = false;
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};

	let state = DbManagerState::new(companion_workflow_ids());
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		!effects.iter().any(|effect| matches!(
			effect,
			ManagerEffect::RunReclaimJob { .. } | ManagerEffect::DispatchPendingCleanups
		)),
		"an unadmitted branch with nothing queued must start no reclaim work"
	);

	// Raising the percent again lets the ordinary scan through, still subject to the percent.
	refresh.compaction_admitted = true;
	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500, triggers);
	assert!(
		effects.iter().any(|effect| matches!(
			effect,
			ManagerEffect::RunReclaimJob {
				bypass_admission: false,
				..
			}
		)),
		"a re-admitted branch must resume its reclaim scan"
	);
}

#[test]
fn manager_refresh_effects_stop_branch_not_live_with_explicit_reason() {
	let database_branch_id = database_branch_id(0x4103);
	let input = manager_input(database_branch_id);
	let state = DbManagerState::new(companion_workflow_ids());
	let mut refresh = refresh_without_planned_work();
	refresh.branch_is_live = false;
	refresh.branch_lifecycle_generation = Some(9);

	let effects =
		manager_effects_after_refresh(&state, &input, &refresh, 12_000, WakeTriggers::default());
	let [ManagerEffect::StopCompanions { request }] = effects.as_slice() else {
		panic!("expected branch-not-live stop effect");
	};
	assert_eq!(request.database_branch_id, database_branch_id);
	assert_eq!(request.lifecycle_generation, 9);
	assert_eq!(request.requested_at_ms, 12_000);
	assert_eq!(request.reason, ManagerStopReason::BranchNotLive);
}

#[test]
fn manager_active_jobs_store_typed_lanes_independently() {
	let database_branch_id = database_branch_id(0x3300);
	let hot_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 10,
		},
		max_pgno_exclusive: None,
		coverage_txids: vec![5, 10],
		max_pages: 8,
		max_bytes: 1024,
	};
	let reclaim_range = ReclaimJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		delta_reclaim_segments: Vec::new(),
		cursor_segment_pgno: None,
		commit_reclaim_txids: Vec::new(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		stale_hot_job_ids: Vec::new(),
		stale_commit_stage_txids: Vec::new(),
		stale_cold_job_ids: Vec::new(),
		skip_commit_delta: false,
		cold_scan_cursor: None,
		commit_scan_cursor: 0,
		max_keys: 10,
		max_bytes: 4096,
	};

	let mut active_jobs = ManagerActiveJobs {
		hot: Some(ActiveHotCompactionJob::from_planned(planned_hot_job(
			database_branch_id,
			Id::new_v1(3300),
			hot_range.clone(),
		))),
		cold: None,
		reclaim: Some(ActiveReclaimCompactionJob::from_planned(
			planned_reclaim_job(database_branch_id, Id::new_v1(3302), reclaim_range.clone()),
		)),
	};

	assert_eq!(
		active_jobs.hot.as_ref().unwrap().input_range.coverage_txids,
		vec![5, 10]
	);
	assert_eq!(
		active_jobs.reclaim.as_ref().unwrap().input_range.max_keys,
		10
	);

	active_jobs.hot = None;
	assert!(active_jobs.hot.is_none());
	assert!(active_jobs.reclaim.is_some());
}

#[test]
fn hot_planning_uses_sha256_fingerprint_and_changes_with_inputs() {
	let database_branch_id = database_branch_id(0x3600);
	let root = root_with_watermarks(7, 0, 0);
	let head = head(database_branch_id, 2);
	let mut hot_inputs = HotInputSnapshot {
		commits: vec![(1, commit(1)), (2, commit(2))],
		delta_chunks: vec![(b"delta-key".to_vec(), b"delta-value".to_vec())],
		pidx_entries: vec![(b"pidx-key".to_vec(), b"pidx-value".to_vec())],
		pitr_interval_coverage: Vec::new(),
		total_value_bytes: 24,
		selected_max_txid: Some(2),
		oversized_commit_txid: None,
		selected_max_pgno_exclusive: None,
	};
	let mut snapshot = ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head.clone()),
		root: root.clone(),
		dirty: None,
		db_pins: Vec::new(),
		hot_inputs,
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	let first_job = plan_hot_job(
		database_branch_id,
		&snapshot,
		Id::new_v1(3600),
		1_000,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	let second_job = plan_hot_job(
		database_branch_id,
		&snapshot,
		Id::new_v1(3601),
		1_001,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	assert_eq!(first_job.input_fingerprint, second_job.input_fingerprint);

	let mut expected = Sha256::new();
	update_expected_fingerprint(&mut expected, database_branch_id.as_uuid().as_bytes());
	update_expected_fingerprint(&mut expected, &root.manifest_generation.to_be_bytes());
	update_expected_fingerprint(&mut expected, &root.hot_watermark_txid.to_be_bytes());
	update_expected_fingerprint(&mut expected, &head.head_txid.to_be_bytes());
	update_expected_fingerprint(&mut expected, &head.head_txid.to_be_bytes());
	for (txid, commit) in &snapshot.hot_inputs.commits {
		update_expected_fingerprint(&mut expected, &txid.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.wall_clock_ms.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.versionstamp);
		update_expected_fingerprint(&mut expected, &commit.db_size_pages.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.post_apply_checksum.to_be_bytes());
	}
	for (key, value) in &snapshot.hot_inputs.delta_chunks {
		update_expected_fingerprint(&mut expected, key);
		update_expected_fingerprint(&mut expected, value);
	}
	for (key, value) in &snapshot.hot_inputs.pidx_entries {
		update_expected_fingerprint(&mut expected, key);
		update_expected_fingerprint(&mut expected, value);
	}
	assert_eq!(
		first_job.input_fingerprint,
		finish_expected_fingerprint(expected)
	);

	hot_inputs = snapshot.hot_inputs;
	hot_inputs.delta_chunks[0].1[0] ^= 0xff;
	snapshot.hot_inputs = hot_inputs;
	let changed_job = plan_hot_job(
		database_branch_id,
		&snapshot,
		Id::new_v1(3602),
		1_002,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	assert_ne!(first_job.input_fingerprint, changed_job.input_fingerprint);
}

#[test]
fn hot_planning_caps_drain_head_to_span_window() {
	let database_branch_id = database_branch_id(0x3603);
	let hot_watermark_txid = 5_u64;
	let backlog_head_txid = hot_watermark_txid + TEST_MAX_HOT_DRAIN_SPAN_TXIDS + 10_000;
	let hot_inputs = HotInputSnapshot {
		commits: vec![(6, commit(6))],
		delta_chunks: Vec::new(),
		pidx_entries: Vec::new(),
		pitr_interval_coverage: Vec::new(),
		total_value_bytes: 8,
		selected_max_txid: Some(6),
		oversized_commit_txid: None,
		selected_max_pgno_exclusive: None,
	};
	let mut snapshot = ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head(database_branch_id, backlog_head_txid)),
		root: root_with_watermarks(7, hot_watermark_txid, 0),
		dirty: None,
		db_pins: Vec::new(),
		hot_inputs,
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	// A backlog past the window drains a capped job; later windows catch up on later refreshes.
	let capped_job = plan_hot_job(
		database_branch_id,
		&snapshot,
		Id::new_v1(3603),
		1_000,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	assert_eq!(
		capped_job.drain_head_txid,
		hot_watermark_txid + TEST_MAX_HOT_DRAIN_SPAN_TXIDS
	);

	// A backlog inside the window still drains to the real head.
	let in_window_head_txid = hot_watermark_txid + 100;
	snapshot.head = Some(head(database_branch_id, in_window_head_txid));
	let uncapped_job = plan_hot_job(
		database_branch_id,
		&snapshot,
		Id::new_v1(3604),
		1_001,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	assert_eq!(uncapped_job.drain_head_txid, in_window_head_txid);
}

#[tokio::test]
async fn repair_fdb_cleanup_clears_a_staged_shard_whose_bytes_do_not_match_its_ref() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3402);
	let stale_job_id = Id::new_v1(36);
	let staged_blob = vec![7_u8; 32];
	let stage_key =
		keys::branch_compaction_stage_hot_shard_key(database_branch_id, stale_job_id, 0, 1, 0);
	let ref_key =
		keys::branch_compaction_stage_hot_ref_key(database_branch_id, stale_job_id, 1, 0, 1);
	let input_range =
		repair_reclaim_input_range(vec![stale_job_id], Vec::new(), std::iter::once(1));
	let input = ReclaimFdbJobInput {
		database_branch_id,
		job_id: Id::new_v1(37),
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 1,
		base_manifest_generation: 1,
		input_fingerprint: fingerprint_repair_reclaim_range(database_branch_id, &input_range),
		input_range,
		bypass_admission: false,
	};

	let output = db
		.txn("test_depotinline_workflows_compaction", {
			let staged_blob = staged_blob.clone();
			let input = input.clone();
			let stage_key = stage_key.clone();
			let ref_key = ref_key.clone();
			move |tx| {
				let staged_blob = staged_blob.clone();
				let input = input.clone();
				let stage_key = stage_key.clone();
				let ref_key = ref_key.clone();
				async move {
					tx.informal().set(
						&keys::branches_list_key(database_branch_id),
						&encode_database_branch_record(branch_record(database_branch_id, 1))?,
					);
					tx.informal().set(
						&keys::branch_compaction_root_key(database_branch_id),
						&encode_compaction_root(root(1))?,
					);
					tx.informal().set(&stage_key, &staged_blob);
					// A ref whose content hash does not describe the staged blob. Retaining the pair
					// would leave the ref row in place, which keeps the staging orphan scan
					// re-reporting this job on every refresh with nothing ever reclaiming it.
					tx.informal().set(
						&ref_key,
						&encode_staged_hot_shard_ref(StagedHotShardRef {
							shard_id: 0,
							as_of_txid: 1,
							min_txid: 1,
							size_bytes: staged_blob.len() as u64,
							content_hash: [0_u8; 32],
						})?,
					);

					cleanup_repair_fdb_outputs_tx(&tx, &input, rivet_util::timestamp::now()).await
				}
			}
		})
		.await?;

	assert_eq!(output.status, CompactionJobStatus::Succeeded);
	assert!(!output.has_more);

	let remaining = db
		.txn("test_depotinline_workflows_compaction", move |tx| {
			let stage_key = stage_key.clone();
			let ref_key = ref_key.clone();
			async move {
				let blob = tx.informal().get(&stage_key, Snapshot).await?;
				let staged_ref = tx.informal().get(&ref_key, Snapshot).await?;
				Ok((blob.is_some(), staged_ref.is_some()))
			}
		})
		.await?;
	assert_eq!(remaining, (false, false));

	Ok(())
}

#[tokio::test]
async fn repair_fdb_cleanup_lifecycle_generation_rejects_recreated_branch() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3400);
	let stale_job_id = Id::new_v1(34);
	let staged_blob = vec![7_u8; 32];
	let stage_key =
		keys::branch_compaction_stage_hot_shard_key(database_branch_id, stale_job_id, 0, 1, 0);
	// The cleanup rejects at the branch-lifecycle check before scanning the staging area, so the ref
	// rows do not need to exist for this test; only the stale job id is carried.
	let input_range =
		repair_reclaim_input_range(vec![stale_job_id], Vec::new(), std::iter::once(1));
	let input = ReclaimFdbJobInput {
		database_branch_id,
		job_id: Id::new_v1(35),
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 0,
		base_manifest_generation: 1,
		input_fingerprint: fingerprint_repair_reclaim_range(database_branch_id, &input_range),
		input_range,
		bypass_admission: false,
	};

	let output = db
		.txn("test_depotinline_workflows_compaction", {
			let staged_blob = staged_blob.clone();
			let input = input.clone();
			let stage_key = stage_key.clone();
			move |tx| {
				let staged_blob = staged_blob.clone();
				let input = input.clone();
				let stage_key = stage_key.clone();
				async move {
					tx.informal().set(
						&keys::branches_list_key(database_branch_id),
						&encode_database_branch_record(branch_record(database_branch_id, 1))?,
					);
					tx.informal().set(
						&keys::branch_compaction_root_key(database_branch_id),
						&encode_compaction_root(root(1))?,
					);
					tx.informal().set(&stage_key, &staged_blob);

					cleanup_repair_fdb_outputs_tx(&tx, &input, rivet_util::timestamp::now()).await
				}
			}
		})
		.await?;

	assert_eq!(
		output.status,
		CompactionJobStatus::Rejected {
			reason: "database branch lifecycle changed".to_string(),
		}
	);
	let stage_after = db
		.txn("test_depotinline_workflows_compaction", move |tx| {
			let stage_key = stage_key.clone();
			async move {
				Ok(tx
					.informal()
					.get(&stage_key, Snapshot)
					.await?
					.map(Vec::from))
			}
		})
		.await?;
	assert_eq!(stage_after, Some(staged_blob));

	Ok(())
}

#[tokio::test]
async fn hot_input_snapshot_caps_on_complete_txid_units() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3600);
	let root = root_with_watermarks(1, 0, 0);
	let head = head(database_branch_id, 300);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			let head = head.clone();
			move |tx| {
				let root = root.clone();
				let head = head.clone();
				async move {
					for txid in 1..=head.head_txid {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
						tx.informal().set(
							&keys::branch_delta_chunk_key(database_branch_id, txid, 0),
							&encoded_delta(txid)?,
						);
					}

					read_hot_input_snapshot(
						&tx,
						database_branch_id,
						Some(&head),
						&root,
						None,
						None,
						Snapshot,
						PitrPolicy::from_config(&rivet_config::config::Sqlite::default()),
						1_000,
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(snapshot.selected_max_txid, Some(249));
	assert_eq!(snapshot.commits.len(), 249);
	assert_eq!(snapshot.delta_chunks.len(), 249);

	let planned = plan_hot_job(
		database_branch_id,
		&ManagerFdbSnapshot {
			branch_record: Some(branch_record(database_branch_id, 0)),
			head: Some(head),
			root,
			dirty: None,
			db_pins: Vec::new(),
			hot_inputs: snapshot,
			reclaim_inputs: ReclaimInputSnapshot::default(),
			bucket_proof_blocked_reclaim: false,
			cleared_dirty: false,
		},
		Id::new_v1(3600),
		1_000,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("hot job should be planned");
	// One txid short of the 500-key cap's 250: every delta here dirties page 1, so the slice reserves
	// a single PIDX row for it and admits commits until the reservation no longer fits alongside them.
	assert_eq!(planned.input_range.txids.max_txid, 249);
	assert_eq!(planned.input_range.coverage_txids, vec![249]);

	Ok(())
}

/// The repair walk clears a PIDX row stranded below the watermark only when a real SHARD version
/// covers its page, and leaves live rows alone.
#[tokio::test]
async fn stale_pidx_chunk_clears_covered_rows_and_keeps_live_rows() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3610);
	let root = root_with_watermarks(1, 100, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					// Stranded: owner below the watermark, and shard 0 has a folded image at 90 that
					// covers it.
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 1),
						&28_u64.to_be_bytes(),
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 28, 0),
						&shard_image(28, &[1])?,
					);
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 90),
						&shard_image(90, &[1])?,
					);
					// Live: owner above the watermark, so the row is the correct owner and reads must
					// keep resolving through it.
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 2),
						&150_u64.to_be_bytes(),
					);

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(
		chunk.candidates,
		vec![(
			keys::branch_pidx_key(database_branch_id, 1),
			28_u64.to_be_bytes().to_vec()
		)]
	);
	assert!(!chunk.has_more);

	Ok(())
}

/// A stranded row whose page has no covering SHARD version must be retained. Clearing it would drop
/// the only pointer to the page's contents, so the walk fails closed.
#[tokio::test]
async fn stale_pidx_chunk_retains_row_without_covering_shard_version() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3611);
	let root = root_with_watermarks(1, 100, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 1),
						&28_u64.to_be_bytes(),
					);
					// The only image of shard 0 predates the owner txid, so it cannot be the fold that
					// covered this page.
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 27),
						&shard_image(27, &[1])?,
					);

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	assert!(chunk.candidates.is_empty());
	assert!(!chunk.has_more);

	Ok(())
}

/// A window filled entirely with live rows must still advance its cursor. Otherwise the live rows
/// that fill every window stall the stranded rows sitting behind them and the walk never drains,
/// which is the failure mode the windowed reclaim scans already had to fix.
#[tokio::test]
async fn stale_pidx_chunk_advances_cursor_past_live_rows() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3612);
	let root = root_with_watermarks(1, 100, 0);
	// One full window of live rows, then a single stranded row behind them.
	let live_pages = u32::try_from(CMP_FDB_BATCH_MAX_KEYS).unwrap();
	let stranded_pgno = live_pages + 1;

	let (first, second) = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					for pgno in 1..=live_pages {
						tx.informal().set(
							&keys::branch_pidx_key(database_branch_id, pgno),
							&150_u64.to_be_bytes(),
						);
					}
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, stranded_pgno),
						&28_u64.to_be_bytes(),
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 28, 0),
						&shard_image(28, &[stranded_pgno])?,
					);
					tx.informal().set(
						&keys::branch_shard_key(
							database_branch_id,
							stranded_pgno / keys::SHARD_SIZE,
							90,
						),
						&shard_image(90, &[stranded_pgno])?,
					);

					let first = read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await?;
					let second = read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						first.next_pgno_cursor,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await?;

					Ok((first, second))
				}
			}
		})
		.await?;

	// The first window found nothing clearable but still reports more work and moves the cursor.
	assert!(first.candidates.is_empty());
	assert!(first.has_more);
	assert_eq!(first.next_pgno_cursor, Some(live_pages + 1));
	// The second window reaches the row the live rows were hiding.
	assert_eq!(
		second.candidates,
		vec![(
			keys::branch_pidx_key(database_branch_id, stranded_pgno),
			28_u64.to_be_bytes().to_vec()
		)]
	);
	assert!(!second.has_more);

	Ok(())
}

/// The sweep is one-shot per branch, so it must not spend its marker on a walk that could not have
/// found anything. Below the first fold every row classifies live no matter what the branch holds, so
/// the sweep returns without walking and leaves the marker unwritten for the walk that comes after
/// folding starts.
#[tokio::test]
async fn stale_pidx_sweep_does_not_retire_a_branch_below_the_first_fold() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3613);

	let outcome = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branches_list_key(database_branch_id),
					&encode_database_branch_record(branch_record(database_branch_id, 1))?,
				);
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root_with_watermarks(1, 0, 0))?,
				);
				tx.informal().set(
					&keys::branch_pidx_key(database_branch_id, 1),
					&28_u64.to_be_bytes(),
				);

				sweep_stale_pidx_chunk_tx(
					&tx,
					&SweepStalePidxInput {
						database_branch_id,
						base_lifecycle_generation: 1,
						base_manifest_generation: 1,
						pgno_cursor: None,
						retained_unconfirmed: false,
					},
					None,
				)
				.await
			},
		)
		.await?;

	assert!(matches!(
		outcome,
		StalePidxSweepOutcome::Terminal(CompactionJobStatus::Succeeded)
	));
	assert!(
		read_raw_key(
			&db,
			&keys::branch_compaction_pidx_repair_key(database_branch_id),
		)
		.await?
		.is_none()
	);

	Ok(())
}

/// A walk above the first fold is a real one, so finishing it retires the branch and records the
/// watermark it ran at.
#[tokio::test]
async fn stale_pidx_sweep_retires_the_branch_once_the_walk_finishes() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3614);

	let outcome = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branches_list_key(database_branch_id),
					&encode_database_branch_record(branch_record(database_branch_id, 1))?,
				);
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root_with_watermarks(1, 100, 0))?,
				);
				// Stranded below the watermark with a folded image covering its page, so the walk has
				// a real row to clear.
				tx.informal().set(
					&keys::branch_pidx_key(database_branch_id, 1),
					&28_u64.to_be_bytes(),
				);
				tx.informal().set(
					&keys::branch_delta_chunk_key(database_branch_id, 28, 0),
					&shard_image(28, &[1])?,
				);
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, 90),
					&shard_image(90, &[1])?,
				);

				sweep_stale_pidx_chunk_tx(
					&tx,
					&SweepStalePidxInput {
						database_branch_id,
						base_lifecycle_generation: 1,
						base_manifest_generation: 1,
						pgno_cursor: None,
						retained_unconfirmed: false,
					},
					None,
				)
				.await
			},
		)
		.await?;

	assert!(matches!(
		outcome,
		StalePidxSweepOutcome::Continue {
			has_more: false,
			cleared: 1,
			..
		}
	));
	assert_eq!(
		read_raw_key(
			&db,
			&keys::branch_compaction_pidx_repair_key(database_branch_id),
		)
		.await?,
		Some(100_u64.to_be_bytes().to_vec())
	);

	Ok(())
}

/// Commit selection must leave the PIDX clear lane room for every page the slice folds. A slice that
/// folds a page without clearing its PIDX row strands that row forever: its owner txid falls below the
/// next slice's `min_txid`, which the owner-window filter skips, so the row pins its delta and its
/// commit against reclaim for the life of the branch.
#[tokio::test]
async fn hot_input_snapshot_reserves_budget_for_every_folded_page() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3603);
	let root = root_with_watermarks(1, 0, 0);
	// Enough commits that the budget must cut the slice short. Each dirties its own page, so every
	// selected commit contributes a distinct PIDX row to clear. That is the shape that starves the
	// clear lane when only the commits are budgeted.
	let head = head(database_branch_id, 300);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			let head = head.clone();
			move |tx| {
				let root = root.clone();
				let head = head.clone();
				async move {
					for txid in 1..=head.head_txid {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
						tx.informal().set(
							&keys::branch_delta_chunk_key(database_branch_id, txid, 0),
							&encoded_delta_on_page(txid, txid as u32)?,
						);
						tx.informal().set(
							&keys::branch_pidx_key(database_branch_id, txid as u32),
							&txid.to_be_bytes(),
						);
					}

					read_hot_input_snapshot(
						&tx,
						database_branch_id,
						Some(&head),
						&root,
						None,
						None,
						Snapshot,
						PitrPolicy::from_config(&rivet_config::config::Sqlite::default()),
						1_000,
					)
					.await
				}
			}
		})
		.await?;

	// The slice really is budget-capped, so this exercises the contended path rather than one that
	// happened to fit whole.
	assert!(
		snapshot.commits.len() < 300,
		"slice should be capped by the budget, selected {}",
		snapshot.commits.len()
	);
	assert!(!snapshot.commits.is_empty());
	// Every selected commit dirtied one page and still owns that page's PIDX row, so the slice carries
	// one clear per selected commit. Before the reservation the commits consumed the whole budget and
	// this was zero.
	assert_eq!(snapshot.pidx_entries.len(), snapshot.commits.len());

	let selected_max_txid = snapshot
		.selected_max_txid
		.expect("slice should select commits");
	let pidx_keys = snapshot
		.pidx_entries
		.iter()
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>();
	let expected_pidx_keys = (1..=selected_max_txid)
		.map(|txid| keys::branch_pidx_key(database_branch_id, txid as u32))
		.collect::<Vec<_>>();
	assert_eq!(pidx_keys, expected_pidx_keys);

	Ok(())
}

#[tokio::test]
async fn hot_input_snapshot_caps_on_value_bytes() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3602);
	let root = root_with_watermarks(1, 0, 0);
	let head = head(database_branch_id, 2);
	let first_commit = encode_commit_row(commit(1))?;
	// The selected delta must be a real LTX blob because the snapshot decodes it for the
	// byte-volume gate; the over-budget one is never selected, so its bytes are opaque.
	let first_delta = encoded_delta(1)?;
	let second_commit = encode_commit_row(commit(2))?;
	let second_delta = vec![2_u8; CMP_FDB_BATCH_MAX_VALUE_BYTES];
	let expected_value_bytes = (first_commit.len() + first_delta.len()) as u64;

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			let head = head.clone();
			let first_commit = first_commit.clone();
			let first_delta = first_delta.clone();
			let second_commit = second_commit.clone();
			let second_delta = second_delta.clone();
			move |tx| {
				let root = root.clone();
				let head = head.clone();
				let first_commit = first_commit.clone();
				let first_delta = first_delta.clone();
				let second_commit = second_commit.clone();
				let second_delta = second_delta.clone();
				async move {
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 1),
						&first_commit,
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 1, 0),
						&first_delta,
					);
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 2),
						&second_commit,
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 2, 0),
						&second_delta,
					);

					read_hot_input_snapshot(
						&tx,
						database_branch_id,
						Some(&head),
						&root,
						None,
						None,
						Snapshot,
						PitrPolicy::from_config(&rivet_config::config::Sqlite::default()),
						1_000,
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(snapshot.selected_max_txid, Some(1));
	assert_eq!(snapshot.commits.len(), 1);
	assert_eq!(snapshot.commits[0].0, 1);
	assert_eq!(snapshot.delta_chunks.len(), 1);
	assert_eq!(snapshot.total_value_bytes, expected_value_bytes);

	Ok(())
}

#[tokio::test]
async fn reclaim_input_snapshot_bounds_commit_scan_by_reclaim_ceiling() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3700);
	let root = root_with_watermarks(1, 10, 0);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 10),
						&encode_commit_row(commit(10))?,
					);
					let mut malformed_high_key = keys::branch_commit_key(database_branch_id, 11);
					malformed_high_key.push(b'/');
					tx.informal()
						.set(&malformed_high_key, b"must-not-be-scanned");

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(snapshot.commit_reclaim_txids, vec![10]);
	assert_eq!(snapshot.commits.len(), 1);

	Ok(())
}

/// A scan that crosses its elapsed bound reports the truncation and leaves its cursor alone, so the
/// caller re-derives the same window rather than acting on a partial set. The delete side compares
/// what it re-derives against the planned set, and a partial set can only ever miscompare, so
/// truncation has to be distinguishable from "this window really does hold less".
#[tokio::test]
async fn reclaim_input_snapshot_truncates_on_elapsed_bound() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3702);
	let root = root_with_watermarks(1, 10, 0);
	let commit_scan_cursor = 3;

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					for txid in 3..=10 {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
					}

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						commit_scan_cursor,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						// Already elapsed, so the scan gives up before it reads any txid's deltas.
						Some(Instant::now()),
						true,
					)
					.await
				}
			}
		})
		.await?;

	assert!(snapshot.scan_truncated);
	// Unmoved, so the next pass re-derives this window from the same place.
	assert_eq!(snapshot.next_commit_scan_cursor, commit_scan_cursor);
	// Truncation is not the end of the range; reporting it complete would retire a window holding
	// reclaimable history.
	assert!(!snapshot.commit_scan_complete);
	assert!(snapshot.commit_reclaim_txids.is_empty());
	assert!(snapshot.delta_reclaim_segments.is_empty());

	Ok(())
}

#[tokio::test]
async fn reclaim_input_snapshot_caps_on_complete_txid_units() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3701);
	let root = root_with_watermarks(1, 300, 0);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					for txid in 1..=root.hot_watermark_txid {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
						tx.informal().set(
							&keys::branch_delta_chunk_key(database_branch_id, txid, 0),
							&encoded_delta(txid)?,
						);
					}

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	// The commit scan is budget-capped at 250 txid units, which is what this test is about: the
	// window stops on a whole txid rather than splitting one across passes.
	let delta_txids = snapshot
		.delta_chunks
		.iter()
		.map(|(key, _)| keys::decode_branch_delta_chunk_txid(database_branch_id, key))
		.collect::<Result<Vec<_>>>()?;
	assert_eq!(snapshot.commits.len(), 250);
	assert_eq!(snapshot.delta_chunks.len(), 250);
	assert_eq!(delta_txids.last(), Some(&250));

	// No shards are materialized, so the shard-materialization gate withholds every folded delta, and
	// the commit rows are withheld with them: the sweep finds history by scanning `COMMITS`, so a
	// commit deleted ahead of its surviving delta would strand that delta forever.
	assert!(snapshot.delta_reclaim_segments.is_empty());
	assert!(snapshot.commit_reclaim_txids.is_empty());

	Ok(())
}

// When cold storage is enabled, reclaim must not delete commits/deltas above the cold watermark,
// since cold compaction still needs them to archive each hot-fold boundary. Without this bound,
// reclaim races ahead of cold, strips the commits cold's versionstamp scan depends on, and stalls
// cold compaction permanently (observed on branch 6d17e582-ee16-4755-93b7-f5920182ef5a).
#[tokio::test]
async fn reclaim_bounded_by_cold_watermark_when_cold_enabled() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3702);
	// Hot has folded up to 300, but cold has only archived through 100.
	let root = root_with_watermarks(1, 300, 100);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					for txid in 1..=root.hot_watermark_txid {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
						tx.informal().set(
							&keys::branch_delta_chunk_key(database_branch_id, txid, 0),
							&encoded_delta(txid)?,
						);
					}

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	// Reclaim stops at the cold watermark (100), leaving 101..=300 intact for cold to archive. The
	// commit scan now reaches the hot watermark (budget-capped at 250 units) so folded deltas above the
	// floor can be considered, but both COMMITS and DELTA reclaim cap at the cold watermark: COMMITS via
	// the delete bound, and folded DELTAs because only txids at or below the cold watermark are proven
	// recoverable (published to cold) without a materialized shard.
	assert_eq!(snapshot.commits.len(), 250);
	assert_eq!(snapshot.delta_chunks.len(), 250);
	assert_eq!(snapshot.commit_reclaim_txids.last(), Some(&100));
	assert_eq!(snapshot.commit_reclaim_txids.len(), 100);
	assert_eq!(
		reclaim_txids(&snapshot.delta_reclaim_segments).last(),
		Some(&100)
	);
	assert_eq!(reclaim_txids(&snapshot.delta_reclaim_segments).len(), 100);

	Ok(())
}

// A branch whose leading commit history is retained must still reclaim the history behind it. Every
// scanned txid is charged against the batch budget whether or not it is reclaimable, so a scan that
// always restarted at txid 0 spent the whole window on the retained prefix and reported "nothing
// reclaimable" forever, permanently stranding the reclaimable tail (observed as ~8 GB of folded
// deltas pinned on an idle branch). The windowed cursor makes each pass advance past the prefix.
#[tokio::test]
async fn reclaim_input_snapshot_advances_commit_scan_past_retained_prefix() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3703);
	let hot_watermark_txid = 300;
	let root = root_with_watermarks(1, hot_watermark_txid, 0);
	// The budget admits 250 txid units (one commit row plus one delta chunk each), so the retained
	// prefix exactly fills the first window.
	let retained_prefix_txid = 250;

	let seed_root = root.clone();
	db.txn("test_depotinline_workflows_compaction", move |tx| {
		let _root = seed_root.clone();
		async move {
			for txid in 1..=hot_watermark_txid {
				tx.informal().set(
					&keys::branch_commit_key(database_branch_id, txid),
					&encode_commit_row(commit(txid as u8))?,
				);
				tx.informal().set(
					&keys::branch_delta_chunk_key(database_branch_id, txid, 0),
					&encoded_delta_on_page(txid, txid as u32)?,
				);
				// Each txid in the prefix still owns its own delta page, so reclaim withholds both its
				// commit row and its delta. These rows survive every pass and cannot drain out of the
				// way on their own.
				if txid <= retained_prefix_txid {
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, txid as u32),
						&txid.to_be_bytes(),
					);
				}
			}

			Ok(())
		}
	})
	.await?;

	let read_window = |commit_scan_cursor: u64| {
		let root = root.clone();
		let db = db.clone();
		async move {
			db.txn("test_depotinline_workflows_compaction", move |tx| {
				let root = root.clone();
				async move {
					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						commit_scan_cursor,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			})
			.await
		}
	};

	// The first window is entirely retained rows, so it plans no work. It must still report where to
	// resume and that the sweep is unfinished.
	let first = read_window(0).await?;
	assert!(first.commit_reclaim_txids.is_empty());
	assert!(first.delta_reclaim_segments.is_empty());
	assert!(!first.commit_scan_complete);
	assert_eq!(
		first.next_commit_scan_cursor,
		retained_prefix_txid.saturating_add(1)
	);

	// Resuming from that cursor reaches the tail the old from-zero scan could never see. The window
	// it scans is the subject here; whether those rows classify as reclaimable depends on the
	// materialization gate, which this fixture deliberately leaves unsatisfied.
	let second = read_window(first.next_commit_scan_cursor).await?;
	assert_eq!(
		second
			.commits
			.iter()
			.map(|(txid, _, _, _)| *txid)
			.collect::<Vec<_>>(),
		(retained_prefix_txid + 1..=hot_watermark_txid).collect::<Vec<_>>()
	);
	assert!(second.commit_scan_complete);
	assert_eq!(
		second.next_commit_scan_cursor,
		hot_watermark_txid.saturating_add(1)
	);

	// A cursor past the reclaimable range terminates the sweep instead of rescanning.
	let exhausted = read_window(second.next_commit_scan_cursor).await?;
	assert!(exhausted.commits.is_empty());
	assert!(exhausted.commit_scan_complete);

	Ok(())
}

fn restore_point_pin(at_txid: u64) -> DbHistoryPin {
	let mut at_versionstamp = [0; 16];
	at_versionstamp[8..16].copy_from_slice(&at_txid.to_be_bytes());
	DbHistoryPin {
		at_versionstamp,
		at_txid,
		kind: DbHistoryPinKind::RestorePoint,
		owner_database_branch_id: None,
		owner_bucket_branch_id: None,
		owner_restore_point: None,
		created_at_ms: 1_000,
	}
}

// C6: a folded delta (no live PIDX owner) whose shard is materialized at the smallest coverage fold is
// reclaimable. Here txid 1 changed page 1 (shard 0), was overwritten at txid 2 so PIDX no longer points
// at it, and `SHARD/0/1` + `CMP/fold/1` record the materialization. The shard-materialization gate
// passes, so the folded delta is dropped.
//
// Head sits at the overwriting txid, not at the folded one: only a commit at txid 2 can make PIDX
// name txid 2 as an owner, and that commit advances head. A fixture that leaves head at 1 describes a
// state the commit path cannot produce, and reclaim withholds the delta rather than trusting an owner
// it cannot see.
#[tokio::test]
async fn folded_delta_dropped_when_shard_materialized() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3710);
	let root = root_with_watermarks(1, 1, 0);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_meta_head_key(database_branch_id),
						&encode_db_head(head(database_branch_id, 2))?,
					);
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 1),
						&encode_commit_row(commit(1))?,
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 1, 0),
						&encoded_delta(1)?,
					);
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 1),
						&vec![0xab; 16],
					);
					tx.informal().set(
						&keys::branch_compaction_fold_key(database_branch_id, 1),
						&encode_fold_index_entry(FoldIndexEntry {
							shard_ids: vec![0],
							versionstamp: [0; 16],
						})?,
					);
					// Page 1 was overwritten at txid 2, so PIDX points past txid 1 (folded).
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 1),
						&2_u64.to_be_bytes(),
					);

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(reclaim_txids(&snapshot.delta_reclaim_segments), vec![1]);

	Ok(())
}

// C6 #1 regression: a folded delta must NOT be dropped when no shard covers the coverage fold that still
// needs it. Page 1 changed at txid 1 (shard 0), a restore point pins txid 1, and the page was overwritten
// at txid 30 so PIDX no longer points at txid 1. Because no `SHARD/0` version is materialized in
// `[1, 1]`, a PIDX-veto-only reclaim would corrupt the pinned read; the materialization gate withholds
// `DELTA/1`.
#[tokio::test]
async fn folded_delta_retained_when_pin_fold_lacks_shard() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3711);
	let root = root_with_watermarks(1, 30, 0);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_meta_head_key(database_branch_id),
						&encode_db_head(head(database_branch_id, 30))?,
					);
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 1),
						&encode_commit_row(commit(1))?,
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 1, 0),
						&encoded_delta(1)?,
					);
					// The page was overwritten at txid 30, so PIDX points past txid 1 (folded) but no shard
					// version was ever materialized for shard 0.
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 1),
						&30_u64.to_be_bytes(),
					);

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[restore_point_pin(1)],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	assert!(
		!reclaim_txids(&snapshot.delta_reclaim_segments).contains(&1),
		"a folded delta with no shard covering its pinned coverage fold must be retained"
	);
	// The pinned txid 1 is a fold, so its commit metadata is also retained.
	assert!(!snapshot.commit_reclaim_txids.contains(&1));

	Ok(())
}

// C6: a folded delta at or below the cold watermark is reclaimable without a materialized FDB shard,
// because cold compaction has already published its pages (a read refills from the cold tier). No
// `SHARD` row or fold index entry is seeded here; the gate passes on the cold-watermark clause alone.
#[tokio::test]
async fn folded_delta_dropped_below_cold_watermark() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3712);
	let root = root_with_watermarks(1, 5, 3);

	let snapshot = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_meta_head_key(database_branch_id),
						&encode_db_head(head(database_branch_id, 5))?,
					);
					tx.informal().set(
						&keys::branch_commit_key(database_branch_id, 1),
						&encode_commit_row(commit(1))?,
					);
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 1, 0),
						&encoded_delta(1)?,
					);
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 1),
						&2_u64.to_be_bytes(),
					);

					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						None,
						0,
						None,
						Snapshot,
						1_000,
						&mut CompactionBatchBudget::fdb(),
						None,
						true,
					)
					.await
				}
			}
		})
		.await?;

	assert!(
		reclaim_txids(&snapshot.delta_reclaim_segments).contains(&1),
		"a folded delta at or below the cold watermark is recoverable from cold and reclaimable"
	);

	Ok(())
}

/// The sweep clears the history it just classified, inside the transaction that classified it. This
/// is the property the whole v2 drain rests on: there is no planned set to compare against, so the
/// `Serializable` reads plus `compare_and_clear` are the only fence, and a window that commits is a
/// window whose rows are gone.
#[tokio::test]
async fn sweep_commit_delta_chunk_clears_what_it_derives() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3730);
	let root = root_with_watermarks(1, 10, 0);

	let output = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branches_list_key(database_branch_id),
						&encode_database_branch_record(branch_record(database_branch_id, 1))?,
					);
					tx.informal().set(
						&keys::branch_compaction_root_key(database_branch_id),
						&encode_compaction_root(root)?,
					);
					for txid in 1..=3_u64 {
						tx.informal().set(
							&keys::branch_commit_key(database_branch_id, txid),
							&encode_commit_row(commit(txid as u8))?,
						);
					}

					crate::workflows::db_reclaimer::sweep_commit_delta_chunk_tx(
						&tx,
						&SweepCommitDeltaChunkInput {
							database_branch_id,
							base_lifecycle_generation: 1,
							base_manifest_generation: 1,
							commit_scan_cursor: 0,
							cursor_segment_pgno: None,
							bypass_admission: false,
						},
						1_000,
						CompactionThrottleClass::Reclaim
							.resolve_from(&rivet_config::config::Sqlite::default()),
					)
					.await
				}
			}
		})
		.await?;

	assert!(matches!(output.status, CompactionJobStatus::Succeeded));
	assert!(!output.throttled);
	// Every scanned txid is behind the cursor now, reclaimed or not, so a retained prefix cannot stall
	// the rows behind it.
	assert!(output.next_commit_scan_cursor > 3);
	assert!(output.commit_scan_complete);
	assert!(output.key_count > 0);

	// The clears committed with the derive, so nothing is left for a second pass to find.
	let remaining = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx_scan_range_values_limited(
					&tx,
					&keys::branch_commit_key(database_branch_id, 0),
					&keys::branch_commit_key(database_branch_id, u64::MAX),
					500,
					Snapshot,
				)
				.await
			},
		)
		.await?;
	assert!(remaining.is_empty());

	Ok(())
}

#[tokio::test]
async fn reclaim_expired_pitr_rows_capped_by_slice_budget() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3720);

	let (retained, expired) = db
		.txn("test_depotinline_workflows_compaction", move |tx| {
			async move {
				for bucket in 0..5_i64 {
					tx.informal().set(
						&keys::branch_pitr_interval_key(database_branch_id, bucket),
						&encode_pitr_interval_coverage(PitrIntervalCoverage {
							txid: bucket as u64 + 1,
							versionstamp: [bucket as u8; 16],
							wall_clock_ms: bucket,
							expires_at_ms: 500,
						})?,
					);
				}
				// A retained row sorting after every expired row must still be collected once the
				// expired budget is exhausted: it is reclaim coverage, not a delete candidate.
				tx.informal().set(
					&keys::branch_pitr_interval_key(database_branch_id, 100),
					&encode_pitr_interval_coverage(PitrIntervalCoverage {
						txid: 100,
						versionstamp: [9; 16],
						wall_clock_ms: 100,
						expires_at_ms: i64::MAX,
					})?,
				);

				let mut budget = CompactionBatchBudget::with_limits(3, u64::MAX);
				read_pitr_interval_reclaim_rows(
					&tx,
					database_branch_id,
					1_000,
					Snapshot,
					&mut budget,
				)
				.await
			}
		})
		.await?;

	// The expired set is the first three buckets in scan order; the tail drains in later slices.
	assert_eq!(
		expired
			.iter()
			.map(|(bucket, ..)| *bucket)
			.collect::<Vec<_>>(),
		vec![0, 1, 2]
	);
	assert_eq!(retained.len(), 1);
	assert_eq!(retained[0].coverage.txid, 100);

	Ok(())
}

#[tokio::test]
async fn dead_shard_chunk_budget_stop_resumes_before_unfinished_fold() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3721);

	db.txn("test_depotinline_workflows_compaction", move |tx| {
		async move {
			// Shard 0 versions at txids 2, 4, and 6 with matching folds. With no coverage, the
			// versions at 2 and 4 are dead (superseded by 4 and 6 respectively).
			for txid in [2_u64, 4, 6] {
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, txid),
					&encoded_delta(txid)?,
				);
				tx.informal().set(
					&keys::branch_compaction_fold_key(database_branch_id, txid),
					&encode_fold_index_entry(FoldIndexEntry {
						shard_ids: vec![0],
						versionstamp: commit(txid as u8).versionstamp,
					})?,
				);
			}

			// A one-key budget fits the first candidate and refuses the second, so the walk stops
			// with the cursor still before the unfinished fold.
			let mut budget = CompactionBatchBudget::with_limits(1, u64::MAX);
			let first = read_dead_shard_versions_chunk(
				&tx,
				database_branch_id,
				&[],
				&[],
				&DeadShardScanState::default(),
				Snapshot,
				&mut budget,
			)
			.await?;
			assert_eq!(first.candidates.len(), 1);
			assert_eq!(first.candidates[0].reference.shard_id, 0);
			assert_eq!(first.candidates[0].reference.as_of_txid, 2);
			assert_eq!(first.candidates[0].reference.superseded_by_txid, 4);
			assert!(first.has_more);
			assert_eq!(first.next_scan.fold_cursor, Some(4));
			assert_eq!(first.next_scan.prev.get(&0), Some(&4));

			// Resuming from the returned scan state with a fresh budget re-walks the unfinished
			// fold and finds the remaining dead version.
			let mut budget = CompactionBatchBudget::fdb();
			let second = read_dead_shard_versions_chunk(
				&tx,
				database_branch_id,
				&[],
				&[],
				&first.next_scan,
				Snapshot,
				&mut budget,
			)
			.await?;
			assert_eq!(second.candidates.len(), 1);
			assert_eq!(second.candidates[0].reference.as_of_txid, 4);
			assert_eq!(second.candidates[0].reference.superseded_by_txid, 6);
			assert!(!second.has_more);

			Ok(())
		}
	})
	.await?;

	Ok(())
}

/// The hot drain span these tests plan against. Mirrors the
/// `sqlite.compaction_max_hot_drain_span_txids` default rather than reading config, so a deployment
/// retuning the span does not change what the planning tests cover.
const TEST_MAX_HOT_DRAIN_SPAN_TXIDS: u64 = 512;

/// The manager reclaim interval these tests assert against. Mirrors the
/// `sqlite.manager_reclaim_interval_ms` default rather than reading config, so a deployment
/// retuning the interval does not change what the scheduling tests cover.
const TEST_RECLAIM_INTERVAL_MS: i64 = 10 * 60 * 1000;

fn wake_intervals(idle_poll_ms: i64) -> ManagerWakeIntervals {
	ManagerWakeIntervals {
		idle_poll_ms,
		reclaim_ms: TEST_RECLAIM_INTERVAL_MS,
	}
}

#[test]
fn manager_wake_falls_back_to_the_idle_poll_when_no_signal_arrives() {
	let database_branch_id = database_branch_id(0x4180);
	let input = manager_input(database_branch_id);
	let idle_poll_interval_ms = 12 * 60 * 60 * 1000;
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.next_reclaim_check_at_ms = Some(900);

	// Both planning timers fired and nothing signaled, which is how a branch settles after its last
	// write. Without the idle fallback both deadlines clear and the manager parks forever.
	schedule_next_wake(
		&mut state,
		&input,
		1_000,
		false,
		WakeTriggers {
			hot: false,
			reclaim: true,
		},
		wake_intervals(idle_poll_interval_ms),
	);

	let deadline = state
		.next_reclaim_check_at_ms
		.expect("idle manager must leave a wake deadline armed");
	assert!(
		deadline >= 1_000 + idle_poll_interval_ms - idle_poll_interval_ms / 10
			&& deadline <= 1_000 + idle_poll_interval_ms + idle_poll_interval_ms / 10,
		"idle wake {deadline} must land within +/-10% of the poll interval"
	);
}

#[test]
fn manager_wake_keeps_signal_intervals_and_pending_deadlines() {
	let database_branch_id = database_branch_id(0x4181);
	let input = manager_input(database_branch_id);
	let idle_poll_interval_ms = 12 * 60 * 60 * 1000;

	// A signaled iteration re-arms on the short planning intervals, not the idle poll.
	let mut signaled = DbManagerState::new(companion_workflow_ids());
	schedule_next_wake(
		&mut signaled,
		&input,
		1_000,
		true,
		WakeTriggers {
			hot: true,
			reclaim: false,
		},
		wake_intervals(idle_poll_interval_ms),
	);
	assert_eq!(
		signaled.next_reclaim_check_at_ms,
		Some(1_000 + TEST_RECLAIM_INTERVAL_MS)
	);

	// An unsignaled iteration with a deadline still pending leaves it alone.
	let mut pending = DbManagerState::new(companion_workflow_ids());
	pending.next_reclaim_check_at_ms = Some(5_000);
	schedule_next_wake(
		&mut pending,
		&input,
		1_000,
		false,
		WakeTriggers {
			hot: false,
			reclaim: false,
		},
		wake_intervals(idle_poll_interval_ms),
	);
	assert_eq!(pending.next_reclaim_check_at_ms, Some(5_000));
}

#[test]
fn manager_idle_poll_jitter_is_stable_per_branch_and_spread_across_branches() {
	let idle_poll_interval_ms = 12 * 60 * 60 * 1000;
	let triggers = WakeTriggers {
		hot: false,
		reclaim: true,
	};
	let idle_deadline = |branch_value: u128| {
		let branch_id = database_branch_id(branch_value);
		let input = manager_input(branch_id);
		let mut state = DbManagerState::new(companion_workflow_ids());
		schedule_next_wake(
			&mut state,
			&input,
			1_000,
			false,
			triggers,
			wake_intervals(idle_poll_interval_ms),
		);
		state.next_reclaim_check_at_ms.expect("deadline armed")
	};

	// Pure function of the branch id, so replays land on the same deadline.
	assert_eq!(idle_deadline(0x4182), idle_deadline(0x4182));
	// Different branches spread, so a restart does not re-align every manager into one window.
	assert_ne!(idle_deadline(0x4182), idle_deadline(0x4183));
}

#[cfg(feature = "test-faults")]
#[test]
fn manager_wake_arms_nothing_when_planning_timers_are_disabled() {
	let database_branch_id = database_branch_id(0x4184);
	let input = DbManagerInput::with_planning_timers_disabled(database_branch_id, None);
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.next_reclaim_check_at_ms = Some(900);

	schedule_next_wake(
		&mut state,
		&input,
		1_000,
		false,
		WakeTriggers::default(),
		wake_intervals(12 * 60 * 60 * 1000),
	);

	assert_eq!(state.next_reclaim_check_at_ms, None);
}

#[test]
fn manager_settling_after_its_last_write_reaches_the_idle_poll_instead_of_parking() {
	let database_branch_id = database_branch_id(0x4185);
	let input = manager_input(database_branch_id);
	let idle_poll_interval_ms = 12 * 60 * 60 * 1000;
	let mut state = DbManagerState::new(companion_workflow_ids());

	// The last commit's compaction settles: a signal arrives and arms both planning timers.
	let mut now_ms = 1_000;
	schedule_next_wake(
		&mut state,
		&input,
		now_ms,
		true,
		WakeTriggers {
			hot: true,
			reclaim: false,
		},
		wake_intervals(idle_poll_interval_ms),
	);

	// Nothing writes to the branch again, so every following iteration fires on a timer with no
	// signal. Each fired timer clears itself and is not re-armed.
	while let Some(deadline) = state.next_reclaim_check_at_ms {
		assert!(
			deadline > now_ms,
			"the manager must not busy-loop on an already-passed deadline"
		);
		now_ms = deadline;

		let triggers = WakeTriggers {
			hot: false,
			reclaim: state.next_reclaim_check_at_ms.is_some_and(|d| now_ms >= d),
		};
		schedule_next_wake(
			&mut state,
			&input,
			now_ms,
			false,
			triggers,
			wake_intervals(idle_poll_interval_ms),
		);

		// This is the park: before the idle fallback, draining both timers left the manager
		// listening with no deadline and nothing but a commit could ever wake it again.
		assert!(
			state.next_reclaim_check_at_ms.is_some(),
			"manager parked with no wake deadline at {now_ms}"
		);

		if state
			.next_reclaim_check_at_ms
			.is_some_and(|d| d > now_ms + TEST_RECLAIM_INTERVAL_MS)
		{
			// Reached the idle poll, which is the terminal state for a branch nobody writes to.
			return;
		}
	}

	panic!("manager never fell back to the idle poll");
}

/// A complete shard image holding exactly `pgnos`, shaped the way a fold writes one.
fn shard_image(as_of_txid: u64, pgnos: &[u32]) -> Result<Vec<u8>> {
	let pages = pgnos
		.iter()
		.map(|pgno| DirtyPage {
			pgno: *pgno,
			bytes: vec![*pgno as u8; keys::PAGE_SIZE as usize],
		})
		.collect::<Vec<_>>();
	let commit = pgnos.iter().copied().max().unwrap_or(1);

	encode_ltx_v3(
		LtxHeader::delta(as_of_txid, commit, as_of_txid as i64),
		&pages,
	)
}

fn page_update(pgno: u32) -> (u32, Vec<u8>) {
	(pgno, vec![pgno as u8; keys::PAGE_SIZE as usize])
}

/// A fold onto a shard that already has a published version must carry that version's pages
/// forward rather than staging a sparse image of only the pages this fold touched.
#[tokio::test]
async fn hot_fold_merges_onto_a_published_shard_in_the_hot_tier() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x60d3);

	let blob = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, 4),
					&shard_image(4, &[1, 2])?,
				);
				build_staged_hot_shard_blob(
					&tx,
					database_branch_id,
					Id::new_v1(9),
					0,
					10,
					vec![page_update(3)],
				)
				.await
			},
		)
		.await?;

	let StagedHotShardBlob::Encoded(blob) = blob;
	let pgnos = decode_ltx_v3(&blob)?
		.pages
		.iter()
		.map(|page| page.pgno)
		.collect::<Vec<_>>();
	assert_eq!(
		pgnos,
		vec![1, 2, 3],
		"the fold must carry the merge base's pages forward"
	);

	Ok(())
}

/// A shard whose first version this fold is writing has no merge base anywhere, which is the normal
/// case for a growing database and must stay allowed.
#[tokio::test]
async fn hot_fold_allows_a_first_version_with_no_merge_base() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x60d4);

	let blob = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				build_staged_hot_shard_blob(
					&tx,
					database_branch_id,
					Id::new_v1(9),
					0,
					10,
					vec![page_update(3)],
				)
				.await
			},
		)
		.await?;

	let StagedHotShardBlob::Encoded(blob) = blob;
	let pgnos = decode_ltx_v3(&blob)?
		.pages
		.iter()
		.map(|page| page.pgno)
		.collect::<Vec<_>>();
	assert_eq!(pgnos, vec![3]);

	Ok(())
}

/// The state every long-lived database ends up in: a page written once, folded into no image because
/// its owner txid sits below the slice window that first folded the shard, and kept readable only by
/// its PIDX row. The shard keeps accumulating versions that omit that page, so a coverage check that
/// only asks whether a version exists in range would clear the row and leave the page unreachable.
/// The read path zero-fills an in-range page with no source, so that is silent corruption.
#[tokio::test]
async fn stale_pidx_chunk_retains_a_row_whose_page_no_shard_version_carries() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3612);
	let root = root_with_watermarks(1, 14220, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					// Pages 2 and 3 were written at txid 2 and never rewritten.
					for pgno in [2, 3] {
						tx.informal().set(
							&keys::branch_pidx_key(database_branch_id, pgno),
							&2_u64.to_be_bytes(),
						);
					}
					// Every later fold of shard 0 carries the pages the slice window touched and
					// omits 2 and 3, exactly like the folds a live branch accumulates.
					for as_of_txid in [6232, 9000, 14220] {
						tx.informal().set(
							&keys::branch_shard_key(database_branch_id, 0, as_of_txid),
							&shard_image(as_of_txid, &[1, 4, 5, 6])?,
						);
					}

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	assert!(
		chunk.candidates.is_empty(),
		"pages 2 and 3 are in no shard image, so their rows are the only pointer to them: {:?}",
		chunk
			.candidates
			.iter()
			.map(|(key, _)| key.clone())
			.collect::<Vec<_>>()
	);

	Ok(())
}

/// A page the fold did materialize is clearable, and the memoized probe must not leak one page's
/// answer onto another page of the same shard and owner txid.
#[tokio::test]
async fn stale_pidx_chunk_clears_only_the_pages_a_shard_version_carries() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3613);
	let root = root_with_watermarks(1, 100, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					for pgno in [1, 2, 3] {
						tx.informal().set(
							&keys::branch_pidx_key(database_branch_id, pgno),
							&28_u64.to_be_bytes(),
						);
					}
					tx.informal().set(
						&keys::branch_delta_chunk_key(database_branch_id, 28, 0),
						&shard_image(28, &[1, 2, 3])?,
					);
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 90),
						&shard_image(90, &[1, 3])?,
					);

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	let cleared = chunk
		.candidates
		.iter()
		.map(|(key, _)| decode_branch_pidx_pgno(database_branch_id, key))
		.collect::<Result<Vec<_>>>()?;
	assert_eq!(cleared, vec![1, 3]);

	Ok(())
}

/// Coverage has to be judged against the version reads resolve through, which is the newest one at or
/// below the cap, not any version in range. A later sparse fold wins the read and zero-fills whatever
/// it omits, so an older image still carrying the page does not make the row safe to clear.
#[tokio::test]
async fn stale_pidx_chunk_retains_a_row_the_newest_shard_version_dropped() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3614);
	let root = root_with_watermarks(1, 100, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 2),
						&28_u64.to_be_bytes(),
					);
					// The fold that absorbed the page, superseded by one that dropped it.
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 40),
						&shard_image(40, &[1, 2])?,
					);
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 90),
						&shard_image(90, &[1])?,
					);

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	assert!(
		chunk.candidates.is_empty(),
		"the newest image dropped page 2, so its PIDX row is the only pointer left"
	);

	Ok(())
}

/// A shard version older than the write cannot be the fold that absorbed it, even when it happens to
/// carry that page number from an earlier life of the page.
#[tokio::test]
async fn stale_pidx_chunk_retains_a_row_older_than_every_shard_version() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3615);
	let root = root_with_watermarks(1, 100, 0);

	let chunk = db
		.txn("test_depotinline_workflows_compaction", {
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					tx.informal().set(
						&keys::branch_pidx_key(database_branch_id, 2),
						&28_u64.to_be_bytes(),
					);
					tx.informal().set(
						&keys::branch_shard_key(database_branch_id, 0, 27),
						&shard_image(27, &[1, 2])?,
					);

					read_stale_pidx_chunk(
						&tx,
						database_branch_id,
						&root,
						None,
						Snapshot,
						&mut CompactionBatchBudget::fdb(),
					)
					.await
				}
			}
		})
		.await?;

	assert!(chunk.candidates.is_empty());

	Ok(())
}

/// The safety property for direct-to-shard folds: the orphan-staging cleanup lane clears a finished
/// job's ref rows and never deletes anything from `SHARD`.
///
/// This lane runs on successful jobs too, so under direct folds the image a ref names may be live
/// data. Nothing reachable from here can tell that from an abandoned job's scratch, so the lane must
/// not try. A regression that reintroduces a `SHARD` delete here loses customer pages silently, which
/// is why this asserts on the shard rows rather than only on the refs.
#[tokio::test]
async fn repair_fdb_cleanup_never_deletes_shard_versions() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3410);
	let stale_job_id = Id::new_v1(36);
	let shard_blob_bytes = vec![9_u8; 128];
	// Trailing args are (min_txid, shard_id, as_of_txid), matching the ref written below.
	let ref_key =
		keys::branch_compaction_stage_hot_ref_key(database_branch_id, stale_job_id, 1, 0, 1);
	let input_range =
		repair_reclaim_input_range(vec![stale_job_id], Vec::new(), std::iter::once(1));
	let input = ReclaimFdbJobInput {
		database_branch_id,
		job_id: Id::new_v1(37),
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 1,
		base_manifest_generation: 1,
		input_fingerprint: fingerprint_repair_reclaim_range(database_branch_id, &input_range),
		input_range,
		bypass_admission: false,
	};

	db.txn("test_depotinline_direct_fold_cleanup", {
		let input = input.clone();
		let ref_key = ref_key.clone();
		let shard_blob_bytes = shard_blob_bytes.clone();
		move |tx| {
			let input = input.clone();
			let ref_key = ref_key.clone();
			let shard_blob_bytes = shard_blob_bytes.clone();
			async move {
				tx.informal().set(
					&keys::branches_list_key(database_branch_id),
					&encode_database_branch_record(branch_record(database_branch_id, 1))?,
				);
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root(1))?,
				);
				// The image sits in the live tier, where a direct fold puts it. No blob is written
				// under the job's staging subspace at all.
				shard_blob::write_shard_blob(&tx, database_branch_id, 0, 1, &shard_blob_bytes)?;
				tx.informal().set(
					&ref_key,
					&encode_staged_hot_shard_ref(StagedHotShardRef {
						shard_id: 0,
						as_of_txid: 1,
						min_txid: 1,
						size_bytes: shard_blob_bytes.len() as u64,
						content_hash: content_hash(&shard_blob_bytes),
					})?,
				);

				cleanup_repair_fdb_outputs_tx(&tx, &input, rivet_util::timestamp::now()).await
			}
		}
	})
	.await?;

	let version_rows = db
		.txn(
			"test_depotinline_direct_fold_cleanup_read",
			move |tx| async move {
				let (begin, end) = keys::branch_shard_version_range(database_branch_id, 0, 1);
				tx_scan_range_values_limited(&tx, &begin, &end, 16, Snapshot).await
			},
		)
		.await?;

	assert!(
		!version_rows.is_empty(),
		"cleanup must leave the shard version alone; it cannot tell live data from scratch"
	);
	assert!(
		read_raw_key(&db, &ref_key).await?.is_none(),
		"cleanup must still clear the finished job's ref row"
	);
	Ok(())
}

/// The drain head must be a function of the watermark, not of where the live head happened to be.
///
/// Slice boundaries are otherwise already reproducible: selection breaks where a constant
/// `CompactionBatchBudget::fdb()` runs out over rows above the watermark, which reclaim cannot touch.
/// The head was the one input that tracked something mutable, so it was the only way two drains from
/// the same watermark could pick different boundaries and strand an abandoned job's images at a txid
/// nothing revisits.
#[test]
fn snapped_drain_head_is_stable_while_the_live_head_moves() {
	let watermark = 1_000_u64;
	let grain = HOT_DRAIN_HEAD_GRAIN_TXIDS;

	// Every head inside one grain window snaps to the same drain head, so a retry after the head
	// advanced still folds to the same boundary and overwrites its predecessor's keys.
	let base = snapped_drain_head_txid(watermark, watermark + grain, TEST_MAX_HOT_DRAIN_SPAN_TXIDS);
	assert_eq!(base, watermark + grain);
	for extra in 0..grain {
		assert_eq!(
			snapped_drain_head_txid(
				watermark,
				watermark + grain + extra,
				TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
			),
			base,
			"a head {extra} txids past the grain point must not move the drain head"
		);
	}

	// Crossing the next grain point is allowed to advance it, by exactly one grain.
	assert_eq!(
		snapped_drain_head_txid(
			watermark,
			watermark + 2 * grain,
			TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
		),
		watermark + 2 * grain
	);
}

/// A backlog that has not reached the first grain point snaps back to the watermark.
///
/// Production does not reach this through the unforced path: `hot_lag < COMPACTION_DELTA_THRESHOLD`
/// (128) declines the job first, and the grain is far below that. The cost the grain actually adds is
/// the up-to-`HOT_DRAIN_HEAD_GRAIN_TXIDS - 1` txids left unfolded on a branch that drains and then
/// goes idle, where the delta threshold would previously have been crossed and now is not.
#[test]
fn snapped_drain_head_defers_a_sub_grain_backlog() {
	let watermark = 1_000_u64;
	for lag in 1..HOT_DRAIN_HEAD_GRAIN_TXIDS {
		assert_eq!(
			snapped_drain_head_txid(watermark, watermark + lag, TEST_MAX_HOT_DRAIN_SPAN_TXIDS),
			watermark,
			"a {lag}-txid backlog must snap back to the watermark and plan no job"
		);
	}
}

/// The drain window still bounds a large backlog, and the cap itself lands on a grain point so the
/// bounded case is reproducible too.
#[test]
fn snapped_drain_head_respects_the_drain_window() {
	let watermark = 1_000_u64;
	let snapped = snapped_drain_head_txid(
		watermark,
		watermark + TEST_MAX_HOT_DRAIN_SPAN_TXIDS * 4,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	);
	assert!(snapped <= watermark + TEST_MAX_HOT_DRAIN_SPAN_TXIDS);
	assert_eq!((snapped - watermark) % HOT_DRAIN_HEAD_GRAIN_TXIDS, 0);
}

/// A drain that exists to reach a pin must still pick a reproducible head.
///
/// This is the case a grid snap alone gets wrong. Taking the live head whenever a pin is in the
/// window puts the final slice's boundary back on a moving target, and that boundary is itself a
/// coverage txid, so an abandoned pin-covering drain strands an image at exactly the unstable txid
/// the grid was introduced to eliminate. Raising the head to the pin instead keeps both properties:
/// the drain reaches the pin, and the head does not move when the live head does.
#[test]
fn drain_head_is_reproducible_while_covering_a_pin() {
	let watermark = 1_000_u64;
	let pin = watermark + 5;

	let first = plan_drain_head_txid(
		watermark,
		watermark + 10,
		Some(pin),
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	);
	let after_more_writes = plan_drain_head_txid(
		watermark,
		watermark + 12,
		Some(pin),
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	);
	assert_eq!(
		first, after_more_writes,
		"a moving live head must not move the drain head while a pin is in the window"
	);
	assert!(
		first >= pin,
		"the drain must still reach the pin it was planned for"
	);
}

/// A pin above the next grid point raises the head to the pin; a pin below it leaves the grid point
/// alone. Both are stable inputs, so both are reproducible.
#[test]
fn drain_head_takes_the_higher_of_grid_and_pin() {
	let watermark = 1_000_u64;
	let grain = HOT_DRAIN_HEAD_GRAIN_TXIDS;

	// Pin below the grid point: the grid point wins and still covers the pin.
	let below = plan_drain_head_txid(
		watermark,
		watermark + grain + 4,
		Some(watermark + 3),
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	);
	assert_eq!(below, watermark + grain);

	// Pin above it: the head rises to the pin so the drain reaches it.
	let above = plan_drain_head_txid(
		watermark,
		watermark + grain + 4,
		Some(watermark + grain + 2),
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	);
	assert_eq!(above, watermark + grain + 2);
}

/// `force` is the one case that keeps the live head, because it means "compact what is there now".
#[test]
fn forced_drain_head_keeps_the_live_head() {
	let watermark = 1_000_u64;
	assert_eq!(
		plan_drain_head_txid(
			watermark,
			watermark + 3,
			None,
			true,
			TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
		),
		watermark + 3
	);
}

fn pin_at(database_branch_id: DatabaseBranchId, at_txid: u64) -> DbHistoryPin {
	DbHistoryPin {
		at_versionstamp: [0; 16],
		at_txid,
		kind: DbHistoryPinKind::RestorePoint,
		owner_database_branch_id: Some(database_branch_id),
		owner_bucket_branch_id: None,
		owner_restore_point: None,
		created_at_ms: 0,
	}
}

/// The reproducibility property through the real planner, not just the head arithmetic.
///
/// `plan_drain_head_txid` is unit-tested on hand-fed numbers, but the input that matters -- the
/// highest stable coverage txid -- is derived inside `plan_hot_job` from the pin set and the PITR
/// selection, and it has to exclude the head-derived `selected_max_txid`. This drives that derivation
/// with a real pin and an unforced plan, then moves the head and re-plans.
///
/// Unforced is the point: a forced drain takes the live head by design, so it would pass this
/// vacuously.
#[test]
fn hot_planning_drain_head_is_reproducible_across_a_moving_head() {
	let database_branch_id = database_branch_id(0x3620);
	let hot_watermark_txid = 0_u64;
	let pin_txid = 5_u64;
	let hot_inputs = |selected_max_txid: u64| HotInputSnapshot {
		oversized_commit_txid: None,
		selected_max_pgno_exclusive: None,
		commits: vec![(selected_max_txid, commit(selected_max_txid as u8))],
		delta_chunks: Vec::new(),
		pidx_entries: Vec::new(),
		pitr_interval_coverage: Vec::new(),
		total_value_bytes: 8,
		selected_max_txid: Some(selected_max_txid),
	};
	let snapshot_at = |head_txid: u64| ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head(database_branch_id, head_txid)),
		root: root_with_watermarks(7, hot_watermark_txid, 0),
		dirty: None,
		// A pin below the first grain point. It is what lets an unforced plan run at all here, and
		// it is the coverage txid the drain has to reach.
		db_pins: vec![pin_at(database_branch_id, pin_txid)],
		hot_inputs: hot_inputs(head_txid),
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	// Two plans from the same watermark, with the live head in different places inside one grain
	// window. Both are unforced.
	let first = plan_hot_job(
		database_branch_id,
		&snapshot_at(10),
		Id::new_v1(3620),
		1_000,
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("a pin in the window should admit an unforced hot job");
	let after_more_writes = plan_hot_job(
		database_branch_id,
		&snapshot_at(12),
		Id::new_v1(3621),
		1_001,
		false,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("a pin in the window should admit an unforced hot job");

	assert_eq!(
		first.drain_head_txid, after_more_writes.drain_head_txid,
		"a moving live head must not move the drain head, or an abandoned job's successor \
		 folds different keys and strands its images"
	);
	assert!(
		first.drain_head_txid >= pin_txid,
		"the drain must still reach the pin that admitted it"
	);
}

/// The forced path is the documented exception: it takes the live head, so its boundary moves.
/// Pinned here so the exception stays deliberate rather than becoming an accident.
#[test]
fn forced_hot_planning_drain_head_follows_the_live_head() {
	let database_branch_id = database_branch_id(0x3621);
	let hot_inputs = |selected_max_txid: u64| HotInputSnapshot {
		oversized_commit_txid: None,
		selected_max_pgno_exclusive: None,
		commits: vec![(selected_max_txid, commit(selected_max_txid as u8))],
		delta_chunks: Vec::new(),
		pidx_entries: Vec::new(),
		pitr_interval_coverage: Vec::new(),
		total_value_bytes: 8,
		selected_max_txid: Some(selected_max_txid),
	};
	let snapshot_at = |head_txid: u64| ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head(database_branch_id, head_txid)),
		root: root_with_watermarks(7, 0, 0),
		dirty: None,
		db_pins: Vec::new(),
		hot_inputs: hot_inputs(head_txid),
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	let first = plan_hot_job(
		database_branch_id,
		&snapshot_at(10),
		Id::new_v1(3622),
		1_000,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("a forced hot job should be planned");
	let second = plan_hot_job(
		database_branch_id,
		&snapshot_at(12),
		Id::new_v1(3623),
		1_001,
		true,
		TEST_MAX_HOT_DRAIN_SPAN_TXIDS,
	)
	.expect("a forced hot job should be planned");

	assert_eq!(first.drain_head_txid, 10);
	assert_eq!(
		second.drain_head_txid, 12,
		"forced drains follow the live head, which is why an abandoned forced drain can strand \
		 images no later drain revisits"
	);
}

#[test]
fn db_size_pages_at_txid_takes_the_newest_commit_at_or_below() {
	let entries = [(1_u64, 4_u32), (3, 2), (7, 9)];

	assert_eq!(db_size_pages_at_txid(&entries, 1), Some(4));
	// Between two commits the size is still the older one's.
	assert_eq!(db_size_pages_at_txid(&entries, 2), Some(4));
	assert_eq!(db_size_pages_at_txid(&entries, 3), Some(2));
	assert_eq!(db_size_pages_at_txid(&entries, 6), Some(2));
	assert_eq!(db_size_pages_at_txid(&entries, 9), Some(9));
	// Nothing at or below the txid means no commit records a size for it.
	assert_eq!(db_size_pages_at_txid(&entries, 0), None);
	assert_eq!(db_size_pages_at_txid(&[], 5), None);
}

#[test]
fn a_fold_keeps_pages_a_later_shrink_drops() -> Result<()> {
	// txid 1 writes four pages, txid 2 shrinks the database to two. Folding the coverage txid 1
	// against the size at the slice maximum would drop pages 3 and 4 from txid 1's image even
	// though they are live at that txid, and a read pinned there would resolve them to zeros.
	let decoded_ltx = |txid: u64, pgnos: &[u32]| DecodedLtx {
		header: LtxHeader::delta(txid, 1, txid as i64),
		page_index: Vec::new(),
		pages: pgnos
			.iter()
			.map(|pgno| DirtyPage {
				pgno: *pgno,
				bytes: vec![txid as u8; keys::PAGE_SIZE as usize],
			})
			.collect(),
	};
	let deltas = std::collections::BTreeMap::from([
		(1_u64, decoded_ltx(1, &[1, 2, 3, 4])),
		(2_u64, decoded_ltx(2, &[1])),
	]);
	let db_size_pages_by_txid = [(1_u64, 4_u32), (2, 2)];

	let at_pin = collect_hot_pages_by_shard(
		db_size_pages_at_txid(&db_size_pages_by_txid, 1).context("size at txid 1")?,
		&deltas,
		1,
		None,
	)?;
	let pgnos_at_pin = at_pin
		.values()
		.flatten()
		.map(|(pgno, _)| *pgno)
		.collect::<Vec<_>>();
	assert_eq!(
		pgnos_at_pin,
		vec![1, 2, 3, 4],
		"the fold at the pinned txid must keep every page live at that txid"
	);

	// The slice maximum still folds against its own, smaller size.
	let at_max = collect_hot_pages_by_shard(
		db_size_pages_at_txid(&db_size_pages_by_txid, 2).context("size at txid 2")?,
		&deltas,
		2,
		None,
	)?;
	let pgnos_at_max = at_max
		.values()
		.flatten()
		.map(|(pgno, _)| *pgno)
		.collect::<Vec<_>>();
	assert_eq!(pgnos_at_max, vec![1, 2]);

	Ok(())
}

#[test]
fn a_version_serves_every_retained_txid_up_to_the_next_version() {
	// Versions of one shard at txids 10 and 20. A read at any txid in [10, 20) resolves through the
	// version at 10, so a pin anywhere in that half-open interval retains it.
	let retained = std::collections::BTreeSet::from([15_u64]);

	assert!(
		shard_version_is_retained(&retained, 10, Some(20)),
		"a pin between two versions reads through the older one"
	);
	assert!(
		!shard_version_is_retained(&retained, 20, None),
		"the newer version starts above the pin, so the pin does not retain it"
	);

	// The interval is half open: a pin exactly at the superseding version reads through that one.
	let at_boundary = std::collections::BTreeSet::from([20_u64]);
	assert!(!shard_version_is_retained(&at_boundary, 10, Some(20)));
	assert!(shard_version_is_retained(&at_boundary, 20, Some(30)));

	// An exact match still retains, which is all the old rule could express.
	let exact = std::collections::BTreeSet::from([10_u64]);
	assert!(shard_version_is_retained(&exact, 10, Some(20)));

	// Nothing retained means nothing is held back.
	assert!(!shard_version_is_retained(
		&std::collections::BTreeSet::new(),
		10,
		Some(20)
	));

	// The newest version of a shard has no successor, so it serves every retained txid above it.
	assert!(shard_version_is_retained(&retained, 10, None));
	assert!(!shard_version_is_retained(&retained, 16, None));
}

/// A row the walk classified stale but could not confirm against a shard image carrying its page is
/// retained, and retaining one means the branch is not repaired. The marker is one-shot, so writing
/// it here would retire the sweep with that row still owning its page and nothing would ever revisit
/// it or the delta it pins.
#[tokio::test]
async fn stale_pidx_sweep_does_not_retire_a_branch_with_rows_it_could_not_confirm() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3615);

	let outcome = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branches_list_key(database_branch_id),
					&encode_database_branch_record(branch_record(database_branch_id, 1))?,
				);
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root_with_watermarks(1, 100, 0))?,
				);
				// Stale by owner txid, but the shard's newest image carries page 2, not page 1, so
				// clearing this row would drop the only pointer to page 1's contents.
				tx.informal().set(
					&keys::branch_pidx_key(database_branch_id, 1),
					&28_u64.to_be_bytes(),
				);
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, 90),
					&shard_image(90, &[2])?,
				);

				sweep_stale_pidx_chunk_tx(
					&tx,
					&SweepStalePidxInput {
						database_branch_id,
						base_lifecycle_generation: 1,
						base_manifest_generation: 1,
						pgno_cursor: None,
						retained_unconfirmed: false,
					},
					None,
				)
				.await
			},
		)
		.await?;

	assert!(matches!(
		outcome,
		StalePidxSweepOutcome::Continue {
			has_more: false,
			cleared: 0,
			retained_unconfirmed: true,
			..
		}
	));
	assert!(
		read_raw_key(
			&db,
			&keys::branch_compaction_pidx_repair_key(database_branch_id),
		)
		.await?
		.is_none(),
		"a walk that retained a row it could not confirm must leave the branch unrepaired"
	);

	Ok(())
}

/// Presence in the newest shard image is not enough to prove that a stale PIDX row is redundant.
/// A pre-fix hot pass could skip the owner commit while a later fold copied an older version of the
/// same page forward. Clearing the PIDX row in that state makes reads serve the stale shard bytes.
#[tokio::test]
async fn stale_pidx_sweep_retains_a_row_when_the_shard_has_older_page_bytes() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let bucket_id = Id::new_v1(0x3617);
	let database_id = "stale-pidx-content-mismatch";
	let pgno = 1;
	let owner_txid = 2_u64;
	let writer = Db::new(
		Arc::clone(&db),
		bucket_id,
		database_id.to_owned(),
		NodeId::new(),
	);
	let page = |pgno, fill| DirtyPage {
		pgno,
		bytes: vec![fill; keys::PAGE_SIZE as usize],
	};

	// A later fold copied the old page bytes forward even though the second commit still owns the
	// newer bytes through PIDX. This is the state left by the old oversized-commit compaction bug.
	writer.commit(vec![page(pgno, 0x01)], 1, 1_000).await?;
	writer.commit(vec![page(pgno, 0x1c)], 1, 2_000).await?;
	writer.commit(vec![page(2, 0x02)], 2, 3_000).await?;

	let database_branch_id = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				branch::resolve_database_branch(
					&tx,
					BucketId::from_gas_id(bucket_id),
					database_id,
					Snapshot,
				)
				.await?
				.context("database branch should exist")
			},
		)
		.await?;

	let outcome = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root_with_watermarks(1, 3, 0))?,
				);
				// The newest shard contains page 1 from the first commit, not its owner commit.
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, 3),
					&shard_image(3, &[pgno])?,
				);

				sweep_stale_pidx_chunk_tx(
					&tx,
					&SweepStalePidxInput {
						database_branch_id,
						base_lifecycle_generation: 0,
						base_manifest_generation: 1,
						pgno_cursor: None,
						retained_unconfirmed: false,
					},
					None,
				)
				.await
			},
		)
		.await?;

	// Use a fresh Db so the writer's PIDX cache cannot hide a cleared PIDX row. Before the
	// fix, the sweep clears PIDX and this read returns the stale 0x01 bytes from the shard.
	let reader = Db::new(
		Arc::clone(&db),
		bucket_id,
		database_id.to_owned(),
		NodeId::new(),
	);
	let pages = reader.get_pages(vec![pgno]).await?;
	assert_eq!(pages.len(), 1);
	assert_eq!(pages[0].pgno, pgno);
	let bytes = pages[0].bytes.as_deref().context("page should exist")?;
	assert_eq!(bytes.len(), keys::PAGE_SIZE as usize);
	assert!(
		bytes.iter().all(|byte| *byte == 0x1c),
		"the read must return the newer owner-delta bytes, not stale shard byte {:?}",
		bytes.first()
	);

	assert!(matches!(
		outcome,
		StalePidxSweepOutcome::Continue {
			has_more: false,
			cleared: 0,
			retained_unconfirmed: true,
			..
		}
	));
	assert_eq!(
		read_raw_key(&db, &keys::branch_pidx_key(database_branch_id, pgno),).await?,
		Some(owner_txid.to_be_bytes().to_vec()),
		"the owner delta must remain reachable while the shard has different bytes"
	);

	Ok(())
}

/// An earlier window's retention has to survive into the window that finishes the walk, which is
/// usually a different transaction and often a different activity call.
#[tokio::test]
async fn stale_pidx_sweep_carries_an_earlier_windows_retention_into_the_final_window() -> Result<()>
{
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3616);

	let outcome = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				tx.informal().set(
					&keys::branches_list_key(database_branch_id),
					&encode_database_branch_record(branch_record(database_branch_id, 1))?,
				);
				tx.informal().set(
					&keys::branch_compaction_root_key(database_branch_id),
					&encode_compaction_root(root_with_watermarks(1, 100, 0))?,
				);
				// This window on its own is clean: the image covers the page it clears.
				tx.informal().set(
					&keys::branch_pidx_key(database_branch_id, 1),
					&28_u64.to_be_bytes(),
				);
				tx.informal().set(
					&keys::branch_delta_chunk_key(database_branch_id, 28, 0),
					&shard_image(28, &[1])?,
				);
				tx.informal().set(
					&keys::branch_shard_key(database_branch_id, 0, 90),
					&shard_image(90, &[1])?,
				);

				sweep_stale_pidx_chunk_tx(
					&tx,
					&SweepStalePidxInput {
						database_branch_id,
						base_lifecycle_generation: 1,
						base_manifest_generation: 1,
						pgno_cursor: None,
						// An earlier window of this same walk retained a row.
						retained_unconfirmed: true,
					},
					None,
				)
				.await
			},
		)
		.await?;

	assert!(matches!(
		outcome,
		StalePidxSweepOutcome::Continue {
			has_more: false,
			cleared: 1,
			retained_unconfirmed: true,
			..
		}
	));
	assert!(
		read_raw_key(
			&db,
			&keys::branch_compaction_pidx_repair_key(database_branch_id),
		)
		.await?
		.is_none(),
		"the walk is only clean if every window was clean"
	);

	Ok(())
}

/// The sweep's durable loop state was widened from a bare cursor to a struct. Loop state is stored as
/// self-describing JSON and the enum is untagged, so state an older build persisted still resumes
/// instead of wedging the reclaimer.
#[test]
fn stale_pidx_sweep_state_reads_an_older_builds_bare_cursor() -> Result<()> {
	let legacy: crate::compaction::companion::StalePidxSweepState = serde_json::from_str("7")?;
	assert_eq!(legacy.pgno_cursor(), Some(7));
	assert!(!legacy.retained_unconfirmed());

	let legacy_start: crate::compaction::companion::StalePidxSweepState =
		serde_json::from_str("null")?;
	assert_eq!(legacy_start.pgno_cursor(), None);

	// What this build writes round-trips as the widened form.
	let current = crate::compaction::companion::StalePidxSweepState::Walk {
		pgno_cursor: Some(9),
		retained_unconfirmed: true,
	};
	let round_tripped: crate::compaction::companion::StalePidxSweepState =
		serde_json::from_str(&serde_json::to_string(&current)?)?;
	assert_eq!(round_tripped.pgno_cursor(), Some(9));
	assert!(round_tripped.retained_unconfirmed());

	Ok(())
}

/// A rejection is deterministic for as long as the state that produced it holds. If the request keeps
/// forcing that kind, every refresh plans the same job, it rejects again, and the request never
/// settles, so nothing ever reports the rejection to the caller. Drop the kind from the forced set
/// instead, which lets the request complete carrying the reason.
#[test]
fn force_compaction_tracker_stops_forcing_a_kind_that_was_rejected() {
	let database_branch_id = database_branch_id(0x4301);
	let request_id = Id::new_v1(0x4302);
	let job_id = Id::new_v1(0x4303);
	let request = ForceCompaction {
		database_branch_id,
		request_id,
		requested_work: ForceCompactionWork {
			hot: true,
			cold: false,
			reclaim: false,
			final_settle: false,
		},
	};
	let active_jobs = ManagerActiveJobs::default();
	let mut tracker = ForceCompactionTracker::default();

	tracker.record_request(request, 100, &active_jobs);
	assert!(
		tracker.pending_work().hot,
		"the request forces hot until something comes back"
	);

	tracker.record_job_finished(
		CompactionJobKind::Hot,
		job_id,
		&CompactionJobStatus::Rejected {
			reason: "base manifest generation changed".to_string(),
		},
	);
	assert!(
		!tracker.pending_work().hot,
		"a rejected kind must stop being forced, or the next refresh re-plans the same job"
	);

	// With nothing left to force, the request settles and reports why.
	tracker.complete_ready_requests(&active_jobs, &refresh_without_planned_work(), 101);
	assert!(tracker.pending_requests.is_empty());
	assert_eq!(tracker.recent_results.len(), 1);
	let result = &tracker.recent_results[0];
	assert_eq!(result.request_id, request_id);
	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
	assert!(
		result
			.skipped_noop_reasons
			.iter()
			.any(|reason| reason.contains("base manifest generation changed")),
		"the settled result must carry the rejection reason: {:?}",
		result.skipped_noop_reasons
	);
}

/// A rejection of one kind must not stop the request forcing the kinds it also asked for.
#[test]
fn force_compaction_tracker_keeps_forcing_the_kinds_that_did_not_reject() {
	let database_branch_id = database_branch_id(0x4304);
	let request = ForceCompaction {
		database_branch_id,
		request_id: Id::new_v1(0x4305),
		requested_work: ForceCompactionWork {
			hot: true,
			cold: true,
			reclaim: true,
			final_settle: false,
		},
	};
	let active_jobs = ManagerActiveJobs::default();
	let mut tracker = ForceCompactionTracker::default();

	tracker.record_request(request, 100, &active_jobs);
	tracker.record_job_finished(
		CompactionJobKind::Hot,
		Id::new_v1(0x4306),
		&CompactionJobStatus::Rejected {
			reason: "stale base generation".to_string(),
		},
	);

	let forced = tracker.pending_work();
	assert!(!forced.hot);
	assert!(forced.reclaim);
}

/// The distinct txids covered by a classification's reclaimable segments.
///
/// Reclaim classifies individual delta segments; these tests seed legacy single-blob commits, so
/// each txid contributes exactly one segment and the txid list stays the natural assertion.
fn reclaim_txids(segments: &[crate::compaction::types::DeltaSegmentRef]) -> Vec<u64> {
	let mut txids = segments
		.iter()
		.map(|segment| segment.txid)
		.collect::<Vec<_>>();
	txids.dedup();
	txids
}

/// A commit whose segments do not all fit one window must be admitted across passes rather than
/// stalling the sweep. Before per-segment admission a commit larger than the batch budget was stepped
/// over and its delta stranded, because reclaim finds history by scanning `COMMITS` and never
/// revisits a txid it has passed.
#[tokio::test]
async fn reclaim_commit_window_resumes_inside_a_segmented_txid() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3708);
	let root = root_with_watermarks(1, 1, 0);
	let shard = keys::SHARD_SIZE;
	// Three segments, each on its own shard run, so they are three independently admissible units.
	let max_shards = crate::conveyer::constants::COMMIT_SEGMENT_MAX_SHARDS;
	let segment_pgnos = [0, shard * max_shards, shard * max_shards * 2];

	let read_window = |commit_scan_cursor: u64, cursor_segment_pgno: Option<u32>| {
		let db = db.clone();
		let root = root.clone();
		async move {
			db.txn("test_depotinline_workflows_compaction", move |tx| {
				let root = root.clone();
				async move {
					for (index, first_pgno) in segment_pgnos.iter().enumerate() {
						if index == 0 {
							tx.informal().set(
								&keys::branch_commit_key(database_branch_id, 1),
								&encode_commit_row(commit(1))?,
							);
						}
						tx.informal().set(
							&keys::branch_delta_segment_chunk_key(
								database_branch_id,
								1,
								*first_pgno,
								0,
							),
							&encoded_delta_on_page(1, first_pgno.saturating_add(1))?,
						);
					}

					crate::compaction::shared::read_commit_delta_reclaim_window(
						&tx,
						database_branch_id,
						&root,
						&[],
						&[],
						commit_scan_cursor,
						cursor_segment_pgno,
						Snapshot,
						// Room for the commit row plus one segment's chunk, so each pass takes exactly
						// one segment and has to stop on the next.
						&mut CompactionBatchBudget::with_limits(2, 64 * 1024),
						None,
					)
					.await
				}
			})
			.await
		}
	};

	// The first pass takes one segment and stops on the second, holding the commit cursor on the txid.
	let first = read_window(0, None).await?;
	assert_eq!(first.delta_chunks.len(), 1);
	assert_eq!(first.next_commit_scan_cursor, 1);
	assert_eq!(first.next_segment_pgno, Some(segment_pgnos[1]));
	assert!(!first.commit_scan_complete);

	// Resuming from that cursor picks up inside the same commit rather than restarting it.
	let second = read_window(first.next_commit_scan_cursor, first.next_segment_pgno).await?;
	assert_eq!(second.delta_chunks.len(), 1);
	assert_eq!(second.next_segment_pgno, Some(segment_pgnos[2]));

	// The last pass drains the txid and moves the cursor past it with no segment left to resume from.
	let third = read_window(second.next_commit_scan_cursor, second.next_segment_pgno).await?;
	assert_eq!(third.delta_chunks.len(), 1);
	assert_eq!(third.next_commit_scan_cursor, 2);
	assert_eq!(third.next_segment_pgno, None);

	Ok(())
}

/// The orphan scan must distinguish an abandoned staged commit from one that is still being written
/// or has already landed. Clearing either of those would destroy live data: a stage mid-write loses
/// the commit its actor is still assembling, and a finalized one has had its bytes become real
/// commit data whose quota must not be refunded.
#[tokio::test]
async fn orphan_commit_stage_scan_only_reports_abandoned_stages() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3710);
	let now_ms = 10 * crate::conveyer::constants::COMMIT_STAGE_ORPHAN_GRACE_MS;
	let head_txid = 5_u64;

	let orphans = db
		.txn(
			"test_depotinline_workflows_compaction",
			move |tx| async move {
				// Above head and long idle: the actor is gone.
				tx.informal().set(
					&keys::branch_commit_stage_key(database_branch_id, head_txid + 1),
					&crate::conveyer::types::encode_commit_stage_row(
						crate::conveyer::types::CommitStageRow {
							accounted_bytes: 4_096,
							segments: vec![crate::conveyer::types::StagedSegment::new(0, [1])?],
							generation: 0,
							started_at_ms: 0,
						},
					)?,
				);
				// Above head but recent: an actor is staging this right now.
				tx.informal().set(
					&keys::branch_commit_stage_key(database_branch_id, head_txid + 2),
					&crate::conveyer::types::encode_commit_stage_row(
						crate::conveyer::types::CommitStageRow {
							accounted_bytes: 4_096,
							segments: vec![crate::conveyer::types::StagedSegment::new(0, [1])?],
							generation: 0,
							started_at_ms: now_ms,
						},
					)?,
				);
				// At or below head: its commit already finalized, so its bytes are live.
				tx.informal().set(
					&keys::branch_commit_stage_key(database_branch_id, head_txid),
					&crate::conveyer::types::encode_commit_stage_row(
						crate::conveyer::types::CommitStageRow {
							accounted_bytes: 4_096,
							segments: vec![crate::conveyer::types::StagedSegment::new(0, [1])?],
							generation: 0,
							started_at_ms: 0,
						},
					)?,
				);

				crate::compaction::shared::scan_orphan_commit_stages(
					&tx,
					database_branch_id,
					head_txid,
					now_ms,
					16,
				)
				.await
			},
		)
		.await?;

	assert_eq!(orphans, vec![head_txid + 1]);

	Ok(())
}
