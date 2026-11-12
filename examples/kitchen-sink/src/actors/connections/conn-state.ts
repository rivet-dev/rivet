import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

export type ConnState = {
	username: string;
	role: string;
	counter: number;
	createdAt: number;
	noCount: boolean;
};

type ConnParams = { username?: string; role?: string; noCount?: boolean };

export const connStateActor = actor({
	events: {
		userConnected: event<{ id: string; username: string; role: string }>(),
		userDisconnected: event<{ id: string }>(),
		directMessage: event<{ from: string; message: string }>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counters (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					shared_counter INTEGER NOT NULL,
					disconnection_count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO counters (id, shared_counter, disconnection_count) VALUES (1, 0, 0)",
			);
		},
	}),
	// Per-connection data lives in memory, keyed by connection ID.
	createVars: () => ({ conns: new Map<string, ConnState>() }),
	// Lifecycle hook when a connection is established
	onConnect: (c, conn) => {
		const params = conn.params as ConnParams | undefined;
		c.vars.conns.set(conn.id, {
			username: params?.username || "anonymous",
			role: params?.role || "user",
			counter: 0,
			createdAt: Date.now(),
			noCount: params?.noCount ?? false,
		});

		// Broadcast event about the new connection
		c.broadcast("userConnected", {
			id: conn.id,
			username: "anonymous",
			role: "user",
		});
	},
	// Lifecycle hook when a connection is closed
	onDisconnect: async (c, conn) => {
		const data = c.vars.conns.get(conn.id);
		c.vars.conns.delete(conn.id);
		if (!data?.noCount) {
			await c.db.execute(
				"UPDATE counters SET disconnection_count = disconnection_count + 1 WHERE id = 1",
			);
			c.broadcast("userDisconnected", {
				id: conn.id,
			});
		}
	},
	actions: {
		// Action to increment the connection's counter
		incrementConnCounter: (c, amount = 1) => {
			const data = c.vars.conns.get(c.conn.id);
			if (data) data.counter += amount;
		},

		// Action to increment the shared counter
		incrementSharedCounter: async (c, amount = 1) => {
			const rows = await c.db.execute<{ shared_counter: number }>(
				"UPDATE counters SET shared_counter = shared_counter + ? WHERE id = 1 RETURNING shared_counter",
				amount,
			);
			return rows[0].shared_counter;
		},

		// Get the connection's data
		getConnectionState: (c) => {
			return { id: c.conn.id, ...c.vars.conns.get(c.conn.id) };
		},

		// Check all active connections
		getConnectionIds: (c) => {
			return c.conns
				.keys()
				.filter((id) => !c.vars.conns.get(id)?.noCount)
				.toArray();
		},

		// Get disconnection count
		getDisconnectionCount: async (c) => {
			const rows = await c.db.execute<{ disconnection_count: number }>(
				"SELECT disconnection_count FROM counters WHERE id = 1",
			);
			return rows[0].disconnection_count;
		},

		// Get the data of all active connections
		getAllConnectionStates: (c) => {
			return c.conns
				.keys()
				.map((id) => ({ id, ...c.vars.conns.get(id) }))
				.toArray();
		},

		// Send message to a specific connection with matching ID
		sendToConnection: (c, targetId: string, message: string) => {
			if (c.conns.has(targetId)) {
				c.conns
					.get(targetId)
					?.send("directMessage", { from: c.conn.id, message });
				return true;
			} else {
				return false;
			}
		},

		// Update connection data (simulated for tests)
		updateConnection: (
			c,
			updates: Partial<{ username: string; role: string }>,
		) => {
			const data = c.vars.conns.get(c.conn.id);
			if (data && updates.username) data.username = updates.username;
			if (data && updates.role) data.role = updates.role;
			return data;
		},
		disconnectSelf: (c, reason?: string) => {
			c.conn.disconnect(reason ?? "test.disconnect");
			return true;
		},
	},
});
