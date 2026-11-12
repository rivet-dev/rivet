import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

const myActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL DEFAULT 0)",
			);
			await db.execute("INSERT OR IGNORE INTO settings (id) VALUES (1)");

			// Added in v2. Old actors get the new column with its default value.
			const columns = (await db.execute("PRAGMA table_info(settings)")) as {
				name: string;
			}[];
			if (!columns.some((column) => column.name === "label")) {
				await db.execute(
					"ALTER TABLE settings ADD COLUMN label TEXT NOT NULL DEFAULT 'default'",
				);
			}
		},
	}),
	actions: {
		getLabel: async (c) => {
			const [row] = (await c.db.execute(
				"SELECT label FROM settings WHERE id = 1",
			)) as { label: string }[];
			return row.label;
		},
	},
});

const registry = setup({ use: { myActor } });
registry.start();
