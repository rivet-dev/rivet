import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const counter = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
      );
    },
  }),
  // Seed the initial row from input once, when the actor is first created
  onCreate: async (c, input: { start?: number }) => {
    await c.db.execute("INSERT INTO counter (id, count) VALUES (1, ?)", input.start ?? 0);
  },
  actions: {
    increment: async (c) => {
      const [{ count }] = await c.db.execute<{ count: number }>(
        "UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
      );
      return count;
    },
  },
});
