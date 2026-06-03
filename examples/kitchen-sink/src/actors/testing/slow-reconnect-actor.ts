import { actor, setup } from 'rivetkit'
import { db } from 'rivetkit/db'

export type SlowReconnectRequest =
	| { type: 'client_resume'; version: number }
	| {
		type: 'executor_connect'
		clientId: string
		executorType?: 'local-client' | 'sandbox' | 'virtual'
	}
	| {
		type: 'repro_reconnect'
		clientId?: string
		staggerHandleMs?: number
	}

export interface SlowReconnectStep {
	name: string
	durationMs: number
	rowCount: number
}

export interface SlowReconnectWorkloadResult {
	name: string
	totalMs: number
	steps: SlowReconnectStep[]
}

export interface SlowReconnectResultMessage {
	type: 'slow_reconnect_result'
	trigger: SlowReconnectRequest['type']
	totalMs: number
	results: SlowReconnectWorkloadResult[]
}

export interface SlowReconnectErrorMessage {
	type: 'slow_reconnect_error'
	trigger: SlowReconnectRequest['type'] | 'unknown'
	error: string
}

export interface SlowReconnectVars {
	sql: Db | null
}

interface RawRivetDB {
	execute: (query: string, ...args: unknown[]) => Promise<Record<string, unknown>[]>
}

type SQLPrimitive = string | number | boolean | null

type Db = (<T = Record<string, SQLPrimitive>>(
	query: string,
	...values: SQLPrimitive[]
) => Promise<T[]>) & {
	withTransaction<T>(fn: (tx: Db) => Promise<T>): Promise<T>
}

class AsyncMutex {
	private locked = false
	private waiters: Array<() => void> = []

	async acquire(): Promise<void> {
		if (!this.locked) {
			this.locked = true
			return
		}
		await new Promise<void>((resolve) => this.waiters.push(resolve))
		this.locked = true
	}

	release(): void {
		const next = this.waiters.shift()
		if (next) {
			next()
			return
		}
		this.locked = false
	}
}

function createDb(execute: <T = Record<string, SQLPrimitive>>(
	query: string,
	...values: SQLPrimitive[]
) => Promise<T[]>): Db {
	const mutex = new AsyncMutex()
	let activeTransaction: Db | null = null

	const createTransactionDb = (): Db => {
		const tx = Object.assign(
			<T = Record<string, SQLPrimitive>>(query: string, ...values: SQLPrimitive[]) =>
				execute<T>(query, ...values),
			{
				withTransaction: async <T>(fn: (tx: Db) => Promise<T>): Promise<T> => fn(tx),
			},
		)
		return tx
	}

	const queryWithMutex = async <T = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	): Promise<T[]> => {
		if (activeTransaction) {
			return activeTransaction<T>(query, ...values)
		}
		await mutex.acquire()
		try {
			return await execute<T>(query, ...values)
		} finally {
			mutex.release()
		}
	}

	const sql = Object.assign(queryWithMutex, {
		withTransaction: async <T>(fn: (tx: Db) => Promise<T>): Promise<T> => {
			if (activeTransaction) {
				return fn(activeTransaction)
			}
			await mutex.acquire()
			const tx = createTransactionDb()
			try {
				await execute('BEGIN')
				activeTransaction = tx
				try {
					const result = await fn(tx)
					activeTransaction = null
					await execute('COMMIT')
					return result
				} catch (error) {
					activeTransaction = null
					await execute('ROLLBACK')
					throw error
				}
			} finally {
				activeTransaction = null
				mutex.release()
			}
		},
	})
	return sql
}

const MESSAGE_COUNT = 84
const MESSAGE_TOOL_REF_COUNT = 122
const TOOL_CALL_COUNT = 61
const EXECUTOR_TOOL_COUNT = 42
const THREAD_EVENT_COUNT = 233

const MESSAGE_CONTENT_BYTES = 10_620
const THREAD_EVENT_PAYLOAD_BYTES = 4_036
const TOOL_CALL_RESULT_BYTES = 10_975
const EXECUTOR_TOOL_SCHEMA_BYTES = 2_235

