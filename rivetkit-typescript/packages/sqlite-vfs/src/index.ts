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

// VFS debug logging - set VFS_DEBUG=1 to enable
const VFS_DEBUG = process.env.VFS_DEBUG === "1";
function vfsLog(op: string, details: Record<string, unknown>) {
	if (VFS_DEBUG) {
		console.log(`[VFS] ${op}`, JSON.stringify(details));
	}
}
import SQLiteESMFactory from "wa-sqlite/dist/wa-sqlite-async.mjs";
import { Factory } from "wa-sqlite";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { CHUNK_SIZE, getMetaKey, getChunkKey } from "./kv";
import {
	FILE_META_VERSIONED,
	CURRENT_VERSION,
} from "../schemas/file-meta/versioned.js";
import type { FileMeta } from "../schemas/file-meta/mod.js";

/**
 * Options for creating the KV VFS
 * Operations are scoped to a specific actor's KV store
 */
export interface KvVfsOptions {
	/** Get a single value by key. Returns null if missing. */
	get: (key: Uint8Array) => Promise<Uint8Array | null>;
	/** Get multiple values by keys. Returns null for missing keys. */
	getBatch: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
	/** Put a single key-value pair */
	put: (key: Uint8Array, value: Uint8Array) => Promise<void>;
	/** Put multiple key-value pairs */
	putBatch: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
	/** Delete multiple keys */
	deleteBatch: (keys: Uint8Array[]) => Promise<void>;
}

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
	SQLITE_OPEN_READWRITE: number;
	SQLITE_OPEN_CREATE: number;
}

/**
 * Simple async mutex for serializing database operations
 * wa-sqlite is not safe for concurrent open_v2 calls
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
	readonly #mutex: AsyncMutex;

	constructor(
		sqlite3: SQLite3Api,
		handle: number,
		fileName: string,
		onClose: () => void,
		mutex: AsyncMutex,
	) {
		this.#sqlite3 = sqlite3;
		this.#handle = handle;
		this.#fileName = fileName;
		this.#onClose = onClose;
		this.#mutex = mutex;
	}

	/**
	 * Execute SQL with optional row callback
	 * @param sql - SQL statement to execute
	 * @param callback - Called for each result row with (row, columns) where row is an array of values and columns is an array of column names
	 */
	async exec(sql: string, callback?: (row: unknown[], columns: string[]) => void): Promise<void> {
		return this.#mutex.run(async () => {
			return this.#sqlite3.exec(this.#handle, sql, callback);
		});
	}

	/**
	 * Close the database
	 */
	async close(): Promise<void> {
		await this.#mutex.run(async () => {
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
	#operationMutex = new AsyncMutex();
	#instanceId: string;

	constructor() {
		// Generate unique instance ID for VFS name
		this.#instanceId = crypto.randomUUID().replace(/-/g, "").slice(0, 8);
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
				this.#sqlite3 = Factory(module) as SQLite3Api;

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
		return this.#operationMutex.run(async () => {
			// Initialize wa-sqlite and SqliteSystem on first call
			await this.#ensureInitialized();

			if (!this.#sqlite3 || !this.#sqliteSystem) {
				throw new Error("SQLite not initialized");
			}

			// Register the file with its KV options
			this.#sqliteSystem.registerFile(fileName, options);

			const db = await this.#sqlite3.open_v2(
				fileName,
				this.#sqlite3.SQLITE_OPEN_READWRITE |
					this.#sqlite3.SQLITE_OPEN_CREATE,
				this.#sqliteSystem.name,
			);

			const sqliteSystem = this.#sqliteSystem;
			const onClose = () => {
				sqliteSystem.unregisterFile(fileName);
			};

			return new Database(
				this.#sqlite3,
				db,
				fileName,
				onClose,
				this.#operationMutex,
			);
		});
	}
}

/**
 * KV-backed VFS implementation for wa-sqlite
 */
class SqliteSystem extends (VFS.Base as typeof VFS.Base) {
	readonly #sqlite3: SQLite3Api;
	readonly #name: string;
	readonly #openFiles: Map<number, OpenFile> = new Map();
	readonly #fileOptions: Map<string, KvVfsOptions> = new Map();
	#nextFileId = 1;

	constructor(sqlite3: SQLite3Api, name: string) {
		super();
		this.#sqlite3 = sqlite3;
		this.#name = name;
	}

	get name(): string {
		return this.#name;
	}

	register(): void {
		this.#sqlite3.vfs_register(this, false);
	}

	registerFile(fileName: string, options: KvVfsOptions): void {
		this.#fileOptions.set(fileName, options);
	}

	unregisterFile(fileName: string): void {
		this.#fileOptions.delete(fileName);
	}

