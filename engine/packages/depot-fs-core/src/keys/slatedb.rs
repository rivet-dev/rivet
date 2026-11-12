//! Formal filesystem addresses. Sequence order is descending only for versioned rows.
use depot_types::PartitionKeyHash;
use rivet_keys::keys::codec::DescendingU64;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORMAT, seq], value = vbare_selected(rivet_data::versioned::DepotFsFormatRowData))]
pub struct FormatKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORMAT], next = bytes::Bytes)]
pub struct FormatPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, ino, FS_INODE, seq], value = vbare_selected(rivet_data::versioned::DepotFsInodeRowData))]
pub struct InodeKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_INODE], next = bytes::Bytes)]
pub struct InodePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, ino, FS_DENTRY, entry, seq], value = vbare_selected(rivet_data::versioned::DepotFsDentryRowData))]
pub struct DentryKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub entry: bytes::Bytes,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_DENTRY, entry], next = bytes::Bytes)]
pub struct DentryPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub entry: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, ino, FS_LINK, parent, entry, seq], value = vbare_selected(rivet_data::versioned::DepotFsLinkRowData))]
pub struct LinkKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub parent: u64,
	pub entry: bytes::Bytes,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_LINK, parent, entry], next = bytes::Bytes)]
pub struct LinkPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub parent: u64,
	pub entry: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, ino, FS_XATTR, attribute, seq], value = vbare_selected(rivet_data::versioned::DepotFsXattrRowData))]
pub struct XattrKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub attribute: bytes::Bytes,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_XATTR, attribute], next = bytes::Bytes)]
pub struct XattrPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub attribute: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, ino, FS_KV, key, seq], value = vbare_selected(rivet_data::versioned::DepotFsKvRowData))]
pub struct KvKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub key: bytes::Bytes,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_KV, key], next = bytes::Bytes)]
pub struct KvPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub key: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_ORPHAN, ino, seq], value = vbare_selected(rivet_data::versioned::DepotFsOrphanRowData))]
pub struct OrphanKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub ino: u64,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_ORPHAN, ino], next = bytes::Bytes)]
pub struct OrphanPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub ino: u64,
}
// A range whiteout row sits under object zero like the orphan rows, keyed by the KV file's inode
// and the range's first key, so every whiteout of a volume sorts before its inodes' rows and a
// scan of a volume's metadata meets them first.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_KV_RANGE_WHITEOUT, ino, start, seq], value = vbare_selected(rivet_data::versioned::DepotFsKvRangeWhiteoutRowData))]
pub struct KvRangeWhiteoutKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub ino: u64,
	pub start: bytes::Bytes,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_KV_RANGE_WHITEOUT, ino, start], next = bytes::Bytes)]
pub struct KvRangeWhiteoutPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub ino: u64,
	pub start: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_KV_RANGE_WHITEOUT], next = bytes::Bytes)]
pub struct KvRangeWhiteoutSpacePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_KV_RANGE_WHITEOUT, ino], next = bytes::Bytes)]
pub struct KvRangeWhiteoutRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_DATA, ino, chunk, seq], value = vbare_selected(rivet_data::versioned::DepotFsChunkRowData))]
pub struct ChunkKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub chunk: u64,
	pub seq: DescendingU64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_DATA, ino, chunk], next = bytes::Bytes)]
pub struct ChunkPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
	pub chunk: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_CHECKPOINT, seq, checkpoint], value = vbare_selected(rivet_data::versioned::DepotFsVolumeCheckpointData))]
pub struct VolumeCheckpointKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub seq: u64,
	pub checkpoint: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORKHOLD, seq, checkpoint, fork_incarnation], value = vbare_selected(rivet_data::versioned::DepotFsForkHoldData))]
