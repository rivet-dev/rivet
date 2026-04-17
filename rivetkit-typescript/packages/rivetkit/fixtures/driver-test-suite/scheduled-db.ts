import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

export const scheduledDb = actor({
	state: {
		scheduledCount: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS scheduled_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					action TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		scheduleDbWrite: (c, delayMs: number) => {
			c.schedule.after(delayMs, "onScheduledDbWrite");
		},

		onScheduledDbWrite: async (c) => {
			c.state.scheduledCount++;
			await c.db.execute(
				`INSERT INTO scheduled_log (action, created_at) VALUES ('scheduled', ${Date.now()})`,
			);
		},

		getLogCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM scheduled_log`,
			);
			return results[0].count;
		},

		getScheduledCount: (c) => {
			return c.state.scheduledCount;
		},
	},
});
