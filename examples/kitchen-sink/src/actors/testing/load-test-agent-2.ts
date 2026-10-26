import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

type AgentConcurrent2Request =
	| { type: "agent2_resume"; version: number }
	| {
			type: "agent2_connect";
			clientId: string;
			staggerHandleMs?: number;
			growBytes?: number;
			maxDbBytes?: number;
	  }
	| { type: "agent2_read_recent"; limit: number }
	| {
			type: "agent2_insert_messages";
			clientId: string;
			count: number;
			messageBytes: number;
	  }
	| { type: "agent2_stage"; stage: AgentConcurrent2Stage }
	| ({ type: "agent2_iterate" } & AgentIterationOptions)
	| { type: "force_sleep" }
	| { type: "ping"; id?: number };

/**
 * Single-actor latency stages that mirror the system sim's wake ladder: 50 rows at random
 * places, the 50 oldest rows, a full scan of every message, and one 16 MiB transaction that
 * overwrites contiguous messages in place so the database keeps its size. The large-database
 * stages add a 100 MiB transaction of the same shape (under the 128 MiB transaction cap), an
 * index build over every message (the index is dropped again in its own statement after it is
 * timed) and a `VACUUM` of the whole database.
 */
type AgentConcurrent2Stage =
	| "random"
	| "oldest"
	| "scan"
	| "txn"
	| "txn_large"
	| "index"
	| "vacuum"
	| AgentConcurrent2BenchStage;

const STAGE_ROWS = 50;
const STAGE_TXN_BYTES = 16 * 1024 * 1024;
const STAGE_TXN_LARGE_BYTES = 100 * 1024 * 1024;
const LARGE_DB_STAGES: readonly AgentConcurrent2Stage[] = [
	"txn_large",
	"index",
	"vacuum",
];

/**
 * Stages with the August native SQLite suite's workload shapes, on their own `bench_rows`
 * table of 4,096-byte payloads (a leaf cell plus one overflow page per row): `bench_seed` grows
 * the table by up to 8 MiB per call until it holds `BENCH_ROWS` rows (512 MiB of file), and the others
 * time each operation inside the actor so the result excludes the client's round trip.
 */
type AgentConcurrent2BenchStage =
	| "bench_seed"
	| "bench_point"
	| "bench_adjacent"
	| "bench_range16"
	| "bench_scan"
	| "bench_autocommit"
	| "bench_batch";

const BENCH_STAGES: readonly AgentConcurrent2Stage[] = [
	"bench_seed",
	"bench_point",
	"bench_adjacent",
	"bench_range16",
	"bench_scan",
	"bench_autocommit",
	"bench_batch",
];

const BENCH_ROW_BYTES = 4096;
const BENCH_ROWS = 116_480;
const BENCH_SEED_ROWS_PER_CALL = 2048;
const BENCH_READ_OPS = 1000;
const BENCH_RANGE_OPS = 200;
const BENCH_WRITE_OPS = 100;
const BENCH_BATCH_OPS = 20;
const BENCH_BATCH_ROWS = 100;

interface AgentConcurrent2Step {
	name: string;
	durationMs: number;
	rowCount: number;
}

interface AgentConcurrent2QueryStats {
	total: number;
	reads: number;
	mutations: number;
	tx: number;
	other: number;
	rows: number;
	errors: number;
	slow: number;
	maxMs: number;
	maxStep: string;
	byOperation: Record<string, number>;
	byTable: Record<string, number>;
}

interface AgentConcurrent2StatsSnapshot {
	wakeIndex: number;
	actorIteration: number;
	wakeIteration: number;
	cycle: AgentConcurrent2QueryStats;
	wake: AgentConcurrent2QueryStats;
	actor: AgentConcurrent2QueryStats;
}

interface AgentConcurrent2WorkloadResult {
	name: string;
	totalMs: number;
	steps: AgentConcurrent2Step[];
}

interface AgentConcurrent2ResultMessage {
	type: "agent2_result";
	trigger: AgentConcurrent2Request["type"];
	totalMs: number;
	results: AgentConcurrent2WorkloadResult[];
	stats: AgentConcurrent2StatsSnapshot;
	/** Database size (`page_count * page_size`) after the cycle. */
	dbBytes: number;
	/** Approximate payload bytes appended by thread growth this cycle. */
	grewBytes: number;
}

interface AgentConcurrent2GrowthOptions {
	growBytes: number;
	maxDbBytes: number;
}

interface AgentConcurrent2RunOptions {
	growBytes?: number;
	maxDbBytes?: number;
}

interface AgentConcurrent2ErrorMessage {
	type: "agent2_error";
	trigger: AgentConcurrent2Request["type"] | "unknown";
	error: string;
	stats?: AgentConcurrent2StatsSnapshot;
}

interface AgentConcurrent2Vars {
	sql: AgentConcurrent2Db | null;
	wakeStats: AgentConcurrent2QueryStats | null;
	wakeStartedAt: number | null;
	wakeIteration: number;
	iterationSchemaReady?: boolean;
}

interface RawRivetDB {
	execute: (
		query: string,
		...args: unknown[]
	) => Promise<Record<string, unknown>[]>;
}

type SQLPrimitive = string | number | boolean | null;

interface AgentConcurrent2State {
	runCount: number;
	wakeCount: number;
	queryStats: AgentConcurrent2QueryStats;
}

interface AgentConcurrent2QueryStatsSet {
	cycle: AgentConcurrent2QueryStats;
	wake: AgentConcurrent2QueryStats;
	actor: AgentConcurrent2QueryStats;
}

interface AgentConcurrent2Runtime {
	sql: AgentConcurrent2Db;
	wakeStats: AgentConcurrent2QueryStats;
	vars: AgentConcurrent2Vars;
}

type AgentConcurrent2Db = (<T = Record<string, SQLPrimitive>>(
	query: string,
	...values: SQLPrimitive[]
) => Promise<T[]>) & {
	withTransaction<T>(
		stats: AgentConcurrent2QueryStatsSet,
		fn: (tx: AgentConcurrent2Db) => Promise<T>,
	): Promise<T>;
};

class AsyncMutex {
	private locked = false;
	private waiters: Array<() => void> = [];

	async acquire(): Promise<void> {
		if (!this.locked) {
			this.locked = true;
			return;
		}
		await new Promise<void>((resolve) => this.waiters.push(resolve));
		this.locked = true;
	}

	release(): void {
		const next = this.waiters.shift();
		if (next) {
			next();
			return;
		}
		this.locked = false;
	}
}

function createSerializedDb(
	execute: <T = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	) => Promise<T[]>,
): AgentConcurrent2Db {
	const mutex = new AsyncMutex();
	let activeTransaction: AgentConcurrent2Db | null = null;

	const createTransactionDb = (): AgentConcurrent2Db => {
		const tx = Object.assign(
			<T = Record<string, SQLPrimitive>>(
				query: string,
				...values: SQLPrimitive[]
			) => execute<T>(query, ...values),
			{
				withTransaction: async <T>(
					_stats: AgentConcurrent2QueryStatsSet,
					fn: (tx: AgentConcurrent2Db) => Promise<T>,
				): Promise<T> => fn(tx),
			},
		);
		return tx;
	};

	const queryWithMutex = async <T = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	): Promise<T[]> => {
		await mutex.acquire();
		try {
			return await execute<T>(query, ...values);
		} finally {
			mutex.release();
		}
	};

	return Object.assign(queryWithMutex, {
		withTransaction: async <T>(
			stats: AgentConcurrent2QueryStatsSet,
			fn: (tx: AgentConcurrent2Db) => Promise<T>,
		): Promise<T> => {
			if (activeTransaction) {
				return fn(activeTransaction);
			}
			await mutex.acquire();
			const tx = createTransactionDb();
			try {
				await executeTrackedQuery(
					execute,
					stats,
					"transaction-begin",
					"BEGIN",
				);
				activeTransaction = tx;
				try {
					const result = await fn(tx);
					activeTransaction = null;
					await executeTrackedQuery(
						execute,
						stats,
						"transaction-commit",
						"COMMIT",
					);
					return result;
				} catch (error) {
					activeTransaction = null;
					await executeTrackedQuery(
						execute,
						stats,
						"transaction-rollback",
						"ROLLBACK",
					);
					throw error;
				}
			} finally {
				activeTransaction = null;
				mutex.release();
			}
		},
	});
}

const MESSAGE_COUNT = 84;
const MESSAGE_TOOL_REF_COUNT = 122;
const TOOL_CALL_COUNT = 61;
const EXECUTOR_TOOL_COUNT = 42;
const THREAD_EVENT_COUNT = 233;

const MESSAGE_CONTENT_BYTES = 2_600;
const THREAD_EVENT_PAYLOAD_BYTES = 1_000;
const TOOL_CALL_RESULT_BYTES = 2_700;
const EXECUTOR_TOOL_SCHEMA_BYTES = 550;
const SLOW_QUERY_MS = 1_000;

/** Default cap on thread growth. Growth stops once the database reaches it. */
const DEFAULT_MAX_DB_BYTES = 52_428_800;
/** Upper bound on payload bytes appended inside one growth transaction. */
const GROW_TRANSACTION_BYTES = 1_048_576;
/** Rows per multi-row INSERT statement during growth. */
const GROW_INSERT_BATCH_ROWS = 50;
/** Thread events replayed to a reconnecting client, newest first. */
const CATCHUP_EVENT_LIMIT = 256;
/** Recent top-level assistant messages checked for unresolved tool calls. */
const UNRESOLVED_MESSAGE_WINDOW = 50;
/** Most messages one `agent2_read_recent` may return. */
const MAX_READ_RECENT_LIMIT = 1_000;
/** Most messages one `agent2_insert_messages` may insert. */
const MAX_INSERT_MESSAGES = 10_000;
/** Largest message content one `agent2_insert_messages` may write. */
const MAX_INSERT_MESSAGE_BYTES = 1_048_576;
/** Payload bytes in one grown turn: three messages, one tool result, and four events. */
const GROW_TURN_PAYLOAD_BYTES =
	3 * MESSAGE_CONTENT_BYTES +
	TOOL_CALL_RESULT_BYTES +
	4 * THREAD_EVENT_PAYLOAD_BYTES;

function send(
	websocket: UniversalWebSocket,
	message:
		| AgentConcurrent2ResultMessage
		| AgentConcurrent2ErrorMessage
		| object,
): void {
	if (websocket.readyState === 1) {
		websocket.send(JSON.stringify(message));
	}
}

export const loadTestAgent2 = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	state: {
		runCount: 0,
		wakeCount: 0,
		queryStats: createAgentConcurrent2QueryStats(),
	} as AgentConcurrent2State,
	db: db({
		onMigrate: async (database) => {
			await createAgentConcurrent2Schema(database);
			await seedAgentConcurrent2Data(database);
		},
	}),
	vars: {
		sql: null,
		wakeStats: null,
		wakeStartedAt: null,
		wakeIteration: 0,
	} as AgentConcurrent2Vars,
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		send(websocket, {
			type: "connected",
			actorId: c.actorId,
			timestamp: Date.now(),
		});

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			const promise = handleAgentConcurrent2Message(
				c,
				websocket,
				event.data,
			);
			void c.keepAwake(promise);
		});
	},
	actions: {
		run: async (
			c,
			clientId?: string,
			options?: AgentConcurrent2RunOptions,
		) => {
			const runtime = ensureAgentConcurrent2Runtime(c);
			c.state.runCount++;
			runtime.vars.wakeIteration++;
			const cycleStats = createAgentConcurrent2QueryStats();
			const stats = createAgentConcurrent2StatsSet(
				cycleStats,
				runtime.wakeStats,
				c.state.queryStats,
			);
			const result = await runAgentConcurrent2Workload(
				runtime.sql,
				clientId ?? `agent2-action-${c.state.runCount}`,
				0,
				parseGrowthOptions((options ?? {}) as Record<string, unknown>),
				stats,
			);
			return {
				...result,
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
			};
		},
		getRunCount: (c) => c.state.runCount,
		sleep: (c) => {
			c.sleep();
			return true;
		},
	},
});

