import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";

const todoList = actor({
  db: db({
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
  // Typed queues for ordered, durable writes
  queues: {
    addTodo: queue<{ title: string }>(),
    toggleTodo: queue<{ id: number }>(),
  },
  // Process queue messages sequentially
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      if (message.name === "addTodo") {
        await c.db.execute(
          "INSERT INTO todos (title) VALUES (?)",
          message.body.title,
        );
      } else if (message.name === "toggleTodo") {
        await c.db.execute(
          "UPDATE todos SET completed = NOT completed WHERE id = ?",
          message.body.id,
        );
      }
    }
  },
  // Keep actions read-only
  actions: {
    getTodos: async (c) => {
      return (await c.db.execute(
        "SELECT id, title, completed FROM todos ORDER BY id DESC",
      )) as { id: number; title: string; completed: number }[];
    },
  },
});

export const registry = setup({ use: { todoList } });
