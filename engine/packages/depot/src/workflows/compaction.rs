use anyhow::{Context, Result, bail};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use vbare::OwnedVersionedData;

use crate::conveyer::types::{ColdShardRef, DatabaseBranchId};

pub const SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION: u16 = 1;

pub type CompactionInputFingerprint = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionJobKind {
	Hot,
	Cold,
	Reclaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionJobStatus {
	Requested,
	Succeeded,
	Rejected { reason: String },
	Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxidRange {
	pub min_txid: u64,
	pub max_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotJobInputRange {
	pub txids: TxidRange,
	pub max_pages: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdJobInputRange {
	pub txids: TxidRange,
	pub min_versionstamp: [u8; 16],
	pub max_versionstamp: [u8; 16],
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimJobInputRange {
	pub txids: TxidRange,
	pub max_keys: u32,
	pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedInputRange {
	Hot(HotJobInputRange),
	Cold(ColdJobInputRange),
	Reclaim(ReclaimJobInputRange),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotShardOutputRef {
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub min_txid: u64,
	pub max_txid: u64,
	pub size_bytes: u64,
	pub content_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimOutputRef {
	pub key_count: u32,
	pub byte_count: u64,
	pub min_txid: u64,
	pub max_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_deltas_available")]
pub struct DeltasAvailable {
	pub database_branch_id: DatabaseBranchId,
	pub observed_head_txid: u64,
	pub dirty_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_hot_job_finished")]
pub struct HotJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<HotShardOutputRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_cold_job_finished")]
pub struct ColdJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ColdShardRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_reclaim_job_finished")]
pub struct ReclaimJobFinished {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub output_refs: Vec<ReclaimOutputRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_destroy_database_branch")]
pub struct DestroyDatabaseBranch {
	pub database_branch_id: DatabaseBranchId,
	pub requested_at_ms: i64,
	pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_hot_job")]
pub struct RunHotJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: HotJobInputRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_cold_job")]
pub struct RunColdJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: ColdJobInputRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[signal("depot_sqlite_cmp_run_reclaim_job")]
pub struct RunReclaimJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub status: CompactionJobStatus,
	pub input_range: ReclaimJobInputRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManagerState {
	pub companion_workflow_ids: CompanionWorkflowIds,
	pub active_hot_job: Option<ActiveCompactionJob>,
	pub active_cold_job: Option<ActiveCompactionJob>,
	pub active_reclaim_job: Option<ActiveCompactionJob>,
	pub retry_cursors: ManagerRetryCursors,
	pub planning_deadlines: ManagerPlanningDeadlines,
	pub branch_stop_state: BranchStopState,
	pub last_dirty_cursor: Option<DirtyCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionWorkflowIds {
	pub hot_compacter_workflow_id: Option<Id>,
	pub cold_compacter_workflow_id: Option<Id>,
	pub reclaimer_workflow_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveCompactionJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub input_range: PlannedInputRange,
	pub planned_at_ms: i64,
	pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerRetryCursors {
	pub hot: RetryCursor,
	pub cold: RetryCursor,
	pub reclaim: RetryCursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryCursor {
	pub attempt: u32,
	pub next_attempt_at_ms: Option<i64>,
	pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerPlanningDeadlines {
	pub next_hot_check_at_ms: Option<i64>,
	pub next_cold_check_at_ms: Option<i64>,
	pub next_reclaim_check_at_ms: Option<i64>,
	pub final_settle_check_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStopState {
	Running,
	DestroyRequested { requested_at_ms: i64, reason: String },
	Stopping { requested_at_ms: i64, reason: String },
	Stopped { stopped_at_ms: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyCursor {
	pub observed_head_txid: u64,
	pub dirty_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanionWorkflowState {
	Idle,
	Running(CompanionRunningJob),
	Stopping {
		active_job: Option<CompanionRunningJob>,
		requested_at_ms: i64,
		reason: String,
	},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionRunningJob {
	pub database_branch_id: DatabaseBranchId,
	pub job_id: Id,
	pub job_kind: CompactionJobKind,
	pub base_manifest_generation: u64,
	pub input_fingerprint: CompactionInputFingerprint,
	pub started_at_ms: i64,
	pub attempt: u32,
}

enum VersionedDeltasAvailable {
	V1(DeltasAvailable),
}

enum VersionedHotJobFinished {
	V1(HotJobFinished),
}

enum VersionedColdJobFinished {
	V1(ColdJobFinished),
}

enum VersionedReclaimJobFinished {
	V1(ReclaimJobFinished),
}

enum VersionedDestroyDatabaseBranch {
	V1(DestroyDatabaseBranch),
}

enum VersionedRunHotJob {
	V1(RunHotJob),
}

enum VersionedRunColdJob {
	V1(RunColdJob),
}

enum VersionedRunReclaimJob {
	V1(RunReclaimJob),
}

enum VersionedDbManagerState {
	V1(DbManagerState),
}

enum VersionedCompanionWorkflowState {
	V1(CompanionWorkflowState),
}

macro_rules! impl_workflow_compaction_versioned_data {
	($versioned:ident, $latest:ty, $name:literal, $encode:ident, $decode:ident) => {
		impl OwnedVersionedData for $versioned {
			type Latest = $latest;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V1(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V1(data) => Ok(data),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
					_ => bail!("invalid depot workflow compaction {} version: {version}", $name),
				}
			}

			fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
				match self {
					Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
				}
			}
		}

		pub fn $encode(payload: $latest) -> Result<Vec<u8>> {
			$versioned::wrap_latest(payload)
				.serialize_with_embedded_version(SQLITE_COMPACTION_WORKFLOW_PAYLOAD_VERSION)
				.with_context(|| format!("encode sqlite workflow compaction {}", $name))
		}

		pub fn $decode(payload: &[u8]) -> Result<$latest> {
			$versioned::deserialize_with_embedded_version(payload)
				.with_context(|| format!("decode sqlite workflow compaction {}", $name))
		}
	};
}

impl_workflow_compaction_versioned_data!(
	VersionedDeltasAvailable,
	DeltasAvailable,
	"DeltasAvailable",
	encode_deltas_available,
	decode_deltas_available
);
impl_workflow_compaction_versioned_data!(
	VersionedHotJobFinished,
	HotJobFinished,
	"HotJobFinished",
	encode_hot_job_finished,
	decode_hot_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedColdJobFinished,
	ColdJobFinished,
	"ColdJobFinished",
	encode_cold_job_finished,
	decode_cold_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedReclaimJobFinished,
	ReclaimJobFinished,
	"ReclaimJobFinished",
	encode_reclaim_job_finished,
	decode_reclaim_job_finished
);
impl_workflow_compaction_versioned_data!(
	VersionedDestroyDatabaseBranch,
	DestroyDatabaseBranch,
	"DestroyDatabaseBranch",
	encode_destroy_database_branch,
	decode_destroy_database_branch
);
impl_workflow_compaction_versioned_data!(
	VersionedRunHotJob,
	RunHotJob,
	"RunHotJob",
	encode_run_hot_job,
	decode_run_hot_job
);
impl_workflow_compaction_versioned_data!(
	VersionedRunColdJob,
	RunColdJob,
	"RunColdJob",
	encode_run_cold_job,
	decode_run_cold_job
);
impl_workflow_compaction_versioned_data!(
	VersionedRunReclaimJob,
	RunReclaimJob,
	"RunReclaimJob",
	encode_run_reclaim_job,
	decode_run_reclaim_job
);
impl_workflow_compaction_versioned_data!(
	VersionedDbManagerState,
	DbManagerState,
	"DbManagerState",
	encode_db_manager_state,
	decode_db_manager_state
);
impl_workflow_compaction_versioned_data!(
	VersionedCompanionWorkflowState,
	CompanionWorkflowState,
	"CompanionWorkflowState",
	encode_companion_workflow_state,
	decode_companion_workflow_state
);
