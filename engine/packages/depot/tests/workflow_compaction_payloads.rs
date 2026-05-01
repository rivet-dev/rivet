use depot::{
	types::{ColdShardRef, DatabaseBranchId},
	workflows::compaction::*,
};
use gas::prelude::{Id, SignalTrait};
use uuid::Uuid;

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn gas_id(value: u128, label: u16) -> Id {
	Id::v1(Uuid::from_u128(value), label)
}

fn assert_embedded_version(encoded: &[u8]) {
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION
	);
}

fn fingerprint(seed: u8) -> CompactionInputFingerprint {
	[seed; 32]
}

fn txids() -> TxidRange {
	TxidRange {
		min_txid: 10,
		max_txid: 20,
	}
}

fn hot_input() -> HotJobInputRange {
	HotJobInputRange {
		txids: txids(),
		coverage_txids: vec![12, 20],
		max_pages: 128,
		max_bytes: 512 * 1024,
	}
}

fn cold_input() -> ColdJobInputRange {
	ColdJobInputRange {
		txids: txids(),
		min_versionstamp: [1; 16],
		max_versionstamp: [2; 16],
		max_bytes: 64 * 1024 * 1024,
	}
}

fn reclaim_input() -> ReclaimJobInputRange {
	ReclaimJobInputRange {
		txids: txids(),
		txid_refs: vec![ReclaimTxidRef {
			txid: 10,
			versionstamp: [9; 16],
		}],
		cold_objects: vec![ReclaimColdObjectRef {
			object_key: "db/branch/shard/00000007/0000000000000014-job-hash.ltx".into(),
			object_generation_id: gas_id(0x1111_2222_3333_4444_5555_6666_7777_8888, 9),
			content_hash: [4; 32],
			expected_publish_generation: 42,
			shard_id: 7,
			as_of_txid: 20,
		}],
		staged_hot_shards: vec![StagedHotShardCleanupRef {
			job_id: gas_id(0x1111_2222_3333_4444_5555_6666_7777_9999, 9),
			output_ref: hot_output(),
		}],
		orphan_cold_objects: vec![cold_output()],
		max_keys: 500,
		max_bytes: 2 * 1024 * 1024,
	}
}

fn hot_output() -> HotShardOutputRef {
	HotShardOutputRef {
		shard_id: 7,
		as_of_txid: 20,
		min_txid: 10,
		max_txid: 20,
		size_bytes: 96 * 1024,
		content_hash: [3; 32],
	}
}

fn cold_output() -> ColdShardRef {
	ColdShardRef {
		object_key: "db/branch/shard/00000007/0000000000000014-job-hash.ltx".into(),
		object_generation_id: gas_id(0x1111_2222_3333_4444_5555_6666_7777_8888, 9),
		shard_id: 7,
		as_of_txid: 20,
		min_txid: 10,
		max_txid: 20,
		min_versionstamp: [1; 16],
		max_versionstamp: [2; 16],
		size_bytes: 96 * 1024,
		content_hash: [4; 32],
		publish_generation: 42,
	}
}

fn reclaim_output() -> ReclaimOutputRef {
	ReclaimOutputRef {
		key_count: 12,
		byte_count: 4096,
		min_txid: 10,
		max_txid: 20,
	}
}

fn active_job(kind: CompactionJobKind) -> ActiveCompactionJob {
	let input_range = match kind {
		CompactionJobKind::Hot => PlannedInputRange::Hot(hot_input()),
		CompactionJobKind::Cold => PlannedInputRange::Cold(cold_input()),
		CompactionJobKind::Reclaim => PlannedInputRange::Reclaim(reclaim_input()),
	};

	ActiveCompactionJob {
		database_branch_id: database_branch_id(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111),
		job_id: gas_id(0x0101_0202_0303_0404_0505_0606_0707_0808, 17),
		job_kind: kind,
		base_lifecycle_generation: 7,
		base_manifest_generation: 41,
		input_fingerprint: fingerprint(5),
		input_range,
		planned_at_ms: 1_714_000_000_000,
		attempt: 2,
	}
}

