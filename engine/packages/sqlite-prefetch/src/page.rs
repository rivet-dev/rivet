//! Parsing of the SQLite database file format, enough to plan reads: page kinds, interior
//! child pointers, the overflow chains a B-tree page's cells name, overflow next pointers and
//! free-list trunks. Every parser takes untrusted bytes and returns `None` or a shorter result
//! for anything malformed; nothing here panics on input.

/// A SQLite page number. Page 1 is the first page of the file.
pub type Pgno = u32;

/// The byte offset of the B-tree header on page 1, after the 100-byte database header.
const PAGE1_HEADER: usize = 100;

/// The page geometry the database header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
	pub page_size: u32,
	/// The page size less the reserved bytes at the end of each page.
	pub usable_size: u32,
}

impl Default for Geometry {
	fn default() -> Self {
		Self {
			page_size: 4096,
			usable_size: 4096,
		}
	}
}

impl Geometry {
	/// Reads the geometry from page 1's database header.
	pub fn from_page1(page: &[u8]) -> Option<Self> {
		if page.len() < PAGE1_HEADER || &page[..16] != b"SQLite format 3\0" {
			return None;
		}
		let raw = u16::from_be_bytes([page[16], page[17]]);
		let page_size = match raw {
			1 => 65_536,
			size if size >= 512 && size.is_power_of_two() => u32::from(size),
			_ => return None,
		};
		let reserved = u32::from(page[20]);
		let usable_size = page_size.checked_sub(reserved)?;
		if usable_size < 480 {
			return None;
		}
		Some(Self {
			page_size,
			usable_size,
		})
	}
}

/// The free-list head and length from page 1's database header.
pub fn free_list_head(page1: &[u8]) -> Option<(Pgno, u32)> {
	if page1.len() < PAGE1_HEADER {
		return None;
	}
	Some((be32(page1, 32)?, be32(page1, 36)?))
}

/// The kind of B-tree page a type byte names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtreeKind {
	InteriorIndex,
	InteriorTable,
	LeafIndex,
	LeafTable,
}

impl BtreeKind {
	fn from_type(byte: u8) -> Option<Self> {
		match byte {
			0x02 => Some(Self::InteriorIndex),
			0x05 => Some(Self::InteriorTable),
			0x0a => Some(Self::LeafIndex),
			0x0d => Some(Self::LeafTable),
			_ => None,
		}
	}

	pub fn is_interior(self) -> bool {
		match self {
			Self::InteriorIndex | Self::InteriorTable => true,
			Self::LeafIndex | Self::LeafTable => false,
		}
	}

	fn header_len(self) -> usize {
		match self.is_interior() {
			true => 12,
			false => 8,
		}
	}
}

/// What a page's bytes say it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageClass {
	/// A B-tree page: its type byte and header are well formed.
	Btree(BtreeKind),
	/// Anything else: an overflow page, a free-list trunk or leaf, or garbage. The bytes alone
	/// cannot tell these apart.
	Other,
}

/// Classifies a page from its bytes. Page 1 carries the database header before its B-tree
/// header.
pub fn classify(pgno: Pgno, page: &[u8]) -> PageClass {
	match btree_header(pgno, page) {
		Some((kind, _)) => PageClass::Btree(kind),
		None => PageClass::Other,
	}
}

/// Whether `page`'s bytes prove it is a B-tree page rather than an overflow page in a database
/// of `size_pages` pages. An overflow page starts with its next page number, so its first byte
/// is the next pointer's high byte; below 2^24 pages that byte is zero and can never be a
/// B-tree type byte. Larger databases cannot rule out an overflow page whose next pointer
/// starts with a type byte, so nothing is proven there.
pub fn proves_btree(pgno: Pgno, page: &[u8], size_pages: u32) -> Option<BtreeKind> {
	if pgno != 1 && size_pages >= 1 << 24 {
		return None;
	}
	btree_header(pgno, page).map(|(kind, _)| kind)
}

fn btree_header(pgno: Pgno, page: &[u8]) -> Option<(BtreeKind, usize)> {
	let offset = match pgno {
		1 => PAGE1_HEADER,
		_ => 0,
	};
	let kind = BtreeKind::from_type(*page.get(offset)?)?;
	if page.len() < offset + kind.header_len() {
		return None;
	}
	let cells = usize::from(be16(page, offset + 3)?);
	let pointers_end = offset + kind.header_len() + cells * 2;
	if pointers_end > page.len() {
		return None;
	}
	Some((kind, offset))
}

/// The next pointer of an overflow page: its first four bytes. Zero ends the chain.
pub fn next(page: &[u8]) -> Option<Pgno> {
	be32(page, 0)
}

