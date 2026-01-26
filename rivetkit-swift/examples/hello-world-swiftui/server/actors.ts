import { actor, setup } from "rivetkit";

export const counter = actor({
	state: {
		count: 0,
	},
	actions: {
		increment: (c, amount: number) => {
			c.state.count += amount;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

export const registry = setup({
	use: { counter },
});
