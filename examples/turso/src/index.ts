import { type Client, createClient as createDatabaseClient } from "@libsql/client";
import { createClient as createPlatformClient } from "@tursodatabase/api";
import { actor, setup } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export type Todo = {
	id: number;
	title: string;
	completed: boolean;
	createdAt: number;
};

type TodoDatabase = { name: string; url: string; authToken: string };

// One Platform API client for the whole process. It creates and deletes
// databases. Each actor queries its own database through c.vars.db.
const turso = createPlatformClient({
	org: requireEnv("TURSO_ORG"),
	token: requireEnv("TURSO_PLATFORM_TOKEN"),
});

export const todoList = actor({
	// Rivet's embedded SQLite stores which Turso database belongs to this actor:
	// https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS turso_database (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					name TEXT NOT NULL,
					url TEXT NOT NULL,
					auth_token TEXT NOT NULL
				)
			`);
		},
	}),

	// Create this actor's own Turso database once, when the actor is created:
	// https://rivet.dev/docs/actors/lifecycle#oncreate
	onCreate: async (c) => {
		const name = databaseName(c.actorId);
		const hostname = await createDatabase(name);
		const { jwt } = await turso.databases.createToken(name, {
			authorization: "full-access",
		});
		await c.db.execute(
			"INSERT INTO turso_database (id, name, url, auth_token) VALUES (1, ?, ?, ?)",
			name,
			`libsql://${hostname}`,
			jwt,
		);
	},

	// Open a client to this actor's database each time the actor starts. vars
	// are never persisted, so live objects like clients belong here:
	// https://rivet.dev/docs/actors/state#ephemeral-state
	//
	// The client is never closed. In Node, a libsql:// URL uses HTTP: each
	// statement opens and closes its own stream, so an idle client holds no
	// connections, and dropping vars when the actor sleeps releases it.
	// close() would reject statements that in-flight actions have not sent yet.
	createVars: async (c): Promise<{ database: TodoDatabase; db: Client }> => {
		const database = await loadTodoDatabase(c.db);
		const db = createDatabaseClient({
			url: database.url,
			authToken: database.authToken,
		});
		await db.execute(`
			CREATE TABLE IF NOT EXISTS todos (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				title TEXT NOT NULL,
				completed INTEGER NOT NULL DEFAULT 0,
				created_at INTEGER NOT NULL
			)
		`);
		return { database, db };
	},

	// The actor is gone for good, so delete its database too:
	// https://rivet.dev/docs/actors/lifecycle#ondestroy
	onDestroy: async (c) => {
		await turso.databases.delete(c.vars.database.name);
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		// Each action awaits its statement, so a write is committed before the
		// action returns.
		addTodo: async (c, title: string): Promise<Todo> => {
			const createdAt = Date.now();
			const result = await c.vars.db.execute({
				sql: "INSERT INTO todos (title, created_at) VALUES (?, ?)",
				args: [title, createdAt],
			});
			return {
				id: Number(result.lastInsertRowid),
				title,
				completed: false,
				createdAt,
			};
		},

		toggleTodo: async (c, id: number) => {
			await c.vars.db.execute({
				sql: "UPDATE todos SET completed = NOT completed WHERE id = ?",
				args: [id],
			});
		},

		listTodos: async (c): Promise<Todo[]> => {
			const result = await c.vars.db.execute(
				"SELECT id, title, completed, created_at FROM todos ORDER BY id",
			);
			return result.rows.map((row) => ({
				id: Number(row.id),
				title: String(row.title),
				completed: Boolean(row.completed),
				createdAt: Number(row.created_at),
			}));
		},

		// Destroy the actor, which runs onDestroy:
		// https://rivet.dev/docs/actors/destroy
		deleteList: (c) => {
			c.destroy();
		},
	},
});

// Turso database names allow lowercase letters, digits, and dashes.
export function databaseName(actorId: string): string {
	return `todos-${actorId}`.toLowerCase().replace(/[^a-z0-9-]/g, "-");
}

// Reads the Turso database details that onCreate stored.
async function loadTodoDatabase(db: RawAccess): Promise<TodoDatabase> {
	const rows = (await db.execute(
		"SELECT name, url, auth_token FROM turso_database WHERE id = 1",
	)) as { name: string; url: string; auth_token: string }[];
	const row = rows[0];
	if (!row) throw new Error("this actor has no Turso database");
	return { name: row.name, url: row.url, authToken: row.auth_token };
}

// Creates the database, or finds it if a previous attempt to create this
// actor already did.
async function createDatabase(name: string): Promise<string> {
	try {
		const database = await turso.databases.create(name, {
			group: process.env.TURSO_GROUP ?? "default",
		});
		return database.hostname;
	} catch (error) {
		if ((error as { status?: number }).status !== 409) throw error;
		return (await turso.databases.get(name)).hostname;
	}
}

function requireEnv(name: string): string {
	const value = process.env[name];
	if (!value) throw new Error(`${name} is not set. See .env.example.`);
	return value;
}

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { todoList },
});

registry.start();
