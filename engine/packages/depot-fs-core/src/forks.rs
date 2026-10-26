//! Pure fork image rules shared by local reads, remote base reads and materialization.
pub mod cleanup;
pub mod materialize;
use crate::{CHUNK_BYTES, Error, FsInode, FsKind, Result, store::Row};
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;

/// Copy metadata at sequence zero, but reserve every inherited inode ordinal.
/// The fresh log starts above this floor with its own client epoch.
pub fn initial_position(source: &VolumeLogPosition) -> Result<VolumeLogPosition> {
	if source.volume_seq == u64::MAX {
		return Err(Error::TooLarge);
	}
	Ok(VolumeLogPosition {
		volume_seq: source.volume_seq,
		client_epoch: 0,
	})
}

/// A child inherits its parent's complete logical file image, including positive
/// writes beyond an earlier ancestor's truncation limit. Each link enforces its
/// own limit; copying the parent's limit here would hide those valid writes.
pub fn copy_inode(mut inode: FsInode) -> FsInode {
	inode.inherited_limit =
		(inode.kind == FsKind::File && inode.inline.is_none()).then_some(inode.size);
	inode
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkLookup {
	/// A positive write, explicit tombstone or inherited-byte mask ends the walk.
	Resolved(Option<Vec<u8>>),
	/// Ask the next layer, retaining only this many bytes from its response.
	Base { max_bytes: usize },
}

/// Missing, deleted and materialized-at-zero chunks have distinct semantics.
/// A late seq-0 copy must never undo a truncation that happened before the copy.
pub fn chunk_lookup(
	inode: &FsInode,
	index: u64,
	local: Option<(u64, Option<Row>)>,
) -> Result<ChunkLookup> {
	if inode.kind != FsKind::File {
		return Err(Error::Invalid);
	}
	let start = index
		.checked_mul(CHUNK_BYTES as u64)
		.ok_or(Error::TooLarge)?;
	let bytes = inode.size.saturating_sub(start).min(CHUNK_BYTES as u64) as usize;
	if bytes == 0 || inode.inline.is_some() {
		return Ok(ChunkLookup::Resolved(None));
	}
	match local {
		Some((_, None)) => Ok(ChunkLookup::Resolved(None)),
		Some((seq, Some(Row::Bytes(mut value)))) if value.len() <= CHUNK_BYTES => {
			let count = if seq == 0 {
				inode.inherited_limit.map_or(bytes, |limit| {
					limit.saturating_sub(start).min(bytes as u64) as usize
				})
			} else {
				bytes
			};
			value.truncate(count);
			// Converting takes the buffer without a copy when this row holds the only reference.
			Ok(ChunkLookup::Resolved(
				(!value.is_empty()).then(|| Vec::from(value)),
			))
		}
		Some(_) => Err(Error::Corrupt),
		None => {
			let max_bytes = inode.inherited_limit.map_or(0, |limit| {
				limit.saturating_sub(start).min(bytes as u64) as usize
			});
			Ok(if max_bytes == 0 {
				ChunkLookup::Resolved(None)
			} else {
				ChunkLookup::Base { max_bytes }
			})
		}
	}
}

/// Administrative commands return durable metadata to their owning core only
/// after the store's flush and grant check have completed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Preparation {
	Pending(Box<crate::FsPendingFork>),
	Complete(crate::FsVolumeRecord),
}

pub fn valid_spec(spec: &crate::FsForkSpec, region: u16) -> bool {
	spec.destination_region == region
		&& match &spec.selector {
			crate::FsForkSelector::FsForkNamed(point) => {
				crate::checkpoints::valid_name(&point.name)
			}
			_ => true,
		} && spec.operation_id != [0; 16]
		&& spec
			.restore
			.is_none_or(|old| old != spec.operation_id && spec.parent_region == region)
		&& (spec.restore.is_some()
			|| spec.parent_region != region
			|| spec.parent_volume_key != spec.destination_volume_key)
		&& (spec.minimum.is_none() || matches!(spec.selector, crate::FsForkSelector::FsForkLatest))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Materialization {
	pub record: crate::FsVolumeRecord,
	pub cursor: crate::FsMaterializeCursor,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MaterializeProgress {
	More(crate::FsMaterializeCursor),
	Complete(crate::FsVolumeRecord),
}

/// Source acquisition may flush applied data with its hold. The core consumes
/// this durable proof even if the initiating caller has already expired.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReferenceResult {
	/// No source receipt exists; the core must establish fresh applied progress.
	SourceProgressRequired,
	Complete {
		response: Box<rivet_data::generated::depot_client_protocol_v1::FsForkReferenceResponse>,
		record: Option<crate::FsVolumeRecord>,
	},
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReferenceBarrier {
	Lookup,
	Passed,
}

/// One ordered point lookup supplies both the current row and a durable name
/// reservation. Replica observations alone cannot reveal a pending restore.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LifecycleLookup {
	/// The name's pointer, or for a lookup of one incarnation, that incarnation's own row.
	pub row: Option<crate::FsVolumeRecord>,
	pub pending: Option<Box<crate::FsPendingFork>>,
	/// For a lookup of one incarnation: the name's pointer names a newer incarnation, so this
	/// one no longer holds the name.
	pub superseded: bool,
}

impl From<Option<crate::FsVolumeRecord>> for LifecycleLookup {
	fn from(row: Option<crate::FsVolumeRecord>) -> Self {
		Self {
			row,
			pending: None,
			superseded: false,
		}
	}
}

/// One bounded local step of the destination workflow. The maintainer orders
/// these steps; remote requests never occupy its storage FIFO.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StoreAction {
	Inspect,
	Prepare,
	Remember(crate::FsForkImage),
	Copy {
		after: Option<Vec<u8>>,
		page: rivet_data::generated::shared_depot_fs_read_v1::FsForkMetadataPage,
	},
	/// Revalidate the old incarnation after all previously admitted store work.
	RestoreTarget,
	Publish {
		claimed: Option<[u8; 16]>,
		/// The publication's durable record is a maintainer log op, so it is not flushed.
		logged: bool,
		/// The published incarnation's `incarnation_epoch`, from its maintainer log op.
		incarnation_epoch: u64,
	},
	Fail(crate::FsForkError),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StoreResult {
	/// No durable receipt or preparation exists; the core must validate RAM-only
	/// creations and competing lifecycle work before reserving the name on disk.
	Unprepared,
	/// A Ready restore still owns this live incarnation and may retire its log.
	RestoreTarget(crate::FsVolumeRecord),
	Progress(Preparation),
	Rejected(crate::FsForkError),
}
