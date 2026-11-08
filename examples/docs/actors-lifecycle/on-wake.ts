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
  vars: { intervalId: null as NodeJS.Timeout | null },

  onWake: async (c) => {
    const rows = await c.db.execute<{ count: number }>(
      "SELECT count FROM counter WHERE id = 1",
    );
    console.log('Actor started with count:', rows[0].count);

    // Set up interval for automatic counting
    const intervalId = setInterval(async () => {
      const updated = await c.db.execute<{ count: number }>(
        "UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
      );
      c.broadcast("countChanged", updated[0].count);
      console.log('Auto-increment:', updated[0].count);
    }, 10000);

    // Store interval ID in vars to clean up later if needed
    c.vars.intervalId = intervalId;
  },

  actions: {
    stop: (c) => {
      if (c.vars.intervalId) {
        clearInterval(c.vars.intervalId);
        c.vars.intervalId = null;
      }
    }
  }
});
