import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { migrations } from "rivetkit/unstable/migrations";

const migrateTodos = migrations({
	tableName: "todos_schema_version",
	migrations: [
		{
			version: 1,
			sql: `
				CREATE TABLE todos (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					title TEXT NOT NULL
				)
			`,
		},
		{
			version: 2,
			sql: `
				ALTER TABLE todos
				ADD COLUMN completed INTEGER NOT NULL DEFAULT 0
			`,
		},
	],
});

const todoList = actor({
	db: db({ onMigrate: migrateTodos }),
	actions: {
		addTodo: async (c, title: string) => {
			await c.db.execute("INSERT INTO todos (title) VALUES (?)", title);
		},
	},
});

const registry = setup({ use: { todoList } });
registry.start();
