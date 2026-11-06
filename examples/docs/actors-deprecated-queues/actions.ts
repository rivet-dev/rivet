import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const counter = actor({
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
    // Actions run concurrently, so the update is a single atomic statement.
    // The write is durable once the action returns.
    increment: async (c, amount: number) => {
      const rows = await c.db.execute<{ value: number }>(
        "UPDATE counter SET value = value + ? WHERE id = 1 RETURNING value",
        amount,
      );
      return rows[0].value;
    },
  },
});

// Client: const value = await handle.increment(5);
