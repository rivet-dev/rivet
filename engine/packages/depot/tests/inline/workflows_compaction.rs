use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sha2::{Digest, Sha256};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

use crate::DELTA_OBJECT_CHUNK_BYTES;
use super::{
	ActiveColdCompactionJob, ActiveHotCompactionJob, ActiveReclaimCompactionJob, BranchStopState,
	ColdInputSnapshot, ColdJobFinished, ColdJobInputRange, ColdShardBlob, ColdShardRef,
	CompactionInputFingerprint, CompactionJobKind, CompactionJobStatus, CompactionRoot,
	CompanionWorkflowIds, DatabaseBranchId, DatabaseBranchRecord, DbManagerInput, DbManagerState,
	ForceCompaction, ForceCompactionTracker, ForceCompactionWork, HotInputSnapshot,
	HotJobFinished, HotJobInputRange, HotShardOutputRef, InstallHotJobInput, ManagerActiveJobs,
	ManagerEffect, ManagerFdbSnapshot, ManagerPlanningDeadlines,
	ManagerStopReason, PlannedColdCompactionJob, PlannedHotCompactionJob,
	PlannedReclaimCompactionJob, ReclaimFdbJobInput, ReclaimInputSnapshot, ReclaimJobFinished,
	ReclaimJobInputRange, RefreshManagerOutput, ShardCachePolicy, StageHotJobInput,
	StagedHotShardCleanupRef, TxidRange, cleanup_repair_fdb_outputs_tx,
	fingerprint_hot_inputs, fingerprint_reclaim_inputs, fingerprint_repair_reclaim_range,
	manager_effect_for_requested_stop, manager_effects_after_refresh,
	manager_effects_for_cold_job_finished, manager_effects_for_hot_job_finished,
	manager_effects_for_reclaim_job_finished, install_hot_job_tx, plan_cold_job, plan_hot_job,
	plan_orphan_cold_object_deletes_tx, load_staged_hot_shard_blob, read_hot_input_snapshot,
	read_reclaim_input_snapshot, reclaim_coverage_is_complete, reclaim_fdb_job_tx,
	repair_reclaim_input_range, write_staged_hot_shards,
};
use crate::conveyer::{
	Db, branch, keys,
	ltx::{LtxEncoder, LtxHeader, decode_ltx_v3},
	types::{
		BranchState, BucketBranchId, BucketId, CommitRow, DBHead, DeltaManifest,
		DeltaObjectMeta, DeltaObjectState, DeltaPageIndexEntry, DirtyPage, decode_db_head,
		encode_commit_row, encode_compaction_root, encode_database_branch_record,
		encode_db_head, encode_delta_manifest, encode_delta_object_meta,
		encode_delta_page_index_entry, PitrPolicy,
	},
};

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("depot-workflow-compaction-inline-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	Ok(universaldb::Database::new(Arc::new(driver)))
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
		#[cfg(debug_assertions)]
		generation: 0,
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

fn patterned_page(pgno: u32) -> Vec<u8> {
	let mut state = (pgno as u64)
		.wrapping_mul(0x9e37_79b9_7f4a_7c15)
		.wrapping_add(0xd1b5_4a32_d192_ed03);
	let mut bytes = vec![0; keys::PAGE_SIZE as usize];
	for byte in &mut bytes {
		state ^= state << 13;
		state ^= state >> 7;
		state ^= state << 17;
		*byte = (state >> 32) as u8;
	}
	bytes
}

fn patterned_dirty_page(pgno: u32) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: patterned_page(pgno),
	}
}

fn patterned_dirty_pages(count: u32) -> Vec<DirtyPage> {
	(1..=count).map(patterned_dirty_page).collect()
}

async fn read_test_branch_id(
	db: &universaldb::Database,
	bucket_id: Id,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let database_id = database_id.to_string();
	db.run(move |tx| {
		let database_id = database_id.clone();
		async move {
			branch::resolve_database_branch(
				&tx,
				BucketId::from_gas_id(bucket_id),
				&database_id,
				Snapshot,
			)
			.await?
			.ok_or_else(|| anyhow::anyhow!("database branch should exist"))
		}
	})
	.await
}

