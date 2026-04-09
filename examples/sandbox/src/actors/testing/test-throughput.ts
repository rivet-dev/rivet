import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const testThroughput = actor({
	options: {
		actionTimeout: 300_000,
	},
	db: db({
		onMigrate: async (database) => {
			// await database.execute("BEGIN");
			for (let i = 0; i < 50; i++) {
				await database.execute(`CREATE TABLE IF NOT EXISTS tbl_${i} (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					data TEXT NOT NULL DEFAULT ''
				)`);
			}
			await database.execute(`CREATE TABLE IF NOT EXISTS counter (
				id INTEGER PRIMARY KEY CHECK (id = 1),
				value INTEGER NOT NULL DEFAULT 0
			)`);
			await database.execute("INSERT OR IGNORE INTO counter (id, value) VALUES (1, 0)");
			// await database.execute("COMMIT");
		},
	}),
	actions: {
		increment: async (c) => {
			await c.db.execute("UPDATE counter SET value = value + 1 WHERE id = 1");
			const rows = await c.db.execute("SELECT value FROM counter WHERE id = 1");
			return (rows[0] as { value: number }).value;
		},
	},
});
