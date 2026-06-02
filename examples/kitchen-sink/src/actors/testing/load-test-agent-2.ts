import { actor, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";
import { db } from "rivetkit/db";

type AgentConcurrent2Request =
	| { type: "agent2_resume"; version: number }
	| { type: "agent2_connect"; clientId: string; staggerHandleMs?: number }
	| { type: "force_sleep" }
	| { type: "ping"; id?: number };

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
				await executeTrackedQuery(execute, stats, "transaction-begin", "BEGIN");
				activeTransaction = tx;
				try {
					const result = await fn(tx);
					activeTransaction = null;
					await executeTrackedQuery(execute, stats, "transaction-commit", "COMMIT");
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

function send(
	websocket: UniversalWebSocket,
	message: AgentConcurrent2ResultMessage | AgentConcurrent2ErrorMessage | object,
): void {
	if (websocket.readyState === 1) {
		websocket.send(JSON.stringify(message));
	}
}

export const loadTestAgent2 = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 1_000,
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
			timestamp: Date.now(),
		});

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			const promise = handleAgentConcurrent2Message(c, websocket, event.data);
			void c.keepAwake(promise);
		});
	},
	actions: {
		run: async (c, clientId?: string) => {
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

		if (request.type === "agent2_resume") {
			const startedAt = performance.now();
			const result = await runCatchupSnapshot(
				runtime.sql,
				request.version,
				stats,
			);
			send(websocket, {
				type: "agent2_result",
				trigger: request.type,
				totalMs: Math.round(performance.now() - startedAt),
				results: [result],
				stats: snapshotAgentConcurrent2Stats(c, cycleStats),
			});
			return;
		}

		const result = await runAgentConcurrent2Workload(
			runtime.sql,
			request.clientId,
			request.staggerHandleMs ?? 0,
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
			...(cycleStats ? { stats: snapshotAgentConcurrent2Stats(c, cycleStats) } : {}),
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
		return { type: "agent2_resume", version: numberField(request, "version") };
	}
	if (request.type === "agent2_connect") {
		return {
			type: "agent2_connect",
			clientId: stringField(request, "clientId"),
			...(typeof request.staggerHandleMs === "number"
				? { staggerHandleMs: request.staggerHandleMs }
				: {}),
		};
	}
	throw new Error(`unknown agent concurrent 2 request type: ${String(request.type)}`);
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
		throw new Error(`agent concurrent 2 request ${field} must be a finite number`);
	}
	return value;
}

function createAgentConcurrent2Db(db: RawRivetDB): AgentConcurrent2Db {
	return createSerializedDb(async <T = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	): Promise<T[]> => {
		const converted = values.map((value) =>
			typeof value === "boolean" ? (value ? 1 : 0) : value,
		);
		return (await db.execute(query, ...converted)) as T[];
	});
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
	stats: AgentConcurrent2QueryStatsSet,
): Promise<Omit<AgentConcurrent2ResultMessage, "type" | "trigger" | "stats">> {
	const startedAt = performance.now();
	const buildToolPlanContext = runBuildToolPlanContext(sql, stats);
	const catchupSnapshot = runCatchupSnapshot(sql, 0, stats);
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
	return {
		totalMs: Math.round(performance.now() - startedAt),
		results,
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
	const latestExecutorId = String(latestExecutor[0]?.executor_id ?? "seed-executor");
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
			JOIN messages AS m
				ON m.message_id = tool_use.assistant_message_id
			WHERE tool_use.block_type = 'tool_use'
				AND tool_use.cancelled = 0
				AND m.cancelled = 0
				AND m.role = 'assistant'
				AND m.parent_tool_use_id IS NULL
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

async function runCatchupSnapshot(
	sql: AgentConcurrent2Db,
	version: number,
	stats: AgentConcurrent2QueryStatsSet,
): Promise<AgentConcurrent2WorkloadResult> {
	const startedAt = performance.now();
	const steps: AgentConcurrent2Step[] = [];
	await Promise.all([
		timedQuery(
			sql,
			stats,
			steps,
			"thread-events-list-since-version",
			`SELECT seq, event_type, payload, created_at FROM thread_events WHERE seq > ? ORDER BY seq ASC`,
			version,
		),
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
			JSON.stringify({ type: "message_added", messageId: messageIdValue }),
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
		recordAgentConcurrent2Query(stats, name, query, durationMs, rows.length, false);
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
	const operation = normalized.match(/^([a-z]+)/i)?.[1]?.toLowerCase() ?? "other";
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
	if (operation === "begin" || operation === "commit" || operation === "rollback") {
		return { operation, kind: "tx", table };
	}
	return { operation, kind: "other", table };
}

function extractAgentConcurrent2Table(query: string, operation: string): string {
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
	if (operation === "begin" || operation === "commit" || operation === "rollback") {
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
		return parsed.pendingLaunch !== null && parsed.pendingLaunch !== undefined;
	} catch {
		return false;
	}
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)));
}

async function createAgentConcurrent2Schema(database: RawRivetDB): Promise<void> {
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
	const existing = await database.execute(`SELECT COUNT(*) AS count FROM messages`);
	if (Number(existing[0]?.count ?? 0) > 0) {
		return;
	}

	const now = new Date("2026-05-16T03:58:18.661Z").getTime();
	const text = (size: number) => "x".repeat(size);
	const isoAt = (index: number) => new Date(now + index * 1_000).toISOString();

	await batchInsert(database, `INSERT INTO thread_meta_kv (key, value, updated_at)`, [
		["executor_type", "local-client", isoAt(0)],
		["workspace_intent", JSON.stringify({ spec: null, pendingLaunch: null }), isoAt(0)],
		["executor_status", JSON.stringify({ available: true, message: "ready" }), isoAt(0)],
	]);

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
		executorToolRows.push(["seed-executor", `tool_${index}`, schema, isoAt(index)]);
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
			JSON.stringify({ type: "seed_event", body: text(THREAD_EVENT_PAYLOAD_BYTES) }),
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
