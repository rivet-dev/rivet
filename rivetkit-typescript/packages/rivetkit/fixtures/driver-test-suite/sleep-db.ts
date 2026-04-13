import type { UniversalWebSocket } from "rivetkit";
import { actor, event, queue } from "rivetkit";
import { db } from "rivetkit/db";
import {
	RAW_WS_HANDLER_DELAY,
	RAW_WS_HANDLER_SLEEP_TIMEOUT,
} from "./sleep";

export const SLEEP_DB_TIMEOUT = 1000;

export const sleepWithDb = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		onSleepDbWriteSuccess: false,
		onSleepDbWriteError: null as string | null,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		try {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
			);
			c.state.onSleepDbWriteSuccess = true;
		} catch (error) {
			c.state.onSleepDbWriteError =
				error instanceof Error ? error.message : String(error);
		}
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				onSleepDbWriteSuccess: c.state.onSleepDbWriteSuccess,
				onSleepDbWriteError: c.state.onSleepDbWriteError,
			};
		},
		getLogEntries: async (c) => {
			const results = await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
			return results;
		},
		insertLogEntry: async (c, event: string) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('${event}', ${Date.now()})`,
			);
		},
		setAlarm: (c, delayMs: number) => {
			c.schedule.after(delayMs, "onAlarm");
		},
		onAlarm: async (c) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('alarm', ${Date.now()})`,
			);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWithSlowScheduledDb = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	actions: {
		scheduleSlowAlarm: (c, delayMs: number, workMs: number) => {
			c.schedule.after(delayMs, "onSlowAlarm", workMs);
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
		onSlowAlarm: async (c, workMs: number) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('slow-alarm-start', ${Date.now()})`,
			);
			await new Promise((resolve) => setTimeout(resolve, workMs));
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('slow-alarm-finish', ${Date.now()})`,
			);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWithDbConn = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	connState: {},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onDisconnect: async (c, _conn) => {
		try {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('disconnect', ${Date.now()})`,
			);
		} catch (error) {
			c.log.warn({
				msg: "onDisconnect db write failed",
				error: error instanceof Error ? error.message : String(error),
			});
		}
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		try {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
			);
		} catch (error) {
			c.log.warn({
				msg: "onSleep db write failed",
				error: error instanceof Error ? error.message : String(error),
			});
		}
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
		getLogEntries: async (c) => {
			const results = await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
			return results;
		},
		insertLogEntry: async (c, event: string) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('${event}', ${Date.now()})`,
			);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWithDbAction = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	connState: {},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	events: {
		sleeping: event<void>(),
	},
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		try {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('sleep-start', ${Date.now()})`,
			);
			c.broadcast("sleeping", undefined);
			await new Promise((resolve) => setTimeout(resolve, 500));
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('sleep-end', ${Date.now()})`,
			);
		} catch (error) {
			c.log.warn({
				msg: "onSleep error",
				error: error instanceof Error ? error.message : String(error),
			});
		}
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
		getLogEntries: async (c) => {
			const results = await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
			return results;
		},
		insertLogEntry: async (c, event: string) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('${event}', ${Date.now()})`,
			);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWaitUntil = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep-start', ${Date.now()})`,
		);
		c.waitUntil((async () => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('waituntil-write', ${Date.now()})`,
			);
		})());
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepNestedWaitUntil = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep-start', ${Date.now()})`,
		);
		c.waitUntil((async () => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('outer-waituntil', ${Date.now()})`,
			);
			// Nested waitUntil inside a waitUntil callback
			c.waitUntil((async () => {
				await c.db.execute(
					`INSERT INTO sleep_log (event, created_at) VALUES ('nested-waituntil', ${Date.now()})`,
				);
			})());
		})());
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepEnqueue = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		enqueueSuccess: false,
		enqueueError: null as string | null,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	queues: {
		work: queue<string>(),
	},
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		try {
			await c.queue.send("work", "enqueued-during-sleep");
			c.state.enqueueSuccess = true;
		} catch (error) {
			c.state.enqueueError =
				error instanceof Error ? error.message : String(error);
		}
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			enqueueSuccess: c.state.enqueueSuccess,
			enqueueError: c.state.enqueueError,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepScheduleAfter = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		// Schedule an alarm during onSleep. It should be persisted
		// but not fire a local timeout during shutdown.
		c.schedule.after(100, "onScheduledAction");
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
		onScheduledAction: async (c) => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('scheduled-action', ${Date.now()})`,
			);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepOnSleepThrows = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep-before-throw', ${Date.now()})`,
		);
		throw new Error("onSleep intentional error");
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWaitUntilRejects = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
		// Register a waitUntil that rejects. Shutdown should still complete.
		c.waitUntil(Promise.reject(new Error("waitUntil intentional rejection")));
		// Also register one that succeeds, to verify it still runs.
		c.waitUntil((async () => {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('waituntil-after-reject', ${Date.now()})`,
			);
		})());
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWaitUntilState = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		waitUntilRan: false,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		c.waitUntil((async () => {
			c.state.waitUntilRan = true;
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('waituntil-state', ${Date.now()})`,
			);
		})());
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			waitUntilRan: c.state.waitUntilRan,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWithRawWs = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
		// Delay so there is a window to attempt raw WS connection during shutdown
		await new Promise((resolve) => setTimeout(resolve, 500));
	},
	onWebSocket: (_c, ws: UniversalWebSocket) => {
		ws.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: SLEEP_DB_TIMEOUT,
	},
});

