import { actor } from "rivetkit";

export type ConnState = {
	username: string;
	role: string;
	counter: number;
	createdAt: number;
	noCount: boolean;
};

export const connStateActor = actor({
	state: {
		sharedCounter: 0,
		disconnectionCount: 0,
	},
	// Define connection state
	createConnState: (
		c,
		params: { username?: string; role?: string; noCount?: boolean },
	): ConnState => {
		return {
			username: params?.username || "anonymous",
			role: params?.role || "user",
			counter: 0,
			createdAt: Date.now(),
			noCount: params?.noCount ?? false,
		};
	},
	// Lifecycle hook when a connection is established
	onConnect: (c, conn) => {
		// Broadcast event about the new connection
		c.broadcast("userConnected", {
			id: conn.id,
			username: "anonymous",
			role: "user",
		});
	},
	// Lifecycle hook when a connection is closed
	onDisconnect: (c, conn) => {
		if (!conn.state?.noCount) {
			c.state.disconnectionCount += 1;
			c.broadcast("userDisconnected", {
				id: conn.id,
			});
		}
	},
	actions: {
		// Action to increment the connection's counter
		incrementConnCounter: (c, amount = 1) => {
			c.conn.state.counter += amount;
		},

		// Action to increment the shared counter
		incrementSharedCounter: (c, amount = 1) => {
			c.state.sharedCounter += amount;
			return c.state.sharedCounter;
		},

		// Get the connection state
		getConnectionState: (c) => {
			return { id: c.conn.id, ...c.conn.state };
		},

		// Check all active connections
		getConnectionIds: (c) => {
			return c.conns
				.entries()
				.filter((c) => !c[1].state?.noCount)
				.map((x) => x[0])
				.toArray();
		},

		// Get disconnection count
		getDisconnectionCount: (c) => {
			return c.state.disconnectionCount;
		},

		// Get all active connection states
		getAllConnectionStates: (c) => {
			return c.conns
				.entries()
				.map(([id, conn]) => ({ id, ...conn.state }))
				.toArray();
		},

		// Send message to a specific connection with matching ID
		sendToConnection: (c, targetId: string, message: string) => {
			if (c.conns.has(targetId)) {
				c.conns
					.get(targetId)!
					.send("directMessage", { from: c.conn.id, message });
				return true;
			} else {
				return false;
			}
		},

		// Update connection state (simulated for tests)
		updateConnection: (
			c,
			updates: Partial<{ username: string; role: string }>,
		) => {
			if (updates.username) c.conn.state.username = updates.username;
			if (updates.role) c.conn.state.role = updates.role;
			return c.conn.state;
		},
		disconnectSelf: (c, reason?: string) => {
			c.conn.disconnect(reason ?? "test.disconnect");
			return true;
		},
	},
});
