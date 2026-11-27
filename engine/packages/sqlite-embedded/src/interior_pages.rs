//! Each database's own copies of its b-tree interior pages.
//!
//! The VFS buffers drop a page once SQLite reads it, because SQLite's pager then owns it (see
//! `take_buffered_page`). A page SQLite only reads, such as a table's root, then lives only in
//! the process-wide pager cache, which every database shares. Under heavy writes that cache
//! turns over in seconds, and the next statement of a database that paused that long reads each
//! level of its b-tree from depot again, one round trip per level, while it holds the database's
//! worker or a synchronous caller's thread. Interior pages are a small fraction of a database,
//! so every database keeps copies of them apart from the shared caches.
//!
//! Each copy is the newest one depot acknowledged. A commit replaces or drops it, so a copy can
//! stand in for depot like the committed-page buffer.
//!
//! Memory is bounded per database and per process, so it stays fixed however many databases a
//! process opens. At either bound a database replaces its oldest copy.

use std::sync::atomic::{AtomicU64, Ordering};

use lru::LruCache;
use sqlite_types::PageBytes;

/// About 0.3 % of a 340 MB database of 4 KiB pages.
const MAX_PAGES_PER_DATABASE: usize = 256;
/// What every database in the process may hold together.
const MAX_PROCESS_BYTES: u64 = 64 * 1024 * 1024;

static PROCESS_BYTES: AtomicU64 = AtomicU64::new(0);

/// Whether `page` is a table or index b-tree interior page. Page 1 has its b-tree header after
/// the 100-byte database header.
fn is_interior(pgno: u32, page: &PageBytes) -> bool {
	let header = if pgno == 1 { 100 } else { 0 };
	matches!(page.as_slice().get(header), Some(0x02 | 0x05))
}

fn reserve(bytes: u64) -> bool {
	PROCESS_BYTES
		.try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
			let next = used.checked_add(bytes)?;
			(next <= MAX_PROCESS_BYTES).then_some(next)
		})
		.is_ok()
}

fn release(bytes: u64) {
	PROCESS_BYTES.fetch_sub(bytes, Ordering::AcqRel);
}

#[derive(Debug)]
pub(crate) struct InteriorPages {
	/// Least recently stored first.
	pages: LruCache<u32, PageBytes>,
	bytes: u64,
}

impl Default for InteriorPages {
	fn default() -> Self {
		Self {
			pages: LruCache::unbounded(),
			bytes: 0,
		}
	}
}

impl InteriorPages {
	pub(crate) fn get(&self, pgno: u32) -> Option<&PageBytes> {
		self.pages.peek(&pgno)
	}

	/// Keeps a page SQLite read from depot or a buffer, unless a copy is already kept.
	pub(crate) fn note_read(&mut self, pgno: u32, page: &PageBytes) {
		if is_interior(pgno, page) && !self.pages.contains(&pgno) {
			// A copy, so a long-lived page never keeps a whole response buffer alive.
			self.store(pgno, PageBytes::copy_from_slice(page.as_slice()));
		}
	}

	/// Replaces the copy of a page depot acknowledged, or drops it when the page is no longer
	/// interior.
	pub(crate) fn note_committed(&mut self, pgno: u32, page: &PageBytes) {
		if is_interior(pgno, page) {
			self.store(pgno, page.clone());
		} else {
			self.remove(pgno);
		}
	}

	pub(crate) fn clear(&mut self) {
		self.pages.clear();
		release(std::mem::take(&mut self.bytes));
	}

	pub(crate) fn len(&self) -> usize {
		self.pages.len()
	}

	fn remove(&mut self, pgno: u32) {
		if let Some(old) = self.pages.pop(&pgno) {
			self.bytes -= old.len() as u64;
			release(old.len() as u64);
		}
	}

	fn store(&mut self, pgno: u32, page: PageBytes) {
		self.remove(pgno);
		let bytes = page.len() as u64;
		while self.pages.len() >= MAX_PAGES_PER_DATABASE || !reserve(bytes) {
			let Some((_, oldest)) = self.pages.pop_lru() else {
				return;
			};
			self.bytes -= oldest.len() as u64;
			release(oldest.len() as u64);
		}
		self.bytes += bytes;
		self.pages.put(pgno, page);
	}
}

impl Drop for InteriorPages {
	fn drop(&mut self) {
		self.clear();
	}
}
