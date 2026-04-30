use anyhow::{Context, Result, bail};
use gas::prelude::Id;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use vbare::OwnedVersionedData;

pub const SQLITE_STORAGE_META_VERSION: u16 = 1;
pub const SQLITE_STORAGE_COLD_SCHEMA_VERSION: u32 = 1;
pub const SQLITE_PAGE_SIZE: u32 = crate::keys::PAGE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NamespaceId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NamespacePointerId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DatabasePointerId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NamespaceBranchId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DatabaseBranchId(Uuid);

macro_rules! impl_uuid_id {
	($type:ident) => {
		impl $type {
			pub fn new_v4() -> Self {
				Self(Uuid::new_v4())
			}

			pub fn nil() -> Self {
				Self(Uuid::nil())
			}

			pub fn from_uuid(uuid: Uuid) -> Self {
				Self(uuid)
			}

			pub fn as_uuid(&self) -> Uuid {
				self.0
			}
		}
	};
}

impl_uuid_id!(NamespaceId);
impl_uuid_id!(NamespacePointerId);
impl_uuid_id!(DatabasePointerId);
impl_uuid_id!(NamespaceBranchId);
impl_uuid_id!(DatabaseBranchId);

impl NamespaceId {
	pub fn from_gas_id(id: Id) -> Self {
		let bytes = id.as_bytes();
		let uuid = Uuid::from_slice(&bytes[1..17]).expect("gas v1 ids carry 16 uuid bytes");
		Self(uuid)
	}
}

pub type DatabaseIdStr = String;
pub type NamespaceIdUuid = NamespaceId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct BookmarkStr(String);

impl BookmarkStr {
	pub fn new(bookmark: impl Into<String>) -> Result<Self> {
		let bookmark = bookmark.into();
		let bytes = bookmark.as_bytes();
		if bytes.len() != 33
			|| bytes[16] != b'-'
			|| !bytes[..16].iter().all(|byte| byte.is_ascii_hexdigit())
			|| !bytes[17..].iter().all(|byte| byte.is_ascii_hexdigit())
		{
			bail!("sqlite bookmark must match 0000000000000000-0000000000000000");
		}

		Ok(Self(bookmark))
	}

	pub fn format(ts_ms: i64, txid: u64) -> Result<Self> {
		if ts_ms < 0 {
			bail!("sqlite bookmark timestamp must be non-negative");
		}

		Self::new(format!("{:016x}-{txid:016x}", ts_ms as u64))
	}

	pub fn parse(&self) -> Result<(i64, u64)> {
		let ts_ms = u64::from_str_radix(&self.0[..16], 16)
			.context("parse sqlite bookmark timestamp")?;
		let txid = u64::from_str_radix(&self.0[17..], 16).context("parse sqlite bookmark txid")?;
		let ts_ms = i64::try_from(ts_ms).context("sqlite bookmark timestamp exceeds i64 range")?;

		Ok((ts_ms, txid))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn into_string(self) -> String {
		self.0
	}
}

impl<'de> Deserialize<'de> for BookmarkStr {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let bookmark = String::deserialize(deserializer)?;
		Self::new(bookmark).map_err(serde::de::Error::custom)
	}
}

impl TryFrom<String> for BookmarkStr {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self> {
		Self::new(value)
	}
}

impl TryFrom<&str> for BookmarkStr {
	type Error = anyhow::Error;

