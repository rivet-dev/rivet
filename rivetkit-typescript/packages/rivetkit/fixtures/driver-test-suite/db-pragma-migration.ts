import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

const SLEEP_GRACE_PERIOD_MS = 50;

export const dbPragmaMigrationActor = actor({
	state: {},
	db: db({
		onMigrate: async (db) => {
			const [{ user_version }] = (await db.execute(
				"PRAGMA user_version",
			)) as { user_version: number }[];

			if (user_version < 1) {
				await db.execute(`
					CREATE TABLE items (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						name TEXT NOT NULL
					)
				`);
			}

			if (user_version < 2) {
				await db.execute(`
					ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
				`);
			}

			await db.execute("PRAGMA user_version = 2");
		},
	}),
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
		getUserVersion: async (c) => {
			const results = (await c.db.execute("PRAGMA user_version")) as {
				user_version: number;
			}[];
			return results[0].user_version;
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
		sleepGracePeriod: SLEEP_GRACE_PERIOD_MS,
		sleepTimeout: 1_000,
	},
});
