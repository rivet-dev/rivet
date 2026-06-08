//! Minimal SQLite on-disk page parsing for overflow-aware page fetching.
//!
//! A full table scan over rows whose payloads spill onto overflow pages would
//! otherwise force the actor-side VFS to issue one network round trip per row:
//! each leaf cell points at a separate overflow chain that SQLite reads only
//! when it reaches that row. Because the read path already has the leaf page
//! bytes in hand to serve them, it parses the leaf here, discovers the overflow
//! pages, and returns them alongside the requested leaf. The VFS caches the
//! extra pages, so the later reads of those overflow pages become cache hits.

/// On-disk SQLite B-tree page kinds plus a catch-all for everything else
/// (overflow pages, freelist pages, pointer-map pages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PageKind {
	InteriorIndex,
	InteriorTable,
	LeafIndex,
	LeafTable,
	/// Not a B-tree page: overflow, freelist trunk/leaf, or pointer-map page.
	Other,
}

/// Offset of the B-tree page header within the page bytes. Page 1 carries the
/// 100-byte database header before its B-tree header.
fn header_offset(pgno: u32) -> usize {
	if pgno == 1 { 100 } else { 0 }
}

/// Classify a page from its raw bytes using the one-byte page type at the header
/// offset (0x02 interior index, 0x05 interior table, 0x0a leaf index, 0x0d leaf
/// table). Any other value is not a B-tree page.
fn classify(pgno: u32, page: &[u8]) -> PageKind {
	let hdr = header_offset(pgno);
	match page.get(hdr) {
		Some(0x02) => PageKind::InteriorIndex,
		Some(0x05) => PageKind::InteriorTable,
		Some(0x0a) => PageKind::LeafIndex,
		Some(0x0d) => PageKind::LeafTable,
		_ => PageKind::Other,
	}
}

/// Read a SQLite varint (big-endian, up to 9 bytes) starting at `offset`.
/// Returns the decoded value and the number of bytes consumed, or `None` if the
/// slice ends before the varint terminates.
fn read_varint(page: &[u8], offset: usize) -> Option<(u64, usize)> {
	let mut value: u64 = 0;
	let mut idx = 0;
	while idx < 8 {
		let byte = *page.get(offset + idx)?;
		value = (value << 7) | u64::from(byte & 0x7f);
		idx += 1;
		if byte & 0x80 == 0 {
			return Some((value, idx));
		}
	}
	// The ninth byte contributes all 8 of its bits.
	let byte = *page.get(offset + 8)?;
	value = (value << 8) | u64::from(byte);
	Some((value, 9))
}

/// Overflow-spill threshold parameters derived from the usable page size.
struct SpillParams {
	max_local: usize,
	min_local: usize,
	usable: usize,
}

impl SpillParams {
	fn new(usable: usize, index: bool) -> Self {
		// SQLite's payload-overflow formula (see the file-format spec).
		let max_local = if index {
			((usable - 12) * 64 / 255) - 23
		} else {
			usable - 35
		};
		let min_local = ((usable - 12) * 32 / 255) - 23;
		SpillParams {
			max_local,
			min_local,
			usable,
		}
	}

	/// Number of payload bytes stored locally for a cell whose total payload is
	/// `payload`, or `None` when the payload fits entirely on the page.
	fn local_bytes(&self, payload: usize) -> Option<usize> {
		if payload <= self.max_local {
			return None;
		}
		let surplus = self.min_local + (payload - self.min_local) % (self.usable - 4);
		let local = if surplus <= self.max_local {
			surplus
		} else {
			self.min_local
		};
		Some(local)
	}
}

/// Collect the first overflow page of every cell on a leaf or interior-index
/// page whose payload spills. Returns an empty vector for pages that cannot hold
/// overflowing cells (interior-table pages, overflow pages, freelist pages) or
/// that fail to parse.
///
/// `page_size` is the database page size and `reserved` is the per-page reserved
/// byte count from the database header (usually zero). Out-of-range page numbers
/// are dropped so a misparse cannot point the fetcher at bogus pages.
pub(super) fn overflow_head_pages(
	pgno: u32,
	page: &[u8],
	page_size: usize,
	reserved: usize,
	db_size_pages: u32,
) -> Vec<u32> {
	let (index, has_left_child, has_rowid, header_size) = match classify(pgno, page) {
		PageKind::LeafTable => (false, false, true, 8),
		PageKind::LeafIndex => (true, false, false, 8),
		PageKind::InteriorIndex => (true, true, false, 12),
		// Interior-table cells carry no payload, and overflow, freelist, and
		// pointer-map pages have no cell structure to walk.
		PageKind::InteriorTable | PageKind::Other => return Vec::new(),
	};

	let usable = page_size.saturating_sub(reserved);
	if usable <= 12 || page.len() < page_size {
		return Vec::new();
	}
	let spill = SpillParams::new(usable, index);

	let hdr = header_offset(pgno);
	let Some(num_cells) = page.get(hdr + 3..hdr + 5) else {
		return Vec::new();
	};
	let num_cells = u16::from_be_bytes([num_cells[0], num_cells[1]]) as usize;
	let pointer_array = hdr + header_size;

	let mut heads = Vec::new();
	for cell_idx in 0..num_cells {
		let ptr_at = pointer_array + cell_idx * 2;
		let Some(ptr_bytes) = page.get(ptr_at..ptr_at + 2) else {
			break;
		};
		let cell_off = u16::from_be_bytes([ptr_bytes[0], ptr_bytes[1]]) as usize;
		let mut cursor = cell_off;
		if has_left_child {
			cursor += 4;
		}
		let Some((payload, payload_len)) = read_varint(page, cursor) else {
			continue;
		};
		cursor += payload_len;
		if has_rowid {
			let Some((_, rowid_len)) = read_varint(page, cursor) else {
				continue;
			};
			cursor += rowid_len;
		}
		let payload = payload as usize;
		let Some(local) = spill.local_bytes(payload) else {
			continue;
		};
		let overflow_at = cursor + local;
		let Some(overflow_bytes) = page.get(overflow_at..overflow_at + 4) else {
			continue;
		};
		let overflow_pgno = u32::from_be_bytes([
			overflow_bytes[0],
			overflow_bytes[1],
			overflow_bytes[2],
			overflow_bytes[3],
		]);
		if overflow_pgno >= 1 && overflow_pgno <= db_size_pages {
			heads.push(overflow_pgno);
		}
	}

	heads
}

/// Read the next page number from an overflow page's leading 4-byte forward
/// pointer. Returns `None` when the chain terminates (pointer is zero) or points
/// out of range.
pub(super) fn overflow_next_page(page: &[u8], db_size_pages: u32) -> Option<u32> {
	let bytes = page.get(0..4)?;
	let next = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
	if next >= 1 && next <= db_size_pages {
		Some(next)
	} else {
		None
	}
}

/// Read the per-page reserved byte count from a database header page (page 1).
/// Returns `None` when the slice is not a valid SQLite header page.
pub(super) fn header_reserved_bytes(page: &[u8]) -> Option<usize> {
	const HEADER_MAGIC: &[u8; 16] = b"SQLite format 3\0";
	if page.len() < 100 || &page[..HEADER_MAGIC.len()] != HEADER_MAGIC {
		return None;
	}
	Some(usize::from(page[20]))
}