export const slowReconnectActor = actor({
	state: { runCount: 0 },
	db: db({
		onMigrate: async (database) => {
			await createSlowReconnectSchema(database)
		},
	}),
	vars: { sql: null } as SlowReconnectVars,
	onWebSocket: (c, ws) => {
		const sock = ws as unknown as WebSocket
		if (sock.readyState === WebSocket.OPEN) {
			sock.send('pong')
		}

		ws.addEventListener('message', (event) => {
			const promise = handleSlowReconnectWebSocketMessage(c, sock, event.data)
			void c.keepAwake(promise)
		})
	},
	actions: {
		prepare: async (c) => {
			await createSlowReconnectSchema(c.db)
			return await seedSlowReconnectData(c.db)
		},
		reproReconnect: async (c, clientId?: string) => {
			c.vars.sql ??= createSlowReconnectDb(c.db)
			c.state.runCount++
			return await runReconnectRepro(c.vars.sql, clientId ?? `action-${c.state.runCount}`, 0)
		},
		getRunCount: (c) => c.state.runCount,
		sleep: (c) => {
			c.sleep()
			return true
		},
	},
})

async function handleSlowReconnectWebSocketMessage(
	c: { db: RawRivetDB; vars: SlowReconnectVars; state: { runCount: number } },
	sock: WebSocket,
	data: unknown,
): Promise<void> {
	if (data === 'ping') {
		if (sock.readyState === WebSocket.OPEN) {
			sock.send('pong')
		}
		return
	}

	let trigger: SlowReconnectRequest['type'] | 'unknown' = 'unknown'
	try {
		const request = parseSlowReconnectRequest(data)
		trigger = request.type
		c.vars.sql ??= createSlowReconnectDb(c.db)
		c.state.runCount++

		if (request.type === 'client_resume') {
			const startedAt = performance.now()
			const result = await runCatchupSnapshot(c.vars.sql, request.version)
			sendJSON(sock, {
				type: 'slow_reconnect_result',
				trigger: request.type,
				totalMs: Math.round(performance.now() - startedAt),
				results: [result],
			})
			return
		}

		const clientId =
			request.type === 'executor_connect'
				? request.clientId
				: (request.clientId ?? `slow-reconnect-${c.state.runCount}`)
		const staggerHandleMs = request.type === 'repro_reconnect' ? (request.staggerHandleMs ?? 0) : 0
		const result = await runReconnectRepro(c.vars.sql, clientId, staggerHandleMs)

		if (request.type === 'executor_connect') {
			sendJSON(sock, {
				type: 'executor_connected',
				executorId: clientId,
				registeredToolCount: EXECUTOR_TOOL_COUNT,
				guidanceInventory: [],
				resumeBootstrap: true,
			})
		}

		sendJSON(sock, {
			type: 'slow_reconnect_result',
			trigger: request.type,
			...result,
		})
	} catch (error) {
		sendJSON(sock, {
			type: 'slow_reconnect_error',
			trigger,
			error: error instanceof Error ? error.message : String(error),
		})
	}
}

function parseSlowReconnectRequest(data: unknown): SlowReconnectRequest {
	if (typeof data !== 'string') {
		throw new Error('slowReconnectActor request must be a string')
	}
	const parsed = JSON.parse(data) as unknown
	if (!parsed || typeof parsed !== 'object') {
		throw new Error('slowReconnectActor request must be an object')
	}
	const request = parsed as Record<string, unknown>
	if (request.type === 'client_resume') {
		return { type: 'client_resume', version: numberField(request, 'version') }
	}
	if (request.type === 'executor_connect') {
		const executorType = request.executorType
		return {
			type: 'executor_connect',
			clientId: stringField(request, 'clientId'),
			...(executorType === 'local-client' ||
				executorType === 'sandbox' ||
				executorType === 'virtual'
				? { executorType }
				: {}),
		}
	}
	if (request.type === 'repro_reconnect') {
		return {
			type: 'repro_reconnect',
			...(typeof request.clientId === 'string' ? { clientId: request.clientId } : {}),
			...(typeof request.staggerHandleMs === 'number'
				? { staggerHandleMs: request.staggerHandleMs }
				: {}),
		}
	}
	throw new Error(`Unknown slowReconnectActor request type: ${String(request.type)}`)
}