macro_rules! assert_round_trip {
	($value:expr, $encode:ident, $decode:ident) => {{
		let value = $value;
		let encoded = $encode(value.clone()).expect("payload should encode");
		assert_embedded_version(&encoded);

		let decoded = $decode(&encoded).expect("payload should decode");
		assert_eq!(decoded, value);
	}};
}

#[test]
fn compaction_signal_names_are_stable() {
	assert_eq!(
		<DeltasAvailable as SignalTrait>::NAME,
		"depot_sqlite_cmp_deltas_available"
	);
	assert_eq!(
		<HotJobFinished as SignalTrait>::NAME,
		"depot_sqlite_cmp_hot_job_finished"
	);
	assert_eq!(
		<ColdJobFinished as SignalTrait>::NAME,
		"depot_sqlite_cmp_cold_job_finished"
	);
	assert_eq!(
		<ReclaimJobFinished as SignalTrait>::NAME,
		"depot_sqlite_cmp_reclaim_job_finished"
	);
	assert_eq!(
		<DestroyDatabaseBranch as SignalTrait>::NAME,
		"depot_sqlite_cmp_destroy_database_branch"
	);
	assert_eq!(
		<RunHotJob as SignalTrait>::NAME,
		"depot_sqlite_cmp_run_hot_job"
	);
	assert_eq!(
		<RunColdJob as SignalTrait>::NAME,
		"depot_sqlite_cmp_run_cold_job"
	);
	assert_eq!(
		<RunReclaimJob as SignalTrait>::NAME,
		"depot_sqlite_cmp_run_reclaim_job"
	);
}

#[test]
fn manager_signals_round_trip_with_embedded_version() {
	let database_branch_id = database_branch_id(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);

	assert_round_trip!(
		DeltasAvailable {
			database_branch_id,
			observed_head_txid: 20,
			dirty_updated_at_ms: 1_714_000_000_000,
		},
		encode_deltas_available,
		decode_deltas_available
	);

	assert_round_trip!(
		HotJobFinished {
			database_branch_id,
			job_id: gas_id(0x1000_2000_3000_4000_5000_6000_7000_8000, 10),
			job_kind: CompactionJobKind::Hot,
			base_manifest_generation: 41,
			input_fingerprint: fingerprint(6),
			status: CompactionJobStatus::Succeeded,
			output_refs: vec![hot_output()],
		},
		encode_hot_job_finished,
		decode_hot_job_finished
	);

	assert_round_trip!(
		ColdJobFinished {
			database_branch_id,
			job_id: gas_id(0x2000_3000_4000_5000_6000_7000_8000_9000, 11),
			job_kind: CompactionJobKind::Cold,
			base_manifest_generation: 42,
			input_fingerprint: fingerprint(7),
			status: CompactionJobStatus::Rejected {
				reason: "stale manifest".into(),
			},
			output_refs: vec![cold_output()],
		},
		encode_cold_job_finished,
		decode_cold_job_finished
	);

	assert_round_trip!(
		ReclaimJobFinished {
			database_branch_id,
			job_id: gas_id(0x3000_4000_5000_6000_7000_8000_9000_a000, 12),
			job_kind: CompactionJobKind::Reclaim,
			base_manifest_generation: 43,
			input_fingerprint: fingerprint(8),
			status: CompactionJobStatus::Failed {
				error: "FDB transaction too large".into(),
			},
			output_refs: vec![reclaim_output()],
		},
		encode_reclaim_job_finished,
		decode_reclaim_job_finished
	);

	assert_round_trip!(
		DestroyDatabaseBranch {
			database_branch_id,
			lifecycle_generation: 7,
			requested_at_ms: 1_714_000_010_000,
			reason: "branch deleted".into(),
		},
		encode_destroy_database_branch,
		decode_destroy_database_branch
	);
}

