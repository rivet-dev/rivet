import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const room = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS presence (
          conn_id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          joined_at INTEGER NOT NULL
        );
      `);
    },
  }),

  // Hibernatable connections outlive the actor's memory, so store their data in SQLite
  onConnect: async (c, conn) => {
    const { name } = conn.params as { name: string };
    await c.db.execute(
      "INSERT OR REPLACE INTO presence (conn_id, name, joined_at) VALUES (?, ?, ?)",
      conn.id,
      name,
      Date.now(),
    );
  },

  onDisconnect: async (c, conn) => {
    await c.db.execute("DELETE FROM presence WHERE conn_id = ?", conn.id);
  },

  // A connection that closed while the actor slept may never reach
  // `onDisconnect`, so drop rows for connections that are no longer open
  onWake: async (c) => {
    const rows = (await c.db.execute("SELECT conn_id FROM presence")) as {
      conn_id: string;
    }[];
    for (const row of rows) {
      if (!c.conns.has(row.conn_id)) {
        await c.db.execute("DELETE FROM presence WHERE conn_id = ?", row.conn_id);
      }
    }
  },

  actions: {
    whoAmI: async (c) => {
      const rows = (await c.db.execute(
        "SELECT name, joined_at FROM presence WHERE conn_id = ?",
        c.conn.id,
      )) as { name: string; joined_at: number }[];
      return rows[0];
    },
  },
});
