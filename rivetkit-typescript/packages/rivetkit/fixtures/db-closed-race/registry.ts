import { actor, setup } from "rivetkit";
import { db } from "@/common/database/mod";

// Module-level error collector. The orphaned setInterval writes here after
// the actor is destroyed and state is no longer accessible via actions.
export const collectedErrors: string[] = [];

export const dbClosedRaceActor = actor({
	state: {
		tickCount: 0,
	},
	db: db({
		onMigrate: async (dbHandle) => {
			await dbHandle.execute(`
				CREATE TABLE IF NOT EXISTS tick_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					tick_num INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	run: async (c) => {
		// Cache the db reference up front, like the user's code does.
		// After cleanup, #db is set to undefined, but this cached reference
		// still points at the closed client whose ensureOpen() throws.
		const sql = c.db;
		let tickCount = 0;

		const interval = setInterval(async () => {
			try {
				tickCount += 1;
				await sql.execute(
					`INSERT INTO tick_log (tick_num, created_at) VALUES (${tickCount}, ${Date.now()})`,
				);
			} catch (error: unknown) {
				const msg =
					error instanceof Error ? error.message : String(error);
				collectedErrors.push(msg);
			}
		}, 20);

		// BUG: Not cleaning up the interval on abort.
		void interval;

		// Keep run handler alive until aborted
		await new Promise<void>((resolve) => {
			if (c.aborted) {
				resolve();
				return;
			}
			c.abortSignal.addEventListener("abort", () => resolve(), {
				once: true,
			});
		});
	},
	actions: {
		getTickCount: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) as count FROM tick_log",
			);
			return rows[0]?.count ?? 0;
		},
		destroy: (c) => {
			c.destroy();
		},
	},
	options: {
		sleepTimeout: 60_000,
	},
});

export const registry = setup({
	use: {
		dbClosedRaceActor,
	},
});
