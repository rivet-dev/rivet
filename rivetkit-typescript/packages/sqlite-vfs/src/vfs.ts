/**
 * SQLite raw database with KV storage backend
 *
 * This module provides a SQLite API that uses a KV-backed VFS
 * for storage. Each SqliteVfs instance is independent and can be
 * used concurrently with other instances.
 *
 * Keep this VFS on direct VFS.Base callbacks for minimal wrapper overhead.
 * Use @rivetkit/sqlite/src/FacadeVFS.js as the reference implementation for
 * callback ABI and pointer/data conversion behavior.
 * This implementation is optimized for single-writer semantics because each
 * actor owns one SQLite database.
 * SQLite invokes this VFS with byte-range file operations. This VFS maps those
 * ranges onto fixed-size KV chunks keyed by file tag and chunk index.
 * We intentionally rely on SQLite's pager cache for hot page reuse and do not
 * add a second cache in this VFS. This avoids duplicate cache invalidation
 * logic and keeps memory usage predictable for each actor.
 */

import * as VFS from "@rivetkit/sqlite/src/VFS.js";
import {
	Factory,
	SQLITE_OPEN_CREATE,
	SQLITE_OPEN_READWRITE,
	SQLITE_ROW,
} from "@rivetkit/sqlite";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import {
	CHUNK_SIZE,
	FILE_TAG_JOURNAL,
	FILE_TAG_MAIN,
	FILE_TAG_SHM,
	FILE_TAG_WAL,
	getChunkKey,
	getMetaKey,
	type SqliteFileTag,
} from "./kv";
import {
	FILE_META_VERSIONED,
	CURRENT_VERSION,
} from "../schemas/file-meta/versioned";
import type { FileMeta } from "../schemas/file-meta/mod";
import type { KvVfsOptions } from "./types";

type SqliteEsmFactory = (config?: { wasmBinary?: ArrayBuffer | Uint8Array }) => Promise<unknown>;
type SQLite3Api = ReturnType<typeof Factory>;
type SqliteBindings = Parameters<SQLite3Api["bind_collection"]>[1];
type SqliteVfsRegistration = Parameters<SQLite3Api["vfs_register"]>[0];

interface SQLiteModule {
	UTF8ToString: (ptr: number) => string;
	HEAPU8: Uint8Array;
}

const TEXT_ENCODER = new TextEncoder();
const TEXT_DECODER = new TextDecoder();
const SQLITE_MAX_PATHNAME_BYTES = 64;

// Chunk keys encode the chunk index in 32 bits, so a file can span at most
// 2^32 chunks. At 4 KiB/chunk this yields a hard limit of 16 TiB.
const UINT32_SIZE = 0x100000000;
const MAX_CHUNK_INDEX = 0xffffffff;
const MAX_FILE_SIZE_BYTES = (MAX_CHUNK_INDEX + 1) * CHUNK_SIZE;
const MAX_FILE_SIZE_HI32 = Math.floor(MAX_FILE_SIZE_BYTES / UINT32_SIZE);
const MAX_FILE_SIZE_LO32 = MAX_FILE_SIZE_BYTES % UINT32_SIZE;

// libvfs captures this async/sync mask at registration time. Any VFS callback
// that returns a Promise must be listed here so SQLite uses async relays.
const SQLITE_ASYNC_METHODS = new Set([
	"xOpen",
	"xClose",
	"xRead",
	"xWrite",
	"xTruncate",
	"xSync",
	"xFileSize",
	"xDelete",
	"xAccess",
]);

interface LoadedSqliteRuntime {
	sqlite3: SQLite3Api;
	module: SQLiteModule;
}

function isSqliteEsmFactory(value: unknown): value is SqliteEsmFactory {
	return typeof value === "function";
}

function isSQLiteModule(value: unknown): value is SQLiteModule {
	if (!value || typeof value !== "object") {
		return false;
	}
	const candidate = value as {
		UTF8ToString?: unknown;
		HEAPU8?: unknown;
	};
	return (
		typeof candidate.UTF8ToString === "function" &&
		candidate.HEAPU8 instanceof Uint8Array
	);
}


/**
 * Lazily load and instantiate the async SQLite module for this VFS instance.
 * We do this on first open so actors that do not use SQLite do not pay module
 * parse and wasm initialization cost at startup, and we pass wasmBinary
 * explicitly so this works consistently in both ESM and CJS bundles.
 */
