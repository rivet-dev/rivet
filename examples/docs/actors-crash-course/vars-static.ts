import { actor } from "rivetkit";

const counter = actor({
	vars: { count: 0, lastAccess: 0 },
	actions: {
		increment: (c) => {
			c.vars.lastAccess = Date.now();
			return c.vars.count += 1;
		},
	},
});
