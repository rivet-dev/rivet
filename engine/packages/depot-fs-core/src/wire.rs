//! Domain conversions for generated, typed filesystem read messages.

use crate::{
	Error, Result,
	store::{Row, RowKey, RowRange},
};
use rivet_data::generated::shared_depot_fs_read_v1 as wire;

/// Shared default bounds for native and simulated snapshot readers.
/// Read sessions one tablet holds at once. A held session retains about 1.2 KiB of maintainer
/// heap (`filesystem_session_memory` in depot-worker), so this holds under 20 MiB per tablet.
pub const DEFAULT_MAX_SESSIONS: usize = 16_384;
pub const DEFAULT_READ_SESSION_MS: u64 = 30_000;
/// How long a read session stays registered after its last read. Each read extends it, up to
/// the session's lifetime, so sessions track volumes being read rather than volumes open.
pub const DEFAULT_READ_SESSION_IDLE_MS: u64 = 2_000;
/// The idle timeout of a session opened at the latest position. Nothing reopens such a session
/// at its position, and an HTTP directory cursor resumes it between pages, so it keeps the
/// session lifetime as its idle timeout.
pub const DEFAULT_LATEST_SESSION_IDLE_MS: u64 = DEFAULT_READ_SESSION_MS;
pub const DEFAULT_MAX_READ_ROWS: usize = 1024;
pub const DEFAULT_MAX_READ_BYTES: usize = 4 * 1024 * 1024;
/// Most rows one KV prefetch of an open returns: one page, as the opening scans them. A
/// prefetch that returns fewer covers its whole range; one that returns exactly this many
/// covers the range only through its last row.
pub const PREFETCH_KV_ROWS: usize = 256;

impl From<RowKey> for wire::FsRowKey {
	fn from(key: RowKey) -> Self {
		match key {
			RowKey::Format => Self::FsFormatKey,
			RowKey::Inode(ino) => Self::FsInodeKey(wire::FsInodeKey { ino }),
			RowKey::Dentry(ino, name) => Self::FsDentryKey(wire::FsDentryKey {
				ino,
				name: name.into(),
			}),
			RowKey::Link(ino, parent, name) => Self::FsLinkKey(wire::FsLinkKey {
				ino,
				parent,
				name: name.into(),
			}),
			RowKey::Xattr(ino, name) => Self::FsXattrKey(wire::FsXattrKey {
				ino,
				name: name.into(),
			}),
			RowKey::Kv(ino, key) => Self::FsKvKey(wire::FsKvKey {
				ino,
				key: key.into(),
			}),
			RowKey::Orphan(ino) => Self::FsOrphanKey(wire::FsOrphanKey { ino }),
			RowKey::Chunk(ino, index) => Self::FsChunkKey(wire::FsChunkKey { ino, index }),
		}
	}
}
impl From<wire::FsRowKey> for RowKey {
	fn from(key: wire::FsRowKey) -> Self {
		match key {
			wire::FsRowKey::FsFormatKey => Self::Format,
			wire::FsRowKey::FsInodeKey(v) => Self::Inode(v.ino),
			wire::FsRowKey::FsDentryKey(v) => Self::Dentry(v.ino, v.name.to_vec()),
			wire::FsRowKey::FsLinkKey(v) => Self::Link(v.ino, v.parent, v.name.to_vec()),
			wire::FsRowKey::FsXattrKey(v) => Self::Xattr(v.ino, v.name.to_vec()),
			wire::FsRowKey::FsKvKey(v) => Self::Kv(v.ino, v.key.to_vec()),
			wire::FsRowKey::FsOrphanKey(v) => Self::Orphan(v.ino),
			wire::FsRowKey::FsChunkKey(v) => Self::Chunk(v.ino, v.index),
		}
	}
}
impl From<RowRange> for wire::FsRowRange {
	fn from(range: RowRange) -> Self {
		match range {
			RowRange::All => Self::FsAllRows,
			RowRange::MetadataAll => Self::FsAllMetadataRows,
			RowRange::ChunksAll => Self::FsAllChunkRows,
			RowRange::Metadata(ino) => Self::FsMetadataRows(wire::FsMetadataRows { ino }),
			RowRange::Chunks(ino) => Self::FsChunkRows(wire::FsChunkRows { ino, first: 0 }),
			RowRange::ChunkSuffix { ino, first } => {
				Self::FsChunkRows(wire::FsChunkRows { ino, first })
			}
			RowRange::Directory(ino) => Self::FsDirectoryRows(wire::FsDirectoryRows { ino }),
			RowRange::Xattrs(ino) => Self::FsXattrRows(wire::FsXattrRows { ino }),
			RowRange::Kv { ino, start, end } => Self::FsKvRows(wire::FsKvRows {
				ino,
				start: start.into(),
				end: end.map(Into::into),
			}),
			RowRange::Orphans => Self::FsOrphanRows,
		}
	}
}
impl From<wire::FsRowRange> for RowRange {
	fn from(range: wire::FsRowRange) -> Self {
		match range {
			wire::FsRowRange::FsAllRows => Self::All,
			wire::FsRowRange::FsAllMetadataRows => Self::MetadataAll,
			wire::FsRowRange::FsAllChunkRows => Self::ChunksAll,
			wire::FsRowRange::FsMetadataRows(v) => Self::Metadata(v.ino),
			wire::FsRowRange::FsChunkRows(v) => {
				if v.first == 0 {
					Self::Chunks(v.ino)
				} else {
					Self::ChunkSuffix {
						ino: v.ino,
						first: v.first,
					}
				}
			}
			wire::FsRowRange::FsDirectoryRows(v) => Self::Directory(v.ino),
			wire::FsRowRange::FsXattrRows(v) => Self::Xattrs(v.ino),
			wire::FsRowRange::FsKvRows(v) => Self::Kv {
				ino: v.ino,
				start: v.start.to_vec(),
				end: v.end.map(Into::into),
			},
			wire::FsRowRange::FsOrphanRows => Self::Orphans,
		}
	}
}
impl From<Row> for wire::FsRowValue {
	fn from(row: Row) -> Self {
		match row {
			Row::Format(v) => Self::FsFormat(v),
			Row::Inode(v) => Self::FsInode(v),
			Row::Dentry(v) => Self::FsDentry(v),
			Row::Bytes(v) => Self::FsBytes(v),
			Row::Unit => Self::FsPresent,
		}
	}
}
pub fn decode_row(row: wire::FsReadRow) -> Result<(RowKey, Row)> {
	let key = RowKey::from(row.key);
	let value = match (&key, row.value) {
		(RowKey::Format, wire::FsRowValue::FsFormat(v))
			if v.inline_max_bytes <= crate::CHUNK_BYTES as u64 =>
		{
			Row::Format(v)
		}
		(RowKey::Inode(_), wire::FsRowValue::FsInode(v)) => Row::Inode(v),
		(RowKey::Dentry(..), wire::FsRowValue::FsDentry(v)) => Row::Dentry(v),
		(RowKey::Xattr(..) | RowKey::Kv(..) | RowKey::Chunk(..), wire::FsRowValue::FsBytes(v)) => {
			Row::Bytes(v)
		}
		(RowKey::Link(..) | RowKey::Orphan(_), wire::FsRowValue::FsPresent) => Row::Unit,
		_ => return Err(Error::Corrupt),
	};
	Ok((key, value))
}

