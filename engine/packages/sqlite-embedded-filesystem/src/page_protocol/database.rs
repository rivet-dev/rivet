//! An actor database is one ordinary SQLite-format file, `/actor/sqlite/main.db`, in the actor's
//! database volume. SQLite's side files sit beside it, so every SQLite-owned file stays in one
//! directory apart from the volume's other contents.
use anyhow::{Result, ensure};
use depot_client::filesystem::{
	Error, FileEdit, FileWrite, Filesystem, OpenedFilesystem, transport::OpenPrefetch,
};
use depot_fs_core::{FsKind, ROOT_INO, rules};
use rivet_envoy_protocol as p;
use std::{collections::BTreeSet, sync::Arc};

/// The directory under the volume root that holds every SQLite-owned file.
pub const ACTOR: &str = "actor";
/// The KV inode beside the database that holds the prefetch planner's rows: the recent pages
/// under [`RECENT_PREFIX`] and the overflow runs under [`OVERFLOW_PREFIX`]. One inode keeps one
/// lookup, one staged inode and one footprint entry per database; the open prefetch reads the
/// recent prefix only.
pub const PREFETCH: &str = "main.db-prefetch";
/// The first key byte of the recent-page rows.
pub const RECENT_PREFIX: u8 = 0;
/// The first key byte of the overflow run rows.
pub const OVERFLOW_PREFIX: u8 = 1;
pub const DIRECTORY: &str = "sqlite";
/// Root entries of the layout engines wrote before actor data moved under [`ACTOR`].
const LEGACY_ROOT_ENTRIES: [&str; 2] = ["kv", "sqlite"];
pub const NAME: &str = "main.db";
pub(super) const PAGE: usize = 4096;

/// Storage reads join only contiguous planned pages; speculation belongs to the planner.
const MAX_GAP_PAGES: u32 = 0;

/// A run of the file that covers requested pages, read as one range.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct PageRange {
	pub first_pgno: u32,
	pub len: usize,
	/// Each covered page that was requested: its index in the request and its offset in the
	/// range. A page requested twice appears once per request.
	pub pages: Vec<(usize, usize)>,
}

/// The file offset and byte count of a range of `len` whole pages from `first_pgno`.
fn range_bytes(first_pgno: u32, len: usize) -> (u64, usize) {
	(u64::from(first_pgno - 1) * PAGE as u64, len * PAGE)
}

/// Covers the requested pages inside the file with ranges of at most `page_range_pages`, in file
/// order, reading through gaps of at most `MAX_GAP_PAGES` between requested pages. Pages past
/// `size_pages` are left out and read back as `None`.
pub(super) fn page_ranges(pgnos: &[u32], size_pages: u32) -> Vec<PageRange> {
	page_ranges_of(pgnos, size_pages, 1024)
}

/// `page_ranges` with ranges of at most `max_range_pages`.
pub(super) fn page_ranges_of(
	pgnos: &[u32],
	size_pages: u32,
	max_range_pages: usize,
) -> Vec<PageRange> {
	let mut wanted: Vec<(u32, usize)> = pgnos
		.iter()
		.enumerate()
		.filter(|(_, pgno)| **pgno <= size_pages)
		.map(|(index, pgno)| (*pgno, index))
		.collect();
	wanted.sort_unstable();

	let mut ranges: Vec<PageRange> = Vec::new();
	for (pgno, index) in wanted {
		if let Some(last) = ranges.last_mut() {
			let end = last.first_pgno + last.len as u32;
			if pgno < end {
				last.pages.push((index, (pgno - last.first_pgno) as usize));
				continue;
			}
			let covered = (pgno - last.first_pgno) as usize + 1;
			if pgno - end <= MAX_GAP_PAGES && covered <= max_range_pages {
				last.len = covered;
				last.pages.push((index, covered - 1));
				continue;
			}
		}
		ranges.push(PageRange {
			first_pgno: pgno,
			len: 1,
			pages: vec![(index, 0)],
		});
	}
	ranges
}
/// A volume written by an engine that kept actor KV and SQLite at the volume root. Opening it
/// with the current layout would start an empty database beside the old data, so it fails
/// until the volume is reinitialized.
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"legacy_layout",
	"The actor volume predates the /actor layout and must be reinitialized."
)]
pub struct LegacyLayout;

