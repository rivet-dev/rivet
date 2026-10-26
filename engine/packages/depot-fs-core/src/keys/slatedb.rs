//! Formal filesystem addresses. Sequence order is descending only for versioned rows.
use depot_types::PartitionKeyHash;
use rivet_keys::keys::codec::DescendingU64;
use uuid::Uuid;

rivet_keys::slatedb_key! {
	pub FormatKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_FORMAT, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsFormatRowData);
}
rivet_keys::slatedb_prefix! {
	pub FormatPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_FORMAT];
	next: bytes::Bytes;
}

rivet_keys::slatedb_key! {
	pub InodeKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_INODE, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsInodeRowData);
}
rivet_keys::slatedb_prefix! {
	pub InodePrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_INODE];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub DentryKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, entry: bytes::Bytes, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_DENTRY, {entry}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsDentryRowData);
}
rivet_keys::slatedb_prefix! {
	pub DentryPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, entry: bytes::Bytes) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_DENTRY, {entry}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub LinkKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, parent: u64, entry: bytes::Bytes, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_LINK, {parent}, {entry}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsLinkRowData);
}
rivet_keys::slatedb_prefix! {
	pub LinkPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, parent: u64, entry: bytes::Bytes) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_LINK, {parent}, {entry}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub XattrKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, attribute: bytes::Bytes, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_XATTR, {attribute}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsXattrRowData);
}
rivet_keys::slatedb_prefix! {
	pub XattrPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, attribute: bytes::Bytes) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_XATTR, {attribute}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub KvKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, key: bytes::Bytes, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_KV, {key}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsKvRowData);
}
rivet_keys::slatedb_prefix! {
	pub KvPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, key: bytes::Bytes) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_KV, {key}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub OrphanKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, ino: u64, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_ORPHAN, {ino}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsOrphanRowData);
}
rivet_keys::slatedb_prefix! {
	pub OrphanPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_ORPHAN, {ino}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub ChunkKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, chunk: u64, seq: DescendingU64)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_DATA, {ino}, {chunk}, {seq}];
	value: vbare_selected(rivet_data::versioned::DepotFsChunkRowData);
}
rivet_keys::slatedb_prefix! {
	pub ChunkPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64, chunk: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_DATA, {ino}, {chunk}];
	next: bytes::Bytes;
}
rivet_keys::slatedb_key! {
	pub VolumeCheckpointKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, seq: u64, checkpoint: bytes::Bytes) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_CHECKPOINT, {seq}, {checkpoint}];
	value: vbare_selected(rivet_data::versioned::DepotFsVolumeCheckpointData);
}
rivet_keys::slatedb_key! {
	pub ForkHoldKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, seq: u64, checkpoint: bytes::Bytes, fork_incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_FORKHOLD, {seq}, {checkpoint}, {fork_incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsForkHoldData);
}
rivet_keys::slatedb_key! {
	pub PendingForkCheckKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [FS_PENDING, {hash}, {name}, FS_FORK_CHECK, {incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsPendingForkCheckData);
}
rivet_keys::slatedb_key! {
	pub PendingMaterializeKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [FS_PENDING, {hash}, {name}, FS_MATERIALIZE, {incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsPendingMaterializeData);
}
// `PendingDeleteKey` marks a `Deleted` volume record, so the start scan finds every
// deletion to drive without visiting volume data. It is written and removed in the
// batches that write and remove the record.
rivet_keys::slatedb_key! {
	pub PendingDeleteKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [FS_PENDING, {hash}, {name}, FS_DELETE, {incarnation}];
	value: unit;
}
rivet_keys::slatedb_key! {
	pub PendingSweepKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [FS_PENDING, {hash}, {name}, FS_SWEEP, {incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsPendingSweepData);
}
// `VolumeRecordKey` is the name's pointer: a copy of the stored record with the highest
// `incarnation_epoch`, so an open of an unloaded name reads one row. Every incarnation also
// has its own `VolumeIncarnationRecordKey`, written in the same batch as the pointer, which a
// lookup of a known incarnation reads. The incarnation row survives the record's removal from
// the pointer and goes with the incarnation's data sweep.
rivet_keys::slatedb_key! {
	pub VolumeRecordKey(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}, RECORD];
	value: vbare_selected(rivet_data::versioned::DepotFsVolumeRecordData);
}
rivet_keys::slatedb_key! {
	pub VolumeIncarnationRecordKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}, FS_RECORD];
	value: vbare_selected(rivet_data::versioned::DepotFsVolumeRecordData);
}
// `RepairAuditKey` holds the last logged state of one operator repair call of the name, beside
// the name's own rows so it moves with a split. It is written only by the call's audit op.
rivet_keys::slatedb_key! {
	pub RepairAuditKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, operation: Uuid) => [VOLUME, {hash}, {name}, FS_REPAIR_AUDIT, {operation}];
	value: vbare(rivet_data::versioned::DepotRepairAuditData, <rivet_data::versioned::DepotRepairAuditData as vbare::Versioned>::LATEST);
}
rivet_keys::slatedb_prefix! {
	pub RepairAuditPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}, FS_REPAIR_AUDIT];
}
rivet_keys::slatedb_prefix! {
	pub IncarnationPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}];
}
rivet_keys::slatedb_prefix! {
	pub MetadataPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}];
}
rivet_keys::slatedb_prefix! {
	pub DataPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_DATA, {ino}];
}