async fn read_test_head(
	db: &universaldb::Database,
	database_branch_id: DatabaseBranchId,
) -> Result<DBHead> {
	db.run(move |tx| async move {
		let bytes = tx
			.informal()
			.get(&keys::branch_meta_head_key(database_branch_id), Snapshot)
			.await?
			.ok_or_else(|| anyhow::anyhow!("database head should exist"))?;
		decode_db_head(&bytes)
	})
	.await
}

async fn read_test_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

fn planned_hot_job(
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	input_range: HotJobInputRange,
) -> PlannedHotCompactionJob {
	PlannedHotCompactionJob {
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

fn planned_cold_job(
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	input_range: ColdJobInputRange,
) -> PlannedColdCompactionJob {
	PlannedColdCompactionJob {
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
	CompanionWorkflowIds::new(Id::new_v1(4100), Id::new_v1(4101), Id::new_v1(4102))
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
		txid_refs: Vec::new(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		staged_hot_shards: Vec::new(),
		orphan_cold_objects: Vec::new(),
		max_keys: 10,
		max_bytes: 4096,
	}
}

struct SeededLargeDelta {
	object_id: Uuid,
	chunk_count: u32,
	pages: Vec<DirtyPage>,
}

fn complete_large_delta_rows(
	database_branch_id: DatabaseBranchId,
	txid: u64,
	pages: Vec<DirtyPage>,
) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, SeededLargeDelta)> {
	let object_id = Uuid::from_u128(0xfeed_0000_0000_0000_0000_0000_0000_0001);
	let stage_id = Uuid::from_u128(0xfeed_0000_0000_0000_0000_0000_0000_0002);
	let encoded = LtxEncoder::new(LtxHeader::delta(txid, 1, 1_000)).encode_with_index(&pages)?;
	let object_hash = Sha256::digest(&encoded.bytes);
	let mut object_hash_bytes = [0_u8; 32];
	object_hash_bytes.copy_from_slice(&object_hash);
	let chunks = encoded
		.bytes
		.chunks(DELTA_OBJECT_CHUNK_BYTES)
		.map(Vec::from)
		.collect::<Vec<_>>();
	let chunk_count = u32::try_from(chunks.len())?;
	let pages_by_pgno = pages
		.iter()
		.map(|page| (page.pgno, page.bytes.clone()))
		.collect::<BTreeMap<_, _>>();
	let db_size_pages = pages.iter().map(|page| page.pgno).max().unwrap_or(0);
	let txid_bytes = txid.to_be_bytes().to_vec();
	let versionstamp = [txid as u8; 16];
	let root = root_with_watermarks(1, txid, 0);
	let head = DBHead {
		head_txid: txid,
		db_size_pages,
		post_apply_checksum: 0,
		branch_id: database_branch_id,
		#[cfg(debug_assertions)]
		generation: 0,
	};
	let commit_row = CommitRow {
		wall_clock_ms: 1_234,
		versionstamp,
		db_size_pages,
		post_apply_checksum: 0,
	};
	let manifest = DeltaManifest {
		txid,
		object_id,
		chunk_count,
		encoded_len: encoded.bytes.len() as u64,
		object_hash: object_hash_bytes,
	};
	let object_meta = DeltaObjectMeta {
		object_id,
		stage_id,
		staged_txid: txid,
		chunk_count,
		encoded_len: encoded.bytes.len() as u64,
		object_hash: object_hash_bytes,
		state: DeltaObjectState::Committed { txid },
		created_at_ms: 1_000,
		expires_after_ms: 0,
	};

	let mut rows = vec![
		(
			keys::branches_list_key(database_branch_id),
			encode_database_branch_record(branch_record(database_branch_id, 1))?,
		),
		(
			keys::branch_compaction_root_key(database_branch_id),
			encode_compaction_root(root)?,
		),
		(
			keys::branch_meta_head_key(database_branch_id),
			encode_db_head(head)?,
		),
		(
			keys::branch_commit_key(database_branch_id, txid),
			encode_commit_row(commit_row)?,
		),
		(keys::branch_vtx_key(database_branch_id, versionstamp), txid_bytes.clone()),
		(
			keys::branch_delta_manifest_key(database_branch_id, txid),
			encode_delta_manifest(manifest)?,
		),
		(
			keys::branch_delta_object_ref_key(database_branch_id, object_id),
			txid_bytes,
		),
		(
			keys::branch_delta_object_meta_key(database_branch_id, object_id),
			encode_delta_object_meta(object_meta)?,
		),
		(
			keys::branch_shard_key(database_branch_id, 0, txid),
			encoded.bytes.clone(),
		),
	];
	for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
		rows.push((
			keys::branch_delta_object_chunk_key(database_branch_id, object_id, chunk_idx as u32),
			chunk,
		));
	}
	for page_index in encoded.page_index {
		let page_bytes = pages_by_pgno
			.get(&page_index.pgno)
			.ok_or_else(|| anyhow::anyhow!("missing encoded page {}", page_index.pgno))?;
		let page_hash = Sha256::digest(page_bytes);
		let mut page_hash_bytes = [0_u8; 32];
		page_hash_bytes.copy_from_slice(&page_hash);
		rows.push((
			keys::branch_delta_pageidx_key(database_branch_id, txid, page_index.pgno),
			encode_delta_page_index_entry(DeltaPageIndexEntry {
				txid,
				object_id,
				encoded_offset: page_index.offset,
				encoded_size: u32::try_from(page_index.size)?,
				page_hash: page_hash_bytes,
			})?,
		));
	}

	Ok((
		rows,
		SeededLargeDelta {
			object_id,
			chunk_count,
			pages,
		},
	))
}

fn refresh_without_planned_work() -> RefreshManagerOutput {
	RefreshManagerOutput {
		planning_deadlines: ManagerPlanningDeadlines::after_refresh(1_000),
		planned_hot_job: None,
		planned_cold_job: None,
		planned_reclaim_job: None,
		observed_dirty: None,
		head_txid: Some(4),
		branch_is_live: true,
		branch_lifecycle_generation: Some(1),
		db_pin_count: 0,
		reclaim_noop_reason: Some("reclaim:no-actionable-work".to_string()),
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
				cold: true,
				reclaim: false,
				final_settle: false,
			},
		},
		100,
		&ManagerActiveJobs::default(),
	);
	tracker.record_job_attempted(CompactionJobKind::Cold);
	tracker.record_job_finished(
		CompactionJobKind::Cold,
		job_id,
		&CompactionJobStatus::Failed {
			error: "cold upload failed".to_string(),
		},
	);
	tracker.complete_ready_requests(
		&ManagerActiveJobs::default(),
		&refresh_without_planned_work(),
		101,
	);

	let result = &tracker.recent_results[0];
	assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Cold]);
	assert_eq!(result.completed_job_ids, vec![job_id]);
	assert_eq!(
		result.terminal_error,
		Some("cold upload failed".to_string())
	);
}

