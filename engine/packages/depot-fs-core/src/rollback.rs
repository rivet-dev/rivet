//! Rolling a filesystem back to a checkpoint as ordinary writes on the same volume.
//!
//! A plan compares the checkpoint's metadata rows with the head's and lists the ops that turn
//! the head into the checkpoint, ordered so that a transaction, or a series of them, can carry
//! them. Inodes keep their numbers: a restored inode is written back under its own number, never
//! created, so every op is a complete postimage and no entry allocates an inode.
//!
//! - **Removed:** an inode at the head that is not live at the checkpoint, created after it or
//!   left as an orphan, is unlinked from every name and reclaimed.
//! - **Restored:** an inode live at the checkpoint and missing at the head, or an orphan at the
//!   head, is written back whole: its inode, its names, its extended attributes and KV rows,
//!   and every non-zero chunk of the checkpoint's content.
//! - **Rewritten:** an inode live at both whose rows differ, or whose change sequence is above
//!   the checkpoint, gets the checkpoint's inode and rows back. A file's chunks are compared with
//!   the checkpoint's content, and only the chunks that differ are written.
//!
//! Names follow their parents' directory rows. A head name that differs from the checkpoint's is
//! unlinked first, then the checkpoint's name is written with a rename onto itself, which writes
//! the directory entry and, for a non-directory, its link row.
//!
//! File content keeps the head's representation where it can. A rewritten file keeps the head's
//! inherited limit, lowered to the checkpoint's size, so a fork's inherited chunks never come back
//! past a truncation. A restored file, or one that was inline at the head, inherits nothing: all
//! its local chunks are dropped and the checkpoint's content is written in full.

use std::collections::{BTreeMap, BTreeSet};

use crate::store::{Row, RowKey};
use crate::*;

/// Chunks one chunk step reads and writes at most.
pub const CHUNK_STEP: u64 = 128;

/// How a chunk step decides which chunks to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkMode {
	/// Writes every chunk whose checkpoint content is not all zeros. The file's local chunks
	/// were dropped first, so the others read as holes.
	Copy,
	/// Writes each chunk whose checkpoint content differs from the head's, and the chunk at
	/// `force` regardless, which holds bytes past the checkpoint's size.
	Compare { force: Option<u64> },
}

/// Chunks `first` through `first + count - 1` of one file, written under `target`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkStep {
	pub target: FsInodeUpdate,
	pub first: u64,
	pub count: u64,
	pub mode: ChunkMode,
}

impl ChunkStep {
	/// The logical length of chunk `index` under the target's size.
	pub fn chunk_len(&self, index: u64) -> usize {
		let start = index.saturating_mul(CHUNK_BYTES as u64);
		self.target
			.inode
			.size
			.saturating_sub(start)
			.min(CHUNK_BYTES as u64) as usize
	}
}

/// One step of a rollback, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
	Op(FsOp),
	Chunks(ChunkStep),
}

impl Step {
	/// The inodes the step writes, which a transaction carrying it must hold.
	pub fn inodes(&self) -> BTreeSet<u64> {
		match self {
			// No rollback op creates an inode, so the sequence is never used.
			Self::Op(op) => txn::footprint(0, std::slice::from_ref(op)),
			Self::Chunks(step) => BTreeSet::from([step.target.ino]),
		}
	}

	/// An upper bound of the step's encoded bytes.
	pub fn max_bytes(&self) -> usize {
		match self {
			Self::Op(op) => op_bytes(op),
			Self::Chunks(step) => inode_bytes(&step.target.inode)
				.saturating_add((step.count as usize).saturating_mul(CHUNK_BYTES + 32)),
		}
	}
}

