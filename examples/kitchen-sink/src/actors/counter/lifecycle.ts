import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

type ConnParams = { trackLifecycle?: boolean } | undefined;

export const counterWithLifecycle = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)",
			);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS lifecycle_events (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				);
			`);
		},
	}),
	onWake: async (c) => {
		await recordEvent(c.db, "onWake");
	},
	onSleep: async (c) => {
		await recordEvent(c.db, "onSleep:start");
		await new Promise<void>((resolve) => setTimeout(resolve, 1000));
		await recordEvent(c.db, "onSleep:end");
	},
	onBeforeConnect: async (c, params) => {
		if ((params as ConnParams)?.trackLifecycle) {
			await recordEvent(c.db, "onBeforeConnect");
		}
	},
	onConnect: async (c, conn) => {
		if ((conn.params as ConnParams)?.trackLifecycle) {
			await recordEvent(c.db, "onConnect");
		}
	},
	onDisconnect: async (c, conn) => {
		if ((conn.params as ConnParams)?.trackLifecycle) {
			await recordEvent(c.db, "onDisconnect");
		}
	},
	actions: {
		getEvents: async (c) => {
			const rows = await c.db.execute<{ name: string }>(
				"SELECT name FROM lifecycle_events ORDER BY id",
			);
			return rows.map((row) => row.name);
		},
		increment: async (c, x: number) => {
			const rows = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
				x,
			);
			return rows[0].count;
		},
	},
});

/** Appends a lifecycle hook name so it survives sleep and shows up in `getEvents`. */
async function recordEvent(database: RawAccess, name: string): Promise<void> {
	await database.execute("INSERT INTO lifecycle_events (name) VALUES (?)", name);
}
