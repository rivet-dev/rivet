/**
 * Key management for SQLite VFS storage
 *
 * This module contains constants and utilities for building keys used in the
 * key-value store for SQLite file storage.
 */

/**
 * Size of each file chunk stored in KV.
 *
 * SQLite calls the VFS with byte ranges, but KV stores whole values by key.
 * The VFS maps each byte range to one or more fixed-size chunks, then uses
 * chunk keys to read or write those values in KV.
 */
export const CHUNK_SIZE = 4096;

/** Top-level SQLite prefix (must match SQLITE_PREFIX in actor KV system) */
export const SQLITE_PREFIX = 9;

/** Key prefix byte for file metadata (after SQLITE_PREFIX) */
export const META_PREFIX = 0;

/** Key prefix byte for file chunks (after SQLITE_PREFIX) */
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
 * Format: [SQLITE_PREFIX (1 byte), META_PREFIX (1 byte), file tag (1 byte)]
 */
export function getMetaKey(fileTag: SqliteFileTag): Uint8Array {
	const key = new Uint8Array(3);
	key[0] = SQLITE_PREFIX;
	key[1] = META_PREFIX;
	key[2] = fileTag;
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
	const key = new Uint8Array(7);
	key[0] = SQLITE_PREFIX;
	key[1] = CHUNK_PREFIX;
	key[2] = fileTag;
	key[3] = (chunkIndex >>> 24) & 0xff;
	key[4] = (chunkIndex >>> 16) & 0xff;
	key[5] = (chunkIndex >>> 8) & 0xff;
	key[6] = chunkIndex & 0xff;
	return key;
}
