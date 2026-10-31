//! Build one file update from ordered writes, a final size and atomic metadata.

use super::{Error, Filesystem, Result};
use depot_fs_core::{
	CHUNK_BYTES, FsEntry, FsOp, apply, rules,
	store::{Row, RowKey, RowRange, RowStore},
};
use std::collections::BTreeMap;

/// Ordered byte writes. Later writes replace overlapping bytes in the same entry.
#[derive(Clone, Debug)]
pub struct FileWrite {
	pub offset: u64,
	pub bytes: vbare::Bytes,
}

/// Ordered edits preserve truncate-then-grow zeroing within one atomic entry.
#[derive(Clone, Debug)]
pub enum FileEdit {
	Write(FileWrite),
	Truncate(u64),
	/// Remove an optional attribute after earlier edits, preserving their inode postimage.
	RemoveXattrIfPresent(Vec<u8>),
}

/// A file's data, final size and application metadata become visible together.
#[derive(Clone, Debug, Default)]
pub struct FileUpdate {
	pub writes: Vec<FileWrite>,
	pub size: Option<u64>,
	pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
}

struct Postimages<'a, S> {
	base: &'a S,
	rows: BTreeMap<RowKey, Option<Row>>,
	ranges: Vec<RowRange>,
}
impl<S: RowStore> RowStore for Postimages<'_, S> {
	fn get(&self, key: &RowKey, as_of: Option<u64>) -> depot_fs_core::Result<Option<Row>> {
		if as_of.is_some() {
			return Err(depot_fs_core::Error::Invalid);
		}
		match self.rows.get(key) {
			Some(row) => Ok(row.clone()),
			None if self.ranges.iter().any(|range| range.contains(key)) => Ok(None),
			None => self.base.get(key, None),
		}
	}
	fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
		as_of: Option<u64>,
	) -> depot_fs_core::Result<Vec<(RowKey, Row)>> {
		if as_of.is_some() {
			return Err(depot_fs_core::Error::Invalid);
		}
		if limit == 0 {
			return Ok(Vec::new());
		}
		let mut cursor = after.cloned();
		let mut visible = BTreeMap::new();
		loop {
			let page = self
				.base
				.scan(range, cursor.as_ref(), limit.min(256), None)?;
			if page.is_empty() {
				break;
			}
			cursor = page.last().map(|(key, _)| key.clone());
			for (key, value) in page {
				if !self.ranges.iter().any(|range| range.contains(&key))
					&& !self.rows.contains_key(&key)
				{
					visible.insert(key, value);
				}
			}
			if visible.len() >= limit {
				break;
			}
		}
		for (key, value) in &self.rows {
			if range.contains(key) && after.is_none_or(|after| key > after) {
				if let Some(value) = value {
					visible.insert(key.clone(), value.clone());
				}
			}
		}
		Ok(visible.into_iter().take(limit).collect())
	}
}
impl<S: RowStore> Postimages<'_, S> {
	fn append(&mut self, seq: u64, op: FsOp, ops: &mut Vec<FsOp>) -> depot_fs_core::Result<()> {
		let plan = apply::plan(
			seq,
			&FsEntry {
				txn: None,
				ops: vec![op.clone()],
			},
		)?;
		for range in &plan.ranges {
			self.rows.retain(|key, _| !range.contains(key));
		}
		self.ranges.extend(plan.ranges);
		self.rows.extend(plan.final_rows.rows);
		ops.push(op);
		Ok(())
	}
}
/// Charges an adapter's pending byte buffer to this filesystem opening.
/// Dropping it returns the reservation even when the adapter is cancelled.
pub struct FileBuffer {
	_charge: super::cache::Charge,
}
impl Filesystem {
	pub fn reserve_file_buffer(&self, bytes: usize) -> Result<FileBuffer> {
		self.status()?;
		Ok(FileBuffer {
			_charge: self.budget.reserve(bytes)?,
		})
	}

	/// Maximum encoded entry size for this opening, including filesystem metadata.
	pub fn max_append_bytes(&self) -> Result<usize> {
		Ok(self
			.status()?
			.max_payload_bytes
			.min(self.config.max_operation_bytes))
	}

	/// Rejects an oversized batch before log admission. The caller can safely use
	/// a journal after `TooLarge`; a log or fencing error requires recovery.
	pub async fn update_file(&self, ino: u64, update: &FileUpdate) -> Result<u64> {
		self.update_file_with_ops(ino, update, Vec::new()).await
	}

	/// Like [`Self::update_file`], with `extra` operations in the same entry, so they become
	/// visible with the file's data. See [`Self::edit_file_with_ops`].
	pub async fn update_file_with_ops(
		&self,
		ino: u64,
		update: &FileUpdate,
		extra: Vec<FsOp>,
	) -> Result<u64> {
		let copied = update
			.writes
			.iter()
			.try_fold(0usize, |n, write| {
				n.checked_add(write.bytes.len())
					.and_then(|n| n.checked_add(256))
			})
			.ok_or(Error::TooLarge)?;
		if copied > self.max_append_bytes()? {
			return Err(Error::TooLarge);
		}
		let _copied = self.budget.reserve(copied)?;
		let mut edits: Vec<_> = update.writes.iter().cloned().map(FileEdit::Write).collect();
		if let Some(size) = update.size {
			edits.push(FileEdit::Truncate(size));
		}
		self.edit_file_with_ops(ino, &edits, &update.xattrs, extra)
			.await
	}

	/// Atomically applies ordered writes and truncates, followed by metadata.
	pub async fn edit_file(
		&self,
		ino: u64,
		edits: &[FileEdit],
		xattrs: &[(Vec<u8>, Vec<u8>)],
	) -> Result<u64> {
		self.edit_file_with_ops(ino, edits, xattrs, Vec::new())
			.await
	}

