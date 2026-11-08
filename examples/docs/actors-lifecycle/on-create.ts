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
    },
  }),

  onCreate: async (c, input: { initialCount: number }) => {
    // Seed the database from input. This runs only once per actor.
    await c.db.execute(
      "INSERT INTO counter (id, count) VALUES (1, ?)",
      input.initialCount,
    );
    console.log("Actor created with initial count:", input.initialCount);
  },

  actions: { /* ... */ }
});