	fn try_from(value: &str) -> Result<Self> {
		Self::new(value)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkRef {
	pub bookmark: BookmarkStr,
	pub resolved_versionstamp: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedVersionstamp {
	pub versionstamp: [u8; 16],
	pub bookmark: Option<BookmarkRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchState {
	Live,
	Frozen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceBranchRecord {
	pub branch_id: NamespaceBranchId,
	pub parent: Option<NamespaceBranchId>,
	pub parent_versionstamp: Option<[u8; 16]>,
	pub root_versionstamp: [u8; 16],
	pub fork_depth: u8,
	pub created_at_ms: i64,
	pub created_from_bookmark: Option<BookmarkRef>,
	pub state: BranchState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseBranchRecord {
	pub branch_id: DatabaseBranchId,
	pub namespace_branch: NamespaceBranchId,
	pub parent: Option<DatabaseBranchId>,
	pub parent_versionstamp: Option<[u8; 16]>,
	pub root_versionstamp: [u8; 16],
	pub fork_depth: u8,
	pub created_at_ms: i64,
	pub created_from_bookmark: Option<BookmarkRef>,
	pub state: BranchState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespacePointer {
	pub current_branch: NamespaceBranchId,
	pub last_swapped_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabasePointer {
	pub current_branch: DatabaseBranchId,
	pub last_swapped_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinStatus {
	Pending,
	Ready,
	Failed,
}

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
pub struct ColdManifestIndex {
	pub schema_version: u32,
	pub branch_id: DatabaseBranchId,
	pub chunks: Vec<ColdManifestChunkRef>,
	pub last_pass_at_ms: i64,
	pub last_pass_versionstamp: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdManifestChunkRef {
	pub object_key: String,
	pub pass_versionstamp: [u8; 16],
	pub min_versionstamp: [u8; 16],
	pub max_versionstamp: [u8; 16],
	pub byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdManifestChunk {
	pub schema_version: u32,
	pub branch_id: DatabaseBranchId,
	pub pass_versionstamp: [u8; 16],
	pub layers: Vec<LayerEntry>,
	pub bookmarks: Vec<BookmarkIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerEntry {
	pub kind: LayerKind,
	pub shard_id: Option<u32>,
	pub min_txid: u64,
	pub max_txid: u64,
	pub min_versionstamp: [u8; 16],
	pub max_versionstamp: [u8; 16],
	pub byte_size: u64,
	pub checksum: u64,
	pub object_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerKind {
	Image,
	Delta,
	Pin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkIndexEntry {
	pub schema_version: u32,
	pub bookmark_str: BookmarkStr,
	pub pinned: bool,
	pub pin_object_key: Option<String>,
	pub pin_status: PinStatus,
	pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerSnapshot {
	pub schema_version: u32,
	pub pass_versionstamp: [u8; 16],
	pub databases: Vec<(DatabaseIdStr, NamespaceBranchId, DatabaseBranchId)>,
	pub namespaces: Vec<(NamespaceIdUuid, NamespaceBranchId)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkRecord {
	pub bookmark: BookmarkStr,
	pub database_branch_id: DatabaseBranchId,
	pub created_at_ms: i64,
	pub resolved: Option<ResolvedVersionstamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedBookmarkRecord {
	pub bookmark: BookmarkStr,
	pub database_branch_id: DatabaseBranchId,
	pub versionstamp: [u8; 16],
	pub status: PinStatus,
	pub pin_object_key: Option<String>,
	pub created_at_ms: i64,
	pub updated_at_ms: i64,
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
			_ => bail!("invalid sqlite-storage CommitRow version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedDatabaseBranchRecord {
	V1(DatabaseBranchRecord),
}

impl OwnedVersionedData for VersionedDatabaseBranchRecord {
	type Latest = DatabaseBranchRecord;

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
			_ => bail!("invalid sqlite-storage DatabaseBranchRecord version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedDatabasePointer {
	V1(DatabasePointer),
}

impl OwnedVersionedData for VersionedDatabasePointer {
	type Latest = DatabasePointer;

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
			_ => bail!("invalid sqlite-storage DatabasePointer version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedNamespaceBranchRecord {
	V1(NamespaceBranchRecord),
}

impl OwnedVersionedData for VersionedNamespaceBranchRecord {
	type Latest = NamespaceBranchRecord;

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
			_ => bail!("invalid sqlite-storage NamespaceBranchRecord version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedNamespacePointer {
	V1(NamespacePointer),
}

impl OwnedVersionedData for VersionedNamespacePointer {
	type Latest = NamespacePointer;

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
			_ => bail!("invalid sqlite-storage NamespacePointer version: {version}"),
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

enum VersionedBookmarkRecord {
	V1(BookmarkRecord),
}

enum VersionedPinnedBookmarkRecord {
	V1(PinnedBookmarkRecord),
}

enum VersionedColdManifestIndex {
	V1(ColdManifestIndex),
}

enum VersionedColdManifestChunk {
	V1(ColdManifestChunk),
}

enum VersionedPointerSnapshot {
	V1(PointerSnapshot),
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

impl OwnedVersionedData for VersionedBookmarkRecord {
	type Latest = BookmarkRecord;

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
			_ => bail!("invalid sqlite-storage BookmarkRecord version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl OwnedVersionedData for VersionedPinnedBookmarkRecord {
	type Latest = PinnedBookmarkRecord;

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
			_ => bail!("invalid sqlite-storage PinnedBookmarkRecord version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl OwnedVersionedData for VersionedColdManifestIndex {
	type Latest = ColdManifestIndex;

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
			_ => bail!("invalid sqlite-storage ColdManifestIndex version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl OwnedVersionedData for VersionedColdManifestChunk {
	type Latest = ColdManifestChunk;

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
			_ => bail!("invalid sqlite-storage ColdManifestChunk version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl OwnedVersionedData for VersionedPointerSnapshot {
	type Latest = PointerSnapshot;

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
			_ => bail!("invalid sqlite-storage PointerSnapshot version: {version}"),
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

pub fn encode_commit_row(row: CommitRow) -> Result<Vec<u8>> {
	VersionedCommitRow::wrap_latest(row)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite commit row")
}

pub fn decode_commit_row(payload: &[u8]) -> Result<CommitRow> {
	VersionedCommitRow::deserialize_with_embedded_version(payload)
		.context("decode sqlite commit row")
}

pub fn encode_database_branch_record(record: DatabaseBranchRecord) -> Result<Vec<u8>> {
	VersionedDatabaseBranchRecord::wrap_latest(record)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite database branch record")
}

pub fn decode_database_branch_record(payload: &[u8]) -> Result<DatabaseBranchRecord> {
	VersionedDatabaseBranchRecord::deserialize_with_embedded_version(payload)
		.context("decode sqlite database branch record")
}

pub fn encode_database_pointer(pointer: DatabasePointer) -> Result<Vec<u8>> {
	VersionedDatabasePointer::wrap_latest(pointer)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite database pointer")
}

pub fn decode_database_pointer(payload: &[u8]) -> Result<DatabasePointer> {
	VersionedDatabasePointer::deserialize_with_embedded_version(payload)
		.context("decode sqlite database pointer")
}

pub fn encode_namespace_branch_record(record: NamespaceBranchRecord) -> Result<Vec<u8>> {
	VersionedNamespaceBranchRecord::wrap_latest(record)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite namespace branch record")
}

pub fn decode_namespace_branch_record(payload: &[u8]) -> Result<NamespaceBranchRecord> {
	VersionedNamespaceBranchRecord::deserialize_with_embedded_version(payload)
		.context("decode sqlite namespace branch record")
}

pub fn encode_namespace_pointer(pointer: NamespacePointer) -> Result<Vec<u8>> {
	VersionedNamespacePointer::wrap_latest(pointer)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite namespace pointer")
}

pub fn decode_namespace_pointer(payload: &[u8]) -> Result<NamespacePointer> {
	VersionedNamespacePointer::deserialize_with_embedded_version(payload)
		.context("decode sqlite namespace pointer")
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

pub fn encode_bookmark_record(record: BookmarkRecord) -> Result<Vec<u8>> {
	VersionedBookmarkRecord::wrap_latest(record)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite bookmark record")
}

pub fn decode_bookmark_record(payload: &[u8]) -> Result<BookmarkRecord> {
	VersionedBookmarkRecord::deserialize_with_embedded_version(payload)
		.context("decode sqlite bookmark record")
}

pub fn encode_pinned_bookmark_record(record: PinnedBookmarkRecord) -> Result<Vec<u8>> {
	VersionedPinnedBookmarkRecord::wrap_latest(record)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite pinned bookmark record")
}

pub fn decode_pinned_bookmark_record(payload: &[u8]) -> Result<PinnedBookmarkRecord> {
	VersionedPinnedBookmarkRecord::deserialize_with_embedded_version(payload)
		.context("decode sqlite pinned bookmark record")
}

pub fn encode_cold_manifest_index(index: ColdManifestIndex) -> Result<Vec<u8>> {
	VersionedColdManifestIndex::wrap_latest(index)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite cold manifest index")
}

pub fn decode_cold_manifest_index(payload: &[u8]) -> Result<ColdManifestIndex> {
	VersionedColdManifestIndex::deserialize_with_embedded_version(payload)
		.context("decode sqlite cold manifest index")
}

pub fn encode_cold_manifest_chunk(chunk: ColdManifestChunk) -> Result<Vec<u8>> {
	VersionedColdManifestChunk::wrap_latest(chunk)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite cold manifest chunk")
}

pub fn decode_cold_manifest_chunk(payload: &[u8]) -> Result<ColdManifestChunk> {
	VersionedColdManifestChunk::deserialize_with_embedded_version(payload)
		.context("decode sqlite cold manifest chunk")
}

pub fn encode_pointer_snapshot(snapshot: PointerSnapshot) -> Result<Vec<u8>> {
	VersionedPointerSnapshot::wrap_latest(snapshot)
		.serialize_with_embedded_version(SQLITE_STORAGE_META_VERSION)
		.context("encode sqlite pointer snapshot")
}

pub fn decode_pointer_snapshot(payload: &[u8]) -> Result<PointerSnapshot> {
	VersionedPointerSnapshot::deserialize_with_embedded_version(payload)
		.context("decode sqlite pointer snapshot")
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
			post_apply_checksum: 9,
			branch_id: super::DatabaseBranchId::nil(),
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
