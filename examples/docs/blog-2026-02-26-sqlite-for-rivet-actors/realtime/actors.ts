import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

interface Todo {
  id: number;
  title: string;
  completed: number;
}

export const todoList = actor({
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
  // Typed events that clients can subscribe to
  events: {
    todosUpdated: event<Todo[]>(),
  },
  actions: {
    addTodo: async (c, title: string) => {
      // Write to SQLite
      await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
      // Broadcast updated list to all connected clients
      const todos = (await c.db.execute(
        "SELECT id, title, completed FROM todos ORDER BY id DESC",
      )) as Todo[];
      c.broadcast("todosUpdated", todos);
    },
    toggleTodo: async (c, id: number) => {
      await c.db.execute(
        "UPDATE todos SET completed = NOT completed WHERE id = ?",
        id,
      );
      const todos = (await c.db.execute(
        "SELECT id, title, completed FROM todos ORDER BY id DESC",
      )) as Todo[];
      c.broadcast("todosUpdated", todos);
    },
    getTodos: async (c) => {
      return (await c.db.execute(
        "SELECT id, title, completed FROM todos ORDER BY id DESC",
      )) as Todo[];
    },
  },
});

export const registry = setup({ use: { todoList } });