pub struct ForkHoldKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub seq: u64,
	pub checkpoint: bytes::Bytes,
	pub fork_incarnation: Uuid,
}
// A fork pin: the parent volume's state at the forks `first_fork..=last_fork` of its group,
// taken before the volume's first write after those forks. It sits beside the object's
// checkpoint rows and pins its cut the same way; its value lists the child groups still waiting
// on it, and the pin path of the group write deletes it once that list is empty.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORKPIN, seq, first_fork], value = vbare_selected(rivet_data::versioned::DepotFsForkPinData))]
pub struct ForkPinKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub seq: u64,
	pub first_fork: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_FORK_CHECK, incarnation], value = vbare_selected(rivet_data::versioned::DepotFsPendingForkCheckData))]
pub struct PendingForkCheckKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_MATERIALIZE, incarnation], value = vbare_selected(rivet_data::versioned::DepotFsPendingMaterializeData))]
pub struct PendingMaterializeKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
// `PendingDeleteKey` marks a `Deleted` volume record, so the start scan finds every
// deletion to drive without visiting volume data. It is written and removed in the
// batches that write and remove the record.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_DELETE, incarnation], value = unit)]
pub struct PendingDeleteKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_SWEEP, incarnation], value = vbare_selected(rivet_data::versioned::DepotFsPendingSweepData))]
pub struct PendingSweepKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
// `VolumeRecordKey` is the name's pointer: a copy of the stored record with the highest
// `incarnation_epoch`, so an open of an unloaded name reads one row. Every incarnation also
// has its own `VolumeIncarnationRecordKey`, written in the same batch as the pointer, which a
// lookup of a known incarnation reads. The incarnation row survives the record's removal from
// the pointer and goes with the incarnation's data sweep.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, RECORD], value = vbare_selected(rivet_data::versioned::DepotFsVolumeRecordData))]
pub struct VolumeRecordKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_RECORD], value = vbare_selected(rivet_data::versioned::DepotFsVolumeRecordData))]
pub struct VolumeIncarnationRecordKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
// `RepairAuditKey` holds the last logged state of one operator repair call of the name, beside
// the name's own rows so it moves with a split. It is written only by the call's audit op, at
// the lowest `depot.maintainer_log` version that carries it, so a worker of an older release
// granted the tablet later still reads it.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, FS_REPAIR_AUDIT, operation], value = vbare_selected(rivet_data::versioned::DepotRepairAuditData))]
pub struct RepairAuditKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub operation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, FS_REPAIR_AUDIT])]
pub struct RepairAuditPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}
// `DeletedIncarnationKey` indexes a grouped incarnation deleted after a fork of its group by the
// fork it was deleted at, beside its name's rows so it moves with a split. A child of an earlier
// fork whose acquire finds the name's pointer held by a newer incarnation, or by none, finds the
// incarnation that existed at its fork here. It is written in the batch that stores the
// incarnation's Deleted record and removed in the batch of the incarnation's last data sweep,
// which waits for every child its fork pins wait on.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, FS_DELETED_INCARNATION, partition_key, deleted_at_fork, incarnation], value = unit)]
pub struct DeletedIncarnationKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub partition_key: bytes::Bytes,
	pub deleted_at_fork: u64,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, FS_DELETED_INCARNATION, partition_key])]
pub struct DeletedIncarnationPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub partition_key: bytes::Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation])]
pub struct IncarnationPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino])]
pub struct MetadataPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_DATA, ino])]
pub struct DataPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
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

	/// The row space the log of `key` writes, with the claim log it stamps its rows with. A
	/// claim log writes its base volume's rows, so every claim of a volume shares one row
	/// space; any other key writes its own rows as claim log 0.
	pub fn of_log(key: &depot_types::VolumeKey, incarnation: Uuid) -> (Self, u16) {
		match depot_types::is_claim_log(key) {
			Some((base, claim_log)) => (Self::new(&base, incarnation), claim_log),
			None => (Self::new(key, incarnation), rivet_data::ROOT_CLAIM_LOG),
		}
	}
}