/// Identity prefix for one incarnation. Raw partition bytes remain in its volume record.
#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Scope {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
impl Scope {
	pub fn new(key: &depot_types::VolumeKey, incarnation: Uuid) -> Self {
		Self {
			hash: depot_types::partition_key_hash(key),
			name: key.name.clone().map(Into::into),
			incarnation,
		}
	}
}

use crate::store::{Row, RowKey};
use rivet_data::generated::depot_fs_rows_v1 as wire;
use rivet_keys::keys::codec::{AtVersion, FormalKey, ValueCodec};
use rivet_keys::keys::{KeyAddress, PrefixAddress};

pub fn row_prefix(scope: &Scope, row: &RowKey) -> bytes::Bytes {
	let Scope {
		hash,
		name,
		incarnation,
	} = scope;
	match row {
		RowKey::Format => FormatPrefix::new(*hash, name.clone(), *incarnation, 0).pack(),
		RowKey::Inode(ino) => InodePrefix::new(*hash, name.clone(), *incarnation, *ino).pack(),
		RowKey::Dentry(ino, entry) => DentryPrefix::new(
			*hash,
			name.clone(),
			*incarnation,
			*ino,
			entry.clone().into(),
		)
		.pack(),
		RowKey::Link(ino, parent, entry) => LinkPrefix::new(
			*hash,
			name.clone(),
			*incarnation,
			*ino,
			*parent,
			entry.clone().into(),
		)
		.pack(),
		RowKey::Xattr(ino, attribute) => XattrPrefix::new(
			*hash,
			name.clone(),
			*incarnation,
			*ino,
			attribute.clone().into(),
		)
		.pack(),
		RowKey::Kv(ino, key) => {
			KvPrefix::new(*hash, name.clone(), *incarnation, *ino, key.clone().into()).pack()
		}
		RowKey::Orphan(ino) => OrphanPrefix::new(*hash, name.clone(), *incarnation, 0, *ino).pack(),
		RowKey::Chunk(ino, chunk) => {
			ChunkPrefix::new(*hash, name.clone(), *incarnation, *ino, *chunk).pack()
		}
	}
}

