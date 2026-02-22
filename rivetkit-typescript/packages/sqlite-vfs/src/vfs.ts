/**
 * SQLite raw database with KV storage backend
 *
 * This module provides a SQLite API that uses a KV-backed VFS
 * for storage. Each SqliteVfs instance is independent and can be
 * used concurrently with other instances.
 */

// Note: wa-sqlite VFS.Base type definitions have incorrect types for xRead/xWrite
// The actual runtime uses Uint8Array, not the {size, value} object shown in types
import * as VFS from "wa-sqlite/src/VFS.js";

import SQLiteESMFactory from "wa-sqlite/dist/wa-sqlite-async.mjs";
import { Factory } from "wa-sqlite";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { CHUNK_SIZE, getMetaKey, getChunkKey } from "./kv";
import {
	FILE_META_VERSIONED,
	CURRENT_VERSION,
} from "../schemas/file-meta/versioned";
import type { FileMeta } from "../schemas/file-meta/mod";
import type { KvVfsOptions } from "./types";

/**
 * Represents an open file
 */
interface OpenFile {
	/** File path */
	path: string;
	/** File size in bytes */
	size: number;
	/** Open flags */
	flags: number;
	/** KV options for this file */
	options: KvVfsOptions;
}

/**
 * Encodes file metadata to a Uint8Array using BARE schema
 */
function encodeFileMeta(size: number): Uint8Array {
	const meta: FileMeta = { size: BigInt(size) };
	return FILE_META_VERSIONED.serializeWithEmbeddedVersion(
		meta,
		CURRENT_VERSION,
	);
}

/**
 * Decodes file metadata from a Uint8Array using BARE schema
 */
function decodeFileMeta(data: Uint8Array): number {
	const meta = FILE_META_VERSIONED.deserializeWithEmbeddedVersion(data);
	return Number(meta.size);
}

/**
 * SQLite API interface (subset needed for VFS registration)
 * This is part of wa-sqlite but not exported in TypeScript types
 */
interface SQLite3Api {
	vfs_register: (vfs: unknown, makeDefault?: boolean) => number;
	open_v2: (
		filename: string,
		flags: number,
		vfsName?: string,
	) => Promise<number>;
	close: (db: number) => Promise<void>;
	exec: (
		db: number,
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	) => Promise<void>;
	run: (
		db: number,
		sql: string,
		params: unknown[] | null,
	) => Promise<number>;
	execWithParams: (
		db: number,
		sql: string,
		params: unknown[] | null,
	) => Promise<{ rows: unknown[][]; columns: string[] }>;
	SQLITE_OPEN_READWRITE: number;
	SQLITE_OPEN_CREATE: number;
}

/**
 * Simple async mutex for serializing database operations
 * wa-sqlite calls are not safe to run concurrently on one module instance
 */
class AsyncMutex {
	#locked = false;
	#waiting: (() => void)[] = [];

	async acquire(): Promise<void> {
		while (this.#locked) {
			await new Promise<void>((resolve) => this.#waiting.push(resolve));
		}
		this.#locked = true;
	}

	release(): void {
		this.#locked = false;
		const next = this.#waiting.shift();
		if (next) {
			next();
		}
	}

	async run<T>(fn: () => Promise<T>): Promise<T> {
		await this.acquire();
		try {
			return await fn();
		} finally {
			this.release();
		}
	}
}

/**
 * Database wrapper that provides a simplified SQLite API
 */
export class Database {
	readonly #sqlite3: SQLite3Api;
	readonly #handle: number;
	readonly #fileName: string;
	readonly #onClose: () => void;
	readonly #sqliteMutex: AsyncMutex;

	constructor(
		sqlite3: SQLite3Api,
		handle: number,
		fileName: string,
		onClose: () => void,
		sqliteMutex: AsyncMutex,
	) {
		this.#sqlite3 = sqlite3;
		this.#handle = handle;
		this.#fileName = fileName;
		this.#onClose = onClose;
		this.#sqliteMutex = sqliteMutex;
	}

	/**
	 * Execute SQL with optional row callback
	 * @param sql - SQL statement to execute
	 * @param callback - Called for each result row with (row, columns)
	 */
	async exec(sql: string, callback?: (row: unknown[], columns: string[]) => void): Promise<void> {
		return this.#sqliteMutex.run(async () =>
			this.#sqlite3.exec(this.#handle, sql, callback),
		);
	}