use crate::store::{Row, RowKey};
use rivet_data::generated::depot_fs_rows_v2 as wire;
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
		RowKey::KvRangeWhiteout(ino, start) => KvRangeWhiteoutPrefix::new(
			*hash,
			name.clone(),
			*incarnation,
			0,
			*ino,
			start.clone().into(),
		)
		.pack(),
	}
}

/// Encodes one filesystem row at `version`, the `depot.fs_rows` version its writer selected.
/// `claim_log` is the claim log whose entry writes the version; version 1 carries claim log 0
/// alone and refuses any other.
pub fn encode_row(
	scope: &Scope,
	row: &RowKey,
	claim_log: u16,
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
					wire::FormatRowValue::FsFormat(value)
				}
				None => wire::FormatRowValue::FsDeleted,
				_ => anyhow::bail!("invalid filesystem format"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::FormatRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
		}
		RowKey::Inode(ino) => {
			let key = InodeKey::new(*hash, name.clone(), *incarnation, *ino, seq);
			let value = match value {
				Some(Row::Inode(value)) => wire::InodeRowValue::FsInode(value),
				None => wire::InodeRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::InodeRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
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
				Some(Row::Dentry(value)) => wire::DentryRowValue::FsDentry(value),
				None => wire::DentryRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::DentryRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
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
				Some(Row::Unit) => wire::LinkRowValue::FsPresent,
				None => wire::LinkRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::LinkRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
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
				Some(Row::Bytes(value)) => wire::XattrRowValue::FsBytes(value.clone()),
				None => wire::XattrRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::XattrRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
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
				Some(Row::Bytes(value)) => wire::KvRowValue::FsBytes(value.clone()),
				None => wire::KvRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::KvRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
		}
		RowKey::Orphan(ino) => {
			let key = OrphanKey::new(*hash, name.clone(), *incarnation, 0, *ino, seq);
			let value = match value {
				Some(Row::Unit) => wire::OrphanRowValue::FsPresent,
				None => wire::OrphanRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::OrphanRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
		}
		RowKey::KvRangeWhiteout(ino, start) => {
			let key = KvRangeWhiteoutKey::new(
				*hash,
				name.clone(),
				*incarnation,
				0,
				*ino,
				start.clone().into(),
				seq,
			);
			let value = match value {
				Some(Row::KvRangeWhiteout(end)) => {
					wire::KvRangeWhiteoutRowValue::FsKvRangeWhiteout(crate::FsKvRangeWhiteout {
						end: end.map(Into::into),
					})
				}
				None => wire::KvRangeWhiteoutRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			anyhow::ensure!(
				version >= 2,
				"a range whiteout row has no encoding below version 2"
			);
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::KvRangeWhiteoutRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
		}
		RowKey::Chunk(ino, chunk) => {
			let key = ChunkKey::new(*hash, name.clone(), *incarnation, *ino, *chunk, seq);
			let value = match value {
				Some(Row::Bytes(value)) => wire::ChunkRowValue::FsBytes(value.clone()),
				None => wire::ChunkRowValue::FsDeleted,
				_ => anyhow::bail!("row value does not match its formal key"),
			};
			Ok((
				key.pack(),
				key.serialize(AtVersion::new(
					wire::ChunkRow {
						claim_log,
						row: value,
					},
					version,
				))?,
			))
		}
	}
}

/// Decodes a stored volume record into its own buffer. Maintainer state keeps records past
/// the read, and a record sharing SlateDB's buffer would keep the whole cached block alive, so
/// the value is copied once and every field shares that copy.
pub fn decode_volume_record(stored: &[u8]) -> anyhow::Result<crate::FsVolumeRecord> {
	<VolumeRecordKey as FormalKey>::Codec::decode(bytes::Bytes::copy_from_slice(stored))
}

/// Reads the claim log and tombstone tag of a stored row value in place. The version header
/// picks the wire type and the value is verified, not decoded or converted, so nothing is
/// allocated; a version 1 value is claim log 0. Compaction filters ask this of every row they
/// visit.
pub fn row_stamp(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<(u16, bool)> {
	use rivet_data::generated::depot_fs_rows_v1 as wire1;
	use vbare::View;
	anyhow::ensure!(bytes.len() >= 2, "a stored row value has a version header");
	let version = u16::from_le_bytes([bytes[0], bytes[1]]);
	let body = bytes.slice(2..);
	macro_rules! stamp {
		($wire1:ty, $deleted1:pat, $wire2:ty, $deleted2:pat) => {
			match version {
				1 => {
					let view = View::<$wire1>::new(body)?;
					let deleted = matches!(view.get(), $deleted1);
					(rivet_data::ROOT_CLAIM_LOG, deleted)
				}
				2 => {
					let view = View::<$wire2>::new(body)?;
					let value = view.get();
					(value.claim_log(), matches!(value.row(), $deleted2))
				}
				other => anyhow::bail!("unsupported depot.fs_rows version {other}"),
			}
		};
	}
	Ok(match row {
		RowKey::Format => stamp!(
			wire1::FormatRowWire,
			wire1::FormatRowRef::FsDeleted,
			wire::FormatRowWire,
			wire::FormatRowValueRef::FsDeleted
		),
		RowKey::Inode(..) => stamp!(
			wire1::InodeRowWire,
			wire1::InodeRowRef::FsDeleted,
			wire::InodeRowWire,
			wire::InodeRowValueRef::FsDeleted
		),
		RowKey::Dentry(..) => stamp!(
			wire1::DentryRowWire,
			wire1::DentryRowRef::FsDeleted,
			wire::DentryRowWire,
			wire::DentryRowValueRef::FsDeleted
		),
		RowKey::Link(..) => stamp!(
			wire1::LinkRowWire,
			wire1::LinkRow::FsDeleted,
			wire::LinkRowWire,
			wire::LinkRowValue::FsDeleted
		),
		RowKey::Xattr(..) => stamp!(
			wire1::XattrRowWire,
			wire1::XattrRowRef::FsDeleted,
			wire::XattrRowWire,
			wire::XattrRowValueRef::FsDeleted
		),
		RowKey::Kv(..) => stamp!(
			wire1::KvRowWire,
			wire1::KvRowRef::FsDeleted,
			wire::KvRowWire,
			wire::KvRowValueRef::FsDeleted
		),
		RowKey::Orphan(..) => stamp!(
			wire1::OrphanRowWire,
			wire1::OrphanRow::FsDeleted,
			wire::OrphanRowWire,
			wire::OrphanRowValue::FsDeleted
		),
		RowKey::Chunk(..) => stamp!(
			wire1::ChunkRowWire,
			wire1::ChunkRowRef::FsDeleted,
			wire::ChunkRowWire,
			wire::ChunkRowValueRef::FsDeleted
		),
		RowKey::KvRangeWhiteout(..) => {
			anyhow::ensure!(
				version == 2,
				"unsupported range whiteout row version {version}"
			);
			let view = View::<wire::KvRangeWhiteoutRowWire>::new(body)?;
			let value = view.get();
			(
				value.claim_log(),
				matches!(value.row(), wire::KvRangeWhiteoutRowValueRef::FsDeleted),
			)
		}
	})
}

/// Whether a stored row value is a tombstone, read in place like `row_stamp`.
pub fn row_is_deleted(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<bool> {
	Ok(row_stamp(row, bytes)?.1)
}

/// Decodes a stored row value with the claim log that wrote it. `data` rows are slices of
/// `bytes`, so a row kept past the read keeps the source buffer alive.
pub fn decode_stamped_row(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<(u16, Option<Row>)> {
	Ok(match row {
		RowKey::Format => {
			let stored = <FormatKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::FormatRowValue::FsFormat(value) => {
						anyhow::ensure!(
							value.inline_max_bytes <= crate::CHUNK_BYTES as u64,
							"unsupported inline capacity"
						);
						Some(Row::Format(value))
					}
					wire::FormatRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Inode(..) => {
			let stored = <InodeKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::InodeRowValue::FsInode(value) => Some(Row::Inode(value)),
					wire::InodeRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Dentry(..) => {
			let stored = <DentryKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::DentryRowValue::FsDentry(value) => Some(Row::Dentry(value)),
					wire::DentryRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Link(..) => {
			let stored = <LinkKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::LinkRowValue::FsPresent => Some(Row::Unit),
					wire::LinkRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Xattr(..) => {
			let stored = <XattrKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::XattrRowValue::FsBytes(value) => Some(Row::Bytes(value)),
					wire::XattrRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Kv(..) => {
			let stored = <KvKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::KvRowValue::FsBytes(value) => Some(Row::Bytes(value)),
					wire::KvRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Orphan(..) => {
			let stored = <OrphanKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::OrphanRowValue::FsPresent => Some(Row::Unit),
					wire::OrphanRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::Chunk(..) => {
			let stored = <ChunkKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::ChunkRowValue::FsBytes(value) => Some(Row::Bytes(value)),
					wire::ChunkRowValue::FsDeleted => None,
				},
			)
		}
		RowKey::KvRangeWhiteout(..) => {
			let stored = <KvRangeWhiteoutKey as FormalKey>::Codec::decode(bytes)?;
			(
				stored.claim_log,
				match stored.row {
					wire::KvRangeWhiteoutRowValue::FsKvRangeWhiteout(value) => {
						Some(Row::KvRangeWhiteout(value.end.map(|end| end.to_vec())))
					}
					wire::KvRangeWhiteoutRowValue::FsDeleted => None,
				},
			)
		}
	})
}

/// Decodes a stored row value, dropping the claim log that wrote it.
pub fn decode_row(row: &RowKey, bytes: bytes::Bytes) -> anyhow::Result<Option<Row>> {
	Ok(decode_stamped_row(row, bytes)?.1)
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
	if kind == Some(tags::FS_KV_RANGE_WHITEOUT)
		&& let Ok(key) = KvRangeWhiteoutKey::unpack(bytes)
	{
		anyhow::ensure!(key.object == 0, "a range whiteout must use object zero");
		return Ok((
			Scope {
				hash: key.hash,
				name: key.name,
				incarnation: key.incarnation,
			},
			RowKey::KvRangeWhiteout(key.ino, key.start.to_vec()),
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

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META])]
pub struct MetadataSpacePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_DENTRY])]
pub struct DirectoryRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_XATTR])]
pub struct XattrRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, ino, FS_KV])]
pub struct KvRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_ORPHAN])]
pub struct OrphanRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_CHECKPOINT])]
pub struct CheckpointRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORKHOLD])]
pub struct ForkHoldRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORKPIN])]
pub struct ForkPinRowsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
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
/// These rows are not versioned, so [`versioned_row_prefix_len`] rejects them. The released
/// SST bloom filter rule hashes exactly these, so the kinds it accepts never change.
pub fn scope_hold_prefix_len(bytes: &[u8]) -> Option<usize> {
	use rivet_keys::keys::tags::slatedb as tags;
	let (len, kind) = scope_rows_prefix(bytes)?;
	if kind == tags::FS_CHECKPOINT as u64 || kind == tags::FS_FORKHOLD as u64 {
		Some(len)
	} else {
		None
	}
}

