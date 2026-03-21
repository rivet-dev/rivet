import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

export const dbBlockingActor = actor({
	state: {
		eventLog: [] as Array<{ type: string; ts: number }>,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS blocking_test (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					value TEXT NOT NULL,
					payload TEXT NOT NULL DEFAULT ''
				)
			`);
		},
	}),
	events: {
		tick: event<{ ts: number }>(),
	},
	actions: {
		// Insert many rows to make subsequent queries slow
		seedData: async (c, rowCount: number) => {
			const batchSize = 100;
			for (let i = 0; i < rowCount; i += batchSize) {
				const count = Math.min(batchSize, rowCount - i);
				const values = Array.from(
					{ length: count },
					(_, j) =>
						`('row-${i + j}', '${"x".repeat(1024)}')`,
				).join(",");
				await c.db.execute(
					`INSERT INTO blocking_test (value, payload) VALUES ${values}`,
				);
			}
			const result = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) as count FROM blocking_test",
			);
			return result[0]?.count ?? 0;
		},

		// Run a heavy query that should block the event loop
		heavyQuery: async (c) => {
			const startTs = Date.now();
			// Cross join to force CPU-heavy work inside WASM
			const result = await c.db.execute<{ total: number }>(
				`SELECT COUNT(*) as total FROM blocking_test a, blocking_test b WHERE a.id < 50 AND b.id < 50`,
			);
			const endTs = Date.now();
			return {
				total: result[0]?.total ?? 0,
				durationMs: endTs - startTs,
			};
		},

		// Log a timestamp event (to check if events get processed during heavy queries)
		logEvent: (c, type: string) => {
			const ts = Date.now();
			c.state.eventLog.push({ type, ts });
			c.events.tick.broadcast({ ts });
			return ts;
		},

		// Get the event log
		getEventLog: (c) => {
			return c.state.eventLog;
		},

		// Clear event log
		clearEventLog: (c) => {
			c.state.eventLog = [];
		},

		// Run heavy query AND try to log events concurrently
		// Returns timestamps showing when things actually executed
		concurrencyTest: async (c, rowCount: number) => {
			const timeline: Array<{ label: string; ts: number }> = [];

			timeline.push({ label: "start", ts: Date.now() });

			// Seed data
			const batchSize = 100;
			for (let i = 0; i < rowCount; i += batchSize) {
				const count = Math.min(batchSize, rowCount - i);
				const values = Array.from(
					{ length: count },
					(_, j) =>
						`('row-${i + j}', '${"x".repeat(512)}')`,
				).join(",");
				await c.db.execute(
					`INSERT INTO blocking_test (value, payload) VALUES ${values}`,
				);
			}

			timeline.push({ label: "seeded", ts: Date.now() });

			// Start a heavy query
			const queryPromise = c.db
				.execute<{ total: number }>(
					`SELECT COUNT(*) as total FROM blocking_test a, blocking_test b WHERE a.id < 100 AND b.id < 100`,
				)
				.then((result) => {
					timeline.push({ label: "query_done", ts: Date.now() });
					return result;
				});

			// Try to log events while the query is running
			// These should execute immediately if the event loop isn't blocked
			timeline.push({ label: "before_event_1", ts: Date.now() });
			c.state.eventLog.push({ type: "during_query_1", ts: Date.now() });

			// Schedule a microtask
			await Promise.resolve();
			timeline.push({ label: "after_microtask", ts: Date.now() });

			// Schedule a setTimeout(0) to check if macrotasks are blocked
			const setTimeoutTs = await new Promise<number>((resolve) => {
				setTimeout(() => resolve(Date.now()), 0);
			});
			timeline.push({ label: "after_settimeout0", ts: setTimeoutTs });

			await queryPromise;
			timeline.push({ label: "end", ts: Date.now() });

			return timeline;
		},
	},
});
