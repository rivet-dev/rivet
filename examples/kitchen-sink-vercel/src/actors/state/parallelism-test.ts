import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

export const parallelismTest = actor({
	state: {
		stateCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					count INTEGER NOT NULL DEFAULT 0
				)
			`);
			await db.execute(`
				INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)
			`);
		},
	}),
	events: {
		stateCountChanged: event<{ count: number }>(),
		sqliteCountChanged: event<{ count: number }>(),
	},
	actions: {
		incrementState: (c) => {
			c.state.stateCount += 1;
			c.broadcast("stateCountChanged", { count: c.state.stateCount });
			return { count: c.state.stateCount };
		},
		getStateCount: (c) => {
			return { count: c.state.stateCount };
		},
		incrementSqlite: async (c) => {
			await c.db.execute(`UPDATE counter SET count = count + 1 WHERE id = 1`);
			const results = await c.db.execute<{ count: number }>(
				`SELECT count FROM counter WHERE id = 1`,
			);
			const count = results[0].count;
			c.broadcast("sqliteCountChanged", { count });
			return { count };
		},
		getSqliteCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT count FROM counter WHERE id = 1`,
			);
			return { count: results[0].count };
		},
	},
	options: {
		sleepTimeout: 30_000,
	},
});
