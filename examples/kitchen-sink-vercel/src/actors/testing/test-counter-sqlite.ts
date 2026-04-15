import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const testCounterSqlite = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					value INTEGER NOT NULL DEFAULT 0
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, value) VALUES (1, 0)",
			);
		},
	}),
	actions: {
		increment: async (c, amount: number = 1) => {
			await c.db.execute(
				"UPDATE counter SET value = value + ? WHERE id = 1",
				amount,
			);
			const rows = await c.db.execute(
				"SELECT value FROM counter WHERE id = 1",
			);
			return (rows[0] as { value: number }).value;
		},
		getCount: async (c) => {
			const rows = await c.db.execute(
				"SELECT value FROM counter WHERE id = 1",
			);
			return (rows[0] as { value: number }).value;
		},
		reset: async (c) => {
			await c.db.execute("UPDATE counter SET value = 0 WHERE id = 1");
			return 0;
		},
	},
});