/// Finds `/actor`, creating it in a volume that has none. A volume whose root still holds a
/// legacy entry fails with [`LegacyLayout`]; the legacy lookups run only when `/actor` is
/// missing, so an ordinary open reads nothing extra.
pub async fn actor_directory(fs: &Filesystem) -> Result<u64> {
	match fs.lookup(ROOT_INO, ACTOR.as_bytes()).await {
		Ok(entry) => {
			ensure!(entry.kind == FsKind::Dir, "invalid actor directory inode");
			return Ok(entry.child);
		}
		Err(Error::Filesystem(depot_fs_core::Error::NotFound)) => {}
		Err(error) => return Err(error.into()),
	}

	for name in LEGACY_ROOT_ENTRIES {
		match fs.lookup(ROOT_INO, name.as_bytes()).await {
			Ok(_) => return Err(LegacyLayout.build()),
			Err(Error::Filesystem(depot_fs_core::Error::NotFound)) => {}
			Err(error) => return Err(error.into()),
		}
	}

	let (actor, _) = lookup_or_create(fs, ROOT_INO, ACTOR, FsKind::Dir, 0o700).await?;
	Ok(actor)
}

/// Finds the entry `name` of `kind` in directory `parent`, creating it when it is missing.
/// Returns the entry's inode and whether this call created it.
async fn lookup_or_create(
	fs: &Filesystem,
	parent: u64,
	name: &str,
	kind: FsKind,
	mode: u32,
) -> Result<(u64, bool)> {
	match fs.lookup(parent, name.as_bytes()).await {
		Ok(entry) => {
			ensure!(entry.kind == kind, "invalid SQLite {name} inode");
			Ok((entry.child, false))
		}
		Err(Error::Filesystem(depot_fs_core::Error::NotFound)) => {
			let ino = fs
				.create(
					parent,
					name.as_bytes(),
					rules::new_inode(kind, mode, 0, 0, 0),
				)
				.await?;
			Ok((ino, true))
		}
		Err(error) => Err(error.into()),
	}
}

#[derive(Clone)]
pub struct Head {
	pub txid: u64,
	pub size_pages: u32,
}
pub struct Database {
	pub fs: Arc<Filesystem>,
	pub ino: u64,
	pub root: Head,
	pub(super) prefetch_ino: u64,
	pub(super) prefetcher: sqlite_prefetch::Prefetcher,
	pub(super) pending_lookups: Vec<sqlite_prefetch::Lookup>,
	pub(super) background: Option<super::prefetch::BackgroundRows>,
	pub(super) remove_old_hint: bool,
	pub(super) prefetch_charge: Option<depot_client::filesystem::CacheReservation>,
	/// Whether page replies carry predicted pages. Off, a reply holds only the requested pages
	/// while commits still maintain the planner's rows.
	pub(super) server_prefetch: bool,
}
/// Opens the actor's database volume as its exclusive filesystem owner, creating it inside the
/// same open when it is missing. The owner fences any other opening of the volume.
///
/// The open names what the database opening reads first, so the volume's cold snapshot
/// returns it with the open instead of answering one round trip per row: the SQLite file's
/// path and its recent-page hint. An actor's next start lands on another engine, so nothing
/// of this is in that engine's cache. The opening page read follows as one request.
pub async fn open_volume(
	client: depot_client::Client,
	key: rivet_keys::keys::VolumeKey,
) -> Result<OpenedFilesystem> {
	let prefetch = OpenPrefetch {
		path: [ACTOR, DIRECTORY, NAME]
			.map(|name| name.as_bytes().to_vec())
			.to_vec(),
		xattrs: Vec::new(),
		kv: vec![depot_client::filesystem::transport::KvPrefetch {
			path: [ACTOR, DIRECTORY, PREFETCH]
				.map(|name| name.as_bytes().to_vec())
				.to_vec(),
			start: vec![RECENT_PREFIX],
			end: Some(vec![RECENT_PREFIX + 1]),
		}],
	};
	client
		.open_or_create_filesystem_prefetching(key, prefetch)
		.await
}

