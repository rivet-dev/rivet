/**
 * Key management for SQLite VFS storage
 *
 * This module contains constants and utilities for building keys used in the
 * key-value store for SQLite file storage.
 */

/**
 * Size of each file chunk stored in KV.
 *
 * Set to 4096 to match SQLite's default page size so that one SQLite page
 * maps to exactly one KV value. This avoids partial-chunk reads on page
 * boundaries.
 *
 * Larger chunk sizes (e.g. 32 KiB) would reduce the number of KV keys per
 * database and fit within FDB's recommended 10 KB value chunks (the engine
 * splits values >10 KB internally, see VALUE_CHUNK_SIZE in
 * engine/packages/pegboard/src/actor_kv/mod.rs). However, 4 KiB is kept
 * because:
 *
 * - It matches SQLite's default page_size, avoiding alignment overhead.
 * - At 128 keys per batch and 4 KiB per chunk, a single putBatch can flush
 *   up to 512 KiB of dirty pages, which covers most actor databases.
 * - Changing chunk size is a breaking change for existing persisted databases.
 * - KV max value size is 128 KiB, so 4 KiB is well within limits.
 *
 * If page_size is ever changed via PRAGMA, CHUNK_SIZE must be updated to
 * match so the 1:1 page-to-chunk mapping is preserved.
 */
export const CHUNK_SIZE = 4096;

/** Top-level SQLite prefix (must match SQLITE_PREFIX in actor KV system) */
export const SQLITE_PREFIX = 8;

/** Schema version namespace byte after SQLITE_PREFIX */
export const SQLITE_SCHEMA_VERSION = 1;

/** Key prefix byte for file metadata (after SQLITE_PREFIX + version) */
export const META_PREFIX = 0;

/** Key prefix byte for file chunks (after SQLITE_PREFIX + version) */
export const CHUNK_PREFIX = 1;

/** File kind tag for the actor's main database file */
export const FILE_TAG_MAIN = 0;

/** File kind tag for the actor's rollback journal sidecar */
export const FILE_TAG_JOURNAL = 1;

/** File kind tag for the actor's WAL sidecar */
export const FILE_TAG_WAL = 2;

/** File kind tag for the actor's SHM sidecar */
export const FILE_TAG_SHM = 3;

export type SqliteFileTag =
	| typeof FILE_TAG_MAIN
	| typeof FILE_TAG_JOURNAL
	| typeof FILE_TAG_WAL
	| typeof FILE_TAG_SHM;

/**
 * Gets the key for file metadata
 * Format: [SQLITE_PREFIX (1 byte), version (1 byte), META_PREFIX (1 byte), file tag (1 byte)]
 */
export function getMetaKey(fileTag: SqliteFileTag): Uint8Array {
	const key = new Uint8Array(4);
	key[0] = SQLITE_PREFIX;
	key[1] = SQLITE_SCHEMA_VERSION;
	key[2] = META_PREFIX;
	key[3] = fileTag;
	return key;
}

/**
 * Gets the key for one chunk of file data.
 * Format: [SQLITE_PREFIX, CHUNK_PREFIX, file tag, chunk index (u32 big-endian)]
 *
 * The chunk index is derived from byte offset as floor(offset / CHUNK_SIZE),
 * which is how SQLite byte ranges map onto KV keys.
 */
export function getChunkKey(
	fileTag: SqliteFileTag,
	chunkIndex: number,
): Uint8Array {
	const key = new Uint8Array(8);
	key[0] = SQLITE_PREFIX;
	key[1] = SQLITE_SCHEMA_VERSION;
	key[2] = CHUNK_PREFIX;
	key[3] = fileTag;
	key[4] = (chunkIndex >>> 24) & 0xff;
	key[5] = (chunkIndex >>> 16) & 0xff;
	key[6] = (chunkIndex >>> 8) & 0xff;
	key[7] = chunkIndex & 0xff;
	return key;
}