#[test]
fn manager_effects_map_job_completion_signals_to_workflow_actions() {
	let database_branch_id = database_branch_id(0x4100);
	let input = manager_input(database_branch_id);
	let hot_job_id = Id::new_v1(4103);
	let cold_job_id = Id::new_v1(4104);
	let reclaim_job_id = Id::new_v1(4105);
	let hot_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		coverage_txids: vec![4],
		max_pages: 8,
		max_bytes: 1024,
	};
	let cold_range = ColdJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		min_versionstamp: [1; 16],
		max_versionstamp: [4; 16],
		max_bytes: 2048,
	};
	let reclaim_range = reclaim_range();
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.active_jobs.hot = Some(ActiveHotCompactionJob::from_planned(planned_hot_job(
		database_branch_id,
		hot_job_id,
		hot_range.clone(),
	)));
	state.active_jobs.cold = Some(ActiveColdCompactionJob::from_planned(planned_cold_job(
		database_branch_id,
		cold_job_id,
		cold_range.clone(),
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
			input_fingerprint: [3; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: Vec::new(),
		},
	);
	assert!(matches!(
		hot_effects.as_slice(),
		[ManagerEffect::InstallHotOutput { .. }]
	));

	let cold_effects = manager_effects_for_cold_job_finished(
		&mut state,
		&input,
		ColdJobFinished {
			database_branch_id,
			job_id: cold_job_id,
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: 11,
			input_fingerprint: [3; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: Vec::new(),
		},
	);
	assert!(matches!(
		cold_effects.as_slice(),
		[ManagerEffect::PublishColdOutput { .. }]
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
	);
	assert!(matches!(
		reclaim_effects.as_slice(),
		[ManagerEffect::FinishReclaimJob { .. }]
	));
}

#[test]
fn manager_effects_cover_stale_cold_cleanup_and_branch_stop() {
	let database_branch_id = database_branch_id(0x4101);
	let input = manager_input(database_branch_id);
	let mut state = DbManagerState::new(companion_workflow_ids());
	state.last_observed_branch_lifecycle_generation = Some(5);

	let cleanup_effects = manager_effects_for_cold_job_finished(
		&mut state,
		&input,
		ColdJobFinished {
			database_branch_id,
			job_id: Id::new_v1(4110),
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: 7,
			input_fingerprint: [8; 32],
			status: CompactionJobStatus::Succeeded,
			output_refs: Vec::new(),
		},
	);
	let [
		ManagerEffect::ScheduleUploadedColdOutputCleanup {
			base_lifecycle_generation,
			repair_action,
			..
		},
	] = cleanup_effects.as_slice()
	else {
		panic!("expected stale cold cleanup effect");
	};
	assert_eq!(*base_lifecycle_generation, Some(5));
	assert_eq!(*repair_action, "delete_stale_cold_output");

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
fn manager_refresh_effects_keep_cold_and_reclaim_mutually_exclusive() {
	let database_branch_id = database_branch_id(0x4102);
	let input = manager_input(database_branch_id);
	let state = DbManagerState::new(companion_workflow_ids());
	let refresh = RefreshManagerOutput {
		planning_deadlines: ManagerPlanningDeadlines::after_refresh(1_000),
		planned_hot_job: None,
		planned_cold_job: Some(planned_cold_job(
			database_branch_id,
			Id::new_v1(4120),
			ColdJobInputRange {
				txids: TxidRange {
					min_txid: 1,
					max_txid: 4,
				},
				min_versionstamp: [1; 16],
				max_versionstamp: [4; 16],
				max_bytes: 2048,
			},
		)),
		planned_reclaim_job: Some(planned_reclaim_job(
			database_branch_id,
			Id::new_v1(4121),
			reclaim_range(),
		)),
		observed_dirty: None,
		head_txid: Some(4),
		branch_is_live: true,
		branch_lifecycle_generation: Some(1),
		db_pin_count: 0,
		reclaim_noop_reason: None,
	};

	let effects = manager_effects_after_refresh(&state, &input, &refresh, 1_500);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunColdJob { .. }))
	);
	assert!(
		!effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::RunReclaimJob { .. }))
	);
	assert!(
		effects
			.iter()
			.any(|effect| matches!(effect, ManagerEffect::CompleteReadyForceCompactions { .. }))
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

	let effects = manager_effects_after_refresh(&state, &input, &refresh, 12_000);
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
		coverage_txids: vec![5, 10],
		max_pages: 8,
		max_bytes: 1024,
	};
	let cold_range = ColdJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 10,
		},
		min_versionstamp: [1; 16],
		max_versionstamp: [10; 16],
		max_bytes: 2048,
	};
	let reclaim_range = ReclaimJobInputRange {
		txids: TxidRange {
			min_txid: 1,
			max_txid: 4,
		},
		txid_refs: Vec::new(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		staged_hot_shards: Vec::new(),
		orphan_cold_objects: Vec::new(),
		max_keys: 10,
		max_bytes: 4096,
	};

	let mut active_jobs = ManagerActiveJobs {
		hot: Some(ActiveHotCompactionJob::from_planned(planned_hot_job(
			database_branch_id,
			Id::new_v1(3300),
			hot_range.clone(),
		))),
		cold: Some(ActiveColdCompactionJob::from_planned(planned_cold_job(
			database_branch_id,
			Id::new_v1(3301),
			cold_range.clone(),
		))),
		reclaim: Some(ActiveReclaimCompactionJob::from_planned(
			planned_reclaim_job(database_branch_id, Id::new_v1(3302), reclaim_range.clone()),
		)),
	};

	assert_eq!(
		active_jobs.hot.as_ref().unwrap().input_range.coverage_txids,
		vec![5, 10]
	);
	assert_eq!(
		active_jobs
			.cold
			.as_ref()
			.unwrap()
			.input_range
			.min_versionstamp,
		[1; 16]
	);
	assert_eq!(
		active_jobs.reclaim.as_ref().unwrap().input_range.max_keys,
		10
	);

	active_jobs.hot = None;
	assert!(active_jobs.hot.is_none());
	assert!(active_jobs.cold.is_some());
	assert!(active_jobs.reclaim.is_some());

	active_jobs.cold = None;
	assert!(active_jobs.cold.is_none());
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
		large_delta_manifests: Vec::new(),
		large_delta_pageidx_entries: Vec::new(),
		pidx_entries: vec![(b"pidx-key".to_vec(), b"pidx-value".to_vec())],
		pitr_interval_coverage: Vec::new(),
		total_value_bytes: 24,
	};
	let mut snapshot = ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head.clone()),
		root: root.clone(),
		dirty: None,
		db_pins: Vec::new(),
		hot_inputs,
		cold_inputs: ColdInputSnapshot::default(),
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	let first_job = plan_hot_job(database_branch_id, &snapshot, Id::new_v1(3600), 1_000, true)
		.expect("hot job should be planned");
	let second_job = plan_hot_job(database_branch_id, &snapshot, Id::new_v1(3601), 1_001, true)
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
	let changed_job = plan_hot_job(database_branch_id, &snapshot, Id::new_v1(3602), 1_002, true)
		.expect("hot job should be planned");
	assert_ne!(first_job.input_fingerprint, changed_job.input_fingerprint);
}