/// Length of the scope-level fork pin prefix that starts `bytes`:
/// `[VOLUME, hash, name, incarnation, FS_META, object, FS_FORKPIN]`, with any suffix, as
/// [`scope_hold_prefix_len`] reads the checkpoint and fork-hold prefixes.
pub fn fork_pin_prefix_len(bytes: &[u8]) -> Option<usize> {
	use rivet_keys::keys::tags::slatedb as tags;
	let (len, kind) = scope_rows_prefix(bytes)?;
	if kind == tags::FS_FORKPIN as u64 {
		Some(len)
	} else {
		None
	}
}

/// The length and row kind of the `[VOLUME, hash, name, incarnation, FS_META, object, kind]`
/// prefix that starts `bytes`, for any kind.
fn scope_rows_prefix(bytes: &[u8]) -> Option<(usize, u64)> {
	use foundationdb_tuple::{self as tuple, TupleUnpack};
	use rivet_keys::keys::{
		codec::{BinaryComponent, expect_tag},
		tags::slatedb as tags,
	};
	struct Prefix {
		len: usize,
		kind: u64,
	}
	impl<'a> TupleUnpack<'a> for Prefix {
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
			let len = input.len() - remaining.len();
			Ok((&[], Prefix { len, kind }))
		}
	}
	tuple::unpack::<Prefix>(bytes)
		.ok()
		.map(|prefix| (prefix.len, prefix.kind))
}

