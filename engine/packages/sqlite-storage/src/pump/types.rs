use anyhow::{Context, Result, bail};
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use vbare::OwnedVersionedData;

pub const SQLITE_STORAGE_META_VERSION: u16 = 1;
pub const SQLITE_PAGE_SIZE: u32 = crate::keys::PAGE_SIZE;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DBHead {
	pub head_txid: u64,
	pub db_size_pages: u32,
	#[cfg(debug_assertions)]
	pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaCompact {
	pub materialized_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyPage {
	pub pgno: u32,
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchedPage {
	pub pgno: u32,
	pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionConfig {
	pub retention_ms: u64,
	pub checkpoint_interval_ms: u64,
	pub max_checkpoints: u32,
}

impl Default for RetentionConfig {
	fn default() -> Self {
		Self {
			retention_ms: 0,
			checkpoint_interval_ms: 3_600_000,
			max_checkpoints: 25,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreMarker {
	pub target_txid: u64,
	pub ckp_txid: u64,
	pub started_at_ms: i64,
	pub last_completed_step: RestoreStep,
	pub holder_id: NodeId,
	pub op_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreStep {
	Started,
	CheckpointCopied,
	DeltasReplayed,
	MetaWritten,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkMarker {
	pub src_actor_id: String,
	pub ckp_txid: u64,
	pub target_txid: u64,
	pub started_at_ms: i64,
	pub last_completed_step: ForkStep,
	pub holder_id: NodeId,
	pub op_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkStep {
	Started,
	CheckpointCopied,
	DeltasReplayed,
	MetaWritten,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointMeta {
	pub taken_at_ms: i64,
	pub head_txid: u64,
	pub db_size_pages: u32,
	pub byte_count: u64,
	pub refcount: u32,
	pub pinned_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointEntry {
	pub ckp_txid: u64,
	pub taken_at_ms: i64,
	pub byte_count: u64,
	pub refcount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoints {
	pub entries: Vec<CheckpointEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaMeta {
	pub taken_at_ms: i64,
	pub byte_count: u64,
	pub refcount: u32,
}

struct VersionedV1<T>(T);

impl<T> OwnedVersionedData for VersionedV1<T>
where
	T: Serialize + DeserializeOwned,
{
	type Latest = T;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		Ok(self.0)
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite-storage type version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		serde_bare::to_vec(&self.0).map_err(Into::into)
	}
}

enum VersionedDBHead {
	V1(DBHead),
}

impl OwnedVersionedData for VersionedDBHead {
	type Latest = DBHead;

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
			_ => bail!("invalid sqlite-storage DBHead version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedMetaCompact {
	V1(MetaCompact),
}

impl OwnedVersionedData for VersionedMetaCompact {
	type Latest = MetaCompact;

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
			_ => bail!("invalid sqlite-storage MetaCompact version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_db_head(head: DBHead) -> Result<Vec<u8>> {
	VersionedDBHead::wrap_latest(head)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite db head")
}

pub fn decode_db_head(payload: &[u8]) -> Result<DBHead> {
	VersionedDBHead::deserialize_with_embedded_version(payload).context("decode sqlite db head")
}

pub fn encode_meta_compact(compact: MetaCompact) -> Result<Vec<u8>> {
	VersionedMetaCompact::wrap_latest(compact)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite compact meta")
}

pub fn decode_meta_compact(payload: &[u8]) -> Result<MetaCompact> {
	VersionedMetaCompact::deserialize_with_embedded_version(payload)
		.context("decode sqlite compact meta")
}

fn encode_versioned<T>(value: T, name: &str) -> Result<Vec<u8>>
where
	T: Serialize + DeserializeOwned,
{
	VersionedV1::wrap_latest(value)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.with_context(|| format!("encode sqlite {name}"))
}

fn decode_versioned<T>(payload: &[u8], name: &str) -> Result<T>
where
	T: Serialize + DeserializeOwned,
{
	VersionedV1::<T>::deserialize_with_embedded_version(payload)
		.with_context(|| format!("decode sqlite {name}"))
}

pub fn encode_retention_config(config: RetentionConfig) -> Result<Vec<u8>> {
	encode_versioned(config, "retention config")
}

pub fn decode_retention_config(payload: &[u8]) -> Result<RetentionConfig> {
	decode_versioned(payload, "retention config")
}

pub fn encode_restore_marker(marker: RestoreMarker) -> Result<Vec<u8>> {
	encode_versioned(marker, "restore marker")
}

pub fn decode_restore_marker(payload: &[u8]) -> Result<RestoreMarker> {
	decode_versioned(payload, "restore marker")
}

pub fn encode_fork_marker(marker: ForkMarker) -> Result<Vec<u8>> {
	encode_versioned(marker, "fork marker")
}

pub fn decode_fork_marker(payload: &[u8]) -> Result<ForkMarker> {
	decode_versioned(payload, "fork marker")
}

pub fn encode_checkpoint_meta(meta: CheckpointMeta) -> Result<Vec<u8>> {
	encode_versioned(meta, "checkpoint meta")
}

pub fn decode_checkpoint_meta(payload: &[u8]) -> Result<CheckpointMeta> {
	decode_versioned(payload, "checkpoint meta")
}

pub fn encode_checkpoints(checkpoints: Checkpoints) -> Result<Vec<u8>> {
	encode_versioned(checkpoints, "checkpoints")
}

pub fn decode_checkpoints(payload: &[u8]) -> Result<Checkpoints> {
	decode_versioned(payload, "checkpoints")
}

pub fn encode_delta_meta(meta: DeltaMeta) -> Result<Vec<u8>> {
	encode_versioned(meta, "delta meta")
}

pub fn decode_delta_meta(payload: &[u8]) -> Result<DeltaMeta> {
	decode_versioned(payload, "delta meta")
}

#[cfg(test)]
mod tests {
	use super::{
		DBHead, MetaCompact, SQLITE_STORAGE_META_VERSION, decode_db_head, decode_meta_compact,
		encode_db_head, encode_meta_compact,
	};

	#[test]
	fn db_head_round_trips_with_embedded_version() {
		let head = DBHead {
			head_txid: 42,
			db_size_pages: 128,
			#[cfg(debug_assertions)]
			generation: 7,
		};

		let encoded = encode_db_head(head.clone()).expect("db head should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_STORAGE_META_VERSION
		);

		let decoded = decode_db_head(&encoded).expect("db head should decode");
		assert_eq!(decoded, head);
	}

	#[test]
	fn meta_compact_round_trips_with_embedded_version() {
		let compact = MetaCompact {
			materialized_txid: 24,
		};

		let encoded = encode_meta_compact(compact.clone()).expect("compact meta should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_STORAGE_META_VERSION
		);

		let decoded = decode_meta_compact(&encoded).expect("compact meta should decode");
		assert_eq!(decoded, compact);
	}
}
