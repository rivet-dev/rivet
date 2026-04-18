//! KV key layout for SQLite-over-KV storage.
//!
//! Key layout:
//!   Meta key:  [SQLITE_PREFIX, SCHEMA_VERSION, META_PREFIX, file_tag]       (4 bytes)
//!   Chunk key: [SQLITE_PREFIX, SCHEMA_VERSION, CHUNK_PREFIX, file_tag, chunk_index_u32_be] (8 bytes)

/// Size of each file chunk stored in KV.
pub const CHUNK_SIZE: usize = 4096;

/// Top-level SQLite prefix byte.
pub const SQLITE_PREFIX: u8 = 0x08;

/// Schema version namespace byte after SQLITE_PREFIX.
pub const SQLITE_SCHEMA_VERSION: u8 = 0x01;

/// Key prefix byte for file metadata (after SQLITE_PREFIX + version).
pub const META_PREFIX: u8 = 0x00;

/// Key prefix byte for file chunks (after SQLITE_PREFIX + version).
pub const CHUNK_PREFIX: u8 = 0x01;

/// File kind tag for the actor's main database file.
pub const FILE_TAG_MAIN: u8 = 0x00;

/// File kind tag for the actor's rollback journal sidecar.
pub const FILE_TAG_JOURNAL: u8 = 0x01;

/// File kind tag for the actor's WAL sidecar.
pub const FILE_TAG_WAL: u8 = 0x02;

/// File kind tag for the actor's SHM sidecar.
pub const FILE_TAG_SHM: u8 = 0x03;

/// Returns the 4-byte metadata key for the given file tag.
///
/// Format: `[SQLITE_PREFIX, SCHEMA_VERSION, META_PREFIX, file_tag]`
pub fn get_meta_key(file_tag: u8) -> [u8; 4] {
	[SQLITE_PREFIX, SQLITE_SCHEMA_VERSION, META_PREFIX, file_tag]
}

/// Returns the 8-byte chunk key for the given file tag and chunk index.
///
/// Format: `[SQLITE_PREFIX, SCHEMA_VERSION, CHUNK_PREFIX, file_tag, chunk_index_u32_be]`
///
/// The chunk index is derived from byte offset as `offset / CHUNK_SIZE`.
pub fn get_chunk_key(file_tag: u8, chunk_index: u32) -> [u8; 8] {
	let ci = chunk_index.to_be_bytes();
	[
		SQLITE_PREFIX,
		SQLITE_SCHEMA_VERSION,
		CHUNK_PREFIX,
		file_tag,
		ci[0],
		ci[1],
		ci[2],
		ci[3],
	]
}

/// Maximum file size in bytes before chunk index overflow.
///
/// Chunk indices are u32, so the maximum addressable byte is
/// (u32::MAX as u64 + 1) * CHUNK_SIZE. Writes or truncates beyond this would
/// wrap the chunk index.
pub const MAX_FILE_SIZE: u64 = (u32::MAX as u64 + 1) * CHUNK_SIZE as u64;

