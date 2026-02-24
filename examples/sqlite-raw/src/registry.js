import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
export const todoList = actor({
    db: db({
        onMigrate: async (db) => {
            // Run migrations on wake
            await db.execute(`
				CREATE TABLE IF NOT EXISTS todos (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					title TEXT NOT NULL,
					completed INTEGER DEFAULT 0,
					created_at INTEGER NOT NULL
				)
			`);
        },
    }),
    actions: {
        addTodo: async (c, title) => {
            const createdAt = Date.now();
            await c.db.execute("INSERT INTO todos (title, created_at) VALUES (?, ?)", title, createdAt);
            return { title, createdAt };
        },
        getTodos: async (c) => {
            const rows = await c.db.execute("SELECT * FROM todos ORDER BY created_at DESC");
            return rows;
        },
        toggleTodo: async (c, id) => {
            await c.db.execute("UPDATE todos SET completed = NOT completed WHERE id = ?", id);
            const rows = await c.db.execute("SELECT * FROM todos WHERE id = ?", id);
            return rows[0];
        },
        deleteTodo: async (c, id) => {
            await c.db.execute("DELETE FROM todos WHERE id = ?", id);
            return { id };
        },
    },
});
export const registry = setup({
    use: { todoList },
});