#[tokio::test]
async fn hot_staging_loads_large_delta_pages_from_chunked_object() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let bucket_id = Id::new_v1(4_600);
	let database_id = "hot-staging-large-delta";
	let database_db = Db::new(
		Arc::clone(&db),
		bucket_id,
		database_id.to_string(),
		NodeId::new(),
	);
	let dirty_pages = patterned_dirty_pages(2_049);
	let expected_page = dirty_pages[127].bytes.clone();

	database_db.commit(dirty_pages, 2_049, 1_000).await?;
	let database_branch_id = read_test_branch_id(&db, bucket_id, database_id).await?;
	let head = read_test_head(&db, database_branch_id).await?;
	let root = root_with_watermarks(0, 0, 0);
	let job_id = Id::new_v1(4_601);

	let (output_refs, input_range, input_fingerprint): (
		Vec<HotShardOutputRef>,
		HotJobInputRange,
		CompactionInputFingerprint,
	) = db
		.run({
			let head = head.clone();
			let root = root.clone();
			move |tx| {
				let head = head.clone();
				let root = root.clone();
				async move {
					let hot_inputs = read_hot_input_snapshot(
						&tx,
						database_branch_id,
						Some(&head),
						&root,
						Snapshot,
						PitrPolicy::default(),
						2_000,
					)
					.await?;
					assert!(hot_inputs.delta_chunks.is_empty());
					assert_eq!(hot_inputs.large_delta_manifests.len(), 1);
					assert!(hot_inputs.large_delta_pageidx_entries.len() < 2_049);
					assert!(hot_inputs.large_delta_pageidx_entries.len() >= 400);
					assert!(hot_inputs.large_delta_manifests[0].3.chunk_count > 1);

					let coverage_txids = vec![1];
					let input_fingerprint = fingerprint_hot_inputs(
						database_branch_id,
						&root,
						&head,
						&coverage_txids,
						&hot_inputs,
					);
					let input_range = HotJobInputRange {
						txids: TxidRange {
							min_txid: 1,
							max_txid: 1,
						},
						coverage_txids,
						max_pages: hot_inputs.large_delta_pageidx_entries.len() as u32,
						max_bytes: hot_inputs.total_value_bytes,
					};
					let input = StageHotJobInput {
						database_branch_id,
						job_id,
						job_kind: CompactionJobKind::Hot,
						base_lifecycle_generation: 0,
						base_manifest_generation: 0,
						input_fingerprint,
						input_range: input_range.clone(),
					};
					let output_refs = write_staged_hot_shards(&tx, &input, &head, &hot_inputs).await?;
					Ok((output_refs, input_range, input_fingerprint))
				}
			}
		})
		.await?;

	assert!(
		output_refs.len() > 1,
		"large delta should stage all touched shards"
	);
	let shard_two = output_refs
		.iter()
		.find(|output_ref| output_ref.shard_id == 2)
		.expect("page 128 should land in staged shard 2");
	let staged_blob = db
		.run({
			let shard_two = shard_two.clone();
			move |tx| {
				let shard_two = shard_two.clone();
				async move {
					load_staged_hot_shard_blob(&tx, database_branch_id, job_id, &shard_two, Snapshot)
						.await
				}
			}
		})
		.await?
		.expect("staged shard blob should exist");
	let decoded = decode_ltx_v3(&staged_blob)?;
	assert_eq!(decoded.get_page(128), Some(expected_page.as_slice()));

	let install_input = InstallHotJobInput {
		database_branch_id,
		job_id,
		job_kind: CompactionJobKind::Hot,
		base_lifecycle_generation: 0,
		base_manifest_generation: 0,
		input_fingerprint,
		input_range,
		output_refs: output_refs.clone(),
	};
	let install_output = db
		.run({
			let install_input = install_input.clone();
			move |tx| {
				let install_input = install_input.clone();
				async move { install_hot_job_tx(&tx, &install_input, 2_001).await }
			}
		})
		.await?;
	assert_eq!(install_output.status, CompactionJobStatus::Succeeded);
	let published_row = read_test_value(
		&db,
		keys::branch_shard_key(database_branch_id, shard_two.shard_id, shard_two.as_of_txid),
	)
	.await?
	.expect("published hot shard manifest should exist");
	assert!(published_row.len() < staged_blob.len());
	let published_chunk = read_test_value(
		&db,
		keys::branch_shard_chunk_key(
			database_branch_id,
			shard_two.shard_id,
			shard_two.as_of_txid,
			0,
		),
	)
	.await?
	.expect("published hot shard chunk should exist");
	assert!(published_chunk.len() <= DELTA_OBJECT_CHUNK_BYTES);
	let fetched = database_db.get_pages(vec![128]).await?;
	assert_eq!(fetched[0].bytes.as_deref(), Some(expected_page.as_slice()));

	Ok(())
}

