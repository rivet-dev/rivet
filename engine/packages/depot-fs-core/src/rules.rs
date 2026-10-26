//! Filesystem validation resolves complete postimages before an operation enters the log.

use crate::store::{Row, RowKey, RowRange, RowStore, inode};
use crate::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn name(value: &[u8]) -> Result<()> {
	if value.len() > 255 {
		return Err(Error::NameTooLong);
	}
	if value.is_empty()
		|| value == b"."
		|| value == b".."
		|| value.contains(&0)
		|| value.contains(&b'/')
	{
		return Err(Error::Invalid);
	}
	Ok(())
}

pub fn new_inode(kind: FsKind, mode: u32, uid: u32, gid: u32, now: i64) -> FsInode {
	let directory = kind == FsKind::Dir;
	let inline = (kind == FsKind::File).then(Vec::new);
	FsInode {
		kind,
		mode: mode & 0o7777,
		uid,
		gid,
		rdev: 0,
		size: 0,
		nlink: if directory { 2 } else { 1 },
		times: FsTimes {
			atime: now,
			mtime: now,
			ctime: now,
		},
		parent: ROOT_INO,
		inline: inline.map(Into::into),
		inherited_limit: None,
		target: Vec::new().into(),
		entries: 0,
		xattrs: 0,
	}
}

pub fn root(mode: u32, uid: u32, gid: u32, now: i64) -> FsOp {
	FsOp::FsRoot(FsRoot {
		inode: new_inode(FsKind::Dir, mode, uid, gid, now),
		format: FsFormat {
			inline_max_bytes: DEFAULT_INLINE_MAX_BYTES,
		},
	})
}

fn update(store: &impl RowStore, ino: u64) -> Result<FsInodeUpdate> {
	Ok(FsInodeUpdate {
		ino,
		inode: inode(store, ino, None)?,
	})
}

fn directory(store: &impl RowStore, ino: u64) -> Result<FsInodeUpdate> {
	let value = update(store, ino)?;
	if value.inode.kind != FsKind::Dir {
		return Err(Error::NotDirectory);
	}
	Ok(value)
}

fn touch(value: &mut FsInodeUpdate, now: i64) {
	value.inode.times.mtime = now;
	value.inode.times.ctime = now;
}

pub fn lookup(
	store: &impl RowStore,
	parent: u64,
	entry: &[u8],
	as_of: Option<u64>,
) -> Result<FsDentry> {
	let dir = inode(store, parent, as_of)?;
	if dir.kind != FsKind::Dir {
		return Err(Error::NotDirectory);
	}
	if entry == b"." {
		return Ok(FsDentry {
			child: parent,
			kind: FsKind::Dir,
		});
	}
	if entry == b".." {
		return Ok(FsDentry {
			child: dir.parent,
			kind: FsKind::Dir,
		});
	}
	name(entry)?;
	match store.get(&RowKey::Dentry(parent, entry.to_vec()), as_of)? {
		Some(Row::Dentry(value)) => Ok(value),
		None => Err(Error::NotFound),
		_ => Err(Error::Corrupt),
	}
}

pub fn create(
	store: &impl RowStore,
	parent: u64,
	entry: Vec<u8>,
	mut child: FsInode,
	now: i64,
) -> Result<FsOp> {
	let inline_max_bytes = crate::store::format(store)?.inline_max_bytes;
	name(&entry)?;
	let mut parent = directory(store, parent)?;
	if store
		.get(&RowKey::Dentry(parent.ino, entry.clone()), None)?
		.is_some()
	{
		return Err(Error::Exists);
	}
	if child.kind == FsKind::Symlink && (child.target.is_empty() || child.target.contains(&0)) {
		return Err(Error::Invalid);
	}
	if child
		.inline
		.as_ref()
		.is_some_and(|bytes| bytes.len() as u64 > inline_max_bytes)
	{
		return Err(Error::TooLarge);
	}
	parent.inode.entries = parent.inode.entries.checked_add(1).ok_or(Error::TooLarge)?;
	if child.kind == FsKind::Dir {
		parent.inode.nlink = parent.inode.nlink.checked_add(1).ok_or(Error::TooLarge)?;
		child.parent = parent.ino;
	}
	touch(&mut parent, now);
	Ok(match child.kind {
		FsKind::Dir => FsOp::FsMkdir(FsMkdir {
			parent,
			name: entry.into(),
			inode: child,
		}),
		FsKind::Symlink => {
			child.size = child.target.len() as u64;
			FsOp::FsSymlink(FsSymlink {
				parent,
				name: entry.into(),
				inode: child,
			})
		}
		FsKind::File | FsKind::Kv => FsOp::FsCreate(FsCreate {
			parent,
			name: entry.into(),
			inode: child,
		}),
		_ => FsOp::FsMknod(FsMknod {
			parent,
			name: entry.into(),
			inode: child,
		}),
	})
}

