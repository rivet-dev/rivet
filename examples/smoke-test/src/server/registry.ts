import { actor, setup } from "rivetkit";

const counter = actor({
	options: {
		sleepTimeout: 500,
	},
	state: {
		count: 0,
	},
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		getCount: (c) => {
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});

export type Registry = typeof registry;
