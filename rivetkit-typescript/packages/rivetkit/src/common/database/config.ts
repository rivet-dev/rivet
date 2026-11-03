export type AnyDatabaseProvider = DatabaseProvider<any> | undefined;

export interface ActorMetricsLike {
	totalKvReads: number;
	totalKvWrites: number;
	trackSql(query: string, durationMs: number): void;
}

export type InferDatabaseClient<DBProvider extends AnyDatabaseProvider> =
	DBProvider extends DatabaseProvider<any>
		? Awaited<ReturnType<DBProvider["createClient"]>>
		: RawAccess;

export type SqliteBindings = unknown[] | Record<string, unknown>;

export interface SqliteQueryResult {
	columns: string[];
	rows: unknown[][];
}

export interface SqliteExecuteResult extends SqliteQueryResult {
	changes: number;
	lastInsertRowId?: number | null;
}

export interface SqliteBatchStatement {
	sql: string;
	params?: SqliteBindings;
}

export interface SqliteTransactionOptions {
	/** Static operation name used to aggregate transaction performance. */
	name?: string;
	/** Deadlock-safety timeout in milliseconds. */
	timeout?: number;
	/** @experimental */
	experimental?: {
		/**
		 * Atomically includes actor and hibernatable connection state.
		 * Only single-statement `execute` calls are supported in the transaction.
		 * Concurrent actions that try to mutate state while the transaction is
		 * active fail with `actor.state_transaction_conflict`.
		 */
		includeState?: boolean;
	};
}

/**
 * SQLite profiling configuration.
 *
 * @experimental This entire configuration surface is experimental and subject
 * to change without notice.
 */
export interface SqliteProfilingOptions {
	/** Exact SQL filter. Union with filterQueryFingerprints; both empty means all queries. */
	filterQueries?: string[];
	/** Fingerprint filter, including transaction fingerprints. Filters metrics, not diagnostic logs. */
	filterQueryFingerprints?: string[];
	/** Metric defaults apply even when omitted. Disabled metrics allocate no series. */
	metrics?: SqliteProfilingMetricsOptions;
	/** Duration buckets in seconds. Default [0.01, 0.1, 1]. Positive, finite, strictly increasing.
	 * All actors in a process enabling the duration histogram must use identical buckets. */
	durationHistogramBucketsSeconds?: number[];
	/** Maximum distinct statement fingerprints per process. Default: 512. */
	maxTrackedStatementFingerprints?: number;
	maxTrackedTransactionFingerprints?: number;
	maxPrometheusSeries?: number;
	maxStatementsPerTransactionTrace?: number;
	maxGetPagesRequestsPerTrace?: number;
	maxTransactionNameBytes?: number;
	slowOperationThresholdMs?: number;
	baselineSampleRate?: number;
	maxDiagnosticEventsPerMinute?: number;
	diagnosticEventQueueCapacity?: number;
}

/** SQLite profiling metrics. Physical-request metrics and gauges have no query labels. */
export interface SqliteProfilingMetricsOptions {
	/** Execution count and cumulative seconds. Uses histogram count/sum when enabled, otherwise counters. Default: true. */
	duration?: boolean;
	/** Execution duration histogram. Only applies when duration is enabled. Default: true. */
	durationHistogram?: boolean;
	/** Outcome counters, including success and errors. Default: true. */
	outcomes?: boolean;
	/** Per-query timing phase histograms. Default: false. */
	phaseDurationHistogram?: boolean;
	/** Storage round trips per operation histogram. Default: false. */
	getPagesRoundTripsHistogram?: boolean;
	/** Statements per transaction histogram. Default: false. */
	transactionStatementCountHistogram?: boolean;
	/** Per-query pages returned from storage, including returned prefetch pages. Default: true. */
	pagesFetched?: boolean;
	/** Actor-level page counters by page kind (no query labels). Default: false. */
	localPages?: boolean;
	/** Actor-level byte counters by byte kind (no query labels). Default: false. */
	localBytes?: boolean;
	/** Actor-level physical storage request duration histogram. Default: false. */
	getPagesDurationHistogram?: boolean;
	/** Actor-level pages per storage request histograms, by page kind. Default: false. */
	getPagesPagesHistogram?: boolean;
	/** Actor-level storage response byte histogram. Default: false. */
	getPagesResponseBytesHistogram?: boolean;
	/** Actor-level missing response page counter. Default: false. */
	getPagesMissingPages?: boolean;
	/** Queued SQLite work, aggregated by actor name. Default: true. */
	workerQueueDepth?: boolean;
	/** Executing SQLite work, aggregated by actor name. Default: true. */
	workerInflight?: boolean;
	/** Transaction waiters, aggregated by actor name. Default: true. */
	coordinatorQueueDepth?: boolean;
	/** Counts observations exceeding fingerprint or series limits. Default: true. */
	fingerprintOverflow?: boolean;
	/** Counts diagnostic events dropped by rate limits or backpressure. Default: true. */
	eventDropped?: boolean;
}

export interface SqliteNativeMetrics {
	requestBuildNs: number;
	serializeNs: number;
	transportNs: number;
	stateUpdateNs: number;
	totalNs: number;
	commitCount: number;
	pageCacheEntries: number;
	pageCacheWeightedSize: number;
	pageCacheCapacityPages: number;
	writeBufferDirtyPages: number;
	dbSizePages: number;
}

export interface SqliteDatabase {
	exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void>;
	execute(sql: string, params?: SqliteBindings): Promise<SqliteExecuteResult>;
	executeBatch(
		statements: SqliteBatchStatement[],
	): Promise<SqliteExecuteResult[]>;
	beginTransaction(
		timeoutMs?: number,
		name?: string,
	): Promise<SqliteTransactionDatabase>;
	run(sql: string, params?: SqliteBindings): Promise<void>;
	query(sql: string, params?: SqliteBindings): Promise<SqliteQueryResult>;
	nativeMetrics?():
		| SqliteNativeMetrics
		| Promise<SqliteNativeMetrics | null>
		| null;
	close(): Promise<void>;
}

export interface SqliteTransactionDatabase {
	exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void>;
	execute(sql: string, params?: SqliteBindings): Promise<SqliteExecuteResult>;
	commit(): Promise<void>;
	rollback(): Promise<void>;
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
	metrics?: ActorMetricsLike;

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
	 * Local SQLite profiling controls consumed by the actor runtime.
	 *
	 * @experimental This entire configuration surface is experimental and
	 * subject to change without notice.
	 */
	sqliteProfiling?: SqliteProfilingOptions;
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
	/** Runs a callback in an isolated SQLite transaction. */
	transaction: <T>(
		callback: (tx: RawAccess) => Promise<T> | T,
		options?: SqliteTransactionOptions,
	) => Promise<T>;
	/**
	 * Returns native SQLite metrics when the active runtime supports them.
	 */
	nativeMetrics?: () =>
		| SqliteNativeMetrics
		| Promise<SqliteNativeMetrics | null>
		| null;
	/**
	 * Closes the database connection and releases resources.
	 */
	close: () => Promise<void>;
};
