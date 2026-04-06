import { actor, setup, event } from "rivetkit";
import { db } from "rivetkit/db";

const COUNTER_ROW_ID = 1;

export const counter = actor({
	state: { count: 0 },
	events: {
		newCount: event<number>(),
	},
	actions: {
		increment: (c, x: number) => {
			if (!Number.isFinite(x)) {
				throw new Error("increment value must be a finite number");
			}

			const delta = Math.trunc(x);
			c.state.count += delta;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

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
	events: {
		newCount: event<number>(),
	},
	actions: {
		increment: async (c, x: number) => {
			if (!Number.isFinite(x)) {
				throw new Error("increment value must be a finite number");
			}

			const delta = Math.trunc(x);
			await c.db.execute(
				"UPDATE counter_state SET count = count + ? WHERE id = ?",
				delta,
				COUNTER_ROW_ID,
			);
			const rows = await c.db.execute<{ count: number }>(
				"SELECT count FROM counter_state WHERE id = ?",
				COUNTER_ROW_ID,
			);
			const count = Number(rows[0]?.count ?? 0);
			c.broadcast("newCount", count);
			return count;
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

export const registry = setup({
	use: { counter, sqliteCounter },
});
