import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const sqliteVanillaActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS notes (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					key TEXT NOT NULL UNIQUE,
					value TEXT NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		set: async (c, key: string, value: string) => {
			const updatedAt = Date.now();
			await c.db.execute(
				`INSERT INTO notes (key, value, updated_at) VALUES (?, ?, ?)
				 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at`,
				key,
				value,
				updatedAt,
			);
			return { key, value, updatedAt };
		},
		get: async (c, key: string) => {
			const rows = await c.db.execute(
				"SELECT * FROM notes WHERE key = ?",
				key,
			);
			return rows[0] ?? null;
		},
		getAll: async (c) => {
			return await c.db.execute("SELECT * FROM notes ORDER BY updated_at DESC");
		},
		remove: async (c, key: string) => {
			await c.db.execute("DELETE FROM notes WHERE key = ?", key);
			return { deleted: key };
		},
		count: async (c) => {
			const rows = await c.db.execute("SELECT COUNT(*) as total FROM notes");
			return rows[0];
		},
	},
});