pub fn link(
	store: &impl RowStore,
	ino: u64,
	parent: u64,
	entry: Vec<u8>,
	now: i64,
) -> Result<FsOp> {
	name(&entry)?;
	let mut target = update(store, ino)?;
	if target.inode.kind == FsKind::Dir {
		return Err(Error::IsDirectory);
	}
	if target.inode.nlink == 0 {
		return Err(Error::NotFound);
	}
	let mut parent = directory(store, parent)?;
	if store
		.get(&RowKey::Dentry(parent.ino, entry.clone()), None)?
		.is_some()
	{
		return Err(Error::Exists);
	}
	parent.inode.entries = parent.inode.entries.checked_add(1).ok_or(Error::TooLarge)?;
	target.inode.nlink = target.inode.nlink.checked_add(1).ok_or(Error::TooLarge)?;
	touch(&mut parent, now);
	target.inode.times.ctime = now;
	Ok(FsOp::FsLink(FsLink {
		parent,
		name: entry.into(),
		target,
	}))
}

fn remove_link(target: &mut FsInodeUpdate, now: i64) -> Result<()> {
	if target.inode.kind == FsKind::Dir {
		if target.inode.entries != 0 {
			return Err(Error::NotEmpty);
		}
		target.inode.nlink = 0;
	} else {
		target.inode.nlink = target.inode.nlink.checked_sub(1).ok_or(Error::Corrupt)?;
	}
	target.inode.times.ctime = now;
	Ok(())
}

pub fn unlink(
	store: &impl RowStore,
	parent: u64,
	entry: Vec<u8>,
	rmdir: bool,
	now: i64,
) -> Result<FsOp> {
	name(&entry)?;
	let mut target = update(store, lookup(store, parent, &entry, None)?.child)?;
	let mut parent = directory(store, parent)?;
	if (target.inode.kind == FsKind::Dir) != rmdir {
		return Err(if rmdir {
			Error::NotDirectory
		} else {
			Error::IsDirectory
		});
	}
	remove_link(&mut target, now)?;
	parent.inode.entries = parent.inode.entries.checked_sub(1).ok_or(Error::Corrupt)?;
	if rmdir {
		parent.inode.nlink = parent.inode.nlink.checked_sub(1).ok_or(Error::Corrupt)?;
	}
	touch(&mut parent, now);
	Ok(FsOp::FsUnlink(FsUnlink {
		parent,
		name: entry.into(),
		target,
	}))
}