/// An interior page's child pointers in key order, the right-most pointer last.
pub fn children(pgno: Pgno, page: &[u8]) -> Option<Vec<Pgno>> {
	let (kind, offset) = btree_header(pgno, page)?;
	if !kind.is_interior() {
		return None;
	}
	let cells = usize::from(be16(page, offset + 3)?);
	let mut children = Vec::with_capacity(cells + 1);
	for index in 0..cells {
		let cell = usize::from(be16(page, offset + 12 + index * 2)?);
		let child = be32(page, cell)?;
		if child == 0 {
			return None;
		}
		children.push(child);
	}
	let right = be32(page, offset + 8)?;
	if right == 0 {
		return None;
	}
	children.push(right);
	Some(children)
}

/// An overflow chain a cell names: its first page and how many pages it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverflowRef {
	pub head: Pgno,
	pub pages: u32,
}

/// The overflow chains the cells of a B-tree page name, in cell order. Cells that are
/// malformed or point outside the page are skipped.
pub fn overflow_refs(pgno: Pgno, page: &[u8], geometry: Geometry) -> Vec<OverflowRef> {
	let Some((kind, offset)) = btree_header(pgno, page) else {
		return Vec::new();
	};
	if kind == BtreeKind::InteriorTable {
		return Vec::new();
	}
	let Some(cells) = be16(page, offset + 3) else {
		return Vec::new();
	};
	let usable = u64::from(geometry.usable_size);
	let mut refs = Vec::new();
	for index in 0..usize::from(cells) {
		let Some(cell) = be16(page, offset + kind.header_len() + index * 2) else {
			break;
		};
		if let Some(overflow) = cell_overflow(page, usize::from(cell), kind, usable) {
			refs.push(overflow);
		}
	}
	refs
}

/// The overflow chain one cell names, if its payload spills.
fn cell_overflow(page: &[u8], cell: usize, kind: BtreeKind, usable: u64) -> Option<OverflowRef> {
	let mut at = cell;
	if kind.is_interior() {
		at += 4;
	}
	let (payload, used) = varint(page.get(at..)?)?;
	at += used;
	if kind == BtreeKind::LeafTable {
		let (_, used) = varint(page.get(at..)?)?;
		at += used;
	}
	let local = local_payload(payload, kind, usable)?;
	if local >= payload {
		return None;
	}
	let pointer = at.checked_add(usize::try_from(local).ok()?)?;
	let head = be32(page, pointer)?;
	if head == 0 {
		return None;
	}
	let per_page = usable.checked_sub(4)?;
	let pages = (payload - local).div_ceil(per_page);
	Some(OverflowRef {
		head,
		pages: u32::try_from(pages).ok()?,
	})
}

/// How many payload bytes SQLite keeps on the B-tree page for a payload of `payload` bytes.
/// This is the file format's rule: up to X bytes stay local, and a spilling payload keeps M
/// bytes, or K when K fits, so the overflow pages hold whole multiples of their capacity.
fn local_payload(payload: u64, kind: BtreeKind, usable: u64) -> Option<u64> {
	let max_local = match kind {
		BtreeKind::LeafTable => usable.checked_sub(35)?,
		BtreeKind::InteriorIndex | BtreeKind::LeafIndex => {
			((usable.checked_sub(12)?) * 64 / 255).checked_sub(23)?
		}
		BtreeKind::InteriorTable => return None,
	};
	if payload <= max_local {
		return Some(payload);
	}
	let min_local = ((usable.checked_sub(12)?) * 32 / 255).checked_sub(23)?;
	let per_page = usable.checked_sub(4)?;
	let spill = min_local + (payload - min_local) % per_page;
	match spill <= max_local {
		true => Some(spill),
		false => Some(min_local),
	}
}

/// A free-list trunk page: the next trunk and the leaf pages it lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trunk {
	pub next: Pgno,
	pub leaves: Vec<Pgno>,
}

/// Parses a free-list trunk page. Only call it on a page the free list names as a trunk.
pub fn free_list(page: &[u8], geometry: Geometry) -> Option<Trunk> {
	let next = be32(page, 0)?;
	let count = usize::try_from(be32(page, 4)?).ok()?;
	let capacity = (geometry.usable_size as usize / 4).checked_sub(2)?;
	if count > capacity {
		return None;
	}
	let mut leaves = Vec::with_capacity(count);
	for index in 0..count {
		leaves.push(be32(page, 8 + index * 4)?);
	}
	Some(Trunk { next, leaves })
}

/// Reads a SQLite varint: big-endian groups of seven bits, at most nine bytes, the ninth
/// contributing all eight bits.
fn varint(bytes: &[u8]) -> Option<(u64, usize)> {
	let mut value = 0u64;
	for (index, byte) in bytes.iter().enumerate().take(9) {
		if index == 8 {
			value = (value << 8) | u64::from(*byte);
			return Some((value, 9));
		}
		value = (value << 7) | u64::from(byte & 0x7f);
		if byte & 0x80 == 0 {
			return Some((value, index + 1));
		}
	}
	None
}

fn be16(bytes: &[u8], at: usize) -> Option<u16> {
	let slice = bytes.get(at..at.checked_add(2)?)?;
	Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
	let slice = bytes.get(at..at.checked_add(4)?)?;
	Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
