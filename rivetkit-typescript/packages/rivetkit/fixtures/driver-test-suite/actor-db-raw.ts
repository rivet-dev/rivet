import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const dbActorRaw = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS test_data (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					value TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		insertValue: async (c, value: string) => {
			await c.db.execute(
				`INSERT INTO test_data (value, created_at) VALUES ('${value}', ${Date.now()})`,
			);
			return { success: true };
		},
		getValues: async (c) => {
			const results = (await c.db.execute(
				`SELECT * FROM test_data ORDER BY id`,
			)) as Array<{
				id: number;
				value: string;
				created_at: number;
			}>;
			return results;
		},
		getCount: async (c) => {
			const results = (await c.db.execute(
				`SELECT COUNT(*) as count FROM test_data`,
			)) as Array<{ count: number }>;
			return results[0].count;
		},
		clearData: async (c) => {
			await c.db.execute(`DELETE FROM test_data`);
		},
		// Bulk operations for benchmarking (loop inside actor)
		bulkInsert: async (c, count: number) => {
			const start = performance.now();
			await c.db.execute("BEGIN TRANSACTION");
			for (let i = 0; i < count; i++) {
				await c.db.execute(
					`INSERT INTO test_data (value, created_at) VALUES ('User ${i}', ${Date.now()})`,
				);
			}
			await c.db.execute("COMMIT");
			const elapsed = performance.now() - start;
			return { count, elapsed };
		},
		bulkGet: async (c, count: number) => {
			const start = performance.now();
			for (let i = 0; i < count; i++) {
				await c.db.execute(`SELECT COUNT(*) as count FROM test_data`);
			}
			const elapsed = performance.now() - start;
			return { count, elapsed };
		},
		updateValue: async (c, id: number, value: string) => {
			await c.db.execute(
				`UPDATE test_data SET value = '${value}' WHERE id = ${id}`,
			);
			return { success: true };
		},
		bulkUpdate: async (c, count: number) => {
			const start = performance.now();
			await c.db.execute("BEGIN TRANSACTION");
			for (let i = 1; i <= count; i++) {
				await c.db.execute(
					`UPDATE test_data SET value = 'Updated ${i}' WHERE id = ${i}`,
				);
			}
			await c.db.execute("COMMIT");
			const elapsed = performance.now() - start;
			return { count, elapsed };
		},
	},
});
