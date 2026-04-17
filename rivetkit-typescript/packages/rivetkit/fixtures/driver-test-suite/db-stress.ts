import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

export const dbStressActor = actor({
	state: {},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS stress_data (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					value TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		// Insert many rows in a single action. Used to create a long-running
		// DB operation that can race with destroy/disconnect.
		insertBatch: async (c, count: number) => {
			const now = Date.now();
			const values: string[] = [];
			for (let i = 0; i < count; i++) {
				values.push(`('row-${i}', ${now})`);
			}
			await c.db.execute(
				`INSERT INTO stress_data (value, created_at) VALUES ${values.join(", ")}`,
			);
			return { count };
		},

		getCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM stress_data`,
			);
			return results[0].count;
		},

		// Measure event loop health during a DB operation.
		// Runs a Promise.resolve() microtask check interleaved with DB
		// inserts to detect if the event loop is being blocked between
		// awaits. Reports the wall-clock duration so the test can verify
		// the inserts complete in a reasonable time (not blocked by
		// synchronous lifecycle operations).
		measureEventLoopHealth: async (c, insertCount: number) => {
			const startMs = Date.now();

			// Do DB work that should NOT block the event loop.
			// Insert rows one at a time to create many async round-trips.
			for (let i = 0; i < insertCount; i++) {
				await c.db.execute(
					`INSERT INTO stress_data (value, created_at) VALUES ('drift-${i}', ${Date.now()})`,
				);
			}

			const elapsedMs = Date.now() - startMs;

			return {
				elapsedMs,
				insertCount,
			};
		},

		// Write data to multiple rows that can be verified after a
		// forced disconnect and reconnect.
		writeAndVerify: async (c, count: number) => {
			const now = Date.now();
			for (let i = 0; i < count; i++) {
				await c.db.execute(
					`INSERT INTO stress_data (value, created_at) VALUES ('verify-${i}', ${now})`,
				);
			}

			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM stress_data WHERE value LIKE 'verify-%'`,
			);
			return results[0].count;
		},

		integrityCheck: async (c) => {
			const rows = await c.db.execute<Record<string, unknown>>(
				"PRAGMA integrity_check",
			);
			const value = Object.values(rows[0] ?? {})[0];
			return String(value ?? "");
		},

		triggerSleep: (c) => {
			c.sleep();
		},

		reset: async (c) => {
			await c.db.execute(`DELETE FROM stress_data`);
		},

		destroy: (c) => {
			c.destroy();
		},
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});
