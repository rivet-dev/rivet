import { actor } from "rivetkit";

export const HIBERNATION_SLEEP_TIMEOUT = 500;

export type HibernationConnState = {
	count: number;
	connectCount: number;
	disconnectCount: number;
};

export const hibernationActor = actor({
	state: {
		sleepCount: 0,
		wakeCount: 0,
	},
	createConnState: (c): HibernationConnState => {
		return {
			count: 0,
			connectCount: 0,
			disconnectCount: 0,
		};
	},
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	onConnect: (c, conn) => {
		conn.state.connectCount += 1;
	},
	onDisconnect: (c, conn) => {
		conn.state.disconnectCount += 1;
	},
	actions: {
		// Basic RPC that returns a simple value
		ping: (c) => {
			return "pong";
		},
		// Increment the connection's count
		connIncrement: (c) => {
			c.conn.state.count += 1;
			return c.conn.state.count;
		},
		// Get the connection's count
		getConnCount: (c) => {
			return c.conn.state.count;
		},
		// Get the connection's lifecycle counts
		getConnLifecycleCounts: (c) => {
			return {
				connectCount: c.conn.state.connectCount,
				disconnectCount: c.conn.state.disconnectCount,
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
		getActorCounts: (c) => {
			return {
				sleepCount: c.state.sleepCount,
				wakeCount: c.state.wakeCount,
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
