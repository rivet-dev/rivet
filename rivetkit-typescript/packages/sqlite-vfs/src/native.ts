import { loadNativeBinding } from "./native-loader";
import {
	loadDatabaseBytes,
	persistDatabaseBytes,
	resolveChunkSize,
	resolveKvPrefix,
} from "./storage";
import type { KvVfsOptions, SqliteVfsConfig } from "./types";

type NativeBinding = {
	NativeDatabase: new (bytes?: Uint8Array) => NativeDatabase;
};

type NativeDatabase = {
	exec: (
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	) => void;
	export: () => Uint8Array;
	close: () => void;
};

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
	readonly #db: NativeDatabase;
	readonly #fileName: string;
	readonly #options: KvVfsOptions;
	readonly #mutex: AsyncMutex;
	readonly #chunkSize: number;
	readonly #kvPrefix: number;

	constructor(
		db: NativeDatabase,
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
			this.#db.exec(sql, callback);
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
	#binding: NativeBinding;
	#mutex = new AsyncMutex();
	#chunkSize: number;
	#kvPrefix: number;

	constructor(config: SqliteVfsConfig) {
		this.#binding = loadNativeBinding() as NativeBinding;
		this.#chunkSize = resolveChunkSize(config.chunkSize);
		this.#kvPrefix = resolveKvPrefix(config.kvPrefix);
	}

	async open(fileName: string, options: KvVfsOptions): Promise<Database> {
		return this.#mutex.run(async () => {
			const bytes = await loadDatabaseBytes(
				fileName,
				options,
				this.#chunkSize,
				this.#kvPrefix,
			);
			const db = bytes
				? new this.#binding.NativeDatabase(bytes)
				: new this.#binding.NativeDatabase();

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
