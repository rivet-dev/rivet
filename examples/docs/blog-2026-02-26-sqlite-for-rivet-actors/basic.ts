import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

const todoList = actor({
  // Each actor instance gets its own SQLite database
  db: db({
    // Create tables on first run
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS todos (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          title TEXT NOT NULL,
          completed INTEGER NOT NULL DEFAULT 0
        );
      `);
    },
  }),
  actions: {
    // Parameterized queries with ? placeholders
    addTodo: async (c, title: string) => {
      await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
    },
    toggleTodo: async (c, id: number) => {
      await c.db.execute(
        "UPDATE todos SET completed = NOT completed WHERE id = ?",
        id,
      );
    },
    // SELECT returns typed row objects
    getTodos: async (c) => {
      return (await c.db.execute(
        "SELECT id, title, completed FROM todos ORDER BY id DESC",
      )) as { id: number; title: string; completed: number }[];
    },
  },
});

export const registry = setup({ use: { todoList } });