function stringField(record: Record<string, unknown>, field: string): string {
	const value = record[field]
	if (typeof value !== 'string' || value.length === 0) {
		throw new Error(`slowReconnectActor request ${field} must be a non-empty string`)
	}
	return value
}

function numberField(record: Record<string, unknown>, field: string): number {
	const value = record[field]
	if (typeof value !== 'number' || !Number.isFinite(value)) {
		throw new Error(`slowReconnectActor request ${field} must be a finite number`)
	}
	return value
}

function sendJSON(
	sock: WebSocket,
	message: SlowReconnectResultMessage | SlowReconnectErrorMessage | object,
): void {
	if (sock.readyState === WebSocket.OPEN) {
		sock.send(JSON.stringify(message))
	}
}

function createSlowReconnectDb(db: RawRivetDB): Db {
	return createDb(async <T = Record<string, SQLPrimitive>>(
		query: string,
		...values: SQLPrimitive[]
	): Promise<T[]> => {
		const converted = values.map((value) =>
			typeof value === 'boolean' ? (value ? 1 : 0) : value,
		)
		return (await db.execute(query, ...converted)) as T[]
	})
}

async function runReconnectRepro(
	sql: Db,
	clientId: string,
	staggerHandleMs: number,
): Promise<Omit<SlowReconnectResultMessage, 'type' | 'trigger'>> {
	const startedAt = performance.now()
	const buildToolPlanContext = await runBuildToolPlanContext(sql)
	const catchupSnapshot = await runCatchupSnapshot(sql, 0)
	const recoverToolCalls = await runRecoverToolCalls(sql)
	const handleExecutorConnect = await delay(staggerHandleMs).then(() =>
		runHandleExecutorConnect(sql, clientId),
	)

	const results = [
		handleExecutorConnect,
		buildToolPlanContext,
		catchupSnapshot,
		recoverToolCalls,
	]
	return {
		totalMs: Math.round(performance.now() - startedAt),
		results,
	}
}

async function runHandleExecutorConnect(
	sql: Db,
	clientId: string,
): Promise<SlowReconnectWorkloadResult> {
	const startedAt = performance.now()
	const steps: SlowReconnectStep[] = []
	const nextSeq = await sql.withTransaction(async (tx) => {
		const latestExecutor = await timedQuery(
			tx,
			steps,
			'load-latest-executor-id',
			`SELECT executor_id FROM executor_tools ORDER BY updated_at DESC LIMIT 1`,
		)
		const latestExecutorId = String(latestExecutor[0]?.executor_id ?? 'seed-executor')
		await timedQuery(
			tx,
			steps,
			'select-cached-executor-tools',
			`SELECT tool_name, schema FROM executor_tools WHERE executor_id = ? ORDER BY tool_name ASC`,
			latestExecutorId,
		)
		const executorType = await timedQuery(
			tx,
			steps,
			'select-executor-type',
			`SELECT value FROM thread_meta_kv WHERE key = 'executor_type'`,
		)
		if (!executorType[0]?.value) {
			await timedQuery(
				tx,
				steps,
				'set-executor-type',
				`INSERT OR REPLACE INTO thread_meta_kv (key, value, updated_at) VALUES ('executor_type', ?, ?)`,
				'local-client',
				new Date().toISOString(),
			)
		}
		const sandboxIntent = await timedQuery(
			tx,
			steps,
			'select-sandbox-intent',
			`SELECT value FROM thread_meta_kv WHERE key = 'sandbox_intent'`,
		)
		if (hasPendingLaunch(sandboxIntent[0]?.value)) {
			await timedQuery(
				tx,
				steps,
				'clear-pending-launch',
				`UPDATE thread_meta_kv SET value = ?, updated_at = ? WHERE key = 'sandbox_intent'`,
				JSON.stringify({ spec: null, pendingLaunch: null }),
				new Date().toISOString(),
			)
		}
		const seqRows = await timedQuery(
			tx,
			steps,
			'select-next-thread-event-seq',
			`SELECT COALESCE(MAX(seq), 0) + 1 AS seq FROM thread_events`,
		)
		const seq = Number(seqRows[0]?.seq ?? 1)
		await timedQuery(
			tx,
			steps,
			'insert-executor-connected-event',
			`INSERT INTO thread_events (seq, event_type, payload, created_at) VALUES (?, ?, ?, ?)`,
			seq,
			'executor_connected',
			JSON.stringify({ type: 'executor_connected', executorId: clientId }),
			new Date().toISOString(),
		)
		return seq
	})
	steps.push({
		name: 'transaction-total',
		durationMs: Math.round(performance.now() - startedAt),
		rowCount: nextSeq,
	})
	return {
		name: 'handle-executor-connect',
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	}
}