pub fn rename(
	store: &impl RowStore,
	from: u64,
	from_name: Vec<u8>,
	to: u64,
	to_name: Vec<u8>,
	now: i64,
) -> Result<Option<FsOp>> {
	name(&from_name)?;
	name(&to_name)?;
	let source = lookup(store, from, &from_name, None)?;
	let mut from_parent = directory(store, from)?;
	let mut to_parent = directory(store, to)?;
	let mut target = update(store, source.child)?;
	let replacement = match lookup(store, to, &to_name, None) {
		Ok(value) => Some(value.child),
		Err(Error::NotFound) => None,
		Err(error) => return Err(error),
	};
	if replacement == Some(source.child) {
		return Ok(None);
	}
	let is_dir = target.inode.kind == FsKind::Dir;
	if is_dir {
		let mut ancestor = to;
		let mut seen = BTreeSet::new();
		loop {
			if ancestor == target.ino {
				return Err(Error::Invalid);
			}
			if ancestor == ROOT_INO {
				break;
			}
			if !seen.insert(ancestor) {
				return Err(Error::Corrupt);
			}
			ancestor = directory(store, ancestor)?.inode.parent;
		}
	}
	let mut replaced = replacement.map(|ino| update(store, ino)).transpose()?;
	if let Some(replaced) = &mut replaced {
		if is_dir && replaced.inode.kind != FsKind::Dir {
			return Err(Error::NotDirectory);
		}
		if !is_dir && replaced.inode.kind == FsKind::Dir {
			return Err(Error::IsDirectory);
		}
		remove_link(replaced, now)?;
	}
	from_parent.inode.entries = from_parent
		.inode
		.entries
		.checked_sub(1)
		.ok_or(Error::Corrupt)?;
	if is_dir {
		from_parent.inode.nlink = from_parent
			.inode
			.nlink
			.checked_sub(1)
			.ok_or(Error::Corrupt)?;
	}
	if from == to {
		to_parent = from_parent.clone();
	}
	if replaced.is_none() {
		to_parent.inode.entries = to_parent
			.inode
			.entries
			.checked_add(1)
			.ok_or(Error::TooLarge)?;
	}
	if is_dir && replaced.is_none() {
		to_parent.inode.nlink = to_parent
			.inode
			.nlink
			.checked_add(1)
			.ok_or(Error::TooLarge)?;
	}
	touch(&mut from_parent, now);
	touch(&mut to_parent, now);
	if from == to {
		from_parent = to_parent.clone();
	}
	if is_dir {
		target.inode.parent = to;
	}
	target.inode.times.ctime = now;
	Ok(Some(FsOp::FsRename(FsRename {
		from_parent,
		from_name: from_name.into(),
		to_parent,
		to_name: to_name.into(),
		target,
		replaced,
	})))
}

fn chunk(store: &impl RowStore, ino: u64, index: u64, as_of: Option<u64>) -> Result<Vec<u8>> {
	let mut bytes = match store.get(&RowKey::Chunk(ino, index), as_of)? {
		// The chunk is edited in place, so it needs its own buffer.
		Some(Row::Bytes(bytes)) if bytes.len() <= CHUNK_BYTES => Vec::from(bytes),
		None => Vec::new(),
		_ => return Err(Error::Corrupt),
	};
	bytes.resize(CHUNK_BYTES, 0);
	Ok(bytes)
}

pub fn read(
	store: &impl RowStore,
	ino: u64,
	offset: u64,
	count: usize,
	as_of: Option<u64>,
) -> Result<Vec<u8>> {
	let value = inode(store, ino, as_of)?;
	if value.kind != FsKind::File {
		return Err(if value.kind == FsKind::Dir {
			Error::IsDirectory
		} else {
			Error::Invalid
		});
	}
	let len = value.size.saturating_sub(offset).min(count as u64) as usize;
	if len == 0 {
		return Ok(Vec::new());
	}
	if let Some(bytes) = value.inline {
		if bytes.len() as u64 != value.size {
			return Err(Error::Corrupt);
		}
		return Ok(bytes[offset as usize..offset as usize + len].to_vec());
	}
	let mut result = Vec::with_capacity(len);
	let mut position = offset;
	while result.len() < len {
		let bytes = chunk(store, ino, position / CHUNK_BYTES as u64, as_of)?;
		let start = (position % CHUNK_BYTES as u64) as usize;
		let take = (len - result.len()).min(CHUNK_BYTES - start);
		result.extend_from_slice(&bytes[start..start + take]);
		position += take as u64;
	}
	Ok(result)
}

