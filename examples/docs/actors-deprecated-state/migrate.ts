import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import { decodeLegacyState } from "./decode-legacy-state";

const counter = actor({
  // `state` is removed from the actor definition

  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          count INTEGER NOT NULL
        );
      `);

      // Copy the old `c.state` into the new table once. `onMigrate` runs on
      // every start, so skip the copy when the row already exists.
      const existing = (await db.execute(
        "SELECT 1 FROM counter WHERE id = 1",
      )) as unknown[];
      if (existing.length === 0) {
        const legacy = (await db.execute(
          "SELECT state FROM _rivet_actor_state WHERE id = 1",
        )) as { state: Uint8Array }[];
        const old =
          legacy.length > 0 && legacy[0].state.length > 0
            ? (decodeLegacyState(legacy[0].state) as { count: number })
            : { count: 0 };

        await db.execute(
          "INSERT INTO counter (id, count) VALUES (1, ?)",
          old.count,
        );
      }
    },
  }),

  // Load the migrated data into memory on every start
  createVars: async (c): Promise<{ count: number }> => {
    const rows = (await c.db.execute(
      "SELECT count FROM counter WHERE id = 1",
    )) as { count: number }[];
    return { count: rows[0].count };
  },

  actions: {
    get: (c) => c.vars.count,

    increment: async (c) => {
      c.vars.count += 1;
      await c.db.execute("UPDATE counter SET count = ? WHERE id = 1", c.vars.count);
      return c.vars.count;
    },
  },
});
