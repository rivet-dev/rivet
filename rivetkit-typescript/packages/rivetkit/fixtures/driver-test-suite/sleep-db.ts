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