async function handleAgentConcurrent2Message(
	c: {
		db: RawRivetDB;
		vars: AgentConcurrent2Vars;
		state: AgentConcurrent2State;
		sleep: () => void;
		saveState: (opts: { immediate?: boolean }) => Promise<void>;
	},
	websocket: UniversalWebSocket,
	data: unknown,
): Promise<void> {
	let trigger: AgentConcurrent2Request["type"] | "unknown" = "unknown";
	let cycleStats: AgentConcurrent2QueryStats | null = null;
	try {
		const request = parseAgentConcurrent2Request(data);
		trigger = request.type;

		if (request.type === "ping") {
			send(websocket, {
				type: "pong",
				id: request.id,
				timestamp: Date.now(),
			});
			return;
		}

		if (request.type === "force_sleep") {
			send(websocket, { type: "sleeping", timestamp: Date.now() });
			c.sleep();
			return;
		}

		const runtime = ensureAgentConcurrent2Runtime(c);
		c.state.runCount++;
		runtime.vars.wakeIteration++;
		cycleStats = createAgentConcurrent2QueryStats();
		const stats = createAgentConcurrent2StatsSet(
			cycleStats,
			runtime.wakeStats,
			c.state.queryStats,
		);

		if (request.type === "agent2_iterate") {
			if (!c.vars.iterationSchemaReady) {
				await ensureIterationSchema(c.db);
				c.vars.iterationSchemaReady = true;
			}
			const result = await runAgentIteration(
				runtime.sql,
				request,
				stats,
				async () => {
					c.state.runCount++;
					await c.saveState({ immediate: true });
				},
			);
			send(websocket, {
				...result,
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
			});
			return;
		}

		if (request.type === "agent2_read_recent") {
			const startedAt = performance.now();
			const read = await runReadRecentMessages(
				runtime.sql,
				request.limit,
				stats,
			);
			send(websocket, {
				type: "agent2_read_result",
				rows: read.rows,
				bytes: read.bytes,
				readMs: performance.now() - startedAt,
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
			});
			return;
		}

		if (request.type === "agent2_stage") {
			const startedAt = performance.now();
			const result = await runStage(runtime.sql, request.stage, stats);
			send(websocket, {
				type: "agent2_stage_result",
				stage: request.stage,
				rows: result.rows,
				bytes: result.bytes,
				opMs: result.opMs ?? null,
				stageMs: performance.now() - startedAt,
			});
			return;
		}

		if (request.type === "agent2_insert_messages") {
			const startedAt = performance.now();
			const inserted = await runInsertMessages(
				runtime.sql,
				request.clientId,
				request.count,
				request.messageBytes,
				stats,
			);
			const insertMs = performance.now() - startedAt;
			const dbBytes = await readDbBytes(runtime.sql);
			send(websocket, {
				type: "agent2_insert_result",
				inserted: inserted.messages,
				bytes: inserted.bytes,
				transactions: inserted.transactions,
				insertMs,
				dbBytes,
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
			});
			return;
		}

		if (request.type === "agent2_resume") {
			const startedAt = performance.now();
			const result = await runCatchupSnapshot(
				runtime.sql,
				request.version,
				stats,
			);
			const dbBytes = await readDbBytes(runtime.sql);
			send(websocket, {
				type: "agent2_result",
				trigger: request.type,
				totalMs: Math.round(performance.now() - startedAt),
				results: [result],
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
				dbBytes,
				grewBytes: 0,
			});
			return;
		}

		const result = await runAgentConcurrent2Workload(
			runtime.sql,
			request.clientId,
			request.staggerHandleMs ?? 0,
			{
				growBytes: request.growBytes ?? 0,
				maxDbBytes: request.maxDbBytes ?? DEFAULT_MAX_DB_BYTES,
			},
			stats,
		);
		send(websocket, {
			type: "agent2_result",
			trigger: request.type,
			...result,
			stats: snapshotAgentConcurrent2Stats(c, cycleStats),
		});
	} catch (error) {
		send(websocket, {
			type: "agent2_error",
			trigger,
			error: error instanceof Error ? error.message : String(error),
			...(cycleStats
				? { stats: snapshotAgentConcurrent2Stats(c, cycleStats) }
				: {}),
		});
	}
}

function parseAgentConcurrent2Request(data: unknown): AgentConcurrent2Request {
	if (typeof data !== "string") {
		throw new Error("agent concurrent 2 request must be a string");
	}
	const parsed = JSON.parse(data) as unknown;
	if (!parsed || typeof parsed !== "object") {
		throw new Error("agent concurrent 2 request must be an object");
	}
	const request = parsed as Record<string, unknown>;

	if (request.type === "ping") {
		return {
			type: "ping",
			...(typeof request.id === "number" ? { id: request.id } : {}),
		};
	}
	if (request.type === "force_sleep") {
		return { type: "force_sleep" };
	}
	if (request.type === "agent2_resume") {
		return {
			type: "agent2_resume",
			version: numberField(request, "version"),
		};
	}
	if (request.type === "agent2_stage") {
		const stage = request.stage;
		if (
			stage !== "random" &&
			stage !== "oldest" &&
			stage !== "scan" &&
			stage !== "txn" &&
			!LARGE_DB_STAGES.includes(stage) &&
			!BENCH_STAGES.includes(stage)
		) {
			throw new Error(
				`agent concurrent 2 stage must be random, oldest, scan, txn, ${LARGE_DB_STAGES.join(", ")} or one of ${BENCH_STAGES.join(", ")}`,
			);
		}
		return { type: "agent2_stage", stage };
	}
	if (request.type === "agent2_read_recent") {
		const limit = numberField(request, "limit");
		if (
			!Number.isInteger(limit) ||
			limit < 0 ||
			limit > MAX_READ_RECENT_LIMIT
		) {
			throw new Error(
				`agent concurrent 2 request limit must be an integer from 0 to ${MAX_READ_RECENT_LIMIT}`,
			);
		}
		return { type: "agent2_read_recent", limit };
	}
	if (request.type === "agent2_insert_messages") {
		const count = numberField(request, "count");
		const messageBytes = numberField(request, "messageBytes");
		if (
			!Number.isInteger(count) ||
			count < 0 ||
			count > MAX_INSERT_MESSAGES
		) {
			throw new Error(
				`agent concurrent 2 request count must be an integer from 0 to ${MAX_INSERT_MESSAGES}`,
			);
		}
		if (
			!Number.isInteger(messageBytes) ||
			messageBytes < 1 ||
			messageBytes > MAX_INSERT_MESSAGE_BYTES
		) {
			throw new Error(
				`agent concurrent 2 request messageBytes must be an integer from 1 to ${MAX_INSERT_MESSAGE_BYTES}`,
			);
		}
		return {
			type: "agent2_insert_messages",
			clientId: stringField(request, "clientId"),
			count,
			messageBytes,
		};
	}
	if (request.type === "agent2_iterate") {
		return { type: "agent2_iterate", ...parseIterationOptions(request) };
	}
	if (request.type === "agent2_connect") {
		return {
			type: "agent2_connect",
			clientId: stringField(request, "clientId"),
			...(typeof request.staggerHandleMs === "number"
				? { staggerHandleMs: request.staggerHandleMs }
				: {}),
			...parseGrowthOptions(request),
		};
	}
	throw new Error(
		`unknown agent concurrent 2 request type: ${String(request.type)}`,
	);
}

/** Validates the optional growth knobs shared by `agent2_connect` and the `run` action. */
function parseGrowthOptions(
	record: Record<string, unknown>,
): AgentConcurrent2GrowthOptions {
	const growBytes = optionalNumberField(record, "growBytes") ?? 0;
	const maxDbBytes =
		optionalNumberField(record, "maxDbBytes") ?? DEFAULT_MAX_DB_BYTES;
	if (growBytes < 0) {
		throw new Error("agent concurrent 2 request growBytes must be >= 0");
	}
	if (maxDbBytes <= 0) {
		throw new Error("agent concurrent 2 request maxDbBytes must be > 0");
	}
	return { growBytes, maxDbBytes };
}

function optionalNumberField(
	record: Record<string, unknown>,
	field: string,
): number | undefined {
	if (record[field] === undefined || record[field] === null) {
		return undefined;
	}
	return numberField(record, field);
}

function stringField(record: Record<string, unknown>, field: string): string {
	const value = record[field];
	if (typeof value !== "string" || value.length === 0) {
		throw new Error(`agent concurrent 2 request ${field} must be a string`);
	}
	return value;
}

function numberField(record: Record<string, unknown>, field: string): number {
	const value = record[field];
	if (typeof value !== "number" || !Number.isFinite(value)) {
		throw new Error(
			`agent concurrent 2 request ${field} must be a finite number`,
		);
	}
	return value;
}

function createAgentConcurrent2Db(db: RawRivetDB): AgentConcurrent2Db {
	return createSerializedDb(
		async <T = Record<string, SQLPrimitive>>(
			query: string,
			...values: SQLPrimitive[]
		): Promise<T[]> => {
			const converted = values.map((value) =>
				typeof value === "boolean" ? (value ? 1 : 0) : value,
			);
			return (await db.execute(query, ...converted)) as T[];
		},
	);
}

function ensureAgentConcurrent2Runtime(c: {
	db: RawRivetDB;
	vars: AgentConcurrent2Vars;
	state: AgentConcurrent2State;
}): AgentConcurrent2Runtime {
	c.vars.sql ??= createAgentConcurrent2Db(c.db);
	c.state.queryStats ??= createAgentConcurrent2QueryStats();
	c.state.wakeCount ??= 0;
	if (!c.vars.wakeStats) {
		c.vars.wakeStats = createAgentConcurrent2QueryStats();
		c.vars.wakeStartedAt = Date.now();
		c.vars.wakeIteration = 0;
		c.state.wakeCount++;
	}
	return {
		sql: c.vars.sql,
		wakeStats: c.vars.wakeStats,
		vars: c.vars,
	};
}

function createAgentConcurrent2QueryStats(): AgentConcurrent2QueryStats {
	return {
		total: 0,
		reads: 0,
		mutations: 0,
		tx: 0,
		other: 0,
		rows: 0,
		errors: 0,
		slow: 0,
		maxMs: 0,
		maxStep: "",
		byOperation: {},
		byTable: {},
	};
}

function createAgentConcurrent2StatsSet(
	cycle: AgentConcurrent2QueryStats,
	wake: AgentConcurrent2QueryStats,
	actor: AgentConcurrent2QueryStats,
): AgentConcurrent2QueryStatsSet {
	return { cycle, wake, actor };
}

function snapshotAgentConcurrent2Stats(
	c: { vars: AgentConcurrent2Vars; state: AgentConcurrent2State },
	cycle: AgentConcurrent2QueryStats,
): AgentConcurrent2StatsSnapshot {
	return {
		wakeIndex: c.state.wakeCount,
		actorIteration: c.state.runCount,
		wakeIteration: c.vars.wakeIteration,
		cycle: cloneAgentConcurrent2QueryStats(cycle),
		wake: cloneAgentConcurrent2QueryStats(
			c.vars.wakeStats ?? createAgentConcurrent2QueryStats(),
		),
		actor: cloneAgentConcurrent2QueryStats(c.state.queryStats),
	};
}

function cloneAgentConcurrent2QueryStats(
	stats: AgentConcurrent2QueryStats,
): AgentConcurrent2QueryStats {
	return {
		total: stats.total,
		reads: stats.reads,
		mutations: stats.mutations,
		tx: stats.tx,
		other: stats.other,
		rows: stats.rows,
		errors: stats.errors,
		slow: stats.slow,
		maxMs: stats.maxMs,
		maxStep: stats.maxStep,
		byOperation: { ...stats.byOperation },
		byTable: { ...stats.byTable },
	};
}