async function loadSqliteRuntime(): Promise<LoadedSqliteRuntime> {
	// Keep the module specifier assembled at runtime so TypeScript declaration
	// generation does not try to typecheck this deep dist import path.
	// Uses Array.join() instead of string concatenation to prevent esbuild/tsup
	// from constant-folding the expression at build time, which would allow
	// Turbopack to trace into the WASM package.
	const specifier = ["@rivetkit/sqlite", "dist", "wa-sqlite-async.mjs"].join("/");
	const sqliteModule = await import(specifier);
	if (!isSqliteEsmFactory(sqliteModule.default)) {
		throw new Error("Invalid SQLite ESM factory export");
	}
	const sqliteEsmFactory = sqliteModule.default;
	const require = createRequire(import.meta.url);
	const wasmPath = require.resolve("@rivetkit/sqlite/dist/wa-sqlite-async.wasm");
	const wasmBinary = readFileSync(wasmPath);
	const module = await sqliteEsmFactory({ wasmBinary });
	if (!isSQLiteModule(module)) {
		throw new Error("Invalid SQLite runtime module");
	}
	return {
		sqlite3: Factory(module),
		module,
	};
}

/**
 * Represents an open file
 */
interface OpenFile {
	/** File path */
	path: string;
	/** File kind tag used by compact key layout */
	fileTag: SqliteFileTag;
	/** Precomputed metadata key */
	metaKey: Uint8Array;
	/** File size in bytes */
	size: number;
	/** True when in-memory size has not been persisted yet */
	metaDirty: boolean;
	/** Open flags */
	flags: number;
	/** KV options for this file */
	options: KvVfsOptions;
}