impl From<crate::forks::ChunkLookup> for wire::FsChunkLookup {
	fn from(value: crate::forks::ChunkLookup) -> Self {
		match value {
			crate::forks::ChunkLookup::Resolved(bytes) => {
				Self::FsResolvedChunk(wire::FsResolvedChunk {
					bytes: bytes.map(Into::into),
				})
			}
			crate::forks::ChunkLookup::Base { max_bytes } => {
				Self::FsInheritedChunk(wire::FsInheritedChunk {
					max_bytes: max_bytes as u32,
				})
			}
		}
	}
}
impl TryFrom<wire::FsChunkLookup> for crate::forks::ChunkLookup {
	type Error = Error;
	fn try_from(value: wire::FsChunkLookup) -> Result<Self> {
		match value {
			wire::FsChunkLookup::FsResolvedChunk(v)
				if v.bytes
					.as_ref()
					.is_none_or(|b| b.len() <= crate::CHUNK_BYTES) =>
			{
				Ok(Self::Resolved(v.bytes.map(Into::into)))
			}
			wire::FsChunkLookup::FsInheritedChunk(v)
				if v.max_bytes > 0 && v.max_bytes as usize <= crate::CHUNK_BYTES =>
			{
				Ok(Self::Base {
					max_bytes: v.max_bytes as usize,
				})
			}
			_ => Err(Error::Corrupt),
		}
	}
}

/// Held-image queries read one immutable layer and never recurse into its bases.
/// They can bypass an ordinary reader's store barrier without observing partial apply.
pub fn source_link(operation: &wire::FsReadOperation) -> Option<&crate::FsBaseLink> {
	match operation {
		wire::FsReadOperation::FsBaseChunksRequest(v) => Some(&v.link),
		wire::FsReadOperation::FsForkMetadataRequest(v) => Some(&v.link),
		wire::FsReadOperation::FsBaseKeysRequest(v) => Some(&v.link),
		_ => None,
	}
}

/// Source routing must name the exact held incarnation. The source store separately
/// proves the checkpoint's position and hold before registering its snapshot.
pub fn valid_source_target(request: &wire::FsReadRequest) -> bool {
	source_link(&request.operation).is_some_and(|link| {
		request.client_epoch.is_none()
			&& request.volume_log_key().is_some_and(|key| {
				key.volume_key == link.volume_key && key.volume_incarnation == link.incarnation
			})
	})
}
