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

/**
 * Gets the key for file metadata
 * Format: [SQLITE_PREFIX (1 byte), META_PREFIX (1 byte), filename (UTF-8 encoded)]
 */
export function getMetaKey(fileName: string): Uint8Array {
	const encoder = new TextEncoder();
	const fileNameBytes = encoder.encode(fileName);
	const key = new Uint8Array(2 + fileNameBytes.length);
	key[0] = SQLITE_PREFIX;
	key[1] = META_PREFIX;
	key.set(fileNameBytes, 2);
	return key;
}

/**
 * Gets the key for a file chunk
 * Format: [SQLITE_PREFIX (1 byte), CHUNK_PREFIX (1 byte), filename (UTF-8), null separator (1 byte), chunk index (4 bytes, big-endian)]
 */
export function getChunkKey(fileName: string, chunkIndex: number): Uint8Array {
	const encoder = new TextEncoder();
	const fileNameBytes = encoder.encode(fileName);
	const key = new Uint8Array(2 + fileNameBytes.length + 1 + 4);
	key[0] = SQLITE_PREFIX;
	key[1] = CHUNK_PREFIX;
	key.set(fileNameBytes, 2);
	key[2 + fileNameBytes.length] = 0; // null separator
	// Encode chunk index as 32-bit unsigned integer (big-endian for proper ordering)
	const view = new DataView(key.buffer);
	view.setUint32(2 + fileNameBytes.length + 1, chunkIndex, false);
	return key;
}