pub fn write(
	store: &impl RowStore,
	ino: u64,
	offset: u64,
	bytes: &[u8],
	now: i64,
) -> Result<Option<FsOp>> {
	let mut target = update(store, ino)?;
	if target.inode.kind != FsKind::File {
		return Err(if target.inode.kind == FsKind::Dir {
			Error::IsDirectory
		} else {
			Error::Invalid
		});
	}
	if bytes.is_empty() {
		return Ok(None);
	}
	let end = offset
		.checked_add(bytes.len() as u64)
		.ok_or(Error::TooLarge)?;
	let size = target.inode.size.max(end);
	let inline_max_bytes = crate::store::format(store)?.inline_max_bytes;
	let mut writes = BTreeMap::new();
	if size <= inline_max_bytes {
		let mut content = read(store, ino, 0, size as usize, None)?;
		content.resize(size as usize, 0);
		content[offset as usize..end as usize].copy_from_slice(bytes);
		target.inode.inline = Some(content.into());
	} else {
		if let Some(inline) = target.inode.inline.take() {
			if !inline.is_empty() {
				let mut content = inline.to_vec();
				content.resize(CHUNK_BYTES, 0);
				writes.insert(0, content);
			}
		}
		let mut written = 0;
		while written < bytes.len() {
			let position = offset + written as u64;
			let index = position / CHUNK_BYTES as u64;
			let start = (position % CHUNK_BYTES as u64) as usize;
			let take = (bytes.len() - written).min(CHUNK_BYTES - start);
			let mut content = match writes.remove(&index) {
				Some(value) => value,
				None if start == 0 && take == CHUNK_BYTES => vec![0; CHUNK_BYTES].into(),
				None => chunk(store, ino, index, None)?.into(),
			};
			content[start..start + take].copy_from_slice(&bytes[written..written + take]);
			writes.insert(index, content);
			written += take;
		}
		for (index, content) in &mut writes {
			content.truncate((size - index * CHUNK_BYTES as u64).min(CHUNK_BYTES as u64) as usize);
		}
	}
	target.inode.size = size;
	touch(&mut target, now);
	Ok(Some(FsOp::FsWrite(FsWrite {
		target,
		chunks: writes
			.into_iter()
			.map(|(index, bytes)| FsChunkUpdate {
				index,
				bytes: bytes.into(),
			})
			.collect(),
	})))
}

#[derive(Clone, Debug, Default)]
pub struct Attributes {
	pub mode: Option<u32>,
	pub uid: Option<u32>,
	pub gid: Option<u32>,
	pub size: Option<u64>,
	pub atime: Option<i64>,
	pub mtime: Option<i64>,
}

pub fn setattr(store: &impl RowStore, ino: u64, attrs: Attributes, now: i64) -> Result<FsOp> {
	let mut target = update(store, ino)?;
	let mut writes = Vec::new();
	let mut drop_chunks_from = None;
	if let Some(size) = attrs.size {
		let inline_max_bytes = crate::store::format(store)?.inline_max_bytes;
		if target.inode.kind != FsKind::File {
			return Err(Error::Invalid);
		}
		let old_size = target.inode.size;
		if size <= inline_max_bytes {
			let mut content = read(store, ino, 0, size as usize, None)?;
			content.resize(size as usize, 0);
			target.inode.inline = Some(content.into());
		} else if let Some(mut inline) = target.inode.inline.take() {
			inline.truncate(size as usize);
			if !inline.is_empty() {
				writes.push(FsChunkUpdate {
					index: 0,
					bytes: inline,
				});
			}
		} else if size < old_size && size % CHUNK_BYTES as u64 != 0 {
			let index = size / CHUNK_BYTES as u64;
			let mut content = chunk(store, ino, index, None)?;
			content.truncate((size % CHUNK_BYTES as u64) as usize);
			writes.push(FsChunkUpdate {
				index,
				bytes: content.into(),
			});
		}
		if size < old_size {
			target.inode.inherited_limit =
				target.inode.inherited_limit.map(|limit| limit.min(size));
			let first_dropped = if size <= inline_max_bytes {
				0
			} else {
				size.div_ceil(CHUNK_BYTES as u64)
			};
			drop_chunks_from = Some(first_dropped);
		}
		target.inode.size = size;
		target.inode.times.mtime = now;
	}
	if let Some(mode) = attrs.mode {
		target.inode.mode = mode & 0o7777;
	}
	if let Some(uid) = attrs.uid {
		target.inode.uid = uid;
	}
	if let Some(gid) = attrs.gid {
		target.inode.gid = gid;
	}
	if let Some(atime) = attrs.atime {
		target.inode.times.atime = atime;
	}
	if let Some(mtime) = attrs.mtime {
		target.inode.times.mtime = mtime;
	}
	target.inode.times.ctime = now;
	Ok(FsOp::FsSetAttr(FsSetAttr {
		target,
		chunks: writes,
		drop_chunks_from,
	}))
}

