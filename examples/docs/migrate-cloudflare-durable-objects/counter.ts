import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// docs:start define
// A Durable Object class becomes an actor definition. Public methods become
// actions, and `ctx.storage` becomes the actor's SQLite database.
export const counter = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          value INTEGER NOT NULL
        )
      `);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, value) VALUES (1, 0)",
			);
		},
	}),
	actions: {
		increment: async (c, amount = 1) => {
			// One statement reads and writes, so concurrent calls can't lose updates
			const rows = await c.db.execute<{ value: number }>(
				"UPDATE counter SET value = value + ? WHERE id = 1 RETURNING value",
				amount,
			);
			return rows[0].value;
		},
		get: async (c) => {
			const rows = await c.db.execute<{ value: number }>(
				"SELECT value FROM counter WHERE id = 1",
			);
			return rows[0].value;
		},
	},
});
// docs:end define

// docs:start registry
// The registry replaces the Worker that routes requests to Durable Objects.
export const registry = setup({ use: { counter } });
registry.start();
// docs:end registry