async function runBuildToolPlanContext(sql: Db): Promise<SlowReconnectWorkloadResult> {
	const startedAt = performance.now()
	const steps: SlowReconnectStep[] = []
	const latestExecutor = await timedQuery(
		sql,
		steps,
		'load-latest-executor-id',
		`SELECT executor_id FROM executor_tools ORDER BY updated_at DESC LIMIT 1`,
	)
	const latestExecutorId = String(latestExecutor[0]?.executor_id ?? 'seed-executor')
	await timedQuery(
		sql,
		steps,
		'select-executor-tools',
		`SELECT tool_name, schema FROM executor_tools WHERE executor_id = ? ORDER BY tool_name ASC`,
		latestExecutorId,
	)
	await timedQuery(
		sql,
		steps,
		'count-uncancelled-top-level',
		`SELECT COUNT(*) as count FROM messages WHERE cancelled = 0 AND parent_tool_use_id IS NULL`,
	)
	const unresolvedRows = await timedQuery(
		sql,
		steps,
		'find-unresolved-assistant-message',
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
	)
	const unresolvedMessageId = unresolvedRows[0]?.message_id
	if (typeof unresolvedMessageId === 'string') {
		await timedQuery(
			sql,
			steps,
			'get-persisted-tool-result-ids',
			`SELECT tool_result.tool_use_id
				FROM message_tool_refs AS tool_result
				JOIN messages AS tool_result_message
					ON tool_result_message.message_id = tool_result.source_message_id
				WHERE tool_result.assistant_message_id = ?
					AND tool_result.block_type = 'tool_result'
					AND tool_result.cancelled = 0
					AND tool_result_message.parent_tool_use_id IS NULL`,
			unresolvedMessageId,
		)
		await timedQuery(
			sql,
			steps,
			'get-tool-calls-by-message-id',
			`SELECT * FROM tool_calls WHERE message_id = ?`,
			unresolvedMessageId,
		)
	}
	await timedQuery(
		sql,
		steps,
		'is-last-message-cancelled-assistant',
		`SELECT role, cancelled FROM messages
			WHERE parent_tool_use_id IS NULL
			ORDER BY created_at DESC
			LIMIT 1`,
	)
	await timedQuery(
		sql,
		steps,
		'get-last-uncancelled',
		`SELECT m.* FROM messages m
			WHERE m.cancelled = 0 AND m.parent_tool_use_id IS NULL
			ORDER BY m.created_at DESC
			LIMIT 1`,
	)
	return {
		name: 'build-tool-plan-context',
		totalMs: Math.round(performance.now() - startedAt),
		steps,
	}
}

