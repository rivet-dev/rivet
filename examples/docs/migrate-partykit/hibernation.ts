import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const lobby = actor({
	// Replaces `options = { hibernate: true }` on the PartyKit server
	options: {
		canHibernateWebSocket: true,
	},

	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS members (
          conn_id TEXT PRIMARY KEY,
          name TEXT NOT NULL
        )
      `);
		},
	}),

	// Hibernating connections stay open across actor sleep, but c.vars does not
	// survive it, so per-connection data goes in SQLite. Replaces conn.setState().
	onConnect: async (c, conn) => {
		if (!conn.params) return;
		const { name } = conn.params as { name: string };
		await c.db.execute(
			"INSERT OR REPLACE INTO members (conn_id, name) VALUES (?, ?)",
			conn.id,
			name,
		);
	},

	onDisconnect: async (c, conn) => {
		await c.db.execute("DELETE FROM members WHERE conn_id = ?", conn.id);
	},

	actions: {
		members: async (c) => {
			const rows = await c.db.execute<{ name: string }>(
				"SELECT name FROM members ORDER BY name",
			);
			return rows.map((row) => row.name);
		},
	},
});