pub fn setxattr(
	store: &impl RowStore,
	ino: u64,
	attribute: Vec<u8>,
	value: Vec<u8>,
	create_only: bool,
	replace_only: bool,
	now: i64,
) -> Result<FsOp> {
	name(&attribute)?;
	if create_only && replace_only {
		return Err(Error::Invalid);
	}
	let mut target = update(store, ino)?;
	let exists = store
		.get(&RowKey::Xattr(ino, attribute.clone()), None)?
		.is_some();
	if exists && create_only {
		return Err(Error::Exists);
	}
	if !exists && replace_only {
		return Err(Error::NoAttribute);
	}
	if !exists {
		target.inode.xattrs = target.inode.xattrs.checked_add(1).ok_or(Error::TooLarge)?;
	}
	target.inode.times.ctime = now;
	Ok(FsOp::FsSetXattr(FsSetXattr {
		target,
		name: attribute.into(),
		value: value.into(),
	}))
}

pub fn removexattr(store: &impl RowStore, ino: u64, attribute: Vec<u8>, now: i64) -> Result<FsOp> {
	name(&attribute)?;
	let mut target = update(store, ino)?;
	if store
		.get(&RowKey::Xattr(ino, attribute.clone()), None)?
		.is_none()
	{
		return Err(Error::NoAttribute);
	}
	target.inode.xattrs = target.inode.xattrs.checked_sub(1).ok_or(Error::Corrupt)?;
	target.inode.times.ctime = now;
	Ok(FsOp::FsRemoveXattr(FsRemoveXattr {
		target,
		name: attribute.into(),
	}))
}

/// The caller binds this exclusive name cursor to its retained read session.
pub fn readdir(
	store: &impl RowStore,
	ino: u64,
	after: Option<&[u8]>,
	limit: usize,
	as_of: Option<u64>,
) -> Result<Vec<(Vec<u8>, FsDentry)>> {
	if inode(store, ino, as_of)?.kind != FsKind::Dir {
		return Err(Error::NotDirectory);
	}
	let cursor = after.map(|name| RowKey::Dentry(ino, name.to_vec()));
	store
		.scan(&RowRange::Directory(ino), cursor.as_ref(), limit, as_of)?
		.into_iter()
		.map(|(key, value)| match (key, value) {
			(RowKey::Dentry(_, name), Row::Dentry(value)) => Ok((name, value)),
			_ => Err(Error::Corrupt),
		})
		.collect()
}

/// Resolve raw path bytes, following at most 40 links. Absolute links restart at root.
pub fn resolve(
	store: &impl RowStore,
	start: u64,
	path: &[u8],
	follow_final: bool,
	as_of: Option<u64>,
) -> Result<u64> {
	if path.is_empty() || path.contains(&0) {
		return Err(Error::Invalid);
	}
	let mut current = if path.starts_with(b"/") {
		ROOT_INO
	} else {
		start
	};
	let mut parts: VecDeque<Vec<u8>> = path
		.split(|b| *b == b'/')
		.filter(|v| !v.is_empty())
		.map(|v| v.to_vec())
		.collect();
	let mut links = 0;
	let mut require_directory = path.ends_with(b"/");
	while let Some(part) = parts.pop_front() {
		let found = lookup(store, current, &part, as_of)?;
		if found.kind == FsKind::Symlink && (!parts.is_empty() || follow_final || require_directory)
		{
			links += 1;
			if links > 40 {
				return Err(Error::SymlinkLoop);
			}
			let target = inode(store, found.child, as_of)?.target;
			if target.is_empty() || target.contains(&0) {
				return Err(Error::Corrupt);
			}
			if parts.is_empty() && target.ends_with(b"/") {
				require_directory = true;
			}
			if target.starts_with(b"/") {
				current = ROOT_INO;
			}
			for part in target.split(|b| *b == b'/').filter(|v| !v.is_empty()).rev() {
				parts.push_front(part.to_vec());
			}
		} else {
			current = found.child;
		}
	}
	if require_directory && inode(store, current, as_of)?.kind != FsKind::Dir {
		return Err(Error::NotDirectory);
	}
	Ok(current)
}
