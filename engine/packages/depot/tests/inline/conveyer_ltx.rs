use super::{
	DecodedLtx, EncodedLtx, LTX_HEADER_FLAG_NO_CHECKSUM, LTX_HEADER_SIZE, LTX_MAGIC,
	LTX_PAGE_HEADER_FLAG_SIZE, LTX_PAGE_HEADER_SIZE, LTX_RESERVED_HEADER_BYTES, LTX_TRAILER_SIZE,
	LTX_VERSION, LtxDecoder, LtxEncoder, LtxHeader, decode_ltx_v3, encode_ltx_v3,
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
		encoded.bytes[first_offset + LTX_PAGE_HEADER_SIZE..first_offset + LTX_PAGE_HEADER_SIZE + 4]
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
fn rejects_corrupt_trailer_or_index() {
	let encoded = LtxEncoder::new(sample_header())
		.encode_with_index(&[DirtyPage {
			pgno: 7,
			bytes: repeated_page(0x77),
		}])
		.expect("ltx should encode");

	let mut bad_trailer = encoded.bytes.clone();
	let trailer_idx = bad_trailer.len() - 1;
	bad_trailer[trailer_idx] = 0x01;
	assert!(decode_ltx_v3(&bad_trailer).is_err());

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