	#getOptionsForPath(path: string): KvVfsOptions | undefined {
		const direct = this.#fileOptions.get(path);
		if (direct) {
			return direct;
		}

		const suffixes = ["-journal", "-wal", "-shm"];
		for (const suffix of suffixes) {
			if (path.endsWith(suffix)) {
				const basePath = path.slice(0, -suffix.length);
				const baseOptions = this.#fileOptions.get(basePath);
				if (baseOptions) {
					return baseOptions;
				}
			}
		}

		return undefined;
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xOpen(
		name: string,
		fileId: number,
		flags: number,
		pOutFlags: DataView,
	): number {
		return this.handleAsync(async () => {
			try {
				const resolvedName = name && name.length > 0 ? name : `temp-${fileId}`;
				let options = this.#getOptionsForPath(resolvedName);
				if (!options && this.#fileOptions.size === 1) {
					options = this.#fileOptions.values().next().value;
				}
				if (!options) {
					throw new Error(`File not registered: ${resolvedName}`);
				}

				if (VFS_DEBUG) {
					vfsLog("xOpen", { file: resolvedName, flags });
				}

				const key = getMetaKey(resolvedName);
				const metaData = await options.get(key);
				let size = 0;

				if (metaData) {
					size = decodeFileMeta(metaData);
				}

				const file: OpenFile = {
					path: resolvedName,
					size,
					flags,
					options,
				};

				this.#openFiles.set(fileId, file);
				pOutFlags.setInt32(0, flags, true);
				return VFS.SQLITE_OK;
			} catch (error) {
				vfsLog("xOpen", {
					file: name,
					error: String(error),
				});
				return VFS.SQLITE_CANTOPEN;
			}
		});
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xClose(fileId: number): number {
		this.#openFiles.delete(fileId);
		return VFS.SQLITE_OK;
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xRead(
		fileId: number,
		pData: Uint8Array,
		iOffset: number,
		iAmt: number,
	): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR;
			}

			const offsetRaw =
				typeof iOffset === "bigint" ? Number(iOffset) : iOffset;
			const amountRaw = typeof iAmt === "bigint" ? Number(iAmt) : iAmt;
			const offset = Number.isFinite(offsetRaw) ? offsetRaw : 0;
			const amount = Number.isFinite(amountRaw) ? amountRaw : pData.length;
			const readStart = performance.now();
			const chunkKeys: Uint8Array[] = [];
			const startChunk = Math.floor(offset / CHUNK_SIZE);
			const endChunk = Math.floor((offset + amount - 1) / CHUNK_SIZE);

			for (let chunkIndex = startChunk; chunkIndex <= endChunk; chunkIndex++) {
				chunkKeys.push(getChunkKey(file.path, chunkIndex));
			}

			const chunks = await file.options.getBatch(chunkKeys);

			// Copy the requested data from the chunks into the buffer
			let bytesCopied = 0;
			for (let i = 0; i < chunks.length; i++) {
				const chunkIndex = startChunk + i;
				const chunkData = chunks[i];

				const chunkStartOffset = chunkIndex * CHUNK_SIZE;
				const readStartOffset = Math.max(offset - chunkStartOffset, 0);
				const readEndOffset = Math.min(
					CHUNK_SIZE,
					offset + amount - chunkStartOffset,
				);

				const readLength = readEndOffset - readStartOffset;
				if (readLength <= 0) {
					continue;
				}

				if (chunkData) {
					pData.set(
						chunkData.subarray(readStartOffset, readEndOffset),
						bytesCopied,
					);
				} else {
					// If chunk missing, fill with zeros
					pData.fill(0, bytesCopied, bytesCopied + readLength);
				}

				bytesCopied += readLength;
			}

			if (VFS_DEBUG) {
				vfsLog("xRead", {
					file: file.path,
					offset,
					len: amount,
					chunks: chunkKeys.length,
					ms: (performance.now() - readStart).toFixed(2),
				});
			}

			if (offset + amount > file.size) {
				return VFS.SQLITE_IOERR_SHORT_READ;
			}

			return VFS.SQLITE_OK;
		});
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xWrite(
		fileId: number,
		pData: Uint8Array,
		iOffset: number,
		iAmt: number,
	): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR;
			}

			const offsetRaw =
				typeof iOffset === "bigint" ? Number(iOffset) : iOffset;
			const amountRaw = typeof iAmt === "bigint" ? Number(iAmt) : iAmt;
			const offset = Number.isFinite(offsetRaw) ? offsetRaw : 0;
			const amount = Number.isFinite(amountRaw) ? amountRaw : pData.length;
			const writeStart = performance.now();
			const chunkKeys: Uint8Array[] = [];
			const startChunk = Math.floor(offset / CHUNK_SIZE);
			const endChunk = Math.floor((offset + amount - 1) / CHUNK_SIZE);

			for (let chunkIndex = startChunk; chunkIndex <= endChunk; chunkIndex++) {
				chunkKeys.push(getChunkKey(file.path, chunkIndex));
			}

			const getBatchStart = performance.now();
			const chunks = await file.options.getBatch(chunkKeys);
			const getBatchMs = performance.now() - getBatchStart;

			const entriesToWrite: [Uint8Array, Uint8Array][] = [];

			// Update each chunk with the new data
			let bytesWritten = 0;
			for (let i = 0; i < chunks.length; i++) {
				const chunkIndex = startChunk + i;
				const chunkData = chunks[i];

				const chunkStartOffset = chunkIndex * CHUNK_SIZE;
				const writeStartOffset = Math.max(offset - chunkStartOffset, 0);
				const writeEndOffset = Math.min(
					CHUNK_SIZE,
					offset + amount - chunkStartOffset,
				);

				const writeLength = writeEndOffset - writeStartOffset;
				if (writeLength <= 0) {
					continue;
				}

				// Create or clone the chunk data
				const newChunkData = chunkData
					? new Uint8Array(chunkData)
					: new Uint8Array(CHUNK_SIZE);

				// Copy data into the chunk
				newChunkData.set(
					pData.subarray(bytesWritten, bytesWritten + writeLength),
					writeStartOffset,
				);

				entriesToWrite.push([chunkKeys[i], newChunkData]);
				bytesWritten += writeLength;
			}

			const putBatchStart = performance.now();
			await file.options.putBatch(entriesToWrite);
			const putBatchMs = performance.now() - putBatchStart;

			// Update file size if needed
			const newSize = Math.max(file.size, offset + amount);
			if (newSize !== file.size) {
				file.size = newSize;
				const metaKey = getMetaKey(file.path);
				const metaData = encodeFileMeta(newSize);
				await file.options.put(metaKey, metaData);
			}

			if (VFS_DEBUG) {
				vfsLog("xWrite", {
					file: file.path,
					offset,
					len: amount,
					readChunks: chunkKeys.length,
					writeEntries: entriesToWrite.length,
					getBatchMs: getBatchMs.toFixed(2),
					putBatchMs: putBatchMs.toFixed(2),
					ms: (performance.now() - writeStart).toFixed(2),
				});
			}

			return VFS.SQLITE_OK;
		});
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xTruncate(fileId: number, size: number): number {
		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_IOERR;
		}

		const nextSize = typeof size === "bigint" ? Number(size) : size;
		file.size = Number.isFinite(nextSize) ? nextSize : 0;
		return VFS.SQLITE_OK;
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xSync(fileId: number): number {
		return this.handleAsync(async () => {
			const file = this.#openFiles.get(fileId);
			if (!file) {
				return VFS.SQLITE_IOERR;
			}

			// Update metadata
			const metaKey = getMetaKey(file.path);
			const metaData = encodeFileMeta(file.size);
			await file.options.put(metaKey, metaData);

			return VFS.SQLITE_OK;
		});
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xFileSize(fileId: number, pSize64: DataView): number {
		const file = this.#openFiles.get(fileId);
		if (!file) {
			pSize64.setBigInt64(0, BigInt(0), true);
			return VFS.SQLITE_OK;
		}

		pSize64.setBigInt64(0, BigInt(file.size), true);
		return VFS.SQLITE_OK;
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xDelete(name: string, _syncDir: number): number {
		// In a KV store, we can't easily delete all chunks without scanning
		// For now, we'll just remove the metadata
		const options = this.#getOptionsForPath(name);
		if (options) {
			const metaKey = getMetaKey(name);
			void options.deleteBatch([metaKey]);
		}
		return VFS.SQLITE_OK;
	}

	// @ts-expect-error - wa-sqlite types are incorrect
	xAccess(
		name: string,
		_flags: number,
		pResOut: DataView,
	): number {
		return this.handleAsync(async () => {
			let options = this.#getOptionsForPath(name);
			if (!options && this.#fileOptions.size === 1) {
				options = this.#fileOptions.values().next().value;
			}
			if (!options) {
				pResOut.setInt32(0, 0, true);
				return VFS.SQLITE_OK;
			}

			if (_flags === VFS.SQLITE_ACCESS_EXISTS) {
				const metaKey = getMetaKey(name);
				const metaData = await options.get(metaKey);
				pResOut.setInt32(0, metaData ? 1 : 0, true);
				return VFS.SQLITE_OK;
			}

			pResOut.setInt32(0, 1, true);
			return VFS.SQLITE_OK;
		});
	}
}
