import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const counter = actor({
	// Embedded SQLite database that survives restarts: https://rivet.dev/docs/actors/sqlite
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

	events: {
		newCount: event<number>(),
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		increment: async (c, amount: number) => {
			// Update and read the count in one statement, so concurrent calls never lose an increment
			const [{ count }] = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
				amount,
			);
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("newCount", count);
			return count;
		},

		getCount: async (c) => {
			const [{ count }] = await c.db.execute<{ count: number }>(
				"SELECT count FROM counter WHERE id = 1",
			);
			return count;
		},
	},
});

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { counter },
});

registry.start();
