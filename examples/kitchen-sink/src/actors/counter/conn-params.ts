import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

type ConnParams = { name?: string } | undefined;

export const counterWithParams = actor({
	events: {
		newCount: event<{ count: number; by: string }>(),
	},
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
				CREATE TABLE IF NOT EXISTS initializers (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				);
			`);
		},
	}),
	// Connection names live in memory, keyed by connection ID.
	createVars: () => ({ names: new Map<string, string>() }),
	onConnect: async (c, conn) => {
		const params = conn.params as ConnParams;
		const name = params?.name || "anonymous";
		c.vars.names.set(conn.id, name);

		// Record connection name
		await c.db.execute("INSERT INTO initializers (name) VALUES (?)", name);
	},
	onDisconnect: (c, conn) => {
		c.vars.names.delete(conn.id);
	},
	actions: {
		increment: async (c, x: number) => {
			const rows = await c.db.execute<{ count: number }>(
				"UPDATE counter SET count = count + ? WHERE id = 1 RETURNING count",
				x,
			);
			const count = rows[0].count;
			c.broadcast("newCount", {
				count,
				by: c.vars.names.get(c.conn.id) ?? "anonymous",
			});
			return count;
		},
		getInitializers: async (c) => {
			const rows = await c.db.execute<{ name: string }>(
				"SELECT name FROM initializers ORDER BY id",
			);
			return rows.map((row) => row.name);
		},
	},
});
