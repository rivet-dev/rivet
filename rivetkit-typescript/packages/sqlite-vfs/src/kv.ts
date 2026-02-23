/**
 * Key management for SQLite VFS storage
 *
 * This module contains constants and utilities for building keys used in the
 * key-value store for SQLite file storage.
 */

/** Size of each chunk stored in KV (4KB) */
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
 * Gets the key for a file chunk
 * Format: [SQLITE_PREFIX (1 byte), CHUNK_PREFIX (1 byte), file tag (1 byte), chunk index (4 bytes, big-endian)]
 */
export function createChunkKeyFactory(
	fileTag: SqliteFileTag,
): (chunkIndex: number) => Uint8Array {
	const prefix = new Uint8Array(3);
	prefix[0] = SQLITE_PREFIX;
	prefix[1] = CHUNK_PREFIX;
	prefix[2] = fileTag;

	return (chunkIndex: number): Uint8Array => {
		const key = new Uint8Array(prefix.length + 4);
		key.set(prefix, 0);
		const offset = prefix.length;
		key[offset + 0] = (chunkIndex >>> 24) & 0xff;
		key[offset + 1] = (chunkIndex >>> 16) & 0xff;
		key[offset + 2] = (chunkIndex >>> 8) & 0xff;
		key[offset + 3] = chunkIndex & 0xff;
		return key;
	};
}

export function getChunkKey(
	fileTag: SqliteFileTag,
	chunkIndex: number,
): Uint8Array {
	return createChunkKeyFactory(fileTag)(chunkIndex);
}