/// An upper bound of one op's encoded bytes.
pub fn op_bytes(op: &FsOp) -> usize {
	let ops = match op {
		FsOp::FsSetAttr(v) => {
			inode_bytes(&v.target.inode)
				+ v.chunks
					.iter()
					.map(|chunk| chunk.bytes.len() + 32)
					.sum::<usize>()
		}
		FsOp::FsWrite(v) => {
			inode_bytes(&v.target.inode)
				+ v.chunks
					.iter()
					.map(|chunk| chunk.bytes.len() + 32)
					.sum::<usize>()
		}
		FsOp::FsUnlink(v) => {
			inode_bytes(&v.parent.inode) + inode_bytes(&v.target.inode) + v.name.len()
		}
		FsOp::FsRename(v) => {
			inode_bytes(&v.from_parent.inode)
				+ inode_bytes(&v.to_parent.inode)
				+ inode_bytes(&v.target.inode)
				+ v.from_name.len()
				+ v.to_name.len()
		}
		FsOp::FsSetXattr(v) => inode_bytes(&v.target.inode) + v.name.len() + v.value.len(),
		FsOp::FsRemoveXattr(v) => inode_bytes(&v.target.inode) + v.name.len(),
		FsOp::FsKvPut(v) => v.key.len() + v.value.len(),
		FsOp::FsKvDelete(v) => v.key.len(),
		FsOp::FsReclaim(_) => 0,
		FsOp::FsRoot(_)
		| FsOp::FsCreate(_)
		| FsOp::FsMkdir(_)
		| FsOp::FsSymlink(_)
		| FsOp::FsMknod(_)
		| FsOp::FsLink(_)
		| FsOp::FsKvDeleteRange(_)
		| FsOp::FsCheckpoint(_)
		| FsOp::FsCheckpointRelease(_)
		| FsOp::FsRetentionSet(_)
		| FsOp::FsForkHoldRelease(_)
		| FsOp::FsForkCheckDone(_) => 4096,
	};
	ops.saturating_add(64)
}

fn inode_bytes(inode: &FsInode) -> usize {
	160 + inode.target.len() + inode.inline.as_ref().map_or(0, |inline| inline.len())
}

/// The ops of a rollback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
	pub steps: Vec<Step>,
}

/// The rows of one image that a plan reads, indexed by inode.
#[derive(Default)]
struct Index {
	inodes: BTreeMap<u64, FsInode>,
	orphans: BTreeSet<u64>,
	dentries: BTreeMap<(u64, Vec<u8>), FsDentry>,
	xattrs: BTreeMap<(u64, Vec<u8>), bytes::Bytes>,
	kv: BTreeMap<(u64, Vec<u8>), bytes::Bytes>,
}

impl Index {
	fn new(rows: &BTreeMap<RowKey, Row>) -> Result<Self> {
		let mut index = Self::default();
		for (key, row) in rows {
			match (key, row) {
				(RowKey::Format, Row::Format(_)) => {}
				(RowKey::Inode(ino), Row::Inode(inode)) => {
					index.inodes.insert(*ino, inode.clone());
				}
				(RowKey::Dentry(ino, name), Row::Dentry(dentry)) => {
					index.dentries.insert((*ino, name.clone()), dentry.clone());
				}
				// Link rows mirror the directory entries of non-directories, which the plan
				// restores together with them.
				(RowKey::Link(..), Row::Unit) => {}
				(RowKey::Xattr(ino, name), Row::Bytes(value)) => {
					index.xattrs.insert((*ino, name.clone()), value.clone());
				}
				(RowKey::Kv(ino, key), Row::Bytes(value)) => {
					index.kv.insert((*ino, key.clone()), value.clone());
				}
				(RowKey::Orphan(ino), Row::Unit) => {
					index.orphans.insert(*ino);
				}
				// An image already reads through its whiteouts, so the KV rows the plan compares
				// are the visible ones, and the ordinary KV ops it writes put back what a
				// whiteout hid. The whiteout rows themselves stay as they stand.
				(RowKey::KvRangeWhiteout(..), Row::KvRangeWhiteout(_)) => {}
				(
					RowKey::Format
					| RowKey::Inode(_)
					| RowKey::Dentry(..)
					| RowKey::Link(..)
					| RowKey::Xattr(..)
					| RowKey::Kv(..)
					| RowKey::Orphan(_)
					| RowKey::Chunk(..)
					| RowKey::KvRangeWhiteout(..),
					_,
				) => return Err(Error::Corrupt),
			}
		}
		Ok(index)
	}

	fn rows_of<'a>(
		map: &'a BTreeMap<(u64, Vec<u8>), bytes::Bytes>,
		ino: u64,
	) -> BTreeMap<&'a [u8], &'a bytes::Bytes> {
		map.range((ino, Vec::new())..)
			.take_while(|((owner, _), _)| *owner == ino)
			.map(|((_, name), value)| (name.as_slice(), value))
			.collect()
	}
}

/// Regular files live at both images whose inode rows are equal and hold chunks. Their content
/// can still differ only if a write after the checkpoint left the same postimage, so the caller
/// passes [`plan`] those among them whose change sequence is above the checkpoint.
pub fn content_candidates(
	checkpoint: &BTreeMap<RowKey, Row>,
	head: &BTreeMap<RowKey, Row>,
) -> Result<Vec<u64>> {
	let checkpoint = Index::new(checkpoint)?;
	let head = Index::new(head)?;
	Ok(checkpoint
		.inodes
		.iter()
		.filter(|(ino, inode)| {
			inode.kind == FsKind::File
				&& inode.inline.is_none()
				&& !checkpoint.orphans.contains(ino)
				&& !head.orphans.contains(ino)
				&& head.inodes.get(ino) == Some(inode)
		})
		.map(|(ino, _)| *ino)
		.collect())
}