#[test]
fn companion_signals_round_trip_with_embedded_version() {
	let database_branch_id = database_branch_id(0x1011_2233_4455_6677_8899_aabb_ccdd_eeff);

	assert_round_trip!(
		RunHotJob {
			database_branch_id,
			job_id: gas_id(0x4000_5000_6000_7000_8000_9000_a000_b000, 13),
			job_kind: CompactionJobKind::Hot,
			base_lifecycle_generation: 7,
			base_manifest_generation: 44,
			input_fingerprint: fingerprint(9),
			status: CompactionJobStatus::Requested,
			input_range: hot_input(),
		},
		encode_run_hot_job,
		decode_run_hot_job
	);

	assert_round_trip!(
		RunColdJob {
			database_branch_id,
			job_id: gas_id(0x5000_6000_7000_8000_9000_a000_b000_c000, 14),
			job_kind: CompactionJobKind::Cold,
			base_lifecycle_generation: 8,
			base_manifest_generation: 45,
			input_fingerprint: fingerprint(10),
			status: CompactionJobStatus::Requested,
			input_range: cold_input(),
		},
		encode_run_cold_job,
		decode_run_cold_job
	);

	assert_round_trip!(
		RunReclaimJob {
			database_branch_id,
			job_id: gas_id(0x6000_7000_8000_9000_a000_b000_c000_d000, 15),
			job_kind: CompactionJobKind::Reclaim,
			base_lifecycle_generation: 9,
			base_manifest_generation: 46,
			input_fingerprint: fingerprint(11),
			status: CompactionJobStatus::Requested,
			input_range: reclaim_input(),
		},
		encode_run_reclaim_job,
		decode_run_reclaim_job
	);
}

#[test]
fn workflow_states_round_trip_with_embedded_version() {
	assert_round_trip!(
		DbManagerState {
			companion_workflow_ids: CompanionWorkflowIds {
				hot_compacter_workflow_id: Some(gas_id(
					0x7000_8000_9000_a000_b000_c000_d000_e000,
					20,
				)),
				cold_compacter_workflow_id: Some(gas_id(
					0x8000_9000_a000_b000_c000_d000_e000_f000,
					21,
				)),
				reclaimer_workflow_id: Some(gas_id(
					0x9000_a000_b000_c000_d000_e000_f000_0001,
					22,
				)),
			},
			active_hot_job: Some(active_job(CompactionJobKind::Hot)),
			active_cold_job: Some(active_job(CompactionJobKind::Cold)),
			active_reclaim_job: Some(active_job(CompactionJobKind::Reclaim)),
			retry_cursors: ManagerRetryCursors {
				hot: RetryCursor {
					attempt: 1,
					next_attempt_at_ms: Some(1_714_000_020_000),
					last_error: None,
				},
				cold: RetryCursor {
					attempt: 2,
					next_attempt_at_ms: Some(1_714_000_030_000),
					last_error: Some("upload throttled".into()),
				},
				reclaim: RetryCursor {
					attempt: 0,
					next_attempt_at_ms: None,
					last_error: None,
				},
			},
			planning_deadlines: ManagerPlanningDeadlines {
				next_hot_check_at_ms: Some(1_714_000_040_000),
				next_cold_check_at_ms: Some(1_714_000_050_000),
				next_reclaim_check_at_ms: Some(1_714_000_060_000),
				final_settle_check_at_ms: Some(1_714_000_070_000),
			},
			branch_stop_state: BranchStopState::DestroyRequested {
				lifecycle_generation: 7,
				requested_at_ms: 1_714_000_080_000,
				reason: "delete requested".into(),
			},
			last_dirty_cursor: Some(DirtyCursor {
				observed_head_txid: 20,
				dirty_updated_at_ms: 1_714_000_000_000,
			}),
		},
		encode_db_manager_state,
		decode_db_manager_state
	);

	assert_round_trip!(
		CompanionWorkflowState::Running(CompanionRunningJob {
			database_branch_id: database_branch_id(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_2222),
			job_id: gas_id(0xa000_b000_c000_d000_e000_f000_0001_0002, 23),
			job_kind: CompactionJobKind::Hot,
			base_lifecycle_generation: 7,
			base_manifest_generation: 47,
			input_fingerprint: fingerprint(12),
			started_at_ms: 1_714_000_090_000,
			attempt: 3,
		}),
		encode_companion_workflow_state,
		decode_companion_workflow_state
	);

	assert_round_trip!(
		CompanionWorkflowState::Stopping {
			active_job: None,
			lifecycle_generation: 7,
			requested_at_ms: 1_714_000_100_000,
			reason: "branch deleted".into(),
		},
		encode_companion_workflow_state,
		decode_companion_workflow_state
	);
}