	/// Like [`Self::edit_file`], with `extra` operations appended to the same entry after the
	/// file's operations, so one entry carries a file's data together with rows that describe
	/// it, such as KV rows on another inode. The extra operations are the caller's, already
	/// built; each is applied over the postimages of the operations before it. An entry over
	/// the byte cap is refused before anything is appended.
	pub async fn edit_file_with_ops(
		&self,
		ino: u64,
		edits: &[FileEdit],
		xattrs: &[(Vec<u8>, Vec<u8>)],
		extra: Vec<FsOp>,
	) -> Result<u64> {
		let _scratch = self.reserve_edits(edits, xattrs, &extra)?;
		self.mutate(|base, seq, now| build(base, seq, now, ino, edits, xattrs, &extra))
			.await
	}
	pub(super) fn reserve_edits(
		&self,
		edits: &[FileEdit],
		xattrs: &[(Vec<u8>, Vec<u8>)],
		extra: &[FsOp],
	) -> Result<FileBuffer> {
		let mut bytes = 0usize;
		for edit in edits {
			let length = match edit {
				FileEdit::Write(write) => write.bytes.len(),
				FileEdit::Truncate(_) => 0,
				FileEdit::RemoveXattrIfPresent(name) => name.len(),
			};
			bytes = bytes
				.checked_add(length)
				.and_then(|n| n.checked_add(256))
				.ok_or(Error::TooLarge)?;
		}
		for (name, value) in xattrs {
			bytes = bytes
				.checked_add(name.len())
				.and_then(|n| n.checked_add(value.len()))
				.and_then(|n| n.checked_add(256))
				.ok_or(Error::TooLarge)?;
		}
		for op in extra {
			bytes = bytes
				.checked_add(extra_op_bytes(op))
				.ok_or(Error::TooLarge)?;
		}
		if bytes > self.config.max_operation_bytes {
			return Err(Error::TooLarge);
		}
		// The scratch map and operation list each hold postimages. Partial
		// writes can merge two extra edge chunks per write; account for those too.
		let scratch = bytes
			.checked_add(
				edits
					.len()
					.checked_mul(CHUNK_BYTES * 2)
					.ok_or(Error::TooLarge)?,
			)
			.and_then(|n| n.checked_mul(2))
			.ok_or(Error::TooLarge)?;
		Ok(FileBuffer {
			_charge: self.budget.reserve(scratch)?,
		})
	}
}

/// Bytes an extra operation is charged before its entry is built: a KV row costs what
/// [`super::Transaction::put`] charges it, and any other operation the fixed metadata margin a
/// file edit gets, plus the bytes it carries.
fn extra_op_bytes(op: &FsOp) -> usize {
	match op {
		FsOp::FsKvPut(v) => 96 + v.key.len() + v.value.len(),
		FsOp::FsKvDelete(v) => 96 + v.key.len(),
		FsOp::FsKvDeleteRange(v) => 96 + v.start.len() + v.end.as_ref().map_or(0, |end| end.len()),
		FsOp::FsSetXattr(v) => 256 + v.name.len() + v.value.len(),
		FsOp::FsWrite(v) => {
			256 + v
				.chunks
				.iter()
				.map(|chunk| chunk.bytes.len())
				.sum::<usize>()
		}
		FsOp::FsSetAttr(v) => {
			256 + v
				.chunks
				.iter()
				.map(|chunk| chunk.bytes.len())
				.sum::<usize>()
		}
		FsOp::FsRoot(_)
		| FsOp::FsCreate(_)
		| FsOp::FsMkdir(_)
		| FsOp::FsSymlink(_)
		| FsOp::FsMknod(_)
		| FsOp::FsLink(_)
		| FsOp::FsUnlink(_)
		| FsOp::FsRename(_)
		| FsOp::FsRemoveXattr(_)
		| FsOp::FsReclaim(_)
		| FsOp::FsCheckpoint(_)
		| FsOp::FsCheckpointRelease(_)
		| FsOp::FsRetentionSet(_)
		| FsOp::FsForkHoldRelease(_)
		| FsOp::FsForkCheckDone(_) => 256,
	}
}

/// Shared postimage builder for standalone and pending filesystem writes. `extra` follows the
/// file's operations in the entry, each applied over the postimages before it.
pub(super) fn build<S: RowStore>(
	base: &S,
	seq: u64,
	now: i64,
	ino: u64,
	edits: &[FileEdit],
	xattrs: &[(Vec<u8>, Vec<u8>)],
	extra: &[FsOp],
) -> depot_fs_core::Result<Vec<FsOp>> {
	let mut view = Postimages {
		base,
		rows: BTreeMap::new(),
		ranges: Vec::new(),
	};
	let mut ops = Vec::new();
	for edit in edits {
		let op = match edit {
			FileEdit::RemoveXattrIfPresent(name) => {
				match rules::removexattr(&view, ino, name.clone(), now) {
					Ok(op) => Some(op),
					Err(depot_fs_core::Error::NoAttribute) => None,
					Err(error) => return Err(error),
				}
			}
			FileEdit::Write(write) => rules::write(&view, ino, write.offset, &write.bytes, now)?,
			FileEdit::Truncate(size) => Some(rules::setattr(
				&view,
				ino,
				rules::Attributes {
					size: Some(*size),
					..Default::default()
				},
				now,
			)?),
		};
		if let Some(op) = op {
			view.append(seq, op, &mut ops)?;
		}
	}
	for (name, value) in xattrs {
		let op = rules::setxattr(&view, ino, name.clone(), value.clone(), false, false, now)?;
		view.append(seq, op, &mut ops)?;
	}
	for op in extra {
		view.append(seq, op.clone(), &mut ops)?;
	}
	Ok(ops)
}
