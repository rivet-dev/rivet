import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import { z } from "zod";

const ForkInput = z.object({ label: z.string().max(100) });

const agent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS fork_info (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					source_id TEXT NOT NULL,
					label TEXT
				);
			`);
		},
	}),

	onFork: async (c, fork) => {
		// The source's pending schedules and cron jobs were copied with its database.
		// Cancel them so the fork does not repeat the source's work.
		for (const event of await c.schedule.list()) {
			await c.schedule.cancel(event.id);
		}
		for (const job of await c.cron.list()) {
			await c.cron.delete(job.name);
		}

		// fork.actorId, fork.key and fork.forkedAt come from Rivet. fork.input comes
		// from the caller, so it is validated, and invalid input is ignored rather
		// than thrown, since a throw fails the start and onFork runs again.
		const input = ForkInput.safeParse(fork.input);

		// INSERT OR REPLACE keeps the hook safe to run twice.
		await c.db.execute(
			"INSERT OR REPLACE INTO fork_info (id, source_id, label) VALUES (1, ?, ?)",
			fork.actorId,
			input.success ? input.data.label : null,
		);
	},

	actions: {},
});