interface ResolvedFile {
	options: KvVfsOptions;
	fileTag: SqliteFileTag;
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

function isValidFileSize(size: number): boolean {
	return Number.isSafeInteger(size) && size >= 0 && size <= MAX_FILE_SIZE_BYTES;
}

/**
 * Simple async mutex for serializing database operations
 * @rivetkit/sqlite calls are not safe to run concurrently on one module instance
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
		await this.#sqliteMutex.run(async () => {
			await this.#sqlite3.exec(this.#handle, sql, callback);
		});
	}

	/**
	 * Execute a parameterized SQL statement (no result rows)
	 * @param sql - SQL statement with ? placeholders
	 * @param params - Parameter values to bind
	 */
	async run(sql: string, params?: SqliteBindings): Promise<void> {
		await this.#sqliteMutex.run(async () => {
			for await (const stmt of this.#sqlite3.statements(this.#handle, sql)) {
				if (params) {
					this.#sqlite3.bind_collection(stmt, params);
				}
				while ((await this.#sqlite3.step(stmt)) === SQLITE_ROW) {
					// Consume rows for statements that return results.
				}
			}
		});
	}

	/**
	 * Execute a parameterized SQL query and return results
	 * @param sql - SQL query with ? placeholders
	 * @param params - Parameter values to bind
	 * @returns Object with rows (array of arrays) and columns (column names)
	 */
	async query(sql: string, params?: SqliteBindings): Promise<{ rows: unknown[][]; columns: string[] }> {
		return this.#sqliteMutex.run(async () => {
			const rows: unknown[][] = [];
			let columns: string[] = [];
			for await (const stmt of this.#sqlite3.statements(this.#handle, sql)) {
				if (params) {
					this.#sqlite3.bind_collection(stmt, params);
				}

				while ((await this.#sqlite3.step(stmt)) === SQLITE_ROW) {
					if (columns.length === 0) {
						columns = this.#sqlite3.column_names(stmt);
					}
					rows.push(this.#sqlite3.row(stmt));
				}
			}

			return { rows, columns };
		});
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
	 * Get the raw @rivetkit/sqlite API (for advanced usage)
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
 * Each instance is independent and has its own @rivetkit/sqlite WASM module.
 * This allows multiple instances to operate concurrently without interference.
 */
export class SqliteVfs {
	#sqlite3: SQLite3Api | null = null;
	#sqliteSystem: SqliteSystem | null = null;
	#initPromise: Promise<void> | null = null;
	#openMutex = new AsyncMutex();
	#sqliteMutex = new AsyncMutex();
	#instanceId: string;
	#destroyed = false;

	constructor() {
		// Generate unique instance ID for VFS name
		this.#instanceId = crypto.randomUUID().replace(/-/g, '').slice(0, 8);
	}

	/**
	 * Initialize @rivetkit/sqlite and VFS (called once per instance)
	 */
	async #ensureInitialized(): Promise<void> {
		if (this.#destroyed) {
			throw new Error("SqliteVfs is closed");
		}

		// Fast path: already initialized
		if (this.#sqlite3 && this.#sqliteSystem) {
			return;
		}

		// Synchronously create the promise if not started
		if (!this.#initPromise) {
			this.#initPromise = (async () => {
				const { sqlite3, module } = await loadSqliteRuntime();
				if (this.#destroyed) {
					return;
				}
				this.#sqlite3 = sqlite3;
				this.#sqliteSystem = new SqliteSystem(
					sqlite3,
					module,
					`kv-vfs-${this.#instanceId}`,
				);
				this.#sqliteSystem.register();
			})();
		}

		// Wait for initialization
		try {
			await this.#initPromise;
		} catch (error) {
			this.#initPromise = null;
			throw error;
		}
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
		if (this.#destroyed) {
			throw new Error("SqliteVfs is closed");
		}

		// Serialize all open operations within this instance
		await this.#openMutex.acquire();
		try {
			// Initialize @rivetkit/sqlite and SqliteSystem on first call
			await this.#ensureInitialized();

			if (!this.#sqlite3 || !this.#sqliteSystem) {
				throw new Error("Failed to initialize SQLite");
			}
			const sqlite3 = this.#sqlite3;
			const sqliteSystem = this.#sqliteSystem;

			// Register this filename with its KV options
			sqliteSystem.registerFile(fileName, options);

			// Open database
			const db = await this.#sqliteMutex.run(async () =>
				sqlite3.open_v2(
					fileName,
					SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
					sqliteSystem.name,
				),
			);
			// TODO: Benchmark PRAGMA tuning for KV-backed SQLite after open.
			// Start with journal_mode=PERSIST and journal_size_limit to reduce
			// journal churn on high-latency KV without introducing WAL.

			// Create cleanup callback
			const onClose = () => {
				sqliteSystem.unregisterFile(fileName);
			};

			return new Database(
				sqlite3,
				db,
				fileName,
				onClose,
				this.#sqliteMutex,
			);
		} finally {
			this.#openMutex.release();
		}
	}

	/**
	 * Tears down this VFS instance and releases internal references.
	 */
	async destroy(): Promise<void> {
		if (this.#destroyed) {
			return;
		}
		this.#destroyed = true;

		const initPromise = this.#initPromise;
		if (initPromise) {
			try {
				await initPromise;
			} catch {
				// Initialization failure already surfaced to caller.
			}
		}

		if (this.#sqliteSystem) {
			await this.#sqliteSystem.close();
		}

		this.#sqliteSystem = null;
		this.#sqlite3 = null;
		this.#initPromise = null;
	}

	/**
	 * Alias for destroy to align with DB-style lifecycle naming.
	 */
	async close(): Promise<void> {
		await this.destroy();
	}
}

/**
 * Internal VFS implementation
 */
class SqliteSystem implements SqliteVfsRegistration {
	readonly name: string;
	readonly mxPathName = SQLITE_MAX_PATHNAME_BYTES;
	readonly mxPathname = SQLITE_MAX_PATHNAME_BYTES;
	#mainFileName: string | null = null;
	#mainFileOptions: KvVfsOptions | null = null;
	readonly #openFiles: Map<number, OpenFile> = new Map();
	readonly #sqlite3: SQLite3Api;
	readonly #module: SQLiteModule;
	#heapDataView: DataView;
	#heapDataViewBuffer: ArrayBufferLike;

	constructor(sqlite3: SQLite3Api, module: SQLiteModule, name: string) {
		this.name = name;
		this.#sqlite3 = sqlite3;
		this.#module = module;
		this.#heapDataViewBuffer = module.HEAPU8.buffer;
		this.#heapDataView = new DataView(this.#heapDataViewBuffer);
	}

	async close(): Promise<void> {
		this.#openFiles.clear();
		this.#mainFileName = null;
		this.#mainFileOptions = null;
	}

	isReady(): boolean {
		return true;
	}

