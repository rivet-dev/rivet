import type { ActorMetrics } from "@/actor/metrics";

export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

export type SqliteBindings = unknown[] | Record<string, unknown>;

export interface SqliteQueryResult {
	columns: string[];
	rows: unknown[][];
}

export interface SqliteVfsReadTelemetry {
	count: number;
	durationUs: number;
	requestedBytes: number;
	returnedBytes: number;
	shortReadCount: number;
}

export interface SqliteVfsWriteTelemetry {
	count: number;
	durationUs: number;
	inputBytes: number;
	bufferedCount: number;
	bufferedBytes: number;
	immediateKvPutCount: number;
	immediateKvPutBytes: number;
}

export interface SqliteVfsSyncTelemetry {
	count: number;
	durationUs: number;
	metadataFlushCount: number;
	metadataFlushBytes: number;
}

export interface SqliteVfsAtomicWriteTelemetry {
	beginCount: number;
	commitAttemptCount: number;
	commitSuccessCount: number;
	commitDurationUs: number;
	committedDirtyPagesTotal: number;
	maxCommittedDirtyPages: number;
	committedBufferedBytesTotal: number;
	rollbackCount: number;
	fastPathAttemptCount?: number;
	fastPathSuccessCount?: number;
	fastPathFallbackCount?: number;
	fastPathFailureCount?: number;
	batchCapFailureCount: number;
	commitKvPutFailureCount: number;
}

export interface SqliteVfsKvTelemetry {
	getCount: number;
	getDurationUs: number;
	getKeyCount: number;
	getBytes: number;
	putCount: number;
	putDurationUs: number;
	putKeyCount: number;
	putBytes: number;
	deleteCount: number;
	deleteDurationUs: number;
	deleteKeyCount: number;
	deleteRangeCount: number;
	deleteRangeDurationUs: number;
}

export interface SqliteVfsTelemetry {
	reads: SqliteVfsReadTelemetry;
	writes: SqliteVfsWriteTelemetry;
	syncs: SqliteVfsSyncTelemetry;
	atomicWrite: SqliteVfsAtomicWriteTelemetry;
	kv: SqliteVfsKvTelemetry;
}

export interface SqliteDatabase {
	exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void>;
	run(sql: string, params?: SqliteBindings): Promise<void>;
	query(sql: string, params?: SqliteBindings): Promise<SqliteQueryResult>;
	resetVfsTelemetry?(): Promise<void>;
	snapshotVfsTelemetry?(): Promise<SqliteVfsTelemetry>;
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
	resetVfsTelemetry?: () => Promise<void>;
	snapshotVfsTelemetry?: () => Promise<SqliteVfsTelemetry>;
	/**
	 * Closes the database connection and releases resources.
	 */
	close: () => Promise<void>;
};
