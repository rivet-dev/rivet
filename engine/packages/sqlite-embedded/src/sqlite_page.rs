//! Lightweight SQLite page classification for the VFS prefetch planner.
//!
//! A table scan over rows with overflowing payloads reads B-tree leaf pages and
//! overflow pages interleaved (for example `leaf 2, overflow 20, leaf 3,
//! overflow 21`). The two classes usually live in separate physical regions, so
//! a single stride/forward-scan tracker sees large alternating deltas and never
//! detects the per-class monotonic scan. Classifying each accessed page lets the
//! planner keep one tracker per class so the leaf scan can escalate read-ahead
//! independently of the overflow accesses.

/// Coarse page classification used to separate prefetch tracking streams. Any
/// page that is not a B-tree page (overflow, freelist, or pointer-map) falls
/// into [`PageClass::Overflow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageClass {
	Btree,
	Overflow,
}

/// Classify a page from its raw bytes using the one-byte B-tree page type at the
/// header offset (0x02 interior index, 0x05 interior table, 0x0a leaf index,
/// 0x0d leaf table). Page 1 carries the 100-byte database header before its
/// B-tree header. Any other type byte is treated as a non-B-tree page.
pub fn classify(pgno: u32, page: &[u8]) -> PageClass {
	let hdr = if pgno == 1 { 100 } else { 0 };
	match page.get(hdr) {
		Some(0x02 | 0x05 | 0x0a | 0x0d) => PageClass::Btree,
		_ => PageClass::Overflow,
	}
}
