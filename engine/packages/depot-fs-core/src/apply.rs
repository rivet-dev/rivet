//! Deterministic postimage application. Only range operations read earlier rows.

use crate::store::{Batch, Row, RowKey, RowRange, RowStore};
use crate::*;

fn inode(batch: &mut Batch, update: &FsInodeUpdate) {
	batch.rows.insert(
		RowKey::Inode(update.ino),
		Some(Row::Inode(update.inode.clone())),
	);
}

fn create(batch: &mut Batch, seq: u64, parent: &FsInodeUpdate, name: &[u8], child: &FsInode) {
	inode(batch, parent);
	batch
		.rows
		.insert(RowKey::Inode(seq), Some(Row::Inode(child.clone())));
	batch.rows.insert(
		RowKey::Dentry(parent.ino, name.to_vec()),
		Some(Row::Dentry(FsDentry {
			child: seq,
			kind: child.kind.clone(),
		})),
	);
	if child.kind != FsKind::Dir {
		batch.rows.insert(
			RowKey::Link(seq, parent.ino, name.to_vec()),
			Some(Row::Unit),
		);
	}
}

fn orphan(batch: &mut Batch, target: &FsInodeUpdate) {
	inode(batch, target);
	if target.inode.nlink == 0 {
		batch
			.rows
			.insert(RowKey::Orphan(target.ino), Some(Row::Unit));
	}
}

fn chunks(batch: &mut Batch, target: &FsInodeUpdate, chunks: &[FsChunkUpdate]) -> Result<()> {
	inode(batch, target);
	for chunk in chunks {
		if chunk.bytes.len() > CHUNK_BYTES || target.inode.kind != FsKind::File {
			return Err(Error::Corrupt);
		}
		batch.rows.insert(
			RowKey::Chunk(target.ino, chunk.index),
			Some(Row::Bytes(chunk.bytes.clone())),
		);
	}
	Ok(())
}

fn remove_range(batch: &mut Batch, store: &impl RowStore, range: RowRange, seq: u64) -> Result<()> {
	let mut cursor = None;
	loop {
		let rows = store.scan(&range, cursor.as_ref(), 256, Some(seq - 1))?;
		if rows.is_empty() {
			break;
		}
		cursor = rows.last().map(|(key, _)| key.clone());
		for (key, _) in rows {
			batch.rows.insert(key, None);
		}
	}
	// Include preceding puts in this entry; a later put still wins in operation order.
	for (key, value) in &mut batch.rows {
		if range.contains(key) {
			*value = None;
		}
	}
	Ok(())
}