	/**
	 * Execute a parameterized SQL statement (no result rows)
	 * @param sql - SQL statement with ? placeholders
	 * @param params - Parameter values to bind
	 */
	async run(sql: string, params?: unknown[]): Promise<void> {
		await this.#sqliteMutex.run(async () =>
			this.#sqlite3.run(this.#handle, sql, params ?? null),
		);
	}

	/**
	 * Execute a parameterized SQL query and return results
	 * @param sql - SQL query with ? placeholders
	 * @param params - Parameter values to bind
	 * @returns Object with rows (array of arrays) and columns (column names)
	 */
	async query(sql: string, params?: unknown[]): Promise<{ rows: unknown[][]; columns: string[] }> {
		return this.#sqliteMutex.run(async () =>
			this.#sqlite3.execWithParams(this.#handle, sql, params ?? null),
		);
	}

	/**
	 * Close the database
	 */
	async close(): Promise<void> {
		await this.#sqliteMutex.run(async () => {
			await this.#sqlite3.close(this.#handle);
		});
		this.#onClose();
	}

	/**
	 * Get the raw wa-sqlite API (for advanced usage)
	 */
	get sqlite3(): SQLite3Api {
		return this.#sqlite3;
	}

	/**
	 * Get the raw database handle (for advanced usage)
	 */
	get handle(): number {
		return this.#handle;
	}
}

/**
 * SQLite VFS backed by KV storage.
 *
 * Each instance is independent and has its own wa-sqlite WASM module.
 * This allows multiple instances to operate concurrently without interference.
 */
export class SqliteVfs {
	#sqlite3: SQLite3Api | null = null;
	#sqliteSystem: SqliteSystem | null = null;
	#initPromise: Promise<void> | null = null;
	#openMutex = new AsyncMutex();
	#sqliteMutex = new AsyncMutex();
	#instanceId: string;

	constructor() {
		// Generate unique instance ID for VFS name
		this.#instanceId = crypto.randomUUID().replace(/-/g, '').slice(0, 8);
	}

	/**
	 * Initialize wa-sqlite and VFS (called once per instance)
	 */
	async #ensureInitialized(): Promise<void> {
		// Fast path: already initialized
		if (this.#sqlite3 && this.#sqliteSystem) {
			return;
		}

		// Synchronously create the promise if not started
		if (!this.#initPromise) {
				this.#initPromise = (async () => {
				// Load WASM binary (Node.js environment)
				const require = createRequire(import.meta.url);
				const wasmPath = require.resolve("wa-sqlite/dist/wa-sqlite-async.wasm");
				const wasmBinary = readFileSync(wasmPath);

					// Initialize wa-sqlite module - each instance gets its own module
					const module = await SQLiteESMFactory({ wasmBinary });
					this.#sqlite3 = Factory(module) as unknown as SQLite3Api;

				// Create and register VFS with unique name
				this.#sqliteSystem = new SqliteSystem(this.#sqlite3, `kv-vfs-${this.#instanceId}`);
				this.#sqliteSystem.register();
			})();
		}

		// Wait for initialization
		await this.#initPromise;
	}

	/**
	 * Open a SQLite database using KV storage backend
	 *
	 * @param fileName - The database file name (typically the actor ID)
	 * @param options - KV storage operations for this database
	 * @returns A Database instance
	 */
	async open(
		fileName: string,
		options: KvVfsOptions,
	): Promise<Database> {
		// Serialize all open operations within this instance
		await this.#openMutex.acquire();
		try {
			// Initialize wa-sqlite and SqliteSystem on first call
			await this.#ensureInitialized();

			if (!this.#sqlite3 || !this.#sqliteSystem) {
				throw new Error("Failed to initialize SQLite");
			}

				// Register this filename with its KV options
				this.#sqliteSystem.registerFile(fileName, options);

				// Open database
				const db = await this.#sqliteMutex.run(async () =>
					this.#sqlite3!.open_v2(
						fileName,
						this.#sqlite3!.SQLITE_OPEN_READWRITE |
							this.#sqlite3!.SQLITE_OPEN_CREATE,
						this.#sqliteSystem!.name,
					),
				);

			// Create cleanup callback
			const sqliteSystem = this.#sqliteSystem;
			const onClose = () => {
				sqliteSystem.unregisterFile(fileName);
			};

				return new Database(
					this.#sqlite3,
					db,
					fileName,
					onClose,
					this.#sqliteMutex,
				);
			} finally {
				this.#openMutex.release();
			}
	}
}

