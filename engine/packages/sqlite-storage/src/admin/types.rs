use anyhow::{Result, bail};
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vbare::OwnedVersionedData;

pub const SQLITE_ADMIN_RECORD_VERSION: u16 = 1;
pub const SQLITE_OP_REQUEST_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqliteOpRequest {
	pub request_id: Uuid,
	pub op: SqliteOp,
	pub audit: AuditFields,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqliteOp {
	Restore {
		actor_id: String,
		target: RestoreTarget,
		mode: RestoreMode,
	},
	Fork {
		src_actor_id: String,
		target: RestoreTarget,
		mode: ForkMode,
		dst: ForkDstSpec,
	},
	DescribeRetention {
		actor_id: String,
	},
	GetRetention {
		actor_id: String,
	},
	SetRetention {
		actor_id: String,
		config: crate::pump::types::RetentionConfig,
	},
	ClearRefcount {
		actor_id: String,
		kind: RefcountKind,
		txid: u64,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreTarget {
	Txid(u64),
	TimestampMs(i64),
	LatestCheckpoint,
	CheckpointTxid(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreMode {
	Apply,
	DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkMode {
	Apply,
	DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkDstSpec {
	Allocate { dst_namespace_id: Uuid },
	Existing { dst_actor_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefcountKind {
	Checkpoint,
	Delta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminOpRecord {
	pub operation_id: Uuid,
	pub op_kind: OpKind,
	pub actor_id: String,
	pub created_at_ms: i64,
	pub last_progress_at_ms: i64,
	pub status: OpStatus,
	pub holder_id: Option<NodeId>,
	pub progress: Option<OpProgress>,
	pub result: Option<OpResult>,
	pub audit: AuditFields,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpKind {
	Restore,
	Fork,
	DescribeRetention,
	GetRetention,
	SetRetention,
	ClearRefcount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpStatus {
	Pending,
	InProgress,
	Completed,
	Failed,
	Orphaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpProgress {
	pub step: String,
	pub bytes_done: u64,
	pub bytes_total: u64,
	pub started_at_ms: i64,
	pub eta_ms: Option<i64>,
	pub current_tx_index: u32,
	pub total_tx_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpResult {
	Empty,
	Message { message: String },
	RetentionView(RetentionView),
	RetentionConfig(crate::pump::types::RetentionConfig),
	ClearRefcount(ClearRefcountResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditFields {
	pub caller_id: String,
	pub request_origin_ts_ms: i64,
	pub namespace_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionView {
	pub head: HeadView,
	pub fine_grained_window: Option<FineGrainedWindow>,
	pub checkpoints: Vec<CheckpointView>,
	pub retention_config: crate::pump::types::RetentionConfig,
	pub storage_used_live_bytes: u64,
	pub storage_used_pitr_bytes: u64,
	pub pitr_namespace_budget_bytes: u64,
	pub pitr_namespace_used_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadView {
	pub head_txid: u64,
	pub db_size_pages: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FineGrainedWindow {
	pub from_txid: u64,
	pub to_txid: u64,
	pub from_taken_at_ms: i64,
	pub to_taken_at_ms: i64,
	pub delta_count: u64,
	pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointView {
	pub ckp_txid: u64,
	pub taken_at_ms: i64,
	pub byte_count: u64,
	pub refcount: u32,
	pub pinned_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearRefcountResult {
	pub kind: RefcountKind,
	pub txid: u64,
}

enum VersionedAdminOpRecord {
	V1(AdminOpRecord),
}

enum VersionedSqliteOpRequest {
	V1(SqliteOpRequest),
}

impl OwnedVersionedData for VersionedAdminOpRecord {
	type Latest = AdminOpRecord;

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
			_ => bail!("invalid sqlite admin op record version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_admin_op_record(record: AdminOpRecord) -> Result<Vec<u8>> {
	VersionedAdminOpRecord::wrap_latest(record)
		.serialize_with_embedded_version(SQLITE_ADMIN_RECORD_VERSION)
}

pub fn decode_admin_op_record(payload: &[u8]) -> Result<AdminOpRecord> {
	VersionedAdminOpRecord::deserialize_with_embedded_version(payload)
}

impl OwnedVersionedData for VersionedSqliteOpRequest {
	type Latest = SqliteOpRequest;

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
			_ => bail!("invalid sqlite op request version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_sqlite_op_request(request: SqliteOpRequest) -> Result<Vec<u8>> {
	VersionedSqliteOpRequest::wrap_latest(request)
		.serialize_with_embedded_version(SQLITE_OP_REQUEST_VERSION)
}

pub fn decode_sqlite_op_request(payload: &[u8]) -> Result<SqliteOpRequest> {
	VersionedSqliteOpRequest::deserialize_with_embedded_version(payload)
}