async function runAgentConcurrent2Workload(
	sql: AgentConcurrent2Db,
	clientId: string,
	staggerHandleMs: number,
	growth: AgentConcurrent2GrowthOptions,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<Omit<AgentConcurrent2ResultMessage, "type" | "trigger" | "stats">> {
	const startedAt = performance.now();
	const buildToolPlanContext = runBuildToolPlanContext(sql, stats);
	const catchupSnapshot = runCatchupSnapshot(sql, null, stats);
	const recoverToolCalls = runRecoverToolCalls(sql, stats);
	const mutationMix = runMutationMix(sql, clientId, stats);
	const handleExecutorConnect = delay(staggerHandleMs).then(() =>
		runHandleClientConnect(sql, clientId, stats),
	);

	const results = await Promise.all([
		handleExecutorConnect,
		buildToolPlanContext,
		catchupSnapshot,
		recoverToolCalls,
		mutationMix,
	]);

	// Thread growth runs after the normal workload so the read mix above
	// measures the database as it stood when the client connected.
	let grewBytes = 0;
	if (growth.growBytes > 0) {
		const grown = await runGrowThread(sql, clientId, growth, stats);
		grewBytes = grown.grewBytes;
		results.push(grown.result);
	}
	const dbBytes = await readDbBytes(sql);

	return {
		totalMs: Math.round(performance.now() - startedAt),
		results,
		dbBytes,
		grewBytes,
	};
}

async function runHandleClientConnect(
	sql: AgentConcurrent2Db,
	clientId: string,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	const nextSeq = await sql.withTransaction(stats, async (tx) => {
		const latestExecutor = await timedQuery(
			tx,
			stats,
			steps,
			"load-latest-executor-id",
			`SELECT executor_id FROM executor_tools ORDER BY updated_at DESC LIMIT 1`,
		);
		const latestExecutorId = String(
			latestExecutor[0]?.executor_id ?? "seed-executor",
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"select-cached-executor-tools",
			`SELECT tool_name, schema FROM executor_tools WHERE executor_id = ? ORDER BY tool_name ASC`,
			latestExecutorId,
		);
		const executorType = await timedQuery(
			tx,
			stats,
			steps,
			"select-executor-type",
			`SELECT value FROM thread_meta_kv WHERE key = 'executor_type'`,
		);
		if (!executorType[0]?.value) {
			await timedQuery(
				tx,
				stats,
				steps,
				"set-executor-type",
				`INSERT OR REPLACE INTO thread_meta_kv (key, value, updated_at) VALUES ('executor_type', ?, ?)`,
				"local-client",
				new Date().toISOString(),
			);
		}
		const sandboxIntent = await timedQuery(
			tx,
			stats,
			steps,
			"select-workspace-intent",
			`SELECT value FROM thread_meta_kv WHERE key = 'workspace_intent'`,
		);
		if (hasPendingLaunch(sandboxIntent[0]?.value)) {
			await timedQuery(
				tx,
				stats,
				steps,
				"clear-pending-launch",
				`UPDATE thread_meta_kv SET value = ?, updated_at = ? WHERE key = 'workspace_intent'`,
				JSON.stringify({ spec: null, pendingLaunch: null }),
				new Date().toISOString(),
			);
		}
		const seqRows = await timedQuery(
			tx,
			stats,
			steps,
			"select-next-thread-event-seq",
			`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
		);
		const seq = Number(seqRows[0]?.seq ?? 1);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-client-connected-event",
			`INSERT INTO thread_events (seq, event_type, payload, created_at) VALUES (?, ?, ?, ?)`,
			seq,
			"client_connected",
			JSON.stringify({ type: "client_connected", clientId }),
			new Date().toISOString(),
		);
		return seq;
	});
	steps.push({
		name: "transaction-total",
		durationMs: Math.round(performance.now() - startedAt),
		rowCount: nextSeq,
	});
	return {
		name: "handle-client-connect",
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	};
}

async function runBuildToolPlanContext(
	sql: AgentConcurrent2Db,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	const latestExecutor = await timedQuery(
		sql,
		stats,
		steps,
		"load-latest-executor-id",
		`SELECT executor_id FROM executor_tools ORDER BY updated_at DESC LIMIT 1`,
	);
	const latestExecutorId = String(
		latestExecutor[0]?.executor_id ?? "seed-executor",
	);
	await timedQuery(
		sql,
		stats,
		steps,
		"select-executor-tools",
		`SELECT tool_name, schema FROM executor_tools WHERE executor_id = ? ORDER BY tool_name ASC`,
		latestExecutorId,
	);
	await timedQuery(
		sql,
		stats,
		steps,
		"count-uncancelled-top-level",
		`SELECT COUNT(*) as count FROM messages WHERE cancelled = 0 AND parent_tool_use_id IS NULL`,
	);
	const unresolvedRows = await timedQuery(
		sql,
		stats,
		steps,
		"find-unresolved-assistant-message",
		`SELECT m.*
			FROM message_tool_refs AS tool_use
			JOIN (
				SELECT * FROM messages
				WHERE parent_tool_use_id IS NULL
					AND role = 'assistant'
					AND cancelled = 0
				ORDER BY created_at DESC
				LIMIT ?
			) AS m
				ON m.message_id = tool_use.assistant_message_id
			WHERE tool_use.block_type = 'tool_use'
				AND tool_use.cancelled = 0
				AND NOT EXISTS (
					SELECT 1
					FROM message_tool_refs AS tool_result
					JOIN messages AS tool_result_message
						ON tool_result_message.message_id = tool_result.source_message_id
					WHERE tool_result.assistant_message_id = tool_use.assistant_message_id
						AND tool_result.block_type = 'tool_result'
						AND tool_result.cancelled = 0
						AND tool_result.tool_use_id = tool_use.tool_use_id
						AND tool_result_message.parent_tool_use_id IS NULL
				)
			GROUP BY m.message_id
			ORDER BY m.created_at DESC
			LIMIT 1`,
		UNRESOLVED_MESSAGE_WINDOW,
	);
	const unresolvedMessageId = unresolvedRows[0]?.message_id;
	if (typeof unresolvedMessageId === "string") {
		await timedQuery(
			sql,
			stats,
			steps,
			"get-persisted-tool-result-ids",
			`SELECT tool_result.tool_use_id
				FROM message_tool_refs AS tool_result
				JOIN messages AS tool_result_message
					ON tool_result_message.message_id = tool_result.source_message_id
				WHERE tool_result.assistant_message_id = ?
					AND tool_result.block_type = 'tool_result'
					AND tool_result.cancelled = 0
					AND tool_result_message.parent_tool_use_id IS NULL`,
			unresolvedMessageId,
		);
		await timedQuery(
			sql,
			stats,
			steps,
			"get-tool-calls-by-message-id",
			`SELECT * FROM tool_calls WHERE message_id = ?`,
			unresolvedMessageId,
		);
	}
	await timedQuery(
		sql,
		stats,
		steps,
		"is-last-message-cancelled-assistant",
		`SELECT role, cancelled FROM messages
			WHERE parent_tool_use_id IS NULL
			ORDER BY created_at DESC
			LIMIT 1`,
	);
	await timedQuery(
		sql,
		stats,
		steps,
		"get-last-uncancelled",
		`SELECT m.* FROM messages m
			WHERE m.cancelled = 0 AND m.parent_tool_use_id IS NULL
			ORDER BY m.created_at DESC
			LIMIT 1`,
	);
	return {
		name: "build-tool-plan-context",
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	};
}

/**
 * Loads the catchup snapshot for a client. A `version` resumes from that seq
 * one page at a time. `null` is a fresh connect and replays only the newest
 * events, as a client would before paging older history on demand.
 */
async function runCatchupSnapshot(
	sql: AgentConcurrent2Db,
	version: number | null,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	const threadEventsQuery =
		version === null
			? timedQuery(
					sql,
					stats,
					steps,
					"thread-events-list-since-version",
					`SELECT seq, event_type, payload, created_at FROM (
						SELECT seq, event_type, payload, created_at FROM thread_events ORDER BY seq DESC LIMIT ?
					) ORDER BY seq ASC`,
					CATCHUP_EVENT_LIMIT,
				)
			: timedQuery(
					sql,
					stats,
					steps,
					"thread-events-list-since-version",
					`SELECT seq, event_type, payload, created_at FROM thread_events WHERE seq > ? ORDER BY seq ASC LIMIT ?`,
					version,
					CATCHUP_EVENT_LIMIT,
				);
	await Promise.all([
		threadEventsQuery,
		timedQuery(
			sql,
			stats,
			steps,
			"environment-snapshot",
			`SELECT snapshot FROM environment_snapshot WHERE id = 1`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"thread-settings-snapshot",
			`SELECT settings FROM thread_settings_snapshot WHERE id = 1`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"retry-state",
			`SELECT * FROM retry_state WHERE id = 1`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"queued-messages",
			`SELECT * FROM queued_messages ORDER BY created_at ASC`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"executor-artifacts",
			`SELECT artifact_key, data_type, length(content_base64) AS bytes, tool_call_id, updated_at FROM executor_artifacts ORDER BY updated_at ASC`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"tool-approvals",
			`SELECT * FROM tool_approvals ORDER BY timestamp ASC`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"compaction-summaries",
			`SELECT cut_message_id, created_at FROM compaction_summaries ORDER BY created_at ASC`,
		),
		timedQuery(
			sql,
			stats,
			steps,
			"executor-status",
			`SELECT value FROM thread_meta_kv WHERE key = 'executor_status'`,
		),
	]);
	steps.sort((a, b) => b.durationMs - a.durationMs);
	return {
		name: "catchup-snapshot",
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	};
}

async function runRecoverToolCalls(
	sql: AgentConcurrent2Db,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	await timedQuery(
		sql,
		stats,
		steps,
		"hydrate-tool-progress",
		`SELECT id, progress
			FROM tool_calls
			WHERE progress IS NOT NULL
				AND state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	);
	await timedQuery(
		sql,
		stats,
		steps,
		"get-pending-tool-calls",
		`SELECT * FROM tool_calls
			WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')
			ORDER BY issued_at ASC`,
	);
	await timedQuery(
		sql,
		stats,
		steps,
		"get-next-tool-expiry",
		`SELECT MIN(expires_at) AS expires_at
			FROM tool_calls
			WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	);
	return {
		name: "recover-tool-calls",
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	};
}

async function runMutationMix(
	sql: AgentConcurrent2Db,
	clientId: string,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	const writeCount = await sql.withTransaction(stats, async (tx) => {
		const now = new Date().toISOString();
		const suffix = safeId(clientId);
		const seqRows = await timedQuery(
			tx,
			stats,
			steps,
			"select-max-thread-event-seq",
			`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
		);
		const seq = Number(seqRows[0]?.seq ?? 1);
		const lastMessageRows = await timedQuery(
			tx,
			stats,
			steps,
			"select-last-message-created-at",
			`SELECT MAX(created_at) AS created_at FROM messages`,
		);
		const latestToolRows = await timedQuery(
			tx,
			stats,
			steps,
			"select-existing-tool-call",
			`SELECT id FROM tool_calls ORDER BY issued_at DESC LIMIT 1`,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"select-sandbox-row",
			`SELECT sandbox_id, restart_attempts, traffic_access_token, project_id, repository_url, additional_repositories, setup
				FROM e2b_sandbox
				WHERE id = 1`,
		);

		const messageIdValue = `agent2-message-${suffix}-${seq}`;
		const toolUseIdValue = `agent2-tool-${suffix}-${seq}`;
		const toolCallIdValue = `agent2-call-${suffix}-${seq}`;
		const latestToolCallId = String(latestToolRows[0]?.id ?? toolUseID(1));
		const lastCreatedAt = String(lastMessageRows[0]?.created_at ?? now);

		await timedQuery(
			tx,
			stats,
			steps,
			"upsert-agent-state",
			`INSERT OR REPLACE INTO thread_meta_kv (key, value, updated_at) VALUES (?, ?, ?)`,
			"last_agent_state",
			JSON.stringify({ status: "working", clientId, lastCreatedAt }),
			now,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-work-event",
			`INSERT INTO thread_events (seq, event_type, payload, created_at) VALUES (?, ?, ?, ?)`,
			seq,
			"message_added",
			JSON.stringify({
				type: "message_added",
				messageId: messageIdValue,
			}),
			now,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-message",
			`INSERT INTO messages (role, content, meta, user_state, message_id, created_at, cancelled, parent_tool_use_id, tool_result_for_message_id)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			"assistant",
			"agent concurrent 2 mutation payload",
			JSON.stringify({ clientId, seq }),
			null,
			messageIdValue,
			now,
			0,
			null,
			null,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"delete-message-tool-refs",
			`DELETE FROM message_tool_refs WHERE source_message_id = ?`,
			messageIdValue,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-message-added-event",
			`INSERT OR IGNORE INTO message_added_events (message_id, seq) VALUES (?, ?)`,
			messageIdValue,
			seq,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-message-tool-ref",
			`INSERT INTO message_tool_refs (source_message_id, assistant_message_id, tool_use_id, block_type, cancelled)
				VALUES (?, ?, ?, ?, ?)`,
			messageIdValue,
			messageIdValue,
			toolUseIdValue,
			"tool_use",
			0,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"insert-tool-call",
			`INSERT OR IGNORE INTO tool_calls (id, provider_tool_use_id, tool_name, args, executor_id, message_id, issued_at, expires_at, state, result, progress, completed_at)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			toolCallIdValue,
			`provider-${toolCallIdValue}`,
			"tool_1",
			JSON.stringify({ path: `/tmp/${toolCallIdValue}` }),
			"seed-executor",
			messageIdValue,
			now,
			null,
			"running",
			null,
			JSON.stringify({ pct: 0.5, clientId }),
			null,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"update-tool-call-progress",
			`UPDATE tool_calls SET progress = ? WHERE id = ? AND state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
			JSON.stringify({ pct: 0.75, clientId, updatedAt: now }),
			toolCallIdValue,
		);
		await timedQuery(
			tx,
			stats,
			steps,
			"update-existing-tool-call-progress",
			`UPDATE tool_calls SET progress = ? WHERE id = ? AND state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
			JSON.stringify({ pct: 0.25, clientId, updatedAt: now }),
			latestToolCallId,
		);

		return seq;
	});
	steps.push({
		name: "transaction-total",
		durationMs: Math.round(performance.now() - startedAt),
		rowCount: writeCount,
	});
	return {
		name: "mutation-mix",
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	};
}

/**
 * Reads the database size as `page_count * page_size`. These queries are
 * untracked so per-cycle query stats keep the same shape with or without
 * growth.
 */
async function readDbBytes(sql: AgentConcurrent2Db): Promise<number> {
	const pageCount = await sql<{ page_count: number }>(`PRAGMA page_count`);
	const pageSize = await sql<{ page_size: number }>(`PRAGMA page_size`);
	return (
		Number(pageCount[0]?.page_count ?? 0) *
		Number(pageSize[0]?.page_size ?? 0)
	);
}

/**
 * Appends roughly `growBytes` of resolved agent turns in transactions of at
 * most `GROW_TRANSACTION_BYTES`, stopping early once the database reaches
 * `maxDbBytes`.
 */
async function runGrowThread(
	sql: AgentConcurrent2Db,
	clientId: string,
	growth: AgentConcurrent2GrowthOptions,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<{ result: AgentConcurrent2WorkloadResult; grewBytes: number }> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	const nextText = createTextGenerator(hashString(clientId) ^ Date.now());
	let grewBytes = 0;
	let dbBytes = await readDbBytes(sql);

	while (grewBytes < growth.growBytes && dbBytes < growth.maxDbBytes) {
		const budget = Math.min(
			GROW_TRANSACTION_BYTES,
			growth.growBytes - grewBytes,
			growth.maxDbBytes - dbBytes,
		);
		const transactionStartedAt = performance.now();
		const appended = await sql.withTransaction(stats, (tx) =>
			appendThreadTurns(tx, clientId, budget, stats, nextText),
		);
		steps.push({
			name: "grow-transaction",
			durationMs: Math.round(performance.now() - transactionStartedAt),
			rowCount: appended.turns,
		});
		grewBytes += appended.bytes;
		dbBytes = await readDbBytes(sql);
	}

	return {
		result: {
			name: "grow-thread",
			totalMs: Math.round(performance.now() - startedAt),
			steps,
		},
		grewBytes,
	};
}

/**
 * Inserts enough complete turns to cover `budget` payload bytes. Each turn is
 * a user prompt, an assistant tool call with its completed result, and the
 * tool result message, with matching tool refs, thread events, and
 * message-added rows so the read queries see a consistent thread.
 */
async function appendThreadTurns(
	tx: AgentConcurrent2Db,
	clientId: string,
	budget: number,
	stats: AgentConcurrent2QueryStatsSet,
	nextText: (bytes: number) => string,
): Promise<{ turns: number; bytes: number }> {
	// Per-statement steps are dropped; the caller records one step per transaction.
	const steps: AgentConcurrent2Step[] = [];
	const seqRows = await timedQuery(
		tx,
		stats,
		steps,
		"grow-select-next-thread-event-seq",
		`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
	);
	let seq = Number(seqRows[0]?.seq ?? 1);
	const turnCount = Math.max(1, Math.ceil(budget / GROW_TURN_PAYLOAD_BYTES));
	const suffix = safeId(clientId);
	// Wall-clock timestamps keep grown rows ordered with the rows that later
	// cycles write. Rows inserted in the same millisecond share a timestamp.
	const nextTimestamp = () => new Date().toISOString();

	const messageRows: SQLPrimitive[][] = [];
	const toolRefRows: SQLPrimitive[][] = [];
	const toolCallRows: SQLPrimitive[][] = [];
	const eventRows: SQLPrimitive[][] = [];
	const messageAddedRows: SQLPrimitive[][] = [];
	let bytes = 0;

	const addEvent = (
		eventType: string,
		fields: Record<string, string>,
	): number => {
		const payload = JSON.stringify({
			type: eventType,
			...fields,
			body: nextText(THREAD_EVENT_PAYLOAD_BYTES),
		});
		const eventSeq = seq;
		seq++;
		eventRows.push([eventSeq, eventType, payload, nextTimestamp()]);
		bytes += payload.length;
		return eventSeq;
	};

	const addMessage = (
		id: string,
		role: "user" | "assistant",
		toolResultForMessageId: string | null,
	): void => {
		const content = nextText(MESSAGE_CONTENT_BYTES);
		messageRows.push([
			id,
			role,
			content,
			JSON.stringify({ clientId }),
			null,
			nextTimestamp(),
			0,
			null,
			null,
			toolResultForMessageId,
		]);
		bytes += content.length;
		const eventSeq = addEvent("message_added", { messageId: id });
		messageAddedRows.push([id, eventSeq]);
	};

	for (let turn = 0; turn < turnCount; turn++) {
		const turnId = `${suffix}-${seq}`;
		const userMessageId = `agent2-grow-user-${turnId}`;
		const assistantMessageId = `agent2-grow-assistant-${turnId}`;
		const resultMessageId = `agent2-grow-result-${turnId}`;
		const toolUseId = `agent2-grow-tool-${turnId}`;
		const toolCallId = `agent2-grow-call-${turnId}`;

		addMessage(userMessageId, "user", null);
		addMessage(assistantMessageId, "assistant", null);
		toolRefRows.push([
			assistantMessageId,
			assistantMessageId,
			toolUseId,
			"tool_use",
			0,
		]);

		const issuedAt = nextTimestamp();
		const result = JSON.stringify({
			ok: true,
			run: { status: "done", result: nextText(TOOL_CALL_RESULT_BYTES) },
		});
		toolCallRows.push([
			toolCallId,
			`provider-${toolCallId}`,
			`tool_${turn % 21}`,
			JSON.stringify({ path: `/workspace/src/file-${seq}.ts` }),
			"seed-executor",
			assistantMessageId,
			issuedAt,
			null,
			"completed",
			result,
			null,
			nextTimestamp(),
		]);
		bytes += result.length;
		addEvent("tool_call_completed", { toolCallId });

		addMessage(resultMessageId, "user", assistantMessageId);
		toolRefRows.push([
			resultMessageId,
			assistantMessageId,
			toolUseId,
			"tool_result",
			0,
		]);
	}

	await insertRows(
		tx,
		stats,
		steps,
		"grow-insert-messages",
		`INSERT INTO messages (message_id, role, content, meta, user_state, created_at, cancelled, read_at, parent_tool_use_id, tool_result_for_message_id)`,
		messageRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"grow-insert-message-tool-refs",
		`INSERT INTO message_tool_refs (source_message_id, assistant_message_id, tool_use_id, block_type, cancelled)`,
		toolRefRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"grow-insert-tool-calls",
		`INSERT INTO tool_calls (id, provider_tool_use_id, tool_name, args, executor_id, message_id, issued_at, expires_at, state, result, progress, completed_at)`,
		toolCallRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"grow-insert-thread-events",
		`INSERT INTO thread_events (seq, event_type, payload, created_at)`,
		eventRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"grow-insert-message-added-events",
		`INSERT INTO message_added_events (message_id, seq)`,
		messageAddedRows,
	);

	return { turns: turnCount, bytes };
}

/**
 * Reads the newest top-level messages with their content, the way an agent
 * loads recent history into context after a wake.
 */
async function runReadRecentMessages(
	sql: AgentConcurrent2Db,
	limit: number,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<{ rows: number; bytes: number }> {
	const steps: AgentConcurrent2Step[] = [];
	const rows = await timedQuery<{ content: string | null }>(
		sql,
		stats,
		steps,
		"read-recent-messages",
		`SELECT message_id, role, content, meta, created_at FROM messages WHERE parent_tool_use_id IS NULL AND cancelled = 0 ORDER BY created_at DESC LIMIT ?`,
		limit,
	);
	let bytes = 0;
	for (const row of rows) {
		bytes += String(row.content ?? "").length;
	}
	return { rows: rows.length, bytes };
}

/** Runs one latency stage and returns the rows it touched and their content bytes. */
async function runStage(
	sql: AgentConcurrent2Db,
	stage: AgentConcurrent2Stage,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<{ rows: number; bytes: number; opMs?: number[] }> {
	if (BENCH_STAGES.includes(stage)) {
		return runBenchStage(sql, stage as AgentConcurrent2BenchStage, stats);
	}
	const steps: AgentConcurrent2Step[] = [];
	const maxRows = await timedQuery<{ max: number | null }>(
		sql,
		stats,
		steps,
		"stage-max-rowid",
		`SELECT MAX(rowid) AS max FROM messages`,
	);
	const maxRowid = Number(maxRows[0]?.max ?? 0);
	if (maxRowid === 0) {
		throw new Error("agent concurrent 2 stage needs a preloaded database");
	}

	switch (stage) {
		case "random": {
			let bytes = 0;
			for (let index = 0; index < STAGE_ROWS; index++) {
				const rowid = 1 + Math.floor(Math.random() * maxRowid);
				const rows = await timedQuery<{ content: string | null }>(
					sql,
					stats,
					steps,
					"stage-random-row",
					`SELECT message_id, content FROM messages WHERE rowid = ?`,
					rowid,
				);
				bytes += String(rows[0]?.content ?? "").length;
			}
			return { rows: STAGE_ROWS, bytes };
		}
		case "oldest": {
			const rows = await timedQuery<{ content: string | null }>(
				sql,
				stats,
				steps,
				"stage-oldest-rows",
				`SELECT message_id, content FROM messages ORDER BY rowid ASC LIMIT ?`,
				STAGE_ROWS,
			);
			let bytes = 0;
			for (const row of rows) {
				bytes += String(row.content ?? "").length;
			}
			return { rows: rows.length, bytes };
		}
		case "scan": {
			const rows = await timedQuery<{ rows: number; bytes: number | null }>(
				sql,
				stats,
				steps,
				"stage-full-scan",
				`SELECT COUNT(*) AS rows, SUM(LENGTH(content)) AS bytes FROM messages`,
			);
			return {
				rows: Number(rows[0]?.rows ?? 0),
				bytes: Number(rows[0]?.bytes ?? 0),
			};
		}
		case "index": {
			// An expression index forces a read of every message's content, the shape of an
			// index build on a large table, while the index itself stays small.
			await timedQuery(
				sql,
				stats,
				steps,
				"stage-index-drop-stale",
				`DROP INDEX IF EXISTS idx_messages_c97_content_length`,
			);
			const startedAt = performance.now();
			await timedQuery(
				sql,
				stats,
				steps,
				"stage-index-create",
				`CREATE INDEX idx_messages_c97_content_length ON messages(length(content))`,
			);
			const createMs = performance.now() - startedAt;
			await timedQuery(
				sql,
				stats,
				steps,
				"stage-index-drop",
				`DROP INDEX idx_messages_c97_content_length`,
			);
			return { rows: maxRowid, bytes: 0, opMs: [createMs] };
		}
		case "vacuum": {
			const startedAt = performance.now();
			await timedQuery(sql, stats, steps, "stage-vacuum", `VACUUM`);
			return { rows: maxRowid, bytes: 0, opMs: [performance.now() - startedAt] };
		}
		case "txn":
		case "txn_large": {
			const txnBytes =
				stage === "txn_large" ? STAGE_TXN_LARGE_BYTES : STAGE_TXN_BYTES;
			const sample = await timedQuery<{ bytes: number | null }>(
				sql,
				stats,
				steps,
				"stage-message-bytes",
				`SELECT LENGTH(content) AS bytes FROM messages WHERE rowid = ?`,
				maxRowid,
			);
			const messageBytes = Math.max(1, Number(sample[0]?.bytes ?? 16_384));
			const count = Math.min(
				maxRowid,
				Math.ceil(txnBytes / messageBytes),
			);
			const first = 1 + Math.floor(Math.random() * (maxRowid - count + 1));
			const nextText = createTextGenerator(Date.now());
			const bytes = await sql.withTransaction(stats, async (tx) => {
				let written = 0;
				for (let rowid = first; rowid < first + count; rowid++) {
					const content = nextText(messageBytes);
					await timedQuery(
						tx,
						stats,
						steps,
						"stage-txn-update",
						`UPDATE messages SET content = ? WHERE rowid = ?`,
						content,
						rowid,
					);
					written += content.length;
				}
				return written;
			});
			return { rows: count, bytes };
		}
	}
}

/** Runs one `bench_*` stage; see `AgentConcurrent2BenchStage`. */
async function runBenchStage(
	sql: AgentConcurrent2Db,
	stage: AgentConcurrent2BenchStage,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<{ rows: number; bytes: number; opMs: number[] }> {
	await sql(
		`CREATE TABLE IF NOT EXISTS bench_rows (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, revision INTEGER NOT NULL)`,
	);
	const maxRows = await sql<{ max: number | null }>(
		`SELECT MAX(id) AS max FROM bench_rows`,
	);
	const rows = Number(maxRows[0]?.max ?? 0);
	const opMs: number[] = [];
	const randomId = () => 1 + Math.floor(Math.random() * rows);
	const timed = async <T>(fn: () => Promise<T>): Promise<T> => {
		const startedAt = performance.now();
		const result = await fn();
		opMs.push(performance.now() - startedAt);
		return result;
	};

	if (stage === "bench_seed") {
		const target = Math.min(BENCH_ROWS, rows + BENCH_SEED_ROWS_PER_CALL);
		const payload = "x".repeat(BENCH_ROW_BYTES);
		await timed(() =>
			sql.withTransaction(stats, async (tx) => {
				for (let id = rows + 1; id <= target; id++) {
					await tx(
						`INSERT INTO bench_rows (id, payload, revision) VALUES (?, ?, 0)`,
						id,
						payload,
					);
				}
			}),
		);
		return { rows: target, bytes: (target - rows) * BENCH_ROW_BYTES, opMs };
	}
	if (rows < BENCH_ROWS) {
		throw new Error(`bench_rows holds ${rows} of ${BENCH_ROWS} rows; run bench_seed first`);
	}

	let bytes = 0;
	switch (stage) {
		case "bench_point": {
			for (let index = 0; index < BENCH_READ_OPS; index++) {
				const id = randomId();
				const found = await timed(() =>
					sql<{ payload: string }>(`SELECT payload FROM bench_rows WHERE id = ?`, id),
				);
				bytes += found[0]?.payload.length ?? 0;
			}
			return { rows: BENCH_READ_OPS, bytes, opMs };
		}
		case "bench_adjacent": {
			const first = 1 + Math.floor(Math.random() * (rows - BENCH_READ_OPS));
			for (let id = first; id < first + BENCH_READ_OPS; id++) {
				const found = await timed(() =>
					sql<{ payload: string }>(`SELECT payload FROM bench_rows WHERE id = ?`, id),
				);
				bytes += found[0]?.payload.length ?? 0;
			}
			return { rows: BENCH_READ_OPS, bytes, opMs };
		}
		case "bench_range16": {
			for (let index = 0; index < BENCH_RANGE_OPS; index++) {
				const low = 1 + Math.floor(Math.random() * (rows - 16));
				const found = await timed(() =>
					sql<{ payload: string }>(
						`SELECT payload FROM bench_rows WHERE id >= ? AND id < ? ORDER BY id`,
						low,
						low + 16,
					),
				);
				for (const row of found) bytes += row.payload.length;
			}
			return { rows: BENCH_RANGE_OPS * 16, bytes, opMs };
		}
		case "bench_scan": {
			const found = await timed(() =>
				sql<{ rows: number; bytes: number }>(
					`SELECT COUNT(*) AS rows, SUM(LENGTH(payload)) AS bytes FROM bench_rows NOT INDEXED`,
				),
			);
			return { rows: Number(found[0]?.rows ?? 0), bytes: Number(found[0]?.bytes ?? 0), opMs };
		}
		case "bench_autocommit": {
			for (let index = 0; index < BENCH_WRITE_OPS; index++) {
				const id = randomId();
				await timed(() =>
					sql(`UPDATE bench_rows SET revision = revision + 1 WHERE id = ?`, id),
				);
			}
			return { rows: BENCH_WRITE_OPS, bytes: 0, opMs };
		}
		case "bench_batch": {
			for (let index = 0; index < BENCH_BATCH_OPS; index++) {
				await timed(() =>
					sql.withTransaction(stats, async (tx) => {
						for (let row = 0; row < BENCH_BATCH_ROWS; row++) {
							await tx(
								`UPDATE bench_rows SET revision = revision + 1 WHERE id = ?`,
								randomId(),
							);
						}
					}),
				);
			}
			return { rows: BENCH_BATCH_OPS * BENCH_BATCH_ROWS, bytes: 0, opMs };
		}
		case "bench_seed":
			throw new Error("bench_seed is handled above");
	}
}

/**
 * Inserts `count` alternating user and assistant messages of `messageBytes`
 * content each, with their thread and message-added events, in transactions of
 * at most `GROW_TRANSACTION_BYTES` of content.
 */
async function runInsertMessages(
	sql: AgentConcurrent2Db,
	clientId: string,
	count: number,
	messageBytes: number,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<{ messages: number; bytes: number; transactions: number }> {
	const nextText = createTextGenerator(hashString(clientId) ^ Date.now());
	const perTransaction = Math.max(
		1,
		Math.floor(GROW_TRANSACTION_BYTES / messageBytes),
	);
	let messages = 0;
	let bytes = 0;
	let transactions = 0;

	while (messages < count) {
		const batch = Math.min(perTransaction, count - messages);
		bytes += await sql.withTransaction(stats, (tx) =>
			appendMessages(tx, clientId, batch, messageBytes, stats, nextText),
		);
		messages += batch;
		transactions++;
	}

	return { messages, bytes, transactions };
}

/** Inserts one transaction's messages and returns their content bytes. */
async function appendMessages(
	tx: AgentConcurrent2Db,
	clientId: string,
	count: number,
	messageBytes: number,
	stats: AgentConcurrent2QueryStatsSet,
	nextText: (bytes: number) => string,
): Promise<number> {
	// Per-statement steps are dropped; the caller reports totals only.
	const steps: AgentConcurrent2Step[] = [];
	const seqRows = await timedQuery(
		tx,
		stats,
		steps,
		"insert-select-next-thread-event-seq",
		`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
	);
	let seq = Number(seqRows[0]?.seq ?? 1);
	const suffix = safeId(clientId);

	const messageRows: SQLPrimitive[][] = [];
	const eventRows: SQLPrimitive[][] = [];
	const messageAddedRows: SQLPrimitive[][] = [];
	let bytes = 0;

	for (let index = 0; index < count; index++) {
		const id = `agent2-message-${suffix}-${seq}`;
		const role = index % 2 === 0 ? "user" : "assistant";
		const content = nextText(messageBytes);
		const createdAt = new Date().toISOString();
		messageRows.push([
			id,
			role,
			content,
			JSON.stringify({ clientId }),
			null,
			createdAt,
			0,
			null,
			null,
			null,
		]);
		const payload = JSON.stringify({
			type: "message_added",
			messageId: id,
		});
		eventRows.push([seq, "message_added", payload, createdAt]);
		messageAddedRows.push([id, seq]);
		seq++;
		bytes += content.length;
	}

	await insertRows(
		tx,
		stats,
		steps,
		"insert-messages",
		`INSERT INTO messages (message_id, role, content, meta, user_state, created_at, cancelled, read_at, parent_tool_use_id, tool_result_for_message_id)`,
		messageRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"insert-thread-events",
		`INSERT INTO thread_events (seq, event_type, payload, created_at)`,
		eventRows,
	);
	await insertRows(
		tx,
		stats,
		steps,
		"insert-message-added-events",
		`INSERT INTO message_added_events (message_id, seq)`,
		messageAddedRows,
	);

	return bytes;
}

/** Inserts rows with multi-row `INSERT ... VALUES` statements of bounded size. */
async function insertRows(
	tx: AgentConcurrent2Db,
	stats: AgentConcurrent2QueryStatsSet,
	steps: AgentConcurrent2Step[],
	name: string,
	insertPrefix: string,
	rows: SQLPrimitive[][],
): Promise<void> {
	const columnCount = rows[0]?.length ?? 0;
	if (columnCount === 0) {
		return;
	}
	const rowPlaceholder = `(${Array(columnCount).fill("?").join(",")})`;
	for (let index = 0; index < rows.length; index += GROW_INSERT_BATCH_ROWS) {
		const chunk = rows.slice(index, index + GROW_INSERT_BATCH_ROWS);
		const placeholders = chunk.map(() => rowPlaceholder).join(",");
		await timedQuery(
			tx,
			stats,
			steps,
			name,
			`${insertPrefix} VALUES ${placeholders}`,
			...chunk.flat(),
		);
	}
}

const TEXT_WORDS = [
	"the",
	"agent",
	"reads",
	"file",
	"function",
	"returns",
	"value",
	"error",
	"build",
	"test",
	"request",
	"response",
	"thread",
	"message",
	"update",
	"config",
	"module",
	"import",
	"export",
	"const",
	"async",
	"await",
	"state",
	"actor",
	"query",
	"index",
	"table",
	"schema",
	"commit",
	"branch",
	"diff",
	"patch",
	"line",
	"column",
	"string",
	"number",
	"object",
	"array",
	"promise",
	"handler",
	"server",
	"client",
	"socket",
	"stream",
	"buffer",
	"cache",
	"token",
	"model",
	"prompt",
	"output",
	"input",
	"path",
	"directory",
	"process",
	"signal",
	"timeout",
	"retry",
	"status",
	"ready",
	"done",
	"pending",
	"running",
	"because",
	"and",
];

/**
 * Returns a generator of word-based filler text. Unlike the repeated
 * characters in the seed rows, it compresses roughly like real conversation
 * text, so page compression in the storage layer does not hide growth.
 */
function createTextGenerator(seed: number): (bytes: number) => string {
	let state = seed >>> 0 || 1;
	return (bytes: number) => {
		const words: string[] = [];
		let length = 0;
		while (length < bytes) {
			state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
			const word = TEXT_WORDS[(state >>> 16) % TEXT_WORDS.length] ?? "x";
			words.push(word);
			length += word.length + 1;
		}
		return words.join(" ").slice(0, bytes);
	};
}

function hashString(value: string): number {
	let hash = 2_166_136_261;
	for (let index = 0; index < value.length; index++) {
		hash = Math.imul(hash ^ value.charCodeAt(index), 16_777_619);
	}
	return hash >>> 0;
}

async function timedQuery<T = Record<string, SQLPrimitive>>(
	sql: AgentConcurrent2Db,
	stats: AgentConcurrent2QueryStatsSet,
	steps: AgentConcurrent2Step[],
	name: string,
	query: string,
	...values: SQLPrimitive[]
): Promise<T[]> {
	const startedAt = performance.now();
	try {
		const rows = await sql<T>(query, ...values);
		const durationMs = Math.round(performance.now() - startedAt);
		recordAgentConcurrent2Query(
			stats,
			name,
			query,
			durationMs,
			rows.length,
			false,
		);
		steps.push({
			name,
			durationMs,
			rowCount: rows.length,
		});
		return rows;
	} catch (error) {
		const durationMs = Math.round(performance.now() - startedAt);
		recordAgentConcurrent2Query(stats, name, query, durationMs, 0, true);
		throw error;
	}
}

async function executeTrackedQuery<T = Record<string, SQLPrimitive>>(
	execute: <U = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	) => Promise<U[]>,
	stats: AgentConcurrent2QueryStatsSet,
	name: string,
	query: string,
	...values: SQLPrimitive[]
): Promise<T[]> {
	const startedAt = performance.now();
	try {
		const rows = await execute<T>(query, ...values);
		recordAgentConcurrent2Query(
			stats,
			name,
			query,
			Math.round(performance.now() - startedAt),
			rows.length,
			false,
		);
		return rows;
	} catch (error) {
		recordAgentConcurrent2Query(
			stats,
			name,
			query,
			Math.round(performance.now() - startedAt),
			0,
			true,
		);
		throw error;
	}
}