export const sleepWithRawWsCloseDb = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		closeStarted: 0,
		closeFinished: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	onWebSocket: (c, ws: UniversalWebSocket) => {
		ws.onclose = async () => {
			c.state.closeStarted += 1;
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('close-start', ${Date.now()})`,
			);
			await new Promise((resolve) =>
				setTimeout(resolve, RAW_WS_HANDLER_DELAY),
			);
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('close-finish', ${Date.now()})`,
			);
			c.state.closeFinished += 1;
		};

		ws.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			closeStarted: c.state.closeStarted,
			closeFinished: c.state.closeFinished,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: RAW_WS_HANDLER_SLEEP_TIMEOUT,
	},
});

// Grace period shorter than the handler's async work, so the DB gets
// cleaned up while the handler is still running.
const EXCEEDS_GRACE_HANDLER_DELAY = 2000;
const EXCEEDS_GRACE_PERIOD = 200;
const EXCEEDS_GRACE_SLEEP_TIMEOUT = 100;

export { EXCEEDS_GRACE_HANDLER_DELAY, EXCEEDS_GRACE_PERIOD, EXCEEDS_GRACE_SLEEP_TIMEOUT };

// Number of sequential DB writes the handler performs. The loop runs long
// enough that shutdown (close()) runs between two writes. The write that
// follows close() hits the destroyed DB.
const ACTIVE_DB_WRITE_COUNT = 500;
const ACTIVE_DB_WRITE_DELAY_MS = 5;
const ACTIVE_DB_GRACE_PERIOD = 50;
const ACTIVE_DB_SLEEP_TIMEOUT = 500;

export {
	ACTIVE_DB_WRITE_COUNT,
	ACTIVE_DB_WRITE_DELAY_MS,
	ACTIVE_DB_GRACE_PERIOD,
	ACTIVE_DB_SLEEP_TIMEOUT,
};

// Reproduces the production "disk I/O error" scenario: the handler is
// actively performing sequential DB writes (each one acquires and releases
// the wrapper mutex) when the grace period expires. Between two writes,
// client.close() acquires the mutex, sets closed=true, then calls
// db.close() outside the mutex. The next write acquires the mutex and
// calls ensureOpen() which throws "Database is closed".
//
// Without ensureOpen (as in the production version), the write would
// call db.exec() on the already-closing database concurrently with
// db.close(), producing "disk I/O error" or "cannot start a transaction
// within a transaction".
export const sleepWsActiveDbExceedsGrace = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		writesCompleted: 0,
		writeError: null as string | null,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
	},
	onWebSocket: (c, ws: UniversalWebSocket) => {
		const sendMessage = (payload: unknown) => {
			try {
				const result = (ws as { send(data: string): unknown }).send(
					JSON.stringify(payload),
				);
				void Promise.resolve(result).catch((error) => {
					c.log.warn({
						msg: "websocket send failed during active db write test",
						error:
							error instanceof Error ? error.message : String(error),
					});
				});
			} catch (error) {
				c.log.warn({
					msg: "websocket send failed during active db write test",
					error: error instanceof Error ? error.message : String(error),
				});
			}
		};

		ws.addEventListener("message", async (event: any) => {
			if (event.data !== "start-writes") return;

			sendMessage({ type: "started" });

			// Perform many sequential DB writes. Each write acquires and
			// releases the DB wrapper mutex. Between two writes, the
			// shutdown's client.close() can slip in and close the DB.
			for (let i = 0; i < ACTIVE_DB_WRITE_COUNT; i++) {
				try {
					await c.db.execute(
						`INSERT INTO sleep_log (event, created_at) VALUES ('write-${i}', ${Date.now()})`,
					);
					c.state.writesCompleted = i + 1;
				} catch (error) {
					c.state.writeError =
						error instanceof Error ? error.message : String(error);
					sendMessage({
						type: "error",
						index: i,
						error: c.state.writeError,
					});
					return;
				}

				// Small delay between writes to yield the event loop and
				// allow shutdown tasks to run.
				await new Promise((resolve) =>
					setTimeout(resolve, ACTIVE_DB_WRITE_DELAY_MS),
				);
			}

			sendMessage({ type: "finished" });
		});

		sendMessage({ type: "connected" });
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			writesCompleted: c.state.writesCompleted,
			writeError: c.state.writeError,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: ACTIVE_DB_SLEEP_TIMEOUT,
		sleepGracePeriod: ACTIVE_DB_GRACE_PERIOD,
	},
});

