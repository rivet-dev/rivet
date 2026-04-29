use anyhow::{Context, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2};

pub enum DBHead {
	V1(v1::DBHead),
	V2(v2::DBHead),
}

impl OwnedVersionedData for DBHead {
	type Latest = v2::DBHead;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(convert_db_head_v1_to_v2(data)),
			Self::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite-storage db head version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
				Self::V2(data) => serde_bare::to_vec(&convert_db_head_v2_to_v1(data)?)
					.map_err(Into::into),
			},
			2 => match self {
				Self::V1(data) => serde_bare::to_vec(&convert_db_head_v1_to_v2(data))
					.map_err(Into::into),
				Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid sqlite-storage db head version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}

pub enum PreloadHints {
	V2(v2::PreloadHints),
}

impl OwnedVersionedData for PreloadHints {
	type Latest = v2::PreloadHints;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite-storage preload hints version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			2 => match self {
				Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid sqlite-storage preload hints version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}

/// Encode the latest `DBHead` with an embedded 2-byte version prefix.
pub fn encode_db_head(head: v2::DBHead) -> Result<Vec<u8>> {
	DBHead::wrap_latest(head)
		.serialize_with_embedded_version(crate::SQLITE_STORAGE_PROTOCOL_VERSION)
		.context("encode sqlite db head")
}

/// Decode a versioned `DBHead` payload into the latest schema variant.
pub fn decode_db_head(payload: &[u8]) -> Result<v2::DBHead> {
	DBHead::deserialize_with_embedded_version(payload).context("decode sqlite db head")
}

pub fn encode_preload_hints(hints: v2::PreloadHints) -> Result<Vec<u8>> {
	PreloadHints::wrap_latest(hints)
		.serialize_with_embedded_version(crate::SQLITE_STORAGE_PROTOCOL_VERSION)
		.context("encode sqlite preload hints")
}

pub fn decode_preload_hints(payload: &[u8]) -> Result<v2::PreloadHints> {
	PreloadHints::deserialize_with_embedded_version(payload)
		.context("decode sqlite preload hints")
}

fn convert_sqlite_origin_v1_to_v2(origin: v1::SqliteOrigin) -> v2::SqliteOrigin {
	match origin {
		v1::SqliteOrigin::CreatedOnV2 => v2::SqliteOrigin::CreatedOnV2,
		v1::SqliteOrigin::MigratedFromV1 => v2::SqliteOrigin::MigratedFromV1,
		v1::SqliteOrigin::MigrationFromV1InProgress => {
			v2::SqliteOrigin::MigrationFromV1InProgress
		}
	}
}

fn convert_sqlite_origin_v2_to_v1(origin: v2::SqliteOrigin) -> v1::SqliteOrigin {
	match origin {
		v2::SqliteOrigin::CreatedOnV2 => v1::SqliteOrigin::CreatedOnV2,
		v2::SqliteOrigin::MigratedFromV1 => v1::SqliteOrigin::MigratedFromV1,
		v2::SqliteOrigin::MigrationFromV1InProgress => {
			v1::SqliteOrigin::MigrationFromV1InProgress
		}
	}
}

fn convert_db_head_v1_to_v2(head: v1::DBHead) -> v2::DBHead {
	v2::DBHead {
		schema_version: head.schema_version,
		generation: head.generation,
		head_txid: head.head_txid,
		next_txid: head.next_txid,
		materialized_txid: head.materialized_txid,
		db_size_pages: head.db_size_pages,
		page_size: head.page_size,
		shard_size: head.shard_size,
		creation_ts_ms: head.creation_ts_ms,
		sqlite_storage_used: head.sqlite_storage_used,
		sqlite_max_storage: head.sqlite_max_storage,
		origin: convert_sqlite_origin_v1_to_v2(head.origin),
	}
}

fn convert_db_head_v2_to_v1(head: v2::DBHead) -> Result<v1::DBHead> {
	Ok(v1::DBHead {
		schema_version: head.schema_version,
		generation: head.generation,
		head_txid: head.head_txid,
		next_txid: head.next_txid,
		materialized_txid: head.materialized_txid,
		db_size_pages: head.db_size_pages,
		page_size: head.page_size,
		shard_size: head.shard_size,
		creation_ts_ms: head.creation_ts_ms,
		sqlite_storage_used: head.sqlite_storage_used,
		sqlite_max_storage: head.sqlite_max_storage,
		origin: convert_sqlite_origin_v2_to_v1(head.origin),
	})
}
