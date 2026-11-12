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
    },
  }),
  actions: {
    // `put` becomes an upsert
    set: async (c, userId: string, theme: string) => {
      await c.db.execute(
        "INSERT INTO themes (user_id, theme) VALUES (?, ?) ON CONFLICT (user_id) DO UPDATE SET theme = excluded.theme",
        userId,
        theme,
      );
    },
    // `get` becomes a lookup by primary key
    get: async (c, userId: string) => {
      const rows = await c.db.execute<{ theme: string }>(
        "SELECT theme FROM themes WHERE user_id = ?",
        userId,
      );
      return rows[0]?.theme ?? null;
    },
    // `list` by prefix becomes a query over the whole table
    list: async (c) => {
      return await c.db.execute<{ user_id: string; theme: string }>(
        "SELECT user_id, theme FROM themes ORDER BY user_id",
      );
    },
  },
});
