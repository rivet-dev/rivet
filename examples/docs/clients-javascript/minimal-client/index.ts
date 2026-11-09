import { actor, setup } from "rivetkit";
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
  actions: {
    increment: async (c, x: number) => {
      const [{ count }] = await c.db.execute<{ count: number }>(
        "UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
        x,
      );
      return count;
    },
  },
});

export const registry = setup({
  use: { counter },
});

registry.start();