/// Returns a 4-byte key that is lexicographically just past all chunk keys for
/// the given file tag. Useful as the exclusive end bound for deleteRange.
///
/// Format: `[SQLITE_PREFIX, SCHEMA_VERSION, CHUNK_PREFIX, file_tag + 1]`
///
/// This is shorter than a chunk key but lexicographically greater than any
/// 8-byte chunk key with the same file_tag prefix.
pub fn get_chunk_key_range_end(file_tag: u8) -> [u8; 4] {
	[
		SQLITE_PREFIX,
		SQLITE_SCHEMA_VERSION,
		CHUNK_PREFIX,
		file_tag + 1,
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn constants_match_expected_layout() {
		assert_eq!(CHUNK_SIZE, 4096);
		assert_eq!(SQLITE_PREFIX, 8);
		assert_eq!(SQLITE_SCHEMA_VERSION, 1);
		assert_eq!(META_PREFIX, 0);
		assert_eq!(CHUNK_PREFIX, 1);
		assert_eq!(FILE_TAG_MAIN, 0);
		assert_eq!(FILE_TAG_JOURNAL, 1);
		assert_eq!(FILE_TAG_WAL, 2);
		assert_eq!(FILE_TAG_SHM, 3);
	}

	#[test]
	fn meta_key_main() {
		assert_eq!(get_meta_key(FILE_TAG_MAIN), [0x08, 0x01, 0x00, 0x00]);
	}

	#[test]
	fn meta_key_journal() {
		assert_eq!(get_meta_key(FILE_TAG_JOURNAL), [0x08, 0x01, 0x00, 0x01]);
	}

	#[test]
	fn meta_key_wal() {
		assert_eq!(get_meta_key(FILE_TAG_WAL), [0x08, 0x01, 0x00, 0x02]);
	}

	#[test]
	fn meta_key_shm() {
		assert_eq!(get_meta_key(FILE_TAG_SHM), [0x08, 0x01, 0x00, 0x03]);
	}

	#[test]
	fn chunk_key_zero_index() {
		assert_eq!(
			get_chunk_key(FILE_TAG_MAIN, 0),
			[0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]
		);
	}

	#[test]
	fn chunk_key_index_one() {
		assert_eq!(
			get_chunk_key(FILE_TAG_MAIN, 1),
			[0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01]
		);
	}

	#[test]
	fn chunk_key_large_index() {
		// TypeScript: getChunkKey(FILE_TAG_MAIN, 256) => [8, 1, 1, 0, 0, 0, 1, 0]
		assert_eq!(
			get_chunk_key(FILE_TAG_MAIN, 256),
			[0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00]
		);
	}

	#[test]
	fn chunk_key_max_index() {
		// TypeScript: getChunkKey(FILE_TAG_MAIN, 0xFFFFFFFF) => [8, 1, 1, 0, 255, 255, 255, 255]
		assert_eq!(
			get_chunk_key(FILE_TAG_MAIN, u32::MAX),
			[0x08, 0x01, 0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]
		);
	}

	#[test]
	fn chunk_key_journal_tag() {
		assert_eq!(
			get_chunk_key(FILE_TAG_JOURNAL, 42),
			[0x08, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 42]
		);
	}

	#[test]
	fn chunk_key_big_endian_encoding() {
		// 0x01020304 => bytes [1, 2, 3, 4]
		assert_eq!(
			get_chunk_key(FILE_TAG_MAIN, 0x01020304),
			[0x08, 0x01, 0x01, 0x00, 0x01, 0x02, 0x03, 0x04]
		);
	}

	#[test]
	fn chunk_key_range_end_main() {
		// TypeScript: getChunkKeyRangeEnd(FILE_TAG_MAIN) => [8, 1, 1, 1]
		assert_eq!(
			get_chunk_key_range_end(FILE_TAG_MAIN),
			[0x08, 0x01, 0x01, 0x01]
		);
	}

	#[test]
	fn chunk_key_range_end_journal() {
		// TypeScript: getChunkKeyRangeEnd(FILE_TAG_JOURNAL) => [8, 1, 1, 2]
		assert_eq!(
			get_chunk_key_range_end(FILE_TAG_JOURNAL),
			[0x08, 0x01, 0x01, 0x02]
		);
	}

	#[test]
	fn range_end_is_past_all_chunk_keys() {
		// The range end key must be lexicographically greater than any chunk key for the same tag.
		let max_chunk = get_chunk_key(FILE_TAG_MAIN, u32::MAX);
		let range_end = get_chunk_key_range_end(FILE_TAG_MAIN);
		// Compare as slices. The range end [8,1,1,1] > [8,1,1,0,FF,FF,FF,FF]
		// because at byte index 3, 1 > 0.
		assert!(range_end.as_slice() > max_chunk.as_slice());
	}
}
