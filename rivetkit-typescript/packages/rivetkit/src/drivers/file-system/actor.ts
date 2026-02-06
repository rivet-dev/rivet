import type { AnyClient } from "@/client/client";
import type { RawDatabaseClient } from "@/db/config";
import type {
	ActorDriver,
	AnyActorInstance,
	ManagerDriver,
} from "@/driver-helpers/mod";
import type { FileSystemGlobalState } from "./global-state";
import { RegistryConfig } from "@/registry/config";

export type ActorDriverContext = Record<never, never>;

/**
 * Type alias for better-sqlite3 Database.
 * We define this inline to avoid importing from better-sqlite3 directly,
 * since it's an optional peer dependency.
 */
type BetterSQLite3Database = {
	prepare(sql: string): {
		run(...args: unknown[]): { changes: number; lastInsertRowid: number | bigint };
		all(...args: unknown[]): unknown[];
		get(...args: unknown[]): unknown;
	};
	exec(sql: string): void;
	close(): void;
};

/**
 * File System implementation of the Actor Driver
 */
export class FileSystemActorDriver implements ActorDriver {
	#config: RegistryConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: AnyClient;
	#state: FileSystemGlobalState;
	#nativeDatabases: Map<string, BetterSQLite3Database> = new Map();
	#drizzleDatabases: Map<string, any> = new Map();

	constructor(
		config: RegistryConfig,
		managerDriver: ManagerDriver,
		inlineClient: AnyClient,
		state: FileSystemGlobalState,
	) {
		this.#config = config;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;
		this.#state = state;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		return this.#state.startActor(
			this.#config,
			this.#inlineClient,
			this,
			actorId,
		);
	}

	/**
	 * Get the current storage directory path
	 */
	get storagePath(): string {
		return this.#state.storagePath;
	}

	getContext(_actorId: string): ActorDriverContext {
		return {};
	}

	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		await this.#state.kvBatchPut(actorId, entries);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		return await this.#state.kvBatchGet(actorId, keys);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		await this.#state.kvBatchDelete(actorId, keys);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#state.kvListPrefix(actorId, prefix);
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#state.setActorAlarm(actor.id, timestamp);
	}

	async overrideRawDatabaseClient(
		actorId: string,
	): Promise<RawDatabaseClient | undefined> {
		if (!this.#state.useNativeSqlite) {
			return undefined;
		}

		// Check if we already have a cached database for this actor
		const existingDb = this.#nativeDatabases.get(actorId);
		if (existingDb) {
			return {
				exec: (query: string) => {
					const trimmed = query.trim();
					const upper = trimmed.toUpperCase();
					const withoutTrailing =
						trimmed.replace(/;+\s*$/g, "");
					const hasMultipleStatements =
						withoutTrailing.includes(";");
					if (hasMultipleStatements) {
						existingDb.exec(query);
						return [];
					}
					if (upper.startsWith("SELECT") || upper.startsWith("PRAGMA")) {
						// SELECT/PRAGMA queries return data
						return existingDb.prepare(query).all();
					}
					// Non-SELECT queries (INSERT, UPDATE, DELETE, CREATE, etc.)
					// Use run() which doesn't throw for non-returning queries
					existingDb.prepare(query).run();
					return [];
				},
			};
		}

		// Dynamically import better-sqlite3
		try {
			const Database = (await import("better-sqlite3")).default;

			const dbPath = this.#state.getActorDbPath(actorId);
			const db = new Database(dbPath) as BetterSQLite3Database;

			this.#nativeDatabases.set(actorId, db);

			return {
				exec: (query: string) => {
					// HACK: sqlite3 throws error if not using a SELECT statement
					const trimmed = query.trim();
					const upper = trimmed.toUpperCase();
					const withoutTrailing =
						trimmed.replace(/;+\s*$/g, "");
					const hasMultipleStatements =
						withoutTrailing.includes(";");
					if (hasMultipleStatements) {
						db.exec(query);
						return [];
					}
					if (
						upper.startsWith("SELECT") ||
						upper.startsWith("PRAGMA")
					) {
						// SELECT/PRAGMA queries return data
						return db.prepare(query).all();
					} else {
						// Non-SELECT queries (INSERT, UPDATE, DELETE, CREATE, etc.)
						// Use run() which doesn't throw for non-returning queries
						db.prepare(query).run();
						return [];
					}
				},
			};
		} catch (error) {
			throw new Error(
				`Failed to load better-sqlite3. Make sure it's installed: ${error}`,
			);
		}
	}

	async overrideDrizzleDatabaseClient(
		actorId: string,
	): Promise<any | undefined> {
		if (!this.#state.useNativeSqlite) {
			return undefined;
		}

		// Check if we already have a cached drizzle database for this actor
		const existingDrizzleDb = this.#drizzleDatabases.get(actorId);
		if (existingDrizzleDb) {
			return existingDrizzleDb;
		}

		// Get or create the raw better-sqlite3 database
		let rawDb = this.#nativeDatabases.get(actorId);
		if (!rawDb) {
			// Create it via overrideRawDatabaseClient
			await this.overrideRawDatabaseClient(actorId);
			rawDb = this.#nativeDatabases.get(actorId);
			if (!rawDb) {
				throw new Error(
					"Failed to initialize native database for actor",
				);
			}
		}

		// Dynamically import drizzle and wrap the raw database
		try {
			const { drizzle } = await import("drizzle-orm/better-sqlite3");
			const drizzleDb = drizzle(rawDb as any);

			this.#drizzleDatabases.set(actorId, drizzleDb);

			return drizzleDb;
		} catch (error) {
			throw new Error(
				`Failed to load drizzle-orm. Make sure it's installed: ${error}`,
			);
		}
	}

	startSleep(actorId: string): void {
		// Spawns the sleepActor promise
		this.#state.sleepActor(actorId);
	}

	async startDestroy(actorId: string): Promise<void> {
		await this.#state.destroyActor(actorId);
	}
}