/// The length of the `DeletedIncarnationPrefix` that starts `bytes`: one logical volume key's
/// fork index rows, which a lazy fork's miss scans. Hashed into the SST bloom filters so that
/// scan skips every SST holding none of the key's rows, as a point read of an absent key does.
pub fn deleted_incarnation_prefix_len(bytes: &[u8]) -> Option<usize> {
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
			expect_tag(&mut remaining, depth, tags::FS_DELETED_INCARNATION)?;
			bytes::Bytes::unpack_from(&mut remaining, depth)?;
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
impl ForkPinKey {
	pub fn sequence(&self) -> u64 {
		self.seq
	}
	pub fn first_fork(&self) -> u64 {
		self.first_fork
	}
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name])]
pub struct VolumePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_META, object, FS_FORKHOLD, seq, checkpoint])]
pub struct CheckpointForkHoldsPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
	pub object: u64,
	pub seq: u64,
	pub checkpoint: bytes::Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [FS_PENDING])]
pub struct PendingWorkPrefix {}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [FS_PENDING, hash, name])]
pub struct PendingVolumePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [FS_PENDING, hash, name, FS_FORK_CHECK])]
pub struct PendingForkChecksPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
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
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_FORK_PREPARE], value = vbare_selected(rivet_data::versioned::DepotFsPendingForkData))]
pub struct PendingForkKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, FS_FORK_REFERENCE, fork_incarnation], value = vbare_selected(rivet_data::versioned::DepotFsForkReferenceData))]
pub struct ForkReferenceKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub fork_incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, incarnation, FS_INCARNATION], value = vbare_selected(rivet_data::versioned::DepotFsIncarnationRecordData))]
pub struct IncarnationRecordKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [FS_PENDING, hash, name, FS_FORK_RELEASE, incarnation], value = vbare_selected(rivet_data::versioned::DepotFsPendingForkReleaseData))]
pub struct PendingForkReleaseKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}

// A tablet's record of its maintainer log, under the log's own volume address, so it lies
// inside the tablet's hash range like the log's replicas.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, FS_MAINTAINER_LOG], value = vbare_selected(rivet_data::versioned::DepotMaintainerLogRecordData))]
pub struct MaintainerLogRecordKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, FS_FORK_REFERENCE])]
pub struct ForkReferencesPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, incarnation, FS_DATA])]
pub struct DataSpacePrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(key = [VOLUME, hash, name, FS_FORK_FAILURE, incarnation], value = vbare_selected(rivet_data::versioned::DepotFsForkFailureData))]
pub struct ForkFailureKey {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
	pub incarnation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::SlatedbKey)]
#[slatedb(prefix = [VOLUME, hash, name, FS_FORK_FAILURE])]
pub struct ForkFailuresPrefix {
	pub hash: PartitionKeyHash,
	pub name: Option<Vec<u8>>,
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