/// The stored key of a planner row: its prefix byte, then the planner's own key.
pub(super) fn storage_key(prefix: u8, key: &[u8]) -> Vec<u8> {
	let mut stored = Vec::with_capacity(key.len() + 1);
	stored.push(prefix);
	stored.extend_from_slice(key);
	stored
}

impl Database {
	/// Finds the SQLite file, or, in a volume this opening found empty, stages its directory and
	/// file into the volume's first commit so the open itself writes and reads nothing.
	pub async fn from_filesystem(fs: Arc<Filesystem>) -> Result<Self> {
		use depot_client::filesystem::StagedFile;
		let file = |name, kind, mode| StagedFile { name, kind, mode };
		let files = [
			(None, file(ACTOR.as_bytes(), FsKind::Dir, 0o700)),
			(Some(0), file(DIRECTORY.as_bytes(), FsKind::Dir, 0o700)),
			(Some(1), file(NAME.as_bytes(), FsKind::File, 0o600)),
			(Some(1), file(PREFETCH.as_bytes(), FsKind::Kv, 0o600)),
		];
		let (ino, prefetch_ino, new_file) = match fs.stage_tree(&files).await?.as_deref() {
			Some([_, _, ino, prefetch]) => (*ino, *prefetch, true),
			Some(_) => anyhow::bail!("staged SQLite files have the wrong shape"),
			None => {
				let actor = actor_directory(&fs).await?;
				let (directory, _) =
					lookup_or_create(&fs, actor, DIRECTORY, FsKind::Dir, 0o700).await?;
				let (ino, new) =
					lookup_or_create(&fs, directory, NAME, FsKind::File, 0o600).await?;
				let (prefetch, _) =
					lookup_or_create(&fs, directory, PREFETCH, FsKind::Kv, 0o600).await?;
				(ino, prefetch, new)
			}
		};
		let root = read_head(&fs, ino).await?;
		let rows = sqlite_prefetch::OpenRows {
			early: fs
				.kv_get(
					prefetch_ino,
					&storage_key(RECENT_PREFIX, sqlite_prefetch::recency::EARLY_KEY),
				)
				.await?,
			frequent: fs
				.kv_get(
					prefetch_ino,
					&storage_key(RECENT_PREFIX, sqlite_prefetch::recency::FREQUENT_KEY),
				)
				.await?,
			overflow_empty: new_file,
		};
		let now = fs.clock().observe()?.boot_ms;
		let mut prefetcher =
			sqlite_prefetch::Prefetcher::open(rows, root.size_pages, now, Default::default());
		let prefetch_charge = fs.reserve_cache(prefetcher.memory_bytes());
		if prefetch_charge.is_none() {
			prefetcher = sqlite_prefetch::Prefetcher::open(
				Default::default(),
				root.size_pages,
				now,
				Default::default(),
			);
		}
		Ok(Self {
			fs,
			ino,
			root,
			prefetch_ino,
			prefetcher,
			pending_lookups: Vec::new(),
			background: None,
			remove_old_hint: !new_file,
			prefetch_charge,
			server_prefetch: true,
		})
	}

