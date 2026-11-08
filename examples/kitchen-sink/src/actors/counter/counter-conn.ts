import { actor, event } from "rivetkit";

export const counterConn = actor({
	events: {
		newCount: event<number>(),
	},
	// Each live connection keeps its own count, keyed by connection ID.
	createVars: () => ({ counts: new Map<string, number>() }),
	onConnect: (c, conn) => {
		c.vars.counts.set(conn.id, 0);
	},
	onDisconnect: (c, conn) => {
		c.vars.counts.delete(conn.id);
	},
	actions: {
		increment: (c, x: number) => {
			const count = (c.vars.counts.get(c.conn.id) ?? 0) + x;
			c.vars.counts.set(c.conn.id, count);
			c.broadcast("newCount", count);
		},
		setCount: (c, x: number) => {
			c.vars.counts.set(c.conn.id, x);
			c.broadcast("newCount", x);
		},
		getCount: (c) => {
			return c.vars.counts.get(c.conn.id) ?? 0;
		},
		getConnectionCount: (c) => {
			return c.vars.counts.size;
		},
	},
});