function recordAgentConcurrent2Query(
	stats: AgentConcurrent2QueryStatsSet,
	name: string,
	query: string,
	durationMs: number,
	rowCount: number,
	failed: boolean,
): void {
	const classification = classifyAgentConcurrent2Query(query);
	for (const target of [stats.cycle, stats.wake, stats.actor]) {
		target.total++;
		target.rows += rowCount;
		if (failed) target.errors++;
		if (durationMs >= SLOW_QUERY_MS) target.slow++;
		if (durationMs > target.maxMs) {
			target.maxMs = durationMs;
			target.maxStep = `${name}:${classification.table}`;
		}
		target.byOperation[classification.operation] =
			(target.byOperation[classification.operation] ?? 0) + 1;
		target.byTable[classification.table] =
			(target.byTable[classification.table] ?? 0) + 1;
		if (classification.kind === "read") {
			target.reads++;
		} else if (classification.kind === "mutation") {
			target.mutations++;
		} else if (classification.kind === "tx") {
			target.tx++;
		} else {
			target.other++;
		}
	}
}

function classifyAgentConcurrent2Query(query: string): {
	operation: string;
	kind: "read" | "mutation" | "tx" | "other";
	table: string;
} {
	const normalized = query.trim().replace(/\s+/g, " ");
	const operation =
		normalized.match(/^([a-z]+)/i)?.[1]?.toLowerCase() ?? "other";
	const table = extractAgentConcurrent2Table(normalized, operation);
	if (operation === "select") {
		return { operation, kind: "read", table };
	}
	if (
		operation === "insert" ||
		operation === "update" ||
		operation === "delete" ||
		operation === "replace"
	) {
		return { operation, kind: "mutation", table };
	}
	if (
		operation === "begin" ||
		operation === "commit" ||
		operation === "rollback"
	) {
		return { operation, kind: "tx", table };
	}
	return { operation, kind: "other", table };
}

