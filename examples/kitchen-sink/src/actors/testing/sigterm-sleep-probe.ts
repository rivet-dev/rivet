import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export const DEFAULT_ON_SLEEP_DURATION_MS = 5_000;
export const DEFAULT_ON_SLEEP_TICK_MS = 1_000;
const SLEEP_TIMEOUT_MS = 10 * 60 * 1000;
const SLEEP_GRACE_PERIOD_MS = 30 * 60 * 1000;
const ACTOR_STOPPED_CLOSE_CODE = 1000;
const ACTOR_STOPPED_CLOSE_REASON = "actor stopped";

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function formatError(error: unknown): string {
	if (error instanceof Error) return error.stack ?? error.message;
	return String(error);
}

/** Probe status that must survive sleep so `getProof` can report it after a wake. */
interface ProbeStatus {
	label: string;
	wakeCount: number;
	sleepCount: number;
	onSleepDurationMs: number;
	onSleepTickMs: number;
	onSleepStartedAt: number | null;
	onSleepAsyncFinishedAt: number | null;
	onSleepFinishedAt: number | null;
	onSleepLastError: string | null;
}

type ProbeStatusRow = {
	label: string;
	wake_count: number;
	sleep_count: number;
	on_sleep_duration_ms: number;
	on_sleep_tick_ms: number;
	on_sleep_started_at: number | null;
	on_sleep_async_finished_at: number | null;
	on_sleep_finished_at: number | null;
	on_sleep_last_error: string | null;
};

/**
 * Runs a statement that selects the single probe status row, or updates it
 * with `RETURNING *`, and returns the row.
 */
async function queryProbeStatus(
	database: RawAccess,
	sql: string,
	...args: unknown[]
): Promise<ProbeStatus> {
	const [row] = await database.execute<ProbeStatusRow>(sql, ...args);
	if (!row) {
		throw new Error("probe_status row is missing");
	}
	return {
		label: row.label,
		wakeCount: row.wake_count,
		sleepCount: row.sleep_count,
		onSleepDurationMs: row.on_sleep_duration_ms,
		onSleepTickMs: row.on_sleep_tick_ms,
		onSleepStartedAt: row.on_sleep_started_at,
		onSleepAsyncFinishedAt: row.on_sleep_async_finished_at,
		onSleepFinishedAt: row.on_sleep_finished_at,
		onSleepLastError: row.on_sleep_last_error,
	};
}

function loadProbeStatus(database: RawAccess): Promise<ProbeStatus> {
	return queryProbeStatus(
		database,
		"SELECT * FROM probe_status WHERE id = 1",
	);
}

/** Appends one event to the proof log that `getProof` returns. */
async function logProbeEvent(
	database: RawAccess,
	event: string,
	sleepCount: number,
	detail: string,
	createdAt: number,
): Promise<void> {
	await database.execute(
		"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
		event,
		sleepCount,
		detail,
		createdAt,
	);
}