async function runCatchupSnapshot(sql: Db, version: number): Promise<SlowReconnectWorkloadResult> {
	const startedAt = performance.now()
	const steps: SlowReconnectStep[] = []

	await timedQuery(
		sql,
		steps,
		'thread-events-list-since-version',
		`SELECT seq, event_type, payload, created_at FROM thread_events WHERE seq > ? ORDER BY seq ASC`,
		version,
	)
	await timedQuery(
		sql,
		steps,
		'environment-snapshot',
		`SELECT snapshot FROM environment_snapshot WHERE id = 1`,
	)
	await timedQuery(
		sql,
		steps,
		'thread-settings-snapshot',
		`SELECT settings FROM thread_settings_snapshot WHERE id = 1`,
	)
	await timedQuery(sql, steps, 'retry-state', `SELECT * FROM retry_state WHERE id = 1`)
	await timedQuery(
		sql,
		steps,
		'queued-messages',
		`SELECT * FROM queued_messages ORDER BY created_at ASC`,
	)
	await timedQuery(
		sql,
		steps,
		'executor-artifacts',
		`SELECT artifact_key, data_type, length(content_base64) AS bytes, tool_call_id, updated_at FROM executor_artifacts ORDER BY updated_at ASC`,
	)
	await timedQuery(sql, steps, 'tool-approvals', `SELECT * FROM tool_approvals ORDER BY timestamp ASC`)
	await timedQuery(
		sql,
		steps,
		'compaction-summaries',
		`SELECT cut_message_id, created_at FROM compaction_summaries ORDER BY created_at ASC`,
	)
	await timedQuery(
		sql,
		steps,
		'executor-status',
		`SELECT value FROM thread_meta_kv WHERE key = 'executor_status'`,
	)

	steps.sort((a, b) => b.durationMs - a.durationMs)
	return { name: 'catchup-snapshot', totalMs: Math.round(performance.now() - startedAt), steps }
}

async function runRecoverToolCalls(sql: Db): Promise<SlowReconnectWorkloadResult> {
	const startedAt = performance.now()
	const steps: SlowReconnectStep[] = []
	await timedQuery(
		sql,
		steps,
		'hydrate-tool-progress',
		`SELECT id, progress
			FROM tool_calls
			WHERE progress IS NOT NULL
				AND state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	)
	await timedQuery(
		sql,
		steps,
		'get-pending-tool-calls',
		`SELECT * FROM tool_calls
			WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')
			ORDER BY issued_at ASC`,
	)
	await timedQuery(
		sql,
		steps,
		'get-next-tool-expiry',
		`SELECT MIN(expires_at) AS expires_at
			FROM tool_calls
			WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	)
	return { name: 'recover-tool-calls', totalMs: Math.round(performance.now() - startedAt), steps }
}

async function timedQuery<T = Record<string, SQLPrimitive>>(
	sql: Db,
	steps: SlowReconnectStep[],
	name: string,
	query: string,
	...values: SQLPrimitive[]
): Promise<T[]> {
	const startedAt = performance.now()
	const rows = await sql<T>(query, ...values)
	steps.push({
		name,
		durationMs: Math.round(performance.now() - startedAt),
		rowCount: rows.length,
	})
	return rows
}

function hasPendingLaunch(value: unknown): boolean {
	if (typeof value !== 'string' || value.length === 0) {
		return false
	}
	try {
		const parsed = JSON.parse(value) as { pendingLaunch?: unknown }
		return parsed.pendingLaunch !== null && parsed.pendingLaunch !== undefined
	} catch {
		return false
	}
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)))
}

