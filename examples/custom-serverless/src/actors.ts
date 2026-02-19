import { actor, setup, event } from "rivetkit";

const counter = actor({
	state: {
		count: 0,
	},
	events: {
		newCount: event<number>(),
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