export const sleepWsMessageExceedsGrace = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		messageStarted: 0,
		messageFinished: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	onWebSocket: (c, ws: UniversalWebSocket) => {
		ws.addEventListener("message", async (event: any) => {
			if (event.data !== "slow-db-work") return;

			c.state.messageStarted += 1;

			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('msg-start', ${Date.now()})`,
			);

			ws.send(JSON.stringify({ type: "started" }));

			// Wait longer than the grace period so shutdown times out
			// and cleans up the database while this handler is still running.
			await new Promise((resolve) =>
				setTimeout(resolve, EXCEEDS_GRACE_HANDLER_DELAY),
			);

			// This DB write runs after the grace period expired and
			// #cleanupDatabase already destroyed the SQLite VFS.
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('msg-finish', ${Date.now()})`,
			);

			c.state.messageFinished += 1;
		});

		ws.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			messageStarted: c.state.messageStarted,
			messageFinished: c.state.messageFinished,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: EXCEEDS_GRACE_SLEEP_TIMEOUT,
		sleepGracePeriod: EXCEEDS_GRACE_PERIOD,
	},
});

// Reproduces the "cannot start a transaction within a transaction" error.
// Multiple concurrent WS message handlers do DB writes. The grace period
// is shorter than the handler delay, so the VFS gets destroyed while
// handlers are still running. The first handler's DB write fails
// (leaving a transaction open in SQLite), and subsequent handlers get
// "cannot start a transaction within a transaction".
export const sleepWsConcurrentDbExceedsGrace = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		handlerStarted: 0,
		handlerFinished: 0,
		handlerErrors: [] as string[],
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		try {
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
			);
		} catch {
			// DB may already be torn down
		}
	},
	onWebSocket: (c, ws: UniversalWebSocket) => {
		ws.addEventListener("message", async (event: any) => {
			const data = JSON.parse(String(event.data));
			if (data.type !== "slow-db-work") return;

			const index = data.index ?? 0;
			c.state.handlerStarted += 1;

			// Each handler captures the db reference before awaiting.
			// After the delay, the VFS may be destroyed.
			const dbRef = c.db;

			ws.send(JSON.stringify({ type: "started", index }));

			// Stagger the delay slightly per index so handlers resume at
			// different times relative to VFS teardown.
			await new Promise((resolve) =>
				setTimeout(resolve, EXCEEDS_GRACE_HANDLER_DELAY + index * 50),
			);

			// Use the captured dbRef directly. After VFS teardown, the
			// underlying sqlite connection is broken. The first handler
			// to hit it may get "disk I/O error" (leaving a transaction
			// open), and subsequent handlers may get "cannot start a
			// transaction within a transaction".
			//
			// Do NOT catch the error here. Let it propagate so
			// #trackWebSocketCallback logs the actual error message
			// (visible in test output as "websocket callback failed").
			await dbRef.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('handler-${index}-finish', ${Date.now()})`,
			);
			c.state.handlerFinished += 1;
		});

		ws.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			handlerStarted: c.state.handlerStarted,
			handlerFinished: c.state.handlerFinished,
			handlerErrors: c.state.handlerErrors,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: EXCEEDS_GRACE_SLEEP_TIMEOUT,
		sleepGracePeriod: EXCEEDS_GRACE_PERIOD,
	},
});

export const sleepWithRawWsCloseDbListener = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		closeStarted: 0,
		closeFinished: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sleep_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					event TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onWake: async (c) => {
		c.state.startCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('wake', ${Date.now()})`,
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		await c.db.execute(
			`INSERT INTO sleep_log (event, created_at) VALUES ('sleep', ${Date.now()})`,
		);
	},
	onWebSocket: (c, ws: UniversalWebSocket) => {
		ws.addEventListener("close", async () => {
			c.state.closeStarted += 1;
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('close-start', ${Date.now()})`,
			);
			await new Promise((resolve) =>
				setTimeout(resolve, RAW_WS_HANDLER_DELAY),
			);
			await c.db.execute(
				`INSERT INTO sleep_log (event, created_at) VALUES ('close-finish', ${Date.now()})`,
			);
			c.state.closeFinished += 1;
		});

		ws.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => ({
			startCount: c.state.startCount,
			sleepCount: c.state.sleepCount,
			closeStarted: c.state.closeStarted,
			closeFinished: c.state.closeFinished,
		}),
		getLogEntries: async (c) => {
			return await c.db.execute<{
				id: number;
				event: string;
				created_at: number;
			}>(`SELECT * FROM sleep_log ORDER BY id`);
		},
	},
	options: {
		sleepTimeout: RAW_WS_HANDLER_SLEEP_TIMEOUT,
	},
});
