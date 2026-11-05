import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const todoList = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS todos (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          title TEXT NOT NULL
        );
      `);
    },
  }),

  actions: {
    add: async (c, title: string) => {
      await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
    },

    list: async (c) => {
      return (await c.db.execute(
        "SELECT id, title FROM todos ORDER BY id DESC",
      )) as { id: number; title: string }[];
    },
  },
});
