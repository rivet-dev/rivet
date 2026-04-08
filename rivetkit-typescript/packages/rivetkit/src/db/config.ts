import type { ISqliteVfs } from "@rivetkit/sqlite-vfs";
import type { ActorMetrics } from "@/actor/metrics";

export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

export interface NativeSqliteConfig {
	endpoint: string;
	token?: string;
	namespace: string;
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

	/**
	 * KV operations for constructing KV-backed database clients
	 */
	kv: {
		batchPut: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
		batchGet: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
		batchDelete: (keys: Uint8Array[]) => Promise<void>;
		deleteRange: (start: Uint8Array, end: Uint8Array) => Promise<void>;
	};

	/**
	 * SQLite VFS handle for creating KV-backed databases.
	 * May be a standalone VFS or a pooled handle from SqliteVfsPool.
	 */
	sqliteVfs?: ISqliteVfs;

	/**
	 * Actor metrics instance. When provided, KV and SQL operations are tracked.
	 */
	metrics?: ActorMetrics;

	/**
	 * Preloaded SQLite KV entries for VFS read optimization during startup.
	 * When provided, database reads check these sorted entries via binary
	 * search before falling back to KV.
	 */
	preloadedEntries?: [Uint8Array, Uint8Array][];

	/**
	 * Logger for debug output. When provided, SQL queries are logged with
	 * duration and KV call count.
	 */
	log?: { debug(obj: Record<string, unknown>): void };

	/**
	 * Native SQLite channel configuration. When provided, the native addon
	 * connects to this explicit endpoint instead of reading process env.
	 */
	nativeSqliteConfig?: NativeSqliteConfig;
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
export type DrizzleDatabaseClient = {};

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
