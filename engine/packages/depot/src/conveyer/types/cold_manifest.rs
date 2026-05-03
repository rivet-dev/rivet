use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use vbare::OwnedVersionedData;

use super::ids::DatabaseBranchId;
use super::restore_points::RestorePointIndexEntry;
use super::serialization::SQLITE_STORAGE_META_VERSION;

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
	pub restore_points: Vec<RestorePointIndexEntry>,
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

enum VersionedColdManifestIndex {
	V1(ColdManifestIndex),
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
			_ => bail!("invalid depot ColdManifestIndex version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedColdManifestChunk {
	V1(ColdManifestChunk),
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
			_ => bail!("invalid depot ColdManifestChunk version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
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
