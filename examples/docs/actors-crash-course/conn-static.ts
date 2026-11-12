import { actor } from "rivetkit";

const chatRoom = actor({
	// Per-connection data, keyed by connection ID
	createVars: () => ({ visitors: new Map<string, number>() }),
	onConnect: (c, conn) => {
		c.vars.visitors.set(conn.id, Math.random());
	},
	onDisconnect: (c, conn) => {
		c.vars.visitors.delete(conn.id);
	},
	actions: {
		whoAmI: (c) => c.vars.visitors.get(c.conn.id),
	},
});
