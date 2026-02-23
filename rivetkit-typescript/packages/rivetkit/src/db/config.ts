import type { SqliteVfs } from "@rivetkit/sqlite-vfs";

export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

/**
 * Context provided to database providers for creating database clients
 */
export interface DatabaseProviderContext {
	/**
	 * Actor ID
	 */
	actorId: string;

	/**
	 * Override the default raw database client (optional).
	 * If not provided, a KV-backed client will be constructed.
	 */
	overrideRawDatabaseClient?: () => Promise<RawDatabaseClient | undefined>;

	/**
	 * Override the default Drizzle database client (optional).
	 * If not provided, a KV-backed client will be constructed.
	 */
	overrideDrizzleDatabaseClient?: () => Promise<
		DrizzleDatabaseClient | undefined
	>;

	/**
	 * KV operations for constructing KV-backed database clients
	 */
	kv: {
		batchPut: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
		batchGet: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
		batchDelete: (keys: Uint8Array[]) => Promise<void>;
	};

	/**
	 * SQLite VFS instance for creating KV-backed databases.
	 * This should be actor-scoped because @rivetkit/sqlite is not re-entrant per
	 * module instance.
	 */
	sqliteVfs?: SqliteVfs;
}

export type DatabaseProvider<DB extends RawAccess> = {
	/**
	 * Creates a new database client for the actor.
	 * The result is passed to the actor context as `c.db`.
	 * @experimental
	 */
	createClient: (ctx: DatabaseProviderContext) => Promise<DB>;
	/**
	 * Runs before the actor has started.
	 * Use this to run migrations or other setup tasks.
	 * @experimental
	 */
	onMigrate: (client: DB) => void | Promise<void>;
	/**
	 * Runs when the actor is being destroyed.
	 * Use this to clean up database connections and release resources.
	 * @experimental
	 */
	onDestroy?: (client: DB) => void | Promise<void>;
};

/**
 * Raw database client with basic exec method
 */
export interface RawDatabaseClient {
	exec: <TRow extends Record<string, unknown> = Record<string, unknown>>(
		query: string,
		...args: unknown[]
	) => Promise<TRow[]> | TRow[];
}

/**
 * Drizzle database client interface (will be extended by drizzle-orm types)
 */
export interface DrizzleDatabaseClient {
	// This will be extended by BaseSQLiteDatabase from drizzle-orm
	// For now, just a marker interface
}

type ExecuteFunction = <
	TRow extends Record<string, unknown> = Record<string, unknown>,
>(
	query: string,
	...args: unknown[]
) => Promise<TRow[]>;

export type RawAccess = {
	/**
	 * Executes a raw SQL query.
	 */
	execute: ExecuteFunction;
	/**
	 * Closes the database connection and releases resources.
	 */
	close: () => Promise<void>;
};
