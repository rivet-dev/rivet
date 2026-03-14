import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export type Todo = {
	id: string;
	title: string;
	completed: 0 | 1;
	created_at: number;
};

/**
 * A change event broadcast to all connected clients whenever a todo is
 * created, updated, or deleted. TanStack DB on the client side uses these
 * events to keep its local collection in sync without a full re-fetch.
 *
 * Insert/update carry the full row value (key is derived by getKey on the
 * client). Delete carries only the key, since the row no longer exists.
 */
export type TodoChange =
	| { type: "insert"; value: Todo }
	| { type: "update"; value: Todo }
	| { type: "delete"; key: string };

export const todoList = actor({
	// SQLite-backed persistent storage via rivetkit/db
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS todos (
					id TEXT PRIMARY KEY,
					title TEXT NOT NULL,
					completed INTEGER DEFAULT 0,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),

	events: {
		// Broadcast whenever any todo changes so all connected clients
		// can update their TanStack DB collection in real time.
		change: event<TodoChange>(),
	},

	actions: {
		getTodos: async (c): Promise<Todo[]> => {
			const rows = await c.db.execute(
				"SELECT * FROM todos ORDER BY created_at DESC",
			);
			return rows as Todo[];
		},

		addTodo: async (c, id: string, title: string, createdAt: number): Promise<Todo> => {
			await c.db.execute(
				"INSERT INTO todos (id, title, created_at) VALUES (?, ?, ?)",
				id,
				title,
				createdAt,
			);
			const rows = await c.db.execute(
				"SELECT * FROM todos WHERE id = ?",
				id,
			);
			const todo = rows[0] as Todo;
			c.broadcast("change", { type: "insert", value: todo });
			return todo;
		},

		toggleTodo: async (c, id: string): Promise<Todo> => {
			await c.db.execute(
				"UPDATE todos SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END WHERE id = ?",
				id,
			);
			const rows = await c.db.execute(
				"SELECT * FROM todos WHERE id = ?",
				id,
			);
			const todo = rows[0] as Todo;
			c.broadcast("change", { type: "update", value: todo });
			return todo;
		},

		deleteTodo: async (c, id: string): Promise<void> => {
			await c.db.execute("DELETE FROM todos WHERE id = ?", id);
			c.broadcast("change", { type: "delete", key: id });
		},
	},
});

export const registry = setup({
	use: { todoList },
});
