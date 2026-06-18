use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vbare::OwnedVersionedData;

use super::{DirtyPage, ids::DatabaseBranchId};
use super::serialization::SQLITE_STORAGE_META_VERSION;

const HOT_SHARD_MANIFEST_MAGIC: &[u8] = b"rivet.depot.hot_shard_manifest.v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchManifest {
	pub cold_drained_txid: u64,
	pub last_hot_pass_txid: u64,
	pub last_access_ts_ms: i64,
	pub last_access_bucket: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitRow {
	pub wall_clock_ms: i64,
	pub versionstamp: [u8; 16],
	pub db_size_pages: u32,
	pub post_apply_checksum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DBHead {
	pub head_txid: u64,
	pub db_size_pages: u32,
	pub post_apply_checksum: u64,
	pub branch_id: DatabaseBranchId,
	#[cfg(debug_assertions)]
	pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaCompact {
	pub materialized_txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStageMeta {
	pub stage_id: Uuid,
	pub object_id: Uuid,
	pub observed_head_txid: u64,
	pub staged_txid: u64,
	pub caller_expected_head_txid: Option<u64>,
	pub db_size_pages: u32,
	pub now_ms: i64,
	pub dirty_page_count: u32,
	pub dirty_pgnos_hash: [u8; 32],
	pub reserved_storage_bytes: i64,
	pub state: CommitStageState,
	pub created_at_ms: i64,
	pub expires_after_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitStageState {
	Uploading,
	Complete,
	ObjectWritten,
	Finalizing,
	Finalized { txid: u64 },
	Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyPageBatch {
	pub batch_idx: u32,
	pub pages: Vec<DirtyPage>,
	pub batch_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStageComplete {
	pub page_batch_count: u32,
	pub dirty_page_count: u32,
	pub dirty_pages_hash: [u8; 32],
	pub completed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStageFinalized {
	pub stage_id: Uuid,
	pub object_id: Uuid,
	pub txid: u64,
	pub finalized_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaObjectMeta {
	pub object_id: Uuid,
	pub stage_id: Uuid,
	pub staged_txid: u64,
	pub chunk_count: u32,
	pub encoded_len: u64,
	pub object_hash: [u8; 32],
	pub state: DeltaObjectState,
	pub created_at_ms: i64,
	pub expires_after_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaObjectState {
	StageOwned,
	Committed { txid: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaManifest {
	pub txid: u64,
	pub object_id: Uuid,
	pub chunk_count: u32,
	pub encoded_len: u64,
	pub object_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaPageIndexEntry {
	pub txid: u64,
	pub object_id: Uuid,
	pub encoded_offset: u64,
	pub encoded_size: u32,
	pub page_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotShardManifest {
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub chunk_count: u32,
	pub encoded_len: u64,
	pub content_hash: [u8; 32],
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
			_ => bail!("invalid depot DBHead version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedCommitRow {
	V1(CommitRow),
}

impl OwnedVersionedData for VersionedCommitRow {
	type Latest = CommitRow;

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
			_ => bail!("invalid depot CommitRow version: {version}"),
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

enum VersionedCommitStageMeta {
	V1(CommitStageMeta),
}

enum VersionedDirtyPageBatch {
	V1(DirtyPageBatch),
}

enum VersionedCommitStageComplete {
	V1(CommitStageComplete),
}

enum VersionedCommitStageFinalized {
	V1(CommitStageFinalized),
}

enum VersionedDeltaObjectMeta {
	V1(DeltaObjectMeta),
}

enum VersionedDeltaManifest {
	V1(DeltaManifest),
}

enum VersionedDeltaPageIndexEntry {
	V1(DeltaPageIndexEntry),
}

enum VersionedHotShardManifest {
	V1(HotShardManifest),
}

macro_rules! impl_versioned_record {
	($versioned:ident, $latest:ty, $name:literal) => {
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
					_ => bail!("invalid depot {} version: {}", $name, version),
				}
			}

			fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
				match self {
					Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
				}
			}
		}
	};
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
			_ => bail!("invalid depot MetaCompact version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl_versioned_record!(VersionedCommitStageMeta, CommitStageMeta, "CommitStageMeta");
impl_versioned_record!(VersionedDirtyPageBatch, DirtyPageBatch, "DirtyPageBatch");
impl_versioned_record!(
	VersionedCommitStageComplete,
	CommitStageComplete,
	"CommitStageComplete"
);
impl_versioned_record!(
	VersionedCommitStageFinalized,
	CommitStageFinalized,
	"CommitStageFinalized"
);
impl_versioned_record!(VersionedDeltaObjectMeta, DeltaObjectMeta, "DeltaObjectMeta");
impl_versioned_record!(VersionedDeltaManifest, DeltaManifest, "DeltaManifest");
impl_versioned_record!(
	VersionedDeltaPageIndexEntry,
	DeltaPageIndexEntry,
	"DeltaPageIndexEntry"
);
impl_versioned_record!(
	VersionedHotShardManifest,
	HotShardManifest,
	"HotShardManifest"
);

pub fn encode_db_head(head: DBHead) -> Result<Vec<u8>> {
	VersionedDBHead::wrap_latest(head)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite db head")
}

pub fn decode_db_head(payload: &[u8]) -> Result<DBHead> {
	VersionedDBHead::deserialize_with_embedded_version(payload).context("decode sqlite db head")
}

pub fn encode_commit_row(row: CommitRow) -> Result<Vec<u8>> {
	VersionedCommitRow::wrap_latest(row)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite commit row")
}

pub fn decode_commit_row(payload: &[u8]) -> Result<CommitRow> {
	VersionedCommitRow::deserialize_with_embedded_version(payload)
		.context("decode sqlite commit row")
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

pub fn encode_commit_stage_meta(meta: CommitStageMeta) -> Result<Vec<u8>> {
	VersionedCommitStageMeta::wrap_latest(meta)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite commit stage meta")
}

pub fn decode_commit_stage_meta(payload: &[u8]) -> Result<CommitStageMeta> {
	VersionedCommitStageMeta::deserialize_with_embedded_version(payload)
		.context("decode sqlite commit stage meta")
}

pub fn encode_dirty_page_batch(batch: DirtyPageBatch) -> Result<Vec<u8>> {
	VersionedDirtyPageBatch::wrap_latest(batch)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite dirty page batch")
}

pub fn decode_dirty_page_batch(payload: &[u8]) -> Result<DirtyPageBatch> {
	VersionedDirtyPageBatch::deserialize_with_embedded_version(payload)
		.context("decode sqlite dirty page batch")
}

pub fn encode_commit_stage_complete(complete: CommitStageComplete) -> Result<Vec<u8>> {
	VersionedCommitStageComplete::wrap_latest(complete)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite commit stage complete")
}

pub fn decode_commit_stage_complete(payload: &[u8]) -> Result<CommitStageComplete> {
	VersionedCommitStageComplete::deserialize_with_embedded_version(payload)
		.context("decode sqlite commit stage complete")
}

pub fn encode_commit_stage_finalized(finalized: CommitStageFinalized) -> Result<Vec<u8>> {
	VersionedCommitStageFinalized::wrap_latest(finalized)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite commit stage finalized")
}

pub fn decode_commit_stage_finalized(payload: &[u8]) -> Result<CommitStageFinalized> {
	VersionedCommitStageFinalized::deserialize_with_embedded_version(payload)
		.context("decode sqlite commit stage finalized")
}

pub fn encode_delta_object_meta(meta: DeltaObjectMeta) -> Result<Vec<u8>> {
	VersionedDeltaObjectMeta::wrap_latest(meta)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite delta object meta")
}

pub fn decode_delta_object_meta(payload: &[u8]) -> Result<DeltaObjectMeta> {
	VersionedDeltaObjectMeta::deserialize_with_embedded_version(payload)
		.context("decode sqlite delta object meta")
}

pub fn encode_delta_manifest(manifest: DeltaManifest) -> Result<Vec<u8>> {
	VersionedDeltaManifest::wrap_latest(manifest)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite delta manifest")
}

pub fn decode_delta_manifest(payload: &[u8]) -> Result<DeltaManifest> {
	VersionedDeltaManifest::deserialize_with_embedded_version(payload)
		.context("decode sqlite delta manifest")
}

pub fn encode_delta_page_index_entry(entry: DeltaPageIndexEntry) -> Result<Vec<u8>> {
	VersionedDeltaPageIndexEntry::wrap_latest(entry)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite delta page index entry")
}

pub fn decode_delta_page_index_entry(payload: &[u8]) -> Result<DeltaPageIndexEntry> {
	VersionedDeltaPageIndexEntry::deserialize_with_embedded_version(payload)
		.context("decode sqlite delta page index entry")
}

pub fn encode_hot_shard_manifest(manifest: HotShardManifest) -> Result<Vec<u8>> {
	let mut bytes = HOT_SHARD_MANIFEST_MAGIC.to_vec();
	bytes.extend(
		VersionedHotShardManifest::wrap_latest(manifest)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
			.context("encode sqlite hot shard manifest")?,
	);
	Ok(bytes)
}

pub fn decode_hot_shard_manifest(payload: &[u8]) -> Result<HotShardManifest> {
	let payload = payload
		.strip_prefix(HOT_SHARD_MANIFEST_MAGIC)
		.context("sqlite hot shard value is not a manifest")?;
	VersionedHotShardManifest::deserialize_with_embedded_version(payload)
		.context("decode sqlite hot shard manifest")
}