/**
 * Internal VFS implementation
 */
class SqliteSystem extends VFS.Base {
	readonly name: string;
	readonly #fileOptions: Map<string, KvVfsOptions> = new Map();
	readonly #openFiles: Map<number, OpenFile> = new Map();
	readonly #sqlite3: SQLite3Api;

	constructor(sqlite3: SQLite3Api, name: string) {
		super();
		this.#sqlite3 = sqlite3;
		this.name = name;
	}

	/**
	 * Registers the VFS with SQLite
	 */
	register(): void {
		this.#sqlite3.vfs_register(this, false);
	}

	/**
	 * Registers a file with its KV options (before opening)
	 */
	registerFile(fileName: string, options: KvVfsOptions): void {
		this.#fileOptions.set(fileName, options);
	}

	/**
	 * Unregisters a file's KV options (after closing)
	 */
	unregisterFile(fileName: string): void {
		this.#fileOptions.delete(fileName);
	}

	/**
	 * Gets KV options for a file, handling journal/wal files by using the main database's options
	 */
	#getOptionsForPath(path: string): KvVfsOptions | undefined {
		let options = this.#fileOptions.get(path);
		if (!options) {
			// Try to find the main database file by removing common SQLite suffixes
			const mainDbPath = path
				.replace(/-journal$/, "")
				.replace(/-wal$/, "")
				.replace(/-shm$/, "");

			if (mainDbPath !== path) {
				options = this.#fileOptions.get(mainDbPath);
			}
		}
		return options;
	}

	/**
	 * Opens a file
	 */
	xOpen(
		path: string | null,
		fileId: number,
		flags: number,
		pOutFlags: DataView,
	): number {
		return this.handleAsync(async () => {
			if (!path) {
				return VFS.SQLITE_CANTOPEN;
			}

			// Get the registered KV options for this file
			// For journal/wal files, use the main database's options
			const options = this.#getOptionsForPath(path);
			if (!options) {
				throw new Error(`No KV options registered for file: ${path}`);
			}

			// Get existing file size if the file exists
			const metaKey = getMetaKey(path);
			const sizeData = await options.get(metaKey);

			let size: number;

			if (sizeData) {
				// File exists, use existing size
				size = decodeFileMeta(sizeData);
			} else if (flags & VFS.SQLITE_OPEN_CREATE) {
				// File doesn't exist, create it
				size = 0;
				await options.put(metaKey, encodeFileMeta(size));
			} else {
				// File doesn't exist and we're not creating it
				return VFS.SQLITE_CANTOPEN;
			}

			// Store open file info with options
			this.#openFiles.set(fileId, {
				path,
				size,
				flags,
				options,
			});

			// Set output flags
			pOutFlags.setInt32(0, flags & VFS.SQLITE_OPEN_READONLY ? 1 : 0, true);

			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Closes a file
	 */
	xClose(fileId: number): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_OK;
			}

			// Delete file if SQLITE_OPEN_DELETEONCLOSE flag was set
			if (file.flags & VFS.SQLITE_OPEN_DELETEONCLOSE) {
				await this.#delete(file.path);
			}

			this.#openFiles.delete(fileId);
			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Reads data from a file
	 */
	// @ts-expect-error - VFS.Base types are incorrect, runtime uses Uint8Array
	xRead(fileId: number, pData: Uint8Array, iOffset: number): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR_READ;
			}

			const options = file.options;
			const requestedLength = pData.length;
			const fileSize = file.size;

			// If offset is beyond file size, return short read with zeroed buffer
			if (iOffset >= fileSize) {
				pData.fill(0);
				return VFS.SQLITE_IOERR_SHORT_READ;
			}

			// Calculate which chunks we need to read
			const startChunk = Math.floor(iOffset / CHUNK_SIZE);
			const endChunk = Math.floor((iOffset + requestedLength - 1) / CHUNK_SIZE);

			// Fetch all needed chunks
			const chunkKeys: Uint8Array[] = [];
			for (let i = startChunk; i <= endChunk; i++) {
				chunkKeys.push(getChunkKey(file.path, i));
			}

			const chunks = await options.getBatch(chunkKeys);

			// Copy data from chunks to output buffer
			for (let i = startChunk; i <= endChunk; i++) {
				const chunkData = chunks[i - startChunk];
				const chunkOffset = i * CHUNK_SIZE;

				// Calculate the range within this chunk
				const readStart = Math.max(0, iOffset - chunkOffset);
				const readEnd = Math.min(
					CHUNK_SIZE,
					iOffset + requestedLength - chunkOffset,
				);

				if (chunkData) {
					// Copy available data
					const sourceStart = readStart;
					const sourceEnd = Math.min(readEnd, chunkData.length);
					const destStart = chunkOffset + readStart - iOffset;

					if (sourceEnd > sourceStart) {
						pData.set(
							chunkData.slice(sourceStart, sourceEnd),
							destStart,
						);
					}

					// Zero-fill if chunk is smaller than expected
					if (sourceEnd < readEnd) {
						const zeroStart = destStart + (sourceEnd - sourceStart);
						const zeroEnd = destStart + (readEnd - readStart);
						pData.fill(0, zeroStart, zeroEnd);
					}
				} else {
					// Chunk doesn't exist, zero-fill
					const destStart = chunkOffset + readStart - iOffset;
					const destEnd = destStart + (readEnd - readStart);
					pData.fill(0, destStart, destEnd);
				}
			}

			// If we read less than requested (past EOF), return short read
			const actualBytes = Math.min(requestedLength, fileSize - iOffset);
			if (actualBytes < requestedLength) {
				pData.fill(0, actualBytes);
				return VFS.SQLITE_IOERR_SHORT_READ;
			}

			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Writes data to a file
	 */
	// @ts-expect-error - VFS.Base types are incorrect, runtime uses Uint8Array
	xWrite(fileId: number, pData: Uint8Array, iOffset: number): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR_WRITE;
			}

			const options = file.options;
			const writeLength = pData.length;

			// Calculate which chunks we need to modify
			const startChunk = Math.floor(iOffset / CHUNK_SIZE);
			const endChunk = Math.floor((iOffset + writeLength - 1) / CHUNK_SIZE);

			// Fetch existing chunks that we'll need to modify
			const chunkKeys: Uint8Array[] = [];
			for (let i = startChunk; i <= endChunk; i++) {
				chunkKeys.push(getChunkKey(file.path, i));
			}

			const existingChunks = await options.getBatch(chunkKeys);

			// Prepare new chunk data
			const entriesToWrite: [Uint8Array, Uint8Array][] = [];

			for (let i = startChunk; i <= endChunk; i++) {
				const chunkOffset = i * CHUNK_SIZE;
				const existingChunk = existingChunks[i - startChunk];

				// Calculate the range within this chunk that we're writing
				const writeStart = Math.max(0, iOffset - chunkOffset);
				const writeEnd = Math.min(
					CHUNK_SIZE,
					iOffset + writeLength - chunkOffset,
				);

				// Calculate the size this chunk needs to be
				const requiredSize = writeEnd;

				// Create new chunk data
				let newChunk: Uint8Array;
				if (existingChunk && existingChunk.length >= requiredSize) {
					// Use existing chunk (copy it so we can modify)
					newChunk = new Uint8Array(Math.max(existingChunk.length, requiredSize));
					newChunk.set(existingChunk);
				} else if (existingChunk) {
					// Need to expand existing chunk
					newChunk = new Uint8Array(requiredSize);
					newChunk.set(existingChunk);
				} else {
					// Create new chunk
					newChunk = new Uint8Array(requiredSize);
				}

				// Copy data from input buffer to chunk
				const sourceStart = chunkOffset + writeStart - iOffset;
				const sourceEnd = sourceStart + (writeEnd - writeStart);
				newChunk.set(pData.slice(sourceStart, sourceEnd), writeStart);

				entriesToWrite.push([getChunkKey(file.path, i), newChunk]);
			}

			// Update file size if we wrote past the end
			const newSize = Math.max(file.size, iOffset + writeLength);
			if (newSize !== file.size) {
				file.size = newSize;
				entriesToWrite.push([getMetaKey(file.path), encodeFileMeta(file.size)]);
			}

			// Write all chunks and metadata
			await options.putBatch(entriesToWrite);

			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Truncates a file
	 */
	xTruncate(fileId: number, size: number): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR_TRUNCATE;
			}

			const options = file.options;

			// If truncating to larger size, just update metadata
			if (size >= file.size) {
				return VFS.SQLITE_OK;
			}

			// Calculate which chunks to delete
			// Note: When size=0, lastChunkToKeep = floor(-1/4096) = -1, which means
			// all chunks (starting from index 0) will be deleted in the loop below.
			const lastChunkToKeep = Math.floor((size - 1) / CHUNK_SIZE);
			const lastExistingChunk = Math.floor((file.size - 1) / CHUNK_SIZE);

			// Delete chunks beyond the new size
			const keysToDelete: Uint8Array[] = [];
			for (let i = lastChunkToKeep + 1; i <= lastExistingChunk; i++) {
				keysToDelete.push(getChunkKey(file.path, i));
			}

			if (keysToDelete.length > 0) {
				await options.deleteBatch(keysToDelete);
			}

			// Truncate the last kept chunk if needed
			if (size > 0 && size % CHUNK_SIZE !== 0) {
				const lastChunkKey = getChunkKey(file.path, lastChunkToKeep);
				const lastChunkData = await options.get(lastChunkKey);

				if (lastChunkData && lastChunkData.length > size % CHUNK_SIZE) {
					const truncatedChunk = lastChunkData.slice(0, size % CHUNK_SIZE);
					await options.put(lastChunkKey, truncatedChunk);
				}
			}

			// Update file size
			file.size = size;
			await options.put(getMetaKey(file.path), encodeFileMeta(file.size));

			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Syncs file data to storage
	 */
	xSync(fileId: number, _flags: number): number {
		return this.handleAsync(async () => {
			// KV storage is immediately durable, so sync is a no-op
			// But we should ensure size is persisted
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_OK;
			}

			const options = file.options;
			await options.put(getMetaKey(file.path), encodeFileMeta(file.size));
			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Gets the file size
	 */
	xFileSize(fileId: number, pSize: DataView): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR_FSTAT;
			}

			// Set size as 64-bit integer (low and high parts)
			pSize.setBigInt64(0, BigInt(file.size), true);
			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Deletes a file
	 */
	xDelete(path: string, _syncDir: number): number {
		return this.handleAsync(async () => {
			await this.#delete(path);
			return VFS.SQLITE_OK;
		});
	}

	/**
	 * Internal delete implementation
	 */
	async #delete(path: string): Promise<void> {
		const options = this.#getOptionsForPath(path);
		if (!options) {
			throw new Error(`No KV options registered for file: ${path}`);
		}

		// Get file size to find out how many chunks to delete
		const metaKey = getMetaKey(path);
		const sizeData = await options.get(metaKey);

		if (!sizeData) {
			// File doesn't exist, that's OK
			return;
		}

		const size = decodeFileMeta(sizeData);

		// Delete all chunks
		const keysToDelete: Uint8Array[] = [metaKey];
		const numChunks = Math.ceil(size / CHUNK_SIZE);
		for (let i = 0; i < numChunks; i++) {
			keysToDelete.push(getChunkKey(path, i));
		}

		await options.deleteBatch(keysToDelete);
	}

	/**
	 * Checks file accessibility
	 */
	xAccess(path: string, _flags: number, pResOut: DataView): number {
		return this.handleAsync(async () => {
			const options = this.#getOptionsForPath(path);
			if (!options) {
				// File not registered, doesn't exist
				pResOut.setInt32(0, 0, true);
				return VFS.SQLITE_OK;
			}

			const metaKey = getMetaKey(path);
			const metaData = await options.get(metaKey);

			// Set result: 1 if file exists, 0 otherwise
			pResOut.setInt32(0, metaData ? 1 : 0, true);
			return VFS.SQLITE_OK;
		});
	}
}
