import initSqlJs from "sql.js";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import {
	loadDatabaseBytes,
	persistDatabaseBytes,
	resolveChunkSize,
	resolveKvPrefix,
} from "./storage";
import type { KvVfsOptions, SqliteVfsConfig } from "./types";

type SqlJsDatabase = InstanceType<Awaited<ReturnType<typeof initSqlJs>>["Database"]>;

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

export class Database {
	readonly #db: SqlJsDatabase;
	readonly #fileName: string;
	readonly #options: KvVfsOptions;
	readonly #mutex: AsyncMutex;
	readonly #chunkSize: number;
	readonly #kvPrefix: number;

	constructor(
		db: SqlJsDatabase,
		fileName: string,
		options: KvVfsOptions,
		mutex: AsyncMutex,
		chunkSize: number,
		kvPrefix: number,
	) {
		this.#db = db;
		this.#fileName = fileName;
		this.#options = options;
		this.#mutex = mutex;
		this.#chunkSize = chunkSize;
		this.#kvPrefix = kvPrefix;
	}

	async exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void> {
		await this.#mutex.run(async () => {
			const results = this.#db.exec(sql);
			if (callback) {
				for (const result of results) {
					const columns = result.columns;
					for (const row of result.values) {
						callback(row, columns);
					}
				}
			}

			const bytes = this.#db.export();
			await persistDatabaseBytes(
				this.#fileName,
				this.#options,
				this.#chunkSize,
				this.#kvPrefix,
				bytes,
			);
		});
	}

	async close(): Promise<void> {
		await this.#mutex.run(async () => {
			const bytes = this.#db.export();
			await persistDatabaseBytes(
				this.#fileName,
				this.#options,
				this.#chunkSize,
				this.#kvPrefix,
				bytes,
			);
			this.#db.close();
		});
	}
}

export class SqliteVfs {
	#initPromise: Promise<void> | null = null;
	#sqlModule: Awaited<ReturnType<typeof initSqlJs>> | null = null;
	#mutex = new AsyncMutex();
	#chunkSize: number;
	#kvPrefix: number;

	constructor(config: SqliteVfsConfig) {
		this.#chunkSize = resolveChunkSize(config.chunkSize);
		this.#kvPrefix = resolveKvPrefix(config.kvPrefix);
	}

	async #ensureInitialized(): Promise<void> {
		if (this.#sqlModule) {
			return;
		}

		if (!this.#initPromise) {
			this.#initPromise = (async () => {
				const isNode = typeof process !== "undefined" && !!process.versions?.node;
				if (isNode) {
					const require = createRequire(import.meta.url);
					const wasmPath = require.resolve("sql.js/dist/sql-wasm.wasm");
					const wasmBuffer = readFileSync(wasmPath);
					const wasmBinary = wasmBuffer.buffer.slice(
						wasmBuffer.byteOffset,
						wasmBuffer.byteOffset + wasmBuffer.byteLength,
					);
					this.#sqlModule = await initSqlJs({ wasmBinary });
					return;
				}

				this.#sqlModule = await initSqlJs({
					locateFile: (file) => new URL(`./${file}`, import.meta.url).toString(),
				});
			})();
		}

		await this.#initPromise;
	}

	async open(fileName: string, options: KvVfsOptions): Promise<Database> {
		return this.#mutex.run(async () => {
			await this.#ensureInitialized();
			if (!this.#sqlModule) {
				throw new Error("SQLite wasm not initialized");
			}

			const bytes = await loadDatabaseBytes(
				fileName,
				options,
				this.#chunkSize,
				this.#kvPrefix,
			);
			const db = bytes
				? new this.#sqlModule.Database(bytes)
				: new this.#sqlModule.Database();

			return new Database(
				db,
				fileName,
				options,
				this.#mutex,
				this.#chunkSize,
				this.#kvPrefix,
			);
		});
	}
}

export type { KvVfsOptions, SqliteVfsConfig };
