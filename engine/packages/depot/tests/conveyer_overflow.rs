mod common;

use anyhow::Result;
use depot::{
	keys::PAGE_SIZE,
	types::{DirtyPage, GetPagesOptions},
};

const USABLE: usize = PAGE_SIZE as usize;
const MAX_LOCAL_TABLE: usize = USABLE - 35;
const MIN_LOCAL: usize = ((USABLE - 12) * 32 / 255) - 23;

/// Number of payload bytes a leaf-table cell stores locally before spilling the
/// remainder onto overflow pages. Mirrors SQLite's payload-overflow formula.
fn local_payload_bytes(payload: usize) -> usize {
	assert!(payload > MAX_LOCAL_TABLE, "payload must overflow");
	let surplus = MIN_LOCAL + (payload - MIN_LOCAL) % (USABLE - 4);
	if surplus <= MAX_LOCAL_TABLE {
		surplus
	} else {
		MIN_LOCAL
	}
}

/// Append a SQLite varint (big-endian, up to 9 bytes) to `buf`.
fn push_varint(buf: &mut Vec<u8>, mut value: u64) {
	let mut bytes = Vec::new();
	bytes.push((value & 0x7f) as u8);
	value >>= 7;
	while value > 0 {
		bytes.push(((value & 0x7f) as u8) | 0x80);
		value >>= 7;
	}
	bytes.reverse();
	buf.extend_from_slice(&bytes);
}

/// Build a leaf-table page (page type 0x0d) holding one row whose payload spills
/// onto overflow pages, with its first overflow page pointer set to
/// `overflow_head`.
fn leaf_page_with_overflow(payload: usize, overflow_head: u32) -> Vec<u8> {
	let mut page = vec![0u8; USABLE];
	let cell_off = 1024usize;

	// B-tree leaf header.
	page[0] = 0x0d;
	page[3..5].copy_from_slice(&1u16.to_be_bytes());
	page[5..7].copy_from_slice(&(cell_off as u16).to_be_bytes());
	// Cell pointer array (one entry) immediately after the 8-byte header.
	page[8..10].copy_from_slice(&(cell_off as u16).to_be_bytes());

	// Cell: payload-length varint, rowid varint, local payload, overflow pointer.
	let local = local_payload_bytes(payload);
	let mut cell = Vec::new();
	push_varint(&mut cell, payload as u64);
	push_varint(&mut cell, 1);
	let header_len = cell.len();
	cell.extend(std::iter::repeat(0xab).take(local));
	cell.extend_from_slice(&overflow_head.to_be_bytes());
	page[cell_off..cell_off + cell.len()].copy_from_slice(&cell);

	// Sanity-check that the overflow pointer lands where the parser will read it.
	assert_eq!(cell_off + header_len + local, cell_off + cell.len() - 4);

	page
}

/// Build an overflow page whose leading 4-byte forward pointer references
/// `next` (0 terminates the chain).
fn overflow_page(next: u32) -> Vec<u8> {
	let mut page = vec![0xcd; USABLE];
	page[0..4].copy_from_slice(&next.to_be_bytes());
	page
}

fn dirty(pgno: u32, bytes: Vec<u8>) -> DirtyPage {
	DirtyPage { pgno, bytes }
}

fn requested(result: &[depot::types::FetchedPage]) -> Vec<u32> {
	let mut pgnos: Vec<u32> = result
		.iter()
		.filter(|page| page.bytes.is_some())
		.map(|page| page.pgno)
		.collect();
	pgnos.sort_unstable();
	pgnos
}

#[tokio::test]
async fn overflow_expansion_disabled_returns_only_requested_page() -> Result<()> {
	let ctx = common::build_test_db("depot-overflow-disabled", common::TierMode::Disabled).await?;

	// Page 2 is a leaf whose single row spills onto overflow page 3.
	let leaf = leaf_page_with_overflow(5_000, 3);
	ctx.db
		.commit(
			vec![
				dirty(1, vec![0; USABLE]),
				dirty(2, leaf),
				dirty(3, overflow_page(0)),
			],
			3,
			1_000,
		)
		.await?;

	let result = ctx
		.db
		.get_pages_with_options(
			vec![2],
			GetPagesOptions {
				expand_overflow: false,
				..Default::default()
			},
		)
		.await?;

	assert_eq!(
		requested(&result.pages),
		vec![2],
		"disabled expansion must return only the requested leaf page"
	);

	Ok(())
}

#[tokio::test]
async fn overflow_expansion_enabled_returns_overflow_page() -> Result<()> {
	let ctx = common::build_test_db("depot-overflow-enabled", common::TierMode::Disabled).await?;

	let leaf = leaf_page_with_overflow(5_000, 3);
	ctx.db
		.commit(
			vec![
				dirty(1, vec![0; USABLE]),
				dirty(2, leaf),
				dirty(3, overflow_page(0)),
			],
			3,
			1_000,
		)
		.await?;

	let result = ctx
		.db
		.get_pages_with_options(
			vec![2],
			GetPagesOptions {
				expand_overflow: true,
				..Default::default()
			},
		)
		.await?;

	assert_eq!(
		requested(&result.pages),
		vec![2, 3],
		"enabled expansion must return the leaf page plus its overflow page"
	);

	Ok(())
}

#[tokio::test]
async fn overflow_expansion_walks_multi_page_chain() -> Result<()> {
	let ctx = common::build_test_db("depot-overflow-chain", common::TierMode::Disabled).await?;

	// A payload that spills across two overflow pages (3 -> 4 -> end).
	let leaf = leaf_page_with_overflow(9_100, 3);
	ctx.db
		.commit(
			vec![
				dirty(1, vec![0; USABLE]),
				dirty(2, leaf),
				dirty(3, overflow_page(4)),
				dirty(4, overflow_page(0)),
			],
			4,
			1_000,
		)
		.await?;

	let result = ctx
		.db
		.get_pages_with_options(
			vec![2],
			GetPagesOptions {
				expand_overflow: true,
				..Default::default()
			},
		)
		.await?;

	assert_eq!(
		requested(&result.pages),
		vec![2, 3, 4],
		"expansion must walk the full overflow chain"
	);

	Ok(())
}
