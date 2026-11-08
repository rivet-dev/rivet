import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const HIBERNATION_SLEEP_TIMEOUT = 500;

type ConnCountsRow = {
	count: number;
	connect_count: number;
	disconnect_count: number;
};

type ActorCountsRow = {
	sleep_count: number;
	wake_count: number;
};

export const hibernationActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS actor_counts (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					sleep_count INTEGER NOT NULL,
					wake_count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO actor_counts (id, sleep_count, wake_count) VALUES (1, 0, 0)",
			);
			// Hibernatable connections outlive the actor's memory, so their data lives in SQLite.
			await db.execute(`
				CREATE TABLE IF NOT EXISTS conn_counts (
					conn_id TEXT PRIMARY KEY,
					count INTEGER NOT NULL DEFAULT 0,
					connect_count INTEGER NOT NULL DEFAULT 0,
					disconnect_count INTEGER NOT NULL DEFAULT 0
				);
			`);
		},
	}),
	onWake: async (c) => {
		await c.db.execute(
			"UPDATE actor_counts SET wake_count = wake_count + 1 WHERE id = 1",
		);
	},
	onSleep: async (c) => {
		await c.db.execute(
			"UPDATE actor_counts SET sleep_count = sleep_count + 1 WHERE id = 1",
		);
	},
	onConnect: async (c, conn) => {
		await c.db.execute(
			`INSERT INTO conn_counts (conn_id, connect_count) VALUES (?, 1)
			ON CONFLICT (conn_id) DO UPDATE SET connect_count = connect_count + 1`,
			conn.id,
		);
	},
	onDisconnect: async (c, conn) => {
		await c.db.execute(
			`INSERT INTO conn_counts (conn_id, disconnect_count) VALUES (?, 1)
			ON CONFLICT (conn_id) DO UPDATE SET disconnect_count = disconnect_count + 1`,
			conn.id,
		);
	},
	actions: {
		// Basic RPC that returns a simple value
		ping: (_c) => {
			return "pong";
		},
		// Increment the connection's count
		connIncrement: async (c) => {
			const [row] = await c.db.execute<{ count: number }>(
				`INSERT INTO conn_counts (conn_id, count) VALUES (?, 1)
				ON CONFLICT (conn_id) DO UPDATE SET count = count + 1
				RETURNING count`,
				c.conn.id,
			);
			return row.count;
		},
		// Get the connection's count
		getConnCount: async (c) => {
			const rows = await c.db.execute<ConnCountsRow>(
				"SELECT count FROM conn_counts WHERE conn_id = ?",
				c.conn.id,
			);
			return rows[0]?.count ?? 0;
		},
		// Get the connection's lifecycle counts
		getConnLifecycleCounts: async (c) => {
			const rows = await c.db.execute<ConnCountsRow>(
				"SELECT connect_count, disconnect_count FROM conn_counts WHERE conn_id = ?",
				c.conn.id,
			);
			return {
				connectCount: rows[0]?.connect_count ?? 0,
				disconnectCount: rows[0]?.disconnect_count ?? 0,
			};
		},
		// Get all connection IDs
		getConnectionIds: (c) => {
			return c.conns
				.entries()
				.map((x) => x[0])
				.toArray();
		},
		// Get actor sleep/wake counts
		getActorCounts: async (c) => {
			const rows = await c.db.execute<ActorCountsRow>(
				"SELECT sleep_count, wake_count FROM actor_counts WHERE id = 1",
			);
			return {
				sleepCount: rows[0].sleep_count,
				wakeCount: rows[0].wake_count,
			};
		},
		// Trigger sleep
		triggerSleep: (c) => {
			c.sleep();
		},
	},
	options: {
		sleepTimeout: HIBERNATION_SLEEP_TIMEOUT,
	},
});
