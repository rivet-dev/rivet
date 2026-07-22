import { actor } from "rivetkit";
import { migrations } from "rivetkit/unstable/migrations";
import { db } from "@/common/database/mod";

const migrateItems = migrations({
	tableName: "items_schema_version",
	migrations: [
		{
			version: 1,
			sql: `
				CREATE TABLE items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				)
			`,
		},
		{
			version: 2,
			sql: `
				ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
			`,
		},
	],
});

export const dbPragmaMigrationActor = actor({
	state: {},
	db: db({ onMigrate: migrateItems }),
	actions: {
		insertItem: async (c, name: string) => {
			await c.db.execute(`INSERT INTO items (name) VALUES ('${name}')`);
			const results = await c.db.execute<{ id: number }>(
				"SELECT last_insert_rowid() as id",
			);
			return { id: results[0].id };
		},
		insertItemWithStatus: async (c, name: string, status: string) => {
			await c.db.execute(
				`INSERT INTO items (name, status) VALUES ('${name}', '${status}')`,
			);
			const results = await c.db.execute<{ id: number }>(
				"SELECT last_insert_rowid() as id",
			);
			return { id: results[0].id };
		},
		getItems: async (c) => {
			return await c.db.execute<{
				id: number;
				name: string;
				status: string;
			}>("SELECT id, name, status FROM items ORDER BY id");
		},
		getSchemaVersion: async (c) => {
			const results = await c.db.execute<{ schema_version: number }>(
				"SELECT schema_version FROM items_schema_version WHERE singleton = 1",
			);
			return results[0].schema_version;
		},
		getColumns: async (c) => {
			const results = await c.db.execute<{ name: string }>(
				"PRAGMA table_info(items)",
			);
			return results.map((r) => r.name);
		},
		triggerSleep: (c) => {
			c.sleep();
		},
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});
