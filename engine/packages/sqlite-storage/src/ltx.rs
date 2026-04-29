//! LTX V3 encoding helpers for sqlite-storage blobs.

use anyhow::{Result, bail, ensure};

use crate::types::{DirtyPage, SQLITE_PAGE_SIZE};

pub const LTX_MAGIC: &[u8; 4] = b"LTX1";
pub const LTX_VERSION: u32 = 3;
pub const LTX_HEADER_SIZE: usize = 100;
pub const LTX_PAGE_HEADER_SIZE: usize = 6;
pub const LTX_TRAILER_SIZE: usize = 16;
pub const LTX_HEADER_FLAG_NO_CHECKSUM: u32 = 1 << 1;
pub const LTX_PAGE_HEADER_FLAG_SIZE: u16 = 1 << 0;
pub const LTX_RESERVED_HEADER_BYTES: usize = 28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LtxHeader {
	pub flags: u32,
	pub page_size: u32,
	pub commit: u32,
	pub min_txid: u64,
	pub max_txid: u64,
	pub timestamp_ms: i64,
	pub pre_apply_checksum: u64,
	pub wal_offset: i64,
	pub wal_size: i64,
	pub wal_salt1: u32,
	pub wal_salt2: u32,
	pub node_id: u64,
}

impl LtxHeader {
	pub fn delta(txid: u64, commit: u32, timestamp_ms: i64) -> Self {
		Self {
			flags: LTX_HEADER_FLAG_NO_CHECKSUM,
			page_size: SQLITE_PAGE_SIZE,
			commit,
			min_txid: txid,
			max_txid: txid,
			timestamp_ms,
			pre_apply_checksum: 0,
			wal_offset: 0,
			wal_size: 0,
			wal_salt1: 0,
			wal_salt2: 0,
			node_id: 0,
		}
	}

	pub fn encode(&self) -> Result<[u8; LTX_HEADER_SIZE]> {
		self.validate()?;

		let mut buf = [0u8; LTX_HEADER_SIZE];
		buf[0..4].copy_from_slice(LTX_MAGIC);
		buf[4..8].copy_from_slice(&self.flags.to_be_bytes());
		buf[8..12].copy_from_slice(&self.page_size.to_be_bytes());
		buf[12..16].copy_from_slice(&self.commit.to_be_bytes());
		buf[16..24].copy_from_slice(&self.min_txid.to_be_bytes());
		buf[24..32].copy_from_slice(&self.max_txid.to_be_bytes());
		buf[32..40].copy_from_slice(&self.timestamp_ms.to_be_bytes());
		buf[40..48].copy_from_slice(&self.pre_apply_checksum.to_be_bytes());
		buf[48..56].copy_from_slice(&self.wal_offset.to_be_bytes());
		buf[56..64].copy_from_slice(&self.wal_size.to_be_bytes());
		buf[64..68].copy_from_slice(&self.wal_salt1.to_be_bytes());
		buf[68..72].copy_from_slice(&self.wal_salt2.to_be_bytes());
		buf[72..80].copy_from_slice(&self.node_id.to_be_bytes());

		Ok(buf)
	}