function extractAgentConcurrent2Table(
	query: string,
	operation: string,
): string {
	const lower = query.toLowerCase();
	if (operation === "select") {
		return firstMatch(lower, /\bfrom\s+([a-z0-9_]+)/) ?? "unknown";
	}
	if (operation === "insert" || operation === "replace") {
		return firstMatch(lower, /\binto\s+([a-z0-9_]+)/) ?? "unknown";
	}
	if (operation === "update") {
		return firstMatch(lower, /\bupdate\s+([a-z0-9_]+)/) ?? "unknown";
	}
	if (operation === "delete") {
		return firstMatch(lower, /\bfrom\s+([a-z0-9_]+)/) ?? "unknown";
	}
	if (
		operation === "begin" ||
		operation === "commit" ||
		operation === "rollback"
	) {
		return "transaction";
	}
	return "unknown";
}

function firstMatch(value: string, pattern: RegExp): string | null {
	return pattern.exec(value)?.[1] ?? null;
}

function hasPendingLaunch(value: unknown): boolean {
	if (typeof value !== "string" || value.length === 0) {
		return false;
	}
	try {
		const parsed = JSON.parse(value) as { pendingLaunch?: unknown };
		return (
			parsed.pendingLaunch !== null && parsed.pendingLaunch !== undefined
		);
	} catch {
		return false;
	}
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)));
}

