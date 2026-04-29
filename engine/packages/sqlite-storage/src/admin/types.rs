use anyhow::{Result, bail};
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vbare::OwnedVersionedData;

pub const SQLITE_ADMIN_RECORD_VERSION: u16 = 1;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditFields {
	pub caller_id: String,
	pub request_origin_ts_ms: i64,
	pub namespace_id: Uuid,
}

enum VersionedAdminOpRecord {
	V1(AdminOpRecord),
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
