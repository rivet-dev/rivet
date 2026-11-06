import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface Settings {
	theme: "light" | "dark";
	notifications: boolean;
}

const defaultSettings: Settings = { theme: "light", notifications: true };

export const profile = actor({
	db: db({
		onMigrate: async (db) => {
			// docs:start kv-table
			// A small key-value table replaces ctx.storage.get/put
			await db.execute(`
        CREATE TABLE IF NOT EXISTS kv (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        )
      `);
			// docs:end kv-table
			await db.execute(`
        CREATE TABLE IF NOT EXISTS orders (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          item TEXT NOT NULL,
          total INTEGER NOT NULL
        )
      `);
		},
	}),

	// docs:start load
	// Runs on every start, before any action or connection is handled.
	// Replaces loading data inside blockConcurrencyWhile() in the constructor.
	createVars: async (c): Promise<{ settings: Settings }> => {
		const rows = await c.db.execute<{ value: string }>(
			"SELECT value FROM kv WHERE key = 'settings'",
		);
		return {
			settings: rows[0]
				? (JSON.parse(rows[0].value) as Settings)
				: defaultSettings,
		};
	},
	// docs:end load

	actions: {
		// docs:start kv
		getSettings: (c) => c.vars.settings,

		updateSettings: async (c, patch: Partial<Settings>) => {
			c.vars.settings = { ...c.vars.settings, ...patch };
			await c.db.execute(
				"INSERT OR REPLACE INTO kv (key, value) VALUES ('settings', ?)",
				JSON.stringify(c.vars.settings),
			);
			return c.vars.settings;
		},
		// docs:end kv

		// docs:start sql
		// ctx.storage.sql.exec(query, ...bindings).toArray()
		recentOrders: async (c, limit: number) => {
			return await c.db.execute<{
				id: number;
				item: string;
				total: number;
			}>(
				"SELECT id, item, total FROM orders ORDER BY id DESC LIMIT ?",
				limit,
			);
		},
		// docs:end sql

		// docs:start transaction
		// ctx.storage.transactionSync(() => { ... })
		placeOrder: async (c, item: string, total: number) => {
			return await c.db.transaction(async (tx) => {
				await tx.execute(
					"INSERT INTO orders (item, total) VALUES (?, ?)",
					item,
					total,
				);
				const rows = await tx.execute<{ count: number }>(
					"SELECT COUNT(*) AS count FROM orders",
				);
				return rows[0].count;
			});
		},
		// docs:end transaction
	},
});