/// Encodes one filesystem row at `version`, the `depot.fs_rows` version its writer selected.
pub fn encode_row(
	scope: &Scope,
	row: &RowKey,
	seq: u64,
	value: Option<Row>,
	version: u16,
) -> anyhow::Result<(bytes::Bytes, bytes::Bytes)> {
	let Scope {
		hash,
		name,
		incarnation,
	} = scope;
	let seq = DescendingU64(seq);
	match row {
		RowKey::Format => {
			let key = FormatKey::new(*hash, name.clone(), *incarnation, 0, seq);
			let value = match value {
				Some(Row::Format(value)) if value.inline_max_bytes <= crate::CHUNK_BYTES as u64 => {
					wire::FormatRow::FsFormat(value)
				}
				None => wire::FormatRow::FsDeleted,
				_ => anyhow::bail!("invalid filesystem format"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Inode(ino) => {
			let key = InodeKey::new(*hash, name.clone(), *incarnation, *ino, seq);
			let value = match value {
				Some(Row::Inode(value)) => wire::InodeRow::FsInode(value),
				None => wire::InodeRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Dentry(ino, entry) => {
			let key = DentryKey::new(
				*hash,
				name.clone(),
				*incarnation,
				*ino,
				entry.clone().into(),
				seq,
			);
			let value = match value {
				Some(Row::Dentry(value)) => wire::DentryRow::FsDentry(value),
				None => wire::DentryRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Link(ino, parent, entry) => {
			let key = LinkKey::new(
				*hash,
				name.clone(),
				*incarnation,
				*ino,
				*parent,
				entry.clone().into(),
				seq,
			);
			let value = match value {
				Some(Row::Unit) => wire::LinkRow::FsPresent,
				None => wire::LinkRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Xattr(ino, attribute) => {
			let key = XattrKey::new(
				*hash,
				name.clone(),
				*incarnation,
				*ino,
				attribute.clone().into(),
				seq,
			);
			let value = match value {
				Some(Row::Bytes(value)) => wire::XattrRow::FsBytes(value.clone()),
				None => wire::XattrRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Kv(ino, key) => {
			let key = KvKey::new(
				*hash,
				name.clone(),
				*incarnation,
				*ino,
				key.clone().into(),
				seq,
			);
			let value = match value {
				Some(Row::Bytes(value)) => wire::KvRow::FsBytes(value.clone()),
				None => wire::KvRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Orphan(ino) => {
			let key = OrphanKey::new(*hash, name.clone(), *incarnation, 0, *ino, seq);
			let value = match value {
				Some(Row::Unit) => wire::OrphanRow::FsPresent,
				None => wire::OrphanRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
		RowKey::Chunk(ino, chunk) => {
			let key = ChunkKey::new(*hash, name.clone(), *incarnation, *ino, *chunk, seq);
			let value = match value {
				Some(Row::Bytes(value)) => wire::ChunkRow::FsBytes(value.clone()),
				None => wire::ChunkRow::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((key.pack(), key.serialize(AtVersion::new(value, version))?))
		}
	}
}

/// Decodes a stored volume record into its own buffer. Maintainer state keeps records past
/// the read, and a record sharing SlateDB's buffer would keep the whole cached block alive, so
/// the value is copied once and every field shares that copy.
pub fn decode_volume_record(stored: &[u8]) -> anyhow::Result<crate::FsVolumeRecord> {
	<VolumeRecordKey as FormalKey>::Codec::decode(bytes::Bytes::copy_from_slice(stored))
}

/// Whether a stored row value is a tombstone. The value is verified and its tag read in place,
/// so nothing is decoded or allocated. Compaction filters ask this of every row they visit.
pub fn row_is_deleted(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<bool> {
	use rivet_data::versioned::{
		DepotFsChunkRowData, DepotFsDentryRowData, DepotFsFormatRowData, DepotFsInodeRowData,
		DepotFsKvRowData, DepotFsLinkRowData, DepotFsOrphanRowData, DepotFsXattrRowData,
	};
	use vbare::Versioned;
	Ok(match row {
		RowKey::Format => match DepotFsFormatRowData::open(bytes)?.get() {
			wire::FormatRowRef::FsFormat(_) => false,
			wire::FormatRowRef::FsDeleted => true,
		},
		RowKey::Inode(..) => match DepotFsInodeRowData::open(bytes)?.get() {
			wire::InodeRowRef::FsInode(_) => false,
			wire::InodeRowRef::FsDeleted => true,
		},
		RowKey::Dentry(..) => match DepotFsDentryRowData::open(bytes)?.get() {
			wire::DentryRowRef::FsDentry(_) => false,
			wire::DentryRowRef::FsDeleted => true,
		},
		RowKey::Link(..) => match DepotFsLinkRowData::open(bytes)?.get() {
			wire::LinkRow::FsPresent => false,
			wire::LinkRow::FsDeleted => true,
		},
		RowKey::Xattr(..) => match DepotFsXattrRowData::open(bytes)?.get() {
			wire::XattrRowRef::FsBytes(_) => false,
			wire::XattrRowRef::FsDeleted => true,
		},
		RowKey::Kv(..) => match DepotFsKvRowData::open(bytes)?.get() {
			wire::KvRowRef::FsBytes(_) => false,
			wire::KvRowRef::FsDeleted => true,
		},
		RowKey::Orphan(..) => match DepotFsOrphanRowData::open(bytes)?.get() {
			wire::OrphanRow::FsPresent => false,
			wire::OrphanRow::FsDeleted => true,
		},
		RowKey::Chunk(..) => match DepotFsChunkRowData::open(bytes)?.get() {
			wire::ChunkRowRef::FsBytes(_) => false,
			wire::ChunkRowRef::FsDeleted => true,
		},
	})
}

/// Decodes a stored row value. `data` rows are slices of `bytes`, so a row kept past the read
/// keeps the source buffer alive.
pub fn decode_row(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<Option<Row>> {
	Ok(match row {
		RowKey::Format => match <FormatKey as FormalKey>::Codec::decode(bytes)? {
			wire::FormatRow::FsFormat(value) => {
				anyhow::ensure!(
					value.inline_max_bytes <= crate::CHUNK_BYTES as u64,
					"unsupported inline capacity"
				);
				Some(Row::Format(value))
			}
			wire::FormatRow::FsDeleted => None,
		},
		RowKey::Inode(..) => match <InodeKey as FormalKey>::Codec::decode(bytes)? {
			wire::InodeRow::FsInode(value) => Some(Row::Inode(value)),
			wire::InodeRow::FsDeleted => None,
		},
		RowKey::Dentry(..) => match <DentryKey as FormalKey>::Codec::decode(bytes)? {
			wire::DentryRow::FsDentry(value) => Some(Row::Dentry(value)),
			wire::DentryRow::FsDeleted => None,
		},
		RowKey::Link(..) => match <LinkKey as FormalKey>::Codec::decode(bytes)? {
			wire::LinkRow::FsPresent => Some(Row::Unit),
			wire::LinkRow::FsDeleted => None,
		},
		RowKey::Xattr(..) => match <XattrKey as FormalKey>::Codec::decode(bytes)? {
			wire::XattrRow::FsBytes(value) => Some(Row::Bytes(value)),
			wire::XattrRow::FsDeleted => None,
		},
		RowKey::Kv(..) => match <KvKey as FormalKey>::Codec::decode(bytes)? {
			wire::KvRow::FsBytes(value) => Some(Row::Bytes(value)),
			wire::KvRow::FsDeleted => None,
		},
		RowKey::Orphan(..) => match <OrphanKey as FormalKey>::Codec::decode(bytes)? {
			wire::OrphanRow::FsPresent => Some(Row::Unit),
			wire::OrphanRow::FsDeleted => None,
		},
		RowKey::Chunk(..) => match <ChunkKey as FormalKey>::Codec::decode(bytes)? {
			wire::ChunkRow::FsBytes(value) => Some(Row::Bytes(value)),
			wire::ChunkRow::FsDeleted => None,
		},
	})
}

/// Reads the space and row tag of a filesystem row address without decoding the
/// rest. Compaction filters decode every row they visit, so trying each key type
/// and discarding the errors costs a failed parse, and an error, per candidate.
fn row_tag(bytes: &[u8]) -> Option<(usize, Option<usize>)> {
	use foundationdb_tuple::{self as tuple, TupleUnpack};
	use rivet_keys::keys::{
		codec::{BinaryComponent, expect_tag},
		tags::slatedb as tags,
	};
	struct Tag(usize, Option<usize>);
	impl<'a> TupleUnpack<'a> for Tag {
		fn unpack(
			input: &'a [u8],
			depth: tuple::TupleDepth,
		) -> tuple::PackResult<(&'a [u8], Self)> {
			let mut remaining = input;
			expect_tag(&mut remaining, depth, tags::VOLUME)?;
			PartitionKeyHash::unpack_from(&mut remaining, depth)?;
			Option::<Vec<u8>>::unpack_from(&mut remaining, depth)?;
			Uuid::unpack_from(&mut remaining, depth)?;
			let space = u64::unpack_from(&mut remaining, depth)? as usize;
			if space != tags::FS_META {
				return Ok((&[], Tag(space, None)));
			}
			u64::unpack_from(&mut remaining, depth)?;
			let kind = u64::unpack_from(&mut remaining, depth)? as usize;
			Ok((&[], Tag(space, Some(kind))))
		}
	}
	tuple::unpack::<Tag>(bytes).ok().map(|tag| (tag.0, tag.1))
}

pub fn decode_key(bytes: &[u8]) -> anyhow::Result<(Scope, RowKey, u64)> {
	use rivet_keys::keys::tags::slatedb as tags;
	let Some((space, kind)) = row_tag(bytes) else {
		anyhow::bail!("not a versioned filesystem row");
	};
	let decode_chunk = space == tags::FS_DATA;
	let kind = if space == tags::FS_META { kind } else { None };
	if !decode_chunk && kind.is_none() {
		anyhow::bail!("not a versioned filesystem row");
	}
	if kind == Some(tags::FS_FORMAT)
		&& let Ok(key) = FormatKey::unpack(bytes)
	{
		anyhow::ensure!(key.object == 0, "format must use object zero");
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Format,
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_INODE)
		&& let Ok(key) = InodeKey::unpack(bytes)
	{
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Inode(key.ino),
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_DENTRY)
		&& let Ok(key) = DentryKey::unpack(bytes)
	{
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Dentry(key.ino, key.entry.to_vec()),
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_LINK)
		&& let Ok(key) = LinkKey::unpack(bytes)
	{
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Link(key.ino, key.parent, key.entry.to_vec()),
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_XATTR)
		&& let Ok(key) = XattrKey::unpack(bytes)
	{
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Xattr(key.ino, key.attribute.to_vec()),
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_KV)
		&& let Ok(key) = KvKey::unpack(bytes)
	{
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Kv(key.ino, key.key.to_vec()),
			key.seq.0,
		));
	}
	if kind == Some(tags::FS_ORPHAN)
		&& let Ok(key) = OrphanKey::unpack(bytes)
	{
		anyhow::ensure!(key.object == 0, "orphan must use object zero");
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Orphan(key.ino),
			key.seq.0,
		));
	}
	if decode_chunk && let Ok(key) = ChunkKey::unpack(bytes) {
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::Chunk(key.ino, key.chunk),
			key.seq.0,
		));
	}
	anyhow::bail!("not a versioned filesystem row")
}

rivet_keys::slatedb_prefix! {
	pub MetadataSpacePrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}, FS_META];
}
rivet_keys::slatedb_prefix! {
	pub DirectoryRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_DENTRY];
}
rivet_keys::slatedb_prefix! {
	pub XattrRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_XATTR];
}
rivet_keys::slatedb_prefix! {
	pub KvRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, ino: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {ino}, FS_KV];
}
rivet_keys::slatedb_prefix! {
	pub OrphanRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_ORPHAN];
}
rivet_keys::slatedb_prefix! {
	pub CheckpointRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_CHECKPOINT];
}
rivet_keys::slatedb_prefix! {
	pub ForkHoldRowsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64) => [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_FORKHOLD];
}

/// Parses the formal row address and the descending sequence byte-type marker.
/// It accepts that address with any suffix, so extraction is stable for both
/// complete stored keys and prefix seeks. Control records have no such prefix.
pub fn versioned_row_prefix_len(bytes: &[u8]) -> Option<usize> {
	use foundationdb_tuple::{self as tuple, TupleUnpack};
	use rivet_keys::keys::{
		codec::{BinaryComponent, expect_tag},
		tags::slatedb as tags,
	};
	struct Length(usize);
	impl<'a> TupleUnpack<'a> for Length {
		fn unpack(
			input: &'a [u8],
			depth: tuple::TupleDepth,
		) -> tuple::PackResult<(&'a [u8], Self)> {
			let mut remaining = input;
			expect_tag(&mut remaining, depth, tags::VOLUME)?;
			PartitionKeyHash::unpack_from(&mut remaining, depth)?;
			Option::<Vec<u8>>::unpack_from(&mut remaining, depth)?;
			Uuid::unpack_from(&mut remaining, depth)?;
			let space = u64::unpack_from(&mut remaining, depth)?;
			let ino = u64::unpack_from(&mut remaining, depth)?;
			if space == tags::FS_DATA as u64 {
				u64::unpack_from(&mut remaining, depth)?;
			} else if space == tags::FS_META as u64 {
				let kind = u64::unpack_from(&mut remaining, depth)?;
				match kind as usize {
					tags::FS_INODE => {}
					tags::FS_FORMAT if ino == 0 => {}
					tags::FS_DENTRY | tags::FS_XATTR | tags::FS_KV => {
						bytes::Bytes::unpack_from(&mut remaining, depth)?;
					}
					tags::FS_LINK => {
						u64::unpack_from(&mut remaining, depth)?;
						bytes::Bytes::unpack_from(&mut remaining, depth)?;
					}
					tags::FS_ORPHAN if ino == 0 => {
						u64::unpack_from(&mut remaining, depth)?;
					}
					_ => {
						return Err(tuple::PackError::Message(
							"not a versioned row address".into(),
						));
					}
				}
			} else {
				return Err(tuple::PackError::Message("not a filesystem row".into()));
			}
			if remaining.first() != Some(&1) {
				return Err(tuple::PackError::MissingBytes);
			}
			Ok((&[], Length(input.len() - remaining.len() + 1)))
		}
	}
	tuple::unpack::<Length>(bytes).ok().map(|length| length.0)
}

/// Length of the scope-level checkpoint or fork-hold prefix that starts `bytes`:
/// `[VOLUME, hash, name, incarnation, FS_META, object, FS_CHECKPOINT | FS_FORKHOLD]`.
/// It accepts that prefix with any suffix, so a stored row and the scan prefix agree.
/// These rows are not versioned, so [`versioned_row_prefix_len`] rejects them.
pub fn scope_hold_prefix_len(bytes: &[u8]) -> Option<usize> {
	use foundationdb_tuple::{self as tuple, TupleUnpack};
	use rivet_keys::keys::{
		codec::{BinaryComponent, expect_tag},
		tags::slatedb as tags,
	};
	struct Length(usize);
	impl<'a> TupleUnpack<'a> for Length {
		fn unpack(
			input: &'a [u8],
			depth: tuple::TupleDepth,
		) -> tuple::PackResult<(&'a [u8], Self)> {
			let mut remaining = input;
			expect_tag(&mut remaining, depth, tags::VOLUME)?;
			PartitionKeyHash::unpack_from(&mut remaining, depth)?;
			Option::<Vec<u8>>::unpack_from(&mut remaining, depth)?;
			Uuid::unpack_from(&mut remaining, depth)?;
			let space = u64::unpack_from(&mut remaining, depth)?;
			if space != tags::FS_META as u64 {
				return Err(tuple::PackError::Message("not filesystem metadata".into()));
			}
			u64::unpack_from(&mut remaining, depth)?;
			let kind = u64::unpack_from(&mut remaining, depth)?;
			if kind != tags::FS_CHECKPOINT as u64 && kind != tags::FS_FORKHOLD as u64 {
				return Err(tuple::PackError::Message(
					"not a checkpoint or fork hold row".into(),
				));
			}
			Ok((&[], Length(input.len() - remaining.len())))
		}
	}
	tuple::unpack::<Length>(bytes).ok().map(|length| length.0)
}

impl VolumeCheckpointKey {
	pub fn checkpoint_name(&self) -> &[u8] {
		&self.checkpoint
	}
	pub fn incarnation(&self) -> Uuid {
		self.incarnation
	}
	pub fn sequence(&self) -> u64 {
		self.seq
	}
}
impl ForkHoldKey {
	pub fn sequence(&self) -> u64 {
		self.seq
	}
}

rivet_keys::slatedb_prefix! {
	pub VolumePrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}];
}
rivet_keys::slatedb_prefix! {
	pub CheckpointForkHoldsPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid, object: u64, seq: u64, checkpoint: bytes::Bytes)
		=> [VOLUME, {hash}, {name}, {incarnation}, FS_META, {object}, FS_FORKHOLD, {seq}, {checkpoint}];
}

rivet_keys::slatedb_prefix! {
	pub PendingWorkPrefix() => [FS_PENDING];
}
rivet_keys::slatedb_prefix! {
	pub PendingVolumePrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [FS_PENDING, {hash}, {name}];
}
rivet_keys::slatedb_prefix! {
	pub PendingForkChecksPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [FS_PENDING, {hash}, {name}, FS_FORK_CHECK];
}
impl PendingSweepKey {
	pub fn scope(&self) -> Scope {
		Scope {
			hash: self.hash,
			name: self.name.clone(),
			incarnation: self.incarnation,
		}
	}
}

/// Decode only the owned incarnation prefix, independently of the row kind.
/// Listing control rows can then seek past file data without scanning its size.
pub fn decode_scope(bytes: &[u8]) -> Option<Scope> {
	use foundationdb_tuple::{self as tuple, TupleUnpack};
	use rivet_keys::keys::{
		codec::{BinaryComponent, expect_tag},
		tags::slatedb as tags,
	};
	struct Prefix(Scope);
	impl<'a> TupleUnpack<'a> for Prefix {
		fn unpack(
			input: &'a [u8],
			depth: tuple::TupleDepth,
		) -> tuple::PackResult<(&'a [u8], Self)> {
			let mut remaining = input;
			expect_tag(&mut remaining, depth, tags::VOLUME)?;
			let hash = PartitionKeyHash::unpack_from(&mut remaining, depth)?;
			let name = Option::<Vec<u8>>::unpack_from(&mut remaining, depth)?;
			let incarnation = Uuid::unpack_from(&mut remaining, depth)?;
			Ok((
				&[],
				Prefix(Scope {
					hash,
					name,
					incarnation,
				}),
			))
		}
	}
	tuple::unpack::<Prefix>(bytes).ok().map(|v| v.0)
}

// Exact reservation reads protect logical-name admission before background
// recovery has visited this operation in its pending-work scan.
rivet_keys::slatedb_key! {
	pub PendingForkKey(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [FS_PENDING, {hash}, {name}, FS_FORK_PREPARE];
	value: vbare_selected(rivet_data::versioned::DepotFsPendingForkData);
}
rivet_keys::slatedb_key! {
	pub ForkReferenceKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, fork_incarnation: Uuid) => [VOLUME, {hash}, {name}, FS_FORK_REFERENCE, {fork_incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsForkReferenceData);
}
rivet_keys::slatedb_key! {
	pub IncarnationRecordKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}, FS_INCARNATION];
	value: vbare_selected(rivet_data::versioned::DepotFsIncarnationRecordData);
}
rivet_keys::slatedb_key! {
	pub PendingForkReleaseKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [FS_PENDING, {hash}, {name}, FS_FORK_RELEASE, {incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsPendingForkReleaseData);
}

// A tablet's record of its maintainer log, under the log's own volume address, so it lies
// inside the tablet's hash range like the log's replicas.
rivet_keys::slatedb_key! {
	pub MaintainerLogRecordKey(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}, FS_MAINTAINER_LOG];
	value: vbare_selected(rivet_data::versioned::DepotMaintainerLogRecordData);
}