	pub async fn refresh(&mut self) -> Result<()> {
		let root = read_head(&self.fs, self.ino).await?;
		if root.txid != self.root.txid {
			self.prefetcher.commit_failed();
			self.pending_lookups.clear();
		}
		self.prefetcher.head(root.size_pages);
		self.root = root;
		Ok(())
	}
	/// Returns one entry per requested page, in request order; pages past the end are `None`.
	///
	/// Requested pages that are near each other in the file are read as one covering range,
	/// and every range is read as one filesystem read, whose chunks the volume fetches in one
	/// round of requests. A wake requests hundreds of pages, so reading them one round trip at
	/// a time, or one range at a time, dominated actor start.
	///
	/// Pages are slices of the range reads, so no page is copied on its way to the response.
	pub async fn read_pages(&self, pgnos: &[u32]) -> Result<Vec<Option<bytes::Bytes>>> {
		ensure!(pgnos.iter().all(|&pgno| pgno > 0), "invalid SQLite page");

		let ranges = page_ranges(pgnos, self.root.size_pages);
		let byte_ranges: Vec<(u64, usize)> = ranges
			.iter()
			.map(|range| range_bytes(range.first_pgno, range.len))
			.collect();
		let reads = self.fs.read_ranges(self.ino, &byte_ranges).await?;

		let mut pages = vec![None; pgnos.len()];
		for (range, bytes) in ranges.iter().zip(reads) {
			ensure!(bytes.len() == range.len * PAGE, "incomplete SQLite page");
			let bytes = bytes::Bytes::from(bytes);
			for &(index, offset) in &range.pages {
				pages[index] = Some(bytes.slice(offset * PAGE..(offset + 1) * PAGE));
			}
		}
		Ok(pages)
	}
	/// The wire single-shot path stays one standalone filesystem entry.
	pub async fn commit(&mut self, dirty: Vec<p::SqliteDirtyPage>, size_pages: u32) -> Result<u64> {
		ensure!(
			dirty.len()
				<= (sqlite_types::SHARD_SIZE * sqlite_types::COMMIT_SEGMENT_MAX_SHARDS) as usize,
			"large SQLite commits must use staging"
		);
		ensure!(
			dirty.len() * PAGE <= self.fs.max_transaction_bytes(),
			"SQLite transaction byte limit exceeded"
		);
		let mut seen = BTreeSet::new();
		ensure!(
			dirty
				.iter()
				.all(|p| p.pgno > 0 && p.bytes.len() == PAGE && seen.insert(p.pgno)),
			"invalid SQLite dirty page"
		);
		let mut builder = self.prefetcher.commit_builder();
		for page in &dirty {
			self.prefetcher
				.add_dirty(&mut builder, page.pgno, &page.bytes);
		}
		let plan = self
			.prefetch_commit(&builder, size_pages, dirty.len())
			.await?;
		let ops = self.prefetch_ops(&plan);
		let mut edits: Vec<_> = dirty
			.into_iter()
			.filter(|p| p.pgno <= size_pages)
			.map(|p| {
				FileEdit::Write(FileWrite {
					offset: u64::from(p.pgno - 1) * PAGE as u64,
					bytes: p.bytes,
				})
			})
			.collect();
		edits.extend(self.final_edits(size_pages));
		let result = self.fs.edit_file_with_ops(self.ino, &edits, &[], ops).await;
		match result {
			Ok(seq) => {
				self.prefetch_committed(plan);
				Ok(self.committed(seq, size_pages))
			}
			Err(error) => {
				self.prefetcher.commit_failed();
				Err(error.into())
			}
		}
	}

	/// Takes the head a commit of this file left, from the commit itself instead of a read of
	/// the inode. The commit's final entry always writes the inode, since the size it sets
	/// stamps the change time even when unchanged, and the filesystem gives an inode the
	/// sequence of the entry that last wrote it, so the head is `seq` and the size the commit
	/// set. Only this opening writes the file, so no other entry lands in between.
	pub(super) fn committed(&mut self, seq: u64, size_pages: u32) -> u64 {
		self.root = Head {
			txid: seq,
			size_pages,
		};
		seq
	}
}

/// Reads the SQLite file's head from its inode.
async fn read_head(fs: &Filesystem, ino: u64) -> Result<Head> {
	let inode = fs.getattr(ino).await?;
	ensure!(inode.size % PAGE as u64 == 0, "invalid SQLite file size");
	Ok(Head {
		txid: inode.change_seq,
		size_pages: (inode.size / PAGE as u64).try_into()?,
	})
}
