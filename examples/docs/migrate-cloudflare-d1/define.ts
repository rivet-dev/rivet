import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// One actor per tenant, each with its own SQLite database. The tenant ID is
// the actor key, so the table no longer needs a tenant_id column.
export const tenantDb = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS users (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					email TEXT NOT NULL UNIQUE,
					name TEXT NOT NULL
				)
			`);
		},
	}),
	actions: {
		listUsers: async (c) => {
			return await c.db.execute<{
				id: number;
				email: string;
				name: string;
			}>("SELECT id, email, name FROM users ORDER BY id");
		},
		addUser: async (c, email: string, name: string) => {
			const rows = await c.db.execute<{ id: number }>(
				"INSERT INTO users (email, name) VALUES (?, ?) RETURNING id",
				email,
				name,
			);
			return rows[0].id;
		},
	},
});

export const registry = setup({ use: { tenantDb } });
registry.start();
