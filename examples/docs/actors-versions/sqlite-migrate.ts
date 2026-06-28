import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

const todoList = actor({
	db: db({
		onMigrate: async (db) => {
			const [{ user_version }] = (await db.execute(
				"PRAGMA user_version",
			)) as { user_version: number }[];

			if (user_version < 1) {
				await db.execute(`
					CREATE TABLE todos (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						title TEXT NOT NULL
					);
				`);
			}

			if (user_version < 2) {
				await db.execute(`
					ALTER TABLE todos ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;
				`);
			}

			await db.execute("PRAGMA user_version = 2");
		},
	}),
	actions: {
		addTodo: async (c, title: string) => {
			await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
		},
	},
});

const registry = setup({ use: { todoList } });
registry.start();
