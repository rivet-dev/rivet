import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const counter = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
			);
			await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
		},
	}),
	actions: {
		increment: async (c) => {
			// Update and read the count in one atomic statement
			const [{ count }] = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
			);
			return count;
		},
	},
});
