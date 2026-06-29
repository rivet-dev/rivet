import { actor, queue, setup } from "rivetkit";
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
  queues: {
    addTodo: queue<{ title: string }>(),
    addComment: queue<{ todoId: number; body: string }>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      if (message.name === "addTodo") {
        await c.db.execute("INSERT INTO todos (title) VALUES (?)", message.body.title);
      } else if (message.name === "addComment") {
        await c.db.execute(
          "INSERT INTO comments (todo_id, body) VALUES (?, ?)",
          message.body.todoId,
          message.body.body,
        );
      }
    }
  },
  actions: {
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
