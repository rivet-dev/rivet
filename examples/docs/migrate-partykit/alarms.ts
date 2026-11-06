import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const gameRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS rounds (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          ended_at INTEGER
        )
      `);
		},
	}),

	// docs:start cron
	// Replaces a partykit.json cron. Jobs are stored per actor, so setting the
	// same name again updates the job instead of adding another one.
	onCreate: async (c) => {
		await c.cron.set({
			name: "cleanup",
			expression: "0 * * * *",
			action: "cleanup",
		});
	},
	// docs:end cron

	actions: {
		// docs:start alarm
		startRound: async (c) => {
			await c.db.execute("INSERT INTO rounds DEFAULT VALUES");

			// Replaces room.storage.setAlarm(Date.now() + 60_000) and onAlarm().
			// An actor can have many pending schedules, each calling an action.
			await c.schedule.after(60_000, "endRound");
			await c.schedule.after(50_000, "warn", "10 seconds left");
		},

		endRound: async (c) => {
			await c.db.execute(
				"UPDATE rounds SET ended_at = ? WHERE ended_at IS NULL",
				Date.now(),
			);
			c.broadcast("roundEnded");
		},

		warn: (c, message: string) => {
			c.broadcast("warning", message);
		},
		// docs:end alarm

		cleanup: async (c) => {
			await c.db.execute(
				"DELETE FROM rounds WHERE ended_at < ?",
				Date.now() - 86_400_000,
			);
		},
	},
});
