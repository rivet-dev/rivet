import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const counter = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          count INTEGER NOT NULL
        );
      `);
      await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
    },
  }),

  // Load the count from SQLite into memory on every start
  createVars: async (c): Promise<{ count: number }> => {
    const rows = (await c.db.execute(
      "SELECT count FROM counter WHERE id = 1",
    )) as { count: number }[];
    return { count: rows[0].count };
  },

  actions: {
    get: (c) => c.vars.count,

    increment: async (c) => {
      // Update the in-memory value and write it back to SQLite
      c.vars.count += 1;
      await c.db.execute("UPDATE counter SET count = ? WHERE id = 1", c.vars.count);
      return c.vars.count;
    },
  },
});