async function createSlowReconnectSchema(database: RawRivetDB): Promise<void> {
	await database.execute(`CREATE TABLE IF NOT EXISTS executor_tools (
		executor_id TEXT NOT NULL,
		tool_name TEXT NOT NULL,
		schema TEXT NOT NULL,
		updated_at TEXT NOT NULL,
		PRIMARY KEY (executor_id, tool_name)
	)`)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_executor_tools_executor ON executor_tools(executor_id)`,
	)
	await database.execute(`CREATE TABLE IF NOT EXISTS thread_meta_kv (
		key TEXT PRIMARY KEY,
		value TEXT,
		updated_at TEXT NOT NULL
	)`)
	await database.execute(`CREATE TABLE IF NOT EXISTS thread_events (
		seq INTEGER PRIMARY KEY,
		event_type TEXT NOT NULL,
		payload TEXT NOT NULL,
		created_at TEXT NOT NULL DEFAULT (datetime('now'))
	)`)
	await database.execute(`CREATE INDEX IF NOT EXISTS idx_thread_events_seq ON thread_events(seq)`)
	await database.execute(`CREATE TABLE IF NOT EXISTS message_added_events (
		message_id TEXT PRIMARY KEY,
		seq INTEGER NOT NULL UNIQUE
	)`)
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
	)`)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_role_cancelled_created_at ON messages(parent_tool_use_id, role, cancelled, created_at DESC)`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_cancelled_created_at ON messages(parent_tool_use_id, cancelled, created_at DESC)`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_parent_created_at ON messages(parent_tool_use_id, created_at DESC)`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_messages_role_created_at ON messages(role, created_at)`,
	)
	await database.execute(`CREATE TABLE IF NOT EXISTS message_tool_refs (
		source_message_id TEXT NOT NULL,
		assistant_message_id TEXT NOT NULL,
		tool_use_id TEXT NOT NULL,
		block_type TEXT NOT NULL CHECK(block_type IN ('tool_use', 'tool_result')),
		cancelled INTEGER NOT NULL DEFAULT 0,
		PRIMARY KEY (source_message_id, block_type, tool_use_id)
	)`)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_assistant_lookup ON message_tool_refs(assistant_message_id, block_type, cancelled, tool_use_id)`,
	)
	await database.execute(
		`CREATE UNIQUE INDEX IF NOT EXISTS idx_message_tool_refs_live_tool_result ON message_tool_refs(assistant_message_id, tool_use_id) WHERE block_type = 'tool_result' AND cancelled = 0`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_source_message ON message_tool_refs(source_message_id)`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_message_tool_refs_tool_use_lookup ON message_tool_refs(tool_use_id, assistant_message_id) WHERE block_type = 'tool_use' AND cancelled = 0`,
	)
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
	)`)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_message_id ON tool_calls(message_id)`,
	)
	await database.execute(`CREATE INDEX IF NOT EXISTS idx_tool_calls_state ON tool_calls(state)`)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_calls_expires_at ON tool_calls(expires_at) WHERE state IN ('queued', 'pending_reconnect', 'pending_ack', 'running')`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS environment_snapshot (id INTEGER PRIMARY KEY CHECK (id = 1), snapshot TEXT NOT NULL, updated_at TEXT NOT NULL)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS thread_settings_snapshot (id INTEGER PRIMARY KEY CHECK (id = 1), settings TEXT NOT NULL, updated_at TEXT NOT NULL)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS retry_state (id INTEGER PRIMARY KEY CHECK (id = 1), attempt INTEGER NOT NULL DEFAULT 0, scheduled_at INTEGER NOT NULL, reason TEXT NOT NULL)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS queued_messages (message_id TEXT PRIMARY KEY, content TEXT NOT NULL, user_state TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), steer INTEGER NOT NULL DEFAULT 0, user_meta TEXT)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS executor_artifacts (artifact_key TEXT PRIMARY KEY, data_type TEXT NOT NULL, content_base64 TEXT NOT NULL, tool_call_id TEXT, updated_at TEXT NOT NULL)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS tool_approvals (id TEXT PRIMARY KEY, tool_call_id TEXT NOT NULL UNIQUE, tool_name TEXT NOT NULL, args TEXT NOT NULL, reason TEXT, to_allow TEXT, context TEXT NOT NULL CHECK(context IN ('thread', 'subagent')), subagent_tool_name TEXT, parent_tool_call_id TEXT, timestamp INTEGER NOT NULL, matched_rule TEXT, rule_source TEXT CHECK(rule_source IN ('user', 'built-in')))`,
	)
	await database.execute(
		`CREATE INDEX IF NOT EXISTS idx_tool_approvals_timestamp ON tool_approvals(timestamp)`,
	)
	await database.execute(
		`CREATE TABLE IF NOT EXISTS compaction_summaries (summary_id TEXT PRIMARY KEY, summary_text TEXT NOT NULL, cut_message_id TEXT NOT NULL, created_at TEXT NOT NULL)`,
	)
}

async function seedSlowReconnectData(database: RawRivetDB): Promise<{
	seeded: boolean
	messages: number
	toolCalls: number
	threadEvents: number
}> {
	const existing = await database.execute(`SELECT COUNT(*) AS count FROM messages`)
	if (Number(existing[0]?.count ?? 0) > 0) {
		const [toolCalls] = await database.execute(`SELECT COUNT(*) AS count FROM tool_calls`)
		const [threadEvents] = await database.execute(`SELECT COUNT(*) AS count FROM thread_events`)
		return {
			seeded: false,
			messages: Number(existing[0]?.count ?? 0),
			toolCalls: Number(toolCalls?.count ?? 0),
			threadEvents: Number(threadEvents?.count ?? 0),
		}
	}

	// Randomize the seeded data volume so each actor's database differs
	// substantially in size. This exercises the reconnect repro across a wide
	// range of catch-up payload sizes instead of a single fixed shape.
	const vary = (base: number, min = 1) =>
		Math.max(min, Math.round(base * (0.25 + Math.random() * 3.75)))
	const messageCount = vary(MESSAGE_COUNT, 2)
	const messageToolRefCount = vary(MESSAGE_TOOL_REF_COUNT, 2)
	const toolCallCount = vary(TOOL_CALL_COUNT)
	const executorToolCount = vary(EXECUTOR_TOOL_COUNT)
	const threadEventCount = vary(THREAD_EVENT_COUNT)
	const assistantSpan = Math.max(1, Math.floor(messageCount / 2))

	const now = new Date('2026-05-16T03:58:18.661Z').getTime()
	const text = (size: number) => 'x'.repeat(size)
	const isoAt = (index: number) => new Date(now + index * 1_000).toISOString()

	await batchInsert(database, `INSERT INTO thread_meta_kv (key, value, updated_at)`, [
		['executor_type', 'local-client', isoAt(0)],
		['sandbox_intent', JSON.stringify({ spec: null, pendingLaunch: null }), isoAt(0)],
		['executor_status', JSON.stringify({ available: true, message: 'ready' }), isoAt(0)],
	])

	const messageRows: unknown[][] = []
	for (let index = 1; index <= messageCount; index++) {
		const role = index % 2 === 0 ? 'assistant' : 'user'
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
		])
	}
	await batchInsert(
		database,
		`INSERT INTO messages (message_id, role, content, meta, user_state, created_at, cancelled, read_at, parent_tool_use_id, tool_result_for_message_id)`,
		messageRows,
		20,
	)

	const messageToolRefRows: unknown[][] = []
	for (let index = 0; index < messageToolRefCount / 2; index++) {
		const assistantIndex = 2 + (index % assistantSpan) * 2
		const sourceIndex = Math.max(1, assistantIndex - 1)
		const resultIndex = Math.min(messageCount, assistantIndex + 1)
		const toolUseId = toolUseID(index + 1)
		messageToolRefRows.push([
			messageId(sourceIndex),
			messageId(assistantIndex),
			toolUseId,
			'tool_use',
			0,
		])
		messageToolRefRows.push([
			messageId(resultIndex),
			messageId(assistantIndex),
			toolUseId,
			'tool_result',
			0,
		])
	}
	await batchInsert(
		database,
		`INSERT INTO message_tool_refs (source_message_id, assistant_message_id, tool_use_id, block_type, cancelled)`,
		messageToolRefRows,
		50,
	)

	const toolCallRows: unknown[][] = []
	for (let index = 1; index <= toolCallCount; index++) {
		const assistantIndex = 2 + ((index - 1) % assistantSpan) * 2
		toolCallRows.push([
			toolUseID(index),
			`provider-${index}`,
			`tool_${index % 21}`,
			JSON.stringify({ path: `/tmp/file-${index}` }),
			'seed-executor',
			messageId(assistantIndex),
			isoAt(index),
			null,
			'completed',
			JSON.stringify({
				ok: true,
				run: { status: 'done', result: text(TOOL_CALL_RESULT_BYTES) },
			}),
			null,
			isoAt(index + 100),
		])
	}
	await batchInsert(
		database,
		`INSERT INTO tool_calls (id, provider_tool_use_id, tool_name, args, executor_id, message_id, issued_at, expires_at, state, result, progress, completed_at)`,
		toolCallRows,
		20,
	)

	const executorToolRows: unknown[][] = []
	for (let index = 1; index <= executorToolCount; index++) {
		const schema = JSON.stringify({
			name: `tool_${index}`,
			description: text(EXECUTOR_TOOL_SCHEMA_BYTES),
			input_schema: { type: 'object', properties: {} },
		})
		executorToolRows.push(['seed-executor', `tool_${index}`, schema, isoAt(index)])
	}
	await batchInsert(
		database,
		`INSERT INTO executor_tools (executor_id, tool_name, schema, updated_at)`,
		executorToolRows,
		42,
	)

	const threadEventRows: unknown[][] = []
	for (let index = 1; index <= threadEventCount; index++) {
		threadEventRows.push([
			index,
			index % 3 === 0 ? 'message_added' : 'agent_state_changed',
			JSON.stringify({ type: 'seed_event', body: text(THREAD_EVENT_PAYLOAD_BYTES) }),
			isoAt(index),
		])
	}
	await batchInsert(
		database,
		`INSERT INTO thread_events (seq, event_type, payload, created_at)`,
		threadEventRows,
		25,
	)

	const messageAddedRows: unknown[][] = []
	for (let index = 1; index <= messageCount; index++) {
		messageAddedRows.push([messageId(index), index])
	}
	await batchInsert(
		database,
		`INSERT INTO message_added_events (message_id, seq)`,
		messageAddedRows,
		50,
	)

	await database.execute(
		`INSERT INTO environment_snapshot (id, snapshot, updated_at) VALUES (1, ?, ?)`,
		JSON.stringify({ cwd: '/workspace', body: text(3_620) }),
		isoAt(0),
	)
	await database.execute(
		`INSERT INTO thread_settings_snapshot (id, settings, updated_at) VALUES (1, ?, ?)`,
		JSON.stringify({ maxTokens: 20_000, body: text(55) }),
		isoAt(0),
	)
	return {
		seeded: true,
		messages: messageCount,
		toolCalls: toolCallCount,
		threadEvents: threadEventCount,
	}
}

async function batchInsert(
	database: RawRivetDB,
	insertPrefix: string,
	rows: unknown[][],
	batchSize = 100,
): Promise<void> {
	if (rows.length === 0) {
		return
	}
	const columnCount = rows[0]?.length ?? 0
	if (columnCount === 0) {
		return
	}
	const rowPlaceholder = `(${'?,'.repeat(columnCount).slice(0, -1)})`
	for (let index = 0; index < rows.length; index += batchSize) {
		const chunk = rows.slice(index, index + batchSize)
		const values = chunk.map(() => rowPlaceholder).join(',')
		const bindings = chunk.flat()
		await database.execute(`${insertPrefix} VALUES ${values}`, ...bindings)
	}
}

function messageId(index: number): string {
	return `M-${String(index).padStart(22, '0')}`
}

function toolUseID(index: number): string {
	return `toolu_${String(index).padStart(22, '0')}`
}

export const registry = setup({
	use: { slowReconnectActor },
	maxIncomingMessageSize: 5 * 1024 * 1024,
	maxOutgoingMessageSize: 5 * 1024 * 1024,
})

if (import.meta.main) {
	registry.start()
}
