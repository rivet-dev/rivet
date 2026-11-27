import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// docs:start directory
// Records every tenant, so you can list them without a shared database
const tenantDirectory = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS tenants (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL)",
			);
		},
	}),
	actions: {
		register: async (c, tenantId: string) => {
			await c.db.execute(
				"INSERT OR IGNORE INTO tenants (id, created_at) VALUES (?, ?)",
				tenantId,
				Date.now(),
			);
		},
		listTenants: async (c) => {
			const rows = await c.db.execute<{ id: string }>(
				"SELECT id FROM tenants ORDER BY id",
			);
			return rows.map((row) => row.id);
		},
	},
});

const tenantDb = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT NOT NULL UNIQUE)",
			);
		},
	}),
	// Register the tenant in the directory when its actor is first created
	onCreate: async (c) => {
		const client = c.client<typeof registry>();
		await client.tenantDirectory.getOrCreate(["main"]).register(c.key[0]);
	},
	actions: {
		countUsers: async (c) => {
			const rows = await c.db.execute<{ n: number }>(
				"SELECT COUNT(*) AS n FROM users",
			);
			return rows[0].n;
		},
	},
});
// docs:end directory

export const registry = setup({ use: { tenantDirectory, tenantDb } });
registry.start();