#[test]
fn cold_planning_uses_sha256_fingerprint_and_changes_with_inputs() {
	let database_branch_id = database_branch_id(0x3601);
	let root = root_with_watermarks(8, 4, 1);
	let mut cold_inputs = ColdInputSnapshot {
		commits: vec![(2, commit(3)), (3, commit(4))],
		shard_blobs: vec![ColdShardBlob {
			shard_id: 1,
			as_of_txid: 4,
			key: b"shard-key".to_vec(),
			bytes: b"shard-bytes".to_vec(),
		}],
		total_value_bytes: 11,
		min_versionstamp: [2; 16],
		max_versionstamp: [4; 16],
	};
	let mut snapshot = ManagerFdbSnapshot {
		branch_record: Some(branch_record(database_branch_id, 0)),
		head: Some(head(database_branch_id, 4)),
		root: root.clone(),
		dirty: None,
		db_pins: Vec::new(),
		hot_inputs: HotInputSnapshot::default(),
		cold_inputs,
		reclaim_inputs: ReclaimInputSnapshot::default(),
		bucket_proof_blocked_reclaim: false,
		cleared_dirty: false,
	};

	let first_job = plan_cold_job(database_branch_id, &snapshot, Id::new_v1(3610), 1_000, true)
		.expect("cold job should be planned");
	let second_job = plan_cold_job(database_branch_id, &snapshot, Id::new_v1(3611), 1_001, true)
		.expect("cold job should be planned");
	assert_eq!(first_job.input_fingerprint, second_job.input_fingerprint);

	let mut expected = Sha256::new();
	update_expected_fingerprint(&mut expected, database_branch_id.as_uuid().as_bytes());
	update_expected_fingerprint(&mut expected, &root.manifest_generation.to_be_bytes());
	update_expected_fingerprint(&mut expected, &root.hot_watermark_txid.to_be_bytes());
	update_expected_fingerprint(&mut expected, &root.cold_watermark_txid.to_be_bytes());
	update_expected_fingerprint(&mut expected, &root.cold_watermark_versionstamp);
	update_expected_fingerprint(&mut expected, &snapshot.cold_inputs.min_versionstamp);
	update_expected_fingerprint(&mut expected, &snapshot.cold_inputs.max_versionstamp);
	for (txid, commit) in &snapshot.cold_inputs.commits {
		update_expected_fingerprint(&mut expected, &txid.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.wall_clock_ms.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.versionstamp);
		update_expected_fingerprint(&mut expected, &commit.db_size_pages.to_be_bytes());
		update_expected_fingerprint(&mut expected, &commit.post_apply_checksum.to_be_bytes());
	}
	for blob in &snapshot.cold_inputs.shard_blobs {
		update_expected_fingerprint(&mut expected, &blob.shard_id.to_be_bytes());
		update_expected_fingerprint(&mut expected, &blob.as_of_txid.to_be_bytes());
		update_expected_fingerprint(&mut expected, &blob.key);
		update_expected_fingerprint(&mut expected, &blob.bytes);
	}
	assert_eq!(
		first_job.input_fingerprint,
		finish_expected_fingerprint(expected)
	);

	cold_inputs = snapshot.cold_inputs;
	cold_inputs.shard_blobs[0].bytes[0] ^= 0xff;
	snapshot.cold_inputs = cold_inputs;
	let changed_job = plan_cold_job(database_branch_id, &snapshot, Id::new_v1(3612), 1_002, true)
		.expect("cold job should be planned");
	assert_ne!(first_job.input_fingerprint, changed_job.input_fingerprint);
}