async function createAgentConcurrent2Schema(
	database: RawRivetDB,
): Promise<void> {
	await database.execute(`CREATE TABLE IF NOT EXISTS executor_tools (
		executor_id TEXT NOT NULL,
		tool_name TEXT NOT NULL,
		schema TEXT NOT NULL,
		updated_at TEXT NOT NULL,
		PRIMARY KEY (executor_id, tool_name)
	)`);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_executor_tools_executor ON executor_tools(executor_id)`,
	);
	await database.execute(`CREATE TABLE IF NOT EXISTS thread_meta_kv (
		key TEXT PRIMARY KEY,
		value TEXT,
		updated_at TEXT NOT NULL
	)`);
	await database.execute(`CREATE TABLE IF NOT EXISTS thread_events (
		seq INTEGER PRIMARY KEY,
		event_type TEXT NOT NULL,
		payload TEXT NOT NULL,
		created_at TEXT NOT NULL DEFAULT (datetime('now'))
	)`);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_thread_events_seq ON thread_events(seq)`,
	);
	await database.execute(`CREATE TABLE IF NOT EXISTS message_added_events (
		message_id TEXT PRIMARY KEY,
		seq INTEGER NOT NULL UNIQUE
	)`);
	await database.execute(`CREATE TABLE IF NOT EXISTS messages (
		message_id TEXT PRIMARY KEY,
		role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'info')),
		content TEXT NOT NULL,
		meta TEXT,
		user_state TEXT,
		created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
		cancelled INTEGER NOT NULL DEFAULT 0,
		read_at TEXT,
		parent_tool_use_id TEXT,
		tool_result_for_message_id TEXT
	)`);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_role_cancelled_created_at ON messages(parent_tool_use_id, role, cancelled, created_at DESC)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_cancelled_created_at ON messages(parent_tool_use_id, cancelled, created_at DESC)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_created_at ON messages(parent_tool_use_id, created_at DESC)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_role_created_at ON messages(role, created_at)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at)`,
	);
	await database.execute(`CREATE TABLE IF NOT EXISTS message_tool_refs (
		source_message_id TEXT NOT NULL,
		assistant_message_id TEXT NOT NULL,
		tool_use_id TEXT NOT NULL,
		block_type TEXT NOT NULL CHECK(block_type IN ('tool_use', 'tool_result')),
		cancelled INTEGER NOT NULL DEFAULT 0,
		PRIMARY KEY (source_message_id, block_type, tool_use_id)
	)`);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_assistant_lookup ON message_tool_refs(assistant_message_id, block_type, cancelled, tool_use_id)`,
	);
	await database.execute(
		`CREATE UNIQUE INDEX IF NOT EXISTS idx_message_tool_refs_live_tool_result ON message_tool_refs(assistant_message_id, tool_use_id) WHERE block_type = 'tool_result' AND cancelled = 0`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_source_message ON message_tool_refs(source_message_id)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_tool_use_lookup ON message_tool_refs(tool_use_id, assistant_message_id) WHERE block_type = 'tool_use' AND cancelled = 0`,
	);
	await database.execute(`CREATE TABLE IF NOT EXISTS tool_calls (
		id TEXT PRIMARY KEY,
		provider_tool_use_id TEXT NOT NULL,
		tool_name TEXT NOT NULL,
		args TEXT NOT NULL,
		executor_id TEXT,
		message_id TEXT NOT NULL,
		issued_at TEXT NOT NULL,
		expires_at TEXT,
		state TEXT NOT NULL CHECK(state IN ('queued', 'pending_reconnect', 'pending_ack', 'running', 'completed', 'expired', 'revoked')),
		result TEXT,
		progress TEXT,
		completed_at TEXT
	)`);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_message_id ON tool_calls(message_id)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_state ON tool_calls(state)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_issued_at ON tool_calls(issued_at)`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_expires_at ON tool_calls(expires_at) WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS environment_snapshot (id INTEGER PRIMARY KEY CHECK (id = 1), snapshot TEXT NOT NULL, updated_at TEXT NOT NULL)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS thread_settings_snapshot (id INTEGER PRIMARY KEY CHECK (id = 1), settings TEXT NOT NULL, updated_at TEXT NOT NULL)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS retry_state (id INTEGER PRIMARY KEY CHECK (id = 1), attempt INTEGER NOT NULL DEFAULT 0, scheduled_at INTEGER NOT NULL, reason TEXT NOT NULL)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS queued_messages (message_id TEXT PRIMARY KEY, content TEXT NOT NULL, user_state TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), steer INTEGER NOT NULL DEFAULT 0, user_meta TEXT)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS executor_artifacts (artifact_key TEXT PRIMARY KEY, data_type TEXT NOT NULL, content_base64 TEXT NOT NULL, tool_call_id TEXT, updated_at TEXT NOT NULL)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS e2b_sandbox (
			id INTEGER PRIMARY KEY CHECK (id = 1),
			sandbox_id TEXT,
			restart_attempts INTEGER NOT NULL DEFAULT 0,
			traffic_access_token TEXT,
			project_id TEXT,
			repository_url TEXT,
			additional_repositories TEXT,
			setup TEXT,
			created_at TEXT NOT NULL,
			updated_at TEXT NOT NULL
		)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS tool_approvals (id TEXT PRIMARY KEY, tool_call_id TEXT NOT NULL UNIQUE, tool_name TEXT NOT NULL, args TEXT NOT NULL, reason TEXT, to_allow TEXT, context TEXT NOT NULL CHECK(context IN ('thread', 'subagent')), subagent_tool_name TEXT, parent_tool_call_id TEXT, timestamp INTEGER NOT NULL, matched_rule TEXT, rule_source TEXT CHECK(rule_source IN ('user', 'built-in')))`,
	);
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_approvals_timestamp ON tool_approvals(timestamp)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS compaction_summaries (summary_id TEXT PRIMARY KEY, summary_text TEXT NOT NULL, cut_message_id TEXT NOT NULL, created_at TEXT NOT NULL)`,
	);
}

