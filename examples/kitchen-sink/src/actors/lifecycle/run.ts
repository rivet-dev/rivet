import { actor, queue } from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../../index.ts";

export const RUN_SLEEP_TIMEOUT = 500;

type RunStatusRow = {
	run_started: number;
	run_exited: number;
	destroy_called: number;
};

/**
 * Creates a one-row `run_status` table that records run handler progress so it
 * survives sleep.
 */
function runStatusDb() {
	return db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS run_status (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					run_started INTEGER NOT NULL DEFAULT 0,
					run_exited INTEGER NOT NULL DEFAULT 0,
					destroy_called INTEGER NOT NULL DEFAULT 0,
					tick_count INTEGER NOT NULL DEFAULT 0,
					last_tick_at INTEGER NOT NULL DEFAULT 0,
					wake_count INTEGER NOT NULL DEFAULT 0
				);
			`);
			await db.execute("INSERT OR IGNORE INTO run_status (id) VALUES (1)");
		},
	});
}

// Actor that tracks tick counts and respects abort signal
export const runWithTicks = actor({
	db: runStatusDb(),
	run: async (c) => {
		await c.db.execute("UPDATE run_status SET run_started = 1 WHERE id = 1");
		c.log.info("run handler started");

		while (!c.aborted) {
			const rows = await c.db.execute<{ tick_count: number }>(
				"UPDATE run_status SET tick_count = tick_count + 1, last_tick_at = ? WHERE id = 1 RETURNING tick_count",
				Date.now(),
			);
			c.log.info({ msg: "tick", tickCount: rows[0].tick_count });

			// Wait 50ms between ticks, or exit early if aborted
			await new Promise<void>((resolve) => {
				const timeout = setTimeout(resolve, 50);
				c.abortSignal.addEventListener(
					"abort",
					() => {
						clearTimeout(timeout);
						resolve();
					},
					{ once: true },
				);
			});
		}

		await c.db.execute("UPDATE run_status SET run_exited = 1 WHERE id = 1");
		c.log.info("run handler exiting gracefully");
	},
	actions: {
		getState: async (c) => {
			const rows = await c.db.execute<
				RunStatusRow & { tick_count: number; last_tick_at: number }
			>(
				"SELECT tick_count, last_tick_at, run_started, run_exited FROM run_status WHERE id = 1",
			);
			const row = rows[0];
			return {
				tickCount: row.tick_count,
				lastTickAt: row.last_tick_at,
				runStarted: row.run_started === 1,
				runExited: row.run_exited === 1,
			};
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor that consumes from a queue in the run handler
export const runWithQueueConsumer = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS run_status (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					run_started INTEGER NOT NULL DEFAULT 0
				);
			`);
			await db.execute("INSERT OR IGNORE INTO run_status (id) VALUES (1)");
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages_received (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					body TEXT NOT NULL
				);
			`);
		},
	}),
	queues: {
		messages: queue<unknown>(),
	},
	run: async (c) => {
		await c.db.execute("UPDATE run_status SET run_started = 1 WHERE id = 1");
		c.log.info("run handler started, waiting for messages");

		for await (const message of c.queue.iter()) {
			c.log.info({ msg: "received message", body: message.body });
			await c.db.execute(
				"INSERT INTO messages_received (name, body) VALUES (?, ?)",
				message.name,
				JSON.stringify(message.body ?? null),
			);
		}

		c.log.info("run handler exiting gracefully");
	},
	actions: {
		getState: async (c) => {
			const messages = await c.db.execute<{ name: string; body: string }>(
				"SELECT name, body FROM messages_received ORDER BY seq",
			);
			const status = await c.db.execute<{ run_started: number }>(
				"SELECT run_started FROM run_status WHERE id = 1",
			);
			return {
				messagesReceived: messages.map((row) => ({
					name: row.name,
					body: JSON.parse(row.body) as unknown,
				})),
				runStarted: status[0].run_started === 1,
			};
		},
		sendMessage: async (c, body: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.runWithQueueConsumer.getForId(c.actorId);
			await handle.send("messages", body);
			return true;
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor that exits the run handler after a short delay to test crash behavior
export const runWithEarlyExit = actor({
	db: runStatusDb(),
	run: async (c) => {
		await c.db.execute("UPDATE run_status SET run_started = 1 WHERE id = 1");
		c.log.info("run handler started, will exit after delay");
		// Wait a bit so we can observe the runStarted state before exit
		await new Promise((resolve) => setTimeout(resolve, 200));
		c.log.info("run handler exiting early");
		// Exit without respecting abort signal
	},
	onDestroy: async (c) => {
		await c.db.execute("UPDATE run_status SET destroy_called = 1 WHERE id = 1");
	},
	actions: {
		getState: async (c) => {
			const rows = await c.db.execute<RunStatusRow>(
				"SELECT run_started, destroy_called FROM run_status WHERE id = 1",
			);
			return {
				runStarted: rows[0].run_started === 1,
				destroyCalled: rows[0].destroy_called === 1,
			};
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor that throws an error in the run handler to test crash behavior
export const runWithError = actor({
	db: runStatusDb(),
	run: async (c) => {
		await c.db.execute("UPDATE run_status SET run_started = 1 WHERE id = 1");
		c.log.info("run handler started, will throw error");
		await new Promise((resolve) => setTimeout(resolve, 50));
		throw new Error("intentional error in run handler");
	},
	onDestroy: async (c) => {
		await c.db.execute("UPDATE run_status SET destroy_called = 1 WHERE id = 1");
	},
	actions: {
		getState: async (c) => {
			const rows = await c.db.execute<RunStatusRow>(
				"SELECT run_started, destroy_called FROM run_status WHERE id = 1",
			);
			return {
				runStarted: rows[0].run_started === 1,
				destroyCalled: rows[0].destroy_called === 1,
			};
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor without a run handler for comparison
export const runWithoutHandler = actor({
	db: runStatusDb(),
	onWake: async (c) => {
		await c.db.execute(
			"UPDATE run_status SET wake_count = wake_count + 1 WHERE id = 1",
		);
	},
	actions: {
		getState: async (c) => {
			const rows = await c.db.execute<{ wake_count: number }>(
				"SELECT wake_count FROM run_status WHERE id = 1",
			);
			return {
				wakeCount: rows[0].wake_count,
			};
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});
