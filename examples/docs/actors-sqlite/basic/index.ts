import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const todoList = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS todos (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          title TEXT NOT NULL
        );
      `);

      await db.execute(`
        CREATE TABLE IF NOT EXISTS comments (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          todo_id INTEGER NOT NULL,
          body TEXT NOT NULL,
          FOREIGN KEY(todo_id) REFERENCES todos(id)
        );
      `);
    },
  }),
  actions: {
    addTodo: async (c, title: string) => {
      await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
    },
    addComment: async (c, todoId: number, body: string) => {
      await c.db.execute(
        "INSERT INTO comments (todo_id, body) VALUES (?, ?)",
        todoId,
        body,
      );
    },
    getTodos: async (c) => {
      return (await c.db.execute(
        "SELECT id, title FROM todos ORDER BY id DESC",
      )) as {
        id: number;
        title: string;
      }[];
    },
  },
});

export const registry = setup({ use: { todoList } });
registry.start();
