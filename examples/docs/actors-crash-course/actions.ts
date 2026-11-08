import { actor } from "rivetkit";

const counter = actor({
	vars: { count: 0 },
	actions: {
		increment: (c, amount: number) => (c.vars.count += amount),
		getCount: (c) => c.vars.count,
	},
});