	hasAsyncMethod(methodName: string): boolean {
		return SQLITE_ASYNC_METHODS.has(methodName);
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
		if (!this.#mainFileName) {
			this.#mainFileName = fileName;
			this.#mainFileOptions = options;
			return;
		}

		if (this.#mainFileName !== fileName) {
			throw new Error(
				`SqliteSystem is actor-scoped and expects one main file. Got ${fileName}, expected ${this.#mainFileName}.`,
			);
		}

		this.#mainFileOptions = options;
	}

	/**
	 * Unregisters a file's KV options (after closing)
	 */
	unregisterFile(fileName: string): void {
		if (this.#mainFileName === fileName) {
			this.#mainFileName = null;
			this.#mainFileOptions = null;
		}
	}

	/**
	 * Resolve file path to the actor's main DB file or known SQLite sidecars.
	 */
	#resolveFile(path: string): ResolvedFile | null {
		if (!this.#mainFileName || !this.#mainFileOptions) {
			return null;
		}

		if (path === this.#mainFileName) {
			return { options: this.#mainFileOptions, fileTag: FILE_TAG_MAIN };
		}
		if (path === `${this.#mainFileName}-journal`) {
			return { options: this.#mainFileOptions, fileTag: FILE_TAG_JOURNAL };
		}
		if (path === `${this.#mainFileName}-wal`) {
			return { options: this.#mainFileOptions, fileTag: FILE_TAG_WAL };
		}
		if (path === `${this.#mainFileName}-shm`) {
			return { options: this.#mainFileOptions, fileTag: FILE_TAG_SHM };
		}

		return null;
	}

	#resolveFileOrThrow(path: string): ResolvedFile {
		const resolved = this.#resolveFile(path);
		if (resolved) {
			return resolved;
		}

		if (!this.#mainFileName) {
			throw new Error(`No KV options registered for file: ${path}`);
		}

		throw new Error(
			`Unsupported SQLite file path ${path}. Expected one of ${this.#mainFileName}, ${this.#mainFileName}-journal, ${this.#mainFileName}-wal, ${this.#mainFileName}-shm.`,
		);
	}

	#chunkKey(file: OpenFile, chunkIndex: number): Uint8Array {
		return getChunkKey(file.fileTag, chunkIndex);
	}

	async xOpen(
		_pVfs: number,
		zName: number,
		fileId: number,
		flags: number,
		pOutFlags: number,
	): Promise<number> {
		const path = this.#decodeFilename(zName, flags);
		if (!path) {
			return VFS.SQLITE_CANTOPEN;
		}

		// Get the registered KV options for this file
		// For journal/wal files, use the main database's options
		const { options, fileTag } = this.#resolveFileOrThrow(path);
		const metaKey = getMetaKey(fileTag);

		// Get existing file size if the file exists
		const sizeData = await options.get(metaKey);

		let size: number;

		if (sizeData) {
			// File exists, use existing size
			size = decodeFileMeta(sizeData);
			if (!isValidFileSize(size)) {
				return VFS.SQLITE_IOERR;
			}
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
			fileTag,
			metaKey,
			size,
			metaDirty: false,
			flags,
			options,
		});

		// Set output flags to the actual flags used.
		this.#writeInt32(pOutFlags, flags);

		return VFS.SQLITE_OK;
	}

	async xClose(fileId: number): Promise<number> {
		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_OK;
		}

		// Delete-on-close files should skip metadata flush because the file will
		// be removed immediately.
		if (file.flags & VFS.SQLITE_OPEN_DELETEONCLOSE) {
			await this.#delete(file.path);
			this.#openFiles.delete(fileId);
			return VFS.SQLITE_OK;
		}

		if (file.metaDirty) {
			await file.options.put(file.metaKey, encodeFileMeta(file.size));
			file.metaDirty = false;
		}

