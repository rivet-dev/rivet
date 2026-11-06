import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const preferences = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS themes (
          user_id TEXT PRIMARY KEY,
          theme TEXT NOT NULL
        )
      `);
      await db.execute(`
        CREATE TABLE IF NOT EXISTS migrations (
          name TEXT PRIMARY KEY
        )
      `);
    },
  }),

  // `onWake` runs on every start before the actor handles requests
  onWake: async (c) => {
    const done = await c.db.execute(
      "SELECT 1 FROM migrations WHERE name = 'kv-themes'",
    );
    if (done.length > 0) return;

    // Copy every KV entry under the old prefix. Returned keys still include
    // the prefix, so strip it to get the user ID.
    const entries = await c.kv.list("theme:");
    for (const [key, theme] of entries) {
      const userId = key.slice("theme:".length);
      await c.db.execute(
        "INSERT INTO themes (user_id, theme) VALUES (?, ?) ON CONFLICT (user_id) DO UPDATE SET theme = excluded.theme",
        userId,
        theme,
      );
    }

    // Mark the copy as done so it runs once
    await c.db.execute("INSERT INTO migrations (name) VALUES ('kv-themes')");
  },

  actions: {
    get: async (c, userId: string) => {
      const rows = await c.db.execute<{ theme: string }>(
        "SELECT theme FROM themes WHERE user_id = ?",
        userId,
      );
      return rows[0]?.theme ?? null;
    },
  },
});
