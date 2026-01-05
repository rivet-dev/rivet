import { actor, setup } from "rivetkit";

export const counter = actor({
	state: {
		count: 0,
	},

	actions: {
		increment: (c) => {
			c.state.count++;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},

		decrement: (c) => {
			c.state.count--;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},

		getCount: (c) => c.state.count,
	},
});

export const registry = setup({
	use: { counter },
});
