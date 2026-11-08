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

  // CreateContext in onCreate hook
  onCreate: async (c, input: { initial: number }) => {
    await c.db.execute("INSERT INTO counter (id, count) VALUES (1, ?)", input.initial);
  },

  // ActionContext in actions
  actions: {
    increment: async (c) => {
      await c.db.execute("UPDATE counter SET count = count + 1 WHERE id = 1");
      const [row] = (await c.db.execute(
        "SELECT count FROM counter WHERE id = 1",
      )) as { count: number }[];
      return row.count;
    }
  }
});