	fn validate(&self) -> Result<()> {
		ensure!(
			self.flags & !LTX_HEADER_FLAG_NO_CHECKSUM == 0,
			"unsupported header flags: 0x{:08x}",
			self.flags
		);
		ensure!(
			self.page_size >= 512 && self.page_size <= 65_536 && self.page_size.is_power_of_two(),
			"invalid page size {}",
			self.page_size
		);
		ensure!(self.min_txid > 0, "min_txid must be greater than zero");
		ensure!(self.max_txid > 0, "max_txid must be greater than zero");
		ensure!(
			self.min_txid <= self.max_txid,
			"min_txid {} must be <= max_txid {}",
			self.min_txid,
			self.max_txid
		);
		ensure!(
			self.pre_apply_checksum == 0,
			"pre_apply_checksum must be zero"
		);
		ensure!(self.wal_offset >= 0, "wal_offset must be non-negative");
		ensure!(self.wal_size >= 0, "wal_size must be non-negative");
		ensure!(
			self.wal_offset != 0 || self.wal_size == 0,
			"wal_size requires wal_offset"
		);
		ensure!(
			self.wal_offset != 0 || (self.wal_salt1 == 0 && self.wal_salt2 == 0),
			"wal salts require wal_offset"
		);

		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LtxPageIndexEntry {
	pub pgno: u32,
	pub offset: u64,
	pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedLtx {
	pub bytes: Vec<u8>,
	pub page_index: Vec<LtxPageIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedLtx {
	pub header: LtxHeader,
	pub page_index: Vec<LtxPageIndexEntry>,
	pub pages: Vec<DirtyPage>,
}

impl DecodedLtx {
	pub fn get_page(&self, pgno: u32) -> Option<&[u8]> {
		self.pages
			.binary_search_by_key(&pgno, |page| page.pgno)
			.ok()
			.map(|idx| self.pages[idx].bytes.as_slice())
	}
}

#[derive(Debug, Clone)]
pub struct LtxEncoder {
	header: LtxHeader,
}

impl LtxEncoder {
	pub fn new(header: LtxHeader) -> Self {
		Self { header }
	}

	pub fn encode(&self, pages: &[DirtyPage]) -> Result<Vec<u8>> {
		Ok(self.encode_with_index(pages)?.bytes)
	}

	pub fn encode_with_index(&self, pages: &[DirtyPage]) -> Result<EncodedLtx> {
		let mut encoded = Vec::new();
		encoded.extend_from_slice(&self.header.encode()?);

		let mut sorted_pages = pages.to_vec();
		sorted_pages.sort_by_key(|page| page.pgno);

		let mut prev_pgno = 0u32;
		let mut page_index = Vec::with_capacity(sorted_pages.len());

		for page in &sorted_pages {
			ensure!(page.pgno > 0, "page number must be greater than zero");
			ensure!(
				page.pgno > prev_pgno,
				"page numbers must be unique and strictly increasing"
			);
			ensure!(
				page.bytes.len() == self.header.page_size as usize,
				"page {} had {} bytes, expected {}",
				page.pgno,
				page.bytes.len(),
				self.header.page_size
			);

			let offset = encoded.len() as u64;
			let compressed = lz4_flex::block::compress(&page.bytes);

			encoded.extend_from_slice(&page.pgno.to_be_bytes());
			encoded.extend_from_slice(&LTX_PAGE_HEADER_FLAG_SIZE.to_be_bytes());
			encoded.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
			encoded.extend_from_slice(&compressed);

			page_index.push(LtxPageIndexEntry {
				pgno: page.pgno,
				offset,
				size: encoded.len() as u64 - offset,
			});
			prev_pgno = page.pgno;
		}

		// A zero page header terminates the page section before the page index.
		encoded.extend_from_slice(&[0u8; LTX_PAGE_HEADER_SIZE]);

		let index_start = encoded.len();
		for entry in &page_index {
			append_uvarint(&mut encoded, entry.pgno as u64);
			append_uvarint(&mut encoded, entry.offset);
			append_uvarint(&mut encoded, entry.size);
		}
		append_uvarint(&mut encoded, 0);

		let index_size = (encoded.len() - index_start) as u64;
		encoded.extend_from_slice(&index_size.to_be_bytes());

		// We explicitly opt out of rolling checksums, so the trailer stays zeroed.
		encoded.extend_from_slice(&[0u8; LTX_TRAILER_SIZE]);

		Ok(EncodedLtx {
			bytes: encoded,
			page_index,
		})
	}
}

pub fn encode_ltx_v3(header: LtxHeader, pages: &[DirtyPage]) -> Result<Vec<u8>> {
	LtxEncoder::new(header).encode(pages)
}

#[derive(Debug, Clone)]
pub struct LtxDecoder<'a> {
	bytes: &'a [u8],
}

impl<'a> LtxDecoder<'a> {
	pub fn new(bytes: &'a [u8]) -> Self {
		Self { bytes }
	}

	pub fn decode(&self) -> Result<DecodedLtx> {
		self.decode_with_footer(self.bytes.len().saturating_sub(LTX_TRAILER_SIZE + 8))
			.or_else(|_| self.decode_with_footer(self.bytes.len().saturating_sub(8)))
	}

	fn decode_with_footer(&self, footer_start: usize) -> Result<DecodedLtx> {
		ensure!(
			self.bytes.len() >= LTX_HEADER_SIZE + LTX_PAGE_HEADER_SIZE + std::mem::size_of::<u64>(),
			"ltx blob too small: {} bytes",
			self.bytes.len()
		);

		let header = LtxHeader::decode(&self.bytes[..LTX_HEADER_SIZE])?;
		ensure!(
			footer_start + std::mem::size_of::<u64>() <= self.bytes.len(),
			"ltx footer starts outside blob"
		);

		let index_size = u64::from_be_bytes(
			self.bytes[footer_start..footer_start + std::mem::size_of::<u64>()]
				.try_into()
				.expect("ltx page index footer should be 8 bytes"),
		) as usize;
		let page_section_start = LTX_HEADER_SIZE;
		ensure!(
			footer_start >= page_section_start + LTX_PAGE_HEADER_SIZE,
			"ltx footer overlaps page section"
		);
		ensure!(
			index_size <= footer_start - page_section_start - LTX_PAGE_HEADER_SIZE,
			"ltx page index size {} exceeds available bytes",
			index_size
		);

		let index_start = footer_start - index_size;
		let page_section = &self.bytes[page_section_start..index_start];
		let page_index = decode_page_index(&self.bytes[index_start..footer_start])?;
		let (pages, computed_index) =
			decode_pages(page_section_start, page_section, header.page_size)?;

		ensure!(
			page_index == computed_index,
			"ltx page index did not match encoded page frames"
		);

		Ok(DecodedLtx {
			header,
			page_index,
			pages,
		})
	}
}

pub fn decode_ltx_v3(bytes: &[u8]) -> Result<DecodedLtx> {
	LtxDecoder::new(bytes).decode()
}

fn append_uvarint(buf: &mut Vec<u8>, mut value: u64) {
	while value >= 0x80 {
		buf.push((value as u8 & 0x7f) | 0x80);
		value >>= 7;
	}
	buf.push(value as u8);
}

fn decode_uvarint(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
	let mut shift = 0u32;
	let mut value = 0u64;

	loop {
		ensure!(*cursor < bytes.len(), "unexpected end of varint");
		let byte = bytes[*cursor];
		*cursor += 1;

		value |= u64::from(byte & 0x7f) << shift;
		if byte & 0x80 == 0 {
			return Ok(value);
		}

		shift += 7;
		ensure!(shift < 64, "varint exceeded 64 bits");
	}
}

fn decode_page_index(index_bytes: &[u8]) -> Result<Vec<LtxPageIndexEntry>> {
	let mut cursor = 0usize;
	let mut prev_pgno = 0u32;
	let mut page_index = Vec::new();

	loop {
		let pgno = decode_uvarint(index_bytes, &mut cursor)?;
		if pgno == 0 {
			break;
		}

		ensure!(
			pgno <= u64::from(u32::MAX),
			"page index pgno {} exceeded u32",
			pgno
		);
		let pgno = pgno as u32;
		ensure!(
			pgno > prev_pgno,
			"page index pgno {} was not strictly increasing",
			pgno
		);

		let offset = decode_uvarint(index_bytes, &mut cursor)?;
		let size = decode_uvarint(index_bytes, &mut cursor)?;
		page_index.push(LtxPageIndexEntry { pgno, offset, size });
		prev_pgno = pgno;
	}

	ensure!(cursor == index_bytes.len(), "page index had trailing bytes");

	Ok(page_index)
}

fn decode_pages(
	page_section_offset: usize,
	page_section: &[u8],
	page_size: u32,
) -> Result<(Vec<DirtyPage>, Vec<LtxPageIndexEntry>)> {
	let mut cursor = 0usize;
	let mut prev_pgno = 0u32;
	let mut pages = Vec::new();
	let mut page_index = Vec::new();

	while cursor < page_section.len() {
		let frame_offset = cursor;
		ensure!(
			page_section.len() - cursor >= LTX_PAGE_HEADER_SIZE,
			"page frame missing header"
		);

		let pgno = u32::from_be_bytes(
			page_section[cursor..cursor + 4]
				.try_into()
				.expect("page header pgno should decode"),
		);
		let flags = u16::from_be_bytes(
			page_section[cursor + 4..cursor + LTX_PAGE_HEADER_SIZE]
				.try_into()
				.expect("page header flags should decode"),
		);
		cursor += LTX_PAGE_HEADER_SIZE;

		if pgno == 0 {
			ensure!(flags == 0, "page-section sentinel must use zero flags");
			ensure!(
				cursor == page_section.len(),
				"page-section sentinel must terminate the page section"
			);
			return Ok((pages, page_index));
		}

		ensure!(
			flags == LTX_PAGE_HEADER_FLAG_SIZE,
			"unsupported page flags 0x{:04x} for page {}",
			flags,
			pgno
		);
		ensure!(
			pgno > prev_pgno,
			"page number {} was not strictly increasing",
			pgno
		);
		ensure!(
			page_section.len() - cursor >= std::mem::size_of::<u32>(),
			"page {} missing compressed size prefix",
			pgno
		);

		let compressed_size = u32::from_be_bytes(
			page_section[cursor..cursor + std::mem::size_of::<u32>()]
				.try_into()
				.expect("compressed size should decode"),
		) as usize;
		cursor += std::mem::size_of::<u32>();
		ensure!(
			page_section.len() - cursor >= compressed_size,
			"page {} compressed payload exceeded page section",
			pgno
		);

		let compressed = &page_section[cursor..cursor + compressed_size];
		cursor += compressed_size;
		let bytes = lz4_flex::block::decompress(compressed, page_size as usize)?;
		ensure!(
			bytes.len() == page_size as usize,
			"page {} decompressed to {} bytes, expected {}",
			pgno,
			bytes.len(),
			page_size
		);

		let size = (cursor - frame_offset) as u64;
		page_index.push(LtxPageIndexEntry {
			pgno,
			offset: (page_section_offset + frame_offset) as u64,
			size,
		});
		pages.push(DirtyPage { pgno, bytes });
		prev_pgno = pgno;
	}

	bail!("page section ended without a zero-page sentinel")
}

impl LtxHeader {
	pub fn decode(bytes: &[u8]) -> Result<Self> {
		ensure!(
			bytes.len() == LTX_HEADER_SIZE,
			"ltx header must be {} bytes, got {}",
			LTX_HEADER_SIZE,
			bytes.len()
		);
		ensure!(&bytes[0..4] == LTX_MAGIC, "invalid ltx magic");
		ensure!(
			bytes[LTX_HEADER_SIZE - LTX_RESERVED_HEADER_BYTES..LTX_HEADER_SIZE]
				.iter()
				.all(|byte| *byte == 0),
			"ltx reserved header bytes must be zero"
		);

		let header = Self {
			flags: u32::from_be_bytes(bytes[4..8].try_into().expect("flags should decode")),
			page_size: u32::from_be_bytes(
				bytes[8..12].try_into().expect("page size should decode"),
			),
			commit: u32::from_be_bytes(bytes[12..16].try_into().expect("commit should decode")),
			min_txid: u64::from_be_bytes(bytes[16..24].try_into().expect("min txid should decode")),
			max_txid: u64::from_be_bytes(bytes[24..32].try_into().expect("max txid should decode")),
			timestamp_ms: i64::from_be_bytes(
				bytes[32..40].try_into().expect("timestamp should decode"),
			),
			pre_apply_checksum: u64::from_be_bytes(
				bytes[40..48]
					.try_into()
					.expect("pre-apply checksum should decode"),
			),
			wal_offset: i64::from_be_bytes(
				bytes[48..56].try_into().expect("wal offset should decode"),
			),
			wal_size: i64::from_be_bytes(bytes[56..64].try_into().expect("wal size should decode")),
			wal_salt1: u32::from_be_bytes(
				bytes[64..68].try_into().expect("wal_salt1 should decode"),
			),
			wal_salt2: u32::from_be_bytes(
				bytes[68..72].try_into().expect("wal_salt2 should decode"),
			),
			node_id: u64::from_be_bytes(bytes[72..80].try_into().expect("node_id should decode")),
		};
		header.validate()?;

		Ok(header)
	}
}

#[cfg(test)]
mod tests {
	use super::{
		DecodedLtx, EncodedLtx, LTX_HEADER_FLAG_NO_CHECKSUM, LTX_HEADER_SIZE, LTX_MAGIC,
		LTX_PAGE_HEADER_FLAG_SIZE, LTX_PAGE_HEADER_SIZE, LTX_RESERVED_HEADER_BYTES,
		LTX_TRAILER_SIZE, LTX_VERSION, LtxDecoder, LtxEncoder, LtxHeader, decode_ltx_v3,
		encode_ltx_v3,
	};
	use crate::types::{DirtyPage, SQLITE_PAGE_SIZE};

	fn repeated_page(byte: u8) -> Vec<u8> {
		repeated_page_with_size(byte, SQLITE_PAGE_SIZE)
	}

	fn repeated_page_with_size(byte: u8, page_size: u32) -> Vec<u8> {
		vec![byte; page_size as usize]
	}

	fn sample_header() -> LtxHeader {
		LtxHeader::delta(7, 48, 1_713_456_789_000)
	}

	fn page_index_bytes(encoded: &EncodedLtx) -> &[u8] {
		let footer_offset = encoded.bytes.len() - LTX_TRAILER_SIZE - std::mem::size_of::<u64>();
		let index_size = u64::from_be_bytes(
			encoded.bytes[footer_offset..footer_offset + std::mem::size_of::<u64>()]
				.try_into()
				.expect("page index footer should decode"),
		) as usize;
		let index_start = footer_offset - index_size;

		&encoded.bytes[index_start..footer_offset]
	}

	#[test]
	fn delta_header_sets_v3_defaults() {
		let header = sample_header();

		assert_eq!(header.flags, LTX_HEADER_FLAG_NO_CHECKSUM);
		assert_eq!(header.page_size, SQLITE_PAGE_SIZE);
		assert_eq!(header.commit, 48);
		assert_eq!(header.min_txid, 7);
		assert_eq!(header.max_txid, 7);
		assert_eq!(header.pre_apply_checksum, 0);
		assert_eq!(header.wal_offset, 0);
		assert_eq!(header.wal_size, 0);
		assert_eq!(header.wal_salt1, 0);
		assert_eq!(header.wal_salt2, 0);
		assert_eq!(header.node_id, 0);
		assert_eq!(LTX_VERSION, 3);
	}

	#[test]
	fn encodes_header_and_zeroed_trailer() {
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[DirtyPage {
				pgno: 9,
				bytes: repeated_page(0x2a),
			}])
			.expect("ltx should encode");

		assert_eq!(&encoded.bytes[0..4], LTX_MAGIC);
		assert_eq!(
			u32::from_be_bytes(encoded.bytes[4..8].try_into().expect("flags")),
			LTX_HEADER_FLAG_NO_CHECKSUM
		);
		assert_eq!(
			u32::from_be_bytes(encoded.bytes[8..12].try_into().expect("page size")),
			SQLITE_PAGE_SIZE
		);
		assert_eq!(
			u32::from_be_bytes(encoded.bytes[12..16].try_into().expect("commit")),
			48
		);
		assert_eq!(
			u64::from_be_bytes(encoded.bytes[16..24].try_into().expect("min txid")),
			7
		);
		assert_eq!(
			u64::from_be_bytes(encoded.bytes[24..32].try_into().expect("max txid")),
			7
		);
		assert_eq!(
			&encoded.bytes[LTX_HEADER_SIZE - LTX_RESERVED_HEADER_BYTES..LTX_HEADER_SIZE],
			&[0u8; LTX_RESERVED_HEADER_BYTES]
		);
		assert_eq!(
			&encoded.bytes[encoded.bytes.len() - LTX_TRAILER_SIZE..],
			&[0u8; LTX_TRAILER_SIZE]
		);
	}

	#[test]
	fn encodes_page_headers_with_lz4_block_size_prefixes() {
		let first_page = repeated_page(0x11);
		let second_page = repeated_page(0x77);
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[
				DirtyPage {
					pgno: 4,
					bytes: first_page.clone(),
				},
				DirtyPage {
					pgno: 12,
					bytes: second_page.clone(),
				},
			])
			.expect("ltx should encode");

		let first_entry = &encoded.page_index[0];
		let second_entry = &encoded.page_index[1];
		let first_offset = first_entry.offset as usize;
		let second_offset = second_entry.offset as usize;

		assert_eq!(encoded.page_index.len(), 2);
		assert_eq!(
			u32::from_be_bytes(
				encoded.bytes[first_offset..first_offset + 4]
					.try_into()
					.expect("first pgno")
			),
			4
		);
		assert_eq!(
			u16::from_be_bytes(
				encoded.bytes[first_offset + 4..first_offset + LTX_PAGE_HEADER_SIZE]
					.try_into()
					.expect("first flags")
			),
			LTX_PAGE_HEADER_FLAG_SIZE
		);

		let compressed_size = u32::from_be_bytes(
			encoded.bytes
				[first_offset + LTX_PAGE_HEADER_SIZE..first_offset + LTX_PAGE_HEADER_SIZE + 4]
				.try_into()
				.expect("first compressed size"),
		) as usize;
		let compressed_bytes = &encoded.bytes[first_offset + LTX_PAGE_HEADER_SIZE + 4
			..first_offset + LTX_PAGE_HEADER_SIZE + 4 + compressed_size];
		let decoded = lz4_flex::block::decompress(compressed_bytes, SQLITE_PAGE_SIZE as usize)
			.expect("page should decompress");

		assert_eq!(decoded, first_page);
		assert_eq!(
			u32::from_be_bytes(
				encoded.bytes[second_offset..second_offset + 4]
					.try_into()
					.expect("second pgno")
			),
			12
		);
		assert_eq!(
			second_entry.offset,
			first_entry.offset + first_entry.size,
			"page frames should be tightly packed"
		);
		assert_eq!(second_page.len(), SQLITE_PAGE_SIZE as usize);
	}

	#[test]
	fn writes_sorted_page_index_with_zero_pgno_sentinel() {
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[
				DirtyPage {
					pgno: 33,
					bytes: repeated_page(0x33),
				},
				DirtyPage {
					pgno: 2,
					bytes: repeated_page(0x02),
				},
				DirtyPage {
					pgno: 17,
					bytes: repeated_page(0x17),
				},
			])
			.expect("ltx should encode");
		let index_bytes = page_index_bytes(&encoded);
		let mut cursor = 0usize;

		for expected in &encoded.page_index {
			assert_eq!(
				super::decode_uvarint(index_bytes, &mut cursor).expect("pgno"),
				expected.pgno as u64
			);
			assert_eq!(
				super::decode_uvarint(index_bytes, &mut cursor).expect("offset"),
				expected.offset
			);
			assert_eq!(
				super::decode_uvarint(index_bytes, &mut cursor).expect("size"),
				expected.size
			);
		}

		assert_eq!(
			encoded
				.page_index
				.iter()
				.map(|entry| entry.pgno)
				.collect::<Vec<_>>(),
			vec![2, 17, 33]
		);
		assert_eq!(
			super::decode_uvarint(index_bytes, &mut cursor).expect("sentinel"),
			0
		);
		assert_eq!(cursor, index_bytes.len());

		let sentinel_start = encoded.bytes.len()
			- LTX_TRAILER_SIZE
			- std::mem::size_of::<u64>()
			- index_bytes.len()
			- LTX_PAGE_HEADER_SIZE;
		assert_eq!(
			&encoded.bytes[sentinel_start..sentinel_start + LTX_PAGE_HEADER_SIZE],
			&[0u8; LTX_PAGE_HEADER_SIZE]
		);
	}

