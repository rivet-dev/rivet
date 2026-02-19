import { actor, event } from "rivetkit";

export const counterConn = actor({
	state: {
		connectionCount: 0,
	},
	events: {
		newCount: event<number>(),
	},
	connState: { count: 0 },
	onConnect: (c, conn) => {
		c.state.connectionCount += 1;
	},
	onDisconnect: (c, conn) => {
		// Note: We can't determine if disconnect was graceful from here
		// For testing purposes, we'll decrement on all disconnects
		// In real scenarios, you'd use connection tracking with timeouts
		c.state.connectionCount -= 1;
	},
	actions: {
		increment: (c, x: number) => {
			c.conn.state.count += x;
			c.broadcast("newCount", c.conn.state.count);
		},
		setCount: (c, x: number) => {
			c.conn.state.count = x;
			c.broadcast("newCount", x);
		},
		getCount: (c) => {
			return c.conn.state.count;
		},
		getConnectionCount: (c) => {
			return c.state.connectionCount;
		},
	},
});