		this.#openFiles.delete(fileId);
		return VFS.SQLITE_OK;
	}

	async xRead(
		fileId: number,
		pData: number,
		iAmt: number,
		iOffsetLo: number,
		iOffsetHi: number,
	): Promise<number> {
		if (iAmt === 0) {
			return VFS.SQLITE_OK;
		}

		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_IOERR_READ;
		}

		const data = this.#module.HEAPU8.subarray(pData, pData + iAmt);
		const options = file.options;
		const requestedLength = iAmt;
		const iOffset = delegalize(iOffsetLo, iOffsetHi);
		if (iOffset < 0) {
			return VFS.SQLITE_IOERR_READ;
		}
		const fileSize = file.size;

		// If offset is beyond file size, return short read with zeroed buffer
		if (iOffset >= fileSize) {
			data.fill(0);
			return VFS.SQLITE_IOERR_SHORT_READ;
		}

		// Calculate which chunks we need to read
		const startChunk = Math.floor(iOffset / CHUNK_SIZE);
		const endChunk = Math.floor((iOffset + requestedLength - 1) / CHUNK_SIZE);

		// Fetch all needed chunks
		const chunkKeys: Uint8Array[] = [];
		for (let i = startChunk; i <= endChunk; i++) {
			chunkKeys.push(this.#chunkKey(file, i));
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
					data.set(chunkData.subarray(sourceStart, sourceEnd), destStart);
				}

				// Zero-fill if chunk is smaller than expected
				if (sourceEnd < readEnd) {
					const zeroStart = destStart + (sourceEnd - sourceStart);
					const zeroEnd = destStart + (readEnd - readStart);
					data.fill(0, zeroStart, zeroEnd);
				}
			} else {
				// Chunk doesn't exist, zero-fill
				const destStart = chunkOffset + readStart - iOffset;
				const destEnd = destStart + (readEnd - readStart);
				data.fill(0, destStart, destEnd);
			}
		}

		// If we read less than requested (past EOF), return short read
		const actualBytes = Math.min(requestedLength, fileSize - iOffset);
		if (actualBytes < requestedLength) {
			data.fill(0, actualBytes);
			return VFS.SQLITE_IOERR_SHORT_READ;
		}

		return VFS.SQLITE_OK;
	}

	async xWrite(
		fileId: number,
		pData: number,
		iAmt: number,
		iOffsetLo: number,
		iOffsetHi: number,
	): Promise<number> {
		if (iAmt === 0) {
			return VFS.SQLITE_OK;
		}

		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_IOERR_WRITE;
		}

		const data = this.#module.HEAPU8.subarray(pData, pData + iAmt);
		const iOffset = delegalize(iOffsetLo, iOffsetHi);
		if (iOffset < 0) {
			return VFS.SQLITE_IOERR_WRITE;
		}
		const options = file.options;
		const writeLength = iAmt;
		const writeEndOffset = iOffset + writeLength;
		if (writeEndOffset > MAX_FILE_SIZE_BYTES) {
			return VFS.SQLITE_IOERR_WRITE;
		}

		// Calculate which chunks we need to modify
		const startChunk = Math.floor(iOffset / CHUNK_SIZE);
		const endChunk = Math.floor((iOffset + writeLength - 1) / CHUNK_SIZE);

		interface WritePlan {
			chunkKey: Uint8Array;
			chunkOffset: number;
			writeStart: number;
			writeEnd: number;
			existingChunkIndex: number;
		}

		// Only fetch chunks where we must preserve existing prefix/suffix bytes.
		const plans: WritePlan[] = [];
		const chunkKeysToFetch: Uint8Array[] = [];
		for (let i = startChunk; i <= endChunk; i++) {
			const chunkOffset = i * CHUNK_SIZE;
			const writeStart = Math.max(0, iOffset - chunkOffset);
			const writeEnd = Math.min(
				CHUNK_SIZE,
				iOffset + writeLength - chunkOffset,
			);
			const existingBytesInChunk = Math.max(
				0,
				Math.min(CHUNK_SIZE, file.size - chunkOffset),
			);
			const needsExisting = writeStart > 0 || existingBytesInChunk > writeEnd;
			const chunkKey = this.#chunkKey(file, i);
			let existingChunkIndex = -1;
			if (needsExisting) {
				existingChunkIndex = chunkKeysToFetch.length;
				chunkKeysToFetch.push(chunkKey);
			}
			plans.push({
				chunkKey,
				chunkOffset,
				writeStart,
				writeEnd,
				existingChunkIndex,
			});
		}

		const existingChunks = chunkKeysToFetch.length > 0
			? await options.getBatch(chunkKeysToFetch)
			: [];

		// Prepare new chunk data
		const entriesToWrite: [Uint8Array, Uint8Array][] = [];

		for (const plan of plans) {
			const existingChunk =
				plan.existingChunkIndex >= 0
					? existingChunks[plan.existingChunkIndex]
					: null;
			// Create new chunk data
			let newChunk: Uint8Array;
			if (existingChunk) {
				newChunk = new Uint8Array(Math.max(existingChunk.length, plan.writeEnd));
				newChunk.set(existingChunk);
			} else {
				newChunk = new Uint8Array(plan.writeEnd);
			}

			// Copy data from input buffer to chunk
			const sourceStart = plan.chunkOffset + plan.writeStart - iOffset;
			const sourceEnd = sourceStart + (plan.writeEnd - plan.writeStart);
			newChunk.set(data.subarray(sourceStart, sourceEnd), plan.writeStart);

			entriesToWrite.push([plan.chunkKey, newChunk]);
		}

		// Update file size if we wrote past the end
		const previousSize = file.size;
		const newSize = Math.max(file.size, writeEndOffset);
		if (newSize !== previousSize) {
			file.size = newSize;
			file.metaDirty = true;
		}
		if (file.metaDirty) {
			entriesToWrite.push([file.metaKey, encodeFileMeta(file.size)]);
		}

		// Write all chunks and metadata
		await options.putBatch(entriesToWrite);
		if (file.metaDirty) {
			file.metaDirty = false;
		}

		return VFS.SQLITE_OK;
	}

	async xTruncate(
		fileId: number,
		sizeLo: number,
		sizeHi: number,
	): Promise<number> {
		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_IOERR_TRUNCATE;
		}

		const size = delegalize(sizeLo, sizeHi);
		if (size < 0 || size > MAX_FILE_SIZE_BYTES) {
			return VFS.SQLITE_IOERR_TRUNCATE;
		}
		const options = file.options;

		// If truncating to larger size, just update metadata
		if (size >= file.size) {
			if (size > file.size) {
				file.size = size;
				file.metaDirty = true;
				await options.put(file.metaKey, encodeFileMeta(file.size));
				file.metaDirty = false;
			}
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
			keysToDelete.push(this.#chunkKey(file, i));
		}

		if (keysToDelete.length > 0) {
			await options.deleteBatch(keysToDelete);
		}

		// Truncate the last kept chunk if needed
		if (size > 0 && size % CHUNK_SIZE !== 0) {
			const lastChunkKey = this.#chunkKey(file, lastChunkToKeep);
			const lastChunkData = await options.get(lastChunkKey);

			if (lastChunkData && lastChunkData.length > size % CHUNK_SIZE) {
				const truncatedChunk = lastChunkData.subarray(0, size % CHUNK_SIZE);
				await options.put(lastChunkKey, truncatedChunk);
			}
		}

		// Update file size
		file.size = size;
		file.metaDirty = true;
		await options.put(file.metaKey, encodeFileMeta(file.size));
		file.metaDirty = false;

		return VFS.SQLITE_OK;
	}

	async xSync(fileId: number, _flags: number): Promise<number> {
		const file = this.#openFiles.get(fileId);
		if (!file || !file.metaDirty) {
			return VFS.SQLITE_OK;
		}

		await file.options.put(file.metaKey, encodeFileMeta(file.size));
		file.metaDirty = false;
		return VFS.SQLITE_OK;
	}

	async xFileSize(fileId: number, pSize: number): Promise<number> {
		const file = this.#openFiles.get(fileId);
		if (!file) {
			return VFS.SQLITE_IOERR_FSTAT;
		}

		// Set size as 64-bit integer.
		this.#writeBigInt64(pSize, BigInt(file.size));
		return VFS.SQLITE_OK;
	}

	async xDelete(_pVfs: number, zName: number, _syncDir: number): Promise<number> {
		await this.#delete(this.#module.UTF8ToString(zName));
		return VFS.SQLITE_OK;
	}

	/**
	 * Internal delete implementation
	 */
	async #delete(path: string): Promise<void> {
		const { options, fileTag } = this.#resolveFileOrThrow(path);
		const metaKey = getMetaKey(fileTag);

		// Get file size to find out how many chunks to delete
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
			keysToDelete.push(getChunkKey(fileTag, i));
		}

		await options.deleteBatch(keysToDelete);
	}

	async xAccess(
		_pVfs: number,
		zName: number,
		_flags: number,
		pResOut: number,
	): Promise<number> {
		// TODO: Measure how often xAccess runs during open and whether these
		// existence checks add meaningful KV round-trip overhead. If they do,
		// consider serving file existence from in-memory state.
		const path = this.#module.UTF8ToString(zName);
		const resolved = this.#resolveFile(path);
		if (!resolved) {
			// File not registered, doesn't exist
			this.#writeInt32(pResOut, 0);
			return VFS.SQLITE_OK;
		}

		const compactMetaKey = getMetaKey(resolved.fileTag);
		const metaData = await resolved.options.get(compactMetaKey);

		// Set result: 1 if file exists, 0 otherwise
		this.#writeInt32(pResOut, metaData ? 1 : 0);
		return VFS.SQLITE_OK;
	}

	xCheckReservedLock(_fileId: number, pResOut: number): number {
		// This VFS is actor-scoped with one writer, so there is no external
		// reserved lock state to report.
		this.#writeInt32(pResOut, 0);
		return VFS.SQLITE_OK;
	}

	xLock(_fileId: number, _flags: number): number {
		return VFS.SQLITE_OK;
	}

	xUnlock(_fileId: number, _flags: number): number {
		return VFS.SQLITE_OK;
	}

	xFileControl(_fileId: number, _flags: number, _pArg: number): number {
		return VFS.SQLITE_NOTFOUND;
	}

	xDeviceCharacteristics(_fileId: number): number {
		return 0;
	}

	xFullPathname(_pVfs: number, zName: number, nOut: number, zOut: number): number {
		const path = this.#module.UTF8ToString(zName);
		const bytes = TEXT_ENCODER.encode(path);
		const out = this.#module.HEAPU8.subarray(zOut, zOut + nOut);
		if (bytes.length >= out.length) {
			return VFS.SQLITE_IOERR;
		}
		out.set(bytes, 0);
		out[bytes.length] = 0;
		return VFS.SQLITE_OK;
	}

	#decodeFilename(zName: number, flags: number): string | null {
		if (!zName) {
			return null;
		}

		if (flags & VFS.SQLITE_OPEN_URI) {
			// Decode SQLite URI filename layout: path\0key\0value\0...\0
			let pName = zName;
			let state: 1 | 2 | 3 | null = 1;
			const charCodes: number[] = [];
			while (state) {
				const charCode = this.#module.HEAPU8[pName++];
				if (charCode) {
					charCodes.push(charCode);
					continue;
				}

				if (!this.#module.HEAPU8[pName]) {
					state = null;
				}
				switch (state) {
					case 1:
						charCodes.push("?".charCodeAt(0));
						state = 2;
						break;
					case 2:
						charCodes.push("=".charCodeAt(0));
						state = 3;
						break;
					case 3:
						charCodes.push("&".charCodeAt(0));
						state = 2;
						break;
				}
			}
			return TEXT_DECODER.decode(new Uint8Array(charCodes));
		}

		return this.#module.UTF8ToString(zName);
	}

	#heapView(): DataView {
		const heapBuffer = this.#module.HEAPU8.buffer;
		if (heapBuffer !== this.#heapDataViewBuffer) {
			this.#heapDataViewBuffer = heapBuffer;
			this.#heapDataView = new DataView(heapBuffer);
		}
		return this.#heapDataView;
	}

	#writeInt32(pointer: number, value: number): void {
		const heapByteOffset = this.#module.HEAPU8.byteOffset + pointer;
		this.#heapView().setInt32(heapByteOffset, value, true);
	}

	#writeBigInt64(pointer: number, value: bigint): void {
		const heapByteOffset = this.#module.HEAPU8.byteOffset + pointer;
		this.#heapView().setBigInt64(heapByteOffset, value, true);
	}
}

/**
 * Rebuild an i64 from Emscripten's legalized (lo32, hi32) pair.
 * SQLite passes file offsets and sizes this way. We decode into unsigned words
 * and reject values above the VFS max file size.
 */
function delegalize(lo32: number, hi32: number): number {
	const hi = hi32 >>> 0;
	const lo = lo32 >>> 0;
	if (hi > MAX_FILE_SIZE_HI32) {
		return -1;
	}
	if (hi === MAX_FILE_SIZE_HI32 && lo > MAX_FILE_SIZE_LO32) {
		return -1;
	}
	return (hi * UINT32_SIZE) + lo;
}