	#[test]
	fn rejects_invalid_pages() {
		let encoder = LtxEncoder::new(sample_header());

		let zero_pgno = encoder.encode(&[DirtyPage {
			pgno: 0,
			bytes: repeated_page(0x01),
		}]);
		assert!(zero_pgno.is_err());

		let wrong_size = encoder.encode(&[DirtyPage {
			pgno: 1,
			bytes: vec![0u8; 128],
		}]);
		assert!(wrong_size.is_err());
	}

	#[test]
	fn free_function_returns_complete_blob() {
		let bytes = encode_ltx_v3(
			sample_header(),
			&[DirtyPage {
				pgno: 5,
				bytes: repeated_page(0x55),
			}],
		)
		.expect("ltx should encode");

		assert!(bytes.len() > LTX_HEADER_SIZE + LTX_PAGE_HEADER_SIZE + LTX_TRAILER_SIZE);
	}

	fn decode_round_trip(encoded: &[u8]) -> DecodedLtx {
		LtxDecoder::new(encoded)
			.decode()
			.expect("ltx should decode")
	}

	#[test]
	fn decodes_round_trip_pages_and_header() {
		let header = sample_header();
		let pages = vec![
			DirtyPage {
				pgno: 8,
				bytes: repeated_page(0x08),
			},
			DirtyPage {
				pgno: 2,
				bytes: repeated_page(0x02),
			},
			DirtyPage {
				pgno: 44,
				bytes: repeated_page(0x44),
			},
		];
		let encoded = LtxEncoder::new(header.clone())
			.encode_with_index(&pages)
			.expect("ltx should encode");
		let decoded = decode_round_trip(&encoded.bytes);

		assert_eq!(decoded.header, header);
		assert_eq!(decoded.page_index, encoded.page_index);
		assert_eq!(
			decoded.pages,
			vec![
				DirtyPage {
					pgno: 2,
					bytes: repeated_page(0x02),
				},
				DirtyPage {
					pgno: 8,
					bytes: repeated_page(0x08),
				},
				DirtyPage {
					pgno: 44,
					bytes: repeated_page(0x44),
				},
			]
		);
		assert_eq!(decoded.get_page(8), Some(repeated_page(0x08).as_slice()));
		assert!(decoded.get_page(99).is_none());
	}

