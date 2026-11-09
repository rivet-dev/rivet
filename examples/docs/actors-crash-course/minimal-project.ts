import { actor, event, setup } from "rivetkit";
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
	events: {
		count: event<number>(),
	},
	actions: {
		increment: async (c, amount: number) => {
			const [{ count }] = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
				amount,
			);
			c.broadcast("count", count);
			return count;
		},
	},
});

export const registry = setup({
	use: { counter },
});

registry.start();
