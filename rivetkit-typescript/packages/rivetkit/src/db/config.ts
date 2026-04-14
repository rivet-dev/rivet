import type { ActorMetrics } from "@/actor/metrics";

export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

export type SqliteBindings = unknown[] | Record<string, unknown>;

export interface SqliteQueryResult {
	columns: string[];
	rows: unknown[][];
}

export interface SqliteDatabase {
	exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void>;
	run(sql: string, params?: SqliteBindings): Promise<void>;
	query(sql: string, params?: SqliteBindings): Promise<SqliteQueryResult>;
	close(): Promise<void>;
}

/**
 * Provider for opening native databases from the active runtime.
 */
export interface NativeDatabaseProvider {
	open(actorId: string): Promise<SqliteDatabase>;
}

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

	/** KV operations exposed for custom database providers. */
	kv: {
		batchPut: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
		batchGet: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
		batchDelete: (keys: Uint8Array[]) => Promise<void>;
		deleteRange: (start: Uint8Array, end: Uint8Array) => Promise<void>;
	};

	/**
	 * Actor metrics instance. When provided, KV and SQL operations are tracked.
	 */
	metrics?: ActorMetrics;

	/**
	 * Logger for debug output. When provided, SQL queries are logged with
	 * duration and KV call count.
	 */
	log?: { debug(obj: Record<string, unknown>): void };

	/**
	 * Provider for opening native databases from the active runtime.
	 */
	nativeDatabaseProvider?: NativeDatabaseProvider;
}

export type DatabaseProvider<DB extends RawAccess> = {
	/**
	 * When true, ActorInstance must provide a sqliteVfs handle even if the
	 * driver also exposes raw or native database overrides.
	 *
	 * Use this for custom providers that open KV-backed SQLite directly from
	 * ctx.sqliteVfs instead of delegating to rivetkit/db.
	 */
	requiresSqliteVfs?: boolean;

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
