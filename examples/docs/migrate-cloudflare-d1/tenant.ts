import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export type User = {
	id: number;
	email: string;
	name: string;
};

// One actor per tenant, each with its own SQLite database. The tenant ID is
// the actor key, so the tables no longer need a tenant_id column.
export const tenantDb = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          email TEXT NOT NULL UNIQUE,
          name TEXT NOT NULL,
          order_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS orders (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          user_id INTEGER NOT NULL REFERENCES users(id),
          total INTEGER NOT NULL
        );
      `);
		},
	}),

	actions: {
		listUsers: async (c) => {
			return await c.db.execute<User>(
				"SELECT id, email, name FROM users ORDER BY id",
			);
		},

		// docs:start queries
		// .all() -> the rows
		searchUsers: async (c, query: string) => {
			return await c.db.execute<User>(
				"SELECT id, email, name FROM users WHERE name LIKE ? ORDER BY id",
				`%${query}%`,
			);
		},

		// .first() -> the first row, or null
		getUser: async (c, id: number) => {
			const rows = await c.db.execute<User>(
				"SELECT id, email, name FROM users WHERE id = ?",
				id,
			);
			return rows[0] ?? null;
		},

		// .first("n") -> read the column from the first row
		countUsers: async (c) => {
			const rows = await c.db.execute<{ n: number }>(
				"SELECT COUNT(*) AS n FROM users",
			);
			return rows[0].n;
		},

		// .run() with meta.last_row_id -> RETURNING
		addUser: async (c, email: string, name: string) => {
			const rows = await c.db.execute<{ id: number }>(
				"INSERT INTO users (email, name) VALUES (?, ?) RETURNING id",
				email,
				name,
			);
			return rows[0].id;
		},

		// .run() with meta.changes -> count the RETURNING rows
		renameUser: async (c, id: number, name: string) => {
			const rows = await c.db.execute(
				"UPDATE users SET name = :name WHERE id = :id RETURNING id",
				{ name, id },
			);
			return rows.length;
		},
		// docs:end queries

		// docs:start batch
		// DB.batch([...]) -> one transaction. Every statement commits together or
		// none do, and you can read results between statements.
		placeOrder: async (c, userId: number, total: number) => {
			return await c.db.transaction(
				async (tx) => {
					const rows = await tx.execute<{ id: number }>(
						"INSERT INTO orders (user_id, total) VALUES (?, ?) RETURNING id",
						userId,
						total,
					);
					await tx.execute(
						"UPDATE users SET order_count = order_count + 1 WHERE id = ?",
						userId,
					);
					return rows[0].id;
				},
				{ name: "place-order" },
			);
		},
		// docs:end batch

		// docs:start delete-user
		// Foreign keys aren't enforced, so ON DELETE CASCADE won't run. Delete
		// child rows yourself in the same transaction as the parent.
		deleteUser: async (c, id: number) => {
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"DELETE FROM orders WHERE user_id = ?",
						id,
					);
					await tx.execute("DELETE FROM users WHERE id = ?", id);
				},
				{ name: "delete-user" },
			);
		},
		// docs:end delete-user

		// docs:start import
		// Load rows copied from D1. Keeping the original IDs keeps references
		// stable, and INSERT OR IGNORE makes a re-run of the import safe.
		importUsers: async (c, users: User[]) => {
			await c.db.transaction(
				async (tx) => {
					for (const user of users) {
						await tx.execute(
							"INSERT OR IGNORE INTO users (id, email, name) VALUES (?, ?, ?)",
							user.id,
							user.email,
							user.name,
						);
					}
				},
				{ name: "import-users" },
			);
		},
		// docs:end import
	},
});

export const registry = setup({ use: { tenantDb } });
registry.start();