export const sigtermSleepProbe = actor({
	// Live connection counters only describe the current run.
	createVars: () => ({
		websockets: new Set<UniversalWebSocket>(),
		connectionCount: 0,
		messageCount: 0,
	}),
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS sigterm_sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					sleep_count INTEGER NOT NULL,
					detail TEXT,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS probe_status (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					label TEXT NOT NULL,
					wake_count INTEGER NOT NULL,
					sleep_count INTEGER NOT NULL,
					on_sleep_duration_ms INTEGER NOT NULL,
					on_sleep_tick_ms INTEGER NOT NULL,
					on_sleep_started_at INTEGER,
					on_sleep_async_finished_at INTEGER,
					on_sleep_finished_at INTEGER,
					on_sleep_last_error TEXT
				)
			`);
			await database.execute(
				`INSERT OR IGNORE INTO probe_status
					(id, label, wake_count, sleep_count, on_sleep_duration_ms, on_sleep_tick_ms)
				VALUES (1, 'unprepared', 0, 0, ?, ?)`,
				DEFAULT_ON_SLEEP_DURATION_MS,
				DEFAULT_ON_SLEEP_TICK_MS,
			);
		},
	}),
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.add(websocket);
		c.vars.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			c.vars.messageCount += 1;
			const data = event.data;
			if (typeof data !== "string") return;

			try {
				const parsed = JSON.parse(data);
				if (parsed.type === "ping") {
					websocket.send(
						JSON.stringify({
							type: "pong",
							connectionId,
							messageCount: c.vars.messageCount,
							timestamp: Date.now(),
						}),
					);
					return;
				}
			} catch {}

			websocket.send(
				JSON.stringify({
					type: "echo",
					connectionId,
					received: data,
					messageCount: c.vars.messageCount,
					timestamp: Date.now(),
				}),
			);
		});

		websocket.addEventListener("close", (event) => {
			c.vars.websockets.delete(websocket);
			c.vars.connectionCount -= 1;
			c.log.info({
				msg: "sigterm sleep probe websocket closed",
				connectionId,
				connectionCount: c.vars.connectionCount,
				code: event.code,
				reason: event.reason,
			});
		});

		// The listeners are attached before this read, so no message or close
		// event is missed while the label loads.
		const sendWelcome = async () => {
			const status = await loadProbeStatus(c.db);
			c.log.info({
				msg: "sigterm sleep probe websocket connected",
				label: status.label,
				connectionId,
				connectionCount: c.vars.connectionCount,
			});
			if (websocket.readyState !== 1) return;
			websocket.send(
				JSON.stringify({
					type: "welcome",
					connectionId,
					label: status.label,
					connectionCount: c.vars.connectionCount,
				}),
			);
		};
		sendWelcome().catch((error) => {
			c.log.error({
				msg: "sigterm sleep probe failed to send welcome",
				connectionId,
				error: formatError(error),
			});
		});
	},
	onWake: async (c) => {
		const status = await queryProbeStatus(
			c.db,
			"UPDATE probe_status SET wake_count = wake_count + 1 WHERE id = 1 RETURNING *",
		);
		c.log.info({
			msg: "sigterm sleep probe onWake",
			label: status.label,
			wakeCount: status.wakeCount,
			sleepCount: status.sleepCount,
		});
		await logProbeEvent(
			c.db,
			"wake",
			status.sleepCount,
			`wake-${status.wakeCount}`,
			Date.now(),
		);
	},
	onSleep: async (c) => {
		const startedAt = Date.now();
		const status = await queryProbeStatus(
			c.db,
			`UPDATE probe_status SET
				sleep_count = sleep_count + 1,
				on_sleep_started_at = ?,
				on_sleep_async_finished_at = NULL,
				on_sleep_finished_at = NULL,
				on_sleep_last_error = NULL
			WHERE id = 1
			RETURNING *`,
			startedAt,
		);
		const { label, sleepCount, onSleepDurationMs, onSleepTickMs } = status;

		c.log.info({
			msg: "sigterm sleep probe onSleep start",
			label,
			sleepCount,
			onSleepDurationMs,
			onSleepTickMs,
		});

		try {
			for (const websocket of c.vars.websockets) {
				if (websocket.readyState !== 1) continue;
				websocket.send(
					JSON.stringify({
						type: "onSleepStarted",
						sleepCount,
						onSleepDurationMs,
						onSleepTickMs,
						timestamp: startedAt,
					}),
				);
			}

			await logProbeEvent(
				c.db,
				"on-sleep-start",
				sleepCount,
				label,
				startedAt,
			);

			const deadline = startedAt + onSleepDurationMs;
			let tickIndex = 0;
			while (Date.now() < deadline) {
				const waitMs = Math.min(
					onSleepTickMs,
					Math.max(0, deadline - Date.now()),
				);
				if (waitMs > 0) await sleep(waitMs);

				tickIndex += 1;
				const tickAt = Date.now();
				const detail = `tick=${tickIndex} elapsed-ms=${tickAt - startedAt}`;
				await logProbeEvent(
					c.db,
					"on-sleep-tick",
					sleepCount,
					detail,
					tickAt,
				);
				c.log.info({
					msg: "sigterm sleep probe onSleep tick",
					label,
					sleepCount,
					tickIndex,
					elapsedMs: tickAt - startedAt,
				});

				for (const websocket of c.vars.websockets) {
					if (websocket.readyState !== 1) continue;
					websocket.send(
						JSON.stringify({
							type: "onSleepTick",
							sleepCount,
							tickIndex,
							elapsedMs: tickAt - startedAt,
							timestamp: tickAt,
						}),
					);
				}
			}

			const asyncFinishedAt = Date.now();
			await c.db.execute(
				"UPDATE probe_status SET on_sleep_async_finished_at = ? WHERE id = 1",
				asyncFinishedAt,
			);
			await logProbeEvent(
				c.db,
				"on-sleep-after-await",
				sleepCount,
				`delay-ms=${asyncFinishedAt - startedAt}`,
				asyncFinishedAt,
			);

			const finishedAt = Date.now();
			await c.db.execute(
				"UPDATE probe_status SET on_sleep_finished_at = ? WHERE id = 1",
				finishedAt,
			);
			await logProbeEvent(
				c.db,
				"on-sleep-finish",
				sleepCount,
				label,
				finishedAt,
			);

			for (const websocket of c.vars.websockets) {
				if (websocket.readyState !== 1) continue;
				websocket.send(
					JSON.stringify({
						type: "onSleepFinished",
						sleepCount,
						elapsedMs: finishedAt - startedAt,
						timestamp: finishedAt,
					}),
				);
				websocket.close(
					ACTOR_STOPPED_CLOSE_CODE,
					ACTOR_STOPPED_CLOSE_REASON,
				);
			}

			c.log.info({
				msg: "sigterm sleep probe onSleep finish",
				label,
				sleepCount,
				elapsedMs: finishedAt - startedAt,
			});
		} catch (error) {
			const message = formatError(error);
			await c.db.execute(
				"UPDATE probe_status SET on_sleep_last_error = ? WHERE id = 1",
				message,
			);
			c.log.error({
				msg: "sigterm sleep probe onSleep error",
				label,
				sleepCount,
				error: message,
			});
			throw error;
		}
	},
	actions: {
		prepare: async (
			c,
			label = `sigterm-sleep-probe-${Date.now()}`,
			onSleepDurationMs = DEFAULT_ON_SLEEP_DURATION_MS,
			onSleepTickMs = DEFAULT_ON_SLEEP_TICK_MS,
		) => {
			if (!Number.isFinite(onSleepDurationMs) || onSleepDurationMs < 0) {
				throw new Error(
					"onSleepDurationMs must be a finite non-negative number",
				);
			}
			if (!Number.isFinite(onSleepTickMs) || onSleepTickMs <= 0) {
				throw new Error(
					"onSleepTickMs must be a finite positive number",
				);
			}
			const status = await queryProbeStatus(
				c.db,
				"UPDATE probe_status SET label = ?, on_sleep_duration_ms = ?, on_sleep_tick_ms = ? WHERE id = 1 RETURNING *",
				label,
				onSleepDurationMs,
				onSleepTickMs,
			);
			await logProbeEvent(
				c.db,
				"prepared",
				status.sleepCount,
				label,
				Date.now(),
			);
			return {
				label: status.label,
				onSleepDurationMs: status.onSleepDurationMs,
				onSleepTickMs: status.onSleepTickMs,
				wakeCount: status.wakeCount,
				sleepCount: status.sleepCount,
				connectionCount: c.vars.connectionCount,
				messageCount: c.vars.messageCount,
			};
		},
		getProof: async (c) => {
			const status = await loadProbeStatus(c.db);
			const rows = await c.db.execute<{
				id: number;
				event: string;
				sleep_count: number;
				detail: string | null;
				created_at: number;
			}>("SELECT * FROM sigterm_sleep_log ORDER BY id");
			return {
				state: {
					label: status.label,
					wakeCount: status.wakeCount,
					sleepCount: status.sleepCount,
					onSleepDurationMs: status.onSleepDurationMs,
					onSleepTickMs: status.onSleepTickMs,
					connectionCount: c.vars.connectionCount,
					messageCount: c.vars.messageCount,
					onSleepStartedAt: status.onSleepStartedAt,
					onSleepAsyncFinishedAt: status.onSleepAsyncFinishedAt,
					onSleepFinishedAt: status.onSleepFinishedAt,
					onSleepLastError: status.onSleepLastError,
				},
				rows,
			};
		},
	},
	options: {
		canHibernateWebSocket: false,
		sleepTimeout: SLEEP_TIMEOUT_MS,
		sleepGracePeriod: SLEEP_GRACE_PERIOD_MS,
	},
});
