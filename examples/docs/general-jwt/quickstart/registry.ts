import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const userProfile = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS profile (id INTEGER PRIMARY KEY CHECK (id = 1), display_name TEXT NOT NULL, visits INTEGER NOT NULL)",
			);
			await db.execute(
				"INSERT OR IGNORE INTO profile (id, display_name, visits) VALUES (1, '', 0)",
			);
		},
	}),

	actions: {
		recordVisit: async (c) => {
			await c.db.execute("UPDATE profile SET visits = visits + 1 WHERE id = 1");
			const [row] = (await c.db.execute(
				"SELECT visits FROM profile WHERE id = 1",
			)) as { visits: number }[];
			return row.visits;
		},
		setDisplayName: async (c, displayName: string) => {
			await c.db.execute(
				"UPDATE profile SET display_name = ? WHERE id = 1",
				displayName,
			);
		},
	},
});

export const registry = setup({ use: { userProfile } });