rivet_keys::slatedb_prefix! {
	pub ForkReferencesPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}, FS_FORK_REFERENCE];
}

rivet_keys::slatedb_prefix! {
	pub DataSpacePrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, {incarnation}, FS_DATA];
}

rivet_keys::slatedb_key! {
	pub ForkFailureKey(hash: PartitionKeyHash, name: Option<Vec<u8>>, incarnation: Uuid) => [VOLUME, {hash}, {name}, FS_FORK_FAILURE, {incarnation}];
	value: vbare_selected(rivet_data::versioned::DepotFsForkFailureData);
}
rivet_keys::slatedb_prefix! {
	pub ForkFailuresPrefix(hash: PartitionKeyHash, name: Option<Vec<u8>>) => [VOLUME, {hash}, {name}, FS_FORK_FAILURE];
}

impl PendingForkCheckKey {
	pub fn scope(&self) -> Scope {
		Scope {
			hash: self.hash,
			name: self.name.clone(),
			incarnation: self.incarnation,
		}
	}
}

impl PendingForkReleaseKey {
	pub fn scope(&self) -> Scope {
		Scope {
			hash: self.hash,
			name: self.name.clone(),
			incarnation: self.incarnation,
		}
	}
}

impl PendingMaterializeKey {
	pub fn scope(&self) -> Scope {
		Scope {
			hash: self.hash,
			name: self.name.clone(),
			incarnation: self.incarnation,
		}
	}
}
