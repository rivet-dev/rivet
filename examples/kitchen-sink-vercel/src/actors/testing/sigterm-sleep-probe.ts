import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

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

export const sigtermSleepProbe = actor({
	state: {
		label: "unprepared",
		wakeCount: 0,
		sleepCount: 0,
		onSleepDurationMs: DEFAULT_ON_SLEEP_DURATION_MS,
		onSleepTickMs: DEFAULT_ON_SLEEP_TICK_MS,
		connectionCount: 0,
		messageCount: 0,
		onSleepStartedAt: null as number | null,
		onSleepAsyncFinishedAt: null as number | null,
		onSleepFinishedAt: null as number | null,
		onSleepLastError: null as string | null,
	},
	createVars: () => ({
		websockets: new Set<UniversalWebSocket>(),
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
		},
	}),
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.add(websocket);
		c.state.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		c.log.info({
			msg: "sigterm sleep probe websocket connected",
			label: c.state.label,
			connectionId,
			connectionCount: c.state.connectionCount,
		});

		websocket.send(
			JSON.stringify({
				type: "welcome",
				connectionId,
				label: c.state.label,
				connectionCount: c.state.connectionCount,
			}),
		);

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			c.state.messageCount += 1;
			const data = event.data;
			if (typeof data !== "string") return;

			try {
				const parsed = JSON.parse(data);
				if (parsed.type === "ping") {
					websocket.send(
						JSON.stringify({
							type: "pong",
							connectionId,
							messageCount: c.state.messageCount,
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
					messageCount: c.state.messageCount,
					timestamp: Date.now(),
				}),
			);
		});

		websocket.addEventListener("close", (event) => {
			c.vars.websockets.delete(websocket);
			c.state.connectionCount -= 1;
			c.log.info({
				msg: "sigterm sleep probe websocket closed",
				label: c.state.label,
				connectionId,
				connectionCount: c.state.connectionCount,
				code: event.code,
				reason: event.reason,
			});
		});
	},
	onWake: async (c) => {
		c.state.wakeCount += 1;
		c.log.info({
			msg: "sigterm sleep probe onWake",
			label: c.state.label,
			wakeCount: c.state.wakeCount,
			sleepCount: c.state.sleepCount,
		});
		await c.db.execute(
			"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
			"wake",
			c.state.sleepCount,
			`wake-${c.state.wakeCount}`,
			Date.now(),
		);
	},
	onSleep: async (c) => {
		const sleepCount = c.state.sleepCount + 1;
		const startedAt = Date.now();
		c.state.sleepCount = sleepCount;
		c.state.onSleepStartedAt = startedAt;
		c.state.onSleepAsyncFinishedAt = null;
		c.state.onSleepFinishedAt = null;
		c.state.onSleepLastError = null;

		c.log.info({
			msg: "sigterm sleep probe onSleep start",
			label: c.state.label,
			sleepCount,
			onSleepDurationMs: c.state.onSleepDurationMs,
			onSleepTickMs: c.state.onSleepTickMs,
		});

		try {
			for (const websocket of c.vars.websockets) {
				if (websocket.readyState !== 1) continue;
				websocket.send(
					JSON.stringify({
						type: "onSleepStarted",
						sleepCount,
						onSleepDurationMs: c.state.onSleepDurationMs,
						onSleepTickMs: c.state.onSleepTickMs,
						timestamp: startedAt,
					}),
				);
			}

			await c.db.execute(
				"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
				"on-sleep-start",
				sleepCount,
				c.state.label,
				startedAt,
			);

			const deadline = startedAt + c.state.onSleepDurationMs;
			let tickIndex = 0;
			while (Date.now() < deadline) {
				const waitMs = Math.min(
					c.state.onSleepTickMs,
					Math.max(0, deadline - Date.now()),
				);
				if (waitMs > 0) await sleep(waitMs);

				tickIndex += 1;
				const tickAt = Date.now();
				const detail = `tick=${tickIndex} elapsed-ms=${tickAt - startedAt}`;
				await c.db.execute(
					"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
					"on-sleep-tick",
					sleepCount,
					detail,
					tickAt,
				);
				c.log.info({
					msg: "sigterm sleep probe onSleep tick",
					label: c.state.label,
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
			c.state.onSleepAsyncFinishedAt = asyncFinishedAt;
			await c.db.execute(
				"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
				"on-sleep-after-await",
				sleepCount,
				`delay-ms=${asyncFinishedAt - startedAt}`,
				asyncFinishedAt,
			);

			const finishedAt = Date.now();
			c.state.onSleepFinishedAt = finishedAt;
			await c.db.execute(
				"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
				"on-sleep-finish",
				sleepCount,
				c.state.label,
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
				label: c.state.label,
				sleepCount,
				elapsedMs: finishedAt - startedAt,
			});
		} catch (error) {
			const message = formatError(error);
			c.state.onSleepLastError = message;
			c.log.error({
				msg: "sigterm sleep probe onSleep error",
				label: c.state.label,
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
			c.state.label = label;
			c.state.onSleepDurationMs = onSleepDurationMs;
			c.state.onSleepTickMs = onSleepTickMs;
			await c.db.execute(
				"INSERT INTO sigterm_sleep_log (event, sleep_count, detail, created_at) VALUES (?, ?, ?, ?)",
				"prepared",
				c.state.sleepCount,
				label,
				Date.now(),
			);
			return {
				label: c.state.label,
				onSleepDurationMs: c.state.onSleepDurationMs,
				onSleepTickMs: c.state.onSleepTickMs,
				wakeCount: c.state.wakeCount,
				sleepCount: c.state.sleepCount,
				connectionCount: c.state.connectionCount,
				messageCount: c.state.messageCount,
			};
		},
		getProof: async (c) => {
			const rows = await c.db.execute<{
				id: number;
				event: string;
				sleep_count: number;
				detail: string | null;
				created_at: number;
			}>("SELECT * FROM sigterm_sleep_log ORDER BY id");
			return {
				state: {
					label: c.state.label,
					wakeCount: c.state.wakeCount,
					sleepCount: c.state.sleepCount,
					onSleepDurationMs: c.state.onSleepDurationMs,
					onSleepTickMs: c.state.onSleepTickMs,
					connectionCount: c.state.connectionCount,
					messageCount: c.state.messageCount,
					onSleepStartedAt: c.state.onSleepStartedAt,
					onSleepAsyncFinishedAt: c.state.onSleepAsyncFinishedAt,
					onSleepFinishedAt: c.state.onSleepFinishedAt,
					onSleepLastError: c.state.onSleepLastError,
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