/// Produces one atomic batch. Reapplying a sequence, including after partial storage
/// work, reads the same previous-sequence snapshot and produces identical versions.
pub fn apply(seq: u64, entry: &FsEntry, store: &impl RowStore) -> Result<Batch> {
	if seq == 0 {
		return Err(Error::Invalid);
	}
	if crate::txn::too_many_creates(&entry.ops) {
		return Err(Error::Invalid);
	}
	let mut batch = Batch::default();
	for op in &entry.ops {
		match op {
			FsOp::FsRoot(v) => {
				if v.format.inline_max_bytes > CHUNK_BYTES as u64 {
					return Err(Error::Corrupt);
				}
				batch
					.rows
					.insert(RowKey::Format, Some(Row::Format(v.format.clone())));
				batch
					.rows
					.insert(RowKey::Inode(ROOT_INO), Some(Row::Inode(v.inode.clone())));
			}
			FsOp::FsCreate(v) => create(&mut batch, seq, &v.parent, &v.name, &v.inode),
			FsOp::FsMkdir(v) => create(&mut batch, seq, &v.parent, &v.name, &v.inode),
			FsOp::FsSymlink(v) => create(&mut batch, seq, &v.parent, &v.name, &v.inode),
			FsOp::FsMknod(v) => create(&mut batch, seq, &v.parent, &v.name, &v.inode),
			FsOp::FsLink(v) => {
				inode(&mut batch, &v.parent);
				inode(&mut batch, &v.target);
				batch.rows.insert(
					RowKey::Dentry(v.parent.ino, v.name.to_vec()),
					Some(Row::Dentry(FsDentry {
						child: v.target.ino,
						kind: v.target.inode.kind.clone(),
					})),
				);
				batch.rows.insert(
					RowKey::Link(v.target.ino, v.parent.ino, v.name.to_vec()),
					Some(Row::Unit),
				);
			}
			FsOp::FsUnlink(v) => {
				inode(&mut batch, &v.parent);
				orphan(&mut batch, &v.target);
				batch
					.rows
					.insert(RowKey::Dentry(v.parent.ino, v.name.to_vec()), None);
				if v.target.inode.kind != FsKind::Dir {
					batch.rows.insert(
						RowKey::Link(v.target.ino, v.parent.ino, v.name.to_vec()),
						None,
					);
				}
			}
			FsOp::FsRename(v) => {
				inode(&mut batch, &v.from_parent);
				inode(&mut batch, &v.to_parent);
				inode(&mut batch, &v.target);
				batch.rows.insert(
					RowKey::Dentry(v.from_parent.ino, v.from_name.to_vec()),
					None,
				);
				batch.rows.insert(
					RowKey::Dentry(v.to_parent.ino, v.to_name.to_vec()),
					Some(Row::Dentry(FsDentry {
						child: v.target.ino,
						kind: v.target.inode.kind.clone(),
					})),
				);
				if v.target.inode.kind != FsKind::Dir {
					batch.rows.insert(
						RowKey::Link(v.target.ino, v.from_parent.ino, v.from_name.to_vec()),
						None,
					);
					batch.rows.insert(
						RowKey::Link(v.target.ino, v.to_parent.ino, v.to_name.to_vec()),
						Some(Row::Unit),
					);
				}
				if let Some(replaced) = &v.replaced {
					orphan(&mut batch, replaced);
					if replaced.inode.kind != FsKind::Dir {
						batch.rows.insert(
							RowKey::Link(replaced.ino, v.to_parent.ino, v.to_name.to_vec()),
							None,
						);
					}
				}
			}
			FsOp::FsSetAttr(v) => {
				chunks(&mut batch, &v.target, &v.chunks)?;
				if let Some(first) = v.drop_chunks_from {
					remove_range(
						&mut batch,
						store,
						RowRange::ChunkSuffix {
							ino: v.target.ino,
							first,
						},
						seq,
					)?;
				}
			}
			FsOp::FsWrite(v) => chunks(&mut batch, &v.target, &v.chunks)?,
			FsOp::FsSetXattr(v) => {
				inode(&mut batch, &v.target);
				batch.rows.insert(
					RowKey::Xattr(v.target.ino, v.name.to_vec()),
					Some(Row::Bytes(v.value.clone())),
				);
			}
			FsOp::FsRemoveXattr(v) => {
				inode(&mut batch, &v.target);
				batch
					.rows
					.insert(RowKey::Xattr(v.target.ino, v.name.to_vec()), None);
			}
			FsOp::FsReclaim(v) => {
				remove_range(&mut batch, store, RowRange::Metadata(v.ino), seq)?;
				remove_range(&mut batch, store, RowRange::Chunks(v.ino), seq)?;
				batch.rows.insert(RowKey::Orphan(v.ino), None);
			}
			FsOp::FsKvPut(v) => {
				batch.rows.insert(
					RowKey::Kv(v.ino, v.key.to_vec()),
					Some(Row::Bytes(v.value.clone())),
				);
			}
			FsOp::FsKvDelete(v) => {
				batch.rows.insert(RowKey::Kv(v.ino, v.key.to_vec()), None);
			}
			FsOp::FsKvDeleteRange(v) => remove_range(
				&mut batch,
				store,
				RowRange::Kv {
					ino: v.ino,
					start: v.start.to_vec(),
					end: v.end.clone().map(Into::into),
				},
				seq,
			)?,
			FsOp::FsCheckpoint(v) => {
				batch.checkpoints.insert((seq, v.name.to_vec()), v.clone());
			}
			FsOp::FsCheckpointRelease(_)
			| FsOp::FsRetentionSet(_)
			| FsOp::FsForkHoldRelease(_)
			| FsOp::FsForkCheckDone(_) => batch.admin.push((seq, op.clone())),
		}
	}
	Ok(batch)
}

/// The bounded application plan keeps postimages from the entry itself and
/// names ranges to scan at seq-1. Apply range tombstones first, then these final
/// postimages; this preserves put/delete-range/put ordering without retaining
/// every row touched by a large delete. Readers publish seq only after both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyPlan {
	pub ranges: Vec<RowRange>,
	pub final_rows: Batch,
}

pub fn plan(seq: u64, entry: &FsEntry) -> Result<ApplyPlan> {
	struct Ranges(std::cell::RefCell<Vec<RowRange>>);
	impl RowStore for Ranges {
		fn get(&self, _: &RowKey, _: Option<u64>) -> Result<Option<Row>> {
			Err(Error::Corrupt)
		}
		fn scan(
			&self,
			range: &RowRange,
			after: Option<&RowKey>,
			_: usize,
			_: Option<u64>,
		) -> Result<Vec<(RowKey, Row)>> {
			if after.is_some() {
				return Err(Error::Corrupt);
			}
			self.0.borrow_mut().push(range.clone());
			Ok(Vec::new())
		}
	}
	let ranges = Ranges(std::cell::RefCell::new(Vec::new()));
	let final_rows = apply(seq, entry, &ranges)?;
	Ok(ApplyPlan {
		ranges: ranges.0.into_inner(),
		final_rows,
	})
}