	#[test]
	fn decodes_varying_valid_page_sizes() {
		for page_size in [512u32, 1024, SQLITE_PAGE_SIZE] {
			let mut header = sample_header();
			header.page_size = page_size;
			header.commit = page_size;
			let page = DirtyPage {
				pgno: 3,
				bytes: repeated_page_with_size(0x5a, page_size),
			};
			let encoded = LtxEncoder::new(header.clone())
				.encode(&[page.clone()])
				.expect("ltx should encode");
			let decoded = decode_ltx_v3(&encoded).expect("ltx should decode");

			assert_eq!(decoded.header, header);
			assert_eq!(decoded.pages, vec![page]);
		}
	}

	#[test]
	fn decodes_legacy_blob_without_trailer() {
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[DirtyPage {
				pgno: 7,
				bytes: repeated_page(0x77),
			}])
			.expect("ltx should encode");
		let legacy_len = encoded.bytes.len() - LTX_TRAILER_SIZE;

		let decoded = decode_ltx_v3(&encoded.bytes[..legacy_len]).expect("ltx should decode");
		assert_eq!(decoded.page_index, encoded.page_index);
		assert_eq!(decoded.get_page(7), Some(repeated_page(0x77).as_slice()));
	}

	#[test]
	fn decodes_nonzero_trailer_bytes() {
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[DirtyPage {
				pgno: 7,
				bytes: repeated_page(0x77),
			}])
			.expect("ltx should encode");

		let mut checksum_trailer = encoded.bytes.clone();
		let trailer_idx = checksum_trailer.len() - 1;
		checksum_trailer[trailer_idx] = 0x01;

		let decoded = decode_ltx_v3(&checksum_trailer).expect("ltx should decode");
		assert_eq!(decoded.page_index, encoded.page_index);
		assert_eq!(decoded.get_page(7), Some(repeated_page(0x77).as_slice()));
	}

	#[test]
	fn rejects_corrupt_index() {
		let encoded = LtxEncoder::new(sample_header())
			.encode_with_index(&[DirtyPage {
				pgno: 7,
				bytes: repeated_page(0x77),
			}])
			.expect("ltx should encode");

		let mut bad_index = encoded.bytes.clone();
		let first_page_offset = encoded.page_index[0].offset as usize;
		let footer_offset = bad_index.len() - LTX_TRAILER_SIZE - std::mem::size_of::<u64>();
		let index_size = u64::from_be_bytes(
			bad_index[footer_offset..footer_offset + std::mem::size_of::<u64>()]
				.try_into()
				.expect("index footer should decode"),
		) as usize;
		let index_start = footer_offset - index_size;
		bad_index[index_start + 1] ^= 0x01;

		let decoded = decode_ltx_v3(&bad_index);
		assert!(decoded.is_err());
		assert_eq!(first_page_offset, encoded.page_index[0].offset as usize);
	}
}
