import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

const COUNTER_ROW_ID = 1;

export const sqliteCounter = actor({
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS counter_state (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					count INTEGER NOT NULL
				)
			`);
			await database.execute(
				"INSERT OR IGNORE INTO counter_state (id, count) VALUES (1, 0)",
			);
		},
	}),
	actions: {
		increment: async (c, amount: number) => {
			if (!Number.isFinite(amount)) {
				throw new Error("increment value must be a finite number");
			}

			const delta = Math.trunc(amount);
			await c.db.execute(
				"UPDATE counter_state SET count = count + ? WHERE id = ?",
				delta,
				COUNTER_ROW_ID,
			);
			const rows = await c.db.execute<{ count: number }>(
				"SELECT count FROM counter_state WHERE id = ?",
				COUNTER_ROW_ID,
			);
			return Number(rows[0]?.count ?? 0);
		},
		getCount: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				"SELECT count FROM counter_state WHERE id = ?",
				COUNTER_ROW_ID,
			);
			return Number(rows[0]?.count ?? 0);
		},
	},
});
