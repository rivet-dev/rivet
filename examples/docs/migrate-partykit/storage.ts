import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

export const counterRoom = actor({
	// room.storage becomes the actor's SQLite database
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          count INTEGER NOT NULL
        )
      `);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)",
			);
		},
	}),

	// Replaces onStart: load into memory on every start, including after sleep
	createVars: async (c): Promise<{ count: number }> => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT count FROM counter WHERE id = 1",
		);
		return { count: rows[0].count };
	},

	events: {
		count: event<number>(),
	},

	actions: {
		increment: async (c) => {
			// Read the new value before awaiting, so concurrent calls each get their own
			const count = ++c.vars.count;

			// Replaces room.storage.put("count", count). Durable once this resolves.
			await c.db.execute(
				"UPDATE counter SET count = ? WHERE id = 1",
				count,
			);
			c.broadcast("count", count);
			return count;
		},

		get: (c) => c.vars.count,
	},
});
