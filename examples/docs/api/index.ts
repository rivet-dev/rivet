import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const counter = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
			);
			await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
		},
	}),
	events: {
		countChanged: event<number>(),
	},
	onRequest: async (c, request) => {
		if (request.method === "POST") {
			const [{ count }] = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
			);
			return Response.json({ count });
		}
		const [{ count }] = await c.db.execute<{ count: number }>(
			"SELECT count FROM counter WHERE id = 1",
		);
		return Response.json({ count });
	},
	actions: {
		increment: async (c, amount: number) => {
			const [{ count }] = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
				amount,
			);
			c.broadcast("countChanged", count);
			return count;
		},
	},
});

export const registry = setup({ use: { counter } });
registry.start();