#[tokio::test]
async fn repair_fdb_cleanup_lifecycle_generation_rejects_recreated_branch() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3400);
	let stale_job_id = Id::new_v1(34);
	let staged_blob = vec![7_u8; 32];
	let stage_key =
		keys::branch_compaction_stage_hot_shard_key(database_branch_id, stale_job_id, 0, 1, 0);
	let output_ref = HotShardOutputRef {
		shard_id: 0,
		as_of_txid: 1,
		min_txid: 1,
		max_txid: 1,
		size_bytes: staged_blob.len() as u64,
		content_hash: super::content_hash(&staged_blob),
	};
	let input_range = repair_reclaim_input_range(
		vec![StagedHotShardCleanupRef {
			job_id: stale_job_id,
			output_ref,
		}],
		Vec::new(),
		std::iter::once(1),
	);
	let input = ReclaimFdbJobInput {
		database_branch_id,
		job_id: Id::new_v1(35),
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 0,
		base_manifest_generation: 1,
		input_fingerprint: fingerprint_repair_reclaim_range(database_branch_id, &input_range),
		input_range,
	};

	let output = db
		.run({
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

					cleanup_repair_fdb_outputs_tx(&tx, &input).await
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
		.run(move |tx| {
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
async fn orphan_cold_delete_lifecycle_generation_rejects_recreated_branch() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3401);
	let orphan = ColdShardRef {
		object_key: "db/orphan.ltx".to_string(),
		object_generation_id: Id::new_v1(36),
		shard_id: 0,
		as_of_txid: 1,
		min_txid: 1,
		max_txid: 1,
		min_versionstamp: [1; 16],
		max_versionstamp: [1; 16],
		size_bytes: 32,
		content_hash: [2; 32],
		publish_generation: 2,
	};

	let output = db
		.run({
			let orphan = orphan.clone();
			move |tx| {
				let orphan = orphan.clone();
				async move {
					tx.informal().set(
						&keys::branches_list_key(database_branch_id),
						&encode_database_branch_record(branch_record(database_branch_id, 1))?,
					);
					tx.informal().set(
						&keys::branch_compaction_root_key(database_branch_id),
						&encode_compaction_root(root(1))?,
					);

					plan_orphan_cold_object_deletes_tx(
						&tx,
						&super::DeleteOrphanColdObjectsInput {
							database_branch_id,
							base_lifecycle_generation: 0,
							orphan_cold_objects: vec![orphan],
						},
					)
					.await
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
	assert!(output.deleted_object_keys.is_empty());

	Ok(())
}

#[tokio::test]
async fn reclaim_input_snapshot_bounds_commit_scan_by_reclaim_ceiling() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3700);
	let root = root_with_watermarks(1, 10, 0);

	let snapshot = db
		.run({
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
						Snapshot,
						false,
						1_000,
					)
					.await
				}
			}
		})
		.await?;

	assert_eq!(
		snapshot
			.txid_refs
			.iter()
			.map(|txid_ref| txid_ref.txid)
			.collect::<Vec<_>>(),
		vec![10]
	);
	assert_eq!(snapshot.commits.len(), 1);

	Ok(())
}

#[tokio::test]
async fn reclaim_fdb_job_removes_complete_large_delta_rows() -> Result<()> {
	let db = test_db().await?;
	let database_branch_id = database_branch_id(0x3701);
	let txid = 1;
	let root = root_with_watermarks(1, txid, 0);
	let (rows, seeded) = complete_large_delta_rows(
		database_branch_id,
		txid,
		patterned_dirty_pages(20),
	)?;
	assert!(
		seeded.chunk_count > 1,
		"test large delta object should span multiple chunks"
	);

	db.run({
		let rows = rows.clone();
		move |tx| {
			let rows = rows.clone();
			async move {
				for (key, value) in rows {
					tx.informal().set(&key, &value);
				}
				Ok(())
			}
		}
	})
	.await?;

	let snapshot = db
		.run({
			let root = root.clone();
			move |tx| {
				let root = root.clone();
				async move {
					read_reclaim_input_snapshot(
						&tx,
						database_branch_id,
						&root,
						&[],
						None,
						ShardCachePolicy::default(),
						Snapshot,
						false,
						2_000,
					)
					.await
				}
			}
		})
		.await?;
	assert_eq!(snapshot.txid_refs.len(), 1);
	assert_eq!(snapshot.large_delta_manifests.len(), 1);
	assert_eq!(snapshot.large_delta_pageidx_entries.len(), seeded.pages.len());
	assert_eq!(snapshot.large_delta_complete_txids, vec![txid]);
	assert_eq!(
		snapshot.large_delta_object_chunks.len(),
		seeded.chunk_count as usize
	);
	assert!(reclaim_coverage_is_complete(&snapshot));

	let input_range = ReclaimJobInputRange {
		txids: TxidRange {
			min_txid: txid,
			max_txid: txid,
		},
		txid_refs: snapshot.txid_refs.clone(),
		cold_objects: Vec::new(),
		shard_cache_evictions: Vec::new(),
		staged_hot_shards: Vec::new(),
		orphan_cold_objects: Vec::new(),
		max_keys: 500,
		max_bytes: 2 * 1024 * 1024,
	};
	let input = ReclaimFdbJobInput {
		database_branch_id,
		job_id: Id::new_v1(3_701),
		job_kind: CompactionJobKind::Reclaim,
		base_lifecycle_generation: 1,
		base_manifest_generation: root.manifest_generation,
		input_fingerprint: fingerprint_reclaim_inputs(database_branch_id, &root, &snapshot),
		input_range,
	};
	let output = db
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { reclaim_fdb_job_tx(&tx, &input, false, 2_000).await }
			}
		})
		.await?;
	assert_eq!(output.status, CompactionJobStatus::Succeeded);
	assert_eq!(output.output_refs.len(), 1);

	assert!(
		read_test_value(&db, keys::branch_commit_key(database_branch_id, txid))
			.await?
			.is_none()
	);
	assert!(
		read_test_value(&db, keys::branch_delta_manifest_key(database_branch_id, txid))
			.await?
			.is_none()
	);
	assert!(
		read_test_value(
			&db,
			keys::branch_delta_object_ref_key(database_branch_id, seeded.object_id),
		)
		.await?
		.is_none()
	);
	assert!(
		read_test_value(
			&db,
			keys::branch_delta_object_meta_key(database_branch_id, seeded.object_id),
		)
		.await?
		.is_none()
	);
	for chunk_idx in 0..seeded.chunk_count {
		assert!(
			read_test_value(
				&db,
				keys::branch_delta_object_chunk_key(database_branch_id, seeded.object_id, chunk_idx),
			)
			.await?
			.is_none()
		);
	}
	for page in &seeded.pages {
		assert!(
			read_test_value(
				&db,
				keys::branch_delta_pageidx_key(database_branch_id, txid, page.pgno),
			)
			.await?
			.is_none()
		);
	}

	Ok(())
}