/// Plans the rollback of `head` to `checkpoint`. Both hold every metadata row of their image
/// and no chunk rows. `changed` names the [`content_candidates`] whose change sequence at the
/// head is above the checkpoint's position.
pub fn plan(
	checkpoint: &BTreeMap<RowKey, Row>,
	head: &BTreeMap<RowKey, Row>,
	changed: &BTreeSet<u64>,
) -> Result<Plan> {
	let c = Index::new(checkpoint)?;
	let h = Index::new(head)?;

	// An orphan at the checkpoint was unlinked and only an open handle kept it, so nothing
	// refers to it once the volume is rolled back.
	let live: BTreeMap<u64, &FsInode> = c
		.inodes
		.iter()
		.filter(|(ino, _)| !c.orphans.contains(ino))
		.map(|(ino, inode)| (*ino, inode))
		.collect();
	if !live.contains_key(&ROOT_INO) || !h.inodes.contains_key(&ROOT_INO) {
		return Err(Error::Corrupt);
	}
	// A head orphan that is live at the checkpoint is reclaimed first and then restored whole.
	let reclaim_first: BTreeSet<u64> = h
		.orphans
		.iter()
		.copied()
		.filter(|ino| live.contains_key(ino))
		.collect();
	let restored: BTreeSet<u64> = live
		.keys()
		.copied()
		.filter(|ino| !h.inodes.contains_key(ino) || reclaim_first.contains(ino))
		.collect();
	let removed: BTreeSet<u64> = h
		.inodes
		.keys()
		.copied()
		.filter(|ino| !live.contains_key(ino))
		.collect();
	// The head as the plan builds on it: restored inodes have no rows there.
	let head_inode = |ino: u64| match restored.contains(&ino) {
		true => None,
		false => h.inodes.get(&ino),
	};

	// Each live inode's postimage, and how its chunks are restored.
	let mut finals: BTreeMap<u64, FsInode> = BTreeMap::new();
	let mut files = Vec::new();
	for (&ino, &checkpoint_inode) in &live {
		let head = head_inode(ino);
		if head.is_some_and(|head| head.kind != checkpoint_inode.kind) {
			return Err(Error::Corrupt);
		}
		let mut target = checkpoint_inode.clone();
		let mut drop_chunks_from = None;
		let mut mode = None;
		if checkpoint_inode.kind == FsKind::File {
			match (&checkpoint_inode.inline, head) {
				(Some(_), None) => target.inherited_limit = None,
				(Some(_), Some(head)) => {
					target.inherited_limit =
						head.inherited_limit.map(|limit| limit.min(target.size));
					if head.inline.is_none() {
						drop_chunks_from = Some(0);
					}
				}
				// A reclaim already dropped the chunks of a head orphan.
				(None, None) => {
					target.inherited_limit = None;
					mode = Some(ChunkMode::Copy);
				}
				(None, Some(head)) if head.inline.is_some() => {
					target.inherited_limit = None;
					drop_chunks_from = Some(0);
					mode = Some(ChunkMode::Copy);
				}
				(None, Some(head)) => {
					target.inherited_limit =
						head.inherited_limit.map(|limit| limit.min(target.size));
					if head != checkpoint_inode || changed.contains(&ino) {
						let mut force = None;
						if head.size > target.size {
							drop_chunks_from = Some(target.size.div_ceil(CHUNK_BYTES as u64));
							if target.size % CHUNK_BYTES as u64 != 0 {
								force = Some(target.size / CHUNK_BYTES as u64);
							}
						}
						mode = Some(ChunkMode::Compare { force });
					}
				}
			}
		}
		if head != Some(&target) || drop_chunks_from.is_some() || mode.is_some() {
			files.push((ino, drop_chunks_from, mode));
		}
		finals.insert(ino, target);
	}
	let post = |ino: u64| -> Result<FsInodeUpdate> {
		let inode = match finals.get(&ino) {
			Some(inode) => inode,
			None => h.inodes.get(&ino).ok_or(Error::Corrupt)?,
		};
		Ok(FsInodeUpdate {
			ino,
			inode: inode.clone(),
		})
	};

	let mut steps = Vec::new();
	for &ino in &reclaim_first {
		steps.push(Step::Op(FsOp::FsReclaim(FsReclaim { ino })));
	}

	// Head names that differ from the checkpoint's. A name in a removed directory that points
	// to a removed inode goes with the reclaims.
	for ((parent, name), dentry) in &h.dentries {
		if restored.contains(parent) {
			continue;
		}
		let kept = match live.contains_key(parent) {
			true => c.dentries.get(&(*parent, name.clone())),
			false => None,
		};
		if kept.is_some_and(|kept| kept == dentry) {
			continue;
		}
		if !live.contains_key(parent) && !live.contains_key(&dentry.child) {
			continue;
		}
		steps.push(Step::Op(FsOp::FsUnlink(FsUnlink {
			parent: post(*parent)?,
			name: name.clone().into(),
			target: post(dentry.child)?,
		})));
	}
	for &ino in &removed {
		steps.push(Step::Op(FsOp::FsReclaim(FsReclaim { ino })));
	}

	// Checkpoint names the head lacks.
	for ((parent, name), dentry) in &c.dentries {
		if !live.contains_key(parent) {
			continue;
		}
		if !live.contains_key(&dentry.child) {
			return Err(Error::Corrupt);
		}
		if !restored.contains(parent)
			&& h.dentries.get(&(*parent, name.clone())) == Some(dentry)
			&& !restored.contains(&dentry.child)
		{
			continue;
		}
		let parent = post(*parent)?;
		steps.push(Step::Op(FsOp::FsRename(FsRename {
			from_parent: parent.clone(),
			from_name: name.clone().into(),
			to_parent: parent,
			to_name: name.clone().into(),
			target: post(dentry.child)?,
			replaced: None,
		})));
	}

	// Inode postimages and file content, shrinking files before growing others so the volume's
	// size never rises above the larger of its current and restored sizes.
	let charge = |inode: Option<&FsInode>| -> i128 {
		inode.map_or(0, |inode| match inode.kind {
			FsKind::File | FsKind::Symlink => inode.size as i128,
			FsKind::Dir
			| FsKind::Fifo
			| FsKind::Socket
			| FsKind::Chardev
			| FsKind::Blockdev
			| FsKind::Kv => 0,
		})
	};
	files.sort_by_key(|(ino, _, _)| (charge(finals.get(ino)) - charge(head_inode(*ino)), *ino));
	for (ino, drop_chunks_from, mode) in files {
		let target = post(ino)?;
		steps.push(Step::Op(FsOp::FsSetAttr(FsSetAttr {
			target: target.clone(),
			chunks: Vec::new(),
			drop_chunks_from,
		})));
		let Some(mode) = mode else {
			continue;
		};
		let chunks = target.inode.size.div_ceil(CHUNK_BYTES as u64);
		let mut first = 0;
		while first < chunks {
			let count = (chunks - first).min(CHUNK_STEP);
			steps.push(Step::Chunks(ChunkStep {
				target: target.clone(),
				first,
				count,
				mode: mode.clone(),
			}));
			first += count;
		}
	}

	// Extended attributes and KV rows.
	for &ino in live.keys() {
		let restored_here = restored.contains(&ino);
		for (rows, xattrs) in [(&c.xattrs, true), (&c.kv, false)] {
			let head_rows = match xattrs {
				true => &h.xattrs,
				false => &h.kv,
			};
			let wanted = Index::rows_of(rows, ino);
			let current = match restored_here {
				true => BTreeMap::new(),
				false => Index::rows_of(head_rows, ino),
			};
			for name in current.keys() {
				if wanted.contains_key(name) {
					continue;
				}
				steps.push(Step::Op(match xattrs {
					true => FsOp::FsRemoveXattr(FsRemoveXattr {
						target: post(ino)?,
						name: name.to_vec().into(),
					}),
					false => FsOp::FsKvDelete(FsKvDelete {
						ino,
						key: name.to_vec().into(),
					}),
				}));
			}
			for (name, value) in wanted {
				if current.get(name) == Some(&value) {
					continue;
				}
				steps.push(Step::Op(match xattrs {
					true => FsOp::FsSetXattr(FsSetXattr {
						target: post(ino)?,
						name: name.to_vec().into(),
						value: value.clone(),
					}),
					false => FsOp::FsKvPut(FsKvPut {
						ino,
						key: name.to_vec().into(),
						value: value.clone(),
					}),
				}));
			}
		}
	}

	Ok(Plan { steps })
}