async function seedAgentConcurrent2Data(database: RawRivetDB): Promise<void> {
	// EXISTS stops at the first row; COUNT(*) would scan the whole table on every wake.
	const existing = await database.execute(
		`SELECT EXISTS(SELECT 1 FROM messages) AS seeded`,
	);
	if (Number(existing[0]?.seeded ?? 0) > 0) {
		return;
	}

	const now = new Date("2026-05-16T03:58:18.661Z").getTime();
	const text = (size: number) => "x".repeat(size);
	const isoAt = (index: number) =>
		new Date(now + index * 1_000).toISOString();

	await batchInsert(
		database,
		`INSERT INTO thread_meta_kv (key, value, updated_at)`,
		[
			["executor_type", "local-client", isoAt(0)],
			[
				"workspace_intent",
				JSON.stringify({ spec: null, pendingLaunch: null }),
				isoAt(0),
			],
			[
				"executor_status",
				JSON.stringify({ available: true, message: "ready" }),
				isoAt(0),
			],
		],
	);

	const messageRows: unknown[][] = [];
	for (let index = 1; index <= MESSAGE_COUNT; index++) {
		const role = index % 2 === 0 ? "assistant" : "user";
		messageRows.push([
			messageId(index),
			role,
			text(MESSAGE_CONTENT_BYTES),
			null,
			null,
			isoAt(index),
			0,
			null,
			null,
			null,
		]);
	}
	await batchInsert(
		database,
		`INSERT INTO messages (message_id, role, content, meta, user_state, created_at, cancelled, read_at, parent_tool_use_id, tool_result_for_message_id)`,
		messageRows,
		20,
	);

	const messageToolRefRows: unknown[][] = [];
	for (let index = 0; index < MESSAGE_TOOL_REF_COUNT / 2; index++) {
		const assistantIndex = 2 + (index % 42) * 2;
		const sourceIndex = Math.max(1, assistantIndex - 1);
		const resultIndex = Math.min(MESSAGE_COUNT, assistantIndex + 1);
		const toolUseId = toolUseID(index + 1);
		messageToolRefRows.push([
			messageId(sourceIndex),
			messageId(assistantIndex),
			toolUseId,
			"tool_use",
			0,
		]);
		messageToolRefRows.push([
			messageId(resultIndex),
			messageId(assistantIndex),
			toolUseId,
			"tool_result",
			0,
		]);
	}
	await batchInsert(
		database,
		`INSERT INTO message_tool_refs (source_message_id, assistant_message_id, tool_use_id, block_type, cancelled)`,
		messageToolRefRows,
		50,
	);

	const toolCallRows: unknown[][] = [];
	for (let index = 1; index <= TOOL_CALL_COUNT; index++) {
		const assistantIndex = 2 + ((index - 1) % 42) * 2;
		toolCallRows.push([
			toolUseID(index),
			`provider-${index}`,
			`tool_${index % 21}`,
			JSON.stringify({ path: `/tmp/file-${index}` }),
			"seed-executor",
			messageId(assistantIndex),
			isoAt(index),
			null,
			"completed",
			JSON.stringify({
				ok: true,
				run: { status: "done", result: text(TOOL_CALL_RESULT_BYTES) },
			}),
			null,
			isoAt(index + 100),
		]);
	}
	await batchInsert(
		database,
		`INSERT INTO tool_calls (id, provider_tool_use_id, tool_name, args, executor_id, message_id, issued_at, expires_at, state, result, progress, completed_at)`,
		toolCallRows,
		20,
	);

	const executorToolRows: unknown[][] = [];
	for (let index = 1; index <= EXECUTOR_TOOL_COUNT; index++) {
		const schema = JSON.stringify({
			name: `tool_${index}`,
			description: text(EXECUTOR_TOOL_SCHEMA_BYTES),
			input_schema: { type: "object", properties: {} },
		});
		executorToolRows.push([
			"seed-executor",
			`tool_${index}`,
			schema,
			isoAt(index),
		]);
	}
	await batchInsert(
		database,
		`INSERT INTO executor_tools (executor_id, tool_name, schema, updated_at)`,
		executorToolRows,
		42,
	);

	const threadEventRows: unknown[][] = [];
	for (let index = 1; index <= THREAD_EVENT_COUNT; index++) {
		threadEventRows.push([
			index,
			index % 3 === 0 ? "message_added" : "agent_state_changed",
			JSON.stringify({
				type: "seed_event",
				body: text(THREAD_EVENT_PAYLOAD_BYTES),
			}),
			isoAt(index),
		]);
	}
	await batchInsert(
		database,
		`INSERT INTO thread_events (seq, event_type, payload, created_at)`,
		threadEventRows,
		25,
	);

	const messageAddedRows: unknown[][] = [];
	for (let index = 1; index <= MESSAGE_COUNT; index++) {
		messageAddedRows.push([messageId(index), index]);
	}
	await batchInsert(
		database,
		`INSERT INTO message_added_events (message_id, seq)`,
		messageAddedRows,
		50,
	);

	await database.execute(
		`INSERT INTO environment_snapshot (id, snapshot, updated_at) VALUES (1, ?, ?)`,
		JSON.stringify({ cwd: "/workspace", body: text(3_620) }),
		isoAt(0),
	);
	await database.execute(
		`INSERT INTO thread_settings_snapshot (id, settings, updated_at) VALUES (1, ?, ?)`,
		JSON.stringify({ maxTokens: 20_000, body: text(55) }),
		isoAt(0),
	);
	await database.execute(
		`INSERT INTO e2b_sandbox (id, sandbox_id, restart_attempts, traffic_access_token, project_id, repository_url, additional_repositories, setup, created_at, updated_at)
			VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		"sandbox-seed",
		0,
		"token-seed",
		"project-seed",
		"https://example.invalid/repo.git",
		JSON.stringify([]),
		JSON.stringify({ commands: [] }),
		isoAt(0),
		isoAt(0),
	);
}

async function batchInsert(
	database: RawRivetDB,
	insertPrefix: string,
	rows: unknown[][],
	batchSize = 100,
): Promise<void> {
	if (rows.length === 0) {
		return;
	}
	const columnCount = rows[0]?.length ?? 0;
	if (columnCount === 0) {
		return;
	}
	const rowPlaceholder = `(${"?,".repeat(columnCount).slice(0, -1)})`;
	for (let index = 0; index < rows.length; index += batchSize) {
		const chunk = rows.slice(index, index + batchSize);
		const values = chunk.map(() => rowPlaceholder).join(",");
		const bindings = chunk.flat();
		await database.execute(`${insertPrefix} VALUES ${values}`, ...bindings);
	}
}

function messageId(index: number): string {
	return `M-${String(index).padStart(22, "0")}`;
}

function toolUseID(index: number): string {
	return `toolu_${String(index).padStart(22, "0")}`;
}

function safeId(value: string): string {
	return value.replace(/[^a-zA-Z0-9_-]/g, "-").slice(0, 80);
}

// ---------------------------------------------------------------------------
// Agent iteration workload
//
// Mirrors one loop iteration of a production thread actor: the commits that
// run between "the model finished streaming" and "the next inference starts",
// in order, with the reads that run between them. `serial` issues the commits
// one after another as the production actor does (13 commits). `merged`
// folds the writes into two transactions, the shape a batching rewrite of the
// actor would have.
// ---------------------------------------------------------------------------

interface AgentIterationOptions {
	clientId: string;
	iteration: number;
	toolCalls: number;
	assistantBytes: number;
	toolResultBytes: number;
	/** Simulated billing HTTP call between the delivery INSERT and its UPDATE. */
	billingHttpMs: number;
	/** Simulated tool run time. */
	toolMs: number;
	/** SELECTs the tick loop issues in one planning pass. */
	planReads: number;
	/** Rows read when the actor rebuilds the transcript for the next request. */
	transcriptLimit: number;
	/** Immediate actor state saves per iteration, one at the end of each tick pass. */
	stateSaves: number;
	mode: "serial" | "merged";
}

interface AgentIterationResult {
	type: "agent2_iterate_result";
	mode: "serial" | "merged";
	iteration: number;
	totalMs: number;
	/** Named critical-path commits in order, with milliseconds. */
	commits: Array<[string, number]>;
	/** Plain reads that did not directly follow a commit. */
	reads: number[];
	/** Reads that ran directly after a commit. */
	readsAfterCommit: number[];
	/** Immediate actor state saves, as `flushDirtyState` runs after each tick. */
	stateSaves: number[];
	planMs: number;
	transcriptMs: number;
	transcriptRows: number;
	dbBytes: number;
}

function parseIterationOptions(
	request: Record<string, unknown>,
): AgentIterationOptions {
	const bounded = (field: string, fallback: number, max: number): number => {
		const value = optionalNumberField(request, field) ?? fallback;
		if (!Number.isFinite(value) || value < 0 || value > max) {
			throw new Error(
				`agent concurrent 2 request ${field} must be from 0 to ${max}`,
			);
		}
		return value;
	};
	const mode = request.mode === "merged" ? "merged" : "serial";
	return {
		clientId: stringField(request, "clientId"),
		iteration: bounded("iteration", 0, 1_000_000_000),
		toolCalls: Math.max(1, bounded("toolCalls", 1, 16)),
		assistantBytes: bounded("assistantBytes", 4_000, 1_048_576),
		toolResultBytes: bounded("toolResultBytes", 2_700, 1_048_576),
		billingHttpMs: bounded("billingHttpMs", 94, 60_000),
		toolMs: bounded("toolMs", 115, 600_000),
		planReads: bounded("planReads", 30, 500),
		transcriptLimit: bounded("transcriptLimit", 500, 10_000),
		stateSaves: bounded("stateSaves", 3, 16),
		mode,
	};
}

async function ensureIterationSchema(database: RawRivetDB): Promise<void> {
	await database.execute(
		`CREATE TABLE IF NOT EXISTS inference_requests (id TEXT PRIMARY KEY, state TEXT NOT NULL, provider_response_id TEXT, created_at TEXT NOT NULL)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS pending_inference_messages (request_id TEXT NOT NULL, message_id TEXT NOT NULL, PRIMARY KEY (request_id, message_id))`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS billing_deliveries (id TEXT PRIMARY KEY, request_id TEXT NOT NULL, state TEXT NOT NULL, attempts INTEGER NOT NULL, created_at TEXT NOT NULL, completed_at TEXT)`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS usage_records (id TEXT PRIMARY KEY, request_id TEXT NOT NULL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL, cost_micros INTEGER NOT NULL, created_at TEXT NOT NULL)`,
	);
}

async function runAgentIteration(
	sql: AgentConcurrent2Db,
	options: AgentIterationOptions,
	stats: AgentConcurrent2QueryStatsSet,
	saveState: () => Promise<void>,
): Promise<AgentIterationResult> {
	const startedAt = performance.now();
	const nextText = createTextGenerator(
		hashString(options.clientId) ^ options.iteration,
	);
	const suffix = `${safeId(options.clientId)}-${options.iteration}`;
	const commits: Array<[string, number]> = [];
	const reads: number[] = [];
	const readsAfterCommit: number[] = [];
	const stateSaves: number[] = [];
	let stateSavesLeft = options.stateSaves;
	/** Ends a tick pass: the actor flushes its dirty state with an immediate save. */
	const endTick = async () => {
		if (stateSavesLeft <= 0) return;
		stateSavesLeft--;
		const t0 = performance.now();
		await saveState();
		stateSaves.push(performance.now() - t0);
	};
	const nowIso = () => new Date().toISOString();

	const timed = async <T>(fn: () => Promise<T>): Promise<[T, number]> => {
		const t0 = performance.now();
		const value = await fn();
		return [value, performance.now() - t0];
	};
	const commit = async (name: string, fn: () => Promise<unknown>) => {
		const [, ms] = await timed(fn);
		commits.push([name, ms]);
	};
	const readAfterCommit = async () => {
		const [, ms] = await timed(() =>
			sql(
				`SELECT key, value FROM thread_meta_kv WHERE key = 'archived'`,
			),
		);
		readsAfterCommit.push(ms);
	};
	/** A planning pass: indexed SELECTs like the tick loop's, no writes. */
	const planningPass = async (count: number): Promise<number> => {
		const t0 = performance.now();
		for (let index = 0; index < count; index++) {
			const [, ms] = await timed(() => {
				switch (index % 5) {
					case 0:
						return sql(
							`SELECT key, value FROM thread_meta_kv WHERE key = 'agent_state'`,
						);
					case 1:
						return sql(
							`SELECT id, state, executor_id FROM tool_calls WHERE state IN ('queued','pending_ack','running') LIMIT 20`,
						);
					case 2:
						return sql(
							`SELECT message_id, role FROM messages WHERE parent_tool_use_id IS NULL ORDER BY created_at DESC LIMIT 1`,
						);
					case 3:
						return sql(
							`SELECT artifact_key FROM executor_artifacts WHERE tool_call_id = ? LIMIT 5`,
							`tc-${suffix}`,
						);
					default:
						return sql(
							`SELECT id, state FROM inference_requests ORDER BY created_at DESC LIMIT 1`,
						);
				}
			});
			reads.push(ms);
		}
		return performance.now() - t0;
	};

	// Setup, not timed: the request row the previous iteration left behind.
	const requestId = `ir-${suffix}`;
	const previousRequest = await sql<{ id: string }>(
		`SELECT id FROM inference_requests WHERE state = 'streaming' ORDER BY created_at DESC LIMIT 1`,
	);
	const currentRequest = previousRequest[0]?.id ?? `ir-seed-${suffix}`;
	if (!previousRequest[0]) {
		await sql(
			`INSERT INTO inference_requests (id, state, provider_response_id, created_at) VALUES (?, 'streaming', NULL, ?)`,
			currentRequest,
			nowIso(),
		);
	}
	const seqRows = await sql<{ seq: number }>(
		`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
	);
	let seq = Number(seqRows[0]?.seq ?? 1);
	const assistantId = `agent-iter-a-${suffix}`;
	const toolIds = Array.from(
		{ length: options.toolCalls },
		(_, index) => `tc-${suffix}-${index}`,
	);

	const setAgentState = (tx: AgentConcurrent2Db, state: string) =>
		tx(
			`INSERT INTO thread_meta_kv (key, value, updated_at) VALUES ('agent_state', ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at`,
			state,
			nowIso(),
		);
	const pushEvent = (tx: AgentConcurrent2Db, type: string, payload: string) =>
		tx(
			`INSERT INTO thread_events (seq, event_type, payload, created_at) VALUES (?, ?, ?, ?)`,
			seq++,
			type,
			payload,
			nowIso(),
		);
	const persistAssistant = async (tx: AgentConcurrent2Db) => {
		await tx(
			`INSERT INTO messages (message_id, role, content, meta, created_at) VALUES (?, 'assistant', ?, ?, ?)`,
			assistantId,
			nextText(options.assistantBytes),
			JSON.stringify({ clientId: options.clientId }),
			nowIso(),
		);
		for (const toolId of toolIds) {
			await tx(
				`INSERT INTO message_tool_refs (source_message_id, assistant_message_id, tool_use_id, block_type, cancelled) VALUES (?, ?, ?, 'tool_use', 0)`,
				assistantId,
				assistantId,
				toolId,
			);
		}
		await tx(
			`INSERT INTO message_added_events (message_id, seq) VALUES (?, ?)`,
			assistantId,
			seq,
		);
		await pushEvent(tx, "message_added", JSON.stringify({ id: assistantId }));
		await tx(
			`UPDATE inference_requests SET state = 'completed' WHERE id = ?`,
			currentRequest,
		);
	};
	const recordUsage = (tx: AgentConcurrent2Db) =>
		tx(
			`INSERT INTO usage_records (id, request_id, input_tokens, output_tokens, cost_micros, created_at) VALUES (?, ?, ?, ?, ?, ?)`,
			`u-${suffix}`,
			currentRequest,
			12_000 + options.iteration,
			600,
			4_200,
			nowIso(),
		);
	const queueToolCalls = async (tx: AgentConcurrent2Db, state: string) => {
		for (const toolId of toolIds) {
			await tx(
				`INSERT INTO tool_calls (id, provider_tool_use_id, tool_name, args, message_id, issued_at, state, executor_id) VALUES (?, ?, 'shell_command', ?, ?, ?, ?, ?)`,
				toolId,
				`toolu-${toolId}`,
				JSON.stringify({ command: "ls -la" }),
				assistantId,
				nowIso(),
				state,
				state === "queued" ? null : "executor-1",
			);
		}
	};
	const completeToolCalls = async (tx: AgentConcurrent2Db) => {
		for (const toolId of toolIds) {
			await tx(
				`UPDATE tool_calls SET state = 'completed', result = ?, completed_at = ? WHERE id = ?`,
				nextText(options.toolResultBytes),
				nowIso(),
				toolId,
			);
		}
	};
	const insertNextRequest = async (tx: AgentConcurrent2Db) => {
		const toolResultId = `agent-iter-r-${suffix}`;
		await tx(
			`INSERT INTO messages (message_id, role, content, meta, created_at, tool_result_for_message_id) VALUES (?, 'user', ?, ?, ?, ?)`,
			toolResultId,
			nextText(options.toolResultBytes),
			JSON.stringify({ clientId: options.clientId }),
			nowIso(),
			assistantId,
		);
		await tx(
			`INSERT INTO inference_requests (id, state, provider_response_id, created_at) VALUES (?, 'streaming', NULL, ?)`,
			requestId,
			nowIso(),
		);
		await tx(
			`INSERT INTO pending_inference_messages (request_id, message_id) VALUES (?, ?)`,
			requestId,
			toolResultId,
		);
	};
	const readTranscript = async (): Promise<[number, number]> => {
		const [rows, ms] = await timed(() =>
			sql(
				`SELECT message_id, role, content FROM messages ORDER BY created_at DESC LIMIT ?`,
				options.transcriptLimit,
			),
		);
		return [ms, rows.length];
	};
	/** The executor's lease acknowledgement arrives while the tool runs. */
	const ackAndRunTool = async () => {
		const ack = async () => {
			for (const toolId of toolIds) {
				await sql(
					`UPDATE tool_calls SET state = 'running', expires_at = ? WHERE id = ?`,
					nowIso(),
					toolId,
				);
			}
		};
		await Promise.all([ack(), delay(options.toolMs)]);
	};

	let planMs = 0;
	let transcriptMs = 0;
	let transcriptRows = 0;

	if (options.mode === "serial") {
		// Model finished: provider id, billing delivery.
		await commit("provider-response-id", () =>
			sql(
				`UPDATE inference_requests SET provider_response_id = ? WHERE id = ?`,
				`resp-${suffix}`,
				currentRequest,
			),
		);
		await commit("billing-insert", () =>
			sql(
				`INSERT INTO billing_deliveries (id, request_id, state, attempts, created_at) VALUES (?, ?, 'pending', 1, ?)`,
				`bd-${suffix}`,
				currentRequest,
				nowIso(),
			),
		);
		await delay(options.billingHttpMs);
		await commit("billing-complete", () =>
			sql(
				`UPDATE billing_deliveries SET state = 'completed', completed_at = ? WHERE id = ?`,
				nowIso(),
				`bd-${suffix}`,
			),
		);
		await commit("assistant-message", () =>
			sql.withTransaction(stats, persistAssistant),
		);
		await commit("state-tool-use", () =>
			sql.withTransaction(stats, async (tx) => {
				await setAgentState(tx, "tool_use");
				await pushEvent(tx, "agent_state", '{"state":"tool_use"}');
			}),
		);
		await readAfterCommit();
		await commit("usage", () => sql.withTransaction(stats, recordUsage));
		await readAfterCommit();
		await commit("state-running-tools", () =>
			sql.withTransaction(stats, async (tx) => {
				await setAgentState(tx, "running_tools");
				await pushEvent(tx, "agent_state", '{"state":"running_tools"}');
			}),
		);
		await readAfterCommit();
		await commit("tool-calls-insert", () =>
			sql.withTransaction(stats, (tx) => queueToolCalls(tx, "queued")),
		);
		planMs += await planningPass(options.planReads);
		for (const toolId of toolIds) {
			await commit("lease-executor", () =>
				sql(
					`UPDATE tool_calls SET state = 'pending_ack', executor_id = 'executor-1' WHERE id = ?`,
					toolId,
				),
			);
			await commit("lease-expiry", () =>
				sql(
					`UPDATE tool_calls SET expires_at = ? WHERE id = ?`,
					nowIso(),
					toolId,
				),
			);
		}
		await endTick();
		await ackAndRunTool();
		await commit("tool-complete", () => completeToolCalls(sql));
		await endTick();
		planMs += await planningPass(Math.round(options.planReads * 1.5));
		[transcriptMs, transcriptRows] = await readTranscript();
		await commit("next-request-insert", () =>
			sql.withTransaction(stats, insertNextRequest),
		);
		await commit("state-streaming", () =>
			sql.withTransaction(stats, async (tx) => {
				await setAgentState(tx, "streaming");
				await pushEvent(tx, "agent_state", '{"state":"streaming"}');
			}),
		);
		await readAfterCommit();
		await endTick();
	} else {
		// Merged: one transaction for everything after inference, one after the tool.
		await commit("post-inference", () =>
			sql.withTransaction(stats, async (tx) => {
				await tx(
					`UPDATE inference_requests SET provider_response_id = ? WHERE id = ?`,
					`resp-${suffix}`,
					currentRequest,
				);
				await tx(
					`INSERT INTO billing_deliveries (id, request_id, state, attempts, created_at) VALUES (?, ?, 'pending', 1, ?)`,
					`bd-${suffix}`,
					currentRequest,
					nowIso(),
				);
				await persistAssistant(tx);
				await recordUsage(tx);
				await setAgentState(tx, "running_tools");
				await pushEvent(tx, "agent_state", '{"state":"running_tools"}');
				await queueToolCalls(tx, "pending_ack");
			}),
		);
		await readAfterCommit();
		planMs += await planningPass(options.planReads);
		await endTick();
		await ackAndRunTool();
		planMs += await planningPass(Math.round(options.planReads * 1.5));
		[transcriptMs, transcriptRows] = await readTranscript();
		await commit("post-tool", () =>
			sql.withTransaction(stats, async (tx) => {
				await completeToolCalls(tx);
				await insertNextRequest(tx);
				await setAgentState(tx, "streaming");
				await pushEvent(tx, "agent_state", '{"state":"streaming"}');
			}),
		);
		await readAfterCommit();
		await endTick();
	}

	return {
		type: "agent2_iterate_result",
		mode: options.mode,
		iteration: options.iteration,
		totalMs: performance.now() - startedAt,
		commits,
		reads,
		readsAfterCommit,
		stateSaves,
		planMs,
		transcriptMs,
		transcriptRows,
		dbBytes: await readDbBytes(sql),
	};
}
